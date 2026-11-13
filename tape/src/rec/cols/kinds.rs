//! Each starting kind of cover (`root_<k>_*`, by `cover::Cover::root`): its slots, mass and capture area, and the means
//! of its genes by built cover. A kind that died out reads 0 (its means NaN); the columns are there for every kind up
//! to the largest seen so far.

use super::{ratio, Row};
use crate::layers::Ctx;
use crate::rec::Recorder;
use pcore::cover;

/// The genes of the cover averaged per kind: gene, column name and unit.
const COVER_GENES: [(usize, &str, &str); 9] = [
    (cover::LIGHT, "light", "log10"),
    (cover::LEAF, "leaf", "log10"),
    (cover::WALLS, "walls", "log10"),
    (cover::STORE, "store", "log10"),
    (cover::FROST, "frost", "K"),
    (cover::MOIST, "moist", "log10"),
    (cover::SEED, "seed", "1"),
    (cover::REPAIR, "repair", "log10"),
    (cover::FIX, "fix", "log10"),
];

impl Recorder {
    pub(super) fn kind_cols(&self, c: &Ctx, row: &mut Row) {
        let w = c.w;
        // The cover of each kind of cover seen so far: its slots, mass, capture area per m² of the world's land, and its
        // genes' means by built cover.
        let sums = cover::sums(w);
        let seen = self.files.series.iter().filter(|c| c.name.starts_with("root_") && c.name.ends_with("_slots")).count();
        let land_area: f64 = (0..w.mesh.cells().n()).filter(|&i| w.is_land(i)).map(|i| w.mesh.cells().area()[i]).sum::<f64>().max(f64::MIN_POSITIVE);
        for k in 0..sums.len().max(seen) {
            let s = sums.get(k).cloned().unwrap_or_default();
            let n = |x: &str| format!("root_{k}_{x}");
            row.push(n("slots"), "1", s.slots);
            row.push(n("cover_kg"), "kg", s.kg);
            row.push(n("cover_area"), "m2/m2", s.leaf / land_area);
            for (&(g, name, unit), &x) in COVER_GENES.iter().zip(&s.genes) {
                debug_assert!(g < cover::NG);
                row.push(n(&format!("cover_{name}")), unit, ratio(x, s.built));
            }
        }
    }
}
