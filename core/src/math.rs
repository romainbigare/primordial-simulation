//! Small functions of decay the laws share, members passing a level (`Pass`), an exact sum, the bits of a set, the
//! slope along a line of points, and a short list held in place (`Few`).

/// 1 - exp(-x), exact for small `x`: the share a first-order decay over `x` time constants takes.
#[inline]
pub(crate) fn one_minus_exp(x: f64) -> f64 {
    -(-x).exp_m1()
}

/// What rounding took from `a + b`, given their computed sum `s`, exactly (Neumaier 1974): kept apart, it makes a sum
/// exact.
#[inline]
pub(crate) fn rounding_lost(a: f64, b: f64, s: f64) -> f64 {
    if a.abs() >= b.abs() { (a - s) + b } else { (b - s) + a }
}

/// The slope at point `i` of a line of `n` points `len` apart whose values are `f(0..n)`: central differences,
/// one-sided at the line's ends (none on a line of one point).
#[inline]
pub(crate) fn central(i: usize, n: usize, f: impl Fn(usize) -> f64, len: f64) -> f64 {
    let (lo, hi) = (i.saturating_sub(1), (i + 1).min(n - 1));
    if hi > lo { (f(hi) - f(lo)) / (len * (hi - lo) as f64) } else { 0.0 }
}

/// (1 - exp(-x)) / x: the mean over a time of a pulse that decays `x` time constants in it (grows, below zero; 1 within
/// 1e-9 of zero).
#[inline]
pub(crate) fn mean_decay(x: f64) -> f64 {
    if x.abs() < 1e-9 { 1.0 } else { one_minus_exp(x) / x }
}

/// Below this many time constants `mean_passed` sums its series: the difference would lose digits.
const SERIES_BELOW: f64 = 1e-3;

/// (x - 1 + exp(-x)) / x²: the mean over a time of what a pulse that decays `x` time constants in it has passed by each
/// moment, per its first rate and the time (`mean_decay` at each moment, weighted by the moment); 1/2 with no decay.
#[inline]
pub(crate) fn mean_passed(x: f64) -> f64 {
    if x.abs() < SERIES_BELOW { 0.5 - x / 6.0 + x * x / 24.0 } else { (1.0 - mean_decay(x)) / x }
}

/// The rings of ground that members standing on nested shares `shares` of it make (a member of a smaller share stands
/// within the ground of each larger one): each ring the share s its members stand on at least, and its own share of
/// the ground, from the largest down.
pub(crate) fn rings(shares: impl Iterator<Item = f64>) -> Vec<(f64, f64)> {
    let mut s: Vec<f64> = shares.collect();
    s.sort_by(|a, b| b.total_cmp(a));
    s.dedup();
    (0..s.len()).map(|k| (s[k], s[k] - s.get(k + 1).copied().unwrap_or(0.0))).filter(|r| r.1 > 0.0).collect()
}

