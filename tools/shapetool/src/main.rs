use pcore::space::network::NONE;
use pcore::space::samples::Samples;
use std::io::Write;
use std::time::Instant;

fn q(v: &mut Vec<f64>, p: f64) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    v[((v.len() - 1) as f64 * p).round() as usize]
}

struct M {
    land: Vec<bool>,
    slope: Vec<f64>,
}

fn prep(s: &Samples, dx: f64) -> M {
    let (w, h) = (s.across, s.down);
    let open = |k: usize| s.pond[k] != NONE && s.ponds[s.pond[k] as usize].pass == NONE;
    let land: Vec<bool> = (0..w * h).map(|k| !open(k)).collect();
    let z = |r: i64, c: i64| s.h[(r.clamp(0, h as i64 - 1) as usize) * w + c.clamp(0, w as i64 - 1) as usize].max(0.0);
    let slope = (0..w * h)
        .map(|k| {
            let (r, c) = ((k / w) as i64, (k % w) as i64);
            let gx = (z(r, c + 1) - z(r, c - 1)) / (2.0 * dx);
            let gy = (z(r + 1, c) - z(r - 1, c)) / (2.0 * dx);
            gx.hypot(gy)
        })
        .collect();
    M { land, slope }
}

fn measure(seed: u64, size: usize) {
    let cfg = pcore::Config::default();
    let (map, terrain) = worldgen::terrain::island_ground(seed, size, size);
    let t0 = Instant::now();
    let s = terrain.samples(seed, &map, (cfg.lat_south, cfg.lat_north));
    let secs = t0.elapsed().as_secs_f64();
    let dx = map.side / 16.0;
    let a = dx * dx;
    let (w, h) = (s.across, s.down);
    let m = prep(&s, dx);
    let n = w * h;
    let landk: Vec<usize> = (0..n).filter(|&k| m.land[k]).collect();
    let mut hs: Vec<f64> = landk.iter().map(|&k| s.h[k]).collect();
    let (h10, h50, h90) = (q(&mut hs, 0.1), q(&mut hs, 0.5), q(&mut hs, 0.9));
    let hmax = hs.iter().cloned().fold(f64::MIN, f64::max);
    let bins = [0.05, 0.3, 0.6, 0.9];
    let mut sb = [0usize; 5];
    for &k in &landk {
        let b = bins.iter().position(|&x| m.slope[k] < x).unwrap_or(4);
        sb[b] += 1;
    }
    let nl = landk.len() as f64;
    // relief in 4 km windows (32 samples), windows at least half land
    let win = 32;
    let mut rel = Vec::new();
    let mut relw: Vec<(f64, f64, f64)> = Vec::new(); // (cx, cy, relief)
    for by in 0..h / win {
        for bx in 0..w / win {
            let ks: Vec<usize> = (0..win).flat_map(|y| (0..win).map(move |x| (by * win + y) * w + bx * win + x)).collect();
            let nland = ks.iter().filter(|&&k| m.land[k]).count();
            if nland * 2 < ks.len() {
                continue;
            }
            let hi = ks.iter().map(|&k| s.h[k].max(0.0)).fold(f64::MIN, f64::max);
            let lo = ks.iter().map(|&k| s.h[k].max(0.0)).fold(f64::MAX, f64::min);
            rel.push(hi - lo);
            relw.push(((bx * win + win / 2) as f64, (by * win + win / 2) as f64, hi - lo));
        }
    }
    // smaller windows (2 km) for halves, since 4 km windows are few on a 22 island
    let mut rel2: Vec<(f64, f64, f64)> = Vec::new();
    let w2 = 16;
    for by in 0..h / w2 {
        for bx in 0..w / w2 {
            let ks: Vec<usize> = (0..w2).flat_map(|y| (0..w2).map(move |x| (by * w2 + y) * w + bx * w2 + x)).collect();
            if ks.iter().filter(|&&k| m.land[k]).count() * 2 < ks.len() {
                continue;
            }
            let hi = ks.iter().map(|&k| s.h[k].max(0.0)).fold(f64::MIN, f64::max);
            let lo = ks.iter().map(|&k| s.h[k].max(0.0)).fold(f64::MAX, f64::min);
            rel2.push(((bx * w2 + w2 / 2) as f64, (by * w2 + w2 / 2) as f64, hi - lo));
        }
    }
    let _ = relw;
    let (r10, r50, r90) = { let mut r = rel.clone(); (q(&mut r, 0.1), q(&mut r, 0.5), q(&mut r, 0.9)) };
    // halves: split land through its centroid along 4 directions; the largest ratio of mean 2 km relief
    let (cx, cy) = (rel2.iter().map(|r| r.0).sum::<f64>() / rel2.len() as f64, rel2.iter().map(|r| r.1).sum::<f64>() / rel2.len() as f64);
    let mut unlike: f64 = 1.0;
    for d in [(1.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, -1.0)] {
        let (mut a1, mut n1, mut a2, mut n2) = (0.0, 0.0, 0.0, 0.0);
        for r in &rel2 {
            if (r.0 - cx) * d.0 + (r.1 - cy) * d.1 > 0.0 { a1 += r.2; n1 += 1.0 } else { a2 += r.2; n2 += 1.0 }
        }
        if n1 > 0.0 && n2 > 0.0 {
            let (m1, m2) = (a1 / n1, a2 / n2);
            unlike = unlike.max(m1.max(m2) / m1.min(m2).max(1.0));
        }
    }
    // coast
    let mut edges = 0usize;
    for k in 0..n {
        let (r, c) = (k / w, k % w);
        if c + 1 < w && m.land[k] != m.land[k + 1] { edges += 1; }
        if r + 1 < h && m.land[k] != m.land[k + w] { edges += 1; }
    }
    let area = nl * a;
    let coast = edges as f64 * dx / (2.0 * (std::f64::consts::PI * area).sqrt());
    // islands
    let mut comp = vec![u32::MAX; n];
    let mut sizes = Vec::new();
    for k in 0..n {
        if !m.land[k] || comp[k] != u32::MAX { continue; }
        let id = sizes.len() as u32;
        let mut st = vec![k];
        comp[k] = id;
        let mut cnt = 0;
        while let Some(i) = st.pop() {
            cnt += 1;
            let (r, c) = (i / w, i % w);
            let mut nb = vec![];
            if c > 0 { nb.push(i - 1) } if c + 1 < w { nb.push(i + 1) } if r > 0 { nb.push(i - w) } if r + 1 < h { nb.push(i + w) }
            for j in nb { if m.land[j] && comp[j] == u32::MAX { comp[j] = id; st.push(j); } }
        }
        sizes.push(cnt);
    }
    let islands = sizes.len();
    let islands_km = sizes.iter().filter(|&&c| c as f64 * a >= 1e6).count();
    // bodies
    let mut body_area = vec![0.0; s.ponds.len()];
    for k in 0..n { if s.pond[k] != NONE { body_area[s.pond[k] as usize] += a; } }
    let bodies: Vec<f64> = (0..s.ponds.len()).filter(|&p| s.ponds[p].pass != NONE).map(|p| body_area[p]).collect();
    let big = bodies.iter().filter(|&&x| x >= 0.25e6).count();
    // channels
    let catch = landk.iter().map(|&k| s.up[k]).fold(0.0, f64::max);
    let mut flat_len = 0.0;
    for &k in &landk {
        if s.up[k] < 1e6 || s.to[k] == NONE { continue; }
        let j = s.to[k] as usize;
        let d = if (j % w != k % w) && (j / w != k / w) { dx * 2f64.sqrt() } else { dx };
        if (s.h[k] - s.h[j].max(0.0)) / d < 0.02 && s.pond[k] == NONE { flat_len += d; }
    }
    // relief in 1 km windows (8 samples) of land whose mean slope is under 0.05
    let mut flat_rel = Vec::new();
    for by in 0..h / 8 {
        for bx in 0..w / 8 {
            let ks: Vec<usize> = (0..8).flat_map(|y| (0..8).map(move |x| (by * 8 + y) * w + bx * 8 + x)).collect();
            if !ks.iter().all(|&k| m.land[k]) || ks.iter().map(|&k| m.slope[k]).sum::<f64>() / 64.0 >= 0.05 { continue; }
            let hi = ks.iter().map(|&k| s.h[k]).fold(f64::MIN, f64::max);
            let lo = ks.iter().map(|&k| s.h[k]).fold(f64::MAX, f64::min);
            flat_rel.push(hi - lo);
        }
    }
    let (f10, f50) = { let mut r = flat_rel.clone(); (q(&mut r, 0.1), q(&mut r, 0.5)) };
    let low_bodies: Vec<f64> = (0..s.ponds.len()).filter(|&p| s.ponds[p].pass != NONE && s.ponds[p].level < 200.0).map(|p| body_area[p]).collect();
    let bare = landk.iter().filter(|&&k| s.loose[k] < 0.1 && s.pond[k] == NONE).count() as f64 / nl;
    let mut by_make = [0usize; 8];
    landk.iter().for_each(|&k| by_make[(s.make[k] as usize).min(7)] += 1);
    let patch = by_make.iter().take_while(|_| true).map(|&c| format!("{:.2}", c as f64 / nl)).collect::<Vec<_>>()[..5].join("/");
    let b = s.books;
    let drift = (b.taken - b.settled - b.shelf - b.beyond) / b.taken;
    println!(
        "{seed}\t{size}\t{h10:.0}\t{h50:.0}\t{h90:.0}\t{hmax:.0}\t{}\t{r10:.0}\t{r50:.0}\t{r90:.0}\t{unlike:.2}\t{coast:.2}\t{islands}/{islands_km}\t{:.3}\t{}/{}\t{:.1}\t{:.0}\t{:.1}\t{bare:.3}\t{patch}\t{secs:.2}\t{:.2e}\t{drift:.1e}\t{f10:.1}/{f50:.1}\t{}/{:.1}",
        sb.iter().map(|&c| format!("{:.2}", c as f64 / nl)).collect::<Vec<_>>().join("/"),
        nl / n as f64,
        bodies.len(), big,
        bodies.iter().sum::<f64>() / 1e6,
        catch / 1e6,
        flat_len / 1e3,
        b.taken,
        low_bodies.len(), low_bodies.iter().sum::<f64>() / 1e6,
    );
}

