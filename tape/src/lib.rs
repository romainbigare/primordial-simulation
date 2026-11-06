//! The history tape: snapshots, the yearly series, the monthly rows, raw events and viewer frames.

mod gzfile;
mod layers;
mod measure;
mod rec;
mod snapshot;
mod things;

pub use layers::{dry_land_flux, fields, frame_now, frame_window, pixels, raster, quantise, Fields, LAYERS, RIVER_WEB, STANDING_M};
pub use rec::{read_viewer_file, Kept, Recorder, Spent, TapeFiles, Written, ENCLOSED, REFINE, TAPE};
pub use things::{things, DIET_BODIES, DIET_COVER, DIET_REMAINS};
pub use snapshot::{fingerprint, load_snapshot, read_snapshot, snapshot_bytes, write_snapshot};
