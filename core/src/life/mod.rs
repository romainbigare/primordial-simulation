//! The registry of bodies: their storage, ids and cells, lineages and born rows (`registry`), the rings of updates due
//! and the outboxes. The cells are cut into domains, the cells under one air node (`domain::Domain`); each domain
//! stores the bodies of its cells, their lists, its ring and its outbox, and finds them by id. A body's id is the domain
//! it was placed in and that domain's count of bodies placed; it never changes, wherever the body is stored. A body that
//! moves to a cell of another domain moves to that domain's storage.

mod domain;
mod registry;

pub use domain::Domain;
pub use registry::Genome;
pub(crate) use registry::Held;

use crate::body::{Body, MAX_INTERVAL};
use crate::lineage::Lineage;
use crate::outbox::Entry;
use crate::space::Mesh;
use crate::{World, AIR};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

/// Updates are kept in a ring of this many batches: more than the longest interval is in batches (`MAX_INTERVAL`) with
/// a batch of one day, the shortest.
const RING: u64 = 384;
/// A body's id is the domain it was placed in shifted left by this many bits, plus that domain's count of bodies placed
/// then (from 1, so no body has id 0; it stays below 2^this, `Domain::add`).
const ID_SHIFT: u32 = 40;

/// Where a body is stored: its domain and its slot there (`Life`'s index). It holds until the body leaves the world or
/// moves to another domain (`Life::relocate`); a body is named for longer by its id.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Handle {
    pub domain: u32,
    pub slot: u32,
}

#[derive(Clone, Debug, Default)]
pub struct Life {
    /// The domains (taken out of the world while body code runs on them, `World::in_domain`)
    pub(crate) domains: Vec<Domain>,
    /// Each cell's domain and its index in that domain's cells
    cell_in: Vec<(u32, u32)>,
    /// The domain each body stored outside the domain it was placed in is stored in, by id (`find`; a body that died
    /// there is left out at the end of each batch, `settle_moved`)
    away: BTreeMap<u64, u32>,
    /// Per cell, from the last survey: members, then their visible traits summed, then the energy to break all its
    /// bodies, J; then their capture area times their shade, m², and that times the share set aside for their light
    /// reactions. The survey's caches are not saved: a snapshot is taken at a month's start, where the survey builds
    /// them before any law reads them; this one the tape reads first, so `tape::load_snapshot` rebuilds it
    /// (`survey::sums`).
    pub sum: Vec<[f64; crate::survey::ROW]>,
    /// Per cell: the energy movers spent pushing through its bodies since the last survey, which then breaks them, J
    /// (`choose::wear`; empty: none)
    pub pushed: Vec<f64>,
    /// Per cell, from the last survey: the share of the light reaching its bodies' capture area that they absorb
    pub absorb: Vec<f32>,
    /// Per cell, from the last survey: the share of that absorbed light set aside for its bodies' light reactions
    pub keep: Vec<f32>,
    /// Per cell, from the last monthly survey: ground temperature, liquid depth, surface height
    pub env: Vec<[f64; 3]>,
    /// Per cell, from the last monthly survey: its loose substances as a brain perceives them, [cell * substances + k]
    pub seen: Vec<f32>,
    /// Per cell, from the last survey: what a brain perceives of it as a place to move to (`choose::cells`)
    pub cellf: Vec<[f32; crate::choose::NF]>,
    /// What each living body showed others at the start of the batch running, cell by cell (`survey::Faces`; empty
    /// outside the batches)
    pub faces: crate::survey::Faces,
    /// The brain of each lineage (None once it died out), and its living bodies
    pub brains: Vec<Option<Lineage>>,
    pub members: Vec<u32>,
    /// Born brain rows, each shared by the bodies born with them (none: free)
    pub genomes: Vec<Genome>,
    free_genomes: Vec<u32>,
    /// A batch runs: the changes of the holders of lineages and born rows wait in the domains until it ends (`count_all`)
    pub(crate) batch: bool,
}

