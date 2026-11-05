//! A chunk (laws.md 0): a run of cells and all that body code writes while it runs on them. The cells are cut into
//! `Config::chunks` runs of neighbouring indices, the same for any number of threads; one task per chunk runs on the
//! pool (`run`), the world read only, and a serial step runs on a chunk of one cell (`World::in_cell`). A chunk writes
//! its cells (their bodies, cover and sums), its view of the store (`store::Moves`: each cell sees only itself and its
//! share of its air node), its events and counters, the lineages and born rows it makes (named when it ends,
//! `registry`) and what its bodies do to other cells (`cross::Entry`). When it ends the world takes all that, chunk by
//! chunk, so every sum runs in the order of the cells (`World::take_chunks`).

use super::{Cell, Handle};
use crate::lineage::Lineage;
use crate::store::{book_by_array, Class, Moves, Node};
use crate::{body::Body, event::Event, Counters, World};
use super::cross::Entry;

/// A run of cells and what body code wrote besides them (`Written`, which the chunk reads as its own fields).
pub struct Chunk<'a> {
    pub(super) cells: &'a mut [Cell],
    pub(super) written: Written,
}

/// What body code wrote on a chunk's cells besides the cells: the world takes it when the chunk ends
/// (`World::take_chunks`).
#[derive(Default)]
pub struct Written {
    /// The chunk's first cell and its count of cells
    pub(super) first: usize,
    len: usize,
    /// The view of the store of its cells and the moves gathered on it
    pub moves: Moves,
    /// What its bodies did to other cells and to parts, in the order made
    pub out: Vec<Entry>,
    pub(crate) events: Vec<Event>,
    pub(crate) counts: Counters,
    /// The lineages and born rows its bodies made, by provisional id (`registry`)
    pub(super) brains: Vec<Lineage>,
    pub(super) genomes: Vec<Vec<f32>>,
    /// The bodies that moved into its cells, by id, with their cell
    arrived: Vec<(u64, u32)>,
}

impl std::ops::Deref for Chunk<'_> {
    type Target = Written;
    fn deref(&self) -> &Written {
        &self.written
    }
}

impl std::ops::DerefMut for Chunk<'_> {
    fn deref_mut(&mut self) -> &mut Written {
        &mut self.written
    }
}

