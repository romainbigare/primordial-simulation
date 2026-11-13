//! The registry of bodies (laws.md 0): the bodies of each ground cell (`Cell`), with ids each cell gives, found by id;
//! the caches of the last survey; the lineages and born rows (`registry`). Body code
//! runs on chunks of cells (`chunk`), and what it does to other cells waits in a queue for the serial pass (`cross`).

mod brains;
mod chunk;
mod cross;
mod disperse;
mod registry;
mod saved;
mod shared;
mod touched;

pub use brains::{Block, Brains, Mind};
pub use chunk::Chunk;
pub(crate) use chunk::{bounds, chunks, run, Of};
pub use cross::{land, Entry};
pub use disperse::lot;
pub use shared::{solids_of, AGE, BROKEN, LACK, PHASE, SPENT, TAKEN, WET_TISSUE};
pub use registry::Genome;

use crate::body::Body;
use crate::cover::Cover;
use crate::space::Mesh;
use crate::survey::ROW;
use crate::World;
use std::collections::BTreeMap;

/// The id of a body or a part is the serial of the cell it was placed or made in shifted left by this many bits, plus
/// that cell's count of bodies placed or of parts made then (`new_id`).
const ID_SHIFT: u32 = 40;
/// In `Life::at_serial`: a serial whose cell is gone.
const GONE: u32 = u32::MAX;

/// The id of the body placed, or the part made, as the `count`-th of its kind in the cell of serial `serial`: from 1, so
/// no id is 0, and below 2^`ID_SHIFT`, so it does not run into the next serial's.
pub(crate) fn new_id(serial: u32, count: u64) -> u64 {
    assert!(count < 1 << ID_SHIFT && (serial as u64) < 1 << (u64::BITS - ID_SHIFT), "cell serial {serial} placed or made 2^{ID_SHIFT}: its ids would run out");
    (serial as u64) << ID_SHIFT | count
}

/// Where a body is stored: its cell and its place in that cell's list. It holds until the pass ends (`Life::compact`);
/// a body is named for longer by its id.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Handle {
    pub cell: u32,
    pub slot: u32,
}

/// The life of one ground cell: its serial, its bodies, in the order they came (one that died or left stays as a hole,
/// not alive, until the pass ends, so a handle holds through a pass), how many bodies were placed in it, its cover, and
/// what the movers did there.
#[derive(Clone, Debug, Default)]
pub struct Cell {
    /// The cell's name for the ids of what is placed or made in it (`new_id`): given when the cell is made, it never
    /// changes and is never given again, while its index may (`Life::at_serial`)
    pub(crate) serial: u32,
    pub(crate) bodies: Vec<Body>,
    /// Bodies placed in it so far (children, split halves, parts of nodes, members drawn, founders)
    placed: u64,
    /// The energy movers spent pushing through its bodies since the last survey, which then breaks them, J
    /// (`choose::wear`)
    pub pushed: f64,
    /// The members that moved into it since the last month (`Counters::traffic`)
    pub(crate) crossings: f32,
    pub cover: Cover,
    /// The brains of its bodies
    pub brains: Brains,
    /// The parts that lie in it (alike lone parts as one lot, `bond::Part::count`), in the order they came, the bonds
    /// between them (`bond`), and how many parts were made in it so far (their ids, `bond::Part::id`)
    pub parts: Vec<crate::bond::Part>,
    pub bonds: crate::bond::Bonds,
    pub(crate) parts_made: u64,
}

