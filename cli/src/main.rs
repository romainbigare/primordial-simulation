//! `ew`: headless runs, pictures and the history tape.

mod args;
mod chem;
mod numbers;
mod render;
mod report;
mod writer;

use args::{Args, FRAMES_EVERY, PICTURES_EVERY, SNAPSHOTS_EVERY, USAGE};
use pcore::World;
use report::Clock;
use std::time::Instant;
use tape::{Kept, Recorder, TAPE};
use writer::Writer;
use worldgen::schedule::Schedule;

/// Years between writes of the tape so far, so that a stopped run keeps its series and frames.
const TAPE_EVERY: u64 = 10;

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
            let (island, shape) = args.island().and_then(|cfg| Ok((cfg, args.shape()?))).unwrap_or_else(|e| {
                eprintln!("ew: {e}");
                std::process::exit(2);
            });
            let mut w = if args.has("--globe") { worldgen::terrain::new_globe(seed, size, args.fine()) } else { worldgen::terrain::new_island(seed, (size, size), shape, island) };
            if args.has("--band") {
                worldgen::planet::stand(&mut w);
            }
            args.apply(&mut w);
            drive(w, &args, args.schedule(), None);
        }
        "resume" => {
            // The snapshot carries the plan of its run (what is still due is placed when it is due) and what its
            // recorder kept across years.
            let from = args.get("--from", String::new());
            let (w, (plan, kept)) = tape::load_snapshot(&from).unwrap_or_else(|e| panic!("load {from}: {e}"));
            let into = w.month() % w.cfg.months_per_year as u64;
            if into != 0 {
                eprintln!("ew: {from} is taken {into} months into a year (--snapshots-at); ew resume goes on from a year's start");
                std::process::exit(2);
            }
            drive(w, &args, plan, Some(kept));
        }
        "view" => view(&args),
        "card" => card(&args),
        "chem" => chem::run(),
        "numbers" => numbers::run(&args.0),
        _ => unreachable!("`Args::check` passes only the commands of the usage"),
    }
}

/// `ew view`: draw a window of a snapshot's map (`render::window`).
fn view(a: &Args) {
    let from = a.get("--from", String::new());
    let (w, _): (World, (Schedule, Kept)) = tape::load_snapshot(&from).unwrap_or_else(|e| panic!("load {from}: {e}"));
    let window = a.window([0.0, 0.0, w.map.w as f64, w.map.h as f64]);
    let out = window.and_then(|win| render::window(&w, win, a.get("--px", 512usize), &a.get("--layer", String::from("map")), a.has("--edges"), &a.get("--out", String::from("view.png"))));
    if let Err(e) = out {
        eprintln!("ew: {e}");
        std::process::exit(2);
    }
}

/// `ew card`: write the test card (`worldgen::testkit::card`) as a snapshot, with a plan that places nothing.
fn card(a: &Args) {
    let w = worldgen::testkit::card();
    let plan = (Schedule::new(None, &[], worldgen::life::Seeding::default()), Recorder::new(&w, FRAMES_EVERY, 0).kept());
    let (out, mut bytes) = (a.get("--out", String::from("card.bin.gz")), Vec::new());
    if let Err(e) = tape::snapshot_bytes(&w, &plan, &mut bytes).and_then(|_| tape::write_snapshot(&bytes, &out)) {
        eprintln!("ew: {out}: {e}");
        std::process::exit(2);
    }
}

/// Give the recorder the month that has just ended: the world's events, at a year's end its counts, and the time spent
/// so far.
fn record(rec: &mut Recorder, w: &mut World, clock: &Clock) {
    let events = w.drain_events();
    let taken = Recorder::wants_counters(w).then(|| w.take_counters());
    rec.record_month(w, events, taken, &clock.spent);
}

