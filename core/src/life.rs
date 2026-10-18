//! The registry of bodies: slots, cells, lineages, brain groups, born rows, and the ring of updates due.

use crate::body::{Body, Consts, MAX_INTERVAL};
use crate::brain::{self, Shape};
use crate::lineage::Lineage;
use crate::rng::Rng;
use crate::World;
use serde::{Deserialize, Serialize};

/// Updates are kept in a ring of this many days (more than the longest interval).
const RING: u64 = 384;
/// The brain group of a body without a brain: none (it has no rows to share).
pub const NO_GROUP: u32 = u32::MAX;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Life {
    pub bodies: Vec<Body>,
    free: Vec<u32>,
    /// Bodies in each cell, in a fixed order
    pub at: Vec<Vec<u32>>,
    /// Per cell, from the last survey: members, then their visible traits summed (mass and capture area are totals).
    /// The survey's caches here are not saved: a snapshot is taken at a month's start, where the survey builds them
    /// all before any law reads them.
    #[serde(skip)]
    pub sum: Vec<[f64; 7]>,
    /// Per cell, from the last survey: the share of the light reaching its bodies' capture area that they absorb
    #[serde(skip)]
    pub absorb: Vec<f32>,
    /// Per body slot, from the last survey: its light per m² of capture area over its cell's mean (`survey::shade`)
    #[serde(skip)]
    pub shade: Vec<f64>,
    /// Per cell: whether its bodies changed since the survey last shared out its light (`Life::stir`; empty: all)
    #[serde(skip)]
    pub stale: Vec<bool>,
    /// Per cell, from the last survey: the share of that absorbed light set aside for its bodies' light reactions
    #[serde(skip)]
    pub keep: Vec<f32>,
    /// Per cell, from the last monthly survey: ground temperature, liquid depth, surface height
    #[serde(skip)]
    pub env: Vec<[f64; 3]>,
    /// Per cell, from the last monthly survey: its loose substances as a brain perceives them, [cell * substances + k]
    #[serde(skip)]
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
    /// Width of the square map, cells (for the groups' centres)
    pub gw: u32,
}

/// Born brain rows and how many hold them: the living bodies born with them (most children share their parent's, `copy`)
/// and the changes of groups not yet made (`Group::due`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Genome {
    pub rows: Vec<f32>,
    pub refs: u32,
}

