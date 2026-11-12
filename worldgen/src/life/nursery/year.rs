//! The year a nursery keeps: a world's year run on a copy, what the cover of each open land cell would read each month
//! (`cover::around_now`), with the light above any cover, the liquid its ground holds and the rain that fell; and the
//! classes of like climate its cells fall in.

use pcore::cover::{self, Around, Parts};
use pcore::World;

/// What one cell's cover reads in one month of the year kept: what it would read (`Around`; its light is that since
/// the month before), the light above any cover since the month before, J/m², the key liquid its ground holds, mol (of
/// which `Around::liquid` is liquid), the rain fallen on it, mol, and the share of what its ground held before the month
/// and that rain that it still holds at the month's end (the rest ran off, sank or rose into the air).
#[derive(Clone, Debug)]
pub(super) struct Month {
    pub around: Around,
    pub sun: f64,
    pub held: f64,
    pub rain: f64,
    pub keep: f64,
}

/// A year of each of `cells` (open land, by index), month by month.
pub(super) struct Year {
    pub cells: Vec<usize>,
    pub months: Vec<Vec<Month>>,
}

impl Year {
    /// The year of a copy of `w` run for `months` months, with the cover it holds (none, or the nursery's).
    pub fn of(w: &World, p: &Parts, cells: &[usize], months: u32) -> Year {
        let mut c = w.clone();
        let mut out: Vec<Vec<Month>> = vec![Vec::with_capacity(months as usize); cells.len()];
        let (mut light, mut rained): (Vec<f64>, Vec<f64>) = (cells.iter().map(|&i| c.light[i]).collect(), cells.iter().map(|&i| c.rained[i]).collect());
        let mut before: Vec<f64> = cells.iter().map(|&i| c.g(p.liquid, i)).collect();
        for _ in 0..months {
            pcore::step(&mut c);
            c.drain_events();
            for (k, &i) in cells.iter().enumerate() {
                let around = cover::around_now(&c, p, i);
                // The light per m² of capture area under the cover the copy holds (`World::canopy`), back to that above it.
                let sun = (around.light - light[k]) / cover::per_capture_area(c.canopy[i] as f64);
                let rain = (c.rained[i] - rained[k]) * c.mesh.cells().area()[i];
                let held = c.g(p.liquid, i);
                let keep = if before[k] + rain > 0.0 { (held / (before[k] + rain)).clamp(0.0, 1.0) } else { 1.0 };
                (light[k], rained[k], before[k]) = (around.light, c.rained[i], held);
                out[k].push(Month { around, sun, held, rain, keep });
            }
        }
        Year { cells: cells.to_vec(), months: out }
    }

    /// The key liquid a ground holds at the start of a cell's year `months` such that, under the year's own keeping
    /// (`Month::keep`) and rain, it holds as much at the year's end, mol: a store whose year closes. Each month it
    /// holds (what it held + the rain) × its keep, so the year's end is A × start + B, and the start is B / (1 − A).
    pub fn closed(months: &[Month]) -> f64 {
        let (a, b) = months.iter().fold((1.0, 0.0), |(a, b), m| (a * m.keep, (b + m.rain) * m.keep));
        if a < 1.0 { b / (1.0 - a) } else { months.first().map_or(0.0, |m| m.held) }
    }

    /// What each cell's year is like, to class it by: its coldest month's ground and its mean ground, K, its mean
    /// wetness (`Physics::wetness`), and the light above it over the year, J/m² (log10).
    pub fn features(&self) -> Vec<[f64; 4]> {
        self.months
            .iter()
            .map(|m| {
                let n = m.len().max(1) as f64;
                let coldest = m.iter().map(|x| x.around.t).fold(f64::MAX, f64::min);
                let mean = m.iter().map(|x| x.around.t).sum::<f64>() / n;
                let wet = m.iter().map(|x| x.around.moist).sum::<f64>() / n;
                let sun = m.iter().map(|x| x.sun).sum::<f64>().max(1.0).log10();
                [coldest, mean, wet, sun]
            })
            .collect()
    }
}