/// Simulate `--years` years: at each month's start place what `plan` says when it is due; print drift and speed, and write the tape, and pictures and snapshots if
/// asked for, to `--out`. A resumed run brings what its recorder kept (`kept`).
fn drive(mut w: World, a: &Args, plan: Schedule, kept: Option<Kept>) {
    // (A setting of the run, not of its world: a snapshot does not keep it.)
    w.cfg.chunks = a.get("--chunks", w.cfg.chunks);
    if let Err(e) = w.cfg.validate() {
        eprintln!("ew: {e}");
        std::process::exit(2);
    }
    let (years, every, out) = (a.get("--years", 10u64), a.get("--every", 10u64), a.get("--out", String::from("out")));
    std::fs::create_dir_all(&out).expect("create out dir");
    let (pictures, snapshots) = (a.get("--pictures-every", PICTURES_EVERY), a.get("--snapshots-every", SNAPSHOTS_EVERY));
    let mut rec = Recorder::new(&w, a.get("--frames-every", FRAMES_EVERY), a.get("--monthly-years", 2u64));
    let resumed = kept.is_some();
    if let Some(kept) = kept {
        rec.resume(&w, kept, &out).unwrap_or_else(|e| panic!("load the tape: {e}"));
    }
    // A new world's ground gets Flow's state from its heat first, so the books read the potential energy of its matter
    // from the start (`check::Totals::potential`); a resumed one holds it.
    if !resumed {
        pcore::flow::state(&mut w, true);
    }
    let t0 = pcore::check::totals(&w);
    let y0 = w.cfg.calendar().years(w.time);
    let per_year = w.cfg.months_per_year as u64;
    let inside = |m: &u64| (w.month() + 1..=(y0 + years) * per_year).contains(m);
    let outside = || format!("--snapshots-at: a date outside this run's years {y0} to {}", y0 + years);
    let snapshots_at = a.snapshot_months(per_year).and_then(|at| if at.iter().all(inside) { Ok(at) } else { Err(outside()) }).unwrap_or_else(|e| {
        eprintln!("ew: {e}");
        std::process::exit(2);
    });
    if pictures > 0 {
        render::save(&w, &format!("{out}/year{y0:04}.png"));
    }
    let mut clock = Clock::new(y0);
    let mut writer = Writer::new();
    record(&mut rec, &mut w, &clock);
    let (mut settled_from, mut settled_to) = (clock.total(), clock.total());
    println!("{}", report::config(&w, &plan.seeding));
    if !resumed {
        report::balances(&w).iter().for_each(|l| println!("{l}"));
    }
    for y in y0 + 1..=y0 + years {
        for month in 0..w.cfg.months_per_year {
            if month == 0 {
                plan.place(&mut w).iter().for_each(|l| println!("{l}"));
            }
            let mut t = Instant::now();
            pcore::step_marked(&mut w, &mut |_, s| {
                clock.spent[s as usize] += t.elapsed();
                t = Instant::now();
            });
            let t = Instant::now();
            record(&mut rec, &mut w, &clock);
            // A snapshot at a date of `--snapshots-at`, with the tape so far.
            let m = w.month();
            if snapshots_at.contains(&m) {
                let at = format!("{out}/snap_{:05}_{:02}.bin.gz", m / per_year, m % per_year);
                writer.start(&w, &(&plan, rec.kept()), Some(at), rec.tape(), out.clone());
            }
            clock.spent[TAPE] += t.elapsed();
        }
        writer.check();
        // The yearly check is the release check, with the books' drift; debug builds also check after every law part.
        if let Some(e) = pcore::check::broken(&w).or_else(|| pcore::check::drifted(&w, &t0)) {
            eprintln!("year {y}: broken state: {e}");
            writer.wait();
            rec.finish(&out).expect("write tape");
            std::process::exit(1);
        }
        let last = y == y0 + years;
        // The settled cost is the simulation of the last years, not the writes of the end.
        if y == report::settled_after(y0, years) {
            settled_from = clock.total();
        }
        if last {
            settled_to = clock.total();
        }
        if y % every == 0 || last {
            println!("{}", report::year(&w, y, &t0, &clock));
            if let Some(line) = report::edges(&w) {
                println!("{line}");
            }
            clock.print_at(y);
        }
        if pictures > 0 && (y % pictures == 0 || last) {
            let t = Instant::now();
            render::save(&w, &format!("{out}/year{y:04}.png"));
            clock.spent[TAPE] += t.elapsed();
        }
        // The tape so far every `TAPE_EVERY` years and at the end, and with it a snapshot at each `--snapshots-every`
        // years and at the end. The writer thread compresses and writes them; the main thread only makes the snapshot's
        // bytes and copies the tape.
        let snapshot = (last || (snapshots > 0 && y % snapshots == 0)).then(|| format!("{out}/snap_{y:05}.bin.gz"));
        if snapshot.is_some() || y % TAPE_EVERY == 0 {
            let t = Instant::now();
            writer.start(&w, &(&plan, rec.kept()), snapshot, rec.tape(), out.clone());
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
    println!("{}", report::speed(&w, years, clock.total(), settled_to - settled_from));
}
