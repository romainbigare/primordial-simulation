//! Downhill: the cells in the order liquid leaves them, and one pass down that order. A priority flood over the floor
//! (the ground without the liquid that may run off), from the lowest cell at the head of its liquid, gives each cell
//! its spill level: the lowest level from which liquid there can reach the lowest cell (Barnes, Lehman and Mulla 2014).
//! Liquid runs from higher levels to lower ones, and at one level from where it stands (below the spill level) out
//! through the cells the flood reached earlier, so the pass by falling level, at one level the last
//! reached first, is a topological order of its links: one pass carries liquid across the map while depressions fill
//! and spill, and a level stretch drains toward its outlet. Where a basin is given, the flood starts from it instead: its
//! cells are reached first, at the head of their liquid, and the pass does not visit them, so what reaches one stays
//! there. The law says how much leaves a cell; this module moves it.

use std::{cmp::Reverse, collections::BinaryHeap};

/// A mesh's adjacency as slices: cell `i`'s entries are `start[i]..start[i + 1]`, each a link and its other cell.
#[derive(Clone, Copy, Debug)]
pub struct Adjacency<'a> {
    /// First entry of each cell, and one past the last (length: cells + 1).
    pub start: &'a [u32],
    /// The link of each entry.
    pub link: &'a [u32],
    /// The other cell of each entry.
    pub other: &'a [u32],
    /// The first cell of each link.
    pub a: &'a [u32],
}

/// The cells in the order of the pass, the rank of each cell in it (the basin's last, unvisited), whether the liquid of each cell stands (it lies
/// below the cell's spill level, in a depression, or in the lowest basin), and the cell the flood reached each from.
#[derive(Clone, Debug, Default)]
pub struct Order {
    cells: Vec<u32>,
    rank: Vec<u32>,
    /// Does the liquid of each cell stand (below its spill level, or in the lowest basin) rather than pass on?
    pub stands: Vec<bool>,
    /// The cell from which the flood reached each cell: its way out (`u32::MAX` for the lowest cell).
    pub outlet: Vec<u32>,
    /// Each cell's spill level, m: the lowest level from which its liquid reaches the lowest cell.
    pub spill: Vec<f64>,
}

/// A key that sorts levels from the lowest up: the bits of `h` in the total order of `f64::total_cmp`.
#[inline]
fn rising(h: f64) -> u64 {
    let b = h.to_bits();
    if b >> 63 == 1 { !b } else { b | 1 << 63 }
}

impl Order {
    /// Bring the order up to date for the floor of each cell, `floor`, m, the head of the liquid of cell `i`,
    /// `head(i)`, m, and the cells of the basin, `basin` (none: the flood starts from the lowest cell).
    pub fn update(&mut self, adj: &Adjacency, floor: &[f64], head: impl Fn(usize) -> f64, basin: &[u32]) {
        let n = floor.len();
        let Some(lowest) = (0..n).min_by(|&a, &b| floor[a].total_cmp(&floor[b])) else { return };
        let low = head(lowest).max(floor[lowest]);
        // Each cell's spill level (infinite until reached).
        let mut level = vec![f64::INFINITY; n];
        let mut flood = BinaryHeap::new();
        (self.cells, self.outlet) = (Vec::with_capacity(n - basin.len()), vec![u32::MAX; n]);
        if basin.is_empty() {
            level[lowest] = low;
            flood.push(Reverse((rising(low), 0u32, lowest as u32)));
        } else {
            basin.iter().for_each(|&c| level[c as usize] = head(c as usize).max(floor[c as usize]));
            basin.iter().for_each(|&c| reach(adj, floor, c as usize, (&mut level, &mut self.outlet), 0, &mut flood));
        }
        while let Some(Reverse((_, _, c))) = flood.pop() {
            self.cells.push(c);
            reach(adj, floor, c as usize, (&mut level, &mut self.outlet), self.cells.len(), &mut flood);
        }
        debug_assert_eq!(self.cells.len() + basin.len(), n, "the mesh is connected");
        // The basin's liquid stands, and no cell is visited after it.
        self.stands = (0..n).map(|i| floor[i] < level[i] || level[i] <= low).collect();
        basin.iter().for_each(|&c| self.stands[c as usize] = true);
        // The pass: the highest level first; at one level the last reached first (`r`: the order the flood reached them
        // in), so each cell comes before the cell it was reached from, its way out, whether its liquid stands or passes.
        let key = |(r, &c): (usize, &u32)| (!rising(level[c as usize]), !(r as u64), c);
        let mut keys: Vec<_> = self.cells.iter().enumerate().map(key).collect();
        keys.sort_unstable();
        self.cells = keys.iter().map(|k| k.2).collect();
        self.rank = vec![u32::MAX; n];
        self.cells.iter().enumerate().for_each(|(r, &c)| self.rank[c as usize] = r as u32);
        self.spill = level;
    }

