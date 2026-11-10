//! How much liquid gathers at a place: the topographic wetness index ln(a / tan β) (Beven and Kirkby 1979) of the
//! terrain's samples, a the area that drains through a sample (`Samples::up`, the terrain's channels) over its width
//! and β the slope of the samples' ground there (`Grid::slope`, no less than real ground's over a sample). A texel finer
//! than the samples takes the index blended between the centres of the four samples about it; a coarser one the mean
//! of the samples it covers. So the index is a function of the place alone, the same in every tile and at every zoom,
//! and the one a shell shares each cell's soil liquid by (`table`).

use pcore::inside::Grid;
use pcore::space::samples::Samples;
use pcore::space::square::Square;
use pcore::space::terrain::SAMPLE_DEPTH;

/// The wetness index of each of `s`, the samples of `map`, row by row.
pub fn sample_index(map: &Square, s: &Samples) -> Vec<f64> {
    let dy = map.length(SAMPLE_DEPTH);
    let grid = Grid::new(s.across, s.down, dy, vec![dy; s.down]);
    let own = vec![dy * dy; s.h.len()];
    Grid::wetness_index(&s.up, &own, &grid.slope(&s.h))
}

/// The wetness index `twi` of samples `s` at the texel at column `x` and row `y` of leaves of `depth`.
pub(super) fn at(s: &Samples, twi: &[f64], depth: u8, (x, y): (i64, i64)) -> f64 {
    let index = |c: i64, r: i64| twi[r.clamp(0, s.down as i64 - 1) as usize * s.across + c.clamp(0, s.across as i64 - 1) as usize];
    if depth <= SAMPLE_DEPTH {
        let k = 1i64 << (SAMPLE_DEPTH - depth);
        let sum: f64 = (0..k * k).map(|q| index(x * k + q % k, y * k + q / k)).sum();
        return sum / (k * k) as f64;
    }
    let per = (1i64 << (depth - SAMPLE_DEPTH)) as f64;
    let (u, v) = ((x as f64 + 0.5) / per - 0.5, (y as f64 + 0.5) / per - 0.5);
    let (c, r) = (u.floor() as i64, v.floor() as i64);
    let (fu, fv) = (u - c as f64, v - r as f64);
    let row = |r: i64| index(c, r) * (1.0 - fu) + index(c + 1, r) * fu;
    row(r) * (1.0 - fv) + row(r + 1) * fv
}
