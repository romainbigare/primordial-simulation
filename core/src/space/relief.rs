//! What the running world laid on the terrain's samples (laws.md 0 and 1.3): the samples' ground is the seed's
//! (`Terrain::samples`) plus the relief, the solid volume per area the load laid on each sample, m (as
//! `World::surface_z`), never under none. Only the load writes it (`flow::runoff`): what settles at a sample lies on
//! it, and what a cell's runoff picks up comes first from its samples' relief. The relief says where in a cell its
//! matter lies; the matter is booked in the cell's ground node.

use super::samples::Samples;
use super::square::Square;
use super::terrain::SAMPLE_DEPTH;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The relief of the samples (the module's doc): of each sample what the load laid on it, m, now and when the samples
/// were last built from it (`built`; none at all before the first lay or build); of each cell how far its ground stood
/// over the mean of its samples, the seed's with what had been laid, at the last build, m (`offset`, `heights`); and how
/// many builds there were.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Relief {
    pub laid: Vec<f64>,
    pub built: Vec<f64>,
    pub offset: Vec<f64>,
    pub builds: u32,
}

impl Relief {
    /// `m` m laid on sample `k` of `n` samples.
    pub fn lay(&mut self, k: usize, m: f64, n: usize) {
        if self.laid.len() != n {
            self.laid.resize(n, 0.0);
        }
        self.laid[k] += m;
    }
    /// At most `m` m taken from sample `k`'s relief, never under none: what was taken, m.
    pub fn take(&mut self, k: usize, m: f64) -> f64 {
        let Some(at) = self.laid.get_mut(k) else { return 0.0 };
        let taken = m.min(*at).max(0.0);
        *at -= taken;
        taken
    }
    /// What lies on sample `k`, m.
    pub fn at(&self, k: usize) -> f64 {
        self.laid.get(k).copied().unwrap_or(0.0)
    }
    /// What was laid on sample `k` since the samples were last built, m (less what was taken).
    pub fn since(&self, k: usize) -> f64 {
        self.at(k) - self.built.get(k).copied().unwrap_or(0.0)
    }
}

/// The relief as saved: of each list the samples that hold some, each with what it holds, and the number of samples;
/// the cells' offsets and the builds.
#[derive(Serialize, Deserialize)]
struct Sparse {
    n: u32,
    laid: Vec<(u32, f64)>,
    built: Vec<(u32, f64)>,
    offset: Vec<f64>,
    builds: u32,
}

/// The samples of `v` that hold some, in their order.
fn sparse(v: &[f64]) -> Vec<(u32, f64)> {
    v.iter().enumerate().filter(|p| *p.1 != 0.0).map(|(k, &m)| (k as u32, m)).collect()
}

/// `n` samples, those of `v` holding what it says and the rest none (none at all where `v` is empty).
fn dense(n: usize, v: &[(u32, f64)]) -> Vec<f64> {
    if v.is_empty() {
        return Vec::new();
    }
    let mut d = vec![0.0; n];
    v.iter().for_each(|&(k, m)| d[k as usize] = m);
    d
}

impl Serialize for Relief {
    fn serialize<S: Serializer>(&self, to: S) -> Result<S::Ok, S::Error> {
        let n = self.laid.len().max(self.built.len()) as u32;
        Sparse { n, laid: sparse(&self.laid), built: sparse(&self.built), offset: self.offset.clone(), builds: self.builds }.serialize(to)
    }
}

impl<'de> Deserialize<'de> for Relief {
    fn deserialize<D: Deserializer<'de>>(from: D) -> Result<Relief, D::Error> {
        let s = Sparse::deserialize(from)?;
        Ok(Relief { laid: dense(s.n as usize, &s.laid), built: dense(s.n as usize, &s.built), offset: s.offset, builds: s.builds })
    }
}

/// The share of a square's side in samples the offsets of the cells are smoothed over, each way (`heights`).
const SMOOTH_SHARE: f64 = 0.5;
/// The passes of the box filter that smooths them: three give nearly a Gaussian's shape.
const SMOOTH_PASSES: usize = 3;
/// The rounds that smooth what the cells' means lack after the first smoothing: each takes most of the step left at
/// the cells' borders.
const SMOOTH_ROUNDS: usize = 8;

