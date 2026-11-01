//! A small brain (laws.md 5.1): a base of one hidden layer, the lineage's, and rows read from it, each body's, that
//! give action scores, shared scorers, a part's shape, the next memory and a critic. Kept apart from the
//! laws so it can be swapped.

use crate::rng::Rng;
use serde::{Deserialize, Serialize};

/// The fixed actions, by their rows: rest; put to the node; take from it; form a part; let go (or leave the inside);
/// join the held part to the ground; break the bonded ground. No copy: children come from the brood.
pub const REST: usize = 0;
pub const PUT: usize = 1;
pub const TAKE: usize = 2;
pub const FORM: usize = 3;
pub const RELEASE: usize = 4;
pub const ANCHOR: usize = 5;
pub const DIG: usize = 6;
pub const FIXED: usize = 7;
/// Verbs toward a perceived body: take from it, put to it, watch it.
pub const TAKE_FROM: usize = 0;
pub const GIVE: usize = 1;
pub const WATCH: usize = 2;
pub const VERBS: usize = 3;
/// Verbs toward a perceived part: hold, take from, put to, join the held part to, break its bonds, enter its inside.
pub const OVERBS: usize = 6;
/// Outputs for the shape of a part made: elongation, hollowness.
pub(crate) const FORM_OUT: usize = 2;

