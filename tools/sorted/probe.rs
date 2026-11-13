// Probe of the start (not committed): probe SEED SIZE OUT YEARS...  the page's world (Wide 20..65, planet air),
// plants only; per cell CSV at each year asked and a row table by latitude.
use pcore::cover::{self, Parts, FROST, LEAF, MOIST, NG, SLOTS, WALL};
use pcore::World;
use std::collections::HashMap;
use std::io::Write;

fn plan() -> worldgen::schedule::Schedule {
    worldgen::schedule::Schedule::new(Some(2), &[], worldgen::life::Seeding { hand: true, rounds: 20 }) // PLAN
}

struct Slot { plants: f64, leaf: f64, rooted: f64, root: u32, genes: [f32; NG], kg: f64, wood: f64, h: f64, net: f64, built: f64 }

fn slots(w: &World, p: &Parts, i: usize) -> Vec<Slot> {
    let mm = p.mm(&w.chem);
    let c = w.life.cover(i);
    let rho = w.chem.subs[p.sub[WALL]].density[0];
    (0..SLOTS).filter(|&x| c.root[x] != cover::NO_ROOT && c.kg(&mm, x) > 0.0).map(|x| Slot { plants: c.plants[x], leaf: c.leaf(&mm, x), rooted: (c.rooted[x] as f64).max(1e-3), root: c.root[x], genes: c.genes[x], kg: c.kg(&mm, x), wood: c.walls(&mm, x) / c.kg(&mm, x).max(1e-30), h: c.height(&w.cfg.phys, &mm, rho, x), net: c.net[x] as f64, built: c.built(&mm, x) }).collect()
}

/// style.ts green(): hue only.
fn hue(g: &[f32; NG]) -> f64 {
    let h = 140.0 - 55.0 * (g[LEAF] as f64 - 0.4) - 4.0 * (g[FROST] as f64 - 268.0) + 25.0 * (g[MOIST] as f64 + 0.3);
    h.clamp(105.0, 150.0)
}