/// What a snapshot keeps of `Life`: per domain its cells, its count of bodies placed and its living bodies in the order of `Life::alive`;
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
        let domains = self.domains.iter().map(|d| (&d.cells[..], d.placed, bodies(d))).collect();
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
        for (d, (_, placed, bodies)) in x.domains.into_iter().enumerate() {
            life.domains[d].placed = placed;
            for b in bodies {
                if (b.id >> ID_SHIFT) as usize != d {
                    life.away.insert(b.id, d as u32);
                }
                life.domains[d].ring[(b.due % RING) as usize].push(b.id);
                life.domains[d].store(b);
            }
        }
        Ok(life)
    }
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
    /// Run `f` on each living body, in the order they are stored, for work that does not depend on order.
    pub fn each_mut(&mut self, f: impl FnMut(&mut Body)) {
        self.domains.iter_mut().flat_map(|d| &mut d.bodies).filter(|b| b.alive).for_each(f);
    }

    /// The handles of the bodies of cell `i`, in the order they came.
    pub fn cell(&self, i: usize) -> impl ExactSizeIterator<Item = Handle> + '_ {
        let (d, k) = self.cell_slot(i);
        self.domains[d].at[k].iter().copied()
    }
    /// How many bodies cell `i` holds.
    pub fn in_cell(&self, i: usize) -> usize {
        self.cell(i).len()
    }
    /// Where the body with id `id` is stored, if it lives: in the domain it was placed in, unless it moved away.
    pub fn find(&self, id: u64) -> Option<Handle> {
        self.domains.get(self.away.get(&id).map_or((id >> ID_SHIFT) as usize, |&d| d as usize))?.find(id)
    }
    /// The first break of the storage, if any: a cell's list that holds a body of another cell or stored in another
    /// domain, an id table that points elsewhere, or a living body not filed exactly once in the rings, in its domain's
    /// at its due batch (`check::broken`).
    pub fn unsound(&self) -> Option<String> {
        let mut filed: BTreeMap<u64, usize> = BTreeMap::new();
        self.domains.iter().flat_map(|x| x.ring.iter().flatten()).for_each(|&id| *filed.entry(id).or_default() += 1);
        for (d, x) in self.domains.iter().enumerate() {
            for (&cell, list) in x.cells.iter().zip(&x.at) {
                for &h in list {
                    let b = &self[h];
                    let ring = x.ring[(b.due % RING) as usize].contains(&b.id) && filed.get(&b.id) == Some(&1);
                    if h.domain as usize != d || b.cell != cell || !b.alive || self.find(b.id) != Some(h) || !ring {
                        return Some(format!("body {} at {h:?} in the list of cell {cell} (domain {d}): its cell {}, filed {ring}", b.id, b.cell));
                    }
                }
            }
        }
        let listed: usize = self.domains.iter().map(|x| x.at.iter().map(|l| l.len()).sum::<usize>()).sum();
        let tables: usize = self.domains.iter().map(|x| x.table.len()).sum();
        (listed != tables).then(|| format!("{listed} bodies in the cells' lists, {tables} in the id tables"))
    }
    /// The domain of cell `i`, read outside body code (by the shell and the tests).
    pub fn domain(&self, i: usize) -> &Domain {
        &self.domains[self.domain_of(i)]
    }
    /// The domain of cell `i`.
    pub(crate) fn domain_of(&self, i: usize) -> usize {
        self.cell_in[i].0 as usize
    }
    /// The domain of cell `i` and its slot among that domain's cells (`Level::children` of its air node).
    #[inline]
    pub(crate) fn cell_slot(&self, i: usize) -> (usize, usize) {
        let (d, k) = self.cell_in[i];
        (d as usize, k as usize)
    }
    /// Body `b`, taken out of `h` in cell `from`, is stored again in its cell now (it moved there): at the end of its
    /// list, in the storage of its domain (where it is noted, `find`), its update due filed there; returns where it is.
    pub fn relocate(&mut self, h: Handle, from: usize, b: Body) -> Handle {
        let (d, due) = (self.domain_of(b.cell as usize), (b.due % RING) as usize);
        let x = &mut self.domains[h.domain as usize];
        x.unstore(h, b.id, from);
        if d != h.domain as usize {
            x.ring[due].retain(|&y| y != b.id);
            self.domains[d].ring[due].push(b.id);
        }
        if (b.id >> ID_SHIFT) as usize == d { self.away.remove(&b.id) } else { self.away.insert(b.id, d as u32) };
        self.domains[d].store(b)
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
        self.cell_in = vec![(0, 0); cells.iter().map(|c| c.len()).sum()];
        for (d, c) in cells.iter().enumerate() {
            c.iter().enumerate().for_each(|(k, &i)| self.cell_in[i as usize] = (d as u32, k as u32));
        }
        let domain = |(d, cells): (usize, Vec<u32>)| {
            let (at, stale, ring) = (vec![Vec::new(); cells.len()], vec![true; cells.len()], vec![Vec::new(); RING as usize]);
            Domain { index: d as u32, at, stale, ring, cells, ..Default::default() }
        };
        self.domains = cells.into_iter().enumerate().map(domain).collect();
    }
    /// The ids of the bodies due in `batch`, taken out of the rings, domain by domain (some may have left the world
    /// since).
    pub(crate) fn take_due(&mut self, batch: u64) -> Vec<u64> {
        let k = (batch % RING) as usize;
        self.domains.iter_mut().flat_map(|d| std::mem::take(&mut d.ring[k])).collect()
    }
    /// The entries of the outboxes, domain by domain, each's in the order made; they are left empty.
    pub(crate) fn take_queued(&mut self) -> Vec<Entry> {
        self.domains.iter_mut().flat_map(|d| std::mem::take(&mut d.out)).collect()
    }

    /// The bodies that moved away and died since are left out of `away`.
    pub(crate) fn settle_moved(&mut self) {
        let domains = &self.domains;
        self.away.retain(|id, d| domains[*d as usize].table.contains_key(id));
    }
}

