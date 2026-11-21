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

/// The relief as saved: of each list its length and the samples that hold some, each with what it holds; the cells'
/// offsets and the builds.
#[derive(Serialize, Deserialize)]
struct Sparse {
    laid: (u32, Vec<(u32, f64)>),
    built: (u32, Vec<(u32, f64)>),
    offset: Vec<f64>,
    builds: u32,
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
        Sparse { laid: sparse(&self.laid), built: sparse(&self.built), offset: self.offset.clone(), builds: self.builds }.serialize(to)
    }
}

impl<'de> Deserialize<'de> for Relief {
    fn deserialize<D: Deserializer<'de>>(from: D) -> Result<Relief, D::Error> {
        let s = Sparse::deserialize(from)?;
        Ok(Relief { laid: dense(s.laid), built: dense(s.built), offset: s.offset, builds: s.builds })
    }
}

/// The heights of the samples built from the seed's samples `seed` of `map` (its rows between latitudes `lat`) and the
/// relief `r` as last built (`Relief::built`, `Relief::offset`), m: each the seed's height with what was laid on it,
/// raised by its cell's offset, which lies on one plane through the cell's centre whose slope each way is the smaller
/// of the slopes to the neighbours on its two sides, none where they differ in sign (minmod: the reconstruction of the
/// finite volumes, van Leer 1979), round a globe's rows where `map` wraps; then each cell raised by the constant it
/// lacks, so that its samples' mean by area is its ground exactly. A cell finer than a sample shares it: the sample
/// takes their offsets' mean by area.
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
    let cells = map.leaves().len();
    for k in 0..n {
        if group[k] == u32::MAX {
            (target[k], group[k]) = (target[k] / weight[k].max(f64::MIN_POSITIVE), (cells + k) as u32);
        }
    }
    let planes = planes(seed, map, (&target, &group));
    let at = |k: usize| {
        let g = group[k] as usize;
        let (x, y) = ((k % seed.across) as f64, (k / seed.across) as f64);
        planes.get(g).copied().flatten().map_or(target[k], |p| p[0] + p[1] * (x - p[3]) + p[2] * (y - p[4]))
    };
    let f: Vec<f64> = (0..n).map(at).collect();
    // What each group's mean by area of the planes lacks of its own offset.
    let mut sums = vec![[0.0; 2]; cells + n];
    for k in 0..n {
        let (g, a) = (group[k] as usize, map.place_of(&seed.leaf(k), south, north).0);
        sums[g] = [sums[g][0] + a, sums[g][1] + a * (target[k] - f[k])];
    }
    (0..n).map(|k| base[k] + f[k] + sums[group[k] as usize][1] / sums[group[k] as usize][0]).collect()
}

/// Of each cell of `map` of a sample or coarser, its plane over the samples `seed` (`heights`): its offset, its slopes
/// along the rows and the columns, m a sample, and its centre (column, row); of the samples' offsets `target` and
/// groups `group`. A cell finer than a sample has none (its sample keeps its offset).
fn planes(seed: &Samples, map: &Square, (target, group): (&[f64], &[u32])) -> Vec<Option<[f64; 5]>> {
    let (across, down) = (seed.across as i64, seed.down as i64);
    let side = super::tree::BLOCK >> SAMPLE_DEPTH;
    let box_of = |l: &super::Leaf| ((l.x / side) as i64, (l.y / side) as i64, 1i64 << (SAMPLE_DEPTH - l.depth));
    // The centre of each group, (column, row).
    let mut centre = vec![[0.0f64; 3]; group.iter().map(|&g| g as usize + 1).max().unwrap_or(0)];
    for (k, &g) in group.iter().enumerate() {
        let c = &mut centre[g as usize];
        *c = [c[0] + (k as i64 % across) as f64, c[1] + (k as i64 / across) as f64, c[2] + 1.0];
    }
    let mid = |g: u32| [centre[g as usize][0] / centre[g as usize][2], centre[g as usize][1] / centre[g as usize][2]];
    // The mean offset and position of the samples just across a side of a cell, each at its group's centre, along the
    // axis `axis` (0 the rows, 1 the columns): the samples `edge` yields, each with how far round the rows it wrapped.
    let across_side = |edge: &mut dyn Iterator<Item = (i64, i64)>, axis: usize| {
        let (mut v, mut x, mut m) = (0.0, 0.0, 0.0);
        for (c, row) in edge {
            let (c0, wrapped) = if map.wrap { (c.rem_euclid(across), (c - c.rem_euclid(across)) as f64) } else { (c, 0.0) };
            if c0 < 0 || c0 >= across || row < 0 || row >= down {
                continue;
            }
            let k = (row * across + c0) as usize;
            (v, x, m) = (v + target[k], x + mid(group[k])[axis] + if axis == 0 { wrapped } else { 0.0 }, m + 1.0);
        }
        (m > 0.0).then(|| (v / m, x / m))
    };
    let minmod = |a: f64, b: f64| if a * b <= 0.0 { 0.0 } else { a.signum() * a.abs().min(b.abs()) };
    map.leaves().iter().enumerate().map(|(i, l)| {
        if l.depth > SAMPLE_DEPTH {
            return None;
        }
        let (x0, y0, m) = box_of(l);
        let (d, [cx, cy]) = (target[(y0 * across + x0) as usize], mid(i as u32));
        let slope = |side: Option<(f64, f64)>, at: f64| side.map(|(v, x)| (v - d) / (x - at));
        let east = slope(across_side(&mut (y0..y0 + m).map(|y| (x0 + m, y)), 0), cx);
        let west = slope(across_side(&mut (y0..y0 + m).map(|y| (x0 - 1, y)), 0), cx);
        let south_side = slope(across_side(&mut (x0..x0 + m).map(|x| (x, y0 + m)), 1), cy);
        let north_side = slope(across_side(&mut (x0..x0 + m).map(|x| (x, y0 - 1)), 1), cy);
        let limited = |a: Option<f64>, b: Option<f64>| a.zip(b).map_or(0.0, |(a, b)| minmod(a, b));
        Some([d, limited(east, west), limited(south_side, north_side), cx, cy])
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::Relief;

    /// A relief saved and read again is the same, also where a list holds only none.
    #[test]
    fn a_relief_reads_back_the_same() {
        let r = Relief { laid: vec![0.0; 4], built: vec![0.0, 1.5, 0.0], offset: vec![0.25], builds: 2 };
        let back: Relief = serde_json::from_str(&serde_json::to_string(&r).expect("saved")).expect("read");
        assert_eq!(back, r);
    }
}
