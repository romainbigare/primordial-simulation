//! Snapshots: the whole world and the shell's own state (`P`: what it still has to place, what its recorder keeps,
//! `Recorder::kept`), in one file.

use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use pcore::World;
use serde::{de::DeserializeOwned, Serialize};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;

/// Start of every snapshot file, before the bincode body: a tag, then the format version (little-endian `u32`).
const SNAPSHOT_TAG: [u8; 4] = *b"EWSN";
/// Raise it whenever the serialized layout of `World` (or of the shell's state) changes.
const SNAPSHOT_VERSION: u32 = 12;
/// Buffer between bincode and the compressor, bytes: bincode reads and writes 8 bytes at a time, and each call into
/// the compressor is slow.
const BUFFER: usize = 1 << 20;

/// FNV-1a, 64 bits: offset basis and prime.
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x100_0000_01b3;

fn bad(e: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e.to_string())
}

/// Save the world and the shell's state (header, then bincode, gzip). Scratch fields marked `serde(skip)` are not saved.
pub fn save_snapshot<P: Serialize>(w: &World, plan: &P, path: impl AsRef<Path>) -> io::Result<()> {
    let mut z = BufWriter::with_capacity(BUFFER, GzEncoder::new(std::fs::File::create(path)?, Compression::fast()));
    z.write_all(&SNAPSHOT_TAG)?;
    z.write_all(&SNAPSHOT_VERSION.to_le_bytes())?;
    bincode::serialize_into(&mut z, w).map_err(bad)?;
    bincode::serialize_into(&mut z, plan).map_err(bad)?;
    z.into_inner().map_err(|e| e.into_error())?.finish()?.flush()
}

/// Load a snapshot written by `save_snapshot` of this format version. Skipped scratch fields
/// come back as their defaults; the mesh is rebuilt from the grid's numbers, and the groups' sums of born rows, as
/// each month does.
pub fn load_snapshot<P: DeserializeOwned>(path: impl AsRef<Path>) -> io::Result<(World, P)> {
    let mut z = BufReader::with_capacity(BUFFER, GzDecoder::new(std::fs::File::open(path)?));
    let mut head = [0u8; 8];
    z.read_exact(&mut head).map_err(|e| bad(format!("not a snapshot ({e})")))?;
    if head[..4] != SNAPSHOT_TAG {
        return Err(bad("not a snapshot (no snapshot header)"));
    }
    let version = u32::from_le_bytes([head[4], head[5], head[6], head[7]]);
    if version != SNAPSHOT_VERSION {
        return Err(bad(format!("snapshot format version {version}; this build reads version {SNAPSHOT_VERSION}")));
    }
    let mut w: World = bincode::deserialize_from(&mut z).map_err(bad)?;
    let plan = bincode::deserialize_from(&mut z).map_err(bad)?;
    let g = &w.grid;
    w.grid = pcore::grid::Grid::new(g.w, g.h, g.cell_m, g.air_scale, &w.cfg);
    w.life.resum();
    Ok((w, plan))
}

/// FNV-1a over the bytes of the world's snapshot body (bincode): two worlds a snapshot cannot tell apart have the same
/// fingerprint.
pub fn fingerprint(w: &World) -> u64 {
    struct Fnv(u64);
    impl Write for Fnv {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            buf.iter().for_each(|&b| self.0 = (self.0 ^ b as u64).wrapping_mul(FNV_PRIME));
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut h = Fnv(FNV_OFFSET);
    bincode::serialize_into(&mut h, w).expect("a world serialises");
    h.0
}
