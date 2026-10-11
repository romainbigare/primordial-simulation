//! The recorder: driven once per simulated month by the shell.

use crate::layers::{frame, Ctx, COVER_MIN, LAYERS};
use flate2::{write::GzEncoder, Compression};
use pcore::check;
use pcore::event::{Event, EventKind};
use pcore::World;
use serde_json::json;
use std::io::{self, BufWriter, Write};
use std::path::Path;

type Column = (String, &'static str, Vec<f64>);

pub struct Recorder {
    /// Meta: width, height, cell size, seed, days per year
    meta: (usize, usize, f64, u64, u64),
    every: u64,
    monthly: u64,
    seen: Vec<bool>,
    frames: Vec<u64>,
    bytes: Vec<u8>,
    series: Vec<Column>,
    log: Vec<(u64, String)>,
}

fn describe(e: &Event) -> String {
    match e.kind {
        EventKind::FirstSubstance(k) => format!("substance {k} first appears"),
        EventKind::Born => format!("body born at node {}", e.node),
        EventKind::Died { cause } => format!("body died at node {} (cause {cause})", e.node),
        EventKind::BondMade => format!("bond made at node {}", e.node),
        EventKind::BondBroken => format!("bond broken at node {}", e.node),
        EventKind::GodAction => format!("god action at node {}", e.node),
    }
}

impl Recorder {
    /// Frames are kept every `every` years, and every month in the first `monthly` years.
    pub fn new(w: &World, every: u64, monthly: u64) -> Self {
        let g = &w.grid;
        Recorder {
            meta: (g.w, g.h, g.cell_m, w.seed, w.days_per_year()),
            every,
            monthly,
            seen: vec![false; w.chem.ns()],
            frames: vec![],
            bytes: vec![],
            series: vec![],
            log: vec![],
        }
    }

    /// Treat substances already present as known (for a run resumed from a snapshot).
    pub fn skip_existing(&mut self, w: &mut World) {
        self.detect(w);
        w.events.clear();
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

    /// Call at the start and after every simulated month.
    pub fn record_month(&mut self, w: &mut World) {
        self.detect(w);
        self.log.extend(w.events.drain(..).map(|e| (e.day, describe(&e))));
        let dpy = w.days_per_year();
        let (year, yearly) = (w.day / dpy, w.day % dpy == 0);
        let framed = year < self.monthly || (yearly && self.every > 0 && year % self.every == 0);
        if !yearly && !framed {
            return;
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

    fn push_series(&mut self, c: &Ctx) {
        let w = c.w;
        let n = w.grid.n() as f64;
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        let tot = check::totals(w);
        let land_liquid: f64 = (0..w.grid.n()).filter(|&i| c.f.z[i] > 0.0).map(|i| c.f.depth[i]).sum::<f64>() * w.grid.area();
        let mut row = vec![("year".to_string(), "yr", w.year())];
        row.extend(tot.elems.iter().enumerate().map(|(j, &v)| (format!("elem_{j}"), "mol", v)));
        row.extend([
            ("energy".to_string(), "J", tot.thermal + tot.chemical),
            ("t_ground".to_string(), "K", mean(&c.f.t)),
            ("t_air".to_string(), "K", mean(&c.t_air)),
            ("absorber".to_string(), "1", mean(&c.absorber)),
            ("ice_frac".to_string(), "1", c.f.ice.iter().filter(|&&v| v >= COVER_MIN).count() as f64 / n),
            ("liquid_land".to_string(), "km3", land_liquid / 1e9),
            ("rain".to_string(), "mol/m2/s", w.diag.rain.iter().map(|&v| v as f64).sum::<f64>() / n),
        ]);
        if self.series.is_empty() {
            self.series = row.iter().map(|r| (r.0.clone(), r.1, vec![])).collect();
        }
        for (col, r) in self.series.iter_mut().zip(row) {
            col.2.push(r.2);
        }
    }

    /// Write `world.ewf.gz` (viewer frames) and `series.csv` into `dir`.
    pub fn finish(&self, dir: impl AsRef<Path>) -> io::Result<()> {
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir)?;
        let (w, h, cell_m, seed, dpy) = self.meta;
        let year = |d: u64| d as f64 / dpy as f64;
        let meta = json!({
            "w": w, "h": h, "cell_m": cell_m, "seed": seed, "days_per_year": dpy,
            "layers": LAYERS.iter().map(|l| json!({"name": l.name, "unit": l.unit, "scale": l.scale, "min": l.min, "max": l.max})).collect::<Vec<_>>(),
            "frames": self.frames.iter().map(|&d| json!({"day": d, "year": year(d)})).collect::<Vec<_>>(),
            "series": self.series.iter().map(|c| (c.0.clone(), json!(c.2))).collect::<serde_json::Map<_, _>>(),
            "series_units": self.series.iter().map(|c| (c.0.clone(), json!(c.1))).collect::<serde_json::Map<_, _>>(),
            "events": self.log.iter().map(|(d, t)| json!({"day": d, "year": year(*d), "text": t})).collect::<Vec<_>>(),
        });
        let meta = serde_json::to_vec(&meta)?;
        let mut z = GzEncoder::new(BufWriter::new(std::fs::File::create(dir.join("world.ewf.gz"))?), Compression::default());
        z.write_all(b"EWF1")?;
        z.write_all(&(meta.len() as u32).to_le_bytes())?;
        z.write_all(&meta)?;
        z.write_all(&self.bytes)?;
        z.finish()?.flush()?;

        let mut csv = self.series.iter().map(|c| c.0.as_str()).collect::<Vec<_>>().join(",");
        for row in 0..self.series.first().map_or(0, |c| c.2.len()) {
            csv += &format!("\n{}", self.series.iter().map(|c| c.2[row].to_string()).collect::<Vec<_>>().join(","));
        }
        std::fs::write(dir.join("series.csv"), csv + "\n")
    }
}