#[derive(Clone, Debug, Default)]
pub struct Life {
    /// The life of each ground cell (taken out of the world while body code runs on it, `chunk::run`)
    pub(crate) cells: Vec<Cell>,
    /// The serial the next cell made takes (`Cell::serial`), and the index of the cell of each serial given (`GONE` once
    /// that cell is gone), rebuilt from the cells (`Life::index`)
    next_serial: u32,
    at_serial: Vec<u32>,
    /// The cell each body stored outside the cell it was placed in is in, by id (`find`), and so for the parts
    /// (`part_cell`)
    moved: BTreeMap<u64, u32>,
    parts_moved: BTreeMap<u64, u32>,
    /// What body code did outside its cell, waiting for the serial pass (`cross::land`)
    pub(crate) queued: Vec<Entry>,
    /// Per cell, from the month's start: what stands on it (`contract::Stand`), and of its bodies their members and
    /// their strength summed (`Life::sum`). Not saved: a snapshot is taken at a month's start, where they are built
    /// before any law reads them; the tape reads them first, so `tape::load_snapshot` builds them again
    /// (`contract::stands`, `survey::sums`)
    pub(crate) stands: Vec<crate::contract::Stand>,
    pub(crate) sums: Vec<[f64; ROW]>,
    /// Per cell, from the month's start (`contract::surroundings`): ground temperature, liquid depth, surface height
    /// (`contract::place`)
    pub env: Vec<[f64; 3]>,
    /// Per cell, from the last survey: its features as every brain perceives them, then the same smoothed over the
    /// neighbours once to `choose::SIGHTS - 1` times (`choose::cells`, read by `feat`)
    pub(crate) feat: Vec<[f32; crate::choose::NCELL]>,
    /// The brain of each lineage (None once it died out), and its living bodies
    pub brains: Vec<Option<crate::lineage::Lineage>>,
    pub members: Vec<u32>,
    /// Born brain rows, each shared by the bodies born with them (none: free)
    pub genomes: Vec<Genome>,
    free_genomes: Vec<u32>,
    /// While a parallel pass runs (`life::run`): what bodies and parts see in each air node as the store held it at the
    /// pass's start (`World::a_seen`, of each substance), the same for every cell under it, worked out where first read;
    /// else empty
    pub(crate) air_seen: Vec<std::sync::OnceLock<Vec<f64>>>,
    /// The cells the passes held, which what follows a pass works on (`touched`)
    pub(crate) touched: touched::Touched,
}

