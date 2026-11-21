//! The planet's air beyond a map's edges (laws.md 1.6): per row of latitude and month of a strip of the planet run by
//! the laws (`worldgen::planet`), what its air holds (`Planet`); and what an air node of the map reads of it beyond
//! its edge, at a latitude and a time of the year (`Planet::at`).

use super::super::Scratch;
use crate::{World, AIR};
use serde::{Deserialize, Serialize};

/// The air of a strip of the planet from pole to pole, by row and month: saved with the world, never found again during
/// a run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Planet {
    /// The latitude of each row's middle, degrees, from north to south
    pub lat: Vec<f64>,
    /// The distance between the middles of two rows on the planet, m
    pub row_m: f64,
    /// Months of the year the table holds, each at its end
    pub months: usize,
    /// Per row and month (`[r * months + m]`): the air's temperature at its column's mean height, K; its warmth (that
    /// raised to a common height by its drop with height, as the wind reads it), K; and its mol per m²
    pub t: Vec<f64>,
    pub warmth: Vec<f64>,
    pub mol: Vec<f64>,
    /// Per row, month and substance (`[(r * months + m) * ns + k]`): the share of the column's mol
    pub share: Vec<f64>,
}

/// The planet's air at a latitude and a time (`Planet::at`): its temperature, K, warmth, K, and its warmth's gradient
/// toward the south, K per m of the planet; its mol per m², and the share of each substance.
pub(super) struct Beyond {
    pub(super) t: f64,
    pub(super) warmth: f64,
    pub(super) south: f64,
    pub(super) mol: f64,
    pub(super) share: Vec<f64>,
}

impl Planet {
    /// The air of the world `w` that ran, per row of its air level and as its last air step found it (`Scratch`):
    /// one month of the table, the row's nodes' mean by area. Rows by `World::mesh`'s tree, north to south.
    pub fn month_of(w: &World) -> (Vec<f64>, Vec<[f64; 3]>, Vec<Vec<f64>>) {
        let (s, air, ns) = (&w.scratch, w.mesh.level(AIR), w.chem.ns());
        let rows = (0..air.n()).map(|j| w.mesh.tree().block_xy(j).1).max().map_or(0, |r| r + 1);
        let (mut lat, mut sum, mut share, mut area) = (vec![0.0; rows], vec![[0.0; 3]; rows], vec![vec![0.0; ns]; rows], vec![0.0; rows]);
        for j in 0..air.n() {
            let (r, a) = (w.mesh.tree().block_xy(j).1, air.area()[j]);
            let one = column(s, j, a);
            area[r] += a;
            lat[r] += a * air.lat()[j];
            sum[r].iter_mut().zip(one).for_each(|(x, v)| *x += a * v);
            (0..ns).for_each(|k| share[r][k] += a * w.store.amounts(crate::store::Class::Air)[k * air.n() + j] / s.nt[j].max(f64::MIN_POSITIVE));
        }
        for r in 0..rows {
            lat[r] /= area[r];
            sum[r].iter_mut().for_each(|x| *x /= area[r]);
            share[r].iter_mut().for_each(|x| *x /= area[r]);
        }
        (lat, sum, share)
    }
    /// The air at latitude `lat`, degrees, at the share `season` of the year: between the two rows about it and the two
    /// months about it, linearly (the months round the year). Past a pole lies the other meridian, the same strip's air
    /// (it is zonal) at the latitude folded back, its south this side's north. Between a pole and its row's middle that
    /// row holds, as the air across the pole is the same row's. Across the equator it is the other half's air in its own
    /// season.
    pub(super) fn at(&self, lat: f64, season: f64) -> Beyond {
        let (lat, turn) = if lat > 90.0 { (180.0 - lat, -1.0) } else if lat < -90.0 { (-180.0 - lat, -1.0) } else { (lat, 1.0) };
        let (rows, ns) = (self.lat.len(), self.share.len() / (self.lat.len() * self.months).max(1));
        // The rows run north to south: the row at or north of `lat`, and how far toward the next.
        let r = self.lat.iter().rposition(|&l| l >= lat).unwrap_or(0).min(rows.saturating_sub(2));
        let f = ((self.lat[r] - lat) / (self.lat[r] - self.lat[r + 1])).clamp(0.0, 1.0);
        // Month m is the table's at the end of month m: the one at or before `season`, and how far toward the next.
        let at = (season * self.months as f64 - 1.0).rem_euclid(self.months as f64);
        let (m, g) = (at.floor() as usize % self.months, at.fract());
        let weights = [(r, m, (1.0 - f) * (1.0 - g)), (r, (m + 1) % self.months, (1.0 - f) * g), (r + 1, m, f * (1.0 - g)), (r + 1, (m + 1) % self.months, f * g)];
        let of = |v: &[f64]| weights.iter().map(|&(r, m, x)| x * v[r * self.months + m]).sum::<f64>();
        // The warmth's gradient toward the south at those months, between the two rows.
        let south = turn * weights.iter().map(|&(r0, m, x)| x * (self.warmth[(r0 + 1).min(rows - 1) * self.months + m] - self.warmth[r0.min(rows - 2) * self.months + m])).sum::<f64>()
            / self.row_m;
        let share = (0..ns).map(|k| weights.iter().map(|&(r, m, x)| x * self.share[(r * self.months + m) * ns + k]).sum()).collect();
        Beyond { t: of(&self.t), warmth: of(&self.warmth), south, mol: of(&self.mol), share }
    }
}

/// Node `j`'s temperature, K, warmth, K, and mol per m² (its area `area`), as its last air step found them.
fn column(s: &Scratch, j: usize, area: f64) -> [f64; 3] {
    [s.ta[j], s.ta[j] + s.gam[j] * s.zair[j], s.nt[j] / area]
}
