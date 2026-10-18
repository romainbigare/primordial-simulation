//! Death and damage: what a body or a share of its members leaves goes to its cell's ground, with its heat.

use super::{light_cat, Body};
use crate::event::{Event, EventKind};
use crate::{bond, World};

/// Remains go to the ground node as loose amounts, with the body's heat.
pub fn die(w: &mut World, slot: usize, mut b: Body, cause: u8) {
    bond::let_go(w, &mut b, false);
    let (n, i) = (w.mesh.cells().n(), b.cell as usize);
    for (k, &a) in b.store.iter().enumerate() {
        w.ground[k * n + i] += a;
    }
    w.e_ground[i] += b.e;
    let (cat, root) = (light_cat(&w.chem, &b.genes), w.life.root(&b));
    w.events.push(Event { day: w.day(), kind: EventKind::Died { cause, cat, root }, node: b.cell, value: b.count });
    w.life.bodies[slot] = b;
    w.life.remove(slot);
}

/// Amounts `m` (mol of each substance, at most what it holds) go from body `b` to its cell's ground as loose amounts, with
/// their heat and fusion enthalpy (`Body::heat_with`).
pub(crate) fn to_ground(w: &mut World, b: &mut Body, m: &[f64]) {
    let (n, i) = (w.mesh.cells().n(), b.cell as usize);
    let q = b.heat_with(&w.chem, m);
    b.lose(&w.chem, m);
    for (k, &a) in m.iter().enumerate() {
        w.ground[k * n + i] += a;
    }
    b.e -= q;
    w.e_ground[i] += q;
}

/// Some of a body's matter goes to its cell's ground as loose amounts: `dead` (mol of each substance), with its share of
/// the heat and of the brood (`Body::lose`). In a node these are the remains of a share `f` of its members, who died of `cause` (the
/// count falls). False if less than one member is left (the caller lets the node die).
pub(super) fn thin(w: &mut World, b: &mut Body, f: f64, dead: &[f64], cause: u8) -> bool {
    let dead: Vec<f64> = dead.iter().zip(&b.store).map(|(&a, &held)| a.min(held)).collect();
    to_ground(w, b, &dead);
    let chem = &w.chem;
    if b.n() > 1.0 && f > 0.0 {
        let (cat, root) = (light_cat(chem, &b.genes), w.life.root(b));
        w.events.push(Event { day: w.day(), kind: EventKind::Died { cause, cat, root }, node: b.cell, value: b.count * f });
        w.life.set_count(b, b.count * (1.0 - f));
    }
    b.count >= 1.0
}

/// A hit tears a share `f` of a body (`Body::built`): it breaks body matter, which holds a body together, and spills the
/// stores that matter held (`dead`, mol of each, as `thin`). In a node, `f` of one member's body or more takes whole
/// members, who die; less takes part of every member's matter, so the node's mean body falls and its members live.
/// `matter0` is the body matter, kg, before the hit. Returns `None` if less than one member is left, else the body
/// matter, kg, torn off members who live on (0 when members died).
fn tear(w: &mut World, b: &mut Body, f: f64, dead: &[f64], cause: u8, matter0: f64) -> Option<f64> {
    let whole = b.n() > 1.0 && f * b.n() >= 1.0;
    let alive = thin(w, b, if whole { f } else { 0.0 }, dead, cause);
    alive.then(|| if whole { 0.0 } else { (matter0 - b.matter_kg(&w.chem)).max(0.0) })
}

/// An individual dies when a hit leaves it this share of its body or less.
const TORN_TO_DEATH: f64 = 0.5;

/// A hit tears the share `f` of body `b`, taken out of `slot` (`tear`). It dies if less than one member is left, or, as an
/// individual, if it is left `TORN_TO_DEATH` of its body (`built0` before the hit) or less, or a store below its safe
/// minimum; else it lives on, smaller, back in its slot. True if it lives.
#[allow(clippy::too_many_arguments)]
pub fn wound(w: &mut World, slot: usize, mut b: Body, f: f64, dead: &[f64], cause: u8, built0: f64, matter0: f64) -> bool {
    let torn = tear(w, &mut b, f, dead, cause, matter0);
    if torn.is_none() || (b.n() <= 1.0 && (b.built(&w.chem) <= TORN_TO_DEATH * built0 * (1.0 + 1e-9) || b.lacking(&w.chem))) {
        die(w, slot, b, cause);
        return false;
    }
    b.vis = b.visible(&w.chem, &w.cfg.phys);
    w.life.bodies[slot] = b;
    true
}
