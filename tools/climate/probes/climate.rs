//! Probe (not committed): run an island (or a globe) with the roster's plants and write per square climate and cover
//! over the last `KEEP` years, and per row means.
//! usage: climate SEED SIZE|gROWS SOUTH NORTH YEARS OUT [STRETCH] [DRIFT_SCALE]

use pcore::cover::{Parts, NG, FROST, MOIST, LIGHT, WALLS};
use pcore::{World, AIR};
use std::io::Write;
use worldgen::life::{plants, Seeding, FIXERS, ROSTER};
use worldgen::schedule::Schedule;

const KEEP: u64 = 5;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let seed: u64 = a[1].parse().unwrap();
    let (south, north): (f64, f64) = (a[3].parse().unwrap(), a[4].parse().unwrap());
    let years: u64 = a[5].parse().unwrap();
    let out = &a[6];
    std::fs::create_dir_all(out).unwrap();
    pcore::par::adopt_thread();
    let mut w = if let Some(r) = a[2].strip_prefix('g') {
        worldgen::terrain::new_globe(seed, r.parse().unwrap(), [0; 4])
    } else {
        let size: usize = a[2].parse().unwrap();
        let mut cfg = pcore::Config { lat_south: south, lat_north: north, ..Default::default() };
        if let Some(s) = a.get(7) {
            probe_stretch(&mut cfg, s, size, (south, north));
        }
        if let Some(d) = a.get(8) {
            cfg.rain_drift_m *= d.parse::<f64>().unwrap();
        }
        let mut w = worldgen::terrain::new_island(seed, size, size, cfg);
        if a.get(9).map(String::as_str) == Some("open") {
            let t = std::time::Instant::now();
            w.planet = Some(worldgen::planet::planet(seed));
            eprintln!("planet in {:.2} s", t.elapsed().as_secs_f64());
        }
        w
    };
    eprintln!("stretch {}", stretch_of(&w));
    let plan = Schedule::new(Some(2), &plants(&ROSTER), Seeding { hand: true, rounds: 20, fixers: FIXERS });
    pcore::flow::state(&mut w, true);
    let t0 = pcore::check::totals(&w);
    let (cells, nsq) = (tape::pixels(&w), w.map.n());
    let n = w.mesh.cells().n();
    let mpy = w.cfg.months_per_year as usize;
    let mut tmon = vec![0.0; n * mpy];
    let mut snow = vec![0.0; n];
    let mut rained0 = vec![0.0; n];
    let mut umax: f64 = 0.0;
    let mut edges0 = [0.0f64; 4];
    let mut umean = 0.0;
    let mut tair_rows: Vec<f64> = Vec::new();
    for y in 1..=years {
        if y == years - KEEP + 1 {
            rained0 = w.rained.clone();
            edges0 = w.counts.edges;
        }
        for m in 0..mpy {
            if m == 0 {
                plan.place(&mut w);
            }
            pcore::step(&mut w);
            if y > years - KEEP {
                let f = tape::fields(&w);
                for i in 0..n {
                    tmon[i * mpy + m] += w.t_ground_last(i) / KEEP as f64;
                    if f.ice[i] > 0.05 {
                        snow[i] += 1.0 / KEEP as f64;
                    }
                }
                let (u, um) = speeds(&w);
                umax = umax.max(u);
                umean += um / (KEEP as f64 * mpy as f64);
            }
        }
        if let Some(e) = pcore::check::broken(&w).or_else(|| pcore::check::drifted(&w, &t0)) {
            eprintln!("year {y}: broken: {e}");
            std::process::exit(1);
        }
        if y % 5 == 0 {
            eprintln!("year {y}");
        }
    }
    // Air temperature by air row.
    let air = w.mesh.level(AIR);
    let _ = &mut tair_rows;
    let p = Parts::of(&w.chem).unwrap();
    let mm = p.mm(&w.chem);
    let vl = w.chem.molar_volume(p.liquid, pcore::chem::Phase::Liquid);
    let f = tape::fields(&w);
    let mut csv = std::fs::File::create(format!("{out}/squares.csv")).unwrap();
    writeln!(csv, "x,y,lat,cell,land,z,t_mean,t_summer,t_winter,rain_m,snow_months,cover_kg,lead,lead_share,frost,moist,light,walls,kinds").unwrap();
    for sq in 0..nsq {
        let (x, yy) = w.map.xy(sq);
        let i = cells[sq];
        let lat = north + (south - north) * (yy as f64 + 0.5) / w.map.h as f64;
        let lat = if w.map.wrap { 90.0 - 180.0 * (yy as f64 + 0.5) / w.map.h as f64 } else { lat };
        let land = w.is_land(i) && f.depth[i] < tape::STANDING_M;
        let mon = &tmon[i * mpy..(i + 1) * mpy];
        let tm = mon.iter().sum::<f64>() / mpy as f64;
        let ts = mon.iter().cloned().fold(f64::MIN, f64::max);
        let tw = mon.iter().cloned().fold(f64::MAX, f64::min);
        let rain = (w.rained[i] - rained0[i]) * vl / KEEP as f64;
        let c = w.life.cover(i);
        let area = w.mesh.cells().area()[i];
        let mut by = std::collections::BTreeMap::<u32, f64>::new();
        let (mut tot, mut built, mut g) = (0.0f64, 0.0f64, [0.0f64; NG]);
        for s in c.held() {
            let kg = c.kg(&mm, s);
            *by.entry(c.root[s]).or_default() += kg;
            tot += kg;
            let b = c.built(&mm, s);
            built += b;
            g.iter_mut().zip(c.genes[s]).for_each(|(a, v)| *a += b * v as f64);
        }
        let (lead, ls) = by.iter().max_by(|a, b| a.1.total_cmp(b.1)).map(|(&r, &k)| (r as i64, k / tot.max(1e-12))).unwrap_or((-1, 0.0));
        let gm = |k: usize| if built > 0.0 { g[k] / built } else { f64::NAN };
        let kinds: Vec<String> = by.iter().map(|(r, k)| format!("{r}:{:.3}", k / area)).collect();
        writeln!(csv, "{x},{yy},{lat:.3},{i},{},{:.1},{tm:.2},{ts:.2},{tw:.2},{rain:.4},{:.2},{:.4},{lead},{ls:.3},{:.2},{:.3},{:.3},{:.3},{}", land as u8, f.z[i], snow[i], tot / area, gm(FROST), gm(MOIST), gm(LIGHT), gm(WALLS), kinds.join(" ")).unwrap();
    }
    let mut air_csv = std::fs::File::create(format!("{out}/air.csv")).unwrap();
    writeln!(air_csv, "j,lat,t_air").unwrap();
    for j in 0..air.n() {
        writeln!(air_csv, "{j},{:.3},{:.2}", air.lat()[j], w.t_air(j)).unwrap();
    }
    let drift = pcore::check::drifted(&w, &t0);
    // Heat into the map at each open edge, W per m of the edge it stands for (the stretch times the map's width).
    let year_s = w.cfg.calendar().year as f64;
    let per_m: Vec<f64> = (0..4).map(|e| (w.counts.edges[e] - edges0[e]) / (KEEP as f64 * year_s) * w.cfg.stretch / (w.map.w as f64 * w.map.side)).collect();
    println!("edges W/m north {:.3e} south {:.3e} east {:.3e} west {:.3e}", per_m[0], per_m[1], per_m[2], per_m[3]);
    let (de, dq) = pcore::check::drift(&w, &t0);
    println!("drift elements {de:.2e} energy {dq:.2e}");
    println!("umax {umax:.3} umean {umean:.4} drift {drift:?}");
    let mut sum = std::fs::File::create(format!("{out}/summary.txt")).unwrap();
    writeln!(sum, "umax {umax:.3} umean {umean:.4} stretch {} drift {drift:?} edgesW/m {:.3e} {:.3e} {:.3e} {:.3e} drift {de:.2e} {dq:.2e}", stretch_of(&w), per_m[0], per_m[1], per_m[2], per_m[3]).unwrap();
}

