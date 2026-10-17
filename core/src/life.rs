//! The registry of bodies: slots, cells, lineages, brain groups, born rows, and the ring of updates due.

use crate::body::{Body, Consts, MAX_INTERVAL};
use crate::brain::{self, Shape};
use crate::lineage::Lineage;
use crate::rng::Rng;
use crate::World;
use serde::{Deserialize, Serialize};

/// Updates are kept in a ring of this many days (more than the longest interval).
const RING: u64 = 384;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Life {
    pub bodies: Vec<Body>,
    free: Vec<u32>,
    /// Bodies in each cell, in a fixed order
    pub at: Vec<Vec<u32>>,
    /// Per cell, from the last survey: members, then their visible traits summed (mass and capture area are totals)
    pub sum: Vec<[f64; 7]>,
    /// Per cell, from the last survey: the share of the light reaching its bodies' capture area that they absorb
    #[serde(skip)]
    pub absorb: Vec<f32>,
    /// Per body slot, from the last survey: its light per m² of capture area over its cell's mean (`survey::shade`)
    #[serde(skip)]
    pub shade: Vec<f64>,
    /// Per cell, from the last survey: the share of that absorbed light set aside for its bodies' light reactions
    #[serde(skip)]
    pub keep: Vec<f32>,
    /// Per cell, from the last monthly survey: ground temperature, liquid depth, surface height
    pub env: Vec<[f64; 3]>,
    /// Per cell, from the last monthly survey: its loose substances as a brain perceives them, [cell * substances + k]
    pub seen: Vec<f32>,
    /// Per cell, from the last survey: what a brain perceives of it as a place to move to
    #[serde(skip)]
    pub cellf: Vec<[f32; 8]>,
    /// Updates due, by day modulo RING: (slot, generation)
    due: Vec<Vec<(u32, u32)>>,
    /// The brain of each lineage (None once it died out), and its living bodies
    pub brains: Vec<Option<Lineage>>,
    pub members: Vec<u32>,
    /// Brain groups (a group with no members is free)
    pub groups: Vec<Group>,
    free_groups: Vec<u32>,
    /// Born brain rows, each shared by the bodies born with them (none: free)
    pub genomes: Vec<Genome>,
    free_genomes: Vec<u32>,
    /// Grid width (for the groups' centres)
    pub gw: u32,
}

/// Born brain rows and how many living bodies were born with them (most children share their parent's, `copy`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Genome {
    pub rows: Vec<f32>,
    pub bodies: u32,
}

/// Bodies that stayed near each other share the rows of one brain (`rows`): the mean of the rows they were born with,
/// plus what the group learned, kept up to date as members come and go.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Group {
    pub lineage: u32,
    pub members: u32,
    /// Sums of the members' cell coordinates
    pub sx: i64,
    pub sy: i64,
    pub rows: Vec<f32>,
    /// Sums of the members' born rows (kept up to date as members come and go, and rebuilt from the members each month,
    /// `resum`; not saved); and of their forgetting rates
    #[serde(skip)]
    pub sum: Vec<f32>,
    /// Members of its bodies (a node counts its count), as of the start of the month (rebuilt with `sum`, not saved): a
    /// body's decisions move the shared rows by its share of them
    #[serde(skip)]
    pub count: f64,
    pub forget: f32,
    /// Moving average of the square of the critic's error (0 until the first one)
    pub var: f32,
    /// Per learned row, a moving average of the square of its share of the learning steps
    pub ms: Vec<f32>,
}

impl Group {
    pub(crate) fn centre(&self) -> (f32, f32) {
        let m = self.members.max(1) as f32;
        (self.sx as f32 / m, self.sy as f32 / m)
    }
    /// The mean of its members' born rows.
    pub fn innate(&self) -> impl Iterator<Item = f32> + '_ {
        let k = 1.0 / self.members.max(1) as f32;
        self.sum.iter().map(move |&s| s * k)
    }
    /// What the group learned: its rows less the mean of its members' born rows.
    pub fn learned(&self) -> impl Iterator<Item = f32> + '_ {
        self.rows.iter().zip(self.innate()).map(|(r, m)| r - m)
    }
}

