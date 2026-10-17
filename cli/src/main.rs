//! Headless runs, pictures and the history tape.

mod chem;
mod numbers;
mod render;

use pcore::World;
use std::time::{Duration, Instant};
use tape::{fields, Recorder};

/// Command line flags: `--name value`.
struct Args(Vec<String>);

impl Args {
    fn has(&self, name: &str) -> bool {
        self.0.iter().any(|a| a == name)
    }
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
            w.cfg.crowd = args.get("--crowd", w.cfg.crowd);
            // A null run (no learning, no imitation, brain rows frozen) to compare a seed with.
            w.cfg.null = args.has("--null");
            // The detailed form to check the nodes against: animals are all individuals (light-eaters stay nodes).
            w.cfg.nodes = !args.has("--individuals");
            // Two-parent copying off (`Config::two_parents`): every child a copy of one parent, to compare with.
            w.cfg.two_parents = !args.has("--no-mix");
            w.cfg.air_days = args.get("--air-days", w.cfg.air_days.min(w.cfg.flow_days));
            // Watch runs keep the moisture path at 3 days, the default of `Config::exchange_days` (the rivers stay whole, `Config::air_days`); kept a multiple of `air_days` and no more than `flow_days` when those are set.
            w.cfg.exchange_days = args.get("--exchange-days", 3u32.min(w.cfg.flow_days).max(w.cfg.air_days));
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
        "numbers" => numbers::run(&args.0),
        "resume" => {
            let from = args.get("--from", String::new());
            drive(tape::load_snapshot(&from).unwrap_or_else(|e| panic!("load {from}: {e}")), &args, true);
        }
        _ => println!(
            "usage:\n  ew run [--seed N] [--size N] [--years N] [--every N] [--out DIR] [--frames-every N] [--monthly-years N] [--life-year N (-1: none)] [--roster (place the ten kinds of worldgen::life::ROSTER, each at its year after the first life, instead of the two: light-eaters first, eaters of remains at +10, eaters of light-eaters at +20, eaters of eaters at +24)] [--flow-days N] [--air-days N] [--exchange-days N (3)] [--own-brains] [--crowd N] [--no-mix (children have one parent, not two: the check to compare with)] [--individuals (animals are never nodes: children placed as individuals, no merge, no split; light-eaters stay nodes)] [--null (learning and imitation off, brain rows frozen: the run to compare a seed with)] [--spray ROUNDS (20)] [--spray-size ROW BIAS STEP (0.2)] [--spray-genes TIMES THE GENE STEP (3)] [--fixers SHARE (0.1: the share of the starting light-eaters that carry the gene that fixes the air's nitrogen)] [--plant-bias (light-eaters start with the hand-set biases, not neutral brains)] [--neutral-eaters (animals start with neutral brains, not the hand-set eater biases: the old start, a lottery)] [--eater-bias (the default; accepted)] [--clever-year N (copies of N living eaters, --clever-n (300), twice: one set with a big brain, one with the brain unchanged, each its own kind)]\n  ew chem [--seed N]\n  ew numbers OLD NEW (the fields of Physics and Config that differ between two runs: name, old, new; each a run's directory or its physics.json)\n  ew resume --from SNAPSHOT [--years N] [--every N] [--out DIR] [--frames-every N] [--monthly-years N]"
        ),
    }
}

