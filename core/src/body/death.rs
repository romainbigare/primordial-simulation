//! Death and damage: what a body or a share of its members leaves goes to its cell's ground, with its heat.

use super::{light_cat, Body};
use crate::event::{Event, EventKind};
use crate::ground::Melting;
use crate::store::Node;
use crate::{bond, life::Handle, World};

/// Remains go to the ground node as loose amounts, with the body's heat.
pub fn die(w: &mut World, h: Handle, mut b: Body, cause: u8) {
    bond::let_go(w, &mut b, false);
    let i = b.cell as usize;
    for (k, &a) in b.store.iter().enumerate().filter(|x| *x.1 != 0.0) {
        w.moves.give(&w.chem, Node::ground(i), k, a, 0.0);
    }
    w.moves.give(&w.chem, Node::ground(i), 0, 0.0, b.e);
    let (cat, root) = (light_cat(&w.chem, &b.genes), w.life.root(&b));
    w.events.push(Event { day: w.day(), kind: EventKind::Died { cause, cat, root }, node: b.cell, value: b.count });
    w.life.remove(h, &b);
}

/// Amounts `m` (mol of each substance, at most what it holds) go from body `b`, at the plateau state `th`, to its cell's
/// ground as loose amounts, with their heat and fusion enthalpy (`Body::heat_with`).
pub(crate) fn to_ground(w: &mut World, b: &mut Body, th: &Melting, m: &[f64]) {
    let i = b.cell as usize;
    let q = b.heat_with(&w.chem, th, m);
    b.lose(&w.chem, m);
    for (k, &a) in m.iter().enumerate().filter(|x| *x.1 != 0.0) {
        w.moves.give(&w.chem, Node::ground(i), k, a, 0.0);
    }
    b.e -= q;
    w.moves.give(&w.chem, Node::ground(i), 0, 0.0, q);
}

/// `dead` (mol of each, at most what it holds) goes from body `b` at the plateau state `th` to the ground; in a node
/// the remains of the share `f` of its members, dead of `cause`. False if less than one member is left.
pub(super) fn thin(w: &mut World, b: &mut Body, f: f64, dead: &[f64], cause: u8, th: &Melting) -> bool {
    let dead: Vec<f64> = dead.iter().zip(&b.store).map(|(&a, &held)| a.min(held)).collect();
    to_ground(w, b, th, &dead);
    let chem = &w.chem;
    if b.n() > 1.0 && f > 0.0 {
        let (cat, root) = (light_cat(chem, &b.genes), w.life.root(b));
        w.events.push(Event { day: w.day(), kind: EventKind::Died { cause, cat, root }, node: b.cell, value: b.count * f });
        w.life.set_count(b, b.count * (1.0 - f));
    }
    b.count >= 1.0
}

/// A hit that tears the share `f` of a member's body kills it: the share `Physics::lethal` or more.
pub(crate) fn torn_to_death(p: &crate::Physics, f: f64) -> bool { f >= p.lethal * (1.0 - 1e-9) }

/// A hit tears the share `f` of the body (`Body::built`) of the share `hit` of the members of body `b`, taken out of
/// `h`: it breaks body matter, which holds a body together, and spills the stores it held (`dead`, mol of each, as
/// `thin`). Those it tears `Physics::lethal` of their body or more die (an individual also when a store falls below its
/// safe minimum), and `dead` holds all they held; else they live on, smaller, and the node's mean body falls. True if
/// the body lives (back in its slot).
pub fn wound(w: &mut World, h: Handle, mut b: Body, (hit, f): (f64, f64), dead: &[f64], cause: u8) -> bool {
    let (dies, th) = (torn_to_death(&w.cfg.phys, f), b.thermal(&w.chem));
    if !thin(w, &mut b, if dies { hit } else { 0.0 }, dead, cause, &th) || (b.n() <= 1.0 && (dies || b.lacking(&w.chem, b.mass(&w.chem), b.temp(&w.chem)))) {
        die(w, h, b, cause);
        return false;
    }
    w.see(&mut b);
    w.life[h] = b;
    true
}