impl Life {
    /// No bodies yet on `mesh`; each cell's serial is its index.
    pub fn new(mesh: &Mesh) -> Life {
        let n = mesh.cells().n();
        let cells = (0..n).map(|i| Cell { serial: i as u32, ..Default::default() }).collect();
        let mut life = Life { cells, next_serial: n as u32, stands: vec![Default::default(); n], sums: vec![[0.0; ROW]; n], ..Default::default() };
        life.index();
        life
    }
    /// The index of the cell of each serial, and where the bodies and parts stored outside the cell they were placed or
    /// made in are (`moved`, `parts_moved`), are found from the cells.
    fn index(&mut self) {
        let Life { cells, next_serial, at_serial, moved, parts_moved, .. } = self;
        *at_serial = vec![GONE; *next_serial as usize];
        cells.iter().enumerate().for_each(|(i, c)| at_serial[c.serial as usize] = i as u32);
        moved.clear();
        parts_moved.clear();
        for (i, c) in cells.iter().enumerate() {
            let away = |id: u64| origin(at_serial, id) != Some(i);
            moved.extend(c.bodies.iter().filter(|b| b.alive && away(b.id)).map(|b| (b.id, i as u32)));
            parts_moved.extend(c.parts.iter().filter(|p| away(p.id)).map(|p| (p.id, i as u32)));
        }
    }
    /// The living bodies with their handles, cell by cell, each cell's in the order they came.
    pub fn living(&self) -> impl Iterator<Item = (Handle, &Body)> {
        self.cells.iter().enumerate().flat_map(|(i, c)| c.bodies.iter().enumerate().map(move |(s, b)| (Handle { cell: i as u32, slot: s as u32 }, b))).filter(|x| x.1.alive)
    }
    /// The living bodies, in the order of `living`.
    pub fn alive(&self) -> impl Iterator<Item = &Body> {
        self.living().map(|x| x.1)
    }
    /// Run `f` on each living body, for work that does not depend on order.
    pub fn each_mut(&mut self, f: impl FnMut(&mut Body)) {
        self.cells.iter_mut().flat_map(|c| &mut c.bodies).filter(|b| b.alive).for_each(f);
    }
    /// The handles of the living bodies of cell `i`, in the order they came.
    pub fn cell(&self, i: usize) -> impl Iterator<Item = Handle> + '_ {
        handles(&self.cells[i], i)
    }
    /// How many living bodies cell `i` holds.
    pub fn in_cell(&self, i: usize) -> usize {
        self.cell(i).count()
    }
    /// Where the body with id `id` is stored, if it lives: in the cell it was placed in, unless it moved away.
    pub fn find(&self, id: u64) -> Option<Handle> {
        self.find_in(self.moved.get(&id).map_or_else(|| origin(&self.at_serial, id), |&c| Some(c as usize))?, id)
    }
    /// Where the living body with id `id` is stored in cell `i`, if it is there.
    pub(crate) fn find_in(&self, i: usize, id: u64) -> Option<Handle> {
        let s = self.cells.get(i)?.bodies.iter().position(|b| b.alive && b.id == id)?;
        Some(Handle { cell: i as u32, slot: s as u32 })
    }
    /// The first break of the storage, if any: a body in the list of a cell that is not its own, or one its id does not
    /// find (`check::broken`).
    pub fn unsound(&self) -> Option<String> {
        self.living().find(|&(h, b)| b.cell != h.cell || self.find(b.id) != Some(h)).map(|(h, b)| format!("body {} at {h:?}: its cell {}, found at {:?}", b.id, b.cell, self.find(b.id)))
    }
    /// The features of cell `i` as a body of sight `sight` (1 to `choose::SIGHTS`) sees them from a neighbour cell, as
    /// the last survey found them (`choose::cells`); its own at sight 1.
    pub fn feat(&self, i: usize, sight: usize) -> &[f32; crate::choose::NCELL] {
        // (The cells are taken out of the world while a pass runs: the count comes from the features.)
        &self.feat[(sight.clamp(1, crate::choose::SIGHTS) - 1) * (self.feat.len() / crate::choose::SIGHTS) + i]
    }
    /// The members of the bodies of cell `i` and their strength summed, from the month's start (`survey::sums`).
    pub fn sum(&self, i: usize) -> &[f64; ROW] {
        &self.sums[i]
    }
    /// The cover of cell `i`.
    pub fn cover(&self, i: usize) -> &Cover {
        &self.cells[i].cover
    }
    /// The holes the pass left are closed: each cell keeps its living bodies in their order, and a body or a part that
    /// is gone from the cell it was noted in leaves `moved` or `parts_moved`. A cell whose freed brain blocks hold much
    /// of its array packs it (`Brains::pack`).
    pub(crate) fn compact(&mut self) {
        let Life { cells, moved, parts_moved, touched, .. } = self;
        // (Only a pass that held a cell changes it: the cells held since the last compaction, or every cell.)
        let list = touched.since_compact().unwrap_or_else(|| (0..cells.len() as u32).collect());
        parts_moved.retain(|&id, &mut i| list.binary_search(&i).is_err() || cells[i as usize].parts.iter().any(|p| p.id == id));
        for &i in &list {
            let c = &mut cells[i as usize];
            if c.bodies.iter().any(|b| !b.alive) {
                for b in c.bodies.iter().filter(|b| !b.alive && b.id != 0) {
                    if moved.get(&b.id) == Some(&i) {
                        moved.remove(&b.id);
                    }
                }
                c.bodies.retain(|b| b.alive);
            }
        }
        for &i in &list {
            let c = &mut cells[i as usize];
            if c.brains.sparse() {
                c.brains.pack(c.bodies.iter_mut().map(|b| &mut b.brain));
            }
        }
        touched.compacted();
    }
    /// Body `id` is now stored in cell `i` (`find`).
    fn note_at(&mut self, id: u64, i: u32) {
        if origin(&self.at_serial, id) == Some(i as usize) { self.moved.remove(&id) } else { self.moved.insert(id, i) };
    }
    /// Part `id` now lies in cell `i` (`part_cell`).
    pub(crate) fn note_part_at(&mut self, id: u64, i: u32) {
        if origin(&self.at_serial, id) == Some(i as usize) { self.parts_moved.remove(&id) } else { self.parts_moved.insert(id, i) };
    }
    /// The cell the part with id `id` lies in, if it exists: the one it was made in, unless its structure moved.
    pub fn part_cell(&self, id: u64) -> Option<usize> {
        let i = self.parts_moved.get(&id).map_or_else(|| origin(&self.at_serial, id), |&c| Some(c as usize))?;
        self.cells.get(i)?.parts.iter().any(|p| p.id == id).then_some(i)
    }
    /// The part with id `id`, if it exists.
    pub fn part(&self, id: u64) -> Option<&crate::bond::Part> {
        let i = self.part_cell(id)?;
        self.cells[i].parts.iter().find(|p| p.id == id)
    }
    /// The part with id `id`, to change (an edit), if it exists.
    pub fn part_mut(&mut self, id: u64) -> Option<&mut crate::bond::Part> {
        let i = self.part_cell(id)?;
        self.cells[i].parts.iter_mut().find(|p| p.id == id)
    }
    /// The life of cell `i`: its bodies, brains, parts and bonds, and its cover.
    pub fn cell_at(&self, i: usize) -> &Cell {
        &self.cells[i]
    }
    /// The serial of cell `i` (`Cell::serial`), a name it keeps for the world's life.
    pub fn serial(&self, i: usize) -> u32 {
        self.cells[i].serial
    }
    /// The index now of the cell of serial `s`; none once it split or joined.
    pub fn at(&self, s: u32) -> Option<usize> {
        index_of(&self.at_serial, s as u64)
    }
    /// The parts of the world, cell by cell, each cell's in its order.
    pub fn parts(&self) -> impl Iterator<Item = &crate::bond::Part> {
        self.cells.iter().flat_map(|c| &c.parts)
    }
}

