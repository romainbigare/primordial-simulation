//! The recorder, driven once per simulated month by the shell. It counts the world's events (`tally`), keeps a row
//! every month (`months`) and writes a row of the yearly series every year (`cols`), keeps viewer frames, and writes it
//! all to a folder; the viewer's file (`ewf`) holds what a resumed run needs to go on.

mod cols;
mod csv;
mod ewf;
mod months;
mod tally;

use crate::layers::{frame, Ctx};
use pcore::par;
use months::MonthRow;
use pcore::event::{Event, EventKind};
use pcore::{Taken, World, AIR};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io;
use std::path::Path;
use std::time::Duration;
pub use csv::Written;
pub use ewf::read_viewer_file;
use tally::YearTally;
use pcore::store::Class;

/// Causes of death in the order of the core's constants (`life::LACK` .. `life::FLAME`).
const CAUSES: [&str; pcore::life::DEATHS] = ["lack", "phase", "spent", "taken", "broken", "age", "drowned", "flame"];

/// The time the shell has spent in the run so far, by process: Flow, Change, the cover and the tape itself (`TIMED`). The recorder reads no clock; the shell measures and hands the totals in.
pub type Spent = [Duration; TIMED.len()];

/// The processes timed, in the order of `Spent`; the yearly columns are `ms_<process>`.
const TIMED: [&str; 4] = ["flow", "change", "cover", "tape"];

/// The place in `Spent` of the tape's own time.
pub const TAPE: usize = TIMED.len() - 1;

/// One column of the yearly series: its name, unit and a value a year (NaN: none that year).
#[derive(Clone)]
struct Column {
    name: String,
    unit: String,
    values: Vec<f64>,
}

/// The world a tape is of: width and height in cells, cell size (m), whether it is a globe (its rows wrap east to west)
/// and the latitudes of its north and south edges (degrees; the rows are evenly spread between them), seed and days per year.
#[derive(Clone, Copy)]
struct Shape {
    w: usize,
    h: usize,
    cell_m: f64,
    globe: bool,
    lat_north: f64,
    lat_south: f64,
    seed: u64,
    days_per_year: u64,
}

/// What the recorder keeps across years and a snapshot carries (`Recorder::kept`), so that a resumed run's series
/// go on as the unbroken run's: the substances seen so far.
#[derive(Serialize, Deserialize)]
pub struct Kept {
    seen: Vec<bool>,
}

impl Kept {
    /// What a recorder that starts on world `w` keeps: no substance seen yet.
    pub fn new(w: &World) -> Kept {
        Kept { seen: vec![false; w.chem.ns()] }
    }
}

/// What a tape's files hold: the viewer's file (frames, events, the series and the months), `physics.json` and
/// `series.csv`. The recorder keeps it as it goes (`Recorder::tape` copies it for another thread to
/// write).
#[derive(Clone)]
pub struct TapeFiles {
    shape: Shape,
    /// The day of each frame kept, and their bytes (`layers::frame`)
    frames: Vec<u64>,
    bytes: Vec<u8>,
    series: Vec<Column>,
    /// The events the viewer shows: day and text
    log: Vec<(u64, String)>,
    months: Vec<MonthRow>,
    /// The run's `Physics` and `Config` as JSON text, written next to the series (`physics.json`)
    physics: String,
}

pub struct Recorder {
    /// Frames are kept every `every` years, and every month in the first `monthly` years
    every: u64,
    monthly: u64,
    /// The substances found anywhere so far (`detect`)
    seen: Vec<bool>,
    files: TapeFiles,
    /// The sums of the year so far
    year: YearTally,
    /// The time spent by process at the last yearly row (`Spent`)
    spent: Spent,
}

/// The run's numbers, `{"physics": {..}, "config": {..}}`, keys sorted, one line each, so two runs compare with a diff.
fn physics_json(w: &World) -> String {
    let mut cfg = serde_json::to_value(&w.cfg).unwrap_or_default();
    if let Some(o) = cfg.as_object_mut() {
        o.remove("phys");
    }
    let doc = json!({"physics": w.cfg.phys, "config": cfg});
    serde_json::to_string_pretty(&doc).unwrap_or_default() + "\n"
}

impl Recorder {
    /// Frames are kept every `every` years, and every month in the first `monthly` years.
    pub fn new(w: &World, every: u64, monthly: u64) -> Self {
        let g = &w.map;
        let shape = Shape { w: g.w, h: g.h, cell_m: g.side, globe: g.wrap, lat_north: w.cfg.lat_north, lat_south: w.cfg.lat_south, seed: w.seed, days_per_year: w.days_per_year() };
        Recorder {
            every,
            monthly,
            seen: Kept::new(w).seen,
            files: TapeFiles { shape, frames: vec![], bytes: vec![], series: vec![], log: vec![], months: vec![], physics: physics_json(w) },
            year: YearTally::default(),
            spent: Spent::default(),
        }
    }

