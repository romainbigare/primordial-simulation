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

/// The open land, where life starts: each cell that holds land (`contract::lands`, the area of a cell out of the liquid
/// that stands over its pores), in the order of the cells, with the area of its land, m², on which its soil and its
/// takers start per m². A kind of cover starts per m² of the ground it stands on there, out into the liquid as far as
/// its roots work (`World::sow`, `cover::footing`): one rule for where a plant stands.
fn open_land(w: &World) -> Vec<(usize, f64)> {
    let land = pcore::contract::lands(w);
    (0..w.mesh.cells().n()).filter(|&i| land[i] > 0.0).map(|i| (i, land[i])).collect()
}
