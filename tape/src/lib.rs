//! The history tape: snapshots, yearly summary series, raw events and viewer frames.

mod layers;
mod measure;
mod rec;

pub use layers::{fields, quantise, Ctx, Fields, Layer, COVER_MIN, LAYERS};
pub use measure::{kinds, learned, light_eater, LIGHT_EATER};
pub use rec::Recorder;

use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use pcore::World;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;

fn bad(e: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e.to_string())
}

/// Save the full state (bincode, gzip). Scratch fields marked `serde(skip)` are not saved.
pub fn save_snapshot(w: &World, path: impl AsRef<Path>) -> io::Result<()> {
    // The buffer goes before the compressor: bincode writes 8 bytes at a time, and each call into the compressor is slow.
    let mut z = BufWriter::with_capacity(1 << 20, GzEncoder::new(std::fs::File::create(path)?, Compression::fast()));
    bincode::serialize_into(&mut z, w).map_err(bad)?;
    z.into_inner().map_err(|e| e.into_error())?.finish()?.flush()
}

/// Load a snapshot. Skipped scratch fields come back as their defaults (the groups' sums of
/// born rows are rebuilt, as each month does). Fields added at the end of `World` since the
/// snapshot was saved come back empty: the file is read as if zeros followed it (an empty list
/// is a zero length).
pub fn load_snapshot(path: impl AsRef<Path>) -> io::Result<World> {
    let z = BufReader::with_capacity(1 << 20, GzDecoder::new(std::fs::File::open(path)?));
    let mut w: World = bincode::deserialize_from(z.chain(io::repeat(0).take(64))).map_err(bad)?;
    w.life.resum();
    Ok(w)
}
