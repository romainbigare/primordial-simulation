//! The share of a body's heat radiation in each band of it (`Chemistry::heat_spans`), by Planck's law, as a table by
//! temperature (`table`), read by `Chemistry::heat_shares`.

use super::HEAT_BANDS;
use crate::units::C2;
use std::f64::consts::PI;

/// The table holds whole K from 0 to this; a body warmer takes the shares at it.
pub(super) const TABLE_K: usize = 2000;
/// Terms of the series of `below`: past 40 they change nothing a double holds where a span's edge is not far beyond
/// the peak.
const TERMS: usize = 40;

/// The share of a black body's radiation at `t` K at wavelengths below `um` µm: (15/π⁴) Σ e^(−nx)/n (x³ + 3x²/n +
/// 6x/n² + 6/n³), x = `C2` / (λ t) (the series of Planck's law, as in Siegel and Howell's tables): 0 below no
/// wavelength, 1 below all.
fn below(um: f64, t: f64) -> f64 {
    let x = C2 / (um * 1e-6 * t.max(f64::MIN_POSITIVE));
    let share: f64 = (1..=TERMS).map(|n| {
        let n = n as f64;
        (-n * x).exp() / n * (x * x * x + 3.0 * x * x / n + 6.0 * x / (n * n) + 6.0 / (n * n * n))
    }).sum::<f64>() * 15.0 / PI.powi(4);
    share.clamp(0.0, 1.0)
}

/// The share of each band at each whole K from 0 to `TABLE_K`: each span's `below` its top less `below` its bottom,
/// the last band the rest, so they sum to 1 exactly. At 0 K all in the last band.
pub(super) fn table(spans: &[[f64; 2]; HEAT_BANDS - 1]) -> Vec<[f64; HEAT_BANDS]> {
    (0..=TABLE_K)
        .map(|k| {
            let t = k as f64;
            let mut s = [0.0; HEAT_BANDS];
            if t > 0.0 {
                spans.iter().enumerate().for_each(|(b, sp)| s[b] = (below(sp[1], t) - below(sp[0], t)).max(0.0));
            }
            s[HEAT_BANDS - 1] = 1.0 - s[..HEAT_BANDS - 1].iter().sum::<f64>();
            s
        })
        .collect()
}