/// The heights of the samples built from the seed's samples `seed` of `map` (its rows between latitudes `lat`) and the
/// relief `r` as last built (`Relief::built`, `Relief::offset`), m: each the seed's height with what was laid on it,
/// raised by its cell's offset; the offsets spread smoothly between the cells (`SMOOTH_SHARE` of a square each way,
/// `SMOOTH_PASSES` passes of a box filter), what the cells' means then lack spread the same way `SMOOTH_ROUNDS` times,
/// and each cell's raised by the constant it still lacks, so that its samples' mean by area is its ground exactly. A cell finer than a sample shares it: the sample takes their offsets' mean by area.
pub(super) fn heights(seed: &Samples, map: &Square, (south, north): (f64, f64), r: &Relief) -> Vec<f64> {
    let n = seed.h.len();
    let base: Vec<f64> = (0..n).map(|k| seed.h[k] + r.built.get(k).copied().unwrap_or(0.0)).collect();
    // Of each sample, the offset it takes and the group it is held to the mean of (its cell; or itself, under cells
    // finer than it).
    let (mut target, mut weight, mut group) = (vec![0.0; n], vec![0.0; n], vec![u32::MAX; n]);
    for (i, l) in map.leaves().iter().enumerate() {
        let area = map.place_of(l, south, north).0;
        if l.depth <= SAMPLE_DEPTH {
            for f in l.split(SAMPLE_DEPTH) {
                let k = seed.index(&f);
                (target[k], weight[k], group[k]) = (r.offset[i], 1.0, i as u32);
            }
        } else {
            let k = seed.index(l);
            (target[k], weight[k]) = (target[k] + r.offset[i] * area, weight[k] + area);
        }
    }
    let (cells, own) = (map.leaves().len() as u32, |k: usize| map.leaves().len() as u32 + k as u32);
    for k in 0..n {
        if group[k] == u32::MAX {
            (target[k], group[k]) = (target[k] / weight[k].max(f64::MIN_POSITIVE), own(k));
        }
    }
    let side = (1usize << (SAMPLE_DEPTH - map.square_depth())) as f64;
    let radius = (SMOOTH_SHARE * side).round() as usize;
    let area: Vec<f64> = (0..n).map(|k| map.place_of(&leaf_of(seed, k), south, north).0).collect();
    // What each group's mean by area of `f` lacks of its own offset, at each of its samples.
    let lacks = |f: &[f64]| {
        let mut sums = vec![[0.0; 2]; (cells as usize) + n];
        (0..n).for_each(|k| {
            let g = group[k] as usize;
            sums[g] = [sums[g][0] + area[k], sums[g][1] + area[k] * (target[k] - f[k])];
        });
        (0..n).map(|k| sums[group[k] as usize][1] / sums[group[k] as usize][0]).collect::<Vec<f64>>()
    };
    // The offsets smoothed, what the groups' means then lack smoothed and added, `SMOOTH_ROUNDS` times; then each group
    // raised by what it still lacks, so its mean is exact.
    let mut smooth = blur(&target, (seed.across, seed.down), radius);
    for _ in 0..SMOOTH_ROUNDS {
        let more = blur(&lacks(&smooth), (seed.across, seed.down), radius);
        smooth.iter_mut().zip(more).for_each(|(f, m)| *f += m);
    }
    let last = lacks(&smooth);
    (0..n).map(|k| base[k] + smooth[k] + last[k]).collect()
}

/// The leaf of sample `k` of samples `s`.
fn leaf_of(s: &Samples, k: usize) -> super::Leaf {
    let side = super::tree::BLOCK >> SAMPLE_DEPTH;
    super::Leaf { x: (k % s.across) as u64 * side, y: (k / s.across) as u64 * side, depth: SAMPLE_DEPTH }
}

/// Values `v` on a grid `across` x `down` smoothed by `SMOOTH_PASSES` passes of a box filter `radius` samples each way
/// along the rows and then the columns (shorter at the grid's edges).
fn blur(v: &[f64], (across, down): (usize, usize), radius: usize) -> Vec<f64> {
    let mut x = v.to_vec();
    let line = |x: &mut [f64], at: &dyn Fn(usize) -> usize, len: usize| {
        let mut prefix = vec![0.0; len + 1];
        (0..len).for_each(|q| prefix[q + 1] = prefix[q] + x[at(q)]);
        (0..len).for_each(|q| {
            let (lo, hi) = (q.saturating_sub(radius), (q + radius + 1).min(len));
            x[at(q)] = (prefix[hi] - prefix[lo]) / (hi - lo) as f64;
        });
    };
    for _ in 0..SMOOTH_PASSES {
        (0..down).for_each(|r| line(&mut x, &|q| r * across + q, across));
        (0..across).for_each(|c| line(&mut x, &|q| q * across + c, down));
    }
    x
}
