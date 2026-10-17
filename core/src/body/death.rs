//! Death and damage: remains to the ground.

use super::{light_cat, Body};
use crate::event::{Event, EventKind};
use crate::{bond, Diag, World};

/// Remains go to the ground node as loose amounts, with the body's heat.
pub fn die(w: &mut World, slot: usize, mut b: Body, cause: u8) {
    bond::let_go(w, &mut b, false);
    let (n, i) = (w.grid.n(), b.cell as usize);
    for (k, &a) in b.store.iter().enumerate() {
        w.ground[k * n + i] += a;
    }
    w.e_ground[i] += b.e;
    let (cat, root, adult) = (light_cat(&w.chem, &b.genes), w.life.root(&b), b.adult(&w.chem));
    w.events.push(Event { day: w.day, kind: EventKind::Died { cause, cat, root, adult }, node: b.cell, value: b.count });
    w.life.bodies[slot] = b;
    w.life.remove(slot);
}

/// Some of a body's matter goes to its cell's ground as loose amounts: `dead` (mol of each
/// substance), with its share of the heat. In a node these are the remains of a share `f` of
/// its members, who died of `cause` (the count falls). False if less than one member is left
/// (the caller lets the node die).
pub(super) fn thin(w: &mut World, b: &mut Body, f: f64, dead: &[f64], cause: u8) -> bool {
    let (n, i, chem) = (w.grid.n(), b.cell as usize, &w.chem);
    // Whether the members who die were grown, judged before their matter leaves the node.
    let adult = b.adult(chem);
    let q = chem.heat_share(b.e, b.heat_cap(chem).max(1e-9), dead);
    // The brood of the members who die goes with them: the same share of it as of the matter.
    let matter = chem.matter().map(|m| (m, b.store[m]));
    for (k, &a) in dead.iter().enumerate() {
        let a = a.min(b.store[k]);
        b.store[k] -= a;
        w.ground[k * n + i] += a;
    }
    if let Some((m, before)) = matter.filter(|x| x.1 > 0.0) {
        b.brood = b.brooded(chem).min(before) * (b.store[m] / before);
    }
    b.e -= q;
    w.e_ground[i] += q;
    if b.n() > 1.0 && f > 0.0 {
        let (cat, root) = (light_cat(chem, &b.genes), w.life.root(b));
        w.events.push(Event { day: w.day, kind: EventKind::Died { cause, cat, root, adult }, node: b.cell, value: b.count * f });
        b.count *= 1.0 - f;
    }
    b.count >= 1.0
}

/// A hit tears a share `f` of a body (partial damage): the energy of a bite, of being
/// pushed through, breaks body matter first, which is what holds a body together, and spills the
/// stores that matter held (`dead`, mol of each, with their share of the heat, as `thin`). The torn
/// share is of the body (`Body::built`), so what the hit weighs against one member's body decides:
/// in a node, `f` of one member or more (`f` times the count at least 1) takes whole members, the
/// ones that die (`thin`); less takes part of every member's matter, so the node's mean body falls
/// and its members live. An individual always lives on smaller (its caller decides when it is too
/// little to live). `matter0` is the body matter, kg, it had before the hit. Returns `None` if less
/// than one member is left, else the body matter, kg, torn off members who live on (0 when members
/// died), which the caller counts with `hurt` once it knows the body lives.
fn tear(w: &mut World, b: &mut Body, f: f64, dead: &[f64], cause: u8, matter0: f64) -> Option<f64> {
    let whole = b.n() > 1.0 && f * b.n() >= 1.0;
    let alive = thin(w, b, if whole { f } else { 0.0 }, dead, cause);
    alive.then(|| if whole { 0.0 } else { (matter0 - b.matter_kg(&w.chem)).max(0.0) })
}

/// An individual dies when this share of its body or more is torn off.
const TORN_TO_DEATH: f64 = 0.5;

/// A hit tears the share `f` of the body in `slot` (`tear`, taken out of its slot). It dies if less than one member is left,
/// or, as an individual, when half of its body (`Body::built`, `built0` before the hit) or more is torn or a store fell
/// below its safe minimum; else it lives on, hurt (`hurt`), back in its slot. True if it lives.
#[allow(clippy::too_many_arguments)]
pub fn wound(w: &mut World, slot: usize, mut b: Body, f: f64, dead: &[f64], cause: u8, built0: f64, matter0: f64) -> bool {
    let torn = tear(w, &mut b, f, dead, cause, matter0);
    if torn.is_none() || (b.n() <= 1.0 && (b.built(&w.chem) <= TORN_TO_DEATH * built0 * (1.0 + 1e-9) || b.lacking(&w.chem))) {
        die(w, slot, b, cause);
        return false;
    }
    hurt(w, &b, torn.unwrap_or(0.0));
    b.vis = b.visible(&w.chem, &w.cfg.phys);
    w.life.bodies[slot] = b;
    true
}

/// Count `kg` of body matter torn off a body that lived on (`tear`) for its starting kind (`Diag::hurt`).
fn hurt(w: &mut World, b: &Body, kg: f64) {
    if kg > 0.0 {
        if let Some(h) = Diag::of_root(&mut w.diag.hurt, w.life.root(b)) {
            *h += kg;
        }
    }
}
