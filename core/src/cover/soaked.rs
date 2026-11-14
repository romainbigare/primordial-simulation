//! Roots in full pores (laws.md 3): a root burns its fuel with the air's gas, which reaches it through the open pores of
//! its ground; in full pores that gas moves about 10,000 times slower than in air (`units::D_GAS_AIR`), so a root there
//! gets none from its ground, only what its own open channels (`CHANNEL`) carry down from the air (Armstrong 1979).
//! What gets none splits its fuel without the gas (`Parts::split`), which frees far less a mol of fuel (Drew 1997).

use super::{Parts, CHANNEL, NG};
use crate::chem::Chemistry;
use crate::life::WET_TISSUE;
use crate::units::{D_GAS_AIR, DAY, R_GAS};

/// The depth under the surface to which the open channels of a root of genes `g` carry the burning's gas from the air,
/// m: a root whose share ε of its section is channel, its tissue using the gas at q mol a m³ a s, is supplied down to
/// L = (2 D ε C / ((1 − ε) q))^½ (diffusion down a pipe whose walls use what passes; Armstrong 1979), D the gas's
/// diffusivity in air, C its concentration in the air near the ground, `p_gas` Pa at `t` K, and q its basal upkeep
/// (`c_basal` at `pace` of the burning's pace at `t_cat`) burnt with the gas, in tissue of the density of body matter
/// and the liquid it holds (`life::WET_TISSUE`).
pub(super) fn aerated(chem: &Chemistry, ph: &crate::Physics, p: &Parts, g: &[f32; NG], (t, pace, p_gas): (f64, f64, f64)) -> f64 {
    let open = (g[CHANNEL] as f64).clamp(0.0, 1.0 - 1e-9);
    let burn = &chem.reactions[p.burn];
    let gas_per: f64 = burn.inputs.iter().filter(|q| !p.sub.contains(&(q.0 as usize))).map(|q| q.1 as f64).sum();
    let density = 1.0 / ((1.0 - WET_TISSUE) / chem.subs[p.sub[super::MATTER]].density[0] + WET_TISSUE / chem.subs[p.liquid].density[1]);
    let q = ph.c_basal * pace / DAY / -chem.dh(p.burn) * gas_per * density;
    let c = p_gas / (R_GAS * t.max(1.0));
    (2.0 * D_GAS_AIR * open * c / ((1.0 - open) * q).max(1e-300)).sqrt()
}

/// The share of the fine roots of a plant that stand in full pores (from `full` m under the surface,
/// `cover::full_from`) deeper than its channels carry the gas (`aerated` m). Its roots reach `need` m (no deeper than its
/// `loose` m of loose ground), but its fine roots, which take up and burn, lie near the top whatever its height: the
/// share above depth d is 1 − e^(−d / `fine_root_m`), within its reach (Jackson, Mooney and Schulze 1997; the frame that
/// holds the plant goes deep, `stand::need`).
pub(super) fn drowned(ph: &crate::Physics, need: f64, (full, loose): (f64, f64), aerated: f64) -> f64 {
    let (deep, from) = (need.min(loose), full.max(aerated));
    if deep <= 0.0 || from >= deep {
        return 0.0;
    }
    let above = |d: f64| -(-d / ph.fine_root_m).exp_m1();
    (1.0 - above(from) / above(deep)).clamp(0.0, 1.0)
}
