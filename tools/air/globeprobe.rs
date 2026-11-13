//! PROBE (never committed): the lifeless globe of the climate test (32 rows) for YEARS years; over its last year, each
//! month: per cell its latitude, land or not, ground temperature, ice depth and the surface's budget (polar_toggle.py);
//! per air node its latitude, light to space and vapour column. globeprobe YEARS OUT_PREFIX
use pcore::store::Class;
use std::io::Write;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (years, out): (u64, &str) = (a[1].parse().unwrap(), &a[2]);
    let mut w = worldgen::terrain::new_globe(1, 32, [0; 4]);
    worldgen::terrain::scale_absorbers(&mut w, 1.0);
    let (n, na) = (w.mesh.cells().n(), w.mesh.level(pcore::AIR).n());
    let mut fc = std::fs::File::create(format!("{out}_cells.csv")).unwrap();
    let mut fa = std::fs::File::create(format!("{out}_air.csv")).unwrap();
    writeln!(fc, "month,i,lat,land,t,ice_m,adt,sw,up,down,down_clear,cloud,sens,lat_heat,to_space,air_sun,ts").unwrap();
    writeln!(fa, "month,j,lat,area,space_J,vap_kg_m2,ratio,t_low,hk,gam,za,zair,tau,cloud").unwrap();
    let lat = w.mesh.cells().lat().to_vec();
    let carea = w.mesh.cells().area().to_vec();
    let alat: Vec<f64> = (0..na).map(|j| { let ch = w.mesh.level(pcore::AIR).children(j); let s: f64 = ch.iter().map(|&i| carea[i as usize]).sum(); ch.iter().map(|&i| lat[i as usize] * carea[i as usize]).sum::<f64>() / s }).collect();
    let aarea = w.mesh.level(pcore::AIR).area().to_vec();
    // A cold start for a look at two states (never a commit): the sun times COLD_SUN for COLD years, then as it is.
    let cold: u64 = std::env::var("COLD").map_or(0, |v| v.parse().unwrap());
    let sun0 = w.cfg.sun;
    if cold > 0 { w.cfg.sun = sun0 * std::env::var("COLD_SUN").map_or(0.85, |v| v.parse().unwrap()); }
    for y in 1..=years {
        if y == cold + 1 { w.cfg.sun = sun0; }
        for m in 0..w.cfg.months_per_year {
            pcore::flow::PROBE_CELL.lock().unwrap().clear();
            pcore::flow::PROBE_SPACE.lock().unwrap().clear();
            pcore::flow::PROBE_NODE.lock().unwrap().clear();
            pcore::step(&mut w);
            let _ = w.drain_events();
            if y < years {
                continue;
            }
            let pc = pcore::flow::PROBE_CELL.lock().unwrap().clone();
            let ps = pcore::flow::PROBE_SPACE.lock().unwrap().clone();
            let pn = pcore::flow::PROBE_NODE.lock().unwrap().clone();
            for i in 0..n {
                let ice = { let (tt, pl) = w.thaw_node_last(i); w.g(0, i) * (1.0 - w.molten(0, i, tt, pl)) * w.chem.molar_volume(0, pcore::chem::Phase::Solid) / carea[i] };
                let v = pc.get(i).copied().unwrap_or([0.0; 11]);
                let d = v[10].max(1e-300);
                write!(fc, "{m},{i},{:.3},{},{:.3},{:.4},{:.4e}", lat[i], w.is_land(i) as u8, w.t_ground_last(i), ice, v[10]).unwrap();
                for k in 0..10 { write!(fc, ",{:.4}", v[k] / d).unwrap(); }
                writeln!(fc).unwrap();
            }
            for j in 0..na {
                let vap = w.store.amounts(Class::Air)[j] * w.chem.subs[0].molar_mass / aarea[j];
                let v = pn.get(j).copied().unwrap_or([0.0; 9]);
                let d = v[8].max(1e-300);
                write!(fa, "{m},{j},{:.3},{:.4e},{:.4e},{:.3}", alat[j], aarea[j], ps.get(j).copied().unwrap_or(0.0), vap).unwrap();
                for k in 0..8 { write!(fa, ",{:.5}", v[k] / d).unwrap(); }
                writeln!(fa).unwrap();
            }
        }
        let polar: Vec<usize> = (0..n).filter(|&i| lat[i].abs() > 67.5).collect();
        let pice = polar.iter().map(|&i| { let (tt, pl) = w.thaw_node_last(i); w.g(0, i) * (1.0 - w.molten(0, i, tt, pl)) * w.chem.molar_volume(0, pcore::chem::Phase::Solid) / carea[i] }).fold(0.0, f64::max);
        let pt = polar.iter().map(|&i| w.t_ground_last(i)).sum::<f64>() / polar.len() as f64;
        eprintln!("year {y}: sun x{:.2}, polar ground {pt:.1} K (at the year's end), polar ice at most {pice:.2} m", w.cfg.sun / sun0);
    }
}
