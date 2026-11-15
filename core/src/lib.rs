//! The world core: the ground, the air and the cover by their laws. No IO, no wall clock, no Earth words.

pub mod change;
pub mod check;
pub mod chem;
pub mod clock;
pub mod contract;
pub mod cover;
pub mod event;
pub mod fauna;
pub mod flame;
pub mod flow;
mod ground;
pub mod inside;
pub mod life;
mod math;
pub mod par;
mod params;
pub mod rng;
mod schedule;
pub mod solve;
pub mod space;
pub mod store;
pub mod units;
mod world;

pub use params::{Config, Physics};
pub use schedule::{day_with_month_at_start, day_with_month_at_start_marked, month_start, step, step_marked, step_with_air, Stage, AIR_STEP_DAYS};
pub use world::{Counters, Taken, World, AIR};