/// A picture of the samples: hypsometric colours with a hillshade, the sea blue, ponds lighter blue.
fn picture(seed: u64, size: usize, out: &str) {
    let cfg = pcore::Config::default();
    let (map, terrain) = worldgen::terrain::island_ground(seed, size, size);
    let s = terrain.samples(seed, &map, (cfg.lat_south, cfg.lat_north));
    let dx = map.side / 16.0;
    let (w, h) = (s.across, s.down);
    let m = prep(&s, dx);
    let mut f = std::fs::File::create(out).unwrap();
    write!(f, "P6\n{w} {h}\n255\n").unwrap();
    let z = |r: usize, c: usize| s.h[r.min(h - 1) * w + c.min(w - 1)].max(0.0);
    for r in 0..h {
        for c in 0..w {
            let k = r * w + c;
            let rgb: [f64; 3] = if !m.land[k] {
                let d = (-s.h[k]).max(0.0);
                let t = (d / 300.0).min(1.0);
                [0.35 - 0.25 * t, 0.55 - 0.3 * t, 0.75 - 0.25 * t]
            } else if s.pond[k] != NONE {
                [0.45, 0.65, 0.9]
            } else {
                let e = s.h[k];
                let stops: [(f64, [f64; 3]); 6] = [(0.0, [0.45, 0.62, 0.35]), (150.0, [0.62, 0.72, 0.42]), (500.0, [0.78, 0.72, 0.5]), (1000.0, [0.65, 0.52, 0.4]), (1800.0, [0.6, 0.6, 0.6]), (2800.0, [0.97, 0.97, 0.97])];
                let mut col = stops[5].1;
                for i in 0..5 {
                    if e < stops[i + 1].0 {
                        let t = ((e - stops[i].0) / (stops[i + 1].0 - stops[i].0)).clamp(0.0, 1.0);
                        col = std::array::from_fn(|j| stops[i].1[j] + (stops[i + 1].1[j] - stops[i].1[j]) * t);
                        break;
                    }
                }
                if s.loose[k] < 0.1 { col = std::array::from_fn(|j| col[j] * 0.6 + 0.35 * 0.4); }
                if std::env::var("MAKES").is_ok() {
                    const C: [[f64; 3]; 5] = [[0.30, 0.28, 0.25], [0.24, 0.23, 0.21], [0.39, 0.36, 0.32], [0.54, 0.51, 0.46], [0.46, 0.41, 0.34]];
                    col = C[(s.make[k] as usize).min(4)].map(|x| x * 1.6);
                }
                let gx = (z(r, c + 1) - z(r, c.saturating_sub(1))) / (2.0 * dx);
                let gy = (z(r + 1, c) - z(r.saturating_sub(1), c)) / (2.0 * dx);
                let shade = (0.75 + 0.9 * (-gx * 0.707 - gy * 0.707) / (1.0 + gx * gx + gy * gy).sqrt()).clamp(0.25, 1.4);
                col.map(|x| x * shade)
            };
            f.write_all(&rgb.map(|x| (x.clamp(0.0, 1.0) * 255.0) as u8)).unwrap();
        }
    }
}

