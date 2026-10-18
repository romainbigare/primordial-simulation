//! Downhill: the cells in order from the highest head to the lowest, and one pass down that order that moves what
//! stands on each cell to its lower neighbours.
//!
//! Liquid on a surface runs from a higher head to a lower one, so the links on which it runs make a graph without
//! cycles, and an order of the cells by falling head is a topological order of it (Kahn 1962): a cell is visited
//! after every cell that can send to it. One pass in that order lets what leaves the highest cell cross the whole map
//! in one step, while depressions fill and spill (the "priority flood" of Barnes and others 2014 orders cells the same
//! way). The order is brought up to date by sorting the cells anew by their heads (`Order::update`): the heads of a
//! pool of standing liquid change rank among themselves at every step, so mending the last order costs more than a
//! new sort of the keys.
//!
//! The law says how much leaves a cell (`pass`'s rule); this module does the order, the moves and the books.

/// A mesh's adjacency as plain slices. The entries of cell `i` are `start[i]..start[i + 1]`; each entry names a link
/// and the other cell of it. A link's flux is + from its cell `a` to its other cell.
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

/// The cells from the highest head to the lowest, ties by cell, and the rank of each cell in that order.
#[derive(Clone, Debug, Default)]
pub struct Order {
    cells: Vec<u32>,
    rank: Vec<u32>,
}

/// A key that sorts heads from the highest down: the bits of `h` in the total order of `f64::total_cmp`, reversed.
#[inline]
fn falling(h: f64) -> u64 {
    let b = h.to_bits();
    !(if b >> 63 == 1 { !b } else { b | 1 << 63 })
}

impl Order {
    /// Bring the order up to date for the heads `head` (one per cell, m).
    pub fn update(&mut self, head: &[f64]) {
        let mut keys: Vec<(u64, u32)> = head.iter().enumerate().map(|(i, &h)| (falling(h), i as u32)).collect();
        keys.sort_unstable();
        self.cells = keys.iter().map(|k| k.1).collect();
        self.rank.resize(head.len(), 0);
        for (r, &c) in self.cells.iter().enumerate() {
            self.rank[c as usize] = r as u32;
        }
    }

    /// The cells, highest head first.
    pub fn cells(&self) -> &[u32] {
        &self.cells
    }

    /// Is cell `a` visited before cell `b` (so it is passed when `b` is visited)?
    #[inline]
    pub fn before(&self, a: usize, b: usize) -> bool {
        self.rank[a] < self.rank[b]
    }
}

/// What crossed the links in a pass, crossing by crossing: the link of each, `links[c]`, and what it carried of each
/// row, `amount[c * rows + k]`, + from the link's cell `a` to the other. A link may cross more than once.
#[derive(Clone, Debug, Default)]
pub struct Crossed {
    pub links: Vec<u32>,
    pub amount: Vec<f64>,
}

/// One pass down `order`. `q` holds rows of amounts that move together, the rows of each cell side by side
/// (`q[i * rows + k]`). At each cell `i` in turn, `rule(i, q, share)` says which share of what `i` holds now leaves
/// along each of its entries in `adj` (`share` has one value per entry, all 0 on entry, summing to at most 1); the
/// rule may read the amounts of every cell and change those of `i` before they leave. Each row then moves by those
/// shares (no more than is left, so rounding never takes a cell below zero): what one cell loses another gains, and
/// `crossed` (emptied first) gets what crossed each link.
pub fn pass(order: &Order, adj: &Adjacency, q: &mut [f64], crossed: &mut Crossed, mut rule: impl FnMut(usize, &mut [f64], &mut [f64])) {
    let n = order.cells.len();
    let rows = q.len() / n.max(1);
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