/// Light passing down through the crowns of members over the ground they share (Beer's law): `l` holds per member its
/// crown's base and top, m, its capture area per ground area x, the share s of the ground it stands on, and a tag. A
/// member stands on its share s, the shares nested (a kind stands where the loose ground holds its roots, and ground that
/// holds deep roots holds shallow ones), with capture area x / s per unit of it spread evenly through its crown (a crown of
/// no depth a sheet at its top); so the ground falls into rings (`rings`), each held by the members of a share at least
/// its own, and in each the light passes down through the capture area of all the crowns at each height, whatever
/// member holds it, absorbing 1 - exp(-y) of what reaches it over capture area y per unit of ground (with every s 1,
/// Beer's law through the canopy). `set(tag, f, bottom, depth)` gets each member's light per unit of capture area over
/// the mean of all (weighted by capture area, the factors have mean 1), the light at its crown's base per unit of capture
/// area over the same mean, and the capture area per unit of ground the light passes through its crown (ln of the light
/// at its top over that at its base): each the mean over its rings by its capture area in each.
pub(crate) fn shade<T: Copy>(l: &[(f64, f64, f64, f64, T)], mut set: impl FnMut(T, f64, f64, f64)) {
    let share = |s: f64| s.max(1e-300);
    let mut got = vec![[0.0; 3]; l.len()];
    for (at_least, ring) in rings(l.iter().map(|m| share(m.3))) {
        let on: Vec<usize> = (0..l.len()).filter(|&q| share(l[q].3) >= at_least).collect();
        let mut heights: Vec<f64> = on.iter().flat_map(|&q| [l[q].0, l[q].1]).collect();
        heights.sort_by(|a, b| b.total_cmp(a));
        heights.dedup();
        // The capture area per unit of ground the light has passed, the light each member's capture area gets, and
        // what the light has passed at each one's top and base.
        let (mut tau, mut lit, mut top, mut base) = (0.0f64, vec![0.0; l.len()], vec![0.0; l.len()], vec![0.0; l.len()]);
        for (k, &h) in heights.iter().enumerate() {
            on.iter().filter(|&&q| l[q].1 == h).for_each(|&q| top[q] = tau);
            // The crowns of no depth at this height: one sheet.
            let sheet = |q: usize| l[q].0 == h && l[q].1 == h;
            let y: f64 = on.iter().filter(|&&q| sheet(q)).map(|&q| l[q].2 / share(l[q].3)).sum();
            on.iter().filter(|&&q| sheet(q)).for_each(|&q| lit[q] += (-tau).exp() * mean_decay(y));
            tau += y;
            on.iter().filter(|&&q| sheet(q)).for_each(|&q| base[q] = tau);
            let Some(&below) = heights.get(k + 1) else { break };
            // The crowns through the span down to the next height, each its capture area spread over its depth.
            let through = |q: usize| l[q].0 <= below && l[q].1 >= h && l[q].1 > l[q].0;
            let part = |q: usize| (h - below) / (l[q].1 - l[q].0);
            let y: f64 = on.iter().filter(|&&q| through(q)).map(|&q| l[q].2 / share(l[q].3) * part(q)).sum();
            on.iter().filter(|&&q| through(q)).for_each(|&q| lit[q] += part(q) * (-tau).exp() * mean_decay(y));
            tau += y;
            on.iter().filter(|&&q| l[q].0 == below && l[q].1 > below).for_each(|&q| base[q] = tau);
        }
        for &q in &on {
            let part = ring / share(l[q].3);
            got[q] = [got[q][0] + part * lit[q], got[q][1] + part * (-base[q]).exp(), got[q][2] + part * (base[q] - top[q])];
        }
    }
    let (sum_x, sum_a) = got.iter().zip(l.iter()).fold((0.0, 0.0), |(sx, sa), (g, m)| (sx + m.2, sa + m.2 * g[0]));
    let all = if sum_x > 0.0 { sum_a / sum_x } else { 1.0 };
    got.iter().zip(l.iter()).for_each(|(g, m)| set(m.4, g[0] / all.max(1e-300), g[1] / all.max(1e-300), g[2]));
}

/// Veltkamp's constant, 2^27 + 1: it splits a number into two halves of 26 bits (`add_times`).
const SPLIT: f64 = 134_217_729.0;

/// Adds `x` to `parts`, a sum held exactly as numbers that do not overlap, the smallest first (Shewchuk 1997, Grow-
/// Expansion with zero elimination): terms that nearly cancel keep what is left of them.
#[inline]
fn grow(parts: &mut Vec<f64>, x: f64) {
    let (mut x, mut kept) = (x, 0);
    for j in 0..parts.len() {
        // `x + y` and its rounding error (Knuth 1969).
        let (y, s) = (parts[j], x + parts[j]);
        let bv = s - x;
        let err = (x - (s - bv)) + (y - bv);
        if err != 0.0 {
            parts[kept] = err;
            kept += 1;
        }
        x = s;
    }
    parts.truncate(kept);
    if x != 0.0 {
        parts.push(x);
    }
}

/// Adds `n x` to `parts` exactly, for a whole `n` of at most 26 bits: `x` split into two halves of 26 bits (Veltkamp),
/// each of whose products with `n` is exact.
#[inline]
pub(crate) fn add_times(parts: &mut Vec<f64>, n: f64, x: f64) {
    let t = SPLIT * x;
    let hi = t - (t - x);
    grow(parts, n * hi);
    grow(parts, n * (x - hi));
}

/// Quinn's contour lengths, in sides of a sample, of the ways to the samples across a side and across a corner: the
/// share of a sample's liquid that goes each way down weighs its drop by them (Quinn et al. 1991).
const CONTOUR: [f64; 2] = [0.5, std::f64::consts::SQRT_2 / 4.0];

