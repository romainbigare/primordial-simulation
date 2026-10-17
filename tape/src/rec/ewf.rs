//! `world.ewf.gz`, the viewer's file and what a resumed run goes on from: gzip of `EWF1`, the header's length
//! (`u32`, little-endian), the header (JSON: the world's size, the layers, the day of each frame, the series and
//! their units, the events and the rows of `months.csv`) and the frames (one byte a cell and layer, `layers::frame`).

use super::months::year_month;
use super::{Column, MonthRow, Recorder};
use crate::layers::LAYERS;
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use pcore::World;
use serde_json::{json, Value};
use std::io::{self, Read, Write};
use std::path::Path;

const FILE: &str = "world.ewf.gz";
const MAGIC: &[u8; 4] = b"EWF1";

impl Recorder {
    /// Write the file into `dir`.
    pub(super) fn write_ewf(&self, dir: &Path) -> io::Result<()> {
        let s = self.shape;
        let year = |d: u64| d as f64 / s.days_per_year as f64;
        let head = json!({
            "w": s.w, "h": s.h, "cell_m": s.cell_m, "seed": s.seed, "days_per_year": s.days_per_year,
            "layers": LAYERS.iter().map(|l| json!({"name": l.name, "unit": l.unit, "scale": l.scale, "min": l.min, "max": l.max})).collect::<Vec<_>>(),
            "frames": self.frames.iter().map(|&d| json!({"day": d, "year": year(d)})).collect::<Vec<_>>(),
            "series": self.series.iter().map(|c| (c.name.clone(), json!(c.values))).collect::<serde_json::Map<_, _>>(),
            "series_units": self.series.iter().map(|c| (c.name.clone(), json!(c.unit))).collect::<serde_json::Map<_, _>>(),
            "events": self.log.iter().map(|(d, t)| json!({"day": d, "year": year(*d), "text": t})).collect::<Vec<_>>(),
            "months": self.months,
        });
        let head = serde_json::to_vec(&head)?;
        // Four large writes, so no `BufWriter` is needed (small writes need one before the compressor).
        let mut z = GzEncoder::new(std::fs::File::create(dir.join(FILE))?, Compression::default());
        z.write_all(MAGIC)?;
        z.write_all(&(head.len() as u32).to_le_bytes())?;
        z.write_all(&head)?;
        z.write_all(&self.bytes)?;
        z.finish()?.flush()
    }

    /// Load what an earlier run of this world wrote to `dir`, up to the world's day: frames, events, series rows and
    /// month rows (nothing if `dir` holds no file).
    pub(super) fn read_ewf(&mut self, dir: &Path, w: &World) -> io::Result<()> {
        let Ok(f) = std::fs::File::open(dir.join(FILE)) else { return Ok(()) };
        let bad = |e: &str| io::Error::new(io::ErrorKind::InvalidData, format!("{}: {e}", dir.display()));
        let mut raw = vec![];
        GzDecoder::new(f).read_to_end(&mut raw)?;
        let n = raw.get(4..8).map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize);
        let head = raw.get(8..8 + n).filter(|_| raw.starts_with(MAGIC)).ok_or_else(|| bad("not a tape"))?;
        let g: Value = serde_json::from_slice(head)?;
        let s = self.shape;
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
        self.frames = frames.iter().map(day).take_while(|&d| d <= w.day).collect();
        self.bytes = raw[8 + n..8 + n + frame * self.frames.len()].to_vec();
        self.log = events.iter().take_while(|e| day(e) <= w.day).map(|e| (day(e), e["text"].as_str().unwrap_or("").to_string())).collect();
        // Series: the rows of years up to this one (a null is a NaN that was written).
        let (series, units) = (object("series")?, object("series_units")?);
        let years = series.get("year").and_then(Value::as_array).ok_or_else(|| bad("no years"))?;
        let rows = years.iter().take_while(|y| y.as_f64().is_some_and(|y| y <= w.year())).count();
        self.series = series
            .iter()
            .map(|(k, v)| {
                let values = v.as_array().filter(|a| a.len() >= rows).ok_or_else(|| bad(&format!("series {k} is short")))?;
                let unit = units.get(k).and_then(Value::as_str).unwrap_or("").to_string();
                Ok(Column { name: k.clone(), unit, values: values[..rows].iter().map(|x| x.as_f64().unwrap_or(f64::NAN)).collect() })
            })
            .collect::<io::Result<_>>()?;
        // Months: the rows up to this month.
        let now = year_month(w);
        let months: Vec<MonthRow> = serde_json::from_value(g["months"].clone())?;
        self.months = months.into_iter().take_while(|r| (r.year, r.month) <= now).collect();
        Ok(())
    }
}