fn main() {
    pcore::par::adopt_thread();
    let a: Vec<String> = std::env::args().collect();
    let (seed, size, out): (u64, usize, &str) = (a[1].parse().unwrap(), a[2].parse().unwrap(), &a[3]);
    let years: Vec<u64> = a[4..].iter().map(|s| s.parse().unwrap()).collect();
    std::fs::create_dir_all(out).unwrap();
    let mut w = worldgen::terrain::new_island(seed, size, size, worldgen::terrain::banded((20.0, 65.0)));
    w.planet = Some(worldgen::planet::planet());
    let plan = plan();
    let p = Parts::of(&w.chem).unwrap();
    let n = w.mesh.cells().n();
    let area = w.mesh.cells().area().to_vec();
    let mut tmin = vec![f64::MAX; n];
    let mut log = std::fs::File::create(format!("{out}/rows.txt")).unwrap();
    let last = *years.iter().max().unwrap();
    for y in 1..=last {
        tmin.iter_mut().for_each(|t| *t = f64::MAX);
        for m in 0..w.cfg.months_per_year {
            if m == 0 {
                let t = std::time::Instant::now();
                for l in plan.place(&mut w) { println!("{l}"); writeln!(log, "{l}").unwrap(); }
                if t.elapsed().as_secs_f64() > 0.05 { let l = format!("placing took {:.2} s", t.elapsed().as_secs_f64()); println!("{l}"); writeln!(log, "{l}").unwrap(); }
            }
            pcore::step(&mut w);
            w.drain_events();
            for i in 0..n { if w.is_land(i) { tmin[i] = tmin[i].min(w.t_ground_last(i)); } }
        }
        if !years.contains(&y) { continue; }
        let mut csv = std::fs::File::create(format!("{out}/cells_{y:03}.csv")).unwrap();
        writeln!(csv, "cell,x,y,lat,tmin,kgm2,root,leadkg,nslots,frost,hue,herbkg,npp,herbleaf,herbcov,treeleaf,wet,herbact,treeact,herbh,herbn").unwrap();
        let mut rows: HashMap<u64, Vec<(f64, f64, Vec<Slot>)>> = HashMap::new();
        let mut island: HashMap<u32, f64> = HashMap::new();
        let mut gsum: HashMap<u32, [f64; NG + 2]> = HashMap::new();
        for i in 0..n {
            if !w.is_land(i) { continue; }
            let s = slots(&w, &p, i);
            let c = w.map.leaves()[i].corner();
            let kg: f64 = s.iter().map(|x| x.kg).sum::<f64>() / area[i];
            let lead = s.iter().max_by(|a, b| a.kg.total_cmp(&b.kg));
            let herb: f64 = s.iter().filter(|x| x.h < 3.0 && x.wood < 0.6).map(|x| x.kg).sum::<f64>() / area[i];
            let npp: f64 = s.iter().map(|x| x.net * x.built).sum::<f64>() / area[i] * 12.0 / 1.6e7;
            let lat = w.mesh.cells().lat()[i];
            let isherb = |x: &Slot| x.h < 3.0 && x.wood < 0.6;
            let herbleaf: f64 = s.iter().filter(|x| isherb(x)).map(|x| x.leaf).sum::<f64>() / area[i];
            let herbcov: f64 = s.iter().filter(|x| isherb(x)).map(|x| -x.rooted * (-(x.leaf / area[i]) / x.rooted).exp_m1()).sum::<f64>();
            let treeleaf: f64 = s.iter().filter(|x| !isherb(x)).map(|x| x.leaf).sum::<f64>() / area[i];
            let wet = cover::wet(&w, &p, i, w.g(p.liquid, i)).1;
            let act = |f: &dyn Fn(&Slot) -> bool| { let k: f64 = s.iter().filter(|x| f(x)).map(|x| x.kg).sum(); s.iter().filter(|x| f(x)).map(|x| x.kg * cover::activity(&w.cfg.phys, &x.genes, wet)).sum::<f64>() / k.max(1e-30) };
            let (herbact, treeact) = (act(&|x| isherb(x)), act(&|x| !isherb(x)));
            let hk: f64 = s.iter().filter(|x| isherb(x)).map(|x| x.kg).sum::<f64>().max(1e-30);
            let herbh: f64 = s.iter().filter(|x| isherb(x)).map(|x| x.kg * x.h).sum::<f64>() / hk;
            let herbn: f64 = s.iter().filter(|x| isherb(x)).map(|x| x.plants).sum::<f64>() / area[i];
            writeln!(csv, "{i},{},{},{lat:.2},{:.2},{kg:.4},{},{:.4},{},{:.1},{:.1},{herb:.4},{npp:.4},{herbleaf:.3},{herbcov:.3},{treeleaf:.3},{wet:.4},{herbact:.3},{treeact:.3},{herbh:.3},{herbn:.3}", c[0], c[1], tmin[i], lead.map_or(-1, |l| l.root as i64), lead.map_or(0.0, |l| l.kg / area[i]), s.len(), lead.map_or(f64::NAN, |l| l.genes[FROST] as f64), lead.map_or(f64::NAN, |l| hue(&l.genes))).unwrap();
            for x in &s { *island.entry(x.root).or_default() += x.kg; let e = gsum.entry(x.root).or_insert([0.0; NG + 2]); for q in 0..NG { e[q] += x.kg * x.genes[q] as f64; } e[NG] += x.kg * x.h; e[NG + 1] += x.kg * x.wood; }
            rows.entry(c[1]).or_default().push((lat, tmin[i], s.into_iter().map(|x| Slot { kg: x.kg / area[i], built: x.built / area[i], ..x }).collect()));
        }
        let tot: f64 = island.values().sum();
        let mut k: Vec<(u32, f64)> = island.into_iter().filter(|e| e.1 > 0.01 * tot).collect();
        k.sort_by(|a, b| b.1.total_cmp(&a.1));
        let l = format!("== year {y}: kinds over 1% of the island's cover {}: {}", k.len(), k.iter().map(|e| format!("{}:{:.0}%", e.0, 100.0 * e.1 / tot)).collect::<Vec<_>>().join(" "));
        println!("{l}");
        writeln!(log, "{l}").unwrap();
        for e in k.iter().take(10) { let g = gsum[&e.0]; let gg: Vec<String> = g.iter().map(|v| format!("{:.2}", v / e.1)).collect(); let m: [f32; NG] = std::array::from_fn(|q| (g[q] / e.1) as f32); writeln!(log, "  kind {}: genes L,leaf,walls,store,frost,moist,seed,repair,fix + h,wood: {} hue {:.0}", e.0, gg.join(" "), hue(&m)).unwrap(); }
        writeln!(log, "{:>5} {:>4} {:>7} {:>7} {:>6} {:>5} {:>5} {:>6} {:>6} {:>6} {:>6} {:>5} {:>5}", "lat", "n", "kg/m2", "lead", "lead%", "kinds", "npp", "frost", "tmin", "herbc%", "hue", "hue-", "hue+").unwrap();
        let mut ys: Vec<&u64> = rows.keys().collect();
        ys.sort();
        for yy in ys.into_iter().rev() {
            let r = &rows[yy];
            let nn = r.len() as f64;
            let kg: f64 = r.iter().map(|c| c.2.iter().map(|x| x.kg).sum::<f64>()).sum::<f64>();
            let mut by: HashMap<u32, f64> = HashMap::new();
            r.iter().flat_map(|c| &c.2).for_each(|x| *by.entry(x.root).or_default() += x.kg);
            let (lead, lk) = by.iter().max_by(|a, b| a.1.total_cmp(b.1)).map_or((u32::MAX, 0.0), |e| (*e.0, *e.1));
            let kinds = by.values().filter(|&&v| v > 0.01 * kg).count();
            let npp: f64 = r.iter().map(|c| c.2.iter().map(|x| x.net * x.built).sum::<f64>()).sum::<f64>() * 12.0 / 1.6e7 / nn;
            let built: f64 = r.iter().flat_map(|c| &c.2).map(|x| x.kg).sum::<f64>().max(1e-30);
            let frost: f64 = r.iter().flat_map(|c| &c.2).map(|x| x.kg * x.genes[FROST] as f64).sum::<f64>() / built;
            let herb: f64 = r.iter().filter(|c| c.2.iter().any(|x| x.h < 3.0 && x.wood < 0.6 && x.kg > 0.01)).count() as f64 / nn;
            let hues: Vec<f64> = r.iter().flat_map(|c| &c.2).filter(|x| x.kg > 0.05).map(|x| hue(&x.genes)).collect();
            let hm = hues.iter().sum::<f64>() / hues.len().max(1) as f64;
            let (hlo, hhi) = hues.iter().fold((f64::MAX, f64::MIN), |a, &h| (a.0.min(h), a.1.max(h)));
            let tm = r.iter().map(|c| c.1).sum::<f64>() / nn;
            let lat = r.iter().map(|c| c.0).sum::<f64>() / nn;
            writeln!(log, "{lat:5.1} {:4} {:7.2} {:>7} {:6.0} {kinds:5} {npp:5.2} {frost:6.1} {tm:6.1} {:6.1} {hm:6.0} {:5.0} {:5.0}", r.len(), kg / nn, if lead == u32::MAX { "-".into() } else { lead.to_string() }, 100.0 * lk / kg.max(1e-30), 100.0 * herb, if hues.is_empty() { f64::NAN } else { hlo }, if hues.is_empty() { f64::NAN } else { hhi }).unwrap();
        }
        log.flush().unwrap();
    }
}
