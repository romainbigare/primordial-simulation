//! Snapshots: the whole world and the shell's own state (`P`: what it still has to place, what its recorder keeps,
//! `Recorder::kept`), in one file.

use flate2::read::GzDecoder;
use pcore::World;
use serde::{de::DeserializeOwned, Serialize};
use std::io::{self, BufReader, Read, Write};
use std::path::Path;

/// Start of every snapshot file, before the bincode body: a tag, then the format version (little-endian `u32`).
const SNAPSHOT_TAG: [u8; 4] = *b"EWSN";
/// Raise it whenever the serialized layout of `World` (or of the shell's state) changes.
const SNAPSHOT_VERSION: u32 = 191;
/// Buffer between bincode and the decompressor, bytes: bincode reads 8 bytes at a time, and each call into the
/// decompressor is slow.
const BUFFER: usize = 1 << 20;

/// FNV-1a, 64 bits: offset basis and prime.
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x100_0000_01b3;

fn bad(e: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e.to_string())
}

/// Put the bytes of a snapshot of the world and the shell's state into `bytes` (replacing what it held): the header,
/// then bincode. The laws' caches are not saved (`docs/tape.md`). The buffer is the caller's, so that it can be used
/// again: filling a new one that grows costs more than the serialising does.
pub fn snapshot_bytes<P: Serialize>(w: &World, plan: &P, bytes: &mut Vec<u8>) -> io::Result<()> {
    bytes.clear();
    bytes.extend_from_slice(&SNAPSHOT_TAG);
    bytes.extend_from_slice(&SNAPSHOT_VERSION.to_le_bytes());
    bincode::serialize_into(&mut *bytes, w).map_err(bad)?;
    bincode::serialize_into(&mut *bytes, plan).map_err(bad)
}

/// A snapshot of the world and the shell's state (`snapshot_bytes`) as the gzip a snapshot file holds (`write_snapshot`),
/// for a shell that hands the file on (the page's save).
pub fn snapshot_gz<P: Serialize>(w: &World, plan: &P) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    snapshot_bytes(w, plan, &mut bytes)?;
    crate::gzfile::gzip(&bytes)
}

/// Write the bytes of a snapshot (`snapshot_bytes`) to `path` as gzip, whole or not at all (`gzfile::write_gzip`).
pub fn write_snapshot(bytes: &[u8], path: impl AsRef<Path>) -> io::Result<()> {
    crate::gzfile::write_gzip(path.as_ref(), &[bytes])
}

/// Load a snapshot file of this format version (`open_snapshot`).
pub fn load_snapshot<P: DeserializeOwned>(path: impl AsRef<Path>) -> io::Result<(World, P)> {
    open_snapshot(std::fs::File::open(path)?)
}

/// Read a snapshot of this format version from its gzip `gz` (`write_snapshot`): a file, or the bytes the page is given
/// (`read_snapshot`).
pub fn open_snapshot<P: DeserializeOwned>(gz: impl Read) -> io::Result<(World, P)> {
    read_snapshot(BufReader::with_capacity(BUFFER, GzDecoder::new(gz)))
}

/// Read the bytes of a snapshot of this format version (`snapshot_bytes`) from `z`; caches come back empty and are
/// rebuilt (the mesh from the map, each body's numbers from its genes, each cell's sums of its bodies that the tape
/// reads before the first survey).
pub fn read_snapshot<P: DeserializeOwned>(mut z: impl Read) -> io::Result<(World, P)> {
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
    w.mesh = World::mesh_of(&w.map, &w.cfg);
    pcore::contract::stands(&mut w);
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
