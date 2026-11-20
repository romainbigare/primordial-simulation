//! Law 2: Change. Substances react inside nodes by mass action, faster when hot (laws.md 2). `kernel`: the rate law
//! and the reactor; `balance`: reactions that undo each other; `gas`: the gas a body or a part exchanges with the air.
//! Here: the step over the ground and air nodes, which react when due and sleep while calm (`clock::Sleep`): `rows`
//! makes their rows, `booking` books them.

mod balance;
mod booking;
mod kernel;
mod rows;

pub use balance::reversible;
pub use kernel::Reactor;
pub(crate) use kernel::open_catalyst;

use crate::{store::Class, AIR, World};
use rows::Rows;

/// Run Law 2 for the step of `dt` seconds at the temperatures of the last flow step: each ground node, then each air
/// node, that is due, for all the time since it last reacted, from the world as it was before its class reacted. The
/// nodes of a class react apart (`Rows::make`, block by block, in parallel with `threads`), each into its row, and the rows
/// are booked after the call (`Rows::book`). Then each sleeps (`Reactor::rest`, at most `sleep_days`).
pub fn step(w: &mut World, dt: f64) {
    crate::flow::ensure(w);
    let (end, most) = (w.time + dt as u64, w.cfg.sleep_days as u64 * w.cfg.calendar().day);
    for (class, len) in [(Class::Ground, w.mesh.cells().n()), (Class::Air, w.mesh.level(AIR).n())] {
        w.sleep.watch(class, len, w.store.amounts(class), w.cfg.wake_share, w.time);
        // Before the law first runs, every node is due.
        if w.sleep.none(class) {
            (0..len).for_each(|i| w.sleep.sleep(class, i, w.time, w.time));
        }
        // By index, so in the order of their cells.
        let due = w.sleep.due(class, w.time);
        if due.is_empty() {
            continue;
        }
        let rows = Rows::make(w, class, &due, (end, most));
        rows.book(w);
        for (i, rest) in rows.rests() {
            w.sleep.sleep(class, i, end, (end + rest).saturating_sub(dt as u64));
            w.sleep.keep(class, i, len, w.store.amounts(class));
        }
    }
}
