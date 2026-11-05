//! World generation: chemistry, terrain and the starting state.

pub mod chemistry;
mod drain;
pub mod generate;
pub mod life;
pub mod richness;
pub mod schedule;
pub mod terrain;
#[cfg(feature = "testkit")]
pub mod testkit;

pub use chemistry::{chemistry0, Init};
pub use generate::generate;
pub use richness::richness;
pub use terrain::new_world;
