//! Headless runs, pictures and the history tape.

mod chem;
mod render;

use pcore::World;
use std::time::{Duration, Instant};
use tape::Recorder;

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
            "usage:\n  ew run [--seed N] [--size N] [--years N] [--every N] [--out DIR] [--frames-every N] [--monthly-years N] [--life-year N (-1: none)]\n  ew chem [--seed N]\n  ew resume --from SNAPSHOT [--years N] [--every N] [--out DIR] [--frames-every N] [--monthly-years N]"
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
    let mut busy = Duration::ZERO;
    let life_year = if resumed { -1 } else { a.get("--life-year", 2i64) };
    for y in y0 + 1..=y0 + years {
        // First light-eaters, a few years later eaters among them.
        for (kind, at) in [(0, life_year), (1, life_year + 3)] {
            if life_year >= 0 && y as i64 - 1 == at {
                worldgen::life::seed(&mut w, kind);
            }
        }
        for _ in 0..w.cfg.months_per_year {
            let t = Instant::now();
            pcore::step(&mut w);
            busy += t.elapsed();
            rec.record_month(&mut w);
        }
        if let Some(e) = pcore::check::broken(&w) {
            eprintln!("year {y}: broken state: {e}");
            rec.finish(&out).expect("write tape");
            std::process::exit(1);
        }
        if y % every == 0 || y == y0 + years {
            let (de, dq) = pcore::check::drift(&w, &t0, &l0);
            let (all, le) = (w.life.alive().count(), w.life.alive().filter(|b| pcore::body::light_eater(&w.chem, &b.genes)).count());
            println!("year {y}: element drift {de:.2e}, energy drift {dq:.2e}, bodies {all} ({le} light-eaters), lineages {}", w.life.lineages());
            render::save(&w, &format!("{out}/year{y:04}.png"));
        }
        if y % snap_every == 0 || y == y0 + years {
            snap(&w, y);
        }
    }
    rec.finish(&out).expect("write tape");
    let s = busy.as_secs_f64();
    println!("{years} years in {s:.2} s ({:.1} ms per year), hash {:016x}", s * 1000.0 / years.max(1) as f64, pcore::check::hash(&w));
}
