//! Where a cell's land is saturated (TOPMODEL, Beven and Kirkby 1979; Beven 1997): its liquid table, over the bed's
//! store or in its loose ground (`Means::deficit`), departs over its land's samples by the wetness index (`soil_level`,
//! `deficit_at`) and reaches the surface of those where the most gathers. It moves no liquid. A law reads the share of its land saturated and the liquid of the drier rest (`wet_ground`).

use super::{deficit_at, soil_level, Means};
use crate::space::terrain::SAMPLE_DEPTH;
use crate::World;

/// The land of a cell as its soil's liquid lies over it (`wet_ground`): the share of its land saturated; the wetness
/// index at and over which a sample is saturated (infinite where none is, minus infinite where all are); and the share
/// of the pores the liquid fills on the mean over the rest of its land (the drier ground).
#[derive(Clone, Copy, Debug)]
pub struct WetGround {
    pub share: f64,
    pub level: f64,
    pub filled: f64,
}

/// The land of cell `i` of means `m` (`Means`, over its land): its samples (`World::terrain_samples`) but the lowest,
/// which the liquid that stands over the cell covers, by its land's share of its area (`contract::land`; the liquid
/// stands over the lowest first, `flow::curve`), where its liquid table lies (`saturated`).
pub fn wet_ground(w: &World, i: usize, m: Means) -> WetGround {
    let s = w.terrain_samples();
    let mut at: Vec<(f64, f64)> = w.map.leaves()[i].split(SAMPLE_DEPTH).map(|f| s.index(&f)).map(|k| (s.h[k], s.twi[k])).collect();
    at.sort_by(|a, b| b.0.total_cmp(&a.0));
    let land = crate::contract::land(w, i) / w.mesh.cells().area()[i];
    at.truncate((land * at.len() as f64).round() as usize);
    let twi: Vec<f64> = at.iter().map(|p| p.1).collect();
    saturated(&w.cfg.phys, &twi, m)
}

/// Ground of equal parts of wetness index `twi` and means `m`: its liquid table departs over them by the index
/// (`soil_level`, `deficit_at`), and a part is saturated where it reaches the surface, at an index of at least the level
/// plus the mean room over `soil_m`. Its saturated parts' pores are full, so the rest hold what they do not.
pub fn saturated(ph: &crate::Physics, twi: &[f64], m: Means) -> WetGround {
    let filled = if m.pores > 0.0 { m.pore / m.pores } else { 1.0 };
    let [d, most] = m.deficit(ph);
    let (share, level) = match soil_level(ph, twi, &vec![1.0 / twi.len().max(1) as f64; twi.len()], m) {
        _ if d <= 0.0 || filled >= 1.0 => (1.0, f64::NEG_INFINITY),
        None => (if d >= most { 0.0 } else { 1.0 }, if d >= most { f64::INFINITY } else { f64::NEG_INFINITY }),
        Some(c) => (twi.iter().filter(|&&t| deficit_at(ph, m, t, c) <= 0.0).count() as f64 / twi.len() as f64, c + d / ph.soil_m),
    };
    let rest = if share < 1.0 { ((filled - share) / (1.0 - share)).clamp(0.0, 1.0) } else { 1.0 };
    WetGround { share, level, filled: rest }
}

/// The land of cell `i` as its key liquid (that of `p`) lies over it now: how the cover's roots meet it (`cover::soak`),
/// and where its liquid table reaches its surface (`wet_ground`): its pores as Flow has them (`Scratch::land_pores`)
/// over the bed's store under the cell (`Class::Deep`).
pub fn wet_land(w: &World, p: &crate::cover::Parts, i: usize) -> (crate::cover::Soak, WetGround) {
    let soak = crate::cover::soak(w, p, i, w.g(p.liquid, i).max(0.0)).1;
    let pores = w.scratch.land_pores(i).max(1e-9);
    let m = Means { pore: soak.filled.min(1.0) * pores, pores, stands: 0.0, store: crate::flow::store_depth(w, i), field: soak.pores.field };
    (soak, wet_ground(w, i, m))
}
