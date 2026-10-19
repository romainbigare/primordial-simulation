//! The check of population nodes against individuals (`docs/handover.md` 7.1, fixed before it runs): seeds 1 and 5 at
//! 32 x 32, with nodes and with `--individuals`, then with `--roster`; seed 1 with `--crowd 8` beside its run at the
//! default `--crowd 4` as the noise. On the columns by starting kind (means over a window of years) the modes agree if
//! (a) each kind's mass and real life are within a factor of two, and no kind lives in one mode only; (b) the main gene
//! means move the same way wherever one moves more than 0.3 (of the copy-mass gene: each gene's move in its mutation
//! steps, `body::steps`, times the copy-mass gene's step, so a gene in kelvins counts as one in decades); (c) the same
//! kind wins (most mass, heaviest member, the
//! order of what eaters eat). A difference larger than the noise is a bug in the node rules: a rule the modes break
//! counts only where the two crowd settings keep it, or break it by less. It also prints a census of the young: the
//! share of each kind's members in young (immature) bodies, and their median member body over the copy mass.
//!
//! The windows are years 41-50, and 26-30 with the roster (after its last kind is placed, in year 24): at 32 x 32 the
//! eaters as individuals pass 10^4 bodies by year 70, and the roster's eaters of remains 10^5 by year 28, doubling each
//! year: too slow for longer runs, let alone a test.

mod common;

use common::run;
use pcore::body::{steps, Layout, ALLOC, CHILD, COPY_MASS, GAS, HEAT, HIDDEN, INTERVAL, KEEP_T, LEAF, REPAIR, WET};
use pcore::{Config, World};
use tape::Recorder;

const SEEDS: [u64; 2] = [1, 5];
const SIDE: usize = 32;
/// Each plan of what is placed when (`--roster` or not) and its window of years, whose means are compared; a run ends
/// with its window.
const PLANS: [(bool, (u64, u64)); 2] = [(false, (41, 50)), (true, (26, 30))];
/// The seed run at a second crowd setting, and that setting (`Config::crowd`; the default is the other).
const NOISE_SEED: u64 = 1;
const NOISE_CROWD: usize = 8;
/// The factor within which a kind's mass and real life must agree (rule a).
const FACTOR: f64 = 2.0;
/// A gene mean moves when it lies this far from its kind's founders, in the copy-mass gene's units (rule b,
/// `copy_units`).
const MOVE: f64 = 0.3;
/// The main genes (`docs/tape.md`, `root_<k>_*`) and their place in the genome.
const GENES: [(&str, usize); 11] = [("gas", GAS), ("leaf", LEAF), ("copy_log", COPY_MASS), ("interval", INTERVAL), ("hidden", HIDDEN), ("heat", HEAT), ("keep_t", KEEP_T), ("repair", REPAIR), ("wet_gene", WET), ("alloc", ALLOC), ("child", CHILD)];
/// The rules the check holds, per plan (default, roster) and rule (a, b, c); it prints the others.
const HELD: [[bool; 3]; 2] = [[false, true, true], [false, true, false]];
/// What eaters could burn of what they took, by source.
const ATE: [&str; 3] = ["ate_light_eaters", "ate_others", "ate_ground"];

/// One starting kind over the window: alive at its end; means of its mass (kg), real life (yr), member mass (kg), each
/// of `GENES` less its founders' value, its share of members in young bodies, and their median member body over the
/// copy mass.
#[derive(Clone, Default)]
struct Kind {
    alive: bool,
    kg: f64,
    life: f64,
    member: f64,
    moves: Vec<f64>,
    young: f64,
    young_size: f64,
}

/// A run over its window: its kinds, and the order of what its eaters ate (most first).
struct Window {
    kinds: Vec<Kind>,
    ate: [usize; 3],
}

/// One year of a run in its window: per kind its members, mass, real life, member mass, the young (`young`) and the
/// `GENES` (from `GENE_AT`); and what its eaters ate (`ATE`).
struct Year {
    kinds: Vec<Vec<f64>>,
    ate: [f64; 3],
}

/// Where a kind's genes start in a row of `Year::kinds`.
const GENE_AT: usize = 6;