/// Sizes of a brain: hidden units, memory numbers, inputs (the memory last), direction, body and part features. A row
/// is `h` weights, then a bias.
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
    /// First row of the direction scorer, of a verb's scorer, of a part's shape.
    pub fn dir(&self) -> usize {
        FIXED
    }
    pub fn verb(&self, v: usize) -> usize {
        let b = FIXED + self.nf + 1 + v.min(VERBS) * (self.nt + 1);
        b + v.saturating_sub(VERBS) * (self.no + 1)
    }
    pub fn form(&self) -> usize {
        self.verb(VERBS + OVERBS)
    }
    /// Rows that learn within a life (all but the memory rows).
    pub fn learned(&self) -> usize {
        self.form() + FORM_OUT
    }
    /// The row that estimates how well-being will go from here (the critic).
    pub(crate) fn critic(&self) -> usize {
        self.learned() + self.mem
    }
    pub fn rows(&self) -> usize {
        self.critic() + 1
    }
    /// Number of weights: the base and every row.
    pub fn weights(&self) -> usize {
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

/// The sums of the hidden layer, before it squashes them, of bodies of one lineage of base `base` and inputs `xs` (each
/// body's `s.ni` one after another): one matrix product of the inputs with the base's weights, plus its biases, into
/// `z` (each body's `s.h` one after another).
pub(crate) fn sums(base: &[f32], s: Shape, xs: &[f32], z: &mut Vec<f32>) {
    let b1 = s.h * s.ni;
    z.clear();
    for x in xs.chunks_exact(s.ni.max(1)) {
        z.extend((0..s.h).map(|m| base[b1 + m] + dot(&base[m * s.ni..(m + 1) * s.ni], x)));
    }
}

/// What a body with rows `rows` thinks from the sums of its hidden layer `z` (`sums`): its hidden layer and every row's
/// value.
pub(crate) fn think(rows: &[f32], s: Shape, z: &[f32], t: &mut Thought) {
    t.h.clear();
    t.h.extend(z.iter().map(|&v| squash(v)));
    // The rows four at a time (four sums that do not wait on each other), then the rest.
    t.out.clear();
    let (n, r) = (s.h, s.r());
    let h = &t.h[..n];
    let mut quads = rows.chunks_exact(4 * r);
    for q in &mut quads {
        let (r0, r1, r2, r3) = (&q[..n], &q[r..r + n], &q[2 * r..2 * r + n], &q[3 * r..3 * r + n]);
        let mut acc = [q[n], q[r + n], q[2 * r + n], q[3 * r + n]];
        for m in 0..n {
            let x = h[m];
            (acc[0], acc[1], acc[2], acc[3]) = (acc[0] + r0[m] * x, acc[1] + r1[m] * x, acc[2] + r2[m] * x, acc[3] + r3[m] * x);
        }
        t.out.extend(acc);
    }
    t.out.extend(quads.remainder().chunks_exact(s.r()).map(|r| r[n] + dot(&r[..n], h)));
}

/// A cheap tanh (error below 0.025).
#[inline]
pub(crate) fn squash(x: f32) -> f32 {
    let x = x.clamp(-3.0, 3.0);
    x * (27.0 + x * x) / (27.0 + 9.0 * x * x)
}

/// A brain perceives the ln of a mass (of a member, kg, or over its own) over this: a factor of about 1,100 is one unit.
pub(crate) const LN_MASS: f32 = 7.0;

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

/// Move each learned row `r` by `lr · g / rms · [h, 1]`, `g = a · coef[r]` its share of the step and `rms` the root of
/// a moving average of its squares (`ms`), capped at `most · lr`: a row that rarely matters learns as fast when it does.
pub fn learn(rows: &mut [f32], h: &[f32], coef: &[f32], a: f32, lr: f32, ms: &mut [f32], (beta, most): (f32, f32)) {
    debug_assert_eq!(ms.len(), coef.len(), "a moving average per learned row");
    for ((r, &c), v) in rows.chunks_exact_mut(h.len() + 1).zip(coef).zip(ms.iter_mut()) {
        let g = a * c;
        if g == 0.0 {
            continue;
        }
        *v += beta * (g * g - *v);
        step(r, h, lr * (g / v.sqrt()).clamp(-most, most));
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
pub(crate) fn blend(w: &mut [f32], o: &[f32], a: f32) {
    for (x, y) in w.iter_mut().zip(o) {
        *x += a * (y - *x);
    }
}

/// Small random weights, then the biases given for the fixed actions, moving and the verbs.
pub fn random(s: Shape, r: &mut Rng, size: f32, fixed: [f32; FIXED], dir: f32, verbs: [f32; VERBS + OVERBS]) -> Vec<f32> {
    let mut w: Vec<f32> = (0..s.weights()).map(|_| size * r.normal() as f32).collect();
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

/// The base of shape `a` carried over to shape `b`: shared weights keep their values, new ones are noise of spread `size`.
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

/// Rows of shape `a` carried over to shape `b`: matched by what they do, weights by hidden unit, the bias kept.
pub fn rerow(w: &[f32], a: Shape, b: Shape, r: &mut Rng, size: f32) -> Vec<f32> {
    // A brain of no hidden units has no born rows (nothing reads them); from none, every weight is new.
    if b.h == 0 {
        return Vec::new();
    }
    let l = b.learned();
    let row = |q: usize| match q {
        q if q < l => Some(q),
        q if q == b.critic() => Some(a.critic()),
        q => (q - l < a.mem).then_some(q),
    };
    let mut o = Vec::with_capacity(b.rows() * b.r());
    for q in 0..b.rows() {
        let src = row(q).filter(|_| !w.is_empty()).map(|p| &w[p * a.r()..(p + 1) * a.r()]);
        o.extend((0..b.h).map(|m| match src {
            Some(s) if m < a.h => s[m],
            _ => size * r.normal() as f32,
        }));
        o.push(src.map_or(size * r.normal() as f32, |s| s[a.h]));
    }
    o
}

/// The step of a node's born rows for mutation size `size`: `row_noise` over sqrt(`row_chance`), as only that share
/// of children change theirs.
pub fn row_step(p: &crate::Physics, size: f32) -> f32 {
    p.row_noise * size / (p.row_chance as f32).sqrt()
}

/// The number of values of an 8-bit uniform.
const U8_SPAN: f32 = 256.0;

/// Add noise near a normal of spread `size` to each weight: a sum of four 8-bit uniforms, centred and scaled, two
/// weights from one draw.
pub fn shake(w: &mut [f32], r: &mut Rng, size: f32) {
    const B: u64 = 0x00ff_00ff_00ff_00ff;
    let (k, mean) = (size * 3f32.sqrt() / U8_SPAN, 2.0 * (U8_SPAN - 1.0));
    // Bytes summed in pairs (16-bit lanes), then lanes in pairs: one sum per 32 bits.
    let sums = |u: u64| {
        let a = (u & B) + (u >> 8 & B);
        let t = a + (a >> 16);
        [(t & 0xffff) as f32 - mean, (t >> 32 & 0xffff) as f32 - mean]
    };
    let mut pairs = w.chunks_exact_mut(2);
    for p in &mut pairs {
        let s = sums(r.u64());
        (p[0], p[1]) = (p[0] + k * s[0], p[1] + k * s[1]);
    }
    if let [x] = pairs.into_remainder() {
        *x += k * sums(r.u64())[0];
    }
}
