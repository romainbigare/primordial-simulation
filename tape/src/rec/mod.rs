//! The recorder, driven once per simulated month by the shell. It counts the world's events (`tally`), writes a row
//! of `months.csv` every month (`months`) and a row of the yearly series every year (`cols`), keeps viewer frames,
//! and writes it all to a folder; the viewer's file (`ewf`) holds what a resumed run needs to go on.

mod cols;
mod ewf;
mod months;
mod tally;

use crate::layers::{frame, Ctx};
use months::MonthRow;
use pcore::event::{Event, EventKind};
use pcore::World;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io;
use std::path::Path;
use tally::{MonthTally, Social, YearTally};

/// Death causes in the order of the core's constants (`body::LACK` .. `body::AGE`).
const CAUSES: [&str; 6] = ["lack", "phase", "spent", "taken", "broken", "age"];

/// One column of the yearly series: its name, unit and a value a year (NaN: none that year).
struct Column {
    name: String,
    unit: String,
    values: Vec<f64>,
}

/// The world a tape is of: width and height in cells, cell size (m), seed and days per year.
#[derive(Clone, Copy)]
struct Shape {
    w: usize,
    h: usize,
    cell_m: f64,
    seed: u64,
    days_per_year: u64,
}

/// What the recorder keeps across years and a snapshot carries (`Recorder::kept`), so that a resumed run's series
/// go on as the unbroken run's: the substances seen so far and `Social`.
#[derive(Serialize, Deserialize)]
pub struct Kept {
    seen: Vec<bool>,
    social: Social,
}

pub struct Recorder {
    shape: Shape,
    /// Frames are kept every `every` years, and every month in the first `monthly` years
    every: u64,
    monthly: u64,
    /// The substances found anywhere so far (`detect`)
    seen: Vec<bool>,
    /// The day of each frame kept, and their bytes (`layers::frame`)
    frames: Vec<u64>,
    bytes: Vec<u8>,
    series: Vec<Column>,
    /// The events the viewer shows: day and text
    log: Vec<(u64, String)>,
    months: Vec<MonthRow>,
    /// The sums of the year so far and of the month to come, and what is kept across years
    year: YearTally,
    month: MonthTally,
    social: Social,
    /// Body updates counted at the last yearly row
    updates: u64,
    /// The run's `Physics` and `Config` as JSON text, written next to the series (`physics.json`)
    physics: String,
}

/// The run's numbers: `{"physics": {..}, "config": {..}}`, every field of `Physics` and of `Config`
/// (the config without its `phys`), keys sorted, one line each, so two runs' files can be
/// compared with a diff or `ew numbers`.
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
        let g = &w.grid;
        Recorder {
            shape: Shape { w: g.w, h: g.h, cell_m: g.cell_m, seed: w.seed, days_per_year: w.days_per_year() },
            every,
            monthly,
            seen: vec![false; w.chem.ns()],
            frames: vec![],
            bytes: vec![],
            series: vec![],
            log: vec![],
            months: vec![],
            year: YearTally::default(),
            month: MonthTally::default(),
            social: Social::default(),
            updates: 0,
            physics: physics_json(w),
        }
    }

    /// What the recorder keeps across years, for a snapshot (taken at a year's end, after its row).
    pub fn kept(&self) -> Kept {
        Kept { seen: self.seen.clone(), social: self.social.clone() }
    }

    /// Start a run resumed from a snapshot: take back what the recorder kept (`kept`), and load what an earlier run
    /// wrote to `dir` (series, months, frames, events) up to the snapshot's day, so the files go on from there. What it
    /// wrote after that day is dropped (a resume from an earlier snapshot replays it). A `dir` without a tape starts the
    /// tape empty. The events before the snapshot are the earlier run's, already counted.
    pub fn resume(&mut self, w: &mut World, kept: Kept, dir: impl AsRef<Path>) -> io::Result<()> {
        (self.seen, self.social) = (kept.seen, kept.social);
        w.events.clear();
        self.updates = w.counts.updates;
        self.read_ewf(dir.as_ref(), w)
    }

    /// A substance with a non-zero amount anywhere that was zero everywhere before.
    fn detect(&mut self, w: &mut World) {
        let (n, na) = (w.grid.n(), w.grid.na());
        for k in 0..self.seen.len() {
            if self.seen[k] {
                continue;
            }
            let hit = [(&w.ground, n, 0), (&w.bed, n, 0), (&w.air, na, n)]
                .iter()
                .find_map(|&(v, len, off)| v[k * len..(k + 1) * len].iter().position(|&x| x > 0.0).map(|p| (off + p, v[k * len + p])));
            if let Some((node, value)) = hit {
                self.seen[k] = true;
                w.events.push(Event { day: w.day, kind: EventKind::FirstSubstance(k as u16), node: node as u32, value });
            }
        }
    }

    /// Call at the start and after every simulated month. A month the tape has already (the snapshot's month, in a
    /// resumed run) is not recorded again.
    pub fn record_month(&mut self, w: &mut World) {
        let dpy = w.days_per_year();
        let (year, yearly) = (w.day / dpy, w.day.is_multiple_of(dpy));
        if self.months.last().is_some_and(|r| (r.year, r.month) == months::year_month(w)) {
            return;
        }
        self.detect(w);
        for e in std::mem::take(&mut w.events) {
            self.count(&e, w);
        }
        let framed = year < self.monthly || (yearly && self.every > 0 && year % self.every == 0);
        self.month(w);
        if !yearly && !framed {
            return;
        }
        if yearly {
            self.year.decisions = [std::mem::take(&mut w.counts.decisions), std::mem::take(&mut w.counts.greedy)];
            self.year.parts = std::mem::take(&mut w.counts.parts);
        }
        let c = Ctx::new(w);
        if yearly {
            self.push_series(&c);
        }
        if framed {
            self.frames.push(w.day);
            frame(&c, &mut self.bytes);
        }
    }

    /// The latest value of a series column (NaN if there is none), for the run's printed line.
    pub fn last(&self, name: &str) -> f64 {
        self.series.iter().find(|c| c.name == name).and_then(|c| c.values.last().copied()).unwrap_or(f64::NAN)
    }

    /// Write `world.ewf.gz` (viewer frames and what a resumed run reads), `physics.json`, `months.csv` and `series.csv`
    /// into `dir`.
    pub fn finish(&self, dir: impl AsRef<Path>) -> io::Result<()> {
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir)?;
        self.write_ewf(dir)?;
        let mut csv = self.series.iter().map(|c| c.name.as_str()).collect::<Vec<_>>().join(",");
        for row in 0..self.series.first().map_or(0, |c| c.values.len()) {
            csv += &format!("\n{}", self.series.iter().map(|c| c.values[row].to_string()).collect::<Vec<_>>().join(","));
        }
        std::fs::write(dir.join("physics.json"), &self.physics)?;
        std::fs::write(dir.join("months.csv"), months::csv(&self.months))?;
        std::fs::write(dir.join("series.csv"), csv + "\n")
    }
}
