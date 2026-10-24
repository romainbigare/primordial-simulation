//! The registry of bodies: their storage, ids and cells, lineages, born rows, and the ring of updates due. The cells are
//! cut into domains, the cells under one air node (`Domain`); a body's id is its domain of birth and that domain's count
//! of births, and never changes, wherever the body is stored.

use crate::body::{Body, MAX_INTERVAL};
use crate::brain::Shape;
use crate::lineage::Lineage;
use crate::space::Mesh;
use crate::{World, AIR};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

/// Updates are kept in a ring of this many days (more than the longest interval).
const RING: u64 = 384;
/// A body's id is its domain of birth shifted left by this many bits, plus that domain's count of births then (from 1,
/// so no body has id 0).
const ID_SHIFT: u32 = 40;

/// The cells under one air node, and the bodies born there.
#[derive(Clone, Debug, Default)]
struct Domain {
    /// Its cells, in order (`Level::children` of its air node)
    cells: Vec<u32>,
    /// Bodies born in it so far
    births: u64,
}

#[derive(Clone, Debug, Default)]
pub struct Life {
    bodies: Vec<Body>,
    free: Vec<u32>,
    /// Bodies in each cell, in the order they came
    at: Vec<Vec<u32>>,
    domains: Vec<Domain>,
    /// The domain of each cell
    domain_of: Vec<u32>,
    /// Each body's slot, by id
    table: BTreeMap<u64, u32>,
    /// Per cell, from the last survey: members, then their visible traits summed, then the energy to break all its
    /// bodies, J. The survey's caches are not saved: a snapshot is taken at a month's start, where the survey builds
    /// them before any law reads them; this one the tape reads first, so `tape::load_snapshot` rebuilds it
    /// (`survey::sums`).
    pub sum: Vec<[f64; 8]>,
    /// Per cell: the energy movers spent pushing through its bodies since the last survey, which then breaks them, J
    /// (`choose::wear`; empty: none)
    pub pushed: Vec<f64>,
    /// Per cell, from the last survey: the share of the light reaching its bodies' capture area that they absorb
    pub absorb: Vec<f32>,
    /// Per cell: whether its bodies changed since the survey last shared out its light (`Life::stir`; empty: all)
    pub stale: Vec<bool>,
    /// Per cell, from the last survey: the share of that absorbed light set aside for its bodies' light reactions
    pub keep: Vec<f32>,
    /// Per cell, from the last monthly survey: ground temperature, liquid depth, surface height
    pub env: Vec<[f64; 3]>,
    /// Per cell, from the last monthly survey: its loose substances as a brain perceives them, [cell * substances + k]
    pub seen: Vec<f32>,
    /// Per cell, from the last survey: what a brain perceives of it as a place to move to (`choose::cells`)
    pub cellf: Vec<[f32; crate::choose::NF]>,
    /// Updates due, by day modulo RING: body ids (`Body::due`)
    due: Vec<Vec<u64>>,
    /// The brain of each lineage (None once it died out), and its living bodies
    pub brains: Vec<Option<Lineage>>,
    pub members: Vec<u32>,
    /// Born brain rows, each shared by the bodies born with them (none: free)
    pub genomes: Vec<Genome>,
    free_genomes: Vec<u32>,
}

/// What a snapshot keeps of `Life`: per domain its cells, its births and its living bodies in the order of `Life::alive`;
/// the energy movers spent in each cell; the lineages and born rows. Where bodies are stored, the ring and the id table
/// follow from the bodies, and the survey's caches are rebuilt (`Life::sum`).
#[derive(Serialize)]
struct Saved<'a> {
    domains: Vec<(&'a [u32], u64, Vec<&'a Body>)>,
    pushed: &'a [f64],
    brains: &'a [Option<Lineage>],
    members: &'a [u32],
    genomes: &'a [Genome],
    free_genomes: &'a [u32],
}

/// `Saved` as it is read back.
#[derive(Deserialize)]
struct Loaded {
    domains: Vec<(Vec<u32>, u64, Vec<Body>)>,
    pushed: Vec<f64>,
    brains: Vec<Option<Lineage>>,
    members: Vec<u32>,
    genomes: Vec<Genome>,
    free_genomes: Vec<u32>,
}

impl Serialize for Life {
    fn serialize<S: Serializer>(&self, to: S) -> Result<S::Ok, S::Error> {
        let bodies = |d: &Domain| d.cells.iter().flat_map(|&c| self.cell(c as usize)).filter(|&s| self.bodies[s].alive).map(|s| &self.bodies[s]).collect();
        let domains = self.domains.iter().map(|d| (&d.cells[..], d.births, bodies(d))).collect();
        let (pushed, brains, members, genomes, free_genomes) = (&self.pushed[..], &self.brains[..], &self.members[..], &self.genomes[..], &self.free_genomes[..]);
        Saved { domains, pushed, brains, members, genomes, free_genomes }.serialize(to)
    }
}

/// A loaded world stores its bodies anew, in the order saved; the caches come back empty.
impl<'de> Deserialize<'de> for Life {
    fn deserialize<D: Deserializer<'de>>(from: D) -> Result<Self, D::Error> {
        let x = Loaded::deserialize(from)?;
        let mut life = Life { pushed: x.pushed, brains: x.brains, members: x.members, genomes: x.genomes, free_genomes: x.free_genomes, ..Default::default() };
        life.cut(x.domains.iter().map(|d| d.0.clone()).collect());
        for (d, (_, births, bodies)) in x.domains.into_iter().enumerate() {
            life.domains[d].births = births;
            for b in bodies {
                life.store(b);
            }
        }
        Ok(life)
    }
}