impl Life {
    pub fn alive(&self) -> impl Iterator<Item = &Body> {
        self.bodies.iter().filter(|b| b.alive)
    }
    pub fn lineages(&self) -> usize {
        self.members.iter().filter(|&&m| m > 0).count()
    }
    pub fn new_lineage(&mut self, l: Lineage) -> u32 {
        self.brains.push(Some(l));
        self.members.push(0);
        (self.brains.len() - 1) as u32
    }
    /// A new group of `lineage` with these rows (the difference from its first member's born rows counts as learned).
    pub fn new_group(&mut self, lineage: u32, rows: Vec<f32>) -> u32 {
        let i = free_place(&mut self.groups, &mut self.free_groups);
        self.groups[i] = Group { lineage, rows, ..Default::default() };
        i as u32
    }
    /// A new group of `lineage` with these rows that keeps what group `from` knows of its learning (`Group::var`,
    /// `Group::ms`).
    pub(crate) fn fork_group(&mut self, from: u32, lineage: u32, rows: Vec<f32>) -> u32 {
        let (var, ms) = (self.groups[from as usize].var, self.groups[from as usize].ms.clone());
        let g = self.new_group(lineage, rows);
        (self.groups[g as usize].var, self.groups[g as usize].ms) = (var, ms);
        g
    }
    /// The brain shape of `lineage` (none once it died out).
    pub(crate) fn shape_of(&self, lineage: u32) -> Option<Shape> {
        self.brains[lineage as usize].as_ref().map(|l| l.shape)
    }
    /// Brain rows of a body of lineage `from` carried over to the shape of lineage `to` where they differ
    /// (`brain::rerow`, new weights of spread `noise`).
    pub(crate) fn carry_rows(&self, rows: Vec<f32>, from: u32, to: u32, r: &mut Rng, noise: f32) -> Vec<f32> {
        match (self.shape_of(from), self.shape_of(to)) {
            (Some(x), Some(y)) if x != y => brain::rerow(&rows, x, y, r, noise),
            _ => rows,
        }
    }
    /// New born rows (not yet given to a body).
    pub fn new_genome(&mut self, rows: Vec<f32>) -> u32 {
        let i = free_place(&mut self.genomes, &mut self.free_genomes);
        self.genomes[i] = Genome { rows, bodies: 0 };
        i as u32
    }
    /// Body `b` has no brain (`Lineage::none`).
    pub(crate) fn brainless(&self, b: &Body) -> bool {
        self.brains.get(b.lineage as usize).and_then(|l| l.as_ref()).is_some_and(|l| l.none())
    }
    /// The starting kind body `b` descends from (`Lineage::root`); `u32::MAX` if its lineage is gone.
    pub fn root(&self, b: &Body) -> u32 {
        self.brains.get(b.lineage as usize).and_then(|l| l.as_ref()).map_or(u32::MAX, |l| l.root)
    }
    /// The rows body `b` was born with.
    pub fn born(&self, b: &Body) -> &[f32] {
        &self.genomes[b.genome as usize].rows
    }
    /// Count body `b` into group `g` (`d` = 1) or out of it (`d` = -1): its cell, its born rows (the rows shift by the
    /// change of the mean, so what the group learned stays) and its forgetting rate.
    pub(crate) fn enrol(&mut self, g: u32, b: &Body, d: i64) {
        self.shift(g, b.cell, d);
        let (gr, born) = (&mut self.groups[g as usize], &self.genomes[b.genome as usize].rows);
        let n0 = gr.members as f32;
        gr.members = (gr.members as i64 + d) as u32;
        gr.forget += d as f32 * b.forget();
        if gr.members == 0 {
            (gr.rows, gr.sum, gr.ms, gr.sx, gr.sy, gr.forget) = (Vec::new(), Vec::new(), Vec::new(), 0, 0, 0.0);
            self.free_groups.push(g);
            return;
        }
        gr.sum.resize(gr.rows.len(), 0.0);
        // A new group keeps the rows it was given (the first member's difference counts as learned).
        let (k1, k0, d) = if n0 > 0.0 { (1.0 / gr.members as f32, 1.0 / n0, d as f32) } else { (0.0, 0.0, d as f32) };
        for ((r, s), &x) in gr.rows.iter_mut().zip(gr.sum.iter_mut()).zip(born) {
            let s1 = *s + d * x;
            *r += s1 * k1 - *s * k0;
            *s = s1;
        }
    }
    /// Rebuild the groups' sums of born rows and their counts of members from their bodies, in slot order.
    pub fn resum(&mut self) {
        for g in self.groups.iter_mut() {
            g.sum.clear();
            g.sum.resize(g.rows.len(), 0.0);
            g.count = 0.0;
        }
        for b in self.bodies.iter().filter(|b| b.alive) {
            let (gr, born) = (&mut self.groups[b.group as usize], &self.genomes[b.genome as usize].rows);
            gr.sum.iter_mut().zip(born).for_each(|(s, &x)| *s += x);
            gr.count += b.n();
        }
    }
    /// Move the centre of group `g` by a member at `cell` (`d` = 1 arrived, -1 left).
    fn shift(&mut self, g: u32, cell: u32, d: i64) {
        let (x, y) = ((cell % self.gw) as i64, (cell / self.gw) as i64);
        let gr = &mut self.groups[g as usize];
        (gr.sx, gr.sy) = (gr.sx + d * x, gr.sy + d * y);
    }
    /// Place a new body on `day`; it first updates `delay` days later (1..=`MAX_INTERVAL`).
    pub fn add(&mut self, mut b: Body, day: u64, delay: u64) -> usize {
        let slot = free_place(&mut self.bodies, &mut self.free);
        (b.gen, b.alive, b.last, b.count) = (self.bodies[slot].gen, true, day, b.n());
        self.at[b.cell as usize].push(slot as u32);
        self.enter(&b);
        self.schedule(day + delay.clamp(1, MAX_INTERVAL as u64), slot as u32, b.gen);
        self.bodies[slot] = b;
        slot
    }
    pub(crate) fn remove(&mut self, slot: usize) {
        let mut b = std::mem::take(&mut self.bodies[slot]);
        let cell = b.cell as usize;
        self.leave(&b);
        (b.alive, b.store, b.genes, b.trace, b.mem) = (false, Vec::new(), Vec::new(), Vec::new(), Vec::new());
        b.gen += 1;
        self.bodies[slot] = b;
        self.at[cell].retain(|&s| s as usize != slot);
        self.free.push(slot as u32);
    }
    /// Count body `b` into its lineage, its born rows and its group.
    fn enter(&mut self, b: &Body) {
        self.members[b.lineage as usize] += 1;
        self.genomes[b.genome as usize].bodies += 1;
        self.enrol(b.group, b, 1);
    }
    /// Count body `b` out of its group, its born rows and its lineage (which dies out with its last body).
    fn leave(&mut self, b: &Body) {
        self.enrol(b.group, b, -1);
        self.unref(b.genome);
        let lin = b.lineage as usize;
        self.members[lin] -= 1;
        if self.members[lin] == 0 {
            self.brains[lin] = None;
        }
    }
    /// One body fewer born with rows `g`.
    fn unref(&mut self, g: u32) {
        let gen = &mut self.genomes[g as usize];
        gen.bodies -= 1;
        if gen.bodies == 0 {
            gen.rows = Vec::new();
            self.free_genomes.push(g);
        }
    }
    /// Body `b` (taken out of its slot) now has these genes and born rows; its group follows.
    pub(crate) fn reborn(&mut self, b: &mut Body, genes: Vec<f32>, rows: Vec<f32>) {
        if rows == self.genomes[b.genome as usize].rows {
            return self.regene(b, genes);
        }
        let g = self.new_genome(rows);
        let new = Body { cell: b.cell, genome: g, genes, ..Default::default() };
        self.genomes[g as usize].bodies += 1;
        self.swap_rows(b.group, b, &new);
        self.unref(b.genome);
        (b.genome, b.genes, b.k) = (g, new.genes, Consts::default());
    }
    /// `enrol(g, new, 1)` then `enrol(g, old, -1)` (a body changes its born rows within its group): the same sums in the
    /// same order, bit for bit, in one pass over the rows.
    fn swap_rows(&mut self, g: u32, old: &Body, new: &Body) {
        let (gr, was, now) = (&self.groups[g as usize], &self.genomes[old.genome as usize].rows, &self.genomes[new.genome as usize].rows);
        if gr.members == 0 || gr.rows.len() != was.len() || was.len() != now.len() {
            self.enrol(g, new, 1);
            return self.enrol(g, old, -1);
        }
        self.shift(g, new.cell, 1);
        self.shift(g, old.cell, -1);
        let (gr, was, now) = (&mut self.groups[g as usize], &self.genomes[old.genome as usize].rows, &self.genomes[new.genome as usize].rows);
        // Members n, then n + 1 with the new body in, then n again.
        let (n, more) = (gr.members as f32, (gr.members + 1) as f32);
        gr.forget += new.forget();
        gr.forget -= old.forget();
        gr.sum.resize(gr.rows.len(), 0.0);
        let (k1, k0) = (1.0 / more, 1.0 / n);
        for (((r, s), &x), &y) in gr.rows.iter_mut().zip(gr.sum.iter_mut()).zip(now).zip(was) {
            let s1 = *s + x;
            *r += s1 * k1 - *s * k0;
            let s2 = s1 - y;
            *r += s2 * k0 - s1 * k1;
            *s = s2;
        }
    }
    /// Body `b` (taken out of its slot) has these genes and the born rows it had (a member split off a node and back): of
    /// its group only the forgetting rate changes.
    pub(crate) fn regene(&mut self, b: &mut Body, genes: Vec<f32>) {
        let old = b.forget();
        (b.genes, b.k) = (genes, Consts::default());
        let gr = &mut self.groups[b.group as usize];
        gr.forget = gr.forget + b.forget() - old;
    }
    /// Body `b` (taken out of its slot) moves to `lineage`, born with `rows`, in its new group `group`.
    pub(crate) fn move_to(&mut self, b: &mut Body, lineage: u32, group: u32, rows: Vec<f32>) {
        self.leave(b);
        (b.lineage, b.group, b.genome) = (lineage, group, self.new_genome(rows));
        self.enter(b);
    }
    /// Size the cell lists and the ring of updates for a grid of `n` cells, `gw` wide (once, on a new grid).
    pub(crate) fn fit_grid(&mut self, n: usize, gw: u32) {
        if self.at.len() != n {
            (self.at, self.due, self.gw) = (vec![Vec::new(); n], vec![Vec::new(); RING as usize], gw);
        }
    }
    /// File an update of body (`slot`, `gen`) on `day`.
    pub(crate) fn schedule(&mut self, day: u64, slot: u32, gen: u32) {
        self.due[(day % RING) as usize].push((slot, gen));
    }
    /// The updates filed for `day`, taken out of the ring.
    pub(crate) fn take_due(&mut self, day: u64) -> Vec<(u32, u32)> {
        std::mem::take(&mut self.due[(day % RING) as usize])
    }
    /// Body `slot` of group `g` moves from cell `from` to `to`.
    pub fn relocate(&mut self, slot: usize, g: u32, from: usize, to: usize) {
        self.at[from].retain(|&s| s as usize != slot);
        self.at[to].push(slot as u32);
        self.shift(g, to as u32, 1);
        self.shift(g, from as u32, -1);
    }
}

impl World {
    /// Place body `c`, its first update after its interval; `keep` is the share of light the reactions of the body it
    /// came from stored (`World::light_keep`), which it inherits.
    pub(crate) fn place(&mut self, c: Body, keep: Option<f32>) -> usize {
        let delay = c.interval();
        let s = self.life.add(c, self.day, delay);
        if let Some(k) = keep {
            self.light_keep.set(s, self.life.bodies[s].gen, k);
        }
        s
    }
}

/// A free place in `v` (the last one freed first, `free`), or a new one at its end.
fn free_place<T: Default>(v: &mut Vec<T>, free: &mut Vec<u32>) -> usize {
    match free.pop() {
        Some(i) => i as usize,
        None => {
            v.push(T::default());
            v.len() - 1
        }
    }
}
