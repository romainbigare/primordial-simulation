//! PROBE (never committed): why the long step's river count moves. Per seed, both air steps: the river count each year
//! from 11, the mean rain, evaporation and runoff on land over those years, the cells that are rivers in one run and
//! not in the other each year (how many, and how often), and at the end each such cell with its flux in both runs.
mod common;

use tape::{fields, is_river, river_count, RIVER_FLOOR, STANDING_M};

struct Out {
    count: Vec<f64>,
    sets: Vec<Vec<bool>>,
    climate: [f64; 3],
    w: pcore::World,
}

#[test]
#[ignore = "probe"]
fn rivers_probe() {
    let runs: Vec<Vec<Out>> = std::thread::scope(|s| {
        let h: Vec<Vec<_>> = [1u64, 2, 3]
            .iter()
            .map(|&seed| {
                [pcore::AIR_STEP_DAYS, 1]
                    .iter()
                    .map(|&d| {
                        s.spawn(move || {
                            let (mut count, mut sets, mut climate) = (vec![], vec![], [0.0; 3]);
                            let w = worldgen::new_world(seed, 32, 32);
                            let (w, _) = common::run_stepped(w, 20, false, |_| {}, |w| pcore::step_with_air(w, d), |y, w, rec| {
                                if y >= 11 {
                                    count.push(rec.last("rivers"));
                                    let f = fields(w);
                                    let n = w.mesh.cells().n();
                                    sets.push((0..n).map(|i| w.is_land(i) && f.depth[i] <= STANDING_M && is_river(w, i)).collect());
                                    debug_assert_eq!(river_count(w, &f) as f64, rec.last("rivers"));
                                    for (k, c) in ["rain_land", "evap_land", "runoff_sea"].iter().enumerate() {
                                        climate[k] += rec.last(c) / 10.0;
                                    }
                                }
                            });
                            Out { count, sets, climate, w }
                        })
                    })
                    .collect()
            })
            .collect();
        h.into_iter().map(|v| v.into_iter().map(|r| r.join().unwrap()).collect()).collect()
    });
    for (si, seed) in runs.iter().enumerate() {
        let (long, short) = (&seed[0], &seed[1]);
        let m = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        println!("seed {}: rivers long {:?} mean {:.2}; 1-day {:?} mean {:.2}; off {:.2}", si + 1, long.count, m(&long.count), short.count, m(&short.count), m(&long.count) - m(&short.count));
        println!("  rain_land {:.6e} / {:.6e} ({:+.3}%), evap_land {:.6e} / {:.6e} ({:+.3}%), runoff_sea {:.6e} / {:.6e} ({:+.3}%)",
            long.climate[0], short.climate[0], 100.0 * (long.climate[0] / short.climate[0] - 1.0),
            long.climate[1], short.climate[1], 100.0 * (long.climate[1] / short.climate[1] - 1.0),
            long.climate[2], short.climate[2], 100.0 * (long.climate[2] / short.climate[2] - 1.0));
        let n = long.w.mesh.cells().n();
        let mut only = vec![[0u32; 2]; n];
        for (a, b) in long.sets.iter().zip(&short.sets) {
            for i in 0..n {
                if a[i] && !b[i] { only[i][0] += 1; }
                if b[i] && !a[i] { only[i][1] += 1; }
            }
        }
        let diff: Vec<usize> = (0..n).filter(|&i| only[i] != [0, 0]).collect();
        println!("  cells that differ in some year: {} (long only cell-years {}, 1-day only {})", diff.len(), diff.iter().map(|&i| only[i][0]).sum::<u32>(), diff.iter().map(|&i| only[i][1]).sum::<u32>());
        for &i in &diff {
            let (fa, fb) = (long.w.liquid_flux[i] as f64, short.w.liquid_flux[i] as f64);
            println!("  cell {i}: years river only long {}, only 1-day {}; at year 20 flux {fa:.0} / {fb:.0} mol/s ({:+.1}%, floor {RIVER_FLOOR}); catchment {:.1} / {:.1}; land {} / {}",
                only[i][0], only[i][1], 100.0 * (fa / fb - 1.0), long.w.catchment(i), short.w.catchment(i), long.w.is_land(i), short.w.is_land(i));
        }
        let land: Vec<usize> = (0..n).filter(|&i| long.w.is_land(i) && short.w.is_land(i)).collect();
        let near = land.iter().filter(|&&i| { let f = long.w.liquid_flux[i] as f64; (f / RIVER_FLOOR - 1.0).abs() < 0.05 }).count();
        let ratio: Vec<f64> = land.iter().filter(|&&i| short.w.liquid_flux[i] as f64 > 0.5 * RIVER_FLOOR).map(|&i| long.w.liquid_flux[i] as f64 / short.w.liquid_flux[i] as f64).collect();
        let mr = ratio.iter().sum::<f64>() / ratio.len().max(1) as f64;
        let sd = (ratio.iter().map(|r| (r - mr).powi(2)).sum::<f64>() / ratio.len().max(1) as f64).sqrt();
        println!("  land cells within 5% of the floor at year 20 (long): {near}; flux long/1-day over {} cells above half the floor: mean {mr:.4}, spread {sd:.4}", ratio.len());
    }
}
