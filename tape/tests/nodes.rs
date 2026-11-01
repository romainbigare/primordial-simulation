//! The check of population nodes against individuals (`docs/handover.md` 7.1, fixed before it runs): seeds 1 and 5 at
//! 32 x 32, with nodes and with `--individuals`, then with `--roster`; seed 5 with `--crowd 8` beside its run at the
//! default `--crowd 4` as the noise (seed 1's eaters are a few dozen at 32 x 32). On the columns by starting kind
//! (means over a window of years) the modes agree if (a) each kind's mass and real life are within a factor of two, and
//! no kind lives in one mode only; (b) the main gene means move the same way wherever one moves more than 0.3 (of the
//! copy-mass gene: each gene's move in its mutation steps, `body::steps`, times the copy-mass gene's step, so a gene in
//! kelvins counts as one in decades); (c) the same kind wins (most mass, heaviest member, the order of what eaters eat);
//! and (d) each kind's births and its deaths by each main cause per member-year, and what the eaters eat a year per kg,
//! are within a factor of two at the same density. Populations boom and crash at different times, and these rates go
//! with how crowded a kind is, so (d) compares them over all the years of a run, matched by the kind's kg per km² of
//! land (`matched`). Deaths by being taken go with how many eaters hunt too (the linear part of Holling's response), so
//! they are counted per member-year and per kg of eaters per km² of the same year. A difference larger than the noise is a bug in the node rules: a rule the modes break counts only
//! where the two crowd settings keep it, or break it by less.
//!
//! The windows are years 31-40, and 26-30 with the roster (after its last kind is placed, in year 24): at 32 x 32 the
//! eaters as individuals pass 10^4 bodies in their forties, so longer runs are too slow for a test.

mod common;

use common::run;
use pcore::body::{steps, Layout, ALLOC, CHILD, COPY_MASS, GAS, HEAT, HIDDEN, INTERVAL, KEEP_T, LEAF, REPAIR, WET};
use pcore::{Config, World};
use std::collections::BTreeMap;
use tape::Recorder;

const SEEDS: [u64; 2] = [1, 5];
const SIDE: usize = 32;
/// Each plan of what is placed when (`--roster` or not) and its window of years, whose means are compared; a run ends
/// with its window.
const PLANS: [(bool, (u64, u64)); 2] = [(false, (31, 40)), (true, (26, 30))];
/// The seed run at a second crowd setting, and that setting (`Config::crowd`; the default is the other).
const NOISE_SEED: u64 = 5;
const NOISE_CROWD: usize = 8;
/// The factor within which a kind's mass and real life must agree (rule a).
const FACTOR: f64 = 2.0;
/// A gene mean moves when it lies this far from its kind's founders, in the copy-mass gene's units (rule b,
/// `copy_units`).
const MOVE: f64 = 0.3;
/// The main genes (`docs/tape.md`, `root_<k>_*`) and their place in the genome.
const GENES: [(&str, usize); 11] = [("gas", GAS), ("leaf", LEAF), ("copy_log", COPY_MASS), ("interval", INTERVAL), ("hidden", HIDDEN), ("heat", HEAT), ("keep_t", KEEP_T), ("repair", REPAIR), ("wet_gene", WET), ("alloc", ALLOC), ("child", CHILD)];
/// The rules the check holds, per plan (default, roster) and rule (a, b, c, d); it prints the others.
const HELD: [[bool; 4]; 2] = [[false, true, false, false], [false, true, true, false]];
/// What eaters took of what they use (`Body::uses`), by source.
const ATE: [&str; 3] = ["ate_cover", "ate_others", "ate_ground"];
/// The causes of death (`docs/tape.md`).
const CAUSES: [&str; 6] = ["lack", "phase", "spent", "taken", "broken", "age"];
/// Being taken, among the rates rule (d) compares (births first, then `CAUSES`).
const TAKEN: usize = 4;
/// A rate is compared (rule d) where it counts at least this many births or deaths at matched density in either run:
/// fewer are the noise of counting (a third of the count at ten).
const EVENTS: f64 = 10.0;
/// The bins of density that rule (d) matches years in, decades of kg per km² of land: a quarter of a decade, so the
/// years matched differ in density by less than a factor of 1.8.
const BIN: f64 = 0.25;
/// Square metres in a km².
const KM2: f64 = 1e6;

