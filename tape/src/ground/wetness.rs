//! How much liquid gathers at a place: the topographic wetness index ln(a / tan β) (Beven and Kirkby 1979) of the
//! terrain's samples, the samples' own (`space::samples::sample_index`), the one a shell shares each cell's soil liquid
//! by (`table`) and the law saturates a cell's land by (`inside::wet_ground`). A texel coarser than the samples takes
//! the mean of those it covers. A finer one takes it through the centres of the sixteen samples about it as a
//! Catmull-Rom spline (it keeps each sample's own index and is smooth with its slope, so a level read on it, the
//! saturated ground's edge, is a smooth line, not a sample's square or diamond); but a sample that drains a channel's
//! source (`inside::drains_source`) stands for its line: the law saturates the whole of it while its channel flows
//! (its index infinite, `inside::flowing`), so the spline takes the hillslope's index at its centre (`hill`) and its
//! line as drawn carries a strip a sample wide of the index `FLOWING_INDEX` (`bank_lines`), as much ground as the
//! sample holds, which narrows to none over the last `STRIP` of the line at its channel's head. So the wet ground hugs
//! the stream on both banks wherever its line bends. A known limit: the tiles never change, so a strip stays drawn in
//! a year its channel runs dry (the law then has its sample's own index). Each is a function of the place alone, the
//! same in every tile.

use super::channels::{to_segment, Window};
use pcore::space::network::NONE;
use pcore::space::samples::Samples;
use pcore::space::terrain::SAMPLE_DEPTH;
use pcore::Physics;

/// A strip of wet ground reaches this far from its line on each bank, samples: half a sample, so a strip along a line
/// a sample long holds a sample's area; and it narrows over this much of its line at its channel's head.
pub(super) const STRIP: f64 = 0.5;
/// The index a strip of wet ground holds: it stands for the law's infinite index of a flowing channel, over any level
/// the law saturates a cell's land from, and a 16-bit float holds it.
const FLOWING_INDEX: f64 = 1000.0;

/// Whether each of `s` drains a channel's source under the laws `ph` (`inside::drains_source`): its line carries a
/// strip of wet ground.
pub(super) fn strips(s: &Samples, ph: &Physics) -> Vec<bool> {
    (0..s.up.len()).map(|k| pcore::inside::drains_source(ph, s, k)).collect()
}

/// The index of each of `s` as the hillslope holds it: a sample of `strips` the mean of its eight neighbours' that are
/// none (its own where all are), the others' their own.
pub(super) fn hill(s: &Samples, strips: &[bool]) -> Vec<f64> {
    (0..s.twi.len())
        .map(|k| {
            if !strips[k] {
                return s.twi[k];
            }
            let (c, r) = ((k % s.across) as i64, (k / s.across) as i64);
            let near: Vec<f64> = (0..9)
                .map(|n| (c + n % 3 - 1, r + n / 3 - 1))
                .filter(|&(x, y)| (x, y) != (c, r) && x >= 0 && y >= 0 && x < s.across as i64 && y < s.down as i64)
                .map(|(x, y)| y as usize * s.across + x as usize)
                .filter(|&j| !strips[j])
                .map(|j| s.twi[j])
                .collect();
            if near.is_empty() { s.twi[k] } else { near.iter().sum::<f64>() / near.len() as f64 }
        })
        .collect()
}

/// Of each texel of `win`, whether a strip of wet ground takes it: one of the lines `lines` (each whether
/// its channel's head lies at its start, and its points, samples) lies within `STRIP` of it, or at a head within that
/// times the share 3t² - 2t³ of the way t its nearest point lies along the line's first `STRIP`.
pub(super) fn bank_lines<'a>(win: &Window, lines: impl Iterator<Item = (bool, Vec<[f64; 2]>)> + 'a) -> Vec<bool> {
    let mut out = vec![false; win.wide * win.high];
    for (head, pts) in lines {
        let mut from = 0.0;
        for w in pts.windows(2) {
            let (a, b) = (w[0], w[1]);
            let long = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
            for y in win.texels(a[1].min(b[1]) - STRIP, a[1].max(b[1]) + STRIP, 1) {
                for x in win.texels(a[0].min(b[0]) - STRIP, a[0].max(b[0]) + STRIP, 0) {
                    let (at, i) = win.texel(x, y);
                    let (_, d2, t) = to_segment(at, a, b);
                    let along = ((from + t * long) / STRIP).min(1.0);
                    let wide = if head { STRIP * along * along * (3.0 - 2.0 * along) } else { STRIP };
                    if d2 < wide * wide {
                        out[i] = true;
                    }
                }
            }
            from += long;
        }
    }
    out
}

/// The wetness index of samples `s` at the texels of `depth` from column `x0` and row `y0`, `wide` x `high` of them (on
/// the map), row by row: coarser than the samples the mean of their own, finer a Catmull-Rom spline through the
/// centres of the sixteen about a texel of the hillslope's (`hill`; the weights of a column and of a row found once
/// each), or `FLOWING_INDEX` where a strip takes it (`strip`, of each texel).
pub(super) fn field(s: &Samples, hill: &[f64], depth: u8, [x0, y0]: [i64; 2], [wide, high]: [usize; 2], strip: &[bool]) -> Vec<f64> {
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
            if strip[out.len()] {
                out.push(FLOWING_INDEX);
                continue;
            }
            let row = |r: i64| (0..4).map(|i| wu[i] * at(hill, c + i as i64 - 1, r)).sum::<f64>();
            out.push((0..4).map(|j| wv[j] * row(r + j as i64 - 1)).sum());
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
