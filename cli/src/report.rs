//! The lines a run prints: what it is, the liquid's start, one line every `--every` years, the rivers and the speed.

use pcore::check::Totals;
use pcore::{World, AIR};
use tape::{Recorder, Spent, RIVER_CATCH, RIVER_FLOOR, TAPE};
use worldgen::life::Seeding;
use pcore::store::Class;
use std::time::Duration;

/// Which run this is: its seed and size, and how its cover is placed.
pub fn config(w: &World, how: &Seeding) -> String {
    format!("config: seed {}, {}x{}, start: the cover with {} rounds of mutation, {} of it fixers", w.seed, w.map.w, w.map.h, how.rounds, how.fixers)
}

/// The heat that came in at each open edge of the map since the start (`Counters::edges`), W per m of the edge it stands
/// for (the stretch times the map's width); none where the edges are closed.
pub fn edges(w: &World) -> Option<String> {
    w.planet.as_ref()?;
    let per_m = |j: f64| j / w.time.max(1) as f64 * w.stretch() / (w.map.w as f64 * w.map.side);
    let [north, south, east, west] = w.counts.edges.map(per_m);
    Some(format!("edges: heat in at the north {north:.3e}, south {south:.3e}, east {east:.3e}, west {west:.3e} W/m (sensible and latent, mean since the start)"))
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

/// The time spent so far, by stage (flow, change, the cover) and in the tape (the recorder each month, the snapshot and
/// the files each time they are written, and the pictures); and the year of the last yearly line, with the time spent
/// then.
pub struct Clock {
    pub spent: Spent,
    printed: Spent,
    since: u64,
}

impl Clock {
    pub fn new(since: u64) -> Self {
        Clock { spent: Spent::default(), printed: Spent::default(), since }
    }

    /// The time spent so far in all stages and the tape.
    pub fn total(&self) -> Duration {
        self.spent.iter().sum()
    }

    /// Start the next line's span at year `y`.
    pub fn print_at(&mut self, y: u64) {
        (self.printed, self.since) = (self.spent, y);
    }
}

/// The yearly line: drift since `t0` (the worst, and of each element signed), the cover, the takers by starting kind
/// (members and living kg, where any were placed), and the speed by stage.
pub fn year(w: &World, y: u64, t0: &Totals, clock: &Clock) -> String {
    let ((de, dq), now) = (pcore::check::drift(w, t0), pcore::check::totals(w));
    let each: Vec<String> = (0..now.elems.len()).map(|j| format!("{:+.1e}", pcore::check::elem_drift(&now, t0, j))).collect();
    let years = (y - clock.since) as f64;
    let ms: Vec<f64> = clock.spent.iter().zip(&clock.printed).map(|(now, before)| (*now - *before).as_secs_f64() * 1000.0 / years).collect();
    let cover = pcore::cover::sums(w);
    let (kg, slots, kinds) = (cover.iter().map(|s| s.kg).sum::<f64>(), cover.iter().map(|s| s.slots).sum::<f64>(), cover.iter().filter(|s| s.slots > 0.0).count());
    let takers = pcore::fauna::sums(w);
    let takers = if takers.is_empty() { String::new() } else { format!(", takers {}", takers.iter().map(|[n, kg]| format!("{n:.0}/{kg:.2e} kg")).collect::<Vec<_>>().join(" ")) };
    format!(
        "year {y}: element drift {de:.2e} ({}), energy drift {dq:.2e}, potential energy gained {:.3e} J, heat not given {:.2e} J, rounded to zero {:.1e} mol {:.1e} J, cover {kg:.3e} kg in {slots} slots of {kinds} starting kinds{takers}; ms/yr flow {:.0}, change {:.0}, cover {:.0}, tape {:.0}",
        each.join(" "),
        w.store.lifted(),
        w.counts.clamped_heat,
        w.counts.rounded[0],
        w.counts.rounded[1],
        ms[0],
        ms[1],
        ms[2],
        ms[TAPE]
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
/// (`settled`, the time spent in the years after `settled_after`, without the writes of the end), the cells, and the
/// world's fingerprint.
pub fn speed(w: &World, years: u64, total: Duration, settled: Duration) -> String {
    let per_year = |t: Duration, n: u64| t.as_secs_f64() * 1000.0 / n.max(1) as f64;
    let last = settled_years(years);
    format!(
        "{years} years in {:.2} s ({:.1} ms per year; {:.1} ms per year over the last {last}), {} cells, hash {:016x}",
        total.as_secs_f64(),
        per_year(total, years),
        per_year(settled, last),
        w.mesh.cells().n(),
        tape::fingerprint(w)
    )
}

/// Rivers as every viewer draws them (`tape::is_river`: dry land cells that carry more than `RIVER_CATCH` times what
/// their own area sheds and more than `RIVER_FLOOR`), the largest catchment on land, and the mean rain on land.
pub fn rivers(w: &World, rec: &Recorder) -> String {
    let top = (0..w.mesh.cells().n()).filter(|&i| w.is_land(i)).map(|i| w.catchment(i)).fold(0.0, f64::max);
    let n = tape::river_count(w, &tape::fields(w));
    format!("rivers: {n} dry land cells over {RIVER_CATCH} cells' own area and {RIVER_FLOOR:.0e} mol/s, largest catchment {top:.3e} cells' own area, mean land rain {:.3e} mol/m2/s", rec.last("rain_land"))
}
