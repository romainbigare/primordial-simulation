//! Headless runs and pictures.

mod render;

use std::time::Instant;

fn arg<T: std::str::FromStr>(args: &[String], name: &str, default: T) -> T {
    args.iter()
        .position(|a| a == name)
        .and_then(|p| args.get(p + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(String::as_str).unwrap_or("help");
    let seed = arg(&args, "--seed", 1u64);
    let size = arg(&args, "--size", 128usize);
    let years = arg(&args, "--years", 10u64);
    let out = arg(&args, "--out", String::from("out"));
    std::fs::create_dir_all(&out).ok();
    match cmd {
        "run" => {
            let mut w = worldgen::new_world(seed, size, size);
            let t0 = pcore::check::totals(&w);
            let l0 = w.ledger.clone();
            render::save(&w, &format!("{out}/year0000.png"));
            let start = Instant::now();
            for y in 1..=years {
                for _ in 0..w.cfg.months_per_year {
                    pcore::step(&mut w);
                }
                if let Some(e) = pcore::check::broken(&w) {
                    eprintln!("year {y}: broken state: {e}");
                    std::process::exit(1);
                }
                let every = arg(&args, "--every", 10u64);
                if y % every == 0 || y == years {
                    let (de, dq) = pcore::check::drift(&w, &t0, &l0);
                    println!("year {y}: element drift {de:.2e}, energy drift {dq:.2e}");
                    render::save(&w, &format!("{out}/year{y:04}.png"));
                }
            }
            let s = start.elapsed().as_secs_f64();
            println!("{years} years in {s:.2} s ({:.1} ms per year)", s * 1000.0 / years as f64);
        }
        _ => println!("usage: ew run [--seed N] [--size N] [--years N] [--every N] [--out DIR]"),
    }
}
