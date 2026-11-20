//! How much liquid gathers at a place: the topographic wetness index ln(a / tan β) (Beven and Kirkby 1979) of the
//! terrain's samples, a the area that drains through a sample (`Samples::up`, the terrain's channels) over its width
//! and β the slope of the samples' ground there (`Grid::slope`, no less than real ground's over a sample). A texel finer
//! than the samples takes the index through the centres of the sixteen samples about it as a Catmull-Rom spline (it
//! keeps each sample's own index and is smooth with its slope, so a level read on it, the saturated ground's edge, is a
//! smooth line, not a sample's square or diamond); a coarser one the mean of the samples it covers. So the index is a
//! function of the place alone, the same in every tile and at every zoom, and the one a shell shares each cell's soil
//! liquid by (`table`): the samples' own (`space::samples::sample_index`).

use pcore::space::samples::Samples;
use pcore::space::terrain::SAMPLE_DEPTH;

/// The wetness index `twi` of samples `s` at the texels of `depth` from column `x0` and row `y0`, `wide` x `high` of them
/// (on the map), row by row: finer than the samples a Catmull-Rom spline through the centres of the sixteen about a texel
/// (the weights of a column and of a row found once each), coarser the mean of those it covers.
pub(super) fn field(s: &Samples, twi: &[f64], depth: u8, [x0, y0]: [i64; 2], [wide, high]: [usize; 2]) -> Vec<f64> {
    let index = |c: i64, r: i64| twi[r.clamp(0, s.down as i64 - 1) as usize * s.across + c.clamp(0, s.across as i64 - 1) as usize];
    if depth <= SAMPLE_DEPTH {
        let k = 1i64 << (SAMPLE_DEPTH - depth);
        let at = |x: i64, y: i64| (0..k * k).map(|q| index(x * k + q % k, y * k + q / k)).sum::<f64>() / (k * k) as f64;
        return (0..wide * high).map(|p| at(x0 + (p % wide) as i64, y0 + (p / wide) as i64)).collect();
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
            let row = |r: i64| (0..4).map(|i| wu[i] * index(c + i as i64 - 1, r)).sum::<f64>();
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