    /// Is cell `a` visited before cell `b` (so it is passed when `b` is visited)?
    #[inline]
    pub fn before(&self, a: usize, b: usize) -> bool {
        self.rank[a] < self.rank[b]
    }
}

/// The flood reaches cell `c` (of `level`, m): each cell next to it not reached yet enters the heap at the higher of
/// its floor and that level, `c` its way out, after the `taken` cells taken out of the heap and those in it (equal
/// levels leave the heap in the order they entered it).
fn reach(adj: &Adjacency, floor: &[f64], c: usize, (level, outlet): (&mut [f64], &mut [u32]), taken: usize, flood: &mut BinaryHeap<Reverse<(u64, u32, u32)>>) {
    for j in adj.other[adj.start[c] as usize..adj.start[c + 1] as usize].iter().map(|&j| j as usize) {
        if level[j] == f64::INFINITY {
            let entered = (taken + flood.len()) as u32;
            (level[j], outlet[j]) = (floor[j].max(level[c]), c as u32);
            flood.push(Reverse((rising(level[j]), entered, j as u32)));
        }
    }
}

/// What crossed the links in a pass: the link of each crossing and what it carried of each row, `amount[c * rows + k]`,
/// + from the link's cell `a`.
#[derive(Clone, Debug, Default)]
pub struct Crossed {
    pub links: Vec<u32>,
    pub amount: Vec<f64>,
}

/// One pass down `order` over rows of amounts that move together (`q[i * rows + k]`): at each cell `rule(i, q, share)`
/// sets the share of what it holds that leaves along each entry (summing to at most 1); each row then moves by those
/// shares, no more than is left, and `crossed` gets what crossed each link.
pub fn pass(order: &Order, adj: &Adjacency, q: &mut [f64], crossed: &mut Crossed, mut rule: impl FnMut(usize, &mut [f64], &mut [f64])) {
    let rows = q.len() / (adj.start.len() - 1).max(1);
    let most = adj.start.windows(2).map(|s| (s[1] - s[0]) as usize).max().unwrap_or(0);
    let (mut share, mut out) = (vec![0.0; most], Vec::with_capacity(most));
    crossed.links.clear();
    crossed.amount.clear();
    for &c in &order.cells {
        let i = c as usize;
        let (e0, e1) = (adj.start[i] as usize, adj.start[i + 1] as usize);
        let share = &mut share[..e1 - e0];
        share.fill(0.0);
        rule(i, q, share);
        // The entries it sends along: the other cell, and the sign of the link's flux.
        out.clear();
        out.extend((e0..e1).zip(share.iter()).filter(|p| *p.1 > 0.0).map(|(e, &f)| {
            let l = adj.link[e];
            crossed.links.push(l);
            (adj.other[e] as usize, f, if adj.a[l as usize] as usize == i { 1.0 } else { -1.0 })
        }));
        let first = crossed.amount.len();
        crossed.amount.resize(first + out.len() * rows, 0.0);
        for k in (0..rows).filter(|_| !out.is_empty()) {
            let (held, mut gone) = (q[i * rows + k], 0.0);
            for (o, &(j, f, sign)) in out.iter().enumerate() {
                let m = (held * f).min(held - gone);
                q[j * rows + k] += m;
                crossed.amount[first + o * rows + k] = sign * m;
                gone += m;
            }
            q[i * rows + k] = held - gone;
        }
    }
}