/// One year of a quantity that rule (d) compares: the density it is matched at (kg per km² of land), its exposure
/// (members, or kg of eaters) and its count (births or deaths, or kg eaten).
type Exposed = (f64, f64, f64);

/// One starting kind: alive at the end of the window; means over the window of its mass (kg), real life (yr), member
/// mass (kg) and each of `GENES` less its founders' value; and each year it lives, its births and its deaths by each of
/// `CAUSES` (`Exposed`, by its members).
#[derive(Clone, Default)]
struct Kind {
    alive: bool,
    kg: f64,
    life: f64,
    member: f64,
    moves: Vec<f64>,
    rates: Vec<Vec<Exposed>>,
}

/// A run: its kinds, the order of what its eaters ate over the window (most first), and each year what they ate
/// (`Exposed`, by the kg of the kinds that are not the light-eaters, kind 0).
struct Window {
    kinds: Vec<Kind>,
    ate: [usize; 3],
    intake: Vec<Exposed>,
}

/// One year of a run: per kind its members, mass, real life, member mass, the `GENES` (from `GENE_AT`), and its births
/// and deaths by cause (from `RATE_AT`); and what its eaters ate (`ATE`).
struct Year {
    year: u64,
    kinds: Vec<Vec<f64>>,
    ate: [f64; 3],
}

/// Where a kind's genes and its births start in a row of `Year::kinds`.
const GENE_AT: usize = 4;
const RATE_AT: usize = GENE_AT + GENES.len();

/// Per main gene, one of its units in the copy-mass gene's: the copy-mass gene's step over its own (`body::steps`).
fn copy_units(w: &World) -> Vec<f64> {
    let lay = Layout::of(&w.chem);
    let step = |g: usize| {
        let mut v = vec![0.0; g + 1];
        v[g] = 1.0;
        steps(&lay, &v)[g] as f64
    };
    GENES.iter().map(|&(_, g)| step(g) / step(COPY_MASS)).collect()
}

/// The mean of the finite values of `v`.
fn mean(v: impl Iterator<Item = f64>) -> f64 {
    let f: Vec<f64> = v.filter(|x| x.is_finite()).collect();
    f.iter().sum::<f64>() / f.len() as f64
}

/// The kinds of a run so far (columns `root_<k>_members`).
fn kinds(rec: &Recorder) -> usize {
    (0..).take_while(|k| !rec.last(&format!("root_{k}_members")).is_nan()).count()
}