/// The overturning's speed on each link between bordering air nodes, m/s: `wind` x the warmth gradient over the link's
/// length as the air reads it (times the stretch); max and mean. Warmth: the node's temperature raised by its drop with
/// height times its cells' mean surface height (0 under liquid). None (0) where the map is one block.
fn speeds(w: &World) -> (f64, f64) {
    let air = w.mesh.level(AIR);
    let spacing = (air.area().iter().sum::<f64>() / air.n() as f64).sqrt() * w.cfg.stretch;
    let side = ((w.cfg.mix * 86400.0f64).sqrt() / spacing).max(1.0) as usize;
    if side > 1 {
        return (0.0, 0.0);
    }
    let cells = w.mesh.cells();
    let f = tape::fields(w);
    let (mut zs, mut gs, mut ar) = (vec![0.0f64; air.n()], vec![0.0f64; air.n()], vec![0.0f64; air.n()]);
    for i in 0..cells.n() {
        let j = cells.parent()[i] as usize;
        let a = cells.area()[i];
        let z = f.z[i] + f.depth[i];
        zs[j] += a * z;
        gs[j] += a * w.air_drop(i).unwrap_or(0.0065);
        ar[j] += a;
    }
    let warm: Vec<f64> = (0..air.n()).map(|j| w.t_air(j) + gs[j] / ar[j] * zs[j] / ar[j]).collect();
    let (mut mx, mut sum, mut k) = (0.0f64, 0.0f64, 0.0f64);
    for l in 0..air.a().len() {
        if air.width()[l] <= 0.0 {
            continue;
        }
        let (a, b) = (air.a()[l] as usize, air.b()[l] as usize);
        let u = w.cfg.wind * (warm[a] - warm[b]).abs() / (air.dist()[l] * w.cfg.stretch);
        mx = mx.max(u);
        sum += u;
        k += 1.0;
    }
    (mx, sum / k.max(1.0))
}

fn stretch_of(w: &World) -> f64 {
    w.cfg.stretch
}

fn probe_stretch(cfg: &mut pcore::Config, s: &str, size: usize, band: (f64, f64)) {
    cfg.stretch = if s == "band" { worldgen::terrain::banded(size, band).stretch } else { s.parse().unwrap() };
}
