//! Small functions of decay the laws share, members passing a level (`Pass`), an exact sum, the bits of a set, and a
//! short list held in place (`Few`).

/// 1 - exp(-x), exact for small `x`: the share a first-order decay over `x` time constants takes.
#[inline]
pub(crate) fn one_minus_exp(x: f64) -> f64 {
    -(-x).exp_m1()
}

/// The bits set in `bits`, lowest first: a set of a few of 64 things (substances, key liquids) visited without the rest.
#[inline]
pub(crate) fn ones(mut bits: u64) -> impl Iterator<Item = usize> {
    std::iter::from_fn(move || {
        (bits != 0).then(|| {
            let k = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            k
        })
    })
}

/// What rounding took from `a + b`, given their computed sum `s`, exactly (Neumaier 1974): kept apart, it makes a sum
/// exact.
#[inline]
pub(crate) fn rounding_lost(a: f64, b: f64, s: f64) -> f64 {
    if a.abs() >= b.abs() { (a - s) + b } else { (b - s) + a }
}

/// `n` times the surface of a cube of volume `v`: n 6 v^(2/3), as the square of the edge (`n` first, for the order of
/// the products).
#[inline]
pub(crate) fn cube_area(n: f64, v: f64) -> f64 {
    n * 6.0 * v.cbrt().powi(2)
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

/// Light passing layers from the tallest down (Beer's law): `l` holds per layer member its height, its capture area per
/// ground area `x` and a tag, and is sorted here, tallest first; members of one height share a layer, and a layer of
/// capture area x absorbs 1 - exp(-x) of what reaches it. `set(tag, f, bottom)` gets each member's light per unit of capture
/// area over the mean of all (weighted by capture area, the factors have mean 1), and the light at the bottom of its
/// layer per unit of capture area over the same mean.
pub(crate) fn shade<T: Copy>(l: &mut [(f32, f64, T)], mut set: impl FnMut(T, f64, f64)) {
    l.sort_by(|a, b| b.0.total_cmp(&a.0));
    let (all, mut above, mut k) = (mean_decay(l.iter().map(|x| x.1).sum()), 0f64, 0);
    while k < l.len() {
        let e = k + l[k..].iter().take_while(|x| x.0 == l[k].0).count();
        let x: f64 = l[k..e].iter().map(|x| x.1).sum();
        let (f, bottom) = ((-above).exp() * mean_decay(x) / all, (-above - x).exp() / all);
        l[k..e].iter().for_each(|y| set(y.2, f, bottom));
        (above, k) = (above + x, e);
    }
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

/// A list of at most `N` small things held in place (no heap): a body's genes (`body::Genes`) and what it works out
/// from them (`body::Consts`). Saved as the list it holds.
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

impl<T: Copy + Default + serde::Serialize, const N: usize> serde::Serialize for Few<T, N> {
    fn serialize<S: serde::Serializer>(&self, to: S) -> Result<S::Ok, S::Error> {
        (**self).serialize(to)
    }
}

impl<'de, T: Copy + Default + serde::Deserialize<'de>, const N: usize> serde::Deserialize<'de> for Few<T, N> {
    fn deserialize<D: serde::Deserializer<'de>>(from: D) -> Result<Self, D::Error> {
        let v = Vec::<T>::deserialize(from)?;
        if v.len() > N {
            return Err(serde::de::Error::invalid_length(v.len(), &"at most the room held"));
        }
        Ok(Few::of(&v))
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
