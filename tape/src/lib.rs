//! The history tape: snapshots, the yearly series, the monthly rows, raw events and viewer frames.

mod fine;
mod gzfile;
mod layers;
mod measure;
mod rec;
mod snapshot;
mod things;

pub use fine::{fine, Samples};
pub use layers::{fields, frame_now, pixels, raster, quantise, Fields, catchment, is_river, river_count, LAYERS, RIVER_CATCH, RIVER_FLOOR, STANDING_M};
pub use rec::{read_viewer_file, Kept, Recorder, Spent, TapeFiles, Written, ENCLOSED, REFINE, TAPE};
pub use things::{things, DIET_BODIES, DIET_COVER, DIET_REMAINS};
pub use snapshot::{fingerprint, load_snapshot, read_snapshot, snapshot_bytes, write_snapshot};
