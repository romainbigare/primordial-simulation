//! A chunk (laws.md 0): a run of cells and what the cover's month writes while it runs on them. The cells are cut into
//! `Config::chunks` runs of neighbouring indices, the same for any number of threads; one task per chunk runs on the
//! pool (`run`), the world read only, and a serial step runs on a chunk of one cell (`World::in_cell`). A chunk writes
//! its cells' cover, its view of the store (`store::Moves`: each cell sees only itself and its share of its air node)
//! and its counters. When it ends the world takes them, chunk by chunk, so every sum runs in the order of the cells
//! (`World::take_chunks`).

use super::Cell;
use crate::store::{book_by_array, Class, Moves, Node};
use crate::{Counters, World};

/// A run of cells and what the cover's month wrote besides them.
pub struct Chunk<'a> {
    cells: &'a mut [Cell],
    /// Its first cell
    first: usize,
    /// The view of the store of its cells and the moves gathered on it
    pub moves: Moves,
    pub(crate) counts: Counters,
}

impl<'a> Chunk<'a> {
    /// A chunk of `cells`, the run from cell `first`; its view with room for all of them (`ns` substances), or none
    /// for a chunk that does not run (`run_on`).
    fn new(first: usize, cells: &'a mut [Cell], ns: usize, room: bool) -> Chunk<'a> {
        let mut moves = Moves::default();
        if room {
            moves.reserve(cells.len(), ns);
        }
        Chunk { cells, first, moves, counts: Counters::default() }
    }
    /// The cells of its chunk, by index, in order.
    pub fn cells(&self) -> std::ops::Range<usize> {
        self.first..self.first + self.cells.len()
    }
    /// The life of its cell `i`. Panics for a cell of another chunk.
    pub fn at(&self, i: usize) -> &Cell {
        &self.cells[i - self.first]
    }
    /// The cover of its cell `i`.
    pub fn cover(&mut self, i: usize) -> &mut crate::cover::Cover {
        &mut self.cells[i - self.first].cover
    }
    /// The takers of its cell `i`, its view and its counters, each apart (`fauna::step`).
    pub(crate) fn fauna_with(&mut self, i: usize) -> (&mut crate::fauna::Fauna, &mut Moves, &mut Counters) {
        (&mut self.cells[i - self.first].fauna, &mut self.moves, &mut self.counts)
    }
    /// The cover of its cell `i`, its view and its counters, each apart (`cover::step`).
    pub(crate) fn cover_with(&mut self, i: usize) -> (&mut crate::cover::Cover, &mut Moves, &mut Counters) {
        (&mut self.cells[i - self.first].cover, &mut self.moves, &mut self.counts)
    }
}

/// The first cell of each chunk of the world (`Config::chunks` runs of its cells, at most one per cell), and the count
/// of cells last: the chunk of cell `i` is the last whose first cell is at most `i`.
fn bounds(w: &World) -> Vec<usize> {
    let n = w.mesh.cells().n();
    let c = (w.cfg.chunks as usize).clamp(1, n.max(1));
    (0..=c).map(|k| k * n / c).collect()
}

/// Run `f` on each chunk of the world's cells, on the pool (`par::each_init`; on the calling thread when `load`, the
/// phase's count of work, is small). The cells are taken out of the world meanwhile, and the world takes what they
/// wrote after (`World::take_chunks`).
pub(crate) fn run(w: &mut World, load: usize, f: impl Fn(&World, &mut Chunk) + Sync + Send) {
    run_on(w, load, |_| true, f);
}

/// `run` on the chunks whose cells `busy` holds work for (it reads each chunk's run of cells); the others write
/// nothing, and the world takes them as empty, in their place.
pub(crate) fn run_on(w: &mut World, load: usize, busy: impl Fn(std::ops::Range<usize>) -> bool + Sync + Send, f: impl Fn(&World, &mut Chunk) + Sync + Send) {
    let b = bounds(w);
    let ns = w.chem.ns();
    let mut cells = std::mem::take(&mut w.life.cells);
    let mut items = Vec::with_capacity(b.len() - 1);
    let mut rest = &mut cells[..];
    for k in 0..b.len() - 1 {
        let (mine, after) = rest.split_at_mut(b[k + 1] - b[k]);
        items.push(Chunk::new(b[k], mine, ns, busy(b[k]..b[k + 1])));
        rest = after;
    }
    // (What the cells see of an air node is worked out once, where they first read it: the store does not change while
    // the pass runs, `store::gather`.)
    w.life.air_seen = (0..w.mesh.level(crate::AIR).n()).map(|_| std::sync::OnceLock::new()).collect();
    let world = &*w;
    crate::par::each_init(&mut items, load, || (), |_, c| {
        if busy(c.cells()) {
            f(world, c);
        }
    });
    let done: Vec<(Moves, Counters)> = items.into_iter().map(|c| (c.moves, c.counts)).collect();
    w.life.air_seen.clear();
    w.life.cells = cells;
    w.take_chunks(done);
}

/// The classes whose nodes a chunk's view holds, which its booking writes.
const VIEWED: [Class; 4] = [Class::Ground, Class::Air, Class::Bed, Class::Aside];

impl World {
    /// A serial step on cell `i`: its life is taken out of the world into a chunk of its own, `f` runs on it as the
    /// cover's month runs on a chunk of the pool, its moves in the name of its cover unless `f` opens them for another
    /// node, it is put back, and the world takes what it wrote (`take_chunks`).
    pub fn in_cell<R>(&mut self, i: usize, f: impl FnOnce(&World, &mut Chunk) -> R) -> R {
        let mut one = [std::mem::take(&mut self.life.cells[i])];
        let mut c = Chunk::new(i, &mut one, self.chem.ns(), true);
        c.moves.open(self, i, Node::new(Class::Life, i));
        let r = f(self, &mut c);
        let done = vec![(c.moves, c.counts)];
        let [cell] = one;
        self.life.cells[i] = cell;
        self.take_chunks(done);
        r
    }
    /// The world takes what chunks `done` wrote, chunk by chunk in order: their counters, and their moves booked (each
    /// array of the store by one task, the chunks' changes in their order and each chunk's in the order of its cells,
    /// `store::book_by_array`) with the potential energy of what life gave its ground or took from it
    /// (`store::life_moved`).
    pub(crate) fn take_chunks(&mut self, mut done: Vec<(Moves, Counters)>) {
        done.iter_mut().for_each(|d| self.counts.add(&mut d.1));
        let open: Vec<&Moves> = done.iter().map(|d| &d.0).filter(|m| m.gathered()).collect();
        if open.is_empty() {
            return;
        }
        let (ns, work) = (self.chem.ns(), open.len());
        let rounded = book_by_array(&mut self.store, ns, &VIEWED, work, |b| open.iter().for_each(|m| m.book_into(b)));
        self.counts.add_rounded(rounded);
        for m in &open {
            m.each_cell(|lift, short| {
                self.store.lift(lift);
                self.counts.clamped_heat += short;
            });
        }
        let fell: Vec<(usize, Vec<f64>)> = open.iter().flat_map(|m| m.fell()).collect();
        if !fell.is_empty() {
            crate::store::life_moved(self, &fell);
        }
    }
}
