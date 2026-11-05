//! The frames of the viewer's file (`tape::read_viewer_file`): a byte a square and layer, undone to the layer's unit.

use serde_json::Value;
use std::io;
use std::path::Path;

/// How a layer's bytes are scaled (`tape::Layer`).
struct Scale {
    name: String,
    log: bool,
    min: f64,
    max: f64,
}

pub struct Frames {
    pub w: usize,
    pub h: usize,
    /// The rows wrap east to west
    pub wrap: bool,
    /// The year of each frame
    pub years: Vec<f64>,
    layers: Vec<Scale>,
    bytes: Vec<u8>,
}

fn bad(e: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e.to_string())
}

impl Frames {
    pub fn read(path: &Path) -> io::Result<Frames> {
        let (head, bytes) = tape::read_viewer_file(path)?;
        let num = |v: &Value, k: &str| v[k].as_f64().ok_or_else(|| bad(format!("{}: no {k}", path.display())));
        let list = |k: &str| head[k].as_array().ok_or_else(|| bad(format!("{}: no {k}", path.display())));
        let layers = list("layers")?
            .iter()
            .map(|l| Ok(Scale { name: l["name"].as_str().unwrap_or("").to_string(), log: l["scale"] == "log", min: num(l, "min")?, max: num(l, "max")? }))
            .collect::<io::Result<Vec<_>>>()?;
        let years = list("frames")?.iter().map(|f| num(f, "year")).collect::<io::Result<Vec<_>>>()?;
        let (w, h) = (num(&head, "w")? as usize, num(&head, "h")? as usize);
        if bytes.len() != w * h * layers.len() * years.len() {
            return Err(bad(format!("{}: the frames do not fit the tape's size", path.display())));
        }
        Ok(Frames { w, h, wrap: head["globe"] == true, years, layers, bytes })
    }

    /// Squares in a frame.
    pub fn squares(&self) -> usize {
        self.w * self.h
    }

    /// The values of layer `name` in frame `t`, one per square, in the layer's unit (a zero byte is zero).
    pub fn layer(&self, t: usize, name: &str) -> io::Result<Vec<f64>> {
        let l = self.layers.iter().position(|s| s.name == name).ok_or_else(|| bad(format!("the tape has no layer {name}")))?;
        let s = &self.layers[l];
        let at = (t * self.layers.len() + l) * self.squares();
        let value = |b: u8| match (b, s.log) {
            (0, true) => 0.0,
            (b, true) => (s.min.ln() + (b as f64 - 1.0) / 254.0 * (s.max.ln() - s.min.ln())).exp(),
            (b, false) => s.min + b as f64 / 255.0 * (s.max - s.min),
        };
        Ok(self.bytes[at..at + self.squares()].iter().map(|&b| value(b)).collect())
    }
}