impl<'a> Chunk<'a> {
    /// A chunk of `cells` from cell `first`, its view with room for all of them (`ns` substances).
    fn new(first: usize, cells: &'a mut [Cell], ns: usize) -> Chunk<'a> {
        let len = cells.len();
        let mut written = Written { first, len, ..Default::default() };
        written.moves.reserve(len, ns);
        Chunk { cells, written }
    }
    /// Its cells, by index.
    pub fn cells(&self) -> std::ops::Range<usize> {
        self.first..self.first + self.cells.len()
    }
    /// The life of its cell `i`. Panics for a cell of another chunk.
    pub fn at(&self, i: usize) -> &Cell {
        &self.cells[i - self.first]
    }
    pub fn at_mut(&mut self, i: usize) -> &mut Cell {
        &mut self.cells[i - self.first]
    }
    /// The handles of the living bodies of its cell `i`, in the order they came.
    pub fn cell(&self, i: usize) -> impl Iterator<Item = Handle> + '_ {
        super::handles(self.at(i), i)
    }
    /// The cover of its cell `i`.
    pub fn cover(&mut self, i: usize) -> &mut crate::cover::Cover {
        &mut self.at_mut(i).cover
    }
    /// The cover of its cell `i`, its view and its counters, each apart (`cover::step`).
    pub(crate) fn cover_with(&mut self, i: usize) -> (&mut crate::cover::Cover, &mut Moves, &mut Counters) {
        (&mut self.cells[i - self.written.first].cover, &mut self.written.moves, &mut self.written.counts)
    }
    /// Place a new body in its cell at `days` d from the start (`World::days_now`); it first updates at its cell's next
    /// step.
    pub(crate) fn add(&mut self, mut b: Body, days: f64) -> Handle {
        let i = b.cell as usize;
        let c = self.at_mut(i);
        c.placed += 1;
        (b.id, b.alive, b.last, b.count) = (super::new_id(c.serial, c.placed), true, days, b.n());
        self.store(b)
    }
    /// Store body `b` at the end of its cell's list.
    fn store(&mut self, b: Body) -> Handle {
        let i = b.cell as usize;
        let list = &mut self.at_mut(i).bodies;
        list.push(b);
        Handle { cell: i as u32, slot: (list.len() - 1) as u32 }
    }
    /// Body `b`, taken out of `h`, leaves the world: its place is a hole that keeps its id until the pass ends, and its
    /// brain's block is free.
    pub(crate) fn remove(&mut self, h: Handle, b: &Body) {
        self.at_mut(h.cell as usize).brains.free(b.brain);
        self[h] = Body { id: b.id, ..Default::default() };
    }
    /// The brains of its cell `i`.
    pub(crate) fn brains(&mut self, i: usize) -> &mut crate::life::Brains {
        &mut self.at_mut(i).brains
    }
    /// The rows of body `b` (taken out of its slot or not) in its cell, of its lineage's shape.
    pub(crate) fn rows(&self, w: &World, b: &Body) -> &[f32] {
        self.shape_of(w, b.lineage).map_or(&[], |s| self.at(b.cell as usize).brains.rows(b.brain, s))
    }
    /// Body `b`, which moved into its cell from another, is stored here.
    pub(crate) fn arrive(&mut self, b: Body) -> Handle {
        self.written.arrived.push((b.id, b.cell));
        self.store(b)
    }
    /// `members` members entered its cell `i` in a move that spent `pushed` J pushing through its bodies
    /// (`choose::land_move`).
    pub fn cross(&mut self, i: usize, members: f32, pushed: f64) {
        let c = self.at_mut(i);
        (c.crossings, c.pushed) = (c.crossings + members, c.pushed + pushed);
    }
    /// Place body `c` made by a law on the world's day, its first update at the next month's start; its visible traits are
    /// worked out from what it holds now, so others see it before its first update.
    pub(crate) fn place(&mut self, w: &World, mut c: Body) -> Handle {
        c.prepare(&w.chem, &w.cfg);
        self.place_part(w, c)
    }
    /// `place` for a part of a node with the node's genes (`nodes::part`), whose constants (`Body::k`) follow from them
    /// already.
    pub(crate) fn place_part(&mut self, w: &World, mut c: Body) -> Handle {
        c.stale = true;
        self.add(c, w.days_now())
    }
    /// The visible traits of its living bodies that changed since the last survey are worked out again
    /// (`Body::visible`, `Body::stale`): once before what reads them, the cells' sums and the batch.
    pub(crate) fn see_changed(&mut self, w: &World) {
        for b in self.cells.iter_mut().flat_map(|c| &mut c.bodies).filter(|b| b.alive && b.stale) {
            (b.vis, b.stale) = (b.visible(&w.chem), false);
        }
    }
}

impl std::ops::Index<Handle> for Chunk<'_> {
    type Output = Body;
    fn index(&self, h: Handle) -> &Body {
        &self.at(h.cell as usize).bodies[h.slot as usize]
    }
}

impl std::ops::IndexMut<Handle> for Chunk<'_> {
    fn index_mut(&mut self, h: Handle) -> &mut Body {
        &mut self.at_mut(h.cell as usize).bodies[h.slot as usize]
    }
}

/// The first cell of each chunk of the world (`Config::chunks` runs of its cells, at most one per cell), and the count
/// of cells last: the chunk of cell `i` is the last whose first cell is at most `i`.
pub(crate) fn bounds(w: &World) -> Vec<usize> {
    let n = w.mesh.cells().n();
    let c = (w.cfg.chunks as usize).clamp(1, n.max(1));
    (0..=c).map(|k| k * n / c).collect()
}

/// The count of chunks of the world (`bounds`).
pub(crate) fn chunks(w: &World) -> usize {
    bounds(w).len() - 1
}

