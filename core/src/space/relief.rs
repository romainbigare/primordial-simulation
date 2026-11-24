//! What the running world laid on the terrain's samples (laws.md 0 and 1.3): the samples' ground is the seed's
//! (`Terrain::samples`) plus the relief, the solid volume per area the load laid on each sample, m (as
//! `World::surface_z`), never under none, and what edits raised or lowered there (`shaped`, under none where they
//! lowered it). The load writes what it laid (`flow::runoff`): what settles at a sample lies on it, and what a cell's
//! runoff picks up comes first from its samples' relief; the edits write what they shaped (`World::shape`). The relief
//! says where in a cell its matter lies; the matter is booked in the cell's ground and bed nodes.

use super::samples::Samples;
use super::square::Square;
use super::terrain::SAMPLE_DEPTH;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The relief of the samples (the module's doc): of each sample what the load laid on it, m, now and when the samples
/// were last built from it (`built`; none at all before the first lay or build), and what edits shaped of it, m
/// (`shaped`); of each cell how far its ground stood over the mean of its samples, the seed's with what had been laid
/// and shaped, at the last build, m (`offset`, `heights`); how many builds there were (`builds`, one each
/// `Config::build_years`, which the years of the build are read from); the version of the samples built from it, one
/// more at every build and every reshape after edits (`version`, by which built samples are kept, `Terrain::built`;
/// none: the seed's samples); the key of each pond of the samples last built, in the order of their numbers
/// (`ponds::keys`, by which the next build numbers its ponds, `ponds::renumber`); and whether edits shaped it since the samples were last built (`due`, not saved: a world
/// read from a snapshot builds them from it).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Relief {
    pub laid: Vec<f64>,
    pub built: Vec<f64>,
    pub shaped: Vec<f64>,
    pub offset: Vec<f64>,
    pub builds: u32,
    pub version: u32,
    pub ponds: Vec<u32>,
    pub due: bool,
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
    /// `m` m shaped of sample `k` of `n` samples by an edit (raised; lowered where under none): the samples are due to
    /// be built again.
    pub fn shape(&mut self, k: usize, m: f64, n: usize) {
        if self.shaped.len() != n {
            self.shaped.resize(n, 0.0);
        }
        self.shaped[k] += m;
        self.due = true;
    }
    /// What edits shaped of sample `k`, m.
    pub fn shaped_at(&self, k: usize) -> f64 {
        self.shaped.get(k).copied().unwrap_or(0.0)
    }
    /// What the samples were last built with on sample `k` over the seed's height: what was laid by then and what edits
    /// shaped, m.
    pub fn ground(&self, k: usize) -> f64 {
        self.built.get(k).copied().unwrap_or(0.0) + self.shaped_at(k)
    }
}

/// The relief as saved: of each list its length and the samples that hold some, each with what it holds; the cells'
/// offsets, the builds, the version and the ponds' keys.
#[derive(Serialize, Deserialize)]
struct Sparse {
    laid: (u32, Vec<(u32, f64)>),
    built: (u32, Vec<(u32, f64)>),
    shaped: (u32, Vec<(u32, f64)>),
    offset: Vec<f64>,
    builds: u32,
    version: u32,
    ponds: Vec<u32>,
}

/// The length of `v` and its samples that hold some, in their order.
fn sparse(v: &[f64]) -> (u32, Vec<(u32, f64)>) {
    (v.len() as u32, v.iter().enumerate().filter(|p| *p.1 != 0.0).map(|(k, &m)| (k as u32, m)).collect())
}

/// `n` samples, those of `v` holding what it says and the rest none.
fn dense((n, v): (u32, Vec<(u32, f64)>)) -> Vec<f64> {
    let mut d = vec![0.0; n as usize];
    v.iter().for_each(|&(k, m)| d[k as usize] = m);
    d
}

