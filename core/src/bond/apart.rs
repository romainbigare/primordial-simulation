//! A part's matter back to its cell's ground: a part that falls apart, whole or a share of a lot, what its walls shed,
//! and the loose solids of the ground a part is made of.

use super::{at, holder, unlink, Part};
use crate::{chem::{Chemistry, Phase}, life::Chunk, store::{Class, Node}, World};

/// The part at `p` of cell `i` of chunk `d` falls apart: all it holds, its walls, the liquid in its hollow and its heat,
/// goes to its cell's ground (`fall`). Then it is gone (`dissolve`). Every way a part falls apart uses it: its walls run
/// off or it is buried in its turn (`month::turns`), a body takes them below `min_kg` (`verbs::take`), or its edge
/// wears them away (`edge::blunted`).
pub(super) fn fall_apart(w: &World, d: &mut Chunk, i: usize, p: usize) {
    let (id, all) = (d.at(i).parts[p].id, d.at(i).parts[p].count);
    fall(w, d, i, (p, all));
    dissolve(w, d, i, id);
}

/// `k` parts of the lot at `p` of cell `i` of chunk `d` fall apart: their share of all it holds, its walls, the liquid
/// in its hollow and its heat (`Part::split`), goes to its cell's ground, moved by its own update on the chunk's view.
pub(super) fn fall(w: &World, d: &mut Chunk, i: usize, (p, k): (usize, f64)) {
    d.moves.open(w, i, Node::new(Class::Part, p));
    let gone = d.at_mut(i).parts[p].split(k);
    d.moves.give_all(w, Node::ground(i), gone.wall.iter().zip(&gone.fill).map(|(a, f)| a + f).enumerate(), gone.e);
}

/// The share `f(k, t, walls)` of each substance `k` of the walls of part `p` leaves them with its heat at the part's
/// temperature `t` (the walls as they are by then): its mol and J are added to `off`, one entry per substance. Returns
/// the part's heat capacity after, J/K.
pub(super) fn shed(chem: &Chemistry, p: &mut Part, f: impl Fn(usize, f64, &[f64]) -> f64, off: &mut [[f64; 2]]) -> f64 {
    let cap = p.heat_cap(chem);
    let (t, mut changed) = (p.e / cap.max(1e-9), false);
    for (k, o) in off.iter_mut().enumerate() {
        if p.wall[k] <= 0.0 {
            continue;
        }
        let f = f(k, t, &p.wall);
        if f > 0.0 {
            let (a, q) = (f * p.wall[k], f * p.wall[k] * chem.subs[k].cp * t);
            (p.e, p.wall[k], changed) = (p.e - q, p.wall[k] - a, true);
            *o = [o[0] + a, o[1] + q];
        }
    }
    if changed { p.heat_cap(chem) } else { cap }
}

/// What parts of cell `i` shed (`shed`: mol and J per substance) goes to its ground, one move per substance, on the
/// chunk's view opened for them.
pub(super) fn to_ground(w: &World, d: &mut Chunk, i: usize, off: &[[f64; 2]]) {
    for (k, &[a, q]) in off.iter().enumerate().filter(|o| o.1[0] > 0.0) {
        d.moves.give(w, Node::ground(i), k, a, q);
    }
}

/// The loose solids of the ground of cell `i` as the chunk's view holds them, mol of each substance: those solid there
/// at its temperature `tg` K, in its mixture (`Chemistry::phase_in`).
pub(super) fn loose_solids(w: &World, d: &Chunk, i: usize, tg: f64) -> Vec<f64> {
    let g = d.moves.amounts(w, Node::ground(i));
    (0..w.chem.ns()).map(|k| if w.chem.phase_in(k, tg, &g) == Phase::Solid { g(k).max(0.0) } else { 0.0 }).collect()
}

/// The part named `id` of cell `i` is gone: its bonds, its holder's hold and its place in the list.
fn dissolve(w: &World, d: &mut Chunk, i: usize, id: u64) {
    loop {
        let Some((o, _)) = d.at(i).bonds.of(id).next() else { break };
        unlink(w, d, i, id, o, true);
    }
    let c = d.at_mut(i);
    if let Some(h) = at(c, id).and_then(|p| holder(c, p)) {
        c.bodies[h].hold = None;
    }
    c.parts.retain(|p| p.id != id);
}
