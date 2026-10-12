//! Law 5 scene: one body learns to move toward what it can take. Flat ground, the energy-rich
//! solid lying loose only in a strip on the east side, one body (of the starting kind that burns
//! that solid and takes it from the ground) just west of the strip, with a neutral brain. Over
//! its first 200 decisions, with learning on against off, over many seeds.

use pcore::body::{self, INTERVAL, LEARN};
use pcore::grid::DIRS;
use pcore::rng::{Purpose, Rng};
use pcore::{bond, brain, change, choose, flow, World};

const W: usize = 12;
const H: usize = 8;
/// First column of the strip, and the loose energy-rich solid on each of its cells, mol/m².
const STRIP: usize = 6;
const RICH: f64 = 20.0;
const DECISIONS: usize = 200;
const SEEDS: u64 = 120;
/// Learning rate genes (log10): on, and off.
const ON: f32 = -1.0;
const OFF: f32 = -4.0;

/// A flat, warm world: every cell a copy of one dry land cell, the strip added on top.
fn scene(seed: u64) -> (World, usize) {
    let (chem, init) = worldgen::chemistry0();
    let cfg = pcore::Config { lat_south: 5.0, lat_north: 10.0, ..Default::default() };
    let mut w = worldgen::terrain::build(seed, W, H, chem, &init, cfg);
    pcore::step(&mut w);
    let (n, ns) = (w.grid.n(), w.chem.ns());
    let land = (0..n).find(|&i| w.life.env[i][2] > 0.0 && w.life.env[i][1] < 1e-3).expect("dry land");
    for k in 0..ns {
        let (g, b) = (w.ground[k * n + land], w.bed[k * n + land]);
        (0..n).for_each(|i| (w.ground[k * n + i], w.bed[k * n + i]) = (g, b));
    }
    let e = w.e_ground[land];
    w.e_ground.iter_mut().for_each(|x| *x = e);
    let (_, rich, _, _) = worldgen::life::roles(&w.chem).expect("roles");
    for i in (0..n).filter(|&i| i % W >= STRIP) {
        w.ground[rich * n + i] += RICH * w.grid.area();
    }
    w.life.env.clear();
    body::survey(&mut w);
    (w, rich)
}

/// What a body perceives of the energy-rich solid in cell `i`.
fn seen(w: &World, rich: usize, i: usize) -> f32 {
    w.life.seen[i * w.chem.ns() + rich]
}

/// Per decision: on the strip; moved; a neighbour was richer; moved to a richer one; the share of
/// richer neighbours (what a move in a random direction would give).
type Decision = (bool, bool, bool, bool, f64);

fn run(seed: u64, learn: f32) -> Vec<Decision> {
    let (mut w, rich) = scene(seed);
    // A neutral brain: small random weights, no bias among resting, putting, taking from the
    // ground and moving (which all start equally likely); copying, the actions on parts and on
    // other bodies kept rare, as they are not part of this scene.
    let mut r = Rng::new(seed, 0, 0, Purpose::Body);
    let fixed = [0.0, -8.0, 0.0, 0.0, -8.0, -8.0, -8.0, -8.0];
    let shape = choose::shape(w.chem.ns());
    let mut brain = brain::random(shape, &mut r, 0.1, fixed, 0.0, [-8.0; brain::VERBS + brain::OVERBS]);
    let rows = brain.split_off(shape.base());
    let lineage = w.life.new_lineage(brain);
    let start = w.grid.idx(STRIP - 1, H / 2);
    let mut b = worldgen::life::make(&mut w, 1, start, lineage, &rows).expect("body");
    // Decisions every 5 days (the shortest interval).
    (b.genes[LEARN], b.genes[INTERVAL]) = (learn, 0.7);
    let slot = w.life.add(b, w.day, 1);
    let mut log = Vec::new();
    // The days of `pcore::step`, one by one, to see each decision.
    let dt = w.cfg.phys_days as f64 * 86_400.0;
    while log.len() < DECISIONS && w.life.bodies[slot].alive {
        let d = w.day % w.cfg.days_per_month as u64;
        if d.is_multiple_of(w.cfg.phys_days as u64) {
            body::survey(&mut w);
            flow::step(&mut w, dt);
            change::step(&mut w, dt);
        }
        if d == 0 {
            bond::step(&mut w);
        }
        let (u, from) = (w.life.updates, w.life.bodies[slot].cell as usize);
        let near: Vec<usize> = DIRS.iter().filter_map(|&d| w.grid.step(from, d)).collect();
        let richer: Vec<usize> = near.iter().copied().filter(|&j| seen(&w, rich, j) > seen(&w, rich, from) + 0.02).collect();
        body::day(&mut w);
        w.day += 1;
        if w.life.updates > u && w.life.bodies[slot].alive {
            let to = w.life.bodies[slot].cell as usize;
            log.push((from % W >= STRIP, to != from, !richer.is_empty(), richer.contains(&to), richer.len() as f64 / near.len() as f64));
        }
    }
    log
}

/// Over all seeds: share of decisions made on the strip; of the moves made from a cell next to a
/// richer one, the share to a richer cell, and what random moves would give; decisions; those
/// moves.
fn measure(learn: f32) -> (f64, f64, f64, usize, usize) {
    let (mut on, mut n, mut moves, mut toward, mut chance) = (0, 0, 0usize, 0, 0.0);
    for seed in 1..=SEEDS {
        for (strip, moved, next, up, share) in run(seed, learn) {
            (on, n) = (on + strip as usize, n + 1);
            if moved && next {
                (moves, toward, chance) = (moves + 1, toward + up as usize, chance + share);
            }
        }
    }
    (on as f64 / n as f64, toward as f64 / moves as f64, chance / moves as f64, n, moves)
}

#[test]
fn body_learns_to_move_toward_what_it_can_take() {
    let (strip_on, toward_on, chance_on, n_on, m_on) = measure(ON);
    let (strip_off, toward_off, chance_off, n_off, m_off) = measure(OFF);
    let report = format!(
        "learning on: on the strip {strip_on:.3}, toward {toward_on:.3} (chance {chance_on:.3}), {n_on} decisions, {m_on} moves; \
         off: on the strip {strip_off:.3}, toward {toward_off:.3} (chance {chance_off:.3}), {n_off} decisions, {m_off} moves"
    );
    eprintln!("{report}");
    // It ends up on the strip clearly more often than a body that does not learn (about 0.74
    // against 0.51 here; 0.70 against 0.53 and 0.76 against 0.48 on seeds 121 to 360).
    assert!(strip_on > strip_off + 0.07, "{report}");
    // Its moves go toward richer cells clearly more often than chance, and clearly more than
    // those of a body that does not learn (about 14 points over chance against 1 here; 15
    // against -1 and 16 against -1 on seeds 121 to 360). Moving pays only through a take at a
    // later decision; it is learned because each row of the brain steps by its own size, so the
    // row for the richness of a neighbour learns when that matters, not drowned by the others.
    let (up_on, up_off) = (toward_on - chance_on, toward_off - chance_off);
    assert!(up_on > 0.05 && up_on > up_off + 0.05, "{report}");
}
