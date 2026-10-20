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
    pub bodies: Bodies,
    pub(crate) free: Vec<u32>,
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
    pub(crate) free_genomes: Vec<u32>,
    /// While a block of cells runs apart from the others of its wave (`waves`): where its new bodies, born rows and
    /// lineages go
    #[serde(skip)]
    pub(crate) pools: Option<Pools>,
    /// While a block runs on a world of its own (`waves`): what it does to the counts and the ring, which the shared
    /// world does after it, block by block
    #[serde(skip)]
    pub(crate) journal: Option<Vec<Op>>,
    /// The born rows and the lineages written since the worlds of the threads last copied them (`waves`)
    #[serde(skip)]
    pub(crate) written: (Vec<u32>, Vec<u32>),
}

/// The places a block of cells takes new entries from while it runs apart from the blocks of its wave (`waves`): first
/// the free places handed to it, then every `stride`-th place from `next` on, so blocks that run at once never take the
/// same place and the places do not depend on which block ran first.
#[derive(Clone, Debug, Default)]
pub(crate) struct Pool {
    pub free: Vec<u32>,
    pub next: u32,
    pub stride: u32,
    /// How many places from `next` on it took, and every place it took
    pub taken: u32,
    pub used: Vec<u32>,
}

impl Pool {
    /// The place of a new entry of `v`, which grows to hold it.
    fn place<T: Default>(&mut self, v: &mut Vec<T>) -> usize {
        let i = self.free.pop().unwrap_or_else(|| {
            self.taken += 1;
            self.next + (self.taken - 1) * self.stride
        }) as usize;
        if i >= v.len() {
            v.resize_with(i + 1, T::default);
        }
        self.used.push(i as u32);
        i
    }
}

/// The pools of a block (`Pool`): of body slots, born rows and lineages.
#[derive(Clone, Debug, Default)]
pub(crate) struct Pools {
    pub bodies: Pool,
    pub genomes: Pool,
    pub lineages: Pool,
}

/// A change of the counts of lineages and born rows, or of the ring, as a block on a world of its own records it.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Op {
    /// A body of lineage .0 born with rows .1 enters (`Life::enter`), or leaves
    Enter(u32, u32),
    Leave(u32, u32),
    /// One holder more, or fewer, of born rows
    Ref(u32),
    Unref(u32),
    /// An update of (slot, generation) due on a day
    Due(u64, u32, u32),
}

/// The bodies by slot, each in a box of its own, so the bodies of a block of cells move to the world of a thread and
/// back by their boxes (`waves`). A slot holds a body or the dead body of its generation, or nothing: a new place, and
/// in the world of a thread a slot out of the block's reach. Saved as the list of the bodies by slot.
#[derive(Clone, Debug, Default)]
pub struct Bodies {
    slots: Vec<Option<Box<Body>>>,
    /// What a slot holding nothing reads as: no body, of the first generation
    nobody: Body,
}

impl Bodies {
    pub fn len(&self) -> usize {
        self.slots.len()
    }
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
    pub fn get(&self, s: usize) -> Option<&Body> {
        self.slots.get(s).map(|b| b.as_deref().unwrap_or(&self.nobody))
    }
    /// The bodies by slot.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &Body> {
        self.slots.iter().map(|b| b.as_deref().unwrap_or(&self.nobody))
    }
    /// The bodies held, in the order of their slots.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Body> {
        self.slots.iter_mut().flatten().map(|b| &mut **b)
    }
    /// Body `b` goes to slot `s`, in the box that held the dead body there if there was one.
    fn set(&mut self, s: usize, b: Body) {
        match &mut self.slots[s] {
            Some(x) => **x = b,
            x => *x = Some(Box::new(b)),
        }
    }
    /// The box of slot `s`, taken out (`waves`).
    #[cfg(feature = "threads")]
    pub(crate) fn take(&mut self, s: usize) -> Option<Box<Body>> {
        self.slots.get_mut(s).and_then(Option::take)
    }
    /// Box `b` goes to slot `s`, which the list grows to hold (`waves`).
    #[cfg(feature = "threads")]
    pub(crate) fn put(&mut self, s: usize, b: Option<Box<Body>>) {
        if s >= self.slots.len() {
            self.slots.resize_with(s + 1, || None);
        }
        self.slots[s] = b;
    }
}

impl std::ops::Index<usize> for Bodies {
    type Output = Body;
    fn index(&self, s: usize) -> &Body {
        self.slots[s].as_deref().unwrap_or(&self.nobody)
    }
}

/// A law changes only a slot that holds a body or a dead body.
impl std::ops::IndexMut<usize> for Bodies {
    fn index_mut(&mut self, s: usize) -> &mut Body {
        self.slots[s].as_deref_mut().expect("a slot that holds a body")
    }
}

impl Serialize for Bodies {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_seq(self.iter())
    }
}

impl<'de> Deserialize<'de> for Bodies {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let slots = Vec::<Body>::deserialize(d)?.into_iter().map(|b| Some(Box::new(b))).collect();
        Ok(Bodies { slots, nobody: Body::default() })
    }
}

