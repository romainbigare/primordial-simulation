//! World generation: chemistry, terrain and the starting state.

pub mod chemistry;
pub mod life;
pub mod planet;
pub mod schedule;
pub mod terrain;
#[cfg(feature = "testkit")]
pub mod testkit;

pub use chemistry::{chemistry0, Init};
