//! Clouds (laws.md 1.4 and 1.5): the air's vapour over its saturation rains with its matter and heat kept.

use pcore::store::{Class, Node};
use pcore::units::DAY;
use worldgen::testkit::{set_amount, Books, LIQUID};

/// An air step of three days.
const DT: f64 = 3.0 * DAY;

/// Air holding twice its vapour rains in one air step, and the world's matter and energy close to 1e-12 over it.
#[test]
fn a_condensing_step_keeps_matter_and_heat() {
    let mut w = worldgen::testkit::small_world();
    let na = w.mesh.level(pcore::AIR).n();
    for j in 0..na {
        let held = w.store.amounts(Class::Air)[LIQUID * na + j];
        set_amount(&mut w, Node::air(j), LIQUID, 2.0 * held);
    }
    let (books, before) = (Books::now(&w), w.store.amounts(Class::Air)[LIQUID * na..(LIQUID + 1) * na].iter().sum::<f64>());
    pcore::flow::state(&mut w, true);
    pcore::flow::air(&mut w, DT);
    let after: f64 = w.store.amounts(Class::Air)[LIQUID * na..(LIQUID + 1) * na].iter().sum();
    assert!(after < 0.8 * before, "the air kept {after:e} of its {before:e} mol of vapour");
    books.assert_within(&w, 1e-12, 1e-12);
}
