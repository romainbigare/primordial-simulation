//! Inside a cell (laws.md 0, `pcore::inside`): the liquid in a cell's soil lies over its ground by how much liquid
//! gathers at each point, and keeps the cell's mean exactly.

use pcore::inside::{deficit_at, saturated, soil_held, soil_level, Means};
use pcore::Physics;

/// The liquid in its pores, its pores' room and what stands, m: a cell over field capacity, its table in its loose
/// ground; its bed's store full.
const MEANS: Means = Means { pore: 0.16, pores: 0.2, stands: 0.0, store: 0.6, field: 0.5 };

/// The liquid table over points of a wide spread of wetness index (from a ridge to a hollow) and unequal areas: the mean
/// by area of its room is the cell's, to the rounding of a sum (it moves no liquid); no point's room is less than none
/// or more than the most; a point where more gathers has no more room, and its soil holds no less.
#[test]
fn the_table_keeps_its_mean_over_a_cell() {
    let ph = Physics::default();
    let twi: Vec<f64> = (0..64).map(|p| -2.0 + 0.25 * p as f64).collect();
    let raw: Vec<f64> = (0..64).map(|p| 1.0 + (p % 5) as f64).collect();
    let area: Vec<f64> = raw.iter().map(|a| a / raw.iter().sum::<f64>()).collect();
    let c = soil_level(&ph, &twi, &area, MEANS).expect("neither saturated nor at its most room");
    let [d, most] = MEANS.deficit(&ph);
    let room: Vec<f64> = twi.iter().map(|&t| deficit_at(&ph, MEANS, t, c)).collect();
    let mean: f64 = room.iter().zip(&area).map(|(h, a)| h * a).sum();
    assert!((mean - d).abs() < 1e-12, "mean room {mean}, the cell's {d}");
    assert!(room.iter().all(|&r| (0.0..=most).contains(&r)), "within none and the most");
    assert!(room.windows(2).all(|p| p[1] <= p[0]), "less room where more gathers");
    let held: Vec<f64> = twi.iter().map(|&t| soil_held(&ph, MEANS, t, c)).collect();
    assert!(held.windows(2).all(|p| p[1] >= p[0]) && held.iter().all(|&h| (0.0..=MEANS.pores).contains(&h)), "the soil holds more where more gathers");
}

/// A valley floor saturates before its ridge: where the most liquid gathers on the terrain's samples (the sample that
/// the most drains through) the wetness index is higher than on most of the ground; and as a ground of a ridge-to-valley
/// spread of that index fills its bed's store, the parts its table reaches the surface of are those of the highest
/// index, and their share grows with the liquid.
#[test]
fn a_valley_floor_saturates_before_its_ridge() {
    let w = worldgen::testkit::small_world();
    let s = w.terrain_samples();
    let floor = (0..s.up.len()).max_by(|&a, &b| s.up[a].total_cmp(&s.up[b])).expect("samples");
    let mut sorted = s.twi.clone();
    sorted.sort_by(f64::total_cmp);
    assert!(s.twi[floor] > sorted[sorted.len() * 9 / 10], "the floor's index {} under the ground's top tenth", s.twi[floor]);
    let ph = Physics::default();
    let twi: Vec<f64> = (0..64).map(|p| 4.0 + 0.25 * p as f64).collect();
    let bed = ph.bed_porosity * ph.bed_depth;
    let mut last = 0.0;
    for store in [0.5, 0.8, 0.95, 1.0].map(|x| x * bed) {
        let g = saturated(&ph, &twi, Means { pore: 0.08, pores: 0.2, stands: 0.0, store, field: 0.4 });
        let wet = twi.iter().filter(|&&t| t >= g.level).count();
        assert!((wet as f64 / twi.len() as f64 - g.share).abs() < 1e-12 && twi.iter().rev().take(wet).all(|&t| t >= g.level), "the highest index saturates first");
        assert!(g.share >= last, "a fuller store saturates no less");
        last = g.share;
    }
    assert!(last > 0.0 && last < 1.0, "a full store saturates part of the ground: {last}");
}
