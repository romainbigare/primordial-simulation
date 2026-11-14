//! Placing takers from outside the world (an edit, laws.md 5): a slot of them in a cell, booked from the outside node.

use super::{mm, sub, BODY, KEEP, NF, NG, STORE};
use crate::contract;
use crate::life::{kin, Parts, WET_TISSUE};
use crate::World;

/// Place `members` takers of genes `genes` and starting kind `root` in an empty slot of cell `i`, each at the living
/// mass of its `MASS` gene with `store` kg of fuel, at its body's temperature (its `KEEP` gene, or its ground's where
/// that is warmer): an edit booked from the outside node (`contract::placed`). False where the chemistry has no life,
/// no slot is empty, or no member is placed.
pub fn place(w: &mut World, i: usize, (genes, root): ([f32; NG], u32), (members, store): (f64, f64)) -> bool {
    let Some(p) = Parts::of(&w.chem) else { return false };
    let Some(x) = kin::empty(&w.life.cell_at(i).fauna.root) else { return false };
    if members <= 0.0 {
        return false;
    }
    let (sub, mm) = (sub(&p), mm(&w.chem, &p));
    let mut mol = [0.0; NF];
    mol[BODY] = members * super::adult(&genes) * (1.0 - WET_TISSUE) / mm[BODY];
    mol[STORE] = members * store / mm[STORE];
    let tb = (genes[KEEP] as f64).max(contract::place(w, i).t);
    let heat = (0..NF).map(|q| mol[q] * w.chem.subs[sub[q]].cp).sum::<f64>() * tb;
    let f = &mut w.life.cells[i].fauna;
    (f.adults[x], f.mol[x], f.e[x], f.genes[x], f.root[x]) = (members, mol, heat, genes, root);
    // (The takers' side is their own: the store books the outside node's.)
    contract::placed(w, i, (0..NF).map(|q| (sub[q], mol[q])), heat);
    true
}