/// Born brain rows and how many hold them: the living bodies born with them (most children share their parent's, `copy`).
/// The rows are shared, not copied, by the worlds of the threads (`waves`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Genome {
    pub rows: std::sync::Arc<[f32]>,
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
        let i = match &mut self.pools {
            Some(p) => p.lineages.place(&mut self.brains),
            None => free_place(&mut self.brains, &mut Vec::new(), None),
        };
        self.members.resize(self.brains.len(), 0);
        self.brains[i] = Some(l);
        self.written.1.push(i as u32);
        i as u32
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
        let i = free_place(&mut self.genomes, &mut self.free_genomes, self.pools.as_mut().map(|p| &mut p.genomes));
        self.genomes[i] = Genome { rows: rows.into(), refs: 0 };
        self.written.0.push(i as u32);
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
        let slot = free_place(&mut self.bodies.slots, &mut self.free, self.pools.as_mut().map(|p| &mut p.bodies));
        (b.gen, b.alive, b.last, b.count) = (self.bodies[slot].gen, true, day as f64, b.n());
        self.at[b.cell as usize].push(slot as u32);
        self.stir(b.cell);
        self.enter(&b);
        self.schedule(day + delay.clamp(1, MAX_INTERVAL as u64), slot as u32, b.gen);
        self.bodies.set(slot, b);
        slot
    }
    /// Body `b`, taken out of `slot`, leaves the world: its slot is free, for a body of the next generation.
    pub(crate) fn remove(&mut self, slot: usize, b: &Body) {
        let cell = b.cell as usize;
        self.leave(b);
        self.bodies[slot] = Body { gen: b.gen + 1, ..Default::default() };
        self.at[cell].retain(|&s| s as usize != slot);
        self.stir(cell as u32);
        // (A slot freed in a block goes back to its pool, for the block's next body.)
        match &mut self.pools {
            Some(p) => p.bodies.free.push(slot as u32),
            None => self.free.push(slot as u32),
        }
    }
    /// Count body `b` into its lineage and its born rows.
    fn enter(&mut self, b: &Body) {
        self.record(Op::Enter(b.lineage, b.genome));
    }
    /// Count body `b` out of its born rows and its lineage (which dies out with its last body).
    fn leave(&mut self, b: &Body) {
        self.record(Op::Leave(b.lineage, b.genome));
    }
    /// Record `op`: in the journal while a block runs on a world of its own, else counted at once.
    fn record(&mut self, op: Op) {
        match &mut self.journal {
            Some(j) => j.push(op),
            None => self.count(op),
        }
    }
    /// Do `op` to the counts and the ring: born rows that no body holds any more are free, and a lineage dies out with
    /// its last body.
    pub(crate) fn count(&mut self, op: Op) {
        match op {
            Op::Enter(lin, g) => {
                self.members[lin as usize] += 1;
                self.genomes[g as usize].refs += 1;
            }
            Op::Leave(lin, g) => {
                self.count(Op::Unref(g));
                let lin = lin as usize;
                self.members[lin] -= 1;
                if self.members[lin] == 0 {
                    self.brains[lin] = None;
                }
            }
            Op::Ref(g) => self.genomes[g as usize].refs += 1,
            Op::Unref(g) => {
                let gen = &mut self.genomes[g as usize];
                gen.refs -= 1;
                if gen.refs == 0 {
                    gen.rows = Default::default();
                    self.free_genomes.push(g);
                }
            }
            Op::Due(day, slot, gen) => self.due[(day % RING) as usize].push((slot, gen)),
        }
    }
    /// Body `b` (taken out of its slot) is now born with `rows` (its own rows are the caller's).
    pub(crate) fn reborn(&mut self, b: &mut Body, rows: Vec<f32>) {
        if *rows == *self.genomes[b.genome as usize].rows {
            return;
        }
        let g = self.new_genome(rows);
        self.record(Op::Ref(g));
        self.record(Op::Unref(b.genome));
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
        self.record(Op::Due(day, slot, gen));
    }
    /// The updates filed for `day`, taken out of the ring.
    pub(crate) fn take_due(&mut self, day: u64) -> Vec<(u32, u32)> {
        std::mem::take(&mut self.due[(day % RING) as usize])
    }
    /// The survey's caches as `from` holds them (`waves`). Every field is named, so a new one must be placed here.
    #[cfg(feature = "threads")]
    pub(crate) fn copy_caches(&mut self, from: &Life) {
        let Life {
            sum, absorb, shade, keep, env, seen, cellf,
            bodies: _, free: _, at: _, stale: _, due: _, brains: _, members: _, genomes: _, free_genomes: _,
            pools: _, journal: _, written: _,
        } = from;
        self.sum.clone_from(sum);
        self.absorb.clone_from(absorb);
        self.shade.clone_from(shade);
        self.keep.clone_from(keep);
        self.env.clone_from(env);
        self.seen.clone_from(seen);
        self.cellf.clone_from(cellf);
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

/// A place in `v` for a new entry: from the pool of the block running (`pool`), else a free place (the last one freed
/// first, `free`) or a new one at its end.
pub(crate) fn free_place<T: Default>(v: &mut Vec<T>, free: &mut Vec<u32>, pool: Option<&mut Pool>) -> usize {
    if let Some(p) = pool {
        return p.place(v);
    }
    free.pop().map(|i| i as usize).unwrap_or_else(|| {
        v.push(T::default());
        v.len() - 1
    })
}
