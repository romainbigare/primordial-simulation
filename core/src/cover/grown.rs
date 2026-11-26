//! A grown stand of a starting kind as the world holds it (handover.md 11: trees planted show as trees): an edit that
//! plants a kind places a copy of it (`place_grown`), its matter from the outside node, rather than seedlings.

use super::groups::Group;
use super::{Parts, NG, NP};
use crate::World;

/// A grown stand of a starting kind: its genes and kind, and per m² of the ground it stands on its amounts (mol of
/// each part) and its plants by group (`Group`, a count and built cover, kg).
#[derive(Clone, Debug)]
pub struct Grown {
    pub genes: [f32; NG],
    pub root: u32,
    pub mol: [f64; NP],
    pub groups: Vec<Group>,
}

/// The grown stand of starting kind `root` in world `w`: of the slots of that kind in every cell, the one that holds the
/// most built cover per m² of the ground it stands on (its cell's ground, `cover::ground`, by its patch's share and the
/// share it stands on, `Cover::stands`), the cells in their order; none where no slot of that kind holds built cover.
pub fn grown(w: &World, root: u32) -> Option<Grown> {
    let mm = Parts::of(&w.chem)?.mm(&w.chem);
    let mut best: Option<(f64, Grown)> = None;
    for i in 0..w.mesh.cells().n() {
        let (c, ground) = (w.life.cover(i), super::ground(w, i));
        for x in c.held().filter(|&x| c.root[x] == root) {
            let (area, built) = (ground * c.patch_share(x) * c.stands[x] as f64, c.built(&mm, x));
            if area <= 0.0 || built <= 0.0 || best.as_ref().is_some_and(|b| built / area <= b.0) {
                continue;
            }
            let (g, held) = c.groups(&mm, x);
            let groups = g[..held].iter().map(|g| Group { n: g.n / area, kg: g.kg / area }).collect();
            best = Some((built / area, Grown { genes: c.genes[x], root, mol: c.mol[x].map(|m| m / area), groups }));
        }
    }
    best.map(|b| b.1)
}
