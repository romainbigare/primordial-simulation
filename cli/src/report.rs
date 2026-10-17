//! The lines a run prints: what it is, the liquid's start, one line every `--every` years, the rivers and the end.

use crate::render::{RIVER_PNG, RIVER_WEB, STANDING_M};
use pcore::body::light_eater;
use pcore::World;
use std::time::Duration;
use tape::Recorder;
use worldgen::life::Seeding;

/// Which run this is: whether it is the real run or its null run (a resumed run keeps the snapshot's), its switches,
/// and which start the animals and the light-eaters had.
pub fn config(w: &World, how: &Seeding) -> String {
    let run = if w.cfg.null { "NULL RUN (learning off, imitation off, brain rows frozen)" } else { "real run" };
    let nodes = if w.cfg.nodes { "on (animals may be nodes)" } else { "OFF (--individuals: animals are individuals, light-eaters nodes)" };
    let parents = if w.cfg.two_parents { "on (a mature alike body of the cell is a second parent)" } else { "OFF (--no-mix: one parent)" };
    let animals = if how.hand[1] { "HAND-SET eater biases (default; --neutral-eaters for neutral brains)" } else { "NEUTRAL brains (--neutral-eaters)" };
    let plants = if how.hand[0] { "hand-set biases (--plant-bias)" } else { "neutral brains (default)" };
    format!("config: seed {}, {}x{}, {run}, nodes {nodes}, two parents {parents}, start: animals {animals}, plants {plants}", w.seed, w.grid.w, w.grid.h)
}

/// The liquid's store at the start, set at balance with the air (`Reactor::balance`): what each reversible pair makes,
/// in the ground, and the gas it holds over the air's.
pub fn balances(w: &World) -> Vec<String> {
    let (n, na) = (w.grid.n(), w.grid.na());
    let mut out = vec![];
    for (f, _, _, u) in pcore::change::reversible(&w.chem) {
        let held: f64 = w.ground[u * n..(u + 1) * n].iter().sum();
        let cells = w.ground[u * n..(u + 1) * n].iter().filter(|&&x| x > 0.0).count();
        if let Some(&(g, c)) = w.chem.reactions[f].inputs.iter().find(|p| w.chem.gas_at_surface(p.0 as usize)) {
            let air: f64 = w.air[g as usize * na..(g as usize + 1) * na].iter().sum();
            out.push(format!("start: substance {u} held at balance with the air in {cells} of {n} cells, {held:.3e} mol, {:.2} times the air's substance {g} ({air:.3e} mol)", held * c as f64 / air));
        }
    }
    out
}

/// Time spent since the last yearly line, by stage (flow, change, bodies, parts) and in the tape, and the body
/// updates counted at it.
pub struct Clock {
    pub stages: [Duration; 4],
    pub tape: Duration,
    pub since: u64,
    pub updates: u64,
}

/// The yearly line: drift, bodies and members, speed, and the counts of the year.
pub fn year(w: &World, y: u64, drift: (f64, f64), clock: &Clock, rec: &Recorder) -> String {
    let (de, dq) = drift;
    let (all, le) = (w.life.alive().count(), w.life.alive().filter(|b| light_eater(&w.chem, &b.genes)).count());
    // Members of light-eaters; individuals, nodes and members of the others.
    let mut m = [0.0; 4];
    for b in w.life.alive() {
        let k = if light_eater(&w.chem, &b.genes) { 0 } else { 1 + (b.n() > 1.0) as usize };
        (m[k], m[3]) = (m[k] + if k == 0 { b.n() } else { 1.0 }, m[3] + if k > 0 { b.n() } else { 0.0 });
    }
    let years = (y - clock.since) as f64;
    let ms = clock.stages.map(|d| d.as_secs_f64() * 1000.0 / years);
    // Microseconds per body update: the cost of one decision, whatever their number.
    let us = clock.stages[2].as_secs_f64() * 1e6 / (w.life.updates - clock.updates).max(1) as f64;
    let ms_tape = clock.tape.as_secs_f64() * 1000.0 / years;
    let most = w.life.at.iter().map(|v| v.len()).max().unwrap_or(0);
    let born = w.life.genomes.iter().filter(|g| !g.rows.is_empty()).count();
    format!(
        "year {y}: element drift {de:.2e}, energy drift {dq:.2e}, heat not given {:.2e} J, bodies {all} (at most {most} in a cell; {le} light-eaters with {:.3e} members; others {} single, {} nodes, {:.3e} members), lineages {}, born brains {born}; ms/yr flow {:.0}, change {:.0}, bodies {:.0}, parts {:.1}, tape {ms_tape:.0}; {us:.2} us/update ({} parts); node splits {:.0} in two, {:.0} one off; kinds {} ({} by members), gifts {}/yr, watches {}/yr",
        w.ledger.clamped_heat,
        m[0],
        m[1],
        m[2],
        m[3],
        w.life.lineages(),
        ms[0],
        ms[1],
        ms[2],
        ms[3],
        w.parts.alive().count(),
        w.diag.splits[0],
        w.diag.splits[1],
        rec.last("kinds"),
        rec.last("kinds_members"),
        rec.last("gave"),
        rec.last("watched")
    )
}

/// Rivers as the viewers draw them (dry land cells with a liquid flux over each viewer's threshold), and the mean
/// rain on land.
pub fn rivers(w: &World) -> String {
    let f = tape::fields(w);
    let flux = |i: usize| w.liquid_flux.get(i).copied().unwrap_or(0.0) as f64;
    let land: Vec<usize> = (0..w.grid.n()).filter(|&i| f.z[i] > 0.0).collect();
    let dry: Vec<usize> = land.iter().copied().filter(|&i| f.depth[i] <= STANDING_M).collect();
    let over = |t: f64| dry.iter().filter(|&&i| flux(i) > t).count();
    let rain = land.iter().map(|&i| w.rain[i] as f64).sum::<f64>() / land.len().max(1) as f64;
    let top = dry.iter().map(|&i| flux(i)).fold(0.0, f64::max);
    format!("rivers: max flux {top:.3e} mol/s on land, dry land cells over {RIVER_WEB:.0e} (web) {}, over {RIVER_PNG:.0e} (png) {}, mean land rain {rain:.3e} mol/m2/s", over(RIVER_WEB), over(RIVER_PNG))
}
