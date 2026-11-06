//! The pattern finder (handover 9.1): reads a run's folder and writes fact cards. It only reads the tape and never feeds
//! anything back into the simulation. The cards hold numbers and categories; naming them is the matcher's job.
//! `frames`: the viewer file's layers; `lasting`: lasting clusters and lanes; `structures`: repeated structures.

mod frames;
mod lasting;
mod structures;

use serde_json::{json, Value};
use std::io;
use std::path::Path;

/// A detector reports a pattern only if it lasts at least this many consecutive frames, ...
const LAST_FRAMES: usize = 3;
/// ... and at least this many years from its first frame to its last (the first years have a frame a month).
const LAST_YEARS: f64 = 20.0;
/// A square is dense when its animal mass per m² is over this many times the mean of all squares in that frame.
const GROUP_DENSITY: f64 = 4.0;
/// A group is at least this many squares.
const GROUP_MIN_SQUARES: usize = 4;
/// A square is on a lane when the passage through it is at least this many times cheaper than uncleared ground
/// (the worn layer, `lanes::eased`).
const LANE_EASE: f64 = 2.0;
/// A lane is at least this many squares long.
const LANE_MIN_SQUARES: usize = 3;
/// A structure is repeated when at least this many copies of it stand in the last snapshot.
const REPEAT_MIN: usize = 2;

/// Read the run folder `dir` and return `{"source": {...}, "cards": [...]}`. The frames of `world.ewf.gz` give the groups
/// and lanes; the snapshots `snap_*.bin.gz` give the structures. A run without frames or without snapshots gives no cards
/// of that kind, and `source` says what was read.
pub fn analyze(dir: &Path) -> io::Result<Value> {
    let file = dir.join("world.ewf.gz");
    let tape_frames = file.exists().then(|| frames::Frames::read(&file)).transpose()?;
    let (found, snapshots) = structures::read(dir)?;
    let mut cards = vec![];
    if let Some(f) = &tape_frames {
        cards.extend(lasting::groups(f)?);
        cards.extend(lasting::lanes(f)?);
    }
    cards.extend(found);
    let source = json!({"frames": tape_frames.as_ref().map_or(0, |f| f.years.len()), "snapshots": snapshots});
    Ok(json!({"source": source, "cards": cards}))
}
