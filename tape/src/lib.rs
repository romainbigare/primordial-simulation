//! The history tape: snapshots, the yearly series, the monthly rows, raw events and viewer frames.

mod gzfile;
mod layers;
mod measure;
mod rec;
mod snapshot;

pub use layers::{dry_land_flux, fields, frame_now, pixels, quantise, Fields, LAYERS, RIVER_WEB, STANDING_M};
pub use rec::{Kept, Recorder, Spent, TapeFiles, Written, TAPE};
pub use snapshot::{fingerprint, load_snapshot, read_snapshot, snapshot_bytes, write_snapshot};
