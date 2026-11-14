//! What a slot of takers needs in a month (laws.md 3b): the energy it burns for its upkeep and to keep its body's heat,
//! the key liquid it sheds as vapour to cool and the least it passes through, and how fast it wears.

use super::{Fauna, BODY, COAT, KEEP, NF, REPAIR};
use crate::life::Parts;
use crate::math::dec;
use crate::units::DAY;
use crate::World;

/// A slot's month: its members' living mass, kg each, its body's temperature, K, and its pace (its burning's rate there
/// over that at `t_cat`); the energy it burns, J, the key liquid it sheds as vapour and the least it passes through
/// besides, mol; its wear (the energy its members turn over per kg over what a kg turns over in a life).
#[derive(Clone, Copy, Default)]
pub(super) struct Need {
    pub(super) mass: f64,
    pub(super) tb: f64,
    pub(super) pace: f64,
    pub(super) burn: f64,
    pub(super) vapour: f64,
    pub(super) least: f64,
    pub(super) wear: f64,
}

/// The need of slot `x` of `f` over `dt` s on ground at `t` K.
/// - Its body stands at its `KEEP` gene where the ground is colder, else at the ground's temperature; its upkeep runs at
///   the pace of its burning there (the bare rate law, Gillooly et al. 2001).
/// - Upkeep: Kleiber's law, (`c_basal` + `c_muscle`) × m^`upkeep_exp` × pace × its repair factor a day per member
///   (Kleiber 1932; Elia 1992 for the muscle).
/// - Heat: a member held above its ground loses g (T_b − t) through its coat, g = `meeh` m^(2/3) / (`r_air` + COAT ×
///   `r_coat`) (Meeh 1879; Scholander et al. 1950); where that is more than its upkeep gives it burns the rest; where
///   less, the excess leaves as vapour of the key liquid at its heat of vaporisation. A body at its ground's temperature
///   gives all its heat to its ground and needs no vapour.
/// - Liquid: besides that vapour a member passes `drink_least` × m^`drink_exp` kg a day through its body, which goes
///   back to its cell in the month (Nagy and Peterson 1988).
/// - Wear: the energy it burns per kg of living mass over `e_life` × 10^REPAIR (the rate of living, Rubner 1908).
pub(super) fn need(w: &World, p: &Parts, f: &Fauna, (x, mm): (usize, &[f64; NF]), (t, dt): (f64, f64)) -> Need {
    let (chem, ph, g, n) = (&w.chem, &w.cfg.phys, &f.genes[x], f.adults[x]);
    if n <= 0.0 {
        return Need::default();
    }
    let m = f.mol[x][BODY] * mm[BODY] / (1.0 - crate::life::WET_TISSUE) / n;
    let (keep, rx) = (g[KEEP] as f64, &chem.reactions[p.burn]);
    let tb = keep.max(t);
    let pace = rx.arrhenius(tb.max(1.0)) / rx.arrhenius(ph.t_cat);
    let rest = m.powf(ph.upkeep_exp) * (ph.c_basal + ph.c_muscle) * pace * ph.repair(g[REPAIR] as f64) / DAY;
    let (held, lost) = if keep > t { (true, coat(ph, m, g[COAT]) * (keep - t)) } else { (false, 0.0) };
    let burnt = rest.max(lost);
    let shed = if held { (rest - lost).max(0.0) } else { 0.0 };
    let liquid = &chem.subs[p.liquid];
    Need {
        mass: m,
        tb,
        pace,
        burn: n * burnt * dt,
        vapour: n * shed * dt / liquid.lv,
        least: n * ph.drink_least * m.powf(ph.drink_exp) * dt / DAY / liquid.molar_mass,
        wear: burnt * dt / (m * ph.e_life * dec(g[REPAIR])).max(1e-300),
    }
}

/// The conductance of a member of `m` kg living through its coat of gene `coat` to its ground, W/K: its surface (Meeh
/// 1879) over the insulation of the still air at it and of its coat.
fn coat(ph: &crate::Physics, m: f64, coat: f32) -> f64 {
    ph.meeh * m.powf(2.0 / 3.0) / (ph.r_air + coat as f64 * ph.r_coat)
}
