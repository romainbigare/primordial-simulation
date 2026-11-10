//! The history tape: snapshots, the yearly series, the monthly rows, raw events and viewer frames.

mod ground;
mod gzfile;
mod layers;
mod measure;
mod rec;
mod snapshot;
mod stands;
mod table;
mod things;

pub use ground::{depths, half, unhalf, Ground, GUTTER, MOST_RUNOFF, PONDS_EXACT, TILE};
pub use layers::{fields, pixels, raster, quantise, Fields, is_river, river_count, LAYERS, RIVER_CATCH, RIVER_FLOOR, STANDING_M};
pub use rec::{read_viewer_file, Kept, Recorder, Spent, TapeFiles, Written, ENCLOSED, TAPE};
pub use things::{things, DIET_BODIES, DIET_COVER, DIET_REMAINS};
pub use stands::{KINDS, KIND_ROW};
pub use table::{Table, ROW};
pub use snapshot::{fingerprint, load_snapshot, read_snapshot, snapshot_bytes, write_snapshot};
