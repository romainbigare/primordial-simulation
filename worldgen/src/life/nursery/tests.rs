//! The garden's law (`Garden`): its liquid is a store whose year closes, so a stand lives on what the year's rain
//! leaves its ground and on nothing more.

use super::garden::Garden;
use super::rounds::run;
use super::year::{Month, Year};
use super::{known, wash, START_KG};
use pcore::cover::Parts;

/// Years a stand grows in the test, as long as a race's round.
const YEARS: u32 = super::RACE_YEARS;

/// A small island after its soil, its parts, and the year kept of its open land cell of the median rain.
fn median() -> (pcore::World, Parts, usize, Vec<Month>) {
    let mut w = crate::new_world(1, 22, 22);
    pcore::flow::state(&mut w, true);
    crate::life::soil(&mut w, &crate::chemistry0().1.soil);
    let p = Parts::of(&w.chem).expect("a chemistry with cover");
    let land: Vec<usize> = (0..w.mesh.cells().n()).filter(|&i| super::super::open_land(&w, i)).collect();
    let year = Year::of(&w, &p, &land, w.cfg.months_per_year);
    let rain = |m: &[Month]| m.iter().map(|x| x.rain).sum::<f64>();
    let mut order: Vec<usize> = (0..land.len()).collect();
    order.sort_by(|&a, &b| rain(&year.months[a]).total_cmp(&rain(&year.months[b])).then(a.cmp(&b)));
    let k = order[order.len() / 2];
    (w, p, land[k], year.months[k].clone())
}

/// A bare garden's ground ends its year with the liquid it began it with; the starting herb grown on the year kept holds
/// a stand, but on the same year with no rain its ground starts dry (the bare year closes at nothing) and the herb
/// holds none.
#[test]
fn a_garden_lives_on_the_rain_its_year_leaves() {
    let (w, p, cell, months) = median();
    let washes = wash(&w.chem, &months[0].around.ground, &months);
    let mut bare = Garden::bare(&w.chem, cell, &months);
    let start = bare.liquid;
    run(&mut bare, &w, &p, (&months, &washes), 1);
    println!("bare ground: {start:.6e} mol at the year's start, {:.6e} at its end", bare.liquid);
    assert!(start > 0.0 && (bare.liquid - start).abs() <= 1e-9 * start, "the bare year did not close: {start} then {}", bare.liquid);
    let herb = known()[0];
    let grown = |months: &[Month]| {
        let mut g = Garden::bare(&w.chem, cell, months);
        g.put(&w, &p, (herb.genes, herb.root), START_KG, months[0].around.t);
        run(&mut g, &w, &p, (months, &washes), YEARS);
        g.stand(&w, &p).iter().map(|s| s.2).sum::<f64>()
    };
    let dry: Vec<Month> = months.iter().map(|m| Month { rain: 0.0, ..m.clone() }).collect();
    let (wet_kg, dry_kg) = (grown(&months), grown(&dry));
    println!("the herb after {YEARS} years: {wet_kg:.3} kg/m2 with the year's rain, {dry_kg:.3e} with none");
    assert!(wet_kg > START_KG, "the herb did not hold a stand on the year kept: {wet_kg}");
    assert!(dry_kg < 1e-3 * wet_kg, "a stand was kept on liquid the year does not give: {dry_kg}");
}
