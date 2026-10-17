//! The history tape: snapshots, the yearly series, the monthly rows, raw events and viewer frames.

mod layers;
mod measure;
mod rec;
mod snapshot;

pub use layers::{dry_land_flux, fields, quantise, Fields, LAYERS, RIVER_WEB, STANDING_M};
pub use rec::{Kept, Recorder};
pub use snapshot::{fingerprint, load_snapshot, save_snapshot};