impl std::ops::Index<Handle> for Life {
    type Output = Body;
    fn index(&self, h: Handle) -> &Body {
        &self.domains[h.domain as usize][h]
    }
}

impl std::ops::IndexMut<Handle> for Life {
    fn index_mut(&mut self, h: Handle) -> &mut Body {
        &mut self.domains[h.domain as usize][h]
    }
}

impl World {
    /// The batch the world is in: whole batches since the start (`Config::batch_days`).
    pub fn batch(&self) -> u64 {
        self.day() / self.cfg.batch_days as u64
    }
    /// The first batch that starts `delay` days from now or later (`delay` 1 to `MAX_INTERVAL`): never the batch the
    /// world is in, so a body placed during a batch updates in a later one.
    pub(crate) fn due_in(&self, delay: u64) -> u64 {
        (self.day() + delay.clamp(1, MAX_INTERVAL as u64)).div_ceil(self.cfg.batch_days as u64)
    }
    /// Place body `b` made outside the laws (a founder, an edit), its first update in the first batch `delay` days on or
    /// later.
    pub fn add(&mut self, mut b: Body, delay: u64) -> Handle {
        b.prepare(&self.chem, &self.cfg);
        let due = self.due_in(delay);
        self.in_domain(b.cell as usize, |w, d| d.add(b, w.day(), due))
    }
}

/// A free place in `v` (the last one freed first, `free`), or a new one at its end.
pub(crate) fn free_place<T: Default>(v: &mut Vec<T>, free: &mut Vec<u32>) -> usize {
    free.pop().map(|i| i as usize).unwrap_or_else(|| {
        v.push(T::default());
        v.len() - 1
    })
}