impl Serialize for Relief {
    fn serialize<S: Serializer>(&self, to: S) -> Result<S::Ok, S::Error> {
        Sparse { laid: sparse(&self.laid), built: sparse(&self.built), shaped: sparse(&self.shaped), offset: self.offset.clone(), builds: self.builds, version: self.version, ponds: self.ponds.clone() }.serialize(to)
    }
}

impl<'de> Deserialize<'de> for Relief {
    fn deserialize<D: Deserializer<'de>>(from: D) -> Result<Relief, D::Error> {
        let s = Sparse::deserialize(from)?;
        Ok(Relief { laid: dense(s.laid), built: dense(s.built), shaped: dense(s.shaped), offset: s.offset, builds: s.builds, version: s.version, ponds: s.ponds, due: false })
    }
}

/// The share of a square's side in samples the offsets of the cells are smoothed over, each way (`heights`).
const SMOOTH_SHARE: f64 = 0.5;
/// The passes of the box filter that smooths them: three give nearly a Gaussian's shape.
const SMOOTH_PASSES: usize = 3;
/// The rounds that smooth what the cells' means lack after the first smoothing: each takes most of the step left at
/// the cells' borders.
const SMOOTH_ROUNDS: usize = 8;

/// How the cells' offsets raise the samples (`lift`, `heights`): the offsets they are of (`Relief::offset`), and of each
/// sample the offsets spread smoothly and what its cell's mean then still lacked, m.
#[derive(Debug)]
pub(super) struct Lift {
    pub(super) offset: Vec<f64>,
    smooth: Vec<f64>,
    last: Vec<f64>,
}

/// The heights of the samples built from the seed's samples `seed` and the relief `r` as last built (`Relief::ground`),
/// raised by the cells' offsets as `l` spreads them (`lift`), m.
pub(super) fn heights(seed: &Samples, r: &Relief, l: &Lift) -> Vec<f64> {
    (0..seed.h.len()).map(|k| seed.h[k] + r.ground(k) + l.smooth[k] + l.last[k]).collect()
}

/// How the cells' offsets of the relief `r` (`Relief::offset`) raise the samples of the seed's samples `seed` of `map`
/// (its rows between latitudes `lat`; `Lift`): each sample by its cell's offset, the offsets spread smoothly between the
/// cells (`SMOOTH_SHARE` of a square each way, `SMOOTH_PASSES` passes of a box filter, round a globe's rows), what the
/// cells' means then lack spread the same way `SMOOTH_ROUNDS` times, and each cell's raised by the constant it still
/// lacks, so that its samples' mean by area is its ground exactly. A cell finer than a sample shares it: the sample
/// takes their offsets' mean by area.
pub(super) fn lift(seed: &Samples, map: &Square, (south, north): (f64, f64), r: &Relief) -> Lift {
    let n = seed.h.len();
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
    // What each group's mean by area of `f` lacks of its own offset, at each of its samples.
    let lacks = |f: &[f64]| {
        let mut sums = vec![[0.0; 2]; (cells as usize) + n];
        (0..n).for_each(|k| {
            let g = group[k] as usize;
            sums[g] = [sums[g][0] + seed.area(k), sums[g][1] + seed.area(k) * (target[k] - f[k])];
        });
        (0..n).map(|k| sums[group[k] as usize][1] / sums[group[k] as usize][0]).collect::<Vec<f64>>()
    };
    // The offsets smoothed, what the groups' means then lack smoothed and added, `SMOOTH_ROUNDS` times; then each group
    // raised by what it still lacks, so its mean is exact.
    let grid = (seed.across, seed.down, map.wrap);
    let mut smooth = blur(&target, grid, radius);
    for _ in 0..SMOOTH_ROUNDS {
        let more = blur(&lacks(&smooth), grid, radius);
        smooth.iter_mut().zip(more).for_each(|(f, m)| *f += m);
    }
    let last = lacks(&smooth);
    Lift { offset: r.offset.clone(), smooth, last }
}

