//! `ew`: headless runs, pictures and the history tape.

mod args;
mod chem;
mod numbers;
mod render;
mod report;
mod writer;

use args::{Args, USAGE};
use pcore::World;
use report::Clock;
use std::time::{Duration, Instant};
use tape::{Kept, Recorder, TAPE};
use writer::Writer;
use worldgen::schedule::Schedule;

/// Years between snapshots (and between writes of the tape so far).
const SNAP_EVERY: u64 = 10;

/// The allocator: mimalloc (Leijen, Zorn and de Moura 2019). Bodies and their rows are made on one thread of the pool
/// and freed on another after a move, a merge or a death. The system's allocator (glibc) frees such a block into the
/// arena of the thread that made it, under that arena's lock, once the freeing thread's small cache is full, and the
/// arena's lines move between cores; mimalloc gives it back to its page's own list without a lock.
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    let args = Args(std::env::args().collect());
    if args.wants_help() {
        println!("{USAGE}");
        return;
    }
    if let Err(e) = args.check() {
        eprintln!("{e}\n\n{USAGE}");
        std::process::exit(2);
    }
    pcore::par::adopt_thread();
    match args.command() {
        "run" => {
            let (size, seed) = (args.get("--size", 128usize), args.get("--seed", 1u64));
            let mut map = if args.has("--globe") { worldgen::terrain::globe(size) } else { worldgen::terrain::island(size, size) };
            map.region = args.region();
            let mut w = worldgen::terrain::new_map(seed, map);
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
        _ => unreachable!("`Args::check` passes only the commands of the usage"),
    }
}

/// Give the recorder the month that has just ended: the world's events, at a year's end its counts, and the time spent
/// so far.
fn record(rec: &mut Recorder, w: &mut World, clock: &Clock) {
    let events = w.drain_events();
    let taken = Recorder::wants_counters(w).then(|| w.take_counters());
    rec.record_month(w, events, taken, &clock.spent);
}

/// Simulate `--years` years: place what `plan` says when it is due, print drift and speed, and write pictures, the
/// tape and snapshots to `--out`. A resumed run brings what its recorder kept (`kept`).
fn drive(mut w: World, a: &Args, plan: Schedule, kept: Option<Kept>) {
    let (years, every, out) = (a.get("--years", 10u64), a.get("--every", 10u64), a.get("--out", String::from("out")));
    std::fs::create_dir_all(&out).expect("create out dir");
    let mut rec = Recorder::new(&w, a.get("--frames-every", 10u64), a.get("--monthly-years", 2u64));
    let resumed = kept.is_some();
    if let Some(kept) = kept {
        rec.resume(&w, kept, &out).unwrap_or_else(|e| panic!("load the tape: {e}"));
    }
    let t0 = pcore::check::totals(&w);
    let y0 = w.cfg.calendar().years(w.time);
    render::save(&w, &format!("{out}/year{y0:04}.png"));
    let mut clock = Clock::new(y0, w.counts.updates);
    let mut writer = Writer::new();
    record(&mut rec, &mut w, &clock);
    println!("{}", report::config(&w, &plan.seeding));
    if !resumed {
        report::balances(&w).iter().for_each(|l| println!("{l}"));
    }
    for y in y0 + 1..=y0 + years {
        plan.place(&mut w).iter().for_each(|l| println!("{l}"));
        for _ in 0..w.cfg.months_per_year {
            let mut t = Instant::now();
            pcore::step_marked(&mut w, &mut |s| {
                clock.spent[s as usize] += t.elapsed();
                t = Instant::now();
            });
            let t = Instant::now();
            record(&mut rec, &mut w, &clock);
            clock.spent[TAPE] += t.elapsed();
        }
        // The yearly check is the release check; debug builds also check after every law part.
        if let Some(e) = pcore::check::broken(&w) {
            eprintln!("year {y}: broken state: {e}");
            writer.wait();
            rec.finish(&out).expect("write tape");
            std::process::exit(1);
        }
        if y % every == 0 || y == y0 + years {
            println!("{}", report::year(&w, y, &t0, &clock, &rec));
            clock.print_at(y, w.counts.updates);
            render::save(&w, &format!("{out}/year{y:04}.png"));
        }
        // A snapshot with each decade and at the end, and with it the tape so far, so a stopped run keeps its series and
        // frames. The writer thread compresses and writes them; the main thread only makes the snapshot's bytes and copies the tape.
        if y % SNAP_EVERY == 0 || y == y0 + years {
            let t = Instant::now();
            writer.start(&w, &(&plan, rec.kept()), format!("{out}/snap_{y:05}.bin.gz"), rec.tape(), out.clone());
            clock.spent[TAPE] += t.elapsed();
        }
    }
    let t = Instant::now();
    writer.wait();
    if years == 0 {
        rec.finish(&out).expect("write tape");
    }
    clock.spent[TAPE] += t.elapsed();
    println!("{}", report::rivers(&w, &rec));
    let s = clock.spent.iter().sum::<Duration>().as_secs_f64();
    println!("{years} years in {s:.2} s ({:.1} ms per year), hash {:016x}", s * 1000.0 / years.max(1) as f64, tape::fingerprint(&w));
}