/// Rounds of the k-means that classes the cells (Lloyd 1982): enough for a few dozen classes of a few thousand cells to
/// settle.
const ROUNDS: usize = 16;

/// The cells of `x` (feature vectors) in at most `k` classes of like features, each feature over its spread across the
/// cells (k-means from `k` cells evenly spaced in the order of the first feature). Returns per class its cells (by place
/// in `x`), the first its representative: the cell nearest the class's mean. Classes left empty are dropped.
pub(super) fn classes(x: &[[f64; 4]], k: usize) -> Vec<Vec<usize>> {
    let n = x.len();
    if n == 0 {
        return Vec::new();
    }
    let spread = spread(x);
    let z: Vec<[f64; 4]> = x.iter().map(|v| std::array::from_fn(|f| v[f] / spread[f])).collect();
    let d = |a: &[f64; 4], b: &[f64; 4]| (0..4).map(|f| (a[f] - b[f]).powi(2)).sum::<f64>();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| z[a][0].total_cmp(&z[b][0]).then(a.cmp(&b)));
    let k = k.min(n);
    let mut centre: Vec<[f64; 4]> = (0..k).map(|c| z[order[(2 * c + 1) * n / (2 * k)]]).collect();
    let nearest = |v: &[f64; 4], centre: &[[f64; 4]]| (0..centre.len()).min_by(|&a, &b| d(v, &centre[a]).total_cmp(&d(v, &centre[b]))).unwrap_or(0);
    let mut of = vec![0; n];
    for _ in 0..ROUNDS {
        of.iter_mut().zip(&z).for_each(|(o, v)| *o = nearest(v, &centre));
        for (c, m) in centre.iter_mut().enumerate() {
            let members: Vec<&[f64; 4]> = z.iter().zip(&of).filter(|e| *e.1 == c).map(|e| e.0).collect();
            if !members.is_empty() {
                *m = std::array::from_fn(|f| members.iter().map(|v| v[f]).sum::<f64>() / members.len() as f64);
            }
        }
    }
    let mut out = Vec::new();
    for (c, m) in centre.iter().enumerate() {
        let mut members: Vec<usize> = (0..n).filter(|&i| of[i] == c).collect();
        if let Some(rep) = members.iter().copied().min_by(|&a, &b| d(&z[a], m).total_cmp(&d(&z[b], m)).then(a.cmp(&b))) {
            members.retain(|&i| i != rep);
            members.insert(0, rep);
            out.push(members);
        }
    }
    out
}

/// The spread of each feature over the cells of `x` (its standard deviation).
pub(super) fn spread(x: &[[f64; 4]]) -> [f64; 4] {
    let n = x.len().max(1) as f64;
    std::array::from_fn(|f| {
        let mean = x.iter().map(|v| v[f]).sum::<f64>() / n;
        (x.iter().map(|v| (v[f] - mean).powi(2)).sum::<f64>() / n).sqrt().max(1e-9)
    })
}

/// How far apart two cells' features are, each over its spread: the square of the distance.
pub(super) fn distance(a: &[f64; 4], b: &[f64; 4], spread: &[f64; 4]) -> f64 {
    (0..4).map(|f| ((a[f] - b[f]) / spread[f]).powi(2)).sum()
}

/// The classes of each class's climate nearest to it in `x`, nearest first (`distance` of their representatives).
pub(super) fn neighbours(x: &[[f64; 4]], classes: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let spread = spread(x);
    let d = |a: usize, b: usize| distance(&x[classes[a][0]], &x[classes[b][0]], &spread);
    (0..classes.len())
        .map(|a| {
            let mut o: Vec<usize> = (0..classes.len()).filter(|&b| b != a).collect();
            o.sort_by(|&b, &c| d(a, b).total_cmp(&d(a, c)).then(b.cmp(&c)));
            o
        })
        .collect()
}