/// Run `f` on each chunk with what `work` gives it (one item per chunk, `count`), which `f` may change, and a scratch
/// made by `init` once per thread, on the pool (`par::each_init`; on the calling thread when `load`, the phase's count of
/// work, is small). The cells are taken out of the world meanwhile, and the world takes what they wrote after (`World::take_chunks`):
/// what the chunks did to other cells is queued (`Life::queued`). Returns `work`.
pub(crate) fn run<T: Send, S>(w: &mut World, work: Vec<T>, load: usize, init: impl Fn(&World) -> S + Sync + Send, f: impl Fn(&World, &mut Chunk, &mut T, &mut S) + Sync + Send) -> Vec<T> {
    let b = bounds(w);
    assert_eq!(work.len(), b.len() - 1, "one item of work per chunk");
    let mut cells = std::mem::take(&mut w.life.cells);
    let mut items = Vec::with_capacity(work.len());
    let mut rest = &mut cells[..];
    for (k, t) in work.into_iter().enumerate() {
        let (mine, after) = rest.split_at_mut(b[k + 1] - b[k]);
        items.push((Chunk::new(b[k], mine, w.chem.ns()), t));
        rest = after;
    }
    // (What the cells see of their air nodes, worked out once per air node; the store does not change while it runs.)
    let ns = w.chem.ns();
    let seen: Vec<f64> = (0..w.mesh.level(crate::AIR).n() * ns).map(|x| w.a_seen(x % ns, x / ns)).collect();
    w.life.air_seen = seen;
    let world = &*w;
    crate::par::each_init(&mut items, load, || init(world), |s, (c, t)| f(world, c, t, s));
    let (done, work): (Vec<Written>, Vec<T>) = items.into_iter().map(|(c, t)| (c.written, t)).unzip();
    w.life.air_seen.clear();
    w.life.cells = cells;
    w.take_chunks(done);
    work
}

/// The node in whose name a serial step moves for a body outside its update (`World::in_cell`): the store books no
/// body's side of a move, so a body node of any index serves.
const A_BODY: Node = Node { class: Class::Body, index: 0 };

/// The classes whose nodes a chunk's view holds, which its booking writes.
const VIEWED: [Class; 4] = [Class::Ground, Class::Air, Class::Bed, Class::Bodies];

impl World {
    /// A serial step on cell `i`: its life is taken out of the world into a chunk of its own, `f` runs on it as body
    /// code runs on a chunk of the pool, its moves in the name of a body (`A_BODY`) unless `f` opens them for another
    /// node, it is put back, and the world takes what it wrote (`take_chunks`).
    pub fn in_cell<R>(&mut self, i: usize, f: impl FnOnce(&World, &mut Chunk) -> R) -> R {
        let mut one = [std::mem::take(&mut self.life.cells[i])];
        let mut c = Chunk::new(i, &mut one, self.chem.ns());
        c.moves.open(self, i, A_BODY);
        let r = f(self, &mut c);
        let done = c.written;
        let [cell] = one;
        self.life.cells[i] = cell;
        self.take_chunks(vec![done]);
        r
    }
    /// The world takes what chunks `done` wrote, chunk by chunk in order: their events and counters, where the bodies
    /// that moved into their cells are stored (`Life::find`), the lineages and born rows they made named (`Life::name`),
    /// their moves booked (each array of the store by one task, the chunks' changes in their order and each chunk's in the
    /// order of its cells, `store::book_by_array`), and what they did to other cells queued (`Life::queued`).
    pub(crate) fn take_chunks(&mut self, mut done: Vec<Written>) {
        for d in &mut done {
            self.events.append(&mut d.events);
            self.counts.add(&mut d.counts);
            d.arrived.iter().for_each(|&(id, i)| self.life.note_at(id, i));
            if !d.brains.is_empty() || !d.genomes.is_empty() {
                let (brains, genomes) = (std::mem::take(&mut d.brains), std::mem::take(&mut d.genomes));
                self.life.name(d.first..d.first + d.len, (brains, genomes), &mut d.out);
            }
        }
        let open: Vec<&Moves> = done.iter().map(|d| &d.moves).filter(|m| m.gathered()).collect();
        if !open.is_empty() {
            let (ns, work) = (self.chem.ns(), open.len());
            let rounded = book_by_array(&mut self.store, ns, &VIEWED, work, |b| open.iter().for_each(|m| m.book_into(b)));
            self.counts.add_rounded(rounded);
            for m in open {
                m.each_cell(|lift, short| {
                    self.store.lift(lift);
                    self.counts.clamped_heat += short;
                });
            }
        }
        done.into_iter().for_each(|d| self.life.queued.extend(d.out));
    }
}
