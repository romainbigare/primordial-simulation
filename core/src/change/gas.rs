//! The gas a reactor exchanged with the air around it: what it takes in latent and sensible heat, and where it goes.

use crate::chem::Chemistry;

/// The gas a reactor put out to and took from the air around it: `pool` after it reacted, `pool0` before, mol of each.
pub(crate) struct GasMove {
    /// Fusion and vaporisation enthalpy the gas put out takes, J; that the gas taken gives up, J
    pub pay: f64,
    pub back: f64,
    /// Heat capacity of the gas put out, J/K; heat the gas taken brings at the air's temperature, J
    pub c_out: f64,
    pub q_in: f64,
}

impl GasMove {
    /// The exchange of `pool` against `pool0`, with air at `ta` K.
    pub(crate) fn of(chem: &Chemistry, pool: &[f64], pool0: &[f64], ta: f64) -> Self {
        let (mut pay, mut back, mut c_out, mut q_in) = (0.0, 0.0, 0.0, 0.0);
        for (k, (&p, &p0)) in pool.iter().zip(pool0).enumerate() {
            let (d, s) = (p - p0, &chem.subs[k]);
            (pay, back) = (pay + d.max(0.0) * s.latent_gas(), back - d.min(0.0) * s.latent_gas());
            (c_out, q_in) = (c_out + d.max(0.0) * s.cp, q_in - d.min(0.0) * s.cp * ta);
        }
        GasMove { pay, back, c_out, q_in }
    }
}

/// Settle the gas of `pool` against `pool0`: of what was put out, the share `paid` goes to the air node's amounts `air`
/// ([k * na + j]) and the rest stays with what reacted (`keep(k, mol)`); what was taken leaves the air.
pub(crate) fn send_gas(pool: &[f64], pool0: &[f64], paid: f64, air: &mut [f64], na: usize, j: usize, mut keep: impl FnMut(usize, f64)) {
    for (k, (&p, &p0)) in pool.iter().zip(pool0).enumerate() {
        let d = p - p0;
        let sent = if d > 0.0 { d * paid } else { d };
        keep(k, d - sent);
        air[k * na + j] += sent;
    }
}
