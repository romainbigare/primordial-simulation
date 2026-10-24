//! The registry of bodies: their storage, ids and cells, lineages, born rows, and the rings of updates due. The cells are
//! cut into domains, the cells under one air node (`Domain`); each domain stores the bodies of its cells, their lists
//! and its ring. A body's id is its domain of birth and that domain's count of births, and never changes, wherever the
//! body is stored; a body that moves to a cell of another domain moves to that domain's storage.

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

/// Where a body is stored: its domain and its slot there (`Life`'s index). It holds until the body leaves the world or
/// moves to another domain (`Life::put`); a body is named for longer by its id.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Handle {
    pub domain: u32,
    pub slot: u32,
}

/// The cells under one air node, the bodies born there, and the bodies of its cells: their slots (the last freed taken
/// first), each cell's list and the ring of their updates.
#[derive(Clone, Debug, Default)]
struct Domain {
    /// Its cells, in order (`Level::children` of its air node)
    cells: Vec<u32>,
    /// Bodies born in it so far
    births: u64,
    bodies: Vec<Body>,
    free: Vec<u32>,
    /// The bodies of each of its cells (by their order in `cells`), in the order they came
    at: Vec<Vec<Handle>>,
    /// Updates due, by day modulo RING: body ids (`Body::due`)
    due: Vec<Vec<u64>>,
}

