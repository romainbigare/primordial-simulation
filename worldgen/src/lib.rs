//! World generation: chemistry, terrain and the starting state.

pub mod chemistry;
pub mod generate;
pub mod life;
pub mod richness;
pub mod schedule;
pub mod terrain;

pub use chemistry::{chemistry0, Init};
pub use generate::generate;
pub use richness::richness;
pub use terrain::new_world;