/// Values `v` on a grid `across` x `down` smoothed by `SMOOTH_PASSES` passes of a box filter `radius` samples each way
/// along the rows and then the columns: shorter at the grid's edges, but round the rows where they `wrap` (a globe's).
/// Each line's running sums are found first, then its values from them; the columns all at once, row by row, so the
/// grid is read in its order.
fn blur(v: &[f64], (across, down, wrap): (usize, usize, bool), radius: usize) -> Vec<f64> {
    let mut x = v.to_vec();
    // The running sums of a row; of the columns, `[q * across + c]` the sum of column c over its first q samples.
    let (mut row, mut col) = (vec![0.0; across + 1], vec![0.0; (down + 1) * across]);
    for _ in 0..SMOOTH_PASSES {
        for line in x.chunks_exact_mut(across) {
            (0..across).for_each(|q| row[q + 1] = row[q] + line[q]);
            for (q, out) in line.iter_mut().enumerate() {
                let (lo, hi) = span(q, (across, wrap), radius);
                let upto = |q: i64| upto(q, across, row[across], |m| row[m]);
                *out = (upto(hi) - upto(lo)) / (hi - lo) as f64;
            }
        }
        for q in 0..down {
            for c in 0..across {
                col[(q + 1) * across + c] = col[q * across + c] + x[q * across + c];
            }
        }
        for q in 0..down {
            let (lo, hi) = span(q, (down, false), radius);
            for c in 0..across {
                let upto = |q: i64| upto(q, down, col[down * across + c], |m| col[m * across + c]);
                x[q * across + c] = (upto(hi) - upto(lo)) / (hi - lo) as f64;
            }
        }
    }
    x
}

/// The samples a box filter of `radius` sums for sample `q` of a line of `len` (round it where `round`): from `lo` up
/// to `hi` (not `hi`), shorter at the line's ends, past them round it.
fn span(q: usize, (len, round): (usize, bool), radius: usize) -> (i64, i64) {
    let (q, r, len) = (q as i64, radius as i64, len as i64);
    if round && 2 * r < len { (q - r, q + r + 1) } else if round { (0, len) } else { ((q - r).max(0), (q + r + 1).min(len)) }
}

/// The sum of a line of `len` samples over its first `q` (past its ends, round it: `whole` for each time round), its
/// running sums `prefix`.
#[inline]
fn upto(q: i64, len: usize, whole: f64, prefix: impl Fn(usize) -> f64) -> f64 {
    let n = len as i64;
    let (turns, at) = if (0..n).contains(&q) { (0, q) } else { (q.div_euclid(n), q.rem_euclid(n)) };
    turns as f64 * whole + prefix(at as usize)
}

#[cfg(test)]
mod tests {
    use super::{blur, Relief};

    /// Round a wrapped grid's rows the box filter has no seam: a step at the seam is smoothed as one inside the rows.
    #[test]
    fn the_blur_has_no_seam_round_a_globe() {
        let (across, down) = (16, 2);
        let v: Vec<f64> = (0..across * down).map(|k| if (k % across + 4) % across < 8 { 1.0 } else { 0.0 }).collect();
        let shifted: Vec<f64> = (0..across * down).map(|k| v[(k / across) * across + (k % across + 4) % across]).collect();
        let (a, b) = (blur(&v, (across, down, true), 2), blur(&shifted, (across, down, true), 2));
        let back: Vec<f64> = (0..across * down).map(|k| b[(k / across) * across + (k % across + across - 4) % across]).collect();
        assert!(a.iter().zip(&back).all(|(x, y)| (x - y).abs() < 1e-12), "the filter moves with the rows");
    }

    /// A relief saved and read again is the same, also where a list holds only none.
    #[test]
    fn a_relief_reads_back_the_same() {
        let r = Relief { laid: vec![0.0; 4], built: vec![0.0, 1.5, 0.0], shaped: vec![-2.0, 0.0], offset: vec![0.25], builds: 2, version: 3, ponds: vec![7, 2], due: false };
        let back: Relief = serde_json::from_str(&serde_json::to_string(&r).expect("saved")).expect("read");
        assert_eq!(back, r);
    }
}
