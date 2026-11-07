//! What a split or a join makes of the numbers of its cells (laws.md 0): a quarter holds its share of what its parent
//! held and its parent's numbers per area; a joined cell the sum of what its quarters held and the mean of their numbers
//! per area, and it keeps each quarter's shares (`Joined`), so its split gives the quarters back as they were.

use crate::store::Class;
use crate::World;
use serde::{Deserialize, Serialize};

/// A cell a split or a join made of old cells (`Source::Made`).
#[derive(Clone, Copy, Debug)]
pub enum Made {
    /// Quarter `q` (row by row) of old cell `parent`, `area` of its area and `bed` of its bed (its area's share of a
    /// flat cell's), and `cover[x]` of slot `x` of its cover (`Cover::parts`)
    Quarter { parent: u32, q: u8, area: f64, bed: f64, cover: [f64; crate::cover::SLOTS] },
    /// The join of old cells `kids`, its quarters row by row, each `area[q]` of its area
    Join { kids: [u32; 4], area: [f64; 4] },
}

/// A number a cell holds whose shares a join keeps (`Joined`): the amount of substance `k` in a class of its nodes
/// (`Class::CELLS`), the heat of one, a volume Flow keeps of its ground (`flow::volumes`).
#[derive(Clone, Copy, Debug)]
pub(crate) enum Of {
    Amount(Class, usize),
    Heat(Class),
    Flow(usize),
}

/// The volumes Flow keeps of a ground node that a cell holds (`flow::volumes`), and which of them is its bed's.
pub(crate) const FLOW_HELD: usize = 3;
pub(crate) const FLOW_BED: usize = 2;

impl Of {
    /// Whether it is a number of the cell's bed (its amounts, its volume), which a quarter holds by its share of the bed.
    fn of_bed(self) -> bool {
        matches!(self, Of::Amount(Class::Bed, _) | Of::Flow(FLOW_BED))
    }
}

impl Made {
    /// The old cells it reads.
    pub(super) fn old(&self) -> Vec<u32> {
        match *self {
            Made::Quarter { parent, .. } => vec![parent],
            Made::Join { kids, .. } => kids.to_vec(),
        }
    }
    /// What it holds of a number a cell holds, `at(i)` old cell `i`'s: a quarter its share, by its parent's record of
    /// `of` (`joined`, `Joined::share`) where it has one, else by its share of the bed for a number of the bed and by
    /// area for the others; a joined cell the sum of its quarters', in order.
    pub(crate) fn held(&self, joined: &[Option<Joined>], of: Option<Of>, at: impl Fn(u32) -> f64) -> f64 {
        match *self {
            Made::Quarter { parent, q, area, bed, .. } => {
                let record = of.zip(joined.get(parent as usize).and_then(Option::as_ref));
                at(parent) * record.map_or(if of.is_some_and(Of::of_bed) { bed } else { area }, |(of, r)| r.share(of, q))
            }
            Made::Join { kids, .. } => sum4(kids.map(at)),
        }
    }
    /// Its number per area: a quarter's is its parent's, a joined cell's the mean of its quarters' by area (`mean_by`).
    pub(crate) fn mean(&self, at: impl Fn(u32) -> f64) -> f64 {
        let area = if let Made::Join { area, .. } = self { *area } else { [0.0; 4] };
        self.pick(at, |x| mean_by(x, area))
    }
    /// A quarter takes its parent's `at`, a joined cell `join` of its quarters', row by row.
    pub(crate) fn pick<T: Copy>(&self, at: impl Fn(u32) -> T, join: impl FnOnce([T; 4]) -> T) -> T {
        match *self {
            Made::Quarter { parent, .. } => at(parent),
            Made::Join { kids, .. } => join(kids.map(at)),
        }
    }
}

/// The sum of four numbers in pairs, so four equal ones (a cell's flat quarters) give it exactly.
pub(crate) fn sum4(x: [f64; 4]) -> f64 {
    (x[0] + x[1]) + (x[2] + x[3])
}

/// The mean of `x` by the weights `wt`, taken about `x[0]` so that equal numbers give it exactly (`x[0]` where nothing
/// weighs).
pub(crate) fn mean_by(x: [f64; 4], wt: [f64; 4]) -> f64 {
    let (sum, of) = x.iter().zip(wt).fold((0.0, 0.0), |(s, w), (&v, wt)| (s + wt * (v - x[0]), w + wt));
    x[0] + if of > 0.0 { sum / of } else { 0.0 }
}

/// What a join keeps of its quarters (laws.md 0): each quarter's share of each number the cell holds (`Of`), and the
/// relief each held (`World::relief`), J, which its split gives back (`World::split`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Joined {
    /// Each quarter's share, row by row, of each number (`Joined::share`)
    shares: Vec<[f64; 4]>,
    pub kept: [f64; 4],
}

impl Joined {
    /// The shares of quarters `kids` (each `area[q]` of the joined cell's area) in the world before their join: each
    /// one's part of the four's sum, by area where they hold none, and the relief of each.
    pub(crate) fn of(w: &World, kids: [usize; 4], area: [f64; 4]) -> Joined {
        let (n, ns) = (w.mesh.cells().n(), w.chem.ns());
        let flow = kids.map(|i| crate::flow::volumes(w, i));
        let mut shares = Vec::with_capacity(Class::CELLS.len() * (ns + 1) + FLOW_HELD);
        let mut push = |x: [f64; 4]| {
            let sum = sum4(x);
            shares.push(if sum > 0.0 { x.map(|v| v / sum) } else { area });
        };
        for c in Class::CELLS {
            let a = w.store.amounts(c);
            (0..ns).for_each(|k| push(kids.map(|i| a[k * n + i])));
        }
        for c in Class::CELLS {
            let h = w.store.heat(c);
            push(if h.is_empty() { [0.0; 4] } else { kids.map(|i| h[i]) });
        }
        (0..FLOW_HELD).for_each(|x| push(flow.map(|v| v.map_or(0.0, |v| v[x]))));
        Joined { shares, kept: kids.map(|i| w.relief[i]) }
    }
    /// Quarter `q`'s share of `of`.
    pub(crate) fn share(&self, of: Of, q: u8) -> f64 {
        let classes = Class::CELLS.len();
        let ns = (self.shares.len() - classes - FLOW_HELD) / classes;
        let place = |c: Class| Class::CELLS.iter().position(|&x| x == c).expect("a class of a cell's nodes");
        let at = match of {
            Of::Amount(c, k) => place(c) * ns + k,
            Of::Heat(c) => classes * ns + place(c),
            Of::Flow(x) => classes * (ns + 1) + x,
        };
        self.shares[at][q as usize]
    }
}