/// The land share by area of a globe's worn samples, its level found as worldgen finds it.
fn globe(seed: u64, rows: usize) {
    let mut cfg = pcore::Config::default();
    (cfg.lat_south, cfg.lat_north) = (-90.0, 90.0);
    let map = worldgen::terrain::globe(rows);
    let mesh = pcore::World::mesh_of(&map.samples(), &cfg);
    let cells = mesh.cells();
    let rise: Vec<f64> = (0..cells.n()).map(|i| pcore::space::terrain::rise(seed, &map, pcore::space::terrain::Place::of(cells, i), 0.0).0).collect();
    let mut order: Vec<usize> = (0..cells.n()).collect();
    order.sort_by(|&a, &b| rise[b].total_cmp(&rise[a]));
    let (total, mut above) = (cells.area().iter().sum::<f64>(), 0.0);
    let level = order.iter().find(|&&i| { above += cells.area()[i]; above >= 0.29 * total }).map_or(0.0, |&i| rise[i]);
    let (chem, init) = worldgen::chemistry0();
    let t = pcore::space::terrain::Terrain::new(seed, level, worldgen::terrain::cuts_of(&chem, &init));
    let s = t.samples(seed, &map, (cfg.lat_south, cfg.lat_north));
    let (mut land, mut all) = (0.0, 0.0);
    for k in 0..s.h.len() {
        let lat = (90.0 - ((k / s.across) as f64 + 0.5) * 180.0 / s.down as f64).to_radians();
        let open = s.pond[k] != NONE && s.ponds[s.pond[k] as usize].pass == NONE;
        all += lat.cos();
        if !open { land += lat.cos(); }
    }
    println!("globe seed {seed} rows {rows}: land share {:.4}", land / all);
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    match a[1].as_str() {
        "measure" => {
            println!("seed\tsize\th10\th50\th90\thmax\tslope<.05/.3/.6/.9/>\tr10\tr50\tr90\tunlike\tcoast\tisl/1km2\tland\tbodies/big\tbodyA\tcatch\tflatCh\tbare\tmakes0-4\tsecs\ttaken\tdrift\tflat1km10/50\tlowBodies/A");
            for spec in &a[2..] {
                let (seed, size) = spec.split_once(':').unwrap();
                measure(seed.parse().unwrap(), size.parse().unwrap());
            }
        }
        "globe" => globe(a[2].parse().unwrap(), a[3].parse().unwrap()),
        "picture" => picture(a[2].parse().unwrap(), a[3].parse().unwrap(), &a[4]),
        _ => panic!(),
    }
}
