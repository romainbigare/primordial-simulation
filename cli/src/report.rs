//! The lines a run prints: what it is, the liquid's start, one line every `--every` years, the rivers and the speed.

use pcore::check::Totals;
use pcore::{World, AIR};
use tape::{Recorder, Spent, REFINE, RIVER_CATCH, RIVER_FLOOR, TAPE};
use worldgen::life::Seeding;
use pcore::store::Class;
use std::time::Duration;

/// Which run this is: whether it is the real run or its null run (a resumed run keeps the snapshot's), its switches,
/// and which start the animals and the light-eaters had.
pub fn config(w: &World, how: &Seeding) -> String {
    let run = if w.cfg.null { "NULL RUN (learning off, imitation off, brain rows frozen)" } else { "real run" };

    let animals = if how.hand { "HAND-SET eater biases (default; --neutral-eaters for neutral brains)" } else { "NEUTRAL brains (--neutral-eaters)" };
    format!("config: seed {}, {}x{}, {run}, start: animals {animals}, light-eaters as cover", w.seed, w.map.w, w.map.h)
}

/// The liquid's store at the start, set at balance with the air (`Reactor::balance`): what each reversible pair makes,
/// in the ground, and the gas it holds over the air's.
pub fn balances(w: &World) -> Vec<String> {
    let (n, na) = (w.mesh.cells().n(), w.mesh.level(AIR).n());
    let mut out = vec![];
    for (f, _, _, u) in pcore::change::reversible(&w.chem) {
        let held: f64 = w.store.amounts(Class::Ground)[u * n..(u + 1) * n].iter().sum();
        let cells = w.store.amounts(Class::Ground)[u * n..(u + 1) * n].iter().filter(|&&x| x > 0.0).count();
        if let Some(&(g, c)) = w.chem.reactions[f].inputs.iter().find(|p| w.chem.gas_at_surface(p.0 as usize)) {
            let air: f64 = w.store.amounts(Class::Air)[g as usize * na..(g as usize + 1) * na].iter().sum();
            out.push(format!("start: substance {u} held at balance with the air in {cells} of {n} cells, {held:.3e} mol, {:.2} times the air's substance {g} ({air:.3e} mol)", held * c as f64 / air));
        }
    }
    out
}

/// The time spent so far, by stage (flow, change, bodies, parts), in refinement and in the tape (the recorder each
/// month, the snapshot and the files each time they are written, and the pictures); and the year and body updates
/// counted at the last yearly line, with the time spent then.
pub struct Clock {
    pub spent: Spent,
    printed: Spent,
    since: u64,
    updates: u64,
}

impl Clock {
    pub fn new(since: u64, updates: u64) -> Self {
        Clock { spent: Spent::default(), printed: Spent::default(), since, updates }
    }

    /// The time spent so far in all stages and the tape.
    pub fn total(&self) -> Duration {
        self.spent.iter().sum()
    }

    /// Start the next line's span at year `y`.
    pub fn print_at(&mut self, y: u64, updates: u64) {
        (self.printed, self.since, self.updates) = (self.spent, y, updates);
    }
}

