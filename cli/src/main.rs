//! `ew`: headless runs, pictures and the history tape.

mod args;
mod chem;
mod numbers;
mod render;
mod report;

use args::{Args, USAGE};
use pcore::World;
use report::Clock;
use std::time::{Duration, Instant};
use tape::{Kept, Recorder};
use worldgen::schedule::Schedule;

/// Years between snapshots (and between writes of the tape so far).
const SNAP_EVERY: u64 = 10;

fn main() {
    let args = Args(std::env::args().collect());
    match args.command() {
        "run" => {
            let size = args.get("--size", 128usize);
            let mut w = worldgen::new_world(args.get("--seed", 1u64), size, size);
            args.apply(&mut w);
            if let Err(e) = w.cfg.validate() {
                eprintln!("ew run: {e}");
                std::process::exit(2);
            }
            let plan = args.schedule();
            drive(w, &args, plan, None);
        }
        "resume" => {
            // The snapshot carries the plan of its run (what is still due is placed when it is due) and what its
            // recorder kept across years.
            let from = args.get("--from", String::new());
            let (w, (plan, kept)) = tape::load_snapshot(&from).unwrap_or_else(|e| panic!("load {from}: {e}"));
            drive(w, &args, plan, Some(kept));
        }
        "chem" => chem::run(&args),
        "numbers" => numbers::run(&args.0),
        _ => println!("{USAGE}"),
    }
}

/// Simulate `--years` years: place what `plan` says when it is due, print drift and speed, and write pictures, the
/// tape and snapshots to `--out`. A resumed run brings what its recorder kept (`kept`).
fn drive(mut w: World, a: &Args, plan: Schedule, kept: Option<Kept>) {
    let (years, every, out) = (a.get("--years", 10u64), a.get("--every", 10u64), a.get("--out", String::from("out")));
    std::fs::create_dir_all(&out).expect("create out dir");
    let mut rec = Recorder::new(&w, a.get("--frames-every", 10u64), a.get("--monthly-years", 2u64));
    let resumed = kept.is_some();
    if let Some(kept) = kept {
        rec.resume(&mut w, kept, &out).unwrap_or_else(|e| panic!("load the tape: {e}"));
    }
    let t0 = pcore::check::totals(&w);
    let y0 = w.cfg.calendar().years(w.time);
    render::save(&w, &format!("{out}/year{y0:04}.png"));
    rec.record_month(&mut w);
    // Time spent in flow, change, bodies and parts, and in the tape (the recorder each month, the files each
    // decade), since the last print; and in all.
    let mut clock = Clock { stages: [Duration::ZERO; 4], tape: Duration::ZERO, since: y0, updates: w.counts.updates };
    let mut busy = Duration::ZERO;
    println!("{}", report::config(&w, &plan.seeding));
    if !resumed {
        report::balances(&w).iter().for_each(|l| println!("{l}"));
    }
    for y in y0 + 1..=y0 + years {
        plan.place(&mut w).iter().for_each(|l| println!("{l}"));
        for _ in 0..w.cfg.months_per_year {
            let mut t = Instant::now();
            pcore::step_marked(&mut w, &mut |s| {
                clock.stages[s as usize] += t.elapsed();
                t = Instant::now();
            });
            let t = Instant::now();
            rec.record_month(&mut w);
            clock.tape += t.elapsed();
        }
        // The yearly check is the release check; debug builds also check after every law part.
        if let Some(e) = pcore::check::broken(&w) {
            eprintln!("year {y}: broken state: {e}");
            rec.finish(&out).expect("write tape");
            std::process::exit(1);
        }
        if y % every == 0 || y == y0 + years {
            println!("{}", report::year(&w, y, pcore::check::drift(&w, &t0), &clock, &rec));
            busy += clock.stages.iter().sum::<Duration>() + clock.tape;
            clock = Clock { stages: [Duration::ZERO; 4], tape: Duration::ZERO, since: y, updates: w.counts.updates };
            render::save(&w, &format!("{out}/year{y:04}.png"));
        }
        if y % SNAP_EVERY == 0 || y == y0 + years {
            tape::save_snapshot(&w, &(&plan, rec.kept()), format!("{out}/snap_{y:05}.bin.gz")).expect("save snapshot");
            // Write the tape so far too, so a stopped run keeps its series and frames.
            let t = Instant::now();
            rec.finish(&out).expect("write tape");
            clock.tape += t.elapsed();
        }
    }
    let t = Instant::now();
    rec.finish(&out).expect("write tape");
    clock.tape += t.elapsed();
    println!("{}", report::rivers(&w, &rec));
    let s = (busy + clock.stages.iter().sum::<Duration>() + clock.tape).as_secs_f64();
    println!("{years} years in {s:.2} s ({:.1} ms per year), hash {:016x}", s * 1000.0 / years.max(1) as f64, tape::fingerprint(&w));
}
