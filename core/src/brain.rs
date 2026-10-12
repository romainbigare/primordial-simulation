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
use serde::{Deserialize, Serialize};

/// Fixed actions: rest, copy, put to the node, take from the node, form a part, let go (or
/// leave the inside), join the held part to the ground, break the bonded ground.
pub const FIXED: usize = 8;
/// Verbs toward a perceived body: take from it, put to it, watch it.
pub const VERBS: usize = 3;
/// Verbs toward a perceived part: hold it, take from it, put to it, join the held part to it,
/// break its bonds, enter the inside of its structure.
pub const OVERBS: usize = 6;
/// Signal outputs.
pub const SIG: usize = 2;

/// Sizes of a brain: hidden units, memory numbers, inputs (the memory last), direction
/// features, body features and part features. A row is `h` weights, then a bias.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Shape {
    pub h: usize,
    pub mem: usize,
    pub ni: usize,
    pub nf: usize,
    pub nt: usize,
    pub no: usize,
}

impl Shape {
    /// Length of a row.
    pub fn r(&self) -> usize {
        self.h + 1
    }
    /// Length of the base.
    pub fn base(&self) -> usize {
        self.h * self.ni + self.h
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
        self.learned() + self.mem
    }
    pub fn rows(&self) -> usize {
        self.critic() + 1
    }
    pub fn len(&self) -> usize {
        self.base() + self.rows() * self.r()
    }
}

/// What one evaluation gives: the hidden layer and every row's value.
#[derive(Default)]
pub struct Thought {
    pub h: Vec<f32>,
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

/// `base` is the lineage's, `rows` the group's.
pub fn think(base: &[f32], rows: &[f32], s: Shape, x: &[f32], t: &mut Thought) {
    let b1 = s.h * s.ni;
    t.h.clear();
    t.h.extend((0..s.h).map(|m| squash(base[b1 + m] + dot(&base[m * s.ni..(m + 1) * s.ni], x))));
    t.out.clear();
    let h = &t.h;
    t.out.extend(rows.chunks_exact(s.r()).map(|r| r[s.h] + dot(&r[..s.h], h)));
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

/// Add `a · coef[r] · [h, 1]` to each row `r` (the critic's), the step kept below 0.1.
pub fn fit(rows: &mut [f32], h: &[f32], coef: &[f32], a: f32) {
    const MAX_STEP: f32 = 0.1;
    let hh = 1.0 + h.iter().map(|x| x * x).sum::<f32>();
    let size = a.abs() * (coef.iter().map(|c| c * c).sum::<f32>() * hh).sqrt();
    let a = if size > MAX_STEP { a * MAX_STEP / size } else { a };
    for (r, &c) in rows.chunks_exact_mut(h.len() + 1).zip(coef) {
        if c != 0.0 {
            step(r, h, a * c);
        }
    }
}

/// Weight of the newest step in each row's mean square of steps.
const BETA: f32 = 0.1;
/// Most a row moves in one step, in units of the learning rate (times [h, 1]).
const MOST: f32 = 0.5;

/// Move each learned row `r` by `lr · g / rms · [h, 1]`, where `g = a · coef[r]` is its share of
/// the step and `rms` the root of a moving average of its squares (kept in `ms`), capped at
/// `MOST · lr`. Each row thus gets a step of its own size: a row that rarely matters (a
/// feature seen only now and then) learns as fast when it does as one that matters every time.
pub fn learn(rows: &mut [f32], h: &[f32], coef: &[f32], a: f32, lr: f32, ms: &mut Vec<f32>) {
    ms.resize(coef.len(), 0.0);
    for ((r, &c), v) in rows.chunks_exact_mut(h.len() + 1).zip(coef).zip(ms.iter_mut()) {
        let g = a * c;
        if g == 0.0 {
            continue;
        }
        *v += BETA * (g * g - *v);
        step(r, h, lr * (g / v.sqrt()).clamp(-MOST, MOST));
    }
}

/// Add `c · [h, 1]` to a row.
#[inline]
fn step(r: &mut [f32], h: &[f32], c: f32) {
    let (w, b) = r.split_at_mut(h.len());
    w.iter_mut().zip(h).for_each(|(w, x)| *w += c * x);
    b[0] += c;
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
    let row = |q: usize| s.base() + q * s.r() + s.h;
    for (a, b) in fixed.iter().enumerate() {
        w[row(a)] = *b;
    }
    w[row(s.dir())] = dir;
    for (v, b) in verbs.iter().enumerate() {
        w[row(s.verb(v))] = *b;
    }
    w
}


/// The base of shape `a` carried over to shape `b`: weights both shapes have keep their values
/// (the memory inputs matched from their first), new ones are noise of spread `size`.
pub fn rebase(w: &[f32], a: Shape, b: Shape, r: &mut Rng, size: f32) -> Vec<f32> {
    let fixed = b.ni - b.mem;
    let input = |i: usize| if i < fixed { Some(i) } else { (i - fixed < a.mem).then(|| a.ni - a.mem + i - fixed) };
    let mut o = Vec::with_capacity(b.base());
    for m in 0..b.h {
        o.extend((0..b.ni).map(|i| match input(i) {
            Some(j) if m < a.h => w[m * a.ni + j],
            _ => size * r.normal() as f32,
        }));
    }
    o.extend((0..b.h).map(|m| if m < a.h { w[a.h * a.ni + m] } else { size * r.normal() as f32 }));
    o
}

/// Rows of shape `a` carried over to shape `b`, the same way: rows matched by what they do
/// (the memory rows from their first), weights by hidden unit, the bias kept.
pub fn rerow(w: &[f32], a: Shape, b: Shape, r: &mut Rng, size: f32) -> Vec<f32> {
    let l = b.learned();
    let row = |q: usize| match q {
        q if q < l => Some(q),
        q if q == b.critic() => Some(a.critic()),
        q => (q - l < a.mem).then_some(q),
    };
    let mut o = Vec::with_capacity(b.rows() * b.r());
    for q in 0..b.rows() {
        let src = row(q).map(|p| &w[p * a.r()..(p + 1) * a.r()]);
        o.extend((0..b.h).map(|m| match src {
            Some(s) if m < a.h => s[m],
            _ => size * r.normal() as f32,
        }));
        o.push(src.map_or(size * r.normal() as f32, |s| s[a.h]));
    }
    o
}
