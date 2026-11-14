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

/// The open land, where life starts, by Flow's state of the world now: each cell that holds land (`flow::land_shares`,
/// the share of its area out of the liquid that stands over its pores; a cell of parts whose mean ground lies under the
/// level among them) under no more than `MAX_STANDING_M` of standing liquid, in the order of the cells, with the area
/// of its land, m², on which life starts per m².
fn open_land(w: &World) -> Vec<(usize, f64)> {
    let all: Vec<usize> = (0..w.mesh.cells().n()).collect();
    let land = pcore::flow::land_shares(w, &all).expect("Flow's state is built before life starts");
    let area = w.mesh.cells().area();
    all.into_iter().filter(|&i| land[i] > 0.0 && pcore::contract::env_of(w, i)[1] <= MAX_STANDING_M).map(|i| (i, area[i] * land[i])).collect()
}
