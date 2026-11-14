//! The history tape: snapshots, the yearly series, the monthly rows, raw events and viewer frames.

mod ground;
mod gzfile;
mod layers;
mod measure;
mod rec;
mod snapshot;
mod stands;
mod table;
mod takers;
mod things;

pub use ground::{depths, half, unhalf, Ground, Law, ALONG_WRAP_M, FLOOD, GUTTER, LINE_GROW, LINE_MOST, LINE_PX, PONDS_EXACT, SHOW_M, STATION_EXP, STEP_MOST, TILE, TYPICAL_RUNOFF, VALLEY_FROM, VALLEY_M};
pub use layers::{fields, pixels, raster, quantise, Fields, is_river, river_count, LAYERS, RIVER_CATCH, RIVER_FLOOR, STANDING_M};
pub use rec::{read_viewer_file, Kept, Recorder, Spent, TapeFiles, Written, TAPE};
pub use things::things;
pub use stands::{KINDS, KIND_ROW};
pub use table::{Table, ROW};
pub use takers::{TAKER_ROW, TAKER_SLOTS};
pub use snapshot::{fingerprint, load_snapshot, read_snapshot, snapshot_bytes, write_snapshot};
