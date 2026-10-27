//! Files of the tape that are written whole and read back by others: the snapshots and the viewer's file.

use flate2::{write::GzEncoder, Compression};
use std::io::{self, Write};
use std::path::Path;

/// Added to a file's name while it is being written.
const PART: &str = ".part";

/// Write `chunks`, one after the other, to `path` as gzip (the fast level: the compression of these files is a cost
/// of every run, and any reader takes any level). They go to a file beside it that takes the name once whole, so
/// a run that stops during the write, and a reader that comes at that moment, never meet half a file under the name.
/// A run killed during the write leaves the `.part` file.
pub(crate) fn write_gzip(path: &Path, chunks: &[&[u8]]) -> io::Result<()> {
    let mut part = path.as_os_str().to_owned();
    part.push(PART);
    // Large writes, so no `BufWriter` is needed (small writes need one before the compressor).
    let mut z = GzEncoder::new(std::fs::File::create(&part)?, Compression::fast());
    for chunk in chunks {
        z.write_all(chunk)?;
    }
    z.finish()?.flush()?;
    std::fs::rename(part, path)
}
