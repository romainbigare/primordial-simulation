//! How much liquid gathers at a place: the topographic wetness index ln(a / tan β) (Beven and Kirkby 1979) of the
//! terrain's samples, the samples' own (`space::samples::sample_index`), the one a shell shares each cell's soil liquid
//! by (`table`) and the law saturates a cell's land by (`inside::wet_ground`). A texel coarser than the samples takes
//! the mean of those it covers. A finer one takes it through the centres of the sixteen samples about it as a
//! Catmull-Rom spline (it keeps each sample's own index and is smooth with its slope, so a level read on it, the
//! saturated ground's edge, is a smooth line, not a sample's square or diamond), but a channel's sample stands there
//! for its line: the spline takes the hillslope's index at its centre (`hill`), and its own index lies along its line
//! as drawn (`bank_lines`, the channels' lines that carve a valley, `valley::carves`), falling to the hillslope's
//! across `BANK` on each bank as the spline falls from one sample's centre to the next. So a channel sample's wet
//! ground hugs its stream on both banks wherever the line bends in it, as wide as the spline drew it about the
//! sample's centre.
//! Each is a function of the place alone, the same in every tile.

use super::channels::{to_segment, Window};
use super::valley::carves;
use pcore::space::network::NONE;
use pcore::space::samples::Samples;
use pcore::space::terrain::SAMPLE_DEPTH;

/// A channel sample's own index falls to its hillslope's over this far from its line on each bank, samples: a sample,
/// as the spline falls from one sample's centre to the next.
const BANK: f64 = 1.0;

/// The index of each of `s` as the hillslope holds it: a channel sample's (one whose line carves a valley,
/// `valley::carves`) the mean of its eight neighbours' that are none (its own where all are), the others' their own.
pub(super) fn hill(s: &Samples) -> Vec<f64> {
    (0..s.twi.len())
        .map(|k| {
            if !carves(s, k) {
                return s.twi[k];
            }
            let (c, r) = ((k % s.across) as i64, (k / s.across) as i64);
            let near: Vec<f64> = (0..9)
                .map(|n| (c + n % 3 - 1, r + n / 3 - 1))
                .filter(|&(x, y)| (x, y) != (c, r) && x >= 0 && y >= 0 && x < s.across as i64 && y < s.down as i64)
                .map(|(x, y)| y as usize * s.across + x as usize)
                .filter(|&j| !carves(s, j))
                .map(|j| s.twi[j])
                .collect();
            if near.is_empty() { s.twi[k] } else { near.iter().sum::<f64>() / near.len() as f64 }
        })
        .collect()
}

/// Of each texel of `win`, the nearest of the lines `lines` (each its sample and its points, samples) within `BANK`
/// of it: its distance, samples, and its sample (`NONE` for none).
pub(super) fn bank_lines<'a>(win: &Window, lines: impl Iterator<Item = (usize, Vec<[f64; 2]>)> + 'a) -> Vec<(f64, u32)> {
    let mut out = vec![(f64::INFINITY, NONE); win.wide * win.high];
    for (q, pts) in lines {
        for w in pts.windows(2) {
            let (a, b) = (w[0], w[1]);
            for y in win.texels(a[1].min(b[1]) - BANK, a[1].max(b[1]) + BANK, 1) {
                for x in win.texels(a[0].min(b[0]) - BANK, a[0].max(b[0]) + BANK, 0) {
                    let (at, i) = win.texel(x, y);
                    let d = to_segment(at, a, b).1.sqrt();
                    if d < BANK && d < out[i].0 {
                        out[i] = (d, q as u32);
                    }
                }
            }
        }
    }
    out
}

/// The wetness index of samples `s` at the texels of `depth` from column `x0` and row `y0`, `wide` x `high` of them (on
/// the map), row by row: coarser than the samples the mean of their own, finer a Catmull-Rom spline through the
/// centres of the sixteen about a texel of the hillslope's (`hill`; the weights of a column and of a row found once
/// each), or where more, the index of the nearest channel's line within `BANK` (`banks`, of each texel), falling from
/// its sample's own on the line to its hillslope's at `BANK` as a Catmull-Rom spline between two flat ends does
/// (3t² - 2t³).
pub(super) fn field(s: &Samples, hill: &[f64], depth: u8, [x0, y0]: [i64; 2], [wide, high]: [usize; 2], banks: &[(f64, u32)]) -> Vec<f64> {
    let at = |v: &[f64], c: i64, r: i64| v[r.clamp(0, s.down as i64 - 1) as usize * s.across + c.clamp(0, s.across as i64 - 1) as usize];
    if depth <= SAMPLE_DEPTH {
        let k = 1i64 << (SAMPLE_DEPTH - depth);
        let mean = |x: i64, y: i64| (0..k * k).map(|q| at(&s.twi, x * k + q % k, y * k + q / k)).sum::<f64>() / (k * k) as f64;
        return (0..wide * high).map(|p| mean(x0 + (p % wide) as i64, y0 + (p / wide) as i64)).collect();
    }
    let per = (1i64 << (depth - SAMPLE_DEPTH)) as f64;
    // The sample before a texel's centre along an axis, and the weights of the four samples from the one before it.
    let weights = |t0: i64, n: usize| -> Vec<(i64, [f64; 4])> {
        (0..n as i64).map(|t| { let u = ((t0 + t) as f64 + 0.5) / per - 0.5; (u.floor() as i64, catmull_rom(u - u.floor())) }).collect()
    };
    let (cols, rows) = (weights(x0, wide), weights(y0, high));
    let mut out = Vec::with_capacity(wide * high);
    for &(r, wv) in &rows {
        for &(c, wu) in &cols {
            let row = |r: i64| (0..4).map(|i| wu[i] * at(hill, c + i as i64 - 1, r)).sum::<f64>();
            let slope: f64 = (0..4).map(|j| wv[j] * row(r + j as i64 - 1)).sum();
            let (d, q) = banks[out.len()];
            let t = d / BANK;
            let line = if q == NONE { f64::NEG_INFINITY } else { s.twi[q as usize] - (s.twi[q as usize] - hill[q as usize]) * t * t * (3.0 - 2.0 * t) };
            out.push(slope.max(line));
        }
    }
    out
}

/// The weights of a uniform Catmull-Rom spline at `t` (0..1) on the four points about it (the second and third the
/// two it runs between).
fn catmull_rom(t: f64) -> [f64; 4] {
    let (t2, t3) = (t * t, t * t * t);
    [0.5 * (-t3 + 2.0 * t2 - t), 0.5 * (3.0 * t3 - 5.0 * t2 + 2.0), 0.5 * (-3.0 * t3 + 4.0 * t2 + t), 0.5 * (t3 - t2)]
}