/// Bodies that stayed near each other share the rows of one brain (`rows`): the mean of the rows their members were born
/// with (a node counts its count), plus what the group learned. As members come and go only their count changes at
/// once; the mean follows when the rows are read (`Life::settle`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Group {
    pub lineage: u32,
    pub members: u32,
    /// Sums of the members' cell coordinates
    pub sx: i64,
    pub sy: i64,
    /// The shared rows as of the last `Life::settle` (read them after it)
    pub rows: Vec<f32>,
    /// Sum of its members' born rows, each body's by its count, as of the last `settle` (rebuilt from the bodies each
    /// month, `resum`; not saved)
    #[serde(skip)]
    pub sum: Vec<f64>,
    /// Members of its bodies now (rebuilt with `sum`, not saved): a body's decisions move the shared rows by its share of
    /// them
    #[serde(skip)]
    pub count: f64,
    /// Members in `sum` (not saved)
    #[serde(skip)]
    formed: f64,
    /// Changes of members not yet in `sum` and `rows`: the born rows (`Life::genomes`) and the change of members born
    /// with them (not saved; none after `settle`)
    #[serde(skip)]
    due: Vec<(u32, f64)>,
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
    /// The mean of its members' born rows (as of the last `Life::settle`).
    pub fn innate(&self) -> impl Iterator<Item = f32> + '_ {
        let k = 1.0 / self.formed;
        self.sum.iter().map(move |&s| (s * k) as f32)
    }
    /// What the group learned: its rows less the mean of its members' born rows (as of the last `Life::settle`).
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
    /// A new group of `lineage` with these rows (the difference from its first member's born rows counts as learned);
    /// `NO_GROUP` for a lineage without a brain.
    pub fn new_group(&mut self, lineage: u32, rows: Vec<f32>) -> u32 {
        if self.brains[lineage as usize].as_ref().is_some_and(|l| l.none()) {
            return NO_GROUP;
        }
        let i = free_place(&mut self.groups, &mut self.free_groups);
        self.groups[i] = Group { lineage, rows, ..Default::default() };
        i as u32
    }
    /// A new group of `lineage` with these rows that keeps what group `from` knows of its learning (`Group::var`,
    /// `Group::ms`; nothing if `from` is `NO_GROUP`); `NO_GROUP` for a lineage without a brain.
    pub(crate) fn fork_group(&mut self, from: u32, lineage: u32, rows: Vec<f32>) -> u32 {
        let g = self.new_group(lineage, rows);
        let Some(f) = self.groups.get(from as usize).filter(|_| g != NO_GROUP) else { return g };
        let (var, ms) = (f.var, f.ms.clone());
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
        self.genomes[i] = Genome { rows, refs: 0 };
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
    /// Count body `b` into group `g` (`d` = 1) or out of it (`d` = -1): its cell, and its members' born rows (`weigh`).
    pub(crate) fn enrol(&mut self, g: u32, b: &Body, d: i64) {
        if g == NO_GROUP {
            return;
        }
        self.shift(g, b.cell, d);
        let gr = &mut self.groups[g as usize];
        gr.members = (gr.members as i64 + d) as u32;
        if gr.members == 0 {
            for (x, _) in std::mem::take(&mut gr.due) {
                self.unref(x);
            }
            let gr = &mut self.groups[g as usize];
            (gr.rows, gr.sum, gr.ms, gr.sx, gr.sy, gr.count, gr.formed) = (Vec::new(), Vec::new(), Vec::new(), 0, 0, 0.0, 0.0);
            self.free_groups.push(g);
            return;
        }
        self.weigh(g, b.genome, d as f64 * b.n());
    }
    /// Body `b` (taken out of its slot) now has `count` members: its group weighs it by them.
    pub(crate) fn set_count(&mut self, b: &mut Body, count: f64) {
        let n0 = b.n();
        b.count = count;
        self.stir(b.cell);
        self.weigh(b.group, b.genome, b.n() - n0);
    }
    /// `n` members (fewer: `n` below 0) born with the rows of `genome` in group `g`: its count changes now, its sum of
    /// born rows and its rows at the next `settle`.
    fn weigh(&mut self, g: u32, genome: u32, n: f64) {
        if n == 0.0 || g == NO_GROUP {
            return;
        }
        let gr = &mut self.groups[g as usize];
        gr.count += n;
        if let Some(d) = gr.due.iter_mut().find(|d| d.0 == genome) {
            d.1 += n;
        } else {
            gr.due.push((genome, n));
            self.genomes[genome as usize].refs += 1;
        }
    }
    /// Group `g`'s changes of members go into its sum of born rows, two born rows a pass, and its rows shift by the change
    /// of the mean (the old sum by the change of 1 / members, the change of the sum by 1 / members now), so what it
    /// learned stays. A group of no members yet keeps the rows it was given (its first members' difference counts as
    /// learned).
    pub(crate) fn settle(&mut self, g: u32) {
        let Life { groups, genomes, .. } = self;
        let Some(gr) = groups.get_mut(g as usize).filter(|gr| !gr.due.is_empty()) else { return };
        gr.sum.resize(gr.rows.len(), 0.0);
        let (k1, k0) = if gr.formed > 0.0 { (1.0 / gr.count, 1.0 / gr.formed) } else { (0.0, 0.0) };
        for (q, two) in gr.due.chunks(2).enumerate() {
            let kk = if q == 0 { k1 - k0 } else { 0.0 };
            let ((x1, n1), (x2, n2)) = (two[0], two.get(1).copied().unwrap_or((two[0].0, 0.0)));
            let (x1, x2) = (&genomes[x1 as usize].rows, &genomes[x2 as usize].rows);
            for (((r, s), &a), &b) in gr.rows.iter_mut().zip(gr.sum.iter_mut()).zip(x1).zip(x2) {
                let d = n1 * a as f64 + n2 * b as f64;
                *r += (*s * kk + d * k1) as f32;
                *s += d;
            }
        }
        gr.formed = gr.count;
        for (x, _) in std::mem::take(&mut gr.due) {
            self.unref(x);
        }
    }
    /// `settle` every group.
    pub(crate) fn settle_all(&mut self) {
        for g in 0..self.groups.len() as u32 {
            self.settle(g);
        }
    }
    /// Rebuild the groups' sums of born rows and their counts of members from their bodies, in slot order.
    pub fn resum(&mut self) {
        self.settle_all();
        for g in self.groups.iter_mut() {
            g.sum.clear();
            g.sum.resize(g.rows.len(), 0.0);
            g.count = 0.0;
        }
        for b in self.bodies.iter().filter(|b| b.alive && b.group != NO_GROUP) {
            let (gr, born, n) = (&mut self.groups[b.group as usize], &self.genomes[b.genome as usize].rows, b.n());
            gr.sum.iter_mut().zip(born).for_each(|(s, &x)| *s += n * x as f64);
            gr.count += n;
        }
        self.groups.iter_mut().for_each(|g| g.formed = g.count);
    }
    /// Move the centre of group `g` by a member at `cell` (`d` = 1 arrived, -1 left).
    fn shift(&mut self, g: u32, cell: u32, d: i64) {
        let (x, y) = ((cell % self.gw) as i64, (cell / self.gw) as i64);
        if let Some(gr) = self.groups.get_mut(g as usize) {
            (gr.sx, gr.sy) = (gr.sx + d * x, gr.sy + d * y);
        }
    }
    /// Place a new body on `day`; it first updates `delay` days later (1..=`MAX_INTERVAL`).
    pub fn add(&mut self, mut b: Body, day: u64, delay: u64) -> usize {
        let slot = free_place(&mut self.bodies, &mut self.free);
        (b.gen, b.alive, b.last, b.count) = (self.bodies[slot].gen, true, day, b.n());
        self.at[b.cell as usize].push(slot as u32);
        self.stir(b.cell);
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
        self.stir(cell as u32);
        self.free.push(slot as u32);
    }
    /// Count body `b` into its lineage, its born rows and its group.
    fn enter(&mut self, b: &Body) {
        self.members[b.lineage as usize] += 1;
        self.genomes[b.genome as usize].refs += 1;
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
    /// One holder fewer of born rows `g`.
    fn unref(&mut self, g: u32) {
        let gen = &mut self.genomes[g as usize];
        gen.refs -= 1;
        if gen.refs == 0 {
            gen.rows = Vec::new();
            self.free_genomes.push(g);
        }
    }
    /// Body `b` (taken out of its slot) now has these genes and born rows; its group follows.
    pub(crate) fn reborn(&mut self, b: &mut Body, genes: Vec<f32>, rows: Vec<f32>) {
        if rows == self.genomes[b.genome as usize].rows {
            return self.regene(b, genes);
        }
        let (g, n) = (self.new_genome(rows), b.n());
        self.genomes[g as usize].refs += 1;
        (b.genes, b.k) = (genes, Consts::default());
        self.weigh(b.group, g, n);
        self.weigh(b.group, b.genome, -n);
        self.unref(b.genome);
        b.genome = g;
    }
    /// Body `b` (taken out of its slot) has these genes and the born rows it had (a member split off a node and back).
    pub(crate) fn regene(&mut self, b: &mut Body, genes: Vec<f32>) {
        (b.genes, b.k) = (genes, Consts::default());
    }
    /// Body `b` (taken out of its slot) moves to `lineage`, born with `rows`, in its new group `group`.
    pub(crate) fn move_to(&mut self, b: &mut Body, lineage: u32, group: u32, rows: Vec<f32>) {
        self.leave(b);
        (b.lineage, b.group, b.genome) = (lineage, group, self.new_genome(rows));
        self.enter(b);
    }
    /// The bodies of `cell` changed (one came, went, or changed its members or what others see of it): the survey
    /// shares out its light again (`survey::shade`).
    pub(crate) fn stir(&mut self, cell: u32) {
        if let Some(x) = self.stale.get_mut(cell as usize) {
            *x = true;
        }
    }
    /// Size the cell lists and the ring of updates for a square map of `n` cells, `gw` wide (once, on a new map).
    pub(crate) fn fit_map(&mut self, n: usize, gw: u32) {
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
        self.stir(from as u32);
        self.stir(to as u32);
        self.shift(g, to as u32, 1);
        self.shift(g, from as u32, -1);
    }
}

impl World {
    /// Place body `c`, its first update after its interval; `keep` is the share of light the reactions of the body it
    /// came from stored (`World::light_keep`), which it inherits. Its visible traits are worked out from what it holds
    /// now: a child or a part split off a node has its own stores and genes, and others see it and its shade
    /// (`survey::shade`) through `Body::vis` before its first update.
    pub(crate) fn place(&mut self, mut c: Body, keep: Option<f32>) -> usize {
        self.see(&mut c);
        let delay = c.interval(self);
        let s = self.life.add(c, self.day(), delay);
        if let Some(k) = keep {
            self.light_keep.set(s, self.life.bodies[s].gen, k);
        }
        s
    }
    /// What others see of body `b` is worked out again from what it holds now (`Body::visible`), and its cell's light
    /// is shared out again.
    pub(crate) fn see(&mut self, b: &mut Body) {
        b.vis = b.visible(&self.chem, &self.cfg);
        self.life.stir(b.cell);
    }
}

/// A free place in `v` (the last one freed first, `free`), or a new one at its end.
pub(crate) fn free_place<T: Default>(v: &mut Vec<T>, free: &mut Vec<u32>) -> usize {
    match free.pop() {
        Some(i) => i as usize,
        None => {
            v.push(T::default());
            v.len() - 1
        }
    }
}