/// `ew run --seed <seed> --size 32` (`--roster` if `roster`) to the end of `window`, with nodes or not and `crowd`.
fn check_run(seed: u64, roster: bool, window: (u64, u64), nodes: bool, crowd: usize) -> Window {
    let (mut founders, mut years, mut unit): (Vec<Option<Vec<f64>>>, Vec<Year>, Vec<f64>) = (vec![], vec![], vec![]);
    let mut land = 0.0;
    run(seed, SIDE, window.1, roster, |c| (c.nodes, c.crowd) = (nodes, crowd), |y, w, rec| {
        let col = |k: usize, name: &str| rec.last(&format!("root_{k}_{name}"));
        if unit.is_empty() {
            unit = copy_units(w);
            land = (0..w.mesh.cells().n()).filter(|&i| w.is_land(i)).map(|i| w.mesh.cells().area()[i]).sum::<f64>() / KM2;
        }
        founders.resize(kinds(rec), None);
        for (k, f) in founders.iter_mut().enumerate().filter(|(k, f)| f.is_none() && col(*k, "members") > 0.0) {
            *f = Some(GENES.iter().map(|g| col(k, g.0)).collect());
        }
        let row = |k: usize| {
            let first = [col(k, "members"), col(k, "kg"), col(k, "life_real"), col(k, "mass")];
            let genes = GENES.iter().map(|g| col(k, g.0));
            let rates = std::iter::once(col(k, "births")).chain(CAUSES.iter().map(|c| col(k, &format!("died_{c}"))));
            first.into_iter().chain(genes).chain(rates).collect()
        };
        years.push(Year { year: y, kinds: (0..kinds(rec)).map(row).collect(), ate: ATE.map(|c| rec.last(c)) });
    });
    let last = &years.last().expect("a year").kinds;
    let eaters = |y: &Year| y.kinds.iter().skip(1).map(|v| v[1]).sum::<f64>();
    let window: Vec<&Year> = years.iter().filter(|y| y.year >= window.0).collect();
    let kinds = (0..last.len())
        .map(|k| {
            let m = |q: usize| mean(window.iter().filter_map(|y| y.kinds.get(k).map(|v| v[q])));
            let start = founders[k].clone().unwrap_or_default();
            let moves = (0..GENES.len()).map(|g| (m(GENE_AT + g) - start.get(g).copied().unwrap_or(f64::NAN)) * unit[g]).collect();
            // Each year it lives: its density, its members (for being taken, times the eaters' kg per km²) and each count.
            let lived = || years.iter().filter_map(|y| y.kinds.get(k).map(|v| (v, eaters(y) / land))).filter(|x| x.0[0] > 0.0);
            let per = |c: usize, hunters: f64| if c == TAKEN { hunters } else { 1.0 };
            let rates = (0..=CAUSES.len()).map(|c| lived().map(|(v, e)| (v[1] / land, v[0] * per(c, e), v[RATE_AT + c])).collect()).collect();
            Kind { alive: last[k][0] > 0.0, kg: m(1), life: m(2), member: m(3), moves, rates }
        })
        .collect::<Vec<Kind>>();
    let eaten: Vec<f64> = (0..ATE.len()).map(|c| mean(window.iter().map(|y| y.ate[c]))).collect();
    let mut ate = [0, 1, 2];
    ate.sort_by(|&a, &b| eaten[b].total_cmp(&eaten[a]));
    let intake = years.iter().map(|y| (eaters(y) / land, eaters(y), y.ate.iter().sum())).filter(|e| e.1 > 0.0).collect();
    Window { kinds, ate, intake }
}

/// A rate of two runs at matched density: each run's years (`Exposed`) binned by log10 of their density in bins of
/// `BIN` decades; over the bins both runs lived in, each run's rate in a bin (its count over its exposure) weighted by
/// the lesser exposure of the two runs there (direct standardisation). Returns the two rates and the larger count at
/// those weights (none where they share no bin).
fn matched(x: &[Exposed], y: &[Exposed]) -> Option<(f64, f64, f64)> {
    let bins = |r: &[Exposed]| {
        let mut m: BTreeMap<i64, (f64, f64)> = BTreeMap::new();
        for &(d, e, c) in r.iter().filter(|p| p.0 > 0.0 && p.1 > 0.0) {
            let b = m.entry((d.log10() / BIN).floor() as i64).or_default();
            (b.0, b.1) = (b.0 + e, b.1 + c);
        }
        m
    };
    let (bx, by) = (bins(x), bins(y));
    let (mut cx, mut cy, mut all) = (0.0, 0.0, 0.0);
    for (b, &(ex, nx)) in &bx {
        if let Some(&(ey, ny)) = by.get(b) {
            let w = ex.min(ey);
            (cx, cy, all) = (cx + w * nx / ex, cy + w * ny / ey, all + w);
        }
    }
    (all > 0.0).then(|| (cx / all, cy / all, cx.max(cy)))
}

/// How far apart two positive numbers are, as the log of their ratio (infinite if one is not positive).
fn apart(x: f64, y: f64) -> f64 {
    if x > 0.0 && y > 0.0 { (x / y).ln().abs() } else { f64::INFINITY }
}

