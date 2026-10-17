//! The world core: one graph, five laws. No IO, no clocks, no Earth words.

pub mod body;
pub mod bond;
pub mod brain;
pub mod change;
pub mod check;
pub mod chem;
pub mod choose;
pub mod copy;
pub mod event;
pub mod flow;
mod ground;
pub mod grid;
pub mod lanes;
pub mod life;
pub mod lineage;
mod math;
pub mod nodes;
pub mod par;
mod params;
pub mod rng;
mod schedule;
pub mod survey;
pub mod units;
mod world;

pub use params::{Config, Physics};
pub use schedule::{step, step_marked, Stage};
pub use world::{Diag, Ledger, PerSlot, World};
