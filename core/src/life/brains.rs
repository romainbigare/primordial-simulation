//! The brains of a cell's bodies (laws.md 0): each body with a brain has one block of numbers in its cell's array: its
//! brain rows, then per learned row the moving average of the squares of its learning steps (`brain::learn`), then the
//! trace of its last decision (`choose::trace`). A block freed is reused by the next of its length, so no body needs an
//! allocation of its own; the chunk of the cell is the one writer of its blocks.

use crate::brain::Shape;
use serde::{Deserialize, Serialize};

/// A body's block in its cell's brains (`Brains`): where it starts, and its length (0: no brain).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    at: u32,
    len: u32,
}

impl Block {
    /// Whether it holds a brain.
    #[inline]
    pub fn some(self) -> bool {
        self.len > 0
    }
}

/// The lengths of the parts of a block of a brain of shape `s`: rows, moving averages, trace.
#[inline]
fn parts(s: Shape) -> [usize; 3] {
    if s.h == 0 { [0; 3] } else { [s.rows() * s.r(), s.learned(), s.h + s.learned() + 1] }
}

/// A brain's numbers in its block (`Brains::mind`).
pub struct Mind<'a> {
    pub rows: &'a mut [f32],
    pub ms: &'a mut [f32],
    pub trace: &'a mut [f32],
}

/// The blocks of the brains of a cell's bodies, and the blocks freed.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Brains {
    v: Vec<f32>,
    free: Vec<Block>,
}

impl Brains {
    /// A block of `len` numbers: the last freed of that length, or a new one at the end.
    fn grab(&mut self, len: usize) -> Block {
        if let Some(x) = self.free.iter().rposition(|f| f.len as usize == len) {
            return self.free.swap_remove(x);
        }
        self.v.resize(self.v.len() + len, 0.0);
        Block { at: (self.v.len() - len) as u32, len: len as u32 }
    }
    /// The numbers of block `b`.
    #[inline]
    fn of(&self, b: Block) -> &[f32] {
        &self.v[b.at as usize..(b.at + b.len) as usize]
    }
    #[inline]
    fn of_mut(&mut self, b: Block) -> &mut [f32] {
        &mut self.v[b.at as usize..(b.at + b.len) as usize]
    }
    /// A new block for a brain of shape `s` with rows `rows`, its moving averages 0 and no trace (none for a shape of
    /// no brain).
    pub(crate) fn alloc(&mut self, s: Shape, rows: &[f32]) -> Block {
        let p = parts(s);
        if p[0] == 0 {
            return Block::default();
        }
        debug_assert_eq!(rows.len(), p[0], "rows of the brain's shape");
        let b = self.grab(p.iter().sum());
        let all = self.of_mut(b);
        all[..p[0]].copy_from_slice(rows);
        all[p[0]..].fill(0.0);
        b
    }
    /// Block `b` is free for another body.
    pub(crate) fn free(&mut self, b: Block) {
        if b.some() {
            self.free.push(b);
        }
    }
    /// The rows of block `b` of a brain of shape `s` (none for no brain).
    #[inline]
    pub fn rows(&self, b: Block, s: Shape) -> &[f32] {
        if b.some() { &self.of(b)[..parts(s)[0]] } else { &[] }
    }
    /// The parts of block `b` of a brain of shape `s`.
    pub(crate) fn mind(&mut self, b: Block, s: Shape) -> Mind<'_> {
        let [r, m, _] = parts(s);
        let all: &mut [f32] = if b.some() { self.of_mut(b) } else { &mut [] };
        let (rows, rest) = all.split_at_mut(r.min(all.len()));
        let (ms, trace) = rest.split_at_mut(m.min(rest.len()));
        Mind { rows, ms, trace }
    }
    /// The rows of block `b` move the share `a` of the way toward those of block `o` (both of a brain of shape `s`).
    pub(crate) fn blend(&mut self, b: Block, o: Block, s: Shape, a: f32) {
        let r = parts(s)[0];
        if !b.some() || !o.some() || b == o {
            return;
        }
        for q in 0..r {
            let (x, y) = (self.v[b.at as usize + q], self.v[o.at as usize + q]);
            self.v[b.at as usize + q] = x + a * (y - x);
        }
    }
    /// To the rows of block `c` comes what the rows of block `b` learned since they were `from` (both of a brain of
    /// shape `s`): x + (r - f), each row's own.
    pub(crate) fn learned_onto(&mut self, c: Block, b: Block, from: &[f32], s: Shape) {
        for (q, &f) in from.iter().enumerate().take(parts(s)[0]) {
            let r = self.v[b.at as usize + q];
            self.v[c.at as usize + q] += r - f;
        }
    }
    /// Into block `c` of a brain of shape `sc`, from block `b` of a brain of shape `sb`: the moving averages of the learned
    /// rows they share, and the trace where it has the same length. Returns whether the trace came.
    pub(crate) fn carry(&mut self, b: Block, sb: Shape, c: Block, sc: Shape) -> bool {
        if !b.some() || !c.some() {
            return false;
        }
        let ([rb, mb, tb], [rc, mc, tc]) = (parts(sb), parts(sc));
        let m = mb.min(mc);
        self.v.copy_within(b.at as usize + rb..b.at as usize + rb + m, c.at as usize + rc);
        if tb == tc {
            self.v.copy_within(b.at as usize + rb + mb..b.at as usize + rb + mb + tb, c.at as usize + rc + mc);
        }
        tb == tc
    }
    /// A copy of block `b` (rows, moving averages and trace) in a new block of this cell.
    pub(crate) fn copy(&mut self, b: Block) -> Block {
        if !b.some() {
            return b;
        }
        let c = self.grab(b.len as usize);
        self.v.copy_within(b.at as usize..(b.at + b.len) as usize, c.at as usize);
        c
    }
    /// All the numbers of block `b`, to carry to another cell (`take_in`).
    pub(crate) fn all(&self, b: Block) -> &[f32] {
        if b.some() { self.of(b) } else { &[] }
    }
    /// A new block holding `all` (`Brains::all` of a block of another cell).
    pub(crate) fn take_in(&mut self, all: &[f32]) -> Block {
        if all.is_empty() {
            return Block::default();
        }
        let b = self.grab(all.len());
        self.of_mut(b).copy_from_slice(all);
        b
    }
}