#[derive(Clone, Debug, Default)]
pub struct Life {
    domains: Vec<Domain>,
    /// Each cell's domain and its place in that domain's cells
    place: Vec<(u32, u32)>,
    /// Where each body is stored, by id
    table: BTreeMap<u64, Handle>,
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
        let bodies = |d: &Domain| d.at.iter().flatten().map(|&h| &self[h]).filter(|b| b.alive).collect();
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
    /// The living bodies with their handles, in a fixed order that does not depend on where they are stored: domain by
    /// domain, cell by cell, each cell's in the order they came.
    pub fn living(&self) -> impl Iterator<Item = (Handle, &Body)> {
        self.domains.iter().flat_map(|d| d.at.iter().flatten()).map(|&h| (h, &self[h])).filter(|x| x.1.alive)
    }
    /// The living bodies, in the order of `living`.
    pub fn alive(&self) -> impl Iterator<Item = &Body> {
        self.living().map(|x| x.1)
    }
    /// Run `f` on each living body, in the order they are stored.
    pub fn each_mut(&mut self, f: impl FnMut(&mut Body)) {
        self.domains.iter_mut().flat_map(|d| &mut d.bodies).filter(|b| b.alive).for_each(f);
    }
    /// The list of the bodies of cell `i`, in the order they came.
    fn list(&self, i: usize) -> &Vec<Handle> {
        let (d, k) = self.place[i];
        &self.domains[d as usize].at[k as usize]
    }
    /// The list of the bodies of cell `i`, to change.
    fn list_mut(&mut self, i: usize) -> &mut Vec<Handle> {
        let (d, k) = self.place[i];
        &mut self.domains[d as usize].at[k as usize]
    }
    /// The handles of the bodies of cell `i`, in the order they came.
    pub fn cell(&self, i: usize) -> impl Iterator<Item = Handle> + '_ {
        self.list(i).iter().copied()
    }
    /// Run `f` on each body of cell `i`, in the order they came.
    pub(crate) fn for_cell(&mut self, i: usize, mut f: impl FnMut(&mut Body)) {
        for k in 0..self.in_cell(i) {
            let h = self.list(i)[k];
            f(&mut self[h]);
        }
    }
    /// How many bodies cell `i` holds.
    pub fn in_cell(&self, i: usize) -> usize {
        self.list(i).len()
    }
    /// Where the body with id `id` is stored, if it lives.
    pub fn find(&self, id: u64) -> Option<Handle> {
        self.table.get(&id).copied()
    }
    /// Whether the body with id `id` is still stored at `h` (a body seen earlier may have left the world since, and its
    /// slot taken by another).
    pub(crate) fn holds(&self, h: Handle, id: u64) -> bool {
        self[h].alive && self[h].id == id
    }
    /// The domain of cell `i`.
    pub fn domain_of(&self, i: usize) -> usize {
        self.place[i].0 as usize
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
    pub(crate) fn add(&mut self, mut b: Body, day: u64, delay: u64) -> Handle {
        let d = self.place[b.cell as usize].0;
        let births = &mut self.domains[d as usize].births;
        *births += 1;
        (b.id, b.alive, b.last, b.count, b.shade) = ((d as u64) << ID_SHIFT | *births, true, day as f64, b.n(), 1.0);
        b.due = day + delay.clamp(1, MAX_INTERVAL as u64);
        self.enter(&b);
        self.store(b)
    }
    /// Store body `b` (counted in its lineage and born rows, its id and next update set) in the domain of its cell: in a
    /// free slot, its cell's list, the id table and the ring.
    fn store(&mut self, b: Body) -> Handle {
        let h = self.slot_for(b.cell);
        self.list_mut(b.cell as usize).push(h);
        self.file(h.domain, &b);
        self.table.insert(b.id, h);
        self.stir(b.cell);
        self[h] = b;
        h
    }
    /// A free slot in the domain of `cell`.
    fn slot_for(&mut self, cell: u32) -> Handle {
        let d = self.place[cell as usize].0;
        let x = &mut self.domains[d as usize];
        Handle { domain: d, slot: free_place(&mut x.bodies, &mut x.free) as u32 }
    }
    /// File body `b`'s next update in the ring of domain `d`.
    fn file(&mut self, d: u32, b: &Body) {
        self.domains[d as usize].due[(b.due % RING) as usize].push(b.id);
    }
    /// Body `b`, taken out of `h`, leaves the world: its slot is free.
    pub(crate) fn remove(&mut self, h: Handle, b: &Body) {
        self.leave(b);
        self.table.remove(&b.id);
        self.list_mut(b.cell as usize).retain(|&x| x != h);
        self.stir(b.cell);
        let x = &mut self.domains[h.domain as usize];
        x.bodies[h.slot as usize] = Body::default();
        x.free.push(h.slot);
    }
    /// Body `b`, taken out of `h`, is put back. If it moved into a cell of another domain (`relocate`), it moves to that
    /// domain's storage, keeping its id and its place in its cell's list, its update due filed there; returns where it is.
    pub fn put(&mut self, h: Handle, b: Body) -> Handle {
        if self.place[b.cell as usize].0 == h.domain {
            self[h] = b;
            return h;
        }
        let (to, from) = (self.slot_for(b.cell), &mut self.domains[h.domain as usize]);
        from.free.push(h.slot);
        let ring = &mut from.due[(b.due % RING) as usize];
        if let Some(k) = ring.iter().position(|&x| x == b.id) {
            ring.swap_remove(k);
            self.file(to.domain, &b);
        }
        let list = self.list_mut(b.cell as usize);
        list.iter_mut().filter(|x| **x == h).for_each(|x| *x = to);
        self.table.insert(b.id, to);
        self[to] = b;
        to
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
    /// The domains are `cells` (each a list of cells; together every cell once): their cell lists and rings sized.
    fn cut(&mut self, cells: Vec<Vec<u32>>) {
        self.place = vec![(0, 0); cells.iter().map(|c| c.len()).sum()];
        for (d, c) in cells.iter().enumerate() {
            c.iter().enumerate().for_each(|(k, &i)| self.place[i as usize] = (d as u32, k as u32));
        }
        let domain = |cells: Vec<u32>| Domain { at: vec![Vec::new(); cells.len()], due: vec![Vec::new(); RING as usize], cells, ..Default::default() };
        self.domains = cells.into_iter().map(domain).collect();
    }
    /// File the next update of body `b` on `day`, in the ring of its cell's domain.
    pub(crate) fn schedule(&mut self, day: u64, b: &mut Body) {
        b.due = day;
        self.file(self.place[b.cell as usize].0, b);
    }
    /// The ids of the bodies due on `day`, taken out of the rings, domain by domain (some may have left the world since).
    pub(crate) fn take_due(&mut self, day: u64) -> Vec<u64> {
        let k = (day % RING) as usize;
        self.domains.iter_mut().flat_map(|d| std::mem::take(&mut d.due[k])).collect()
    }
    /// The body with id `id` moves from cell `from` to `to` (its cell is the caller's to set; its storage moves when it
    /// is put back, `put`).
    pub fn relocate(&mut self, id: u64, from: usize, to: usize) {
        let h = self.table[&id];
        self.list_mut(from).retain(|&x| x != h);
        self.list_mut(to).push(h);
        self.stir(from as u32);
        self.stir(to as u32);
    }
}

impl std::ops::Index<Handle> for Life {
    type Output = Body;
    fn index(&self, h: Handle) -> &Body {
        &self.domains[h.domain as usize].bodies[h.slot as usize]
    }
}

impl std::ops::IndexMut<Handle> for Life {
    fn index_mut(&mut self, h: Handle) -> &mut Body {
        &mut self.domains[h.domain as usize].bodies[h.slot as usize]
    }
}

impl World {
    /// Place body `c`, its first update after its interval; its visible traits are worked out from what it holds now,
    /// so others see it before its first update.
    pub(crate) fn place(&mut self, mut c: Body) -> Handle {
        c.prepare(&self.chem, &self.cfg.phys);
        self.see(&mut c);
        let delay = c.interval(self);
        self.life.add(c, self.day(), delay)
    }
    /// Place body `b` made outside the laws (a founder, an edit), its first update `delay` days on.
    pub fn add(&mut self, mut b: Body, delay: u64) -> Handle {
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
