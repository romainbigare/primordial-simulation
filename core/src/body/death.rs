//! Death and damage: what a body or a share of its members leaves goes to its cell's ground, with its heat.

use super::{each, Body};
use crate::chem::Stores;
use crate::event::{Event, EventKind};
use crate::store::Node;
use crate::{bond, life::{Chunk, Handle}, World};

/// Remains go to the ground node as loose amounts, with the body's heat.
pub fn die(w: &World, d: &mut Chunk, h: Handle, mut b: Body, cause: u8) {
    perish(w, d, &mut b, (cause, w.day()));
    d.remove(h, &b);
}

/// Body `b`, in no slot, dies of `cause` on `day`: it lets go of what it holds and its remains go to its cell's ground.
pub(crate) fn perish(w: &World, d: &mut Chunk, b: &mut Body, (cause, day): (u8, u64)) {
    bond::let_go(d, b, false);
    remains(w, d, b);
    let root = d.root(w, b);
    d.events.push(Event { day, kind: EventKind::Died { cause, root }, node: b.cell, value: b.count });
}

/// All that body `b` holds goes to its cell's ground node as loose amounts, with its heat (the update open moves it).
pub(crate) fn remains(w: &World, d: &mut Chunk, b: &Body) {
    d.moves.give_all(w, Node::ground(b.cell as usize), each(&w.chem, &b.store), b.e);
}

/// Amounts `m` (mol of each store, at most what it holds) go from body `b` to its cell's ground as loose amounts, with
/// their heat and fusion enthalpy (`Body::heat_with`).
pub(crate) fn to_ground(w: &World, d: &mut Chunk, b: &mut Body, m: &Stores) {
    let q = b.heat_with(&w.chem, m);
    b.lose(&w.chem, m);
    b.e -= q;
    d.moves.give_all(w, Node::ground(b.cell as usize), each(&w.chem, m), q);
}

/// `dead` (mol of each store, cut here to what it holds) goes from body `b` to the ground; in a node the remains of the
/// share `f` of its members, dead of `cause` on `day`. False if less than one member is left.
pub(super) fn thin(w: &World, d: &mut Chunk, b: &mut Body, f: f64, dead: &mut Stores, (cause, day): (u8, u64)) -> bool {
    dead.iter_mut().zip(&b.store).for_each(|(a, &held)| *a = a.min(held));
    to_ground(w, d, b, dead);
    if b.n() > 1.0 && f > 0.0 {
        let root = d.root(w, b);
        d.events.push(Event { day, kind: EventKind::Died { cause, root }, node: b.cell, value: b.count * f });
        b.count *= 1.0 - f;
    }
    b.count >= 1.0
}

/// A hit that tears the share `f` of a member's body kills it: the share `Physics::lethal` or more.
pub(crate) fn torn_to_death(p: &crate::Physics, f: f64) -> bool { f >= p.lethal * (1.0 - 1e-9) }

/// A hit tears the share `f` of the body (`Body::built`) of the share `hit` of the members of body `b`, taken out of
/// `h`: it breaks body matter, which holds a body together, and spills the stores it held (`dead`, mol of each store, as
/// `thin`, which cuts it to what the body holds). Those it tears `Physics::lethal` of their body or more die (an individual also when a store falls below its
/// safe minimum), and `dead` holds all they held; else they live on, smaller, and the node's mean body falls. True if
/// the body lives (back in its slot).
pub fn wound(w: &World, d: &mut Chunk, h: Handle, mut b: Body, (hit, f): (f64, f64), dead: &mut Stores, cause: u8) -> bool {
    let dies = torn_to_death(&w.cfg.phys, f);
    if !thin(w, d, &mut b, if dies { hit } else { 0.0 }, dead, (cause, w.day())) || (b.n() <= 1.0 && (dies || b.lacking(&w.chem, b.mass(&w.chem), b.temp(&w.chem)))) {
        die(w, d, h, b, cause);
        return false;
    }
    b.stale = true;
    d[h] = b;
    true
}
