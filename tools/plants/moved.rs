// Scratch (not committed): the share of the woody marks the root rule moves (web/src/roots.ts), after `years`.
use pcore::space::terrain::SAMPLE_DEPTH;
use worldgen::life::{Seeding, FIXERS};
use worldgen::schedule::Schedule;
fn ease(a: f64, b: f64, x: f64) -> f64 { let t = ((x - a) / (b - a)).clamp(0.0, 1.0); t * t * (3.0 - 2.0 * t) }
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (seed, years): (u64, u64) = (a[1].parse().unwrap(), a[2].parse().unwrap());
    let mut world = worldgen::new_world(seed, 22, 22);
    let mut plan = Schedule::new(Some(2), &worldgen::life::plants(&worldgen::life::ROSTER), Seeding { hand: true, rounds: 20, fixers: FIXERS });
    let mpy = world.cfg.months_per_year as u64;
    while world.month() < years * mpy + 6 {
        if world.month() % mpy == 0 { plan.place(&mut world); }
        pcore::step(&mut world);
        world.drain_events();
    }
    let map = &world.map;
    let s = world.terrain_samples();
    let per = (1u64 << (SAMPLE_DEPTH - map.square_depth())) as f64;
    let tree = world.mesh.tree();
    let n = world.mesh.cells().n();
    let mut inside: Vec<Vec<f64>> = vec![Vec::new(); n];
    for k in 0..s.h.len() {
        let i = tree.cell_at(map.point_at(((k % s.across) as f64 + 0.5) / per, ((k / s.across) as f64 + 0.5) / per));
        inside[i].push(s.loose[k]);
    }
    let (mut moved, mut all, mut bare_land, mut land) = (0.0, 0.0, 0usize, 0usize);
    for i in 0..n {
        let r = tape::stands::rows(&world, i);
        for k in 0..tape::KINDS {
            let row = &r[k * tape::KIND_ROW..(k + 1) * tape::KIND_ROW];
            if row[0] < 0.0 || row[1] <= 0.0 { continue; }
            let (height, wood, per_m2) = (row[4], row[8], row[3]);
            if height < 3.0 && wood < 0.6 { continue; } // herbs: the sward
            let need = (0.1 * height).max(0.02);
            let ls = &inside[i];
            if ls.is_empty() { continue; }
            let held: Vec<f64> = ls.iter().map(|&l| ease(0.25 * need, need, l)).collect();
            let mean = (held.iter().sum::<f64>() / held.len() as f64).max(0.1);
            let tv = held.iter().map(|h| (h / mean - 1.0).abs()).sum::<f64>() / held.len() as f64 / 2.0;
            moved += per_m2 * tv;
            all += per_m2;
        }
        if !inside[i].is_empty() && world.surface_z(i) > 0.0 {
            land += inside[i].len();
            bare_land += inside[i].iter().filter(|&&l| l < 0.1).count();
        }
    }
    println!("seed {seed}: woody marks moved {:.1}%; land samples bare (< 0.1 m) {:.1}%", 100.0 * moved / all, 100.0 * bare_land as f64 / land as f64);
}
