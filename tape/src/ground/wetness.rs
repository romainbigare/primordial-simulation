//! How much liquid gathers at a place: the topographic wetness index ln(a / tan β) (Beven and Kirkby 1979) of the
//! terrain's samples, a the area that drains through a sample (`Samples::up`, the terrain's channels) over its width
//! and β the slope of the samples' ground there (`Grid::slope`, no less than real ground's over a sample). A texel finer
//! than the samples takes the index blended between the centres of the four samples about it; a coarser one the mean
//! of the samples it covers. So the index is a function of the place alone, the same in every tile and at every zoom,
//! and the one a shell shares each cell's soil liquid by (`table`): the samples' own (`space::samples::sample_index`).

use pcore::space::samples::Samples;
use pcore::space::terrain::SAMPLE_DEPTH;

/// The wetness index `twi` of samples `s` at the texels of `depth` from column `x0` and row `y0`, `wide` x `high` of them
/// (on the map), row by row: finer than the samples blended between the centres of the four about a texel (the weights
/// of a column and of a row found once each), coarser the mean of those it covers.
pub(super) fn field(s: &Samples, twi: &[f64], depth: u8, [x0, y0]: [i64; 2], [wide, high]: [usize; 2]) -> Vec<f64> {
    let index = |c: i64, r: i64| twi[r.clamp(0, s.down as i64 - 1) as usize * s.across + c.clamp(0, s.across as i64 - 1) as usize];
    if depth <= SAMPLE_DEPTH {
        let k = 1i64 << (SAMPLE_DEPTH - depth);
        let at = |x: i64, y: i64| (0..k * k).map(|q| index(x * k + q % k, y * k + q / k)).sum::<f64>() / (k * k) as f64;
        return (0..wide * high).map(|p| at(x0 + (p % wide) as i64, y0 + (p / wide) as i64)).collect();
    }
    let per = (1i64 << (depth - SAMPLE_DEPTH)) as f64;
    // The sample before a texel's centre along an axis, and its weight on the one after.
    let weights = |t0: i64, n: usize| -> Vec<(i64, f64)> {
        (0..n as i64).map(|t| { let u = ((t0 + t) as f64 + 0.5) / per - 0.5; (u.floor() as i64, u - u.floor()) }).collect()
    };
    let (cols, rows) = (weights(x0, wide), weights(y0, high));
    let mut out = Vec::with_capacity(wide * high);
    for &(r, fv) in &rows {
        for &(c, fu) in &cols {
            let row = |r: i64| index(c, r) * (1.0 - fu) + index(c + 1, r) * fu;
            out.push(row(r) * (1.0 - fv) + row(r + 1) * fv);
        }
    }
    out
}