/// The index now of the cell the body or part named `id` was placed or made in, by the index of the cell of each serial
/// (`Life::at_serial`; none once that cell is gone): the one way from an id to a cell.
fn origin(at_serial: &[u32], id: u64) -> Option<usize> {
    index_of(at_serial, id >> ID_SHIFT)
}

/// The index now of the cell of serial `s`, by the index of the cell of each serial (`Life::at_serial`); none once that
/// cell is gone.
fn index_of(at_serial: &[u32], s: u64) -> Option<usize> {
    at_serial.get(s as usize).filter(|&&i| i != GONE).map(|&i| i as usize)
}

/// The handles of the living bodies of `c`, cell `i`, in the order they came.
fn handles(c: &Cell, i: usize) -> impl Iterator<Item = Handle> + '_ {
    c.bodies.iter().enumerate().filter(|x| x.1.alive).map(move |(s, _)| Handle { cell: i as u32, slot: s as u32 })
}

impl std::ops::Index<Handle> for Life {
    type Output = Body;
    fn index(&self, h: Handle) -> &Body {
        &self.cells[h.cell as usize].bodies[h.slot as usize]
    }
}

impl std::ops::IndexMut<Handle> for Life {
    fn index_mut(&mut self, h: Handle) -> &mut Body {
        &mut self.cells[h.cell as usize].bodies[h.slot as usize]
    }
}

impl World {
    /// Place body `b` made outside the laws (a founder, an edit); it first updates at the next month's start.
    pub fn add(&mut self, mut b: Body) -> Handle {
        b.prepare(&self.chem, &self.cfg);
        self.in_cell(b.cell as usize, |w, d| {
            // (Its brain, if it has none yet: the rows it was born with.)
            if !b.brain.some() {
                b.brain = d.new_brain(w, &b);
            }
            d.add(b, w.days_now())
        })
    }
}

impl Life {
    /// The brain rows of living body `b` (none without a brain).
    pub fn rows_of(&self, b: &Body) -> &[f32] {
        self.brain(b.lineage).map_or(&[], |l| self.cells[b.cell as usize].brains.rows(b.brain, l.shape))
    }
    /// The brain rows of the body at `h`, to change (an edit).
    pub fn rows_mut(&mut self, h: Handle) -> &mut [f32] {
        let b = &self[h];
        let (blk, s) = (b.brain, self.brain(b.lineage).map(|l| l.shape).unwrap_or_default());
        self.cells[h.cell as usize].brains.mind(blk, s).rows
    }
    /// The rows of body `b` (taken out of its slot) move the share `a` of the way toward those of body `o`, if their
    /// brains have one shape (`choose::watched`).
    pub(crate) fn blend_rows(&mut self, b: &Body, o: &Body, a: f32) {
        let (Some(s), Some(so)) = (self.brain(b.lineage).map(|l| l.shape), self.brain(o.lineage).map(|l| l.shape)) else { return };
        if s != so || !b.brain.some() || !o.brain.some() {
            return;
        }
        let (i, j) = (b.cell as usize, o.cell as usize);
        if i == j {
            return self.cells[i].brains.blend(b.brain, o.brain, s, a);
        }
        let from: Vec<f32> = self.cells[j].brains.rows(o.brain, s).to_vec();
        crate::brain::blend(self.cells[i].brains.mind(b.brain, s).rows, &from, a);
    }
}

/// A free place in `v` (the last one freed first, `free`), or a new one at its end.
pub(crate) fn free_place<T: Default>(v: &mut Vec<T>, free: &mut Vec<u32>) -> usize {
    free.pop().map(|i| i as usize).unwrap_or_else(|| {
        v.push(T::default());
        v.len() - 1
    })
}
