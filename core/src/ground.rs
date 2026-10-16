//! Heat of ground and air nodes: heat capacities, temperatures, the melting plateaus of the
//! ground, and drawing heat from a node without taking it below `T_FLOOR`.

use crate::chem::{self, Chemistry};
use crate::grid::Grid;
use crate::params::Config;
use crate::units::T_FLOOR;
use crate::world::{Ledger, World};

/// `World::take_heat` for callers that hold other parts of the world borrowed: take up to `q` from
/// a node with energy `e` and heat capacity `cap` (at most what it holds above `T_FLOOR`), count
/// the shortfall, return what was taken.
#[inline]
pub(crate) fn draw(e: &mut f64, ledger: &mut Ledger, cap: f64, q: f64) -> f64 {
    let took = if q > 0.0 { q.min(heat_above_floor(*e, cap)) } else { q };
    *e -= took;
    ledger.clamped_heat += q - took;
    took
}

/// The share of a wanted amount that was really given (exactly 1 when all of it was).
#[inline]
pub(crate) fn share(took: f64, want: f64) -> f64 {
    if took >= want { 1.0 } else { took / want }
}

/// Heat capacity of a ground node's active layer, J/K: its loose amounts, and `skin_m` of its bed.
pub(crate) fn cap_ground(chem: &Chemistry, grid: &Grid, cfg: &Config, ground: &[f64], bed: &[f64], i: usize) -> f64 {
    let n = grid.n();
    let (mut c, mut bc, mut bv) = (0.0, 0.0, 0.0);
    for k in 0..chem.ns() {
        let s = &chem.subs[k];
        c += ground[k * n + i] * s.cp;
        let b = bed[k * n + i];
        bc += b * s.cp;
        bv += b * s.molar_mass / s.density[0];
    }
    if bv > 0.0 {
        c += bc / bv * cfg.skin_m * grid.area();
    }
    c
}

/// Heat a node with energy `e` and heat capacity `cap` can give before it falls to `T_FLOOR`, J.
#[inline]
pub(crate) fn heat_above_floor(e: f64, cap: f64) -> f64 {
    (e - cap.max(1.0) * T_FLOOR).max(0.0)
}

impl World {
    /// Heat capacity of a ground node's active layer, J/K.
    pub fn heat_cap_ground(&self, i: usize) -> f64 {
        cap_ground(&self.chem, &self.grid, &self.cfg, &self.ground, &self.bed, i)
    }
    /// Melting point of substance `k` in ground node `i`, lowered by what is dissolved in its liquid (`Chemistry::melt_in`).
    pub fn melt(&self, k: usize, i: usize) -> f64 {
        self.chem.melt_in(k, |j| self.g(j, i))
    }
    /// Temperature of a ground node, and the substance held at its melting point with its
    /// liquid share, if any. Molten amounts hold their fusion enthalpy in `e_ground`, so a node
    /// freezes or melts at a plateau.
    pub fn thaw(&self, i: usize) -> (f64, Option<(usize, f64)>) {
        let c = self.heat_cap_ground(i).max(1.0);
        let mut ks: Vec<(f64, usize)> = (0..self.chem.ns()).filter(|&k| self.g(k, i) > 0.0).map(|k| (self.melt(k, i), k)).collect();
        ks.sort_by(|a, b| a.0.total_cmp(&b.0));
        let (e, mut held) = (self.e_ground[i], 0.0);
        for (tm, k) in ks {
            let lat = self.g(k, i) * self.chem.subs[k].lf;
            let lo = c * tm + held;
            if e < lo {
                break;
            }
            if e < lo + lat {
                return (tm, Some((k, (e - lo) / lat)));
            }
            held += lat;
        }
        ((e - held) / c, None)
    }
    /// Take up to `q` J from ground node `i`, never more than it holds above `T_FLOOR`, and return what was
    /// really taken. The shortfall is counted in `ledger.clamped_heat`. Whoever wanted `q` must
    /// pass on only the returned amount (evaporate less, carry less), so energy stays exact.
    /// A negative `q` gives heat to the node and is returned whole.
    pub fn take_heat(&mut self, i: usize, q: f64) -> f64 {
        self.take_heat_with(i, q, 0.0)
    }
    /// `take_heat` for heat that leaves with matter of heat capacity `c` J/K (still counted in
    /// the node): only the heat capacity that stays must be left above `T_FLOOR`.
    pub fn take_heat_with(&mut self, i: usize, q: f64, c: f64) -> f64 {
        // Only a take needs the capacity (a walk over the node's substances): a body taking from
        // the ground asks once per substance, mostly for nothing.
        let cap = if q > 0.0 { self.heat_cap_ground(i) - c } else { 0.0 };
        draw(&mut self.e_ground[i], &mut self.ledger, cap, q)
    }
    /// Temperature of ground node `i`, K.
    pub fn t_ground(&self, i: usize) -> f64 {
        self.thaw(i).0
    }
    /// Thermal energy of a ground node at temperature `t` (with the fusion enthalpy of what is molten).
    pub fn energy_ground(&self, i: usize, t: f64) -> f64 {
        let held: f64 = (0..self.chem.ns()).filter(|&k| self.melt(k, i) < t).map(|k| self.g(k, i) * self.chem.subs[k].lf).sum();
        self.heat_cap_ground(i) * t + held
    }
    /// Molten share of substance `k` in ground node `i` at temperature `t` (from `thaw`).
    pub fn molten(&self, k: usize, i: usize, t: f64, plateau: Option<(usize, f64)>) -> f64 {
        match plateau {
            Some((pk, f)) if pk == k => f,
            _ => (t >= self.melt(k, i)) as u8 as f64,
        }
    }
    /// Heat capacity of air node `j`, J/K.
    pub fn heat_cap_air(&self, j: usize) -> f64 {
        (0..self.chem.ns()).map(|k| self.a(k, j) * self.chem.subs[k].cp).sum()
    }
    /// Temperature of air node `j`, K.
    pub fn t_air(&self, j: usize) -> f64 {
        self.e_air[j] / self.heat_cap_air(j).max(1.0)
    }
    /// Height of the solid surface (bonded + loose solids), m.
    pub fn surface_z(&self, i: usize) -> f64 {
        let (t, pl) = self.thaw(i);
        let mut v = 0.0;
        for k in 0..self.chem.ns() {
            let mv = self.chem.molar_volume(k, chem::Phase::Solid);
            v += (self.b(k, i) + self.g(k, i) * (1.0 - self.molten(k, i, t, pl))) * mv;
        }
        self.base_z + v / self.grid.area()
    }
}
