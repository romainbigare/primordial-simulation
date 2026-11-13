//! The cover's passes of a month: its month on the chunks of `life` (`month`), and before each air step the release of
//! the light set aside in each cell's aside node that its cover does not claim (`claims`, `release`; laws.md 1.4).

use crate::life;
use crate::store::{apply, Batch, Class};
use crate::World;

/// Once a month, after Flow's state is built in full: the cover's month (`cover::step`) in each cell that holds any,
/// one task per chunk on the pool; then its seeds land (`cover::land`).
pub fn month(w: &mut World) {
    let load = w.life.cells.iter().filter(|c| c.cover.any()).count();
    life::run(w, load, super::step);
    super::land(w);
}

/// Per cell, what its cover claims of its aside node, J: the light set aside for it since its last month
/// (`cover::step`), its `Cover::keep` of what its capture area absorbed since.
fn claims(w: &World) -> Vec<f64> {
    let mut claim = vec![0.0; w.mesh.cells().n()];
    let Some(p) = super::Parts::of(&w.chem) else { return claim };
    let mm = p.mm(&w.chem);
    for (i, c) in w.life.cells.iter().map(|x| &x.cover).enumerate().filter(|e| e.1.keep > 0.0) {
        let abs: f64 = c.held().map(|x| c.leaf(&mm, x) * (1.0 - c.reflectance(&w.chem, &p, x))).sum();
        claim[i] += c.keep as f64 * abs * (w.light[i] - c.mark);
    }
    claim
}

/// Before an air step: the heat each aside node holds that its cover does not claim (`claims`) goes back to its
/// ground, so the step starts from it.
pub fn release(w: &mut World) {
    let mut back = Batch::new([Class::Aside, Class::Ground], &[]);
    for (i, c) in claims(w).into_iter().enumerate() {
        let free = w.store.heat(Class::Aside)[i] - c;
        if free > 0.0 {
            back.link((i, i), free);
        }
    }
    apply(w, &[back.flux()], &[]);
}