/// The yearly line: drift since `t0` (the worst, and of each element signed), bodies and members, speed, and the counts
/// of the year.
pub fn year(w: &World, y: u64, t0: &Totals, clock: &Clock, rec: &Recorder) -> String {
    let ((de, dq), now) = (pcore::check::drift(w, t0), pcore::check::totals(w));
    let each: Vec<String> = (0..now.elems.len()).map(|j| format!("{:+.1e}", pcore::check::elem_drift(&now, t0, j))).collect();
    let all = w.life.alive().count();
    // Individuals, nodes and members.
    let mut m = [0.0; 3];
    for b in w.life.alive() {
        let k = (b.n() > 1.0) as usize;
        (m[k], m[2]) = (m[k] + 1.0, m[2] + b.n());
    }
    let years = (y - clock.since) as f64;
    let ms: Vec<f64> = clock.spent.iter().zip(&clock.printed).map(|(now, before)| (*now - *before).as_secs_f64() * 1000.0 / years).collect();
    // Microseconds per body update: the cost of one decision, whatever their number (none without updates).
    let updates = w.counts.updates - clock.updates;
    let us = if updates > 0 { format!("{:.2} us/update", ms[2] * 1e3 * years / updates as f64) } else { String::from("no updates") };
    // The fullest cell (the first of equals): its bodies and the starting kinds among them.
    let fullest = (0..w.mesh.cells().n()).max_by_key(|&i| (w.life.in_cell(i), std::cmp::Reverse(i)));
    let most = fullest.map_or(0, |i| w.life.in_cell(i));
    let mut roots: Vec<u32> = fullest.map_or(vec![], |i| w.life.cell(i).filter_map(|h| w.life.brain(w.life[h].lineage).map(|l| l.root)).collect());
    roots.sort_unstable();
    roots.dedup();
    let most_kinds = roots.len();
    let born = w.life.genomes.iter().filter(|g| !g.rows.is_empty()).count();
    let cover = pcore::cover::sums(w);
    let (kg, slots, kinds) = (cover.iter().map(|s| s.kg).sum::<f64>(), cover.iter().map(|s| s.slots).sum::<f64>(), cover.iter().filter(|s| s.slots > 0.0).count());
    format!(
        "year {y}: element drift {de:.2e} ({}), energy drift {dq:.2e}, potential energy gained {:.3e} J, heat not given {:.2e} J, rounded to zero {:.1e} mol {:.1e} J, cover {kg:.3e} kg in {slots} slots of {kinds} starting kinds, bodies {all} (at most {most} in a cell, of {most_kinds} kinds; {} single, {} nodes, {:.3e} members), lineages {}, born brains {born}; ms/yr flow {:.0}, change {:.0}, bodies {:.0}, parts {:.1}, refine {:.0}, tape {:.0}; {us} ({} parts); gifts {}/yr, watches {}/yr",
        each.join(" "),
        w.store.lifted(),
        w.counts.clamped_heat,
        w.counts.rounded[0],
        w.counts.rounded[1],
        m[0],
        m[1],
        m[2],
        w.life.lineages(),
        ms[0],
        ms[1],
        ms[2],
        ms[3],
        ms[REFINE],
        ms[TAPE],
        w.life.parts().count(),
        rec.last("gave"),
        rec.last("watched")
    )
}

/// Years that make the settled cost of a run: a tenth of its years (at least one).
fn settled_years(years: u64) -> u64 {
    (years / 10).max(1).min(years)
}

/// The year after which the settled cost of a run of `years` years from year `y0` is timed.
pub fn settled_after(y0: u64, years: u64) -> u64 {
    y0 + years - settled_years(years)
}

/// The run's last line: the years, the time spent (`total`), the cost a year over the whole run and over its last tenth
/// (`settled`, the time spent in the years after `settled_after`, without the writes of the end), the cells and the bodies at the end, and the world's fingerprint.
pub fn speed(w: &World, years: u64, total: Duration, settled: Duration) -> String {
    let per_year = |t: Duration, n: u64| t.as_secs_f64() * 1000.0 / n.max(1) as f64;
    let last = settled_years(years);
    format!(
        "{years} years in {:.2} s ({:.1} ms per year; {:.1} ms per year over the last {last}), {} cells, {} bodies, hash {:016x}",
        total.as_secs_f64(),
        per_year(total, years),
        per_year(settled, last),
        w.mesh.cells().n(),
        w.life.alive().count(),
        tape::fingerprint(w)
    )
}

/// Rivers as every viewer draws them (`tape::is_river`: dry land cells that carry more than `RIVER_CATCH` times what
/// their own area sheds and more than `RIVER_FLOOR`), the largest catchment on land, and the mean rain on land.
pub fn rivers(w: &World, rec: &Recorder) -> String {
    let top = (0..w.mesh.cells().n()).filter(|&i| w.is_land(i)).map(|i| tape::catchment(w, i)).fold(0.0, f64::max);
    let n = tape::river_count(w, &tape::fields(w));
    format!("rivers: {n} dry land cells over {RIVER_CATCH} cells' own area and {RIVER_FLOOR:.0e} mol/s, largest catchment {top:.3e} cells' own area, mean land rain {:.3e} mol/m2/s", rec.last("rain_land"))
}