/// Born brain rows and how many hold them: the living bodies born with them (most children share their parent's, `copy`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Genome {
    pub rows: Vec<f32>,
    pub refs: u32,
}

impl Life {
    /// The living bodies with their slots, in a fixed order that does not depend on where they are stored: domain by
    /// domain, cell by cell, each cell's in the order they came.
    pub fn living(&self) -> impl Iterator<Item = (usize, &Body)> {
        let cells = self.domains.iter().flat_map(|d| &d.cells);
        cells.flat_map(|&c| self.cell(c as usize)).map(|s| (s, &self.bodies[s])).filter(|x| x.1.alive)
    }
    /// The living bodies, in the order of `living`.
    pub fn alive(&self) -> impl Iterator<Item = &Body> {
        self.living().map(|x| x.1)
    }
    /// Run `f` on each living body, in the order they are stored.
    pub fn each_mut(&mut self, f: impl FnMut(&mut Body)) {
        self.bodies.iter_mut().filter(|b| b.alive).for_each(f);
    }
    /// The slots of the bodies of cell `i`, in the order they came.
    pub fn cell(&self, i: usize) -> impl Iterator<Item = usize> + '_ {
        self.at[i].iter().map(|&s| s as usize)
    }
    /// Run `f` on each body of cell `i`, in the order they came.
    pub(crate) fn for_cell(&mut self, i: usize, mut f: impl FnMut(&mut Body)) {
        self.at[i].iter().for_each(|&s| f(&mut self.bodies[s as usize]));
    }
    /// How many bodies cell `i` holds.
    pub fn in_cell(&self, i: usize) -> usize {
        self.at[i].len()
    }
    /// The slot of the body with id `id`, if it lives.
    pub fn find(&self, id: u64) -> Option<usize> {
        self.table.get(&id).map(|&s| s as usize)
    }
    /// Whether the body with id `id` still lives in `slot` (a body seen earlier may have left the world since, and its
    /// slot taken by another).
    pub(crate) fn holds(&self, slot: usize, id: u64) -> bool {
        self.bodies[slot].alive && self.bodies[slot].id == id
    }
    /// The domain of cell `i`.
    pub fn domain_of(&self, i: usize) -> usize {
        self.domain_of[i] as usize
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
    /// Place a new body on `day`, born in the domain of its cell; it first updates `delay` days later
    /// (1..=`MAX_INTERVAL`), and has its cell's mean light until the survey shares it out again (`survey::shade`).
    pub(crate) fn add(&mut self, mut b: Body, day: u64, delay: u64) -> usize {
        let d = self.domain_of[b.cell as usize];
        let births = &mut self.domains[d as usize].births;
        *births += 1;
        (b.id, b.alive, b.last, b.count, b.shade) = ((d as u64) << ID_SHIFT | *births, true, day as f64, b.n(), 1.0);
        b.due = day + delay.clamp(1, MAX_INTERVAL as u64);
        self.enter(&b);
        self.store(b)
    }
    /// Store body `b` (counted in its lineage and born rows, its id and next update set): in a free slot, its cell's
    /// list, the id table and the ring.
    fn store(&mut self, b: Body) -> usize {
        let slot = free_place(&mut self.bodies, &mut self.free);
        self.at[b.cell as usize].push(slot as u32);
        self.table.insert(b.id, slot as u32);
        self.due[(b.due % RING) as usize].push(b.id);
        self.stir(b.cell);
        self.bodies[slot] = b;
        slot
    }
    /// Body `b`, taken out of `slot`, leaves the world: its slot is free.
    pub(crate) fn remove(&mut self, slot: usize, b: &Body) {
        let cell = b.cell as usize;
        self.leave(b);
        self.table.remove(&b.id);
        self.bodies[slot] = Body::default();
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
    /// No bodies yet on `mesh`: a domain under each air node, with its cells.
    pub fn new(mesh: &Mesh) -> Life {
        let air = mesh.level(AIR);
        let mut life = Life::default();
        life.cut((0..air.n()).map(|j| air.children(j).to_vec()).collect());
        life
    }
    /// The domains are `cells` (each a list of cells; together every cell once): their cell lists and the ring sized.
    fn cut(&mut self, cells: Vec<Vec<u32>>) {
        let n = cells.iter().map(|c| c.len()).sum();
        (self.at, self.domain_of, self.due) = (vec![Vec::new(); n], vec![0; n], vec![Vec::new(); RING as usize]);
        for (d, c) in cells.iter().enumerate() {
            c.iter().for_each(|&i| self.domain_of[i as usize] = d as u32);
        }
        self.domains = cells.into_iter().map(|cells| Domain { cells, births: 0 }).collect();
    }
    /// File the next update of body `b` on `day`.
    pub(crate) fn schedule(&mut self, day: u64, b: &mut Body) {
        b.due = day;
        self.due[(day % RING) as usize].push(b.id);
    }
    /// The ids of the bodies due on `day`, taken out of the ring (some may have left the world since).
    pub(crate) fn take_due(&mut self, day: u64) -> Vec<u64> {
        std::mem::take(&mut self.due[(day % RING) as usize])
    }
    /// The body with id `id` moves from cell `from` to `to` (its cell is the caller's to set).
    pub fn relocate(&mut self, id: u64, from: usize, to: usize) {
        let slot = self.table[&id];
        self.at[from].retain(|&s| s != slot);
        self.at[to].push(slot);
        self.stir(from as u32);
        self.stir(to as u32);
    }
}

impl std::ops::Index<usize> for Life {
    type Output = Body;
    fn index(&self, slot: usize) -> &Body {
        &self.bodies[slot]
    }
}

impl std::ops::IndexMut<usize> for Life {
    fn index_mut(&mut self, slot: usize) -> &mut Body {
        &mut self.bodies[slot]
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
