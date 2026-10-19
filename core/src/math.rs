//! Small functions of decay the laws share, members passing a level (`Pass`), and an exact sum.

/// 1 - exp(-x), exact for small `x`: the share a first-order decay over `x` time constants takes.
#[inline]
pub(crate) fn one_minus_exp(x: f64) -> f64 {
    -(-x).exp_m1()
}

/// `n` times the surface of a cube of volume `v`: n 6 v^(2/3) (`n` first, for the order of the products).
#[inline]
pub(crate) fn cube_area(n: f64, v: f64) -> f64 {
    n * 6.0 * v.powf(2.0 / 3.0)
}

/// (1 - exp(-x)) / x: the mean over a time of a pulse that decays `x` time constants in it (1 below 1e-9).
#[inline]
pub(crate) fn mean_decay(x: f64) -> f64 {
    if x < 1e-9 { 1.0 } else { one_minus_exp(x) / x }
}

/// Below this steepness times width the shape of `Pass` takes its series (from the Bernoulli numbers), where the
/// closed forms cancel; the next terms are under 1e-16 there.
const SERIES: f64 = 0.1;
/// The steepest shape `Pass` fits, its steepness times its width: past it all but e^-700 of the members lie at one end.
const STEEPEST: f64 = 700.0;
/// Halvings of the steepness in `Pass::new`: to 1e-9 of its range, far finer than its effect on the shares.
const FIT_STEPS: usize = 40;

/// The mean and the variance, in units of the width and of its square, of a density proportional to e^(-u y) on
/// [0, 1]: 1/u - 1/(e^u - 1) and 1/u² - 1/(4 sinh²(u/2)) (1/2 and 1/12 at u = 0, the uniform).
fn shape(u: f64) -> (f64, f64) {
    let u2 = u * u;
    if u.abs() < SERIES {
        return (0.5 - u / 12.0 + u * u2 / 720.0 - u * u2 * u2 / 30240.0, 1.0 / 12.0 - u2 / 240.0 + u2 * u2 / 6048.0 - u2 * u2 * u2 / 172_800.0);
    }
    let h = (0.5 * u).sinh();
    (1.0 / u - 1.0 / u.exp_m1(), 1.0 / u2 - 0.25 / (h * h))
}

/// Members whose values (log10 of their bodies, their wear) lie below a level `top` and all rise alike, by `d`: those who
/// pass the level are the top `d` of them. Below the level their values have a density proportional to e^(-k y), y the
/// distance below it, over a width: the steady state of a stage that members enter at one value and leave at the level,
/// rising at one pace and dying at one rate, is exactly this, k their death rate over their pace (McKendrick 1926; von
/// Foerster 1959), steeper the more die on the way. It is fitted to the members' mean and variance: from rising towards
/// the level (k above 0), through even (k 0), to falling. It is memoryless, so those left below after a rise are again
/// of it, with the same k and the width less the rise. Members so spread that no width holds them (their standard
/// deviation at least their mean distance below the level) are an exponential tail below it, of their mean; members at
/// or above the level all pass.
pub(crate) struct Pass { top: f64, d: f64, k: f64, width: f64 }

impl Pass {
    /// Members of mean `mean` and variance `var` below the level `top` rise by `d`. None if they are of one value or do
    /// not rise: they pass all together or none.
    pub(crate) fn new(mean: f64, var: f64, top: f64, d: f64) -> Option<Pass> {
        let (y, sd) = (top - mean, var.sqrt());
        if sd.is_nan() || sd <= 0.0 || d <= 0.0 {
            return None;
        }
        // Its mean distance below the level in standard deviations falls as the shape steepens towards the level.
        let depth = |u: f64| {
            let (m, v) = shape(u);
            m / v.sqrt()
        };
        let r = y / sd;
        if y <= 0.0 || r <= depth(STEEPEST) {
            return Some(Pass { top, d, k: 1.0 / y.max(f64::MIN_POSITIVE), width: if y > 0.0 { f64::INFINITY } else { 0.0 } });
        }
        let (mut lo, mut hi) = (-STEEPEST, STEEPEST);
        for _ in 0..FIT_STEPS {
            let mid = 0.5 * (lo + hi);
            if depth(mid) > r { lo = mid } else { hi = mid }
        }
        let u = 0.5 * (lo + hi);
        let width = y / shape(u).0;
        Some(Pass { top, d, k: u / width, width })
    }
    /// The share of the members who passed the level and the share left below it; with `extra` ln 10, the shares of
    /// what members hold in proportion to 10^value (their bodies, for log10 bodies).
    pub(crate) fn shares(&self, extra: f64) -> (f64, f64) {
        let (k, d, width) = (self.k + extra, self.d, self.width);
        if d >= width {
            return (1.0, 0.0);
        }
        // (The integral of e^(-k y) over [0, x].)
        let weight = |k: f64, x: f64| if k == 0.0 { x } else { -(-k * x).exp_m1() / k };
        let all = weight(k, width);
        (weight(k, d) / all, (-k * d).exp() * weight(k, width - d) / all)
    }
    /// The mean and the variance of the values of those who passed, and of those left below the level, after the rise.
    pub(crate) fn moments(&self) -> ((f64, f64), (f64, f64)) {
        let k = self.k;
        let of = |w: f64| if w.is_infinite() { (1.0 / k, 1.0 / (k * k)) } else { (w * shape(k * w).0, w * w * shape(k * w).1) };
        let (passed, left) = (of(self.d.min(self.width)), of((self.width - self.d).max(0.0)));
        ((self.top + self.d - passed.0, passed.1), (self.top - left.0, left.1))
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