/// The young of kind `k` at the end of a year: the share of its members in immature bodies (`Body::adult`), nodes or
/// individuals, and the median by members of their member body over their copy mass (NaN without young).
fn young(w: &World, k: u32) -> (f64, f64) {
    let (mut all, mut sizes) = (0.0, vec![]);
    for b in w.life.alive().filter(|b| w.life.brains[b.lineage as usize].as_ref().is_some_and(|l| l.root == k)) {
        all += b.n();
        if !b.adult(&w.chem) {
            sizes.push((b.member_body(&w.chem) / b.dec(COPY_MASS), b.n()));
        }
    }
    sizes.sort_by(|a, b| a.0.total_cmp(&b.0));
    let young: f64 = sizes.iter().map(|s| s.1).sum();
    let mut below = 0.0;
    let median = sizes.iter().find(|s| {
        below += s.1;
        below >= young / 2.0
    });
    (young / all.max(1e-300), median.map_or(f64::NAN, |s| s.0))
}

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
    run(seed, SIDE, window.1, roster, |c| (c.nodes, c.crowd) = (nodes, crowd), |y, w, rec| {
        let col = |k: usize, name: &str| rec.last(&format!("root_{k}_{name}"));
        if unit.is_empty() {
            unit = copy_units(w);
        }
        founders.resize(kinds(rec), None);
        for (k, f) in founders.iter_mut().enumerate().filter(|(k, f)| f.is_none() && col(*k, "members") > 0.0) {
            *f = Some(GENES.iter().map(|g| col(k, g.0)).collect());
        }
        if y >= window.0 {
            let row = |k: usize| {
                let (share, size) = young(w, k as u32);
                [col(k, "members"), col(k, "kg"), col(k, "life_real"), col(k, "mass"), share, size].into_iter().chain(GENES.iter().map(|g| col(k, g.0))).collect()
            };
            years.push(Year { kinds: (0..kinds(rec)).map(row).collect(), ate: ATE.map(|c| rec.last(c)) });
        }
    });
    let last = &years.last().expect("a year").kinds;
    let kinds = (0..last.len())
        .map(|k| {
            let m = |q: usize| mean(years.iter().filter_map(|y| y.kinds.get(k).map(|v| v[q])));
            let start = founders[k].clone().unwrap_or_default();
            let moves = (0..GENES.len()).map(|g| (m(GENE_AT + g) - start.get(g).copied().unwrap_or(f64::NAN)) * unit[g]).collect();
            Kind { alive: last[k][0] > 0.0, kg: m(1), life: m(2), member: m(3), young: m(4), young_size: m(5), moves }
        })
        .collect();
    let eaten: Vec<f64> = (0..ATE.len()).map(|c| mean(years.iter().map(|y| y.ate[c]))).collect();
    let mut ate = [0, 1, 2];
    ate.sort_by(|&a, &b| eaten[b].total_cmp(&eaten[a]));
    Window { kinds, ate }
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

/// One thing the three rules compare between two runs: its rule (0, 1, 2 for a, b, c), what it is, the runs' values,
/// and how far apart the runs are on it: for rule (a) the log of the ratio (infinite where the kind lives in one run
/// only), for (b) and (c) 1 where they disagree, else 0.
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
    }
    let ((mx, hx), (my, hy)) = (winners(x), winners(y));
    push(2, "most mass".into(), format!("kind {mx:?} against {my:?}"), (mx != my) as u8 as f64);
    push(2, "heaviest member".into(), format!("kind {hx:?} against {hy:?}"), (hx != hy) as u8 as f64);
    let (ox, oy) = (x.ate.map(|c| ATE[c]), y.ate.map(|c| ATE[c]));
    push(2, "what eaters eat".into(), format!("{ox:?} against {oy:?}"), (ox != oy) as u8 as f64);
    d
}

/// The rule a difference breaks: past a factor of `FACTOR` for rule (a), any for (b) and (c).
fn breaks(d: &Diff) -> bool {
    d.size > if d.rule == 0 { FACTOR.ln() } else { 0.0 }
}

/// Prints run `r`'s kinds alive, with their census of the young and the genes that moved, and its winners.
fn print(name: &str, r: &Window) {
    for (k, d) in r.kinds.iter().enumerate().filter(|(_, d)| d.alive) {
        let moves: Vec<String> = GENES.iter().zip(&d.moves).filter(|(_, m)| m.abs() > MOVE).map(|(g, m)| format!("{} {m:+.2}", g.0)).collect();
        println!("  {name} kind {k}: {:.3e} kg, life {:.2} yr, member {:.3e} kg, young {:.1}% at {:.2} of the copy mass; moved {moves:?}", d.kg, d.life, d.member, 100.0 * d.young, d.young_size);
    }
    println!("  {name}: most mass, heaviest member {:?}; eaters ate most to least {:?}", winners(r), r.ate.map(|c| ATE[c]));
}

/// Nodes against individuals, by the three rules of handover 7.1, against the noise of the crowd setting.
#[test]
#[ignore = "slow: ten runs of a 32 x 32 world, 50 and 30 years"]
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
            for (rule, label) in ["a", "b", "c"].iter().enumerate() {
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