/// What gathers at each sample of a grid of heights `h` when `gathered` falls on the samples and runs down by
/// multiple flow directions (Quinn et al. 1991): each sample, in `order` (each after every higher one next to it),
/// passes what it gathered to every lower sample next to it (`around`: each with whether it lies across a corner and
/// its distance) by drop over distance times contour length (`CONTOUR`), so it spreads over a convex ridge and
/// gathers in a concave hollow; a sample with no lower one keeps it.
pub(crate) fn spread_down<I: Iterator<Item = (usize, bool, f64)>>(h: &[f64], order: impl Iterator<Item = usize>, around: impl Fn(usize) -> I, mut gathered: Vec<f64>) -> Vec<f64> {
    for p in order {
        let (ways, sum) = ways_down(h, p, &around);
        if sum > 0.0 {
            let a = gathered[p];
            ways.iter().for_each(|&(q, w)| gathered[q] += a * w / sum);
        }
    }
    gathered
}

/// The ways down from sample `p` of a grid of heights `h` to every lower sample next to it (`around`), each with its
/// weight, drop over distance times contour length (`CONTOUR`; Quinn et al. 1991), and the sum of the weights.
pub(crate) fn ways_down<I: Iterator<Item = (usize, bool, f64)>>(h: &[f64], p: usize, around: impl Fn(usize) -> I) -> (Few<(usize, f64), 8>, f64) {
    let mut ways: Few<(usize, f64), 8> = Few::default();
    around(p).filter(|&(q, _, _)| h[q] < h[p]).for_each(|(q, corner, d)| ways.push((q, (h[p] - h[q]) / d * CONTOUR[corner as usize])));
    let sum: f64 = ways.iter().map(|w| w.1).sum();
    (ways, sum)
}

/// A list of at most `N` small things held in place (no heap): the ways down from a point (`ways_down`).
#[derive(Clone, Copy, Debug)]
pub struct Few<T: Copy + Default, const N: usize> {
    v: [T; N],
    n: usize,
}

impl<T: Copy + Default + PartialEq, const N: usize> PartialEq for Few<T, N> {
    fn eq(&self, o: &Self) -> bool {
        **self == **o
    }
}

impl<T: Copy + Default, const N: usize> Default for Few<T, N> {
    fn default() -> Self {
        Few { v: [T::default(); N], n: 0 }
    }
}

impl<T: Copy + Default, const N: usize> Few<T, N> {
    /// `x` goes last. Panics past `N`.
    pub fn push(&mut self, x: T) {
        self.v[self.n] = x;
        self.n += 1;
    }
    pub fn clear(&mut self) {
        self.n = 0;
    }
    /// The things of `x`, in order. Panics past `N`.
    pub fn of(x: &[T]) -> Self {
        let mut f = Few::default();
        f.v[..x.len()].copy_from_slice(x);
        f.n = x.len();
        f
    }
    /// `n` things: those it has, and `x` past them. Panics past `N`.
    pub fn resize(&mut self, n: usize, x: T) {
        if n > self.n {
            self.v[self.n..n].fill(x);
        }
        self.n = n;
    }
    /// `n` things `x`. Panics past `N`.
    pub fn filled(x: T, n: usize) -> Self {
        let mut f = Few { v: [T::default(); N], n };
        f.v[..n].fill(x);
        f
    }
}

impl<'a, T: Copy + Default, const N: usize> IntoIterator for &'a Few<T, N> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T: Copy + Default, const N: usize> std::ops::DerefMut for Few<T, N> {
    fn deref_mut(&mut self) -> &mut [T] {
        &mut self.v[..self.n]
    }
}

impl<T: Copy + Default, const N: usize> std::ops::Deref for Few<T, N> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        &self.v[..self.n]
    }
}

impl<T: Copy + Default, const N: usize> FromIterator<T> for Few<T, N> {
    fn from_iter<I: IntoIterator<Item = T>>(it: I) -> Self {
        let mut f = Few::default();
        it.into_iter().for_each(|x| f.push(x));
        f
    }
}

/// 10 to the power of a gene (a log10 in single precision).
#[inline]
pub fn dec(g: f32) -> f64 {
    (g as f64 * std::f64::consts::LN_10).exp()
}
