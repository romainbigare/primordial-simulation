//! First life: cover and takers placed on land at the start, initial conditions only (laws.md 5); its reactions are
//! found from the chemistry's numbers (`Chemistry::life_roles`). `kinds`: the table of the cover; `seeding`: placing a
//! kind of cover; `soil`: the mineral of the land when life starts; `takers`: the takers' table and their placing.

mod kinds;
mod seeding;
mod soil;
mod takers;

pub use kinds::{Kind, KINDS, LIGHT_ROW, ONE, ROSTER};
pub use seeding::{seed, seed_with, Seeding, FIXERS, FIX_GENE};
pub use soil::soil;
pub use takers::{place_takers, start_members, TakerKind, START_STORE, TAKER_KINDS};

use pcore::World;

/// Land under more standing liquid than this, m, gets no soil and no cover (a basin of standing liquid).
const MAX_STANDING_M: f64 = 0.5;

/// Whether life starts on cell `i`: land under no more than `MAX_STANDING_M` of standing liquid.
fn open_land(w: &World, i: usize) -> bool {
    w.is_land(i) && pcore::contract::env_of(w, i)[1] <= MAX_STANDING_M
}
