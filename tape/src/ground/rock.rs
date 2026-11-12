//! The rock of the ground a shell draws, per terrain sample: how deep its loose ground lies (`Samples::loose`; none where
//! the bed is bare) and the stones its channel moves. The largest stone a stream moves grows with the power it spends on
//! its bed, which goes as its discharge times its slope (Bagnold 1966); the wear's stream power law takes the discharge
//! as the square root of the area drained (`space::wear`, K √A S), so a stone's size is here `STONE_M` per unit of √A S.
//! A function of the samples alone.

use super::half;
use pcore::space::network::NONE;
use pcore::space::samples::Samples;

/// The largest stone a channel moves, m, per m of √(area drained, m²) × its slope: a stream draining 1 km² down a slope
/// of 0.5 moves stones of half a metre, a river of 100 km² on a slope of 0.01 stones of 0.1 m (mountain torrents move
/// boulders of 0.2 to 2 m, lowland rivers gravel; from memory).
const STONE_M: f64 = 0.001;
/// The largest stone drawn, m.
const STONE_MOST_M: f64 = 2.0;

/// Of each sample of `s`, `apart` m between neighbours' centres, row by row, four 16-bit floats: the depth of its loose
/// ground, m, the largest stone its channel moves, m (`STONE_M`; 0 where it runs nowhere: a sink, or under a pond), and
/// two zeros; then of each sample again the reflectance of the make of its bed (red, green, blue: `colours`, one a make,
/// `Samples::make`) and a zero: two planes of a texture, one under the other.
pub fn rock(s: &Samples, apart: f64, colours: &[[f64; 3]]) -> Vec<u16> {
    let mut out = Vec::with_capacity(8 * s.h.len());
    for k in 0..s.h.len() {
        let j = s.to[k];
        let stone = if j == NONE || s.pond[k] != NONE {
            0.0
        } else {
            let j = j as usize;
            let run = apart * if j % s.across != k % s.across && j / s.across != k / s.across { std::f64::consts::SQRT_2 } else { 1.0 };
            let slope = ((s.h[k] - s.h[j]) / run).max(0.0);
            (STONE_M * s.up[k].sqrt() * slope).min(STONE_MOST_M)
        };
        out.extend([half(s.loose[k] as f32), half(stone as f32), 0, 0]);
    }
    for &m in &s.make {
        let c = colours.get(m as usize).copied().unwrap_or([0.0; 3]);
        out.extend([half(c[0] as f32), half(c[1] as f32), half(c[2] as f32), 0]);
    }
    out
}
