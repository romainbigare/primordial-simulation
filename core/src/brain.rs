//! A small brain. Its base (one hidden layer over what a body perceives and remembers) belongs
//! to a lineage and changes only by evolution. Its rows are genes of each body, and a group of
//! bodies shares their mean plus what the group learned: read from
//! the hidden layer, they give fixed action scores, the weights of shared scorers (one used for
//! every neighbour direction, one per verb for every perceived body or part), the mean of the
//! signal and of the shape of a part made, the next memory, and an estimate of how well-being
//! will go. Within a life the rows that
//! pick actions and signals follow a policy gradient against that estimate (actor-critic); all
//! rows also change by imitation. Kept apart from the laws so it can be swapped.

use crate::rng::Rng;

/// Hidden units.
pub const H: usize = 12;
/// Fixed actions: rest, copy, put to the node, take from the node, form a part, let go (or
/// leave the inside), join the held part to the ground, break the bonded ground.
pub const FIXED: usize = 8;
/// Verbs toward a perceived body: take from it, put to it, watch it.
pub const VERBS: usize = 3;
/// Verbs toward a perceived part: hold it, take from it, put to it, join the held part to it,
/// break its bonds, enter the inside of its structure.
pub const OVERBS: usize = 6;
/// Signal outputs and memory numbers.
pub const SIG: usize = 2;
pub const MEM: usize = 6;
/// One row: H weights, then a bias.
pub const R: usize = H + 1;

/// Sizes of the inputs, the direction features, the body features and the part features.
#[derive(Clone, Copy, Debug)]
pub struct Shape {
    pub ni: usize,
    pub nf: usize,
    pub nt: usize,
    pub no: usize,
}

impl Shape {
    /// Length of the base.
    pub fn base(&self) -> usize {
        H * self.ni + H
    }
    /// First row of the direction scorer, of a verb's scorer (body verbs, then part verbs), of
    /// the signal, of the shape of a part made (elongation, hollowness), of the memory.
    pub fn dir(&self) -> usize {
        FIXED
    }
    pub fn verb(&self, v: usize) -> usize {
        let b = FIXED + self.nf + 1 + v.min(VERBS) * (self.nt + 1);
        b + v.saturating_sub(VERBS) * (self.no + 1)
    }
    pub fn sig(&self) -> usize {
        self.verb(VERBS + OVERBS)
    }
    pub fn form(&self) -> usize {
        self.sig() + SIG
    }
    /// Rows that learn within a life (all but the memory rows).
    pub fn learned(&self) -> usize {
        self.form() + 2
    }
    /// The row that estimates how well-being will go from here (the critic).
    pub fn critic(&self) -> usize {
        self.learned() + MEM
    }
    pub fn rows(&self) -> usize {
        self.critic() + 1
    }
    pub fn len(&self) -> usize {
        self.base() + self.rows() * R
    }
}

/// What one evaluation gives: the hidden layer and every row's value.
#[derive(Default)]
pub struct Thought {
    pub h: [f32; H],
    pub out: Vec<f32>,
}

/// Dot product in 8 lanes (so it can use vector instructions).
#[inline]
pub fn dot(a: &[f32], b: &[f32]) -> f32 {
    let mut acc = [0f32; 8];
    let (ca, cb) = (a.chunks_exact(8), b.chunks_exact(8));
    let tail: f32 = ca.remainder().iter().zip(cb.remainder()).map(|(x, y)| x * y).sum();
    for (p, q) in ca.zip(cb) {
        for l in 0..8 {
            acc[l] += p[l] * q[l];
        }
    }
    acc.iter().sum::<f32>() + tail
}

/// `base` is the lineage's (its first `s.base()` numbers are used), `rows` the group's.
pub fn think(base: &[f32], rows: &[f32], s: Shape, x: &[f32], t: &mut Thought) {
    let b1 = H * s.ni;
    for (m, hm) in t.h.iter_mut().enumerate() {
        *hm = squash(base[b1 + m] + dot(&base[m * s.ni..(m + 1) * s.ni], x));
    }
    t.out.clear();
    let h = &t.h;
    t.out.extend(rows.chunks_exact(R).map(|r| {
        let r: &[f32; R] = r.try_into().expect("row");
        r[H] + (0..H).map(|m| r[m] * h[m]).sum::<f32>()
    }));
}

/// A cheap tanh (error below 0.025).
#[inline]
pub fn squash(x: f32) -> f32 {
    let x = x.clamp(-3.0, 3.0);
    x * (27.0 + x * x) / (27.0 + 9.0 * x * x)
}

/// A cheap natural log (error below 0.005), enough for what a brain perceives.
#[inline]
pub fn ln(x: f32) -> f32 {
    let b = x.max(1e-30).to_bits();
    let m = f32::from_bits(b & 0x7f_ffff | 0x3f80_0000) - 1.0;
    ((b >> 23) as f32 - 127.0 + m * (1.3466 - 0.3466 * m)) * std::f32::consts::LN_2
}

/// Score of one option from its features with a scorer's weights ([bias, one per feature]).
#[inline]
pub fn score(c: &[f32], f: &[f32]) -> f32 {
    c[0] + c[1..].iter().zip(f).map(|(a, b)| a * b).sum::<f32>()
}

/// Add `a · coef[r] · [h, 1]` to each learned row `r`, the step kept below `MAX_STEP`.
pub fn learn(rows: &mut [f32], h: &[f32; H], coef: &[f32], a: f32) {
    const MAX_STEP: f32 = 0.1;
    let hh = 1.0 + h.iter().map(|x| x * x).sum::<f32>();
    let size = a.abs() * (coef.iter().map(|c| c * c).sum::<f32>() * hh).sqrt();
    let a = if size > MAX_STEP { a * MAX_STEP / size } else { a };
    for (r, &c) in rows.chunks_exact_mut(R).zip(coef) {
        let c = a * c;
        if c != 0.0 {
            for m in 0..H {
                r[m] += c * h[m];
            }
            r[H] += c;
        }
    }
}

/// Move `w` a share `a` of the way toward `o`.
pub fn blend(w: &mut [f32], o: &[f32], a: f32) {
    for (x, y) in w.iter_mut().zip(o) {
        *x += a * (y - *x);
    }
}

/// Small random weights, then the biases given for the fixed actions, moving and the verbs
/// (toward bodies, then toward parts).
pub fn random(s: Shape, r: &mut Rng, size: f32, fixed: [f32; FIXED], dir: f32, verbs: [f32; VERBS + OVERBS]) -> Vec<f32> {
    let mut w: Vec<f32> = (0..s.len()).map(|_| size * r.normal() as f32).collect();
    let row = |q: usize| s.base() + q * R + H;
    for (a, b) in fixed.iter().enumerate() {
        w[row(a)] = *b;
    }
    w[row(s.dir())] = dir;
    for (v, b) in verbs.iter().enumerate() {
        w[row(s.verb(v))] = *b;
    }
    w
}