/// The kind with the most mass and the kind with the heaviest member, among those alive in `r`.
fn winners(r: &Window) -> (Option<usize>, Option<usize>) {
    let by = |f: fn(&Kind) -> f64| r.kinds.iter().enumerate().filter(|(_, k)| k.alive).max_by(|a, b| f(a.1).total_cmp(&f(b.1))).map(|(k, _)| k);
    (by(|k| k.kg), by(|k| k.member))
}

/// One thing the four rules compare between two runs: its rule (0 to 3 for a to d), what it is, the runs' values, and
/// how far apart the runs are on it: for rules (a) and (d) the log of the ratio (infinite where the kind lives in one
/// run only), for (b) and (c) 1 where they disagree, else 0.
struct Diff {
    rule: usize,
    what: String,
    values: String,
    size: f64,
}

/// How far apart runs `x` and `y` are on everything the rules compare.
fn diffs(x: &Window, y: &Window) -> Vec<Diff> {
    let mut d = vec![];
    let mut push = |rule, what: String, values: String, size| d.push(Diff { rule, what, values, size });
    let none = Kind::default();
    for k in 0..x.kinds.len().max(y.kinds.len()) {
        let (p, q) = (x.kinds.get(k).unwrap_or(&none), y.kinds.get(k).unwrap_or(&none));
        let (lives, both) = (p.alive != q.alive, p.alive && q.alive);
        let (dm, dl) = if lives { (f64::INFINITY, f64::INFINITY) } else if both { (apart(p.kg, q.kg), apart(p.life, q.life)) } else { (0.0, 0.0) };
        push(0, format!("kind {k} lives in one run only"), format!("{} against {}", p.alive, q.alive), if lives { f64::INFINITY } else { 0.0 });
        push(0, format!("kind {k} mass"), format!("{:.3e} against {:.3e} kg", p.kg, q.kg), dm);
        push(0, format!("kind {k} real life"), format!("{:.2} against {:.2} yr", p.life, q.life), dl);
        for (g, (name, _)) in GENES.iter().enumerate().filter(|_| both) {
            let (u, v) = (p.moves[g], q.moves[g]);
            let flip = (u.abs() > MOVE || v.abs() > MOVE) && (u > 0.0) != (v > 0.0);
            push(1, format!("kind {k} {name}"), format!("{u:+.2} against {v:+.2}"), flip as u8 as f64);
        }
        // Births, and deaths by each cause, at matched density where they count at least `EVENTS` in either run.
        for c in (0..=CAUSES.len()).filter(|_| both) {
            let name = if c == 0 { "births".to_string() } else { format!("died {}", CAUSES[c - 1]) };
            let per = if c == TAKEN { "per member-year per kg of eaters a km²" } else { "per member-year" };
            if let Some((u, v, n)) = matched(&p.rates[c], &q.rates[c]).filter(|m| m.2 >= EVENTS) {
                push(3, format!("kind {k} {name} {per}"), format!("{u:.3e} against {v:.3e} ({n:.0} events)"), apart(u, v));
            }
        }
    }
    let ((mx, hx), (my, hy)) = (winners(x), winners(y));
    push(2, "most mass".into(), format!("kind {mx:?} against {my:?}"), (mx != my) as u8 as f64);
    push(2, "heaviest member".into(), format!("kind {hx:?} against {hy:?}"), (hx != hy) as u8 as f64);
    let (ox, oy) = (x.ate.map(|c| ATE[c]), y.ate.map(|c| ATE[c]));
    push(2, "what eaters eat".into(), format!("{ox:?} against {oy:?}"), (ox != oy) as u8 as f64);
    if let Some((u, v, _)) = matched(&x.intake, &y.intake) {
        push(3, "what eaters eat per kg".into(), format!("{u:.3} against {v:.3} a year"), apart(u, v));
    }
    d
}

/// The rule a difference breaks: past a factor of `FACTOR` for rules (a) and (d), any for (b) and (c).
fn breaks(d: &Diff) -> bool {
    d.size > if d.rule == 0 || d.rule == 3 { FACTOR.ln() } else { 0.0 }
}

