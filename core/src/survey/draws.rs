//! The light set aside in the bodies' node of each cell for its cover's light reaction (laws.md 1.4): what the cover
//! claims, and what goes back to the ground before an air step.

use crate::store::{apply, Batch, Class};
use crate::World;

/// Per cell, what its cover claims of its bodies' node, J: the light set aside for it since its last month
/// (`cover::step`), its `Cover::keep` of what its capture area absorbed since.
pub(crate) fn claims(w: &World) -> Vec<f64> {
    let mut claim = vec![0.0; w.mesh.cells().n()];
    let Some(p) = crate::cover::Parts::of(&w.chem) else { return claim };
    let mm = p.mm(&w.chem);
    for d in &w.life.domains {
        let c = &d.cover;
        for (k, &i) in d.cells.iter().enumerate().filter(|e| c.keep[e.0] > 0.0) {
            let abs: f64 = crate::cover::Cover::slots(k).filter(|&x| c.root[x] != crate::cover::NO_ROOT).map(|x| c.leaf(&mm, x) * (1.0 - c.reflectance(&w.chem, &p, x))).sum();
            claim[i as usize] += c.keep[k] as f64 * abs * (w.light[i as usize] - c.mark[k]);
        }
    }
    claim
}

/// Before an air step: the heat each bodies' node holds that its cover does not claim (`claims`) goes back to its
/// ground, so the step starts from it.
pub(crate) fn release(w: &mut World) {
    let mut back = Batch::new([Class::Bodies, Class::Ground], &[]);
    for (i, c) in claims(w).into_iter().enumerate() {
        let free = w.store.heat(Class::Bodies)[i] - c;
        if free > 0.0 {
            back.link((i, i), free);
        }
    }
    apply(w, &[back.flux()], &[]);
}