/// Simulate `--years` years, printing drift and speed and writing pictures and the tape to `--out`.
fn drive(mut w: World, a: &Args, resumed: bool) {
    let (years, every, out) = (a.get("--years", 10u64), a.get("--every", 10u64), a.get("--out", String::from("out")));
    std::fs::create_dir_all(&out).expect("create out dir");
    let mut rec = Recorder::new(&w, a.get("--frames-every", 10u64), a.get("--monthly-years", 2u64));
    if resumed {
        rec.resume(&mut w, &out).unwrap_or_else(|e| panic!("load the tape: {e}"));
    }
    let (t0, l0) = (pcore::check::totals(&w), w.ledger.clone());
    let (y0, snap_every) = (w.day / w.days_per_year(), 10);
    let snap = |w: &World, y: u64| tape::save_snapshot(w, format!("{out}/snap_{y:05}.bin.gz")).expect("save snapshot");
    render::save(&w, &format!("{out}/year{y0:04}.png"));
    rec.record_month(&mut w);
    // Time spent in flow, change, bodies and parts, and in the tape (the recorder each month,
    // the files each decade), since the last print and in all; and the body updates so far.
    let (mut part, mut tape, mut busy, mut since, mut upd) = ([Duration::ZERO; 4], Duration::ZERO, Duration::ZERO, y0, w.life.updates);
    let life_year = if resumed { -1 } else { a.get("--life-year", 2i64) };
    // What is placed and how (initial conditions, reported with every result): the animal kinds
    // start with the hand-set eater biases (review 6, finding 1: 97 to 99% of neutral-brained
    // founders die in their first year, so a start with neutral eaters is a lottery and no law may
    // be set to win it), unless `--neutral-eaters`; the plants start with neutral brains, unless
    // `--plant-bias`; another number of rounds of mutation (`--spray`) or size of spray (`--spray-size` on the row biases, `--spray-genes` on the genes).
    // `--eater-bias` is the default and is still accepted.
    let clever = a.get("--clever-year", -1i64);
    let how = worldgen::life::Seeding { hand: [a.has("--plant-bias"), !a.has("--neutral-eaters") || a.has("--eater-bias")], rounds: a.get("--spray", 20u32), size: a.get("--spray-size", 0.2f32), genes: a.get("--spray-genes", 3.0f32), fixers: a.get("--fixers", worldgen::life::FIXERS) };
    // Every report says whether this is the real run or its null run (a resumed run keeps the snapshot's), and which start the animals and the plants had.
    println!("config: seed {}, {}x{}, {}, nodes {}, two parents {}, start: animals {}, plants {}", w.seed, w.grid.w, w.grid.h, if w.cfg.null { "NULL RUN (learning off, imitation off, brain rows frozen)" } else { "real run" }, if w.cfg.nodes { "on (animals may be nodes)" } else { "OFF (--individuals: animals are individuals, light-eaters nodes)" }, if w.cfg.two_parents { "on (a mature alike body of the cell is a second parent)" } else { "OFF (--no-mix: one parent)" }, if how.hand[1] { "HAND-SET eater biases (default; --neutral-eaters for neutral brains)" } else { "NEUTRAL brains (--neutral-eaters)" }, if how.hand[0] { "hand-set biases (--plant-bias)" } else { "neutral brains (default)" });
    // The liquid's store at the start, set at balance with the air (`Reactor::balance`): what each reversible pair makes, in the ground, and the gas it holds over the air's.
    if !resumed {
        let (n, na) = (w.grid.n(), w.grid.na());
        for (f, _, _, u) in pcore::change::reversible(&w.chem) {
            let held: f64 = w.ground[u * n..(u + 1) * n].iter().sum();
            let cells = w.ground[u * n..(u + 1) * n].iter().filter(|&&x| x > 0.0).count();
            if let Some(&(g, c)) = w.chem.reactions[f].inputs.iter().find(|p| w.chem.phase(p.0 as usize, 290.0) == pcore::chem::Phase::Gas) {
                let air: f64 = w.air[g as usize * na..(g as usize + 1) * na].iter().sum();
                println!("start: substance {u} held at balance with the air in {cells} of {n} cells, {held:.3e} mol, {:.2} times the air's substance {g} ({air:.3e} mol)", held * c as f64 / air);
            }
        }
    }
    // What is placed and when: the two starting kinds, or with `--roster` the ten of `worldgen::life::ROSTER`; the big-brain test (`seed_clever`) at its own year.
    let kinds: Vec<usize> = if a.has("--roster") { worldgen::life::ROSTER.to_vec() } else { vec![0, 1] };
    let plan: Vec<(usize, i64)> = kinds.iter().map(|&k| (k, life_year + worldgen::life::KINDS[k].year)).collect();
    for y in y0 + 1..=y0 + years {
        // The soil gets its mineral when life starts (`worldgen::life::soil`).
        if life_year >= 0 && y as i64 - 1 == life_year {
            println!("{}", worldgen::life::soil(&mut w, &worldgen::chemistry0().1.soil));
        }
        // First light-eaters; once they have spread, the others (each kind at its own year after the first life).
        for &(kind, at) in &plan {
            if life_year >= 0 && at >= 0 && y as i64 - 1 == at {
                println!("{}", worldgen::life::seed_with(&mut w, kind, &how));
            }
        }
        // The big-brain test: copies of living eaters, with the big brain and with the brain unchanged (a control).
        if clever >= 0 && y as i64 - 1 == clever {
            println!("{}", worldgen::life::seed_clever(&mut w, a.get("--clever-n", 300usize)));
        }
        for _ in 0..w.cfg.months_per_year {
            let mut t = Instant::now();
            pcore::step_marked(&mut w, &mut |s| {
                part[s as usize] += t.elapsed();
                t = Instant::now();
            });
            let t = Instant::now();
            rec.record_month(&mut w);
            tape += t.elapsed();
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
            // Microseconds per body update: the cost of one decision, whatever their number.
            let us = part[2].as_secs_f64() * 1e6 / (w.life.updates - upd).max(1) as f64;
            let ms_tape = tape.as_secs_f64() * 1000.0 / (y - since) as f64;
            (since, upd) = (y, w.life.updates);
            println!("year {y}: element drift {de:.2e}, energy drift {dq:.2e}, heat not given {:.2e} J, bodies {all} (at most {} in a cell; {le} light-eaters with {:.3e} members; others {} single, {} nodes, {:.3e} members), lineages {}, born brains {}; ms/yr flow {:.0}, change {:.0}, bodies {:.0}, parts {:.1}, tape {:.0}; {:.2} us/update ({} parts); node splits {:.0} in two, {:.0} one off; kinds {} ({} by members), merged unlike {:.0}/yr, wide {:.0}/yr, gifts {}/yr, watches {}/yr", w.ledger.clamped_heat, w.life.at.iter().map(|v| v.len()).max().unwrap_or(0), m[0], m[1], m[2], m[3], w.life.lineages(), w.life.genomes.iter().filter(|g| !g.rows.is_empty()).count(), ms[0], ms[1], ms[2], ms[3], ms_tape, us, w.parts.alive().count(), w.diag.splits[0], w.diag.splits[1], rec.last("kinds"), rec.last("kinds_members"), rec.last("merged_unlike"), rec.last("merged_wide"), rec.last("gave"), rec.last("watched"));
            busy += part.iter().sum::<Duration>() + tape;
            (part, tape) = ([Duration::ZERO; 4], Duration::ZERO);
            render::save(&w, &format!("{out}/year{y:04}.png"));
        }
        if y % snap_every == 0 || y == y0 + years {
            snap(&w, y);
            // Write the tape so far too, so a stopped run keeps its series and frames.
            let t = Instant::now();
            rec.finish(&out).expect("write tape");
            tape += t.elapsed();
        }
    }
    let t = Instant::now();
    rec.finish(&out).expect("write tape");
    tape += t.elapsed();
    // Rivers as the viewers draw them: dry land cells with a liquid flux over a threshold, mol/s.
    let f = fields(&w);
    let flux = |i: usize| w.liquid_flux.get(i).copied().unwrap_or(0.0) as f64;
    let dry: Vec<usize> = (0..w.grid.n()).filter(|&i| f.z[i] > 0.0 && f.depth[i] <= 1.0).collect();
    let over = |t: f64| dry.iter().filter(|&&i| flux(i) > t).count();
    let rain = (0..w.grid.n()).filter(|&i| f.z[i] > 0.0).map(|i| w.rain[i] as f64).sum::<f64>() / (0..w.grid.n()).filter(|&i| f.z[i] > 0.0).count().max(1) as f64;
    println!("rivers: max flux {:.3e} mol/s on land, dry land cells over 9e3 (web) {}, over 3e4 (png) {}, mean land rain {rain:.3e} mol/m2/s", dry.iter().map(|&i| flux(i)).fold(0.0, f64::max), over(9e3), over(3e4));
    let s = (busy + part.iter().sum::<Duration>() + tape).as_secs_f64();
    println!("{years} years in {s:.2} s ({:.1} ms per year), hash {:016x}", s * 1000.0 / years.max(1) as f64, pcore::check::hash(&w));
}
