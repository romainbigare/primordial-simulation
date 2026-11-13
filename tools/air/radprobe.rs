//! PROBE (never committed): the radiation budget of a world over its last year, per m² of the map.
//! radprobe globe YEARS | wide SEED SIZE YEARS
use pcore::store::Class;
use pcore::World;

fn budget(w: &World, label: &str) {
    let r = *pcore::flow::PROBE_R.lock().unwrap();
    let at = r[7];
    let wm = |x: f64| x / at;
    let n = w.mesh.cells().n();
    let area = w.mesh.cells().area();
    let all: f64 = area.iter().sum();
    let t: f64 = (0..n).map(|i| w.t_ground_last(i) * area[i]).sum::<f64>() / all;
    let land: Vec<usize> = (0..n).filter(|&i| w.is_land(i)).collect();
    let la: f64 = land.iter().map(|&i| area[i]).sum();
    let tl: f64 = land.iter().map(|&i| w.t_ground_last(i) * area[i]).sum::<f64>() / la.max(1.0);
    let na = w.mesh.level(pcore::AIR).n();
    let vap: f64 = w.store.amounts(Class::Air)[0..na].iter().sum::<f64>() * w.chem.subs[0].molar_mass / all;
    println!("{label}: per m2 of the map, W/m2 (mean over the year)");
    println!("  sun at the top            {:8.1}", wm(r[0]));
    println!("  reflected by the air      {:8.1}  (to space from air and surface together {:.1})", wm(r[10]), wm(r[11]));
    println!("  reflected by clouds       {:8.1}  ({:.1}%)", wm(r[1]), 100.0 * r[1] / r[0]);
    println!("  reflected by the surface  {:8.1}  (gross; {:.1}% of what reached it; {:.1} of it escapes)", wm(r[8]), 100.0 * r[8] / (r[2] + r[8]), wm(r[11] - r[10]));
    println!("  absorbed in the air       {:8.1}", wm(r[9]));
    println!("  absorbed at the surface   {:8.1}", wm(r[2]));
    println!("  heat light up (surface)   {:8.1}", wm(r[4]));
    println!("  back from the air         {:8.1}", wm(r[5]));
    println!("  to space from the surface {:8.1}", wm(r[3]));
    println!("  to space from the air     {:8.1}", wm(r[6]));
    println!("  to space in all           {:8.1}", wm(r[3] + r[6]));
    println!("  clear sky: to space {:.1} (surface {:.1}, air {:.1}); the window, surface light to space through it {:.1}", wm(r[12] + r[14]), wm(r[12]), wm(r[14]), wm(r[13]));
    println!("  closure: in {:.3} = clouds + air + surface + space {:.3}", wm(r[0]), wm(r[1] + r[9] + r[2] + r[11]));
    let frozen = |i: usize| { let (tt, pl) = w.thaw_node_last(i); w.g(0, i) * (1.0 - w.molten(0, i, tt, pl)) * w.chem.molar_volume(0, pcore::chem::Phase::Solid) / area[i] >= 0.01 };
    let ice: f64 = (0..n).filter(|&i| frozen(i)).map(|i| area[i]).sum::<f64>() / all;
    let land_ice: f64 = land.iter().filter(|&&i| frozen(i)).map(|&i| area[i]).sum::<f64>() / la.max(1.0);
    println!("  ice (frozen cover over 1 cm) on {:.1}% of the map, {:.1}% of the land, at the year's end", 100.0 * ice, 100.0 * land_ice);
    println!("  ground {t:.2} K, land {tl:.2} K, vapour column {vap:.2} kg/m2");
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mut w = match a[1].as_str() {
        "globe" => {
            let mut w = worldgen::terrain::new_globe(1, 32, [0; 4]);
            worldgen::terrain::scale_absorbers(&mut w, 1.0);
            w
        }
        _ => {
            let seed: u64 = a[2].parse().unwrap();
            let size: usize = a[3].parse().unwrap();
            let mut w = worldgen::terrain::new_island(seed, size, size, worldgen::terrain::banded((20.0, 65.0)));
            worldgen::planet::stand(&mut w);
            w
        }
    };
    let years: u64 = a[if a[1] == "globe" { 2 } else { 4 }].parse().unwrap();
    let plan = (a[1] != "globe").then(|| worldgen::schedule::Schedule::new(Some(2), &worldgen::life::plants(&worldgen::life::ROSTER), worldgen::life::Seeding::default()));
    pcore::flow::state(&mut w, true);
    let mpy = w.cfg.months_per_year;
    for y in 0..years {
        if let Some(p) = &plan {
            p.place(&mut w);
        }
        if y + 1 == years {
            *pcore::flow::PROBE_R.lock().unwrap() = [0.0; 16];
        }
        for _ in 0..mpy {
            pcore::step(&mut w);
            w.drain_events();
        }
    }
    budget(&w, &format!("{} year {years}", a[1]));
}
