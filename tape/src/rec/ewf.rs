//! `world.ewf.gz`, the viewer's file and what a resumed run goes on from: gzip of `EWF2`, the header's length
//! (`u32`, little-endian), the header (JSON: the world's size, whether it is a globe and the latitudes of its edges, the
//! layers, the day of each frame, the series and their units, the events and the rows of `months.csv`) and the frames
//! (one byte a cell and layer, `layers::frame`).

use super::months::year_month;
use super::{Column, MonthRow, Recorder, TapeFiles};
use crate::layers::LAYERS;
use flate2::read::GzDecoder;
use pcore::World;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::{self, Read};
use std::path::Path;

const FILE: &str = "world.ewf.gz";
const MAGIC: &[u8; 4] = b"EWF2";

/// The header of the file, as JSON.
#[derive(Serialize)]
struct Head<'a> {
    w: usize,
    h: usize,
    cell_m: f64,
    globe: bool,
    lat_north: f64,
    lat_south: f64,
    seed: u64,
    days_per_year: u64,
    layers: Vec<Value>,
    frames: Vec<Stamp>,
    series: BTreeMap<&'a str, &'a [f64]>,
    series_units: BTreeMap<&'a str, &'a str>,
    events: Vec<Logged<'a>>,
    months: &'a [MonthRow],
}

/// A day and the year it falls in.
#[derive(Serialize)]
struct Stamp {
    day: u64,
    year: f64,
}

/// An event of the viewer's list: its day, the year and its text.
#[derive(Serialize)]
struct Logged<'a> {
    day: u64,
    year: f64,
    text: &'a str,
}

impl TapeFiles {
    /// Write the file into `dir`.
    pub(super) fn write_ewf(&self, dir: &Path) -> io::Result<()> {
        let s = self.shape;
        let stamp = |&day: &u64| Stamp { day, year: day as f64 / s.days_per_year as f64 };
        let head = Head {
            w: s.w,
            h: s.h,
            cell_m: s.cell_m,
            globe: s.globe,
            lat_north: s.lat_north,
            lat_south: s.lat_south,
            seed: s.seed,
            days_per_year: s.days_per_year,
            layers: LAYERS.iter().map(|l| json!({"name": l.name, "unit": l.unit, "scale": l.scale, "min": l.min, "max": l.max})).collect(),
            frames: self.frames.iter().map(stamp).collect(),
            series: self.series.iter().map(|c| (c.name.as_str(), c.values.as_slice())).collect(),
            series_units: self.series.iter().map(|c| (c.name.as_str(), c.unit.as_str())).collect(),
            events: self.log.iter().map(|(day, text)| Logged { day: *day, year: stamp(day).year, text }).collect(),
            months: &self.months,
        };
        let head = serde_json::to_vec(&head)?;
        crate::gzfile::write_gzip(&dir.join(FILE), &[MAGIC, &(head.len() as u32).to_le_bytes(), &head, &self.bytes])
    }
}

impl Recorder {
    /// Load what an earlier run of this world wrote to `dir`, up to the world's day: frames, events, series rows and
    /// month rows (nothing if `dir` holds no file; an error if its months stop before the world's).
    pub(super) fn read_ewf(&mut self, dir: &Path, w: &World) -> io::Result<()> {
        let Ok(f) = std::fs::File::open(dir.join(FILE)) else { return Ok(()) };
        let bad = |e: &str| io::Error::new(io::ErrorKind::InvalidData, format!("{}: {e}", dir.display()));
        let mut raw = vec![];
        GzDecoder::new(f).read_to_end(&mut raw)?;
        let n = raw.get(4..8).map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize);
        let head = raw.get(8..8 + n).filter(|_| raw.starts_with(MAGIC)).ok_or_else(|| bad("not a tape"))?;
        let g: Value = serde_json::from_slice(head)?;
        let s = self.files.shape;
        if [g["w"].as_u64(), g["h"].as_u64(), g["seed"].as_u64(), g["days_per_year"].as_u64()] != [s.w as u64, s.h as u64, s.seed, s.days_per_year].map(Some) {
            return Err(bad("the tape is of another world"));
        }
        let list = |k: &str| g[k].as_array().ok_or_else(|| bad(&format!("no {k}")));
        let object = |k: &str| g[k].as_object().ok_or_else(|| bad(&format!("no {k}")));
        // Frames and events carry their day: keep those up to the world's day.
        let day = |e: &Value| e["day"].as_u64().unwrap_or(u64::MAX);
        let (frames, events) = (list("frames")?, list("events")?);
        let frame = s.w * s.h * LAYERS.len();
        if raw.len() - 8 - n != frame * frames.len() {
            return Err(bad("the frames do not fit the tape's size"));
        }
        self.files.frames = frames.iter().map(day).take_while(|&d| d <= w.day()).collect();
        self.files.bytes = raw[8 + n..8 + n + frame * self.files.frames.len()].to_vec();
        self.files.log = events.iter().take_while(|e| day(e) <= w.day()).map(|e| (day(e), e["text"].as_str().unwrap_or("").to_string())).collect();
        // Series: the rows of years up to this one (a null is a written NaN). A column with no value yet is left out, as
        // the unbroken run had it then; it comes back with its first value (`Recorder::append`).
        let (series, units) = (object("series")?, object("series_units")?);
        let years = series.get("year").and_then(Value::as_array).ok_or_else(|| bad("no years"))?;
        let rows = years.iter().take_while(|y| y.as_f64().is_some_and(|y| y <= w.year())).count();
        let mut columns = vec![];
        for (k, v) in series {
            let values = v.as_array().filter(|a| a.len() >= rows).ok_or_else(|| bad(&format!("series {k} is short")))?;
            let values: Vec<f64> = values[..rows].iter().map(|x| x.as_f64().unwrap_or(f64::NAN)).collect();
            if values.iter().any(|x| !x.is_nan()) {
                let unit = units.get(k).and_then(Value::as_str).unwrap_or("").to_string();
                columns.push(Column { name: k.clone(), unit, values });
            }
        }
        self.files.series = columns;
        // Months: the rows up to this month.
        let now = year_month(w);
        let months: Vec<MonthRow> = serde_json::from_value(g["months"].clone())?;
        self.files.months = months.into_iter().take_while(|r| (r.year, r.month) <= now).collect();
        if self.files.months.last().map(|r| (r.year, r.month)) != Some(now) {
            return Err(bad("the tape stops before the snapshot"));
        }
        Ok(())
    }
}
