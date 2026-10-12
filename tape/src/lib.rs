//! The history tape: snapshots, yearly summary series, raw events and viewer frames.

mod layers;
mod measure;
mod rec;

pub use layers::{fields, quantise, Ctx, Fields, Layer, COVER_MIN, LAYERS};
pub use measure::{kinds, learned, light_eater, LIGHT_EATER};
pub use rec::Recorder;

use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use pcore::World;
use std::io::{self, BufReader, BufWriter, Write};
use std::path::Path;

fn bad(e: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e.to_string())
}

/// Save the full state (bincode, gzip). Scratch fields marked `serde(skip)` are not saved.
pub fn save_snapshot(w: &World, path: impl AsRef<Path>) -> io::Result<()> {
    let mut z = GzEncoder::new(BufWriter::new(std::fs::File::create(path)?), Compression::fast());
    bincode::serialize_into(&mut z, w).map_err(bad)?;
    z.finish()?.flush()
}

/// Load a snapshot. Skipped scratch fields come back as their defaults (the groups' sums of
/// born rows are rebuilt).
pub fn load_snapshot(path: impl AsRef<Path>) -> io::Result<World> {
    let mut w: World = bincode::deserialize_from(GzDecoder::new(BufReader::new(std::fs::File::open(path)?))).map_err(bad)?;
    w.life.resum();
    Ok(w)
}
