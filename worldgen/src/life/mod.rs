//! First life: bodies placed on land at the start. These are initial conditions only; afterwards everything follows
//! the laws. The reactions they speed up are found from the chemistry's numbers (`Chemistry::life_roles`): the light
//! reaction, the energy-rich solid it makes, the liquid it uses, and the reaction that burns that solid with a gas.
//!
//! `kinds`: the table of starting kinds; `make`: one founder; `seeding`: placing a kind; `soil`: the mineral of the
//! land when life starts; `clever`: the big-brain test.

mod clever;
mod kinds;
mod make;
mod seeding;
mod soil;

pub use clever::{seed_clever, CLEVER_ROOT, CLEVER_SIZES, CONTROL_ROOT};
pub use kinds::{Kind, KINDS, ROSTER};
pub use make::{make, LIGHT_X, LITTER_X};
pub use seeding::{seed, seed_with, Seeding, FIXERS, FIX_GENE};
pub use soil::soil;

use pcore::body::Body;
use pcore::store::Boundary;
use pcore::{survey, World};

/// Land under more standing liquid than this, m, gets no soil and no founder (a basin of standing liquid).
const MAX_STANDING_M: f64 = 0.5;

/// Whether life starts on cell `i`: land (`World::is_land`) under no more than `MAX_STANDING_M` of standing liquid (what
/// the pores of the ground do not hold, `survey::env_of`).
fn open_land(w: &World, i: usize) -> bool {
    w.is_land(i) && survey::env_of(w, i)[1] <= MAX_STANDING_M
}

/// Book a new body like an edit: its stores and its heat, from outside the world.
fn book_body(w: &mut World, b: &Body) {
    w.cross(Boundary::Outside, b.store.iter().copied().enumerate(), b.e);
}