    /// What the recorder keeps across years, for a snapshot (taken at a year's end, after its row).
    pub fn kept(&self) -> Kept {
        Kept { seen: self.seen.clone() }
    }

    /// Start a run resumed from a snapshot: take back what the recorder kept, and load what an earlier run wrote to `dir`
    /// up to the snapshot's day, so the files go on from there (a `dir` without a tape starts it empty).
    pub fn resume(&mut self, w: &World, kept: Kept, dir: impl AsRef<Path>) -> io::Result<()> {
        self.seen = kept.seen;
        self.read_ewf(dir.as_ref(), w)
    }

    /// A substance with a non-zero amount anywhere that was zero everywhere before.
    fn detect(&mut self, w: &World) {
        let (n, na) = (w.mesh.cells().n(), w.mesh.level(AIR).n());
        for k in 0..self.seen.len() {
            if self.seen[k] {
                continue;
            }
            let hit = [(&w.store.amounts(Class::Ground), n, 0), (&w.store.amounts(Class::Bed), n, 0), (&w.store.amounts(Class::Air), na, n)]
                .iter()
                .find_map(|&(v, len, off)| v[k * len..(k + 1) * len].iter().position(|&x| x > 0.0).map(|p| (off + p, v[k * len + p])));
            if let Some((node, value)) = hit {
                self.seen[k] = true;
                self.count(&Event { day: w.day(), kind: EventKind::FirstSubstance(k as u16), node: node as u32, value });
            }
        }
    }

    /// Whether the world is at a year's end, when `record_month` wants the yearly counts (`World::take_counters`).
    pub fn wants_counters(w: &World) -> bool {
        w.time.is_multiple_of(w.cfg.calendar().year)
    }

    /// Call at the start and after every month with the world's events, at a year's end its counts, and the time spent
    /// so far. A month the tape has already (a resumed run's first) is not recorded again.
    pub fn record_month(&mut self, w: &World, events: Vec<Event>, taken: Option<Taken>, spent: &Spent) {
        let cal = w.cfg.calendar();
        let (year, yearly) = (cal.years(w.time), w.time.is_multiple_of(cal.year));
        if self.files.months.last().is_some_and(|r| (r.year, r.month) == months::year_month(w)) {
            return;
        }
        let framed = year < self.monthly || (yearly && self.every > 0 && year % self.every == 0);
        // The recorder counts the events and the month while the world's fields are worked out.
        let (_, ctx) = par::join(
            || {
                for e in &events {
                    self.count(e);
                }
                self.detect(w);
                self.month(w);
            },
            || (yearly || framed).then(|| Ctx::new(w)),
        );
        let Some(c) = ctx else { return };
        if let Some(t) = taken.filter(|_| yearly) {
            (self.year.cover_lost, self.year.cover_budget, self.year.seeds, self.year.flame, self.year.flame_air) = (t.cover_lost, t.cover_budget, t.seeds, t.flame, t.flame_air);
        }
        if yearly {
            self.push_series(&c, spent);
        }
        if framed {
            self.files.frames.push(w.day());
            frame(&c, &mut self.files.bytes);
        }
    }

    /// The latest value of a series column (NaN if there is none), for the run's printed line.
    pub fn last(&self, name: &str) -> f64 {
        self.files.series.iter().find(|c| c.name == name).and_then(|c| c.values.last().copied()).unwrap_or(f64::NAN)
    }

    /// What the tape's files hold now, copied so that another thread can write them while the world goes on.
    pub fn tape(&self) -> TapeFiles {
        self.files.clone()
    }

    /// Write the whole tape into `dir` (`TapeFiles::write` with nothing written before).
    pub fn finish(&self, dir: impl AsRef<Path>) -> io::Result<()> {
        self.files.write(dir.as_ref(), &mut Written::default())
    }
}

impl TapeFiles {
    /// Write `world.ewf.gz` (viewer frames and what a resumed run reads), `physics.json` and `series.csv` into `dir`. The
    /// viewer's file is written whole each time; the CSV file gets only its new rows (`csv`), when `written` says what this
    /// caller wrote to it before.
    pub fn write(&self, dir: &Path, written: &mut Written) -> io::Result<()> {
        std::fs::create_dir_all(dir)?;
        self.write_ewf(dir)?;
        std::fs::write(dir.join("physics.json"), &self.physics)?;
        self.write_csv(dir, written)
    }
}
