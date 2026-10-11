//! World generation: chemistry, terrain and the starting state.

pub mod chemistry;
pub mod terrain;

pub use chemistry::{chemistry0, Init};
pub use terrain::new_world;
