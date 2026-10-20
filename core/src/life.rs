//! The registry of bodies: slots, cells, lineages, born rows, and the ring of updates due.

use crate::body::{Body, MAX_INTERVAL};
use crate::brain::Shape;
use crate::lineage::Lineage;
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
    /// Per cell, from the last survey: members, then their visible traits summed. The survey's caches are not saved:
    /// a snapshot is taken at a month's start, where the survey builds them before any law reads them; this one the
    /// tape reads first, so `tape::load_snapshot` rebuilds it (`survey::sums`).
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
    /// Per cell, from the last survey: what a brain perceives of it as a place to move to (`choose::cells`)
    #[serde(skip)]
    pub cellf: Vec<[f32; crate::choose::NF]>,
    /// Updates due, by day modulo RING: (slot, generation)
    due: Vec<Vec<(u32, u32)>>,
    /// The brain of each lineage (None once it died out), and its living bodies
    pub brains: Vec<Option<Lineage>>,
    pub members: Vec<u32>,
    /// Born brain rows, each shared by the bodies born with them (none: free)
    pub genomes: Vec<Genome>,
    free_genomes: Vec<u32>,
}

/// Born brain rows and how many hold them: the living bodies born with them (most children share their parent's, `copy`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Genome {
    pub rows: Vec<f32>,
    pub refs: u32,
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
    /// The brain shape of `lineage` (none once it died out).
    pub(crate) fn shape_of(&self, lineage: u32) -> Option<Shape> {
        self.brains[lineage as usize].as_ref().map(|l| l.shape)
    }
    /// The rows of a body born with `born` from body `b`: its born rows plus what `b` learned, where the brains have one
    /// shape; else its born rows.
    pub(crate) fn inherit(&self, born: &[f32], lineage: u32, b: &Body) -> Vec<f32> {
        let (from, rows) = (self.born(b), &b.rows);
        if self.shape_of(lineage) != self.shape_of(b.lineage) || born.len() != rows.len() || from.len() != rows.len() {
            return born.to_vec();
        }
        born.iter().zip(rows).zip(from).map(|((x, r), f)| x + (r - f)).collect()
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
    /// Body `b` (taken out of its slot) now has `count` members.
    pub(crate) fn set_count(&mut self, b: &mut Body, count: f64) {
        b.count = count;
        self.stir(b.cell);
    }
    /// Place a new body on `day`; it first updates `delay` days later (1..=`MAX_INTERVAL`).
    pub(crate) fn add(&mut self, mut b: Body, day: u64, delay: u64) -> usize {
        let slot = free_place(&mut self.bodies, &mut self.free);
        (b.gen, b.alive, b.last, b.count) = (self.bodies[slot].gen, true, day as f64, b.n());
        self.at[b.cell as usize].push(slot as u32);
        self.stir(b.cell);
        self.enter(&b);
        self.schedule(day + delay.clamp(1, MAX_INTERVAL as u64), slot as u32, b.gen);
        self.bodies[slot] = b;
        slot
    }
    /// Body `b`, taken out of `slot`, leaves the world: its slot is free, for a body of the next generation.
    pub(crate) fn remove(&mut self, slot: usize, b: &Body) {
        let cell = b.cell as usize;
        self.leave(b);
        self.bodies[slot] = Body { gen: b.gen + 1, ..Default::default() };
        self.at[cell].retain(|&s| s as usize != slot);
        self.stir(cell as u32);
        self.free.push(slot as u32);
    }
    /// Count body `b` into its lineage and its born rows.
    fn enter(&mut self, b: &Body) {
        self.members[b.lineage as usize] += 1;
        self.genomes[b.genome as usize].refs += 1;
    }
    /// Count body `b` out of its born rows and its lineage (which dies out with its last body).
    fn leave(&mut self, b: &Body) {
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
    /// Body `b` (taken out of its slot) is now born with `rows` (its own rows are the caller's).
    pub(crate) fn reborn(&mut self, b: &mut Body, rows: Vec<f32>) {
        if rows == self.genomes[b.genome as usize].rows {
            return;
        }
        let g = self.new_genome(rows);
        self.genomes[g as usize].refs += 1;
        self.unref(b.genome);
        b.genome = g;
    }
    /// Body `b` (taken out of its slot) moves to `lineage`, born with `rows`.
    pub(crate) fn move_to(&mut self, b: &mut Body, lineage: u32, rows: Vec<f32>) {
        self.leave(b);
        (b.lineage, b.genome) = (lineage, self.new_genome(rows));
        self.enter(b);
    }
    /// The bodies of `cell` changed: the survey shares out its light again (`survey::shade`).
    pub(crate) fn stir(&mut self, cell: u32) {
        if let Some(x) = self.stale.get_mut(cell as usize) {
            *x = true;
        }
    }
    /// Size the cell lists and the ring of updates for a map of `n` cells (once, on a new map).
    pub(crate) fn fit_map(&mut self, n: usize) {
        if self.at.len() != n {
            (self.at, self.due) = (vec![Vec::new(); n], vec![Vec::new(); RING as usize]);
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
    /// Body `slot` moves from cell `from` to `to`.
    pub fn relocate(&mut self, slot: usize, from: usize, to: usize) {
        self.at[from].retain(|&s| s as usize != slot);
        self.at[to].push(slot as u32);
        self.stir(from as u32);
        self.stir(to as u32);
    }
}

impl World {
    /// Place body `c`, its first update after its interval; its visible traits are worked out from what it holds now,
    /// so others see it before its first update.
    pub(crate) fn place(&mut self, mut c: Body) -> usize {
        c.prepare(&self.chem, &self.cfg.phys);
        self.see(&mut c);
        let delay = c.interval(self);
        self.life.add(c, self.day(), delay)
    }
    /// Place body `b` made outside the laws (a founder, an edit), its first update `delay` days on.
    pub fn add(&mut self, mut b: Body, delay: u64) -> usize {
        b.prepare(&self.chem, &self.cfg.phys);
        self.life.add(b, self.day(), delay)
    }
    /// What others see of body `b` is worked out again (`Body::visible`), and its cell's light shared out again.
    pub(crate) fn see(&mut self, b: &mut Body) {
        b.vis = b.visible(&self.chem, &self.cfg);
        self.life.stir(b.cell);
    }
}

/// A free place in `v` (the last one freed first, `free`), or a new one at its end.
pub(crate) fn free_place<T: Default>(v: &mut Vec<T>, free: &mut Vec<u32>) -> usize {
    free.pop().map(|i| i as usize).unwrap_or_else(|| {
        v.push(T::default());
        v.len() - 1
    })
}
