//! Where a cell's land is saturated (TOPMODEL, Beven and Kirkby 1979; Beven 1997): the liquid of its soil shared over
//! its land's samples by the wetness index (`soil_level`, `soil_held`) fills the pores of those where the most gathers.
//! A law reads the share of its land saturated and the liquid of the drier rest (`wet_ground`).

use super::{soil_held, soil_level, Means};
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

/// The land of cell `i` whose pores, `pores` m deep as liquid, its liquid fills by the share `filled` (`cover::soak`,
/// past 1 where liquid stands over them): its samples (`World::terrain_samples`) but the lowest, which the liquid that
/// stands over the cell covers, by its land's share of its area (`contract::land`; the liquid stands over the lowest
/// first, `flow::curve`), each holding the liquid the wetness index shares to it (`soil_level`, `soil_held`). A sample
/// is saturated where that fills its pores: at an index of at least the level the soil's liquid departs about plus the
/// mean deficit over `soil_m` (the steady state of the liquid running downhill, Beven and Kirkby 1979). The drier rest
/// holds what the saturated part does not, so the cell's mean is kept.
pub fn wet_ground(w: &World, i: usize, (filled, pores): (f64, f64)) -> WetGround {
    let none = WetGround { share: 0.0, level: f64::INFINITY, filled: filled.clamp(0.0, 1.0) };
    if filled >= 1.0 {
        return WetGround { share: 1.0, level: f64::NEG_INFINITY, filled: 1.0 };
    }
    let s = w.terrain_samples();
    let mut at: Vec<(f64, f64)> = w.map.leaves()[i].split(SAMPLE_DEPTH).map(|f| s.index(&f)).map(|k| (s.h[k], s.twi[k])).collect();
    at.sort_by(|a, b| b.0.total_cmp(&a.0));
    let land = crate::contract::land(w, i) / w.mesh.cells().area()[i];
    at.truncate((land * at.len() as f64).round() as usize);
    let twi: Vec<f64> = at.iter().map(|p| p.1).collect();
    let area = vec![1.0 / twi.len().max(1) as f64; twi.len()];
    let (ph, m) = (&w.cfg.phys, Means { pore: filled * pores, pores, stands: 0.0 });
    let Some(c) = soil_level(ph, &twi, &area, m).filter(|_| !twi.is_empty()) else { return none };
    let held: Vec<f64> = twi.iter().map(|&t| soil_held(ph, m, t, c)).collect();
    let wet = held.iter().filter(|&&h| h >= pores).count();
    let dry: f64 = held.iter().filter(|&&h| h < pores).sum();
    let rest = (twi.len() - wet) as f64;
    WetGround { share: wet as f64 / twi.len() as f64, level: c + (pores - m.pore) / ph.soil_m, filled: if rest > 0.0 { dry / rest / pores } else { 1.0 } }
}

/// The land of cell `i` as its key liquid (that of `p`) lies over it now: how the cover's roots meet it (`cover::soak`)
/// and where it saturates the land (`wet_ground`, its pores as Flow has them, `Scratch::land_pores`).
pub fn wet_land(w: &World, p: &crate::cover::Parts, i: usize) -> (crate::cover::Soak, WetGround) {
    let soak = crate::cover::soak(w, p, i, w.g(p.liquid, i).max(0.0)).1;
    (soak, wet_ground(w, i, (soak.filled, w.scratch.land_pores(i).max(1e-9))))
}
