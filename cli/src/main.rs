//! Headless runs, pictures and the history tape.

mod chem;
mod render;

use pcore::World;
use std::time::{Duration, Instant};
use tape::{fields, Recorder};

/// Command line flags: `--name value`.
struct Args(Vec<String>);

impl Args {
    fn get<T: std::str::FromStr>(&self, name: &str, default: T) -> T {
        self.0.iter().position(|a| a == name).and_then(|p| self.0.get(p + 1)).and_then(|v| v.parse().ok()).unwrap_or(default)
    }
}

fn main() {
    let args = Args(std::env::args().collect());
    match args.0.get(1).map(String::as_str).unwrap_or("help") {
        "run" => {
            let size = args.get("--size", 128usize);
            let mut w = worldgen::new_world(args.get("--seed", 1u64), size, size);
            w.cfg.sun *= args.get("--sun", 1.0);
            w.cfg.share_brains = !args.0.iter().any(|a| a == "--own-brains");
            w.cfg.flow_days = args.get("--flow-days", w.cfg.flow_days);
            w.cfg.regrow = args.get("--regrow", w.cfg.regrow);
            w.cfg.air_days = args.get("--air-days", w.cfg.air_days.min(w.cfg.flow_days));
            w.cfg.exchange_days = args.get("--exchange-days", w.cfg.exchange_days.min(w.cfg.flow_days).max(w.cfg.air_days));
            let absorber = args.get("--absorber", 1.0);
            let na = w.grid.na();
            for j in 0..na {
                let t = w.t_air(j);
                for k in (0..w.chem.ns()).filter(|&k| w.chem.subs[k].ir_abs > 0.0) {
                    w.air[k * na + j] *= absorber;
                }
                w.e_air[j] = w.heat_cap_air(j) * t;
            }
            drive(w, &args, false);
        }
        "chem" => chem::run(&args.0),
        "resume" => {
            let from = args.get("--from", String::new());
            drive(tape::load_snapshot(&from).unwrap_or_else(|e| panic!("load {from}: {e}")), &args, true);
        }
        _ => println!(
            "usage:\n  ew run [--seed N] [--size N] [--years N] [--every N] [--out DIR] [--frames-every N] [--monthly-years N] [--life-year N (-1: none)] [--flow-days N] [--air-days N] [--exchange-days N] [--own-brains]\n  ew chem [--seed N]\n  ew resume --from SNAPSHOT [--years N] [--every N] [--out DIR] [--frames-every N] [--monthly-years N]"
        ),
    }
}

/// Simulate `--years` years, printing drift and speed and writing pictures and the tape to `--out`.
fn drive(mut w: World, a: &Args, resumed: bool) {
    let (years, every, out) = (a.get("--years", 10u64), a.get("--every", 10u64), a.get("--out", String::from("out")));
    std::fs::create_dir_all(&out).expect("create out dir");
    let mut rec = Recorder::new(&w, a.get("--frames-every", 10u64), a.get("--monthly-years", 2u64));
    if resumed {
        rec.skip_existing(&mut w);
    }
    let (t0, l0) = (pcore::check::totals(&w), w.ledger.clone());
    let (y0, snap_every) = (w.day / w.days_per_year(), 10);
    let snap = |w: &World, y: u64| tape::save_snapshot(w, format!("{out}/snap_{y:05}.bin.gz")).expect("save snapshot");
    render::save(&w, &format!("{out}/year{y0:04}.png"));
    rec.record_month(&mut w);
    // Time spent in flow, change, bodies and parts, since the last print and in all.
    let (mut part, mut busy, mut since) = ([Duration::ZERO; 4], Duration::ZERO, y0);
    let life_year = if resumed { -1 } else { a.get("--life-year", 2i64) };
    for y in y0 + 1..=y0 + years {
        // First light-eaters; once they have spread, eaters among them.
        for (kind, at) in [(0, life_year), (1, life_year + 20)] {
            if life_year >= 0 && y as i64 - 1 == at {
                worldgen::life::seed(&mut w, kind);
            }
        }
        for _ in 0..w.cfg.months_per_year {
            let mut t = Instant::now();
            pcore::step_marked(&mut w, &mut |p| {
                part[p] += t.elapsed();
                t = Instant::now();
            });
            rec.record_month(&mut w);
        }
        // The yearly check is the release check; debug builds also check after every law part.
        if let Some(e) = pcore::check::broken(&w) {
            eprintln!("year {y}: broken state: {e}");
            rec.finish(&out).expect("write tape");
            std::process::exit(1);
        }
        if y % every == 0 || y == y0 + years {
            let (de, dq) = pcore::check::drift(&w, &t0, &l0);
            let (all, le) = (w.life.alive().count(), w.life.alive().filter(|b| tape::light_eater(&w.chem, &b.genes)).count());
            // Members of light-eaters; individuals, nodes and members of the others.
            let mut m = [0.0; 4];
            for b in w.life.alive() {
                let k = if tape::light_eater(&w.chem, &b.genes) { 0 } else { 1 + (b.n() > 1.0) as usize };
                (m[k], m[3]) = (m[k] + if k == 0 { b.n() } else { 1.0 }, m[3] + if k > 0 { b.n() } else { 0.0 });
            }
            let ms = part.map(|d| d.as_secs_f64() * 1000.0 / (y - since) as f64);
            since = y;
            println!("year {y}: element drift {de:.2e}, energy drift {dq:.2e}, heat not given {:.2e} J, bodies {all} ({le} light-eaters with {:.3e} members; others {} single, {} nodes, {:.3e} members), lineages {}; ms/yr flow {:.0}, change {:.0}, bodies {:.0}, parts {:.1} ({} parts)", w.ledger.clamped_heat, m[0], m[1], m[2], m[3], w.life.lineages(), ms[0], ms[1], ms[2], ms[3], w.parts.alive().count());
            busy += part.iter().sum::<Duration>();
            part = [Duration::ZERO; 4];
            render::save(&w, &format!("{out}/year{y:04}.png"));
        }
        if y % snap_every == 0 || y == y0 + years {
            snap(&w, y);
        }
    }
    rec.finish(&out).expect("write tape");
    // Rivers as the viewers draw them: dry land cells with a liquid flux over a threshold, mol/s.
    let f = fields(&w);
    let flux = |i: usize| w.diag.liquid_flux.get(i).copied().unwrap_or(0.0) as f64;
    let dry: Vec<usize> = (0..w.grid.n()).filter(|&i| f.z[i] > 0.0 && f.depth[i] <= 1.0).collect();
    let over = |t: f64| dry.iter().filter(|&&i| flux(i) > t).count();
    let rain = (0..w.grid.n()).filter(|&i| f.z[i] > 0.0).map(|i| w.diag.rain[i] as f64).sum::<f64>() / (0..w.grid.n()).filter(|&i| f.z[i] > 0.0).count().max(1) as f64;
    println!("rivers: max flux {:.3e} mol/s on land, dry land cells over 9e3 (web) {}, over 3e4 (png) {}, mean land rain {rain:.3e} mol/m2/s", dry.iter().map(|&i| flux(i)).fold(0.0, f64::max), over(9e3), over(3e4));
    let s = (busy + part.iter().sum::<Duration>()).as_secs_f64();
    println!("{years} years in {s:.2} s ({:.1} ms per year), hash {:016x}", s * 1000.0 / years.max(1) as f64, pcore::check::hash(&w));
}
