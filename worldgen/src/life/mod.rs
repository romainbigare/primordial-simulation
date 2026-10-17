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
use pcore::chem::Chemistry;
use pcore::Ledger;

/// Book new matter like an edit: `amount` mol of substance `k` with `heat` J of heat (its formation enthalpy is
/// added).
fn book(ledger: &mut Ledger, chem: &Chemistry, k: usize, amount: f64, heat: f64) {
    chem.add_elems(k, amount, &mut ledger.imposed_elems);
    ledger.imposed_energy += heat + amount * chem.subs[k].hf;
}

/// Book a new body like an edit: its heat, then each of its stores.
fn book_body(ledger: &mut Ledger, chem: &Chemistry, b: &Body) {
    ledger.imposed_energy += b.e;
    for (k, &a) in b.store.iter().enumerate() {
        book(ledger, chem, k, a, 0.0);
    }
}
