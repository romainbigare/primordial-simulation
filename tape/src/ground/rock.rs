//! The rock of the ground a shell draws, per terrain sample: how deep its loose ground lies (`Samples::loose`; none where
//! the bed is bare) and the stones its channel moves. The largest stone a stream moves grows with the power it spends on
//! its bed, which goes as its discharge times its slope (Bagnold 1966); the wear's stream power law takes the discharge
//! as the square root of the area drained (`space::wear`, K √A S), so a stone's size is here `STONE_M` per unit of √A S,
//! S the slope of the reach about the sample (`bends::Reaches::fall`: from one sample to the next the ground steps),
//! which a shell also reads alone: the form of the bed (riffles and pools, or steps) follows it.
//! And the links of its channel (`links`), by which a shell finds the samples up and down it: what runs through a
//! channel grows from one sample's centre to the next, not in a step at their edge. And the texture of its loose ground
//! (`pcore::soil::textures`), which a shell draws its soil by; and the load the liquid laid on it and how long its loose
//! ground has lain, which a shell draws its deposits by.

use super::bends::Reaches;
use super::half;
use crate::layers::shown;
use pcore::soil::{Pores, Texture};
use pcore::space::network::NONE;
use pcore::space::samples::Samples;

/// The largest stone a channel moves, m, per m of √(area drained, m²) × its slope: a stream draining 1 km² down a slope
/// of 0.5 moves stones of half a metre, a river of 100 km² on a slope of 0.01 stones of 0.1 m (mountain torrents move
/// boulders of 0.2 to 2 m, lowland rivers gravel; from memory).
const STONE_M: f64 = 0.001;
/// The largest stone drawn, m.
const STONE_MOST_M: f64 = 2.0;
/// The age written where no load was laid, years: older than any a shell draws as fresh, so the edge of a deposit,
/// blended between samples, grows no fresher toward bare ground.
const UNLAID_YEARS: f64 = 1e6;

/// The step from sample `k` of `s` to its neighbour `j`, 1 to 8: 1 + (dx + 1) + 3 (dy + 1), less one past the middle.
fn step(s: &Samples, k: usize, j: usize) -> u32 {
    let (dx, dy) = ((j % s.across) as i64 - (k % s.across) as i64, (j / s.across) as i64 - (k / s.across) as i64);
    debug_assert!(dx.abs() <= 1 && dy.abs() <= 1 && (dx, dy) != (0, 0), "a neighbour");
    let n = (dx + 1) + 3 * (dy + 1);
    (if n > 4 { n } else { n + 1 }) as u32
}

/// The links of sample `k`'s channel in `s`, as one whole number a 16-bit float holds: the step to the sample it runs
/// to (`step`; 0: none), 8 more where it joins that one's channel as a tributary (that one's main way comes from
/// elsewhere), and 17 times the step to its main neighbour upstream (0: none).
fn links(s: &Samples, k: usize) -> u32 {
    let down = match s.to[k] {
        NONE => 0,
        j => step(s, k, j as usize) + if s.main[j as usize] == k as u32 { 0 } else { 8 },
    };
    let up = if s.main[k] == NONE { 0 } else { step(s, k, s.main[k] as usize) };
    down + 17 * up
}

/// How long the loose ground of a sample on which `settled` m of load was laid has lain, `age` years: at least a year,
/// and `UNLAID_YEARS` where none was laid.
fn laid_years(settled: f64, age: f64) -> f64 {
    if settled > 0.0 { age.max(1.0) } else { UNLAID_YEARS }
}

/// Of each sample of the reaches `r`, row by row, four 16-bit floats: the depth of its loose ground, m, the largest stone
/// its channel moves, m (`STONE_M`; 0 where it runs nowhere: a sink, or under a pond), the slope of its reach (rise
/// over run; 0 there too) and its channel's links (`links`); then of each sample again the reflectance of the make of
/// its bed (red, green, blue: `colours`, one a make, `Samples::make`) and the wavelength of its channel's bends, m
/// (`Reaches::wavelength`, by which a shell draws as much of them as its pixels show); then of each sample again the shares of
/// coarse and fine particles of its loose ground (`grain`, one a sample), how slowly its pores pass liquid, the decimal
/// logarithm of 1 m/s over their conductivity (`Pores::of`; 4 for 0.1 mm/s, 7 for 0.1 µm/s), and the liquid it holds
/// for roots (`Pores::for_roots` over its loose ground's depth) as the layer `avail` shows it (`layers::shown`); then of
/// each sample again two zeros, how deep the load the
/// liquid laid lies on it, m (`Samples::settled`), and the decimal logarithm of the years its loose ground has lain
/// (`laid_years`), by which a shell draws a deposit: four planes of a texture, one under the other.
pub(super) fn rock(r: &Reaches, colours: &[[f64; 3]], grain: &[Texture]) -> Vec<u16> {
    let s = r.s;
    let mut out = Vec::with_capacity(16 * s.h.len());
    for k in 0..s.h.len() {
        let slope = if s.to[k] == NONE || s.pond[k] != NONE { 0.0 } else { r.fall(k)[0] };
        let stone = (STONE_M * s.up[k].sqrt() * slope).min(STONE_MOST_M);
        out.extend([half(s.loose[k] as f32), half(stone as f32), half(slope as f32), half(links(s, k) as f32)]);
    }
    for (k, &m) in s.make.iter().enumerate() {
        let c = colours.get(m as usize).copied().unwrap_or([0.0; 3]);
        out.extend([half(c[0] as f32), half(c[1] as f32), half(c[2] as f32), half(r.wavelength(k) as f32)]);
    }
    let pores: Vec<Pores> = grain[..s.h.len()].iter().map(Pores::of).collect();
    let avail = shown("avail");
    for ((t, p), &loose) in grain.iter().zip(&pores).zip(&s.loose) {
        let roots = avail(p.for_roots(loose));
        out.extend([half(t.coarse as f32), half(t.fine as f32), half(-p.seep.log10() as f32), half(roots)]);
    }
    for (&settled, &age) in s.settled.iter().zip(&s.age) {
        out.extend([0, 0, half(settled as f32), half(laid_years(settled, age).log10() as f32)]);
    }
    out
}
