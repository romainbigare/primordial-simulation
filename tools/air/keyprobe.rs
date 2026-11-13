//! PROBE (never committed): where the key island's cover stands before the eaters come. keyprobe SEED YEARS OUT.csv
//! Runs `ew run --seed SEED --size 64` (the two kinds) for YEARS years; over its last 3 years, per land cell: the light
//! falling on a m² of capture area (W/m², `World::light`), the ground's wetness (`Physics::wetness`, the mean of the
//! months), the coldest and the mean month's ground temperature, rain and evaporation (mol/m²/s), and at the end the
//! cover kg/m², its capture area m²/m², its height of ground (m) and the depth of standing liquid.
use pcore::World;
use std::io::Write;
use worldgen::life::Seeding;
use worldgen::schedule::Schedule;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (seed, years, out): (u64, u64, &str) = (a[1].parse().unwrap(), a[2].parse().unwrap(), &a[3]);
    let mut w: World = worldgen::new_world(seed, 64, 64);
    worldgen::terrain::scale_absorbers(&mut w, 1.0);
    let plan = Schedule::new(Some(2), &worldgen::life::TWO, Seeding::default());
    let n = w.mesh.cells().n();
    let months = w.cfg.months_per_year;
    let (mut wet, mut tmin, mut tmean, mut light0) = (vec![0.0; n], vec![f64::MAX; n], vec![0.0; n], vec![0.0; n]);
    let mut seconds = 0.0;
    for y in 1..=years {
        plan.place(&mut w);
        if y == years - 2 {
            light0 = w.light.clone();
        }
        for _ in 0..months {
            pcore::step(&mut w);
            let _ = w.drain_events();
            let _ = w.take_counters();
            if y > years - 3 {
                seconds += w.cfg.calendar().month as f64;
                let m = pcore::flow::means(&w, &(0..n).collect::<Vec<usize>>()).unwrap();
                for i in 0..n {
                    wet[i] += w.cfg.phys.wetness(m[i].pore + m[i].stands) / (3 * months) as f64;
                    let t = w.t_ground_last(i);
                    tmin[i] = f64::min(tmin[i], t);
                    tmean[i] += t / (3 * months) as f64;
                }
            }
        }
        eprintln!("year {y}");
    }
    let f = tape::fields(&w);
    let kg = pcore::cover::per_m2(&w, pcore::cover::Cover::kg);
    let leaf = pcore::cover::per_m2(&w, pcore::cover::Cover::leaf);
    let mut o = std::fs::File::create(out).unwrap();
    writeln!(o, "i,land,z,depth,light,wet,tmin,tmean,rain,evap,kg,leaf").unwrap();
    for i in 0..n {
        let light = (w.light[i] - light0[i]) / seconds;
        writeln!(o, "{i},{},{:.2},{:.3},{light:.3},{:.4},{:.2},{:.2},{:.4e},{:.4e},{:.4},{:.4}", w.is_land(i) as u8, f.z[i], f.depth[i], wet[i], tmin[i], tmean[i], w.rain[i], w.evap[i], kg[i], leaf[i]).unwrap();
    }
}
