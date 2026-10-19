//! The world core: one graph, five laws. No IO, no wall clock, no Earth words.

pub mod body;
pub mod bond;
pub mod brain;
pub mod change;
pub mod check;
pub mod chem;
pub mod choose;
pub mod clock;
pub mod copy;
pub mod event;
pub mod flow;
mod ground;
pub mod lanes;
pub mod life;
pub mod lineage;
mod math;
pub mod nodes;
pub mod par;
mod params;
pub mod rng;
mod schedule;
pub mod solve;
pub mod space;
pub mod store;
pub mod survey;
pub mod units;
mod world;

pub use params::{Config, Physics};
pub use schedule::{day, step, step_marked, Stage};
pub use world::{Counters, World, AIR};
