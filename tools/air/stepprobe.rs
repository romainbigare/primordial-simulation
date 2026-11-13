//! PROBE (never committed): the long step's world (32 x 32, the two kinds) with the air's steps DAYS long; at the end of
//! each year the land's evaporation and rain (yearly moving averages, mol/m²/s, by area), and what runs off the land.
//! stepprobe SEED YEARS DAYS
use worldgen::life::Seeding;
use worldgen::schedule::Schedule;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (seed, years, days): (u64, u64, u32) = (a[1].parse().unwrap(), a[2].parse().unwrap(), a[3].parse().unwrap());
    let mut w = worldgen::new_world(seed, 32, 32);
    worldgen::terrain::scale_absorbers(&mut w, 1.0);
    let plan = Schedule::new(Some(2), &worldgen::life::TWO, Seeding::default());
    let n = w.mesh.cells().n();
    let area = w.mesh.cells().area().to_vec();
    for y in 1..=years {
        plan.place(&mut w);
        for _ in 0..w.cfg.months_per_year {
            pcore::step_with_air(&mut w, days);
            let _ = w.drain_events();
            let _ = w.take_counters();
        }
        let land: Vec<usize> = (0..n).filter(|&i| w.is_land(i)).collect();
        let la: f64 = land.iter().map(|&i| area[i]).sum();
        let ev = land.iter().map(|&i| w.evap[i] as f64 * area[i]).sum::<f64>() / la;
        let rn = land.iter().map(|&i| w.rain[i] as f64 * area[i]).sum::<f64>() / la;
        let off = land.iter().map(|&i| w.off_land[i] as f64).sum::<f64>() / la;
        println!("{seed} {days} {y} evap {ev:.6e} rain {rn:.6e} off {off:.6e}");
    }
}