/// Prints run `r`'s kinds alive, with the genes that moved, and its winners.
fn print(name: &str, r: &Window) {
    for (k, d) in r.kinds.iter().enumerate().filter(|(_, d)| d.alive) {
        let moves: Vec<String> = GENES.iter().zip(&d.moves).filter(|(_, m)| m.abs() > MOVE).map(|(g, m)| format!("{} {m:+.2}", g.0)).collect();
        println!("  {name} kind {k}: {:.3e} kg, life {:.2} yr, member {:.3e} kg; moved {moves:?}", d.kg, d.life, d.member);
    }
    println!("  {name}: most mass, heaviest member {:?}; eaters ate most to least {:?}", winners(r), r.ate.map(|c| ATE[c]));
}

/// Nodes against individuals, by the three rules of handover 7.1 and the rates (d), against the noise of the crowd
/// setting.
#[test]
#[ignore = "slow: ten runs of a 32 x 32 world, 40 and 30 years"]
fn nodes_and_individuals_make_the_same_world() {
    let crowd = Config::default().crowd;
    let mut runs: Vec<(u64, usize, bool, usize)> = vec![];
    for plan in 0..PLANS.len() {
        for seed in SEEDS {
            runs.extend([(seed, plan, true, crowd), (seed, plan, false, crowd)]);
        }
        runs.push((NOISE_SEED, plan, true, NOISE_CROWD));
    }
    let done: Vec<Window> = std::thread::scope(|s| {
        let all: Vec<_> = runs.iter().map(|&(seed, plan, nodes, crowd)| s.spawn(move || check_run(seed, PLANS[plan].0, PLANS[plan].1, nodes, crowd))).collect();
        all.into_iter().map(|r| r.join().expect("a run")).collect()
    });
    let find = |seed, plan, nodes, c| &done[runs.iter().position(|&r| r == (seed, plan, nodes, c)).expect("a run")];
    let (mut bugs, mut gaps) = (vec![], vec![]);
    for (plan, &(roster, (from, to))) in PLANS.iter().enumerate() {
        let name = format!("{}, years {from}-{to}", if roster { "roster" } else { "default" });
        println!("{name}:");
        let noise = (find(NOISE_SEED, plan, true, crowd), find(NOISE_SEED, plan, true, NOISE_CROWD));
        print(&format!("seed {NOISE_SEED} crowd {NOISE_CROWD}"), noise.1);
        let kept = diffs(noise.0, noise.1);
        let shown: Vec<String> = kept.iter().filter(|d| breaks(d)).map(|d| format!("{}: {}", d.what, d.values)).collect();
        println!("  noise, crowd {crowd} against {NOISE_CROWD}, breaks: {shown:?}");
        for seed in SEEDS {
            let (n, i) = (find(seed, plan, true, crowd), find(seed, plan, false, crowd));
            print(&format!("seed {seed} nodes"), n);
            print(&format!("seed {seed} individuals"), i);
            let modes = diffs(n, i);
            for (rule, label) in ["a", "b", "c", "d"].iter().enumerate() {
                let broke: Vec<&Diff> = modes.iter().filter(|d| d.rule == rule && breaks(d)).collect();
                let noise_of = |d: &Diff| kept.iter().find(|x| x.what == d.what).map_or(0.0, |x| x.size);
                let past: Vec<&&Diff> = broke.iter().filter(|d| d.size > noise_of(d)).collect();
                let shown: Vec<String> = broke.iter().map(|d| format!("{}: {}", d.what, d.values)).collect();
                println!("  seed {seed}, rule ({label}): {} broken, {} past the noise: {shown:?}", broke.len(), past.len());
                let list = if HELD[plan][rule] { &mut bugs } else { &mut gaps };
                list.extend(past.iter().map(|d| format!("{name}, seed {seed}, rule ({label}): {}: {}", d.what, d.values)));
            }
        }
    }
    println!("differences past the noise in the rules not held: {gaps:#?}");
    assert!(bugs.is_empty(), "nodes and individuals differ past the noise: {bugs:#?}");
}
