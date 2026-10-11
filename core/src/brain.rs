//! A small brain, shared by all bodies of one lineage: one hidden layer over what the body
//! perceives, fixed action scores, and two shared scorers (one used for every neighbour
//! direction, one for every perceived body) whose feature weights depend on the hidden layer.
//! Kept apart from the laws so it can be swapped.

use crate::rng::Rng;

/// Hidden units.
pub const H: usize = 12;
/// Fixed actions: rest, copy, put, take from the node.
pub const FIXED: usize = 4;

/// Sizes of the inputs, the direction features and the body features.
#[derive(Clone, Copy, Debug)]
pub struct Shape {
    pub ni: usize,
    pub nf: usize,
    pub nt: usize,
}

impl Shape {
    fn b1(&self) -> usize {
        H * self.ni
    }
    fn w2(&self) -> usize {
        self.b1() + H
    }
    fn dir(&self) -> usize {
        self.w2() + FIXED * (H + 1)
    }
    fn body(&self) -> usize {
        self.dir() + (self.nf + 1) * (H + 1)
    }
    pub fn len(&self) -> usize {
        self.body() + (self.nt + 1) * (H + 1)
    }
}

/// What one evaluation gives: fixed action scores and the weights of the two scorers
/// ([bias, one per feature]) for this situation.
#[derive(Default)]
pub struct Thought {
    pub fixed: [f32; FIXED],
    pub dir: Vec<f32>,
    pub body: Vec<f32>,
}

/// Rows of `w` (each H weights then a bias) applied to `h`, into `out`.
fn rows(w: &[f32], h: &[f32; H], out: &mut [f32]) {
    for (o, r) in out.iter_mut().zip(w.chunks_exact(H + 1)) {
        *o = r[H] + r[..H].iter().zip(h).map(|(a, b)| a * b).sum::<f32>();
    }
}

/// Dot product in 8 lanes (so it can use vector instructions).
#[inline]
fn dot(a: &[f32], b: &[f32]) -> f32 {
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

pub fn think(w: &[f32], s: Shape, x: &[f32], t: &mut Thought) {
    let mut h = [0f32; H];
    for (m, hm) in h.iter_mut().enumerate() {
        *hm = squash(w[s.b1() + m] + dot(&w[m * s.ni..(m + 1) * s.ni], x));
    }
    rows(&w[s.w2()..s.dir()], &h, &mut t.fixed);
    t.dir.resize(s.nf + 1, 0.0);
    t.body.resize(s.nt + 1, 0.0);
    rows(&w[s.dir()..s.body()], &h, &mut t.dir);
    rows(&w[s.body()..s.len()], &h, &mut t.body);
}

/// A cheap tanh (error below 0.025).
#[inline]
fn squash(x: f32) -> f32 {
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

/// Score of one option from its features with a scorer's weights.
#[inline]
pub fn score(c: &[f32], f: &[f32]) -> f32 {
    c[0] + c[1..].iter().zip(f).map(|(a, b)| a * b).sum::<f32>()
}

/// Small random weights, then the biases given for the fixed actions and the two scorers.
pub fn random(s: Shape, r: &mut Rng, size: f32, fixed: [f32; FIXED], dir: f32, body: f32) -> Vec<f32> {
    let mut w: Vec<f32> = (0..s.len()).map(|_| size * r.normal() as f32).collect();
    for (a, b) in fixed.iter().enumerate() {
        w[s.w2() + a * (H + 1) + H] = *b;
    }
    w[s.dir() + H] = dir;
    w[s.body() + H] = body;
    w
}

pub fn mutate(w: &mut [f32], r: &mut Rng, size: f32) {
    for x in w {
        *x += size * r.normal() as f32;
    }
}
