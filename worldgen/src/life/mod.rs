//! First life: bodies placed on land at the start, initial conditions only (laws.md 8); their reactions are found from
//! the chemistry's numbers (`Chemistry::life_roles`). `kinds`: the table; `make`: one founder; `seeding`: placing a
//! kind; `soil`: the mineral of the land when life starts; `nursery`: the cover sorted by its law before it is placed.

mod kinds;
mod make;
pub mod nursery;
mod seeding;
mod soil;

pub use kinds::{plants, Bodies, Field, Kind, Place, KINDS, ROSTER, TWO};
pub use make::{make, LITTER_X};
pub use seeding::{seed, seed_with, Seeding, FIXERS, FIX_GENE};
pub use soil::soil;

use pcore::body::Body;
use pcore::store::{Boundary, Class, Node};
use pcore::{survey, World};

/// Land under more standing liquid than this, m, gets no soil and no founder (a basin of standing liquid).
const MAX_STANDING_M: f64 = 0.5;

/// Whether life starts on cell `i`: land under no more than `MAX_STANDING_M` of standing liquid.
fn open_land(w: &World, i: usize) -> bool {
    w.is_land(i) && survey::env_of(w, i)[1] <= MAX_STANDING_M
}

/// Book a new body like an edit: its stores and its heat, from outside the world, at its cell's head.
fn book_body(w: &mut World, b: &Body) {
    let held: Vec<(usize, f64)> = pcore::body::each(&w.chem, &b.store).collect();
    w.cross((Boundary::Outside, Node::new(Class::Body, b.cell as usize)), held, b.e);
}
