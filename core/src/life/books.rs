//! The books of the life of a cell in a month (laws.md 3): what its reactions change outside the life that runs them
//! (`Books`), and how that is booked through the cell's view of the store (`book`), each move with its heat. The cover
//! and every other life of a cell keep their books this one way.

use crate::chem::{Chemistry, Heat, Phase, Reaction};
use crate::store::{Moves, Node};
use crate::units::MAX_SUBSTANCES;
use crate::{World, AIR};

/// What a life of a cell changes outside itself in a month: per substance the mol it takes in (+) or gives out (-), the
/// heat its reactions release, J, and per substance its remains, mol; per gas what the cell's share of its air node
/// offers, mol (infinite for what is not a gas: the ground's inputs are limited where they are read); and the `N`
/// flows of its budget, J.
pub struct Books<const N: usize> {
    pub ext: [f64; MAX_SUBSTANCES],
    pub heat: f64,
    pub dead: [f64; MAX_SUBSTANCES],
    pub air: [f64; MAX_SUBSTANCES],
    pub budget: [f64; N],
}

impl<const N: usize> Books<N> {
    /// Nothing changed yet, where the cell's share of its air node offers `air` (per substance, mol) and the budget's
    /// flows start at `budget`.
    pub fn new(air: [f64; MAX_SUBSTANCES], budget: [f64; N]) -> Books<N> {
        Books { ext: [0.0; MAX_SUBSTANCES], heat: 0.0, dead: [0.0; MAX_SUBSTANCES], air, budget }
    }
    /// The most units of `rx` the gas the cell's share of its air node still offers allows (what this life took of it
    /// this month counted), for a life made of the substances `sub`.
    pub fn room(&self, sub: &[usize], rx: &Reaction) -> f64 {
        let outside = rx.inputs.iter().filter(|q| !sub.contains(&(q.0 as usize)));
        outside.map(|&(k, n)| (self.air[k as usize] - self.ext[k as usize]).max(0.0) / n as f64).fold(f64::INFINITY, f64::min)
    }
    /// `x` units of `rx` run in amounts `m` (mol of the substances `sub`, in their order); what is not one of them is the
    /// cell's to take or give.
    pub fn run(&mut self, chem: &Chemistry, sub: &[usize], m: &mut [f64], rx: &Reaction, x: f64) {
        if x <= 0.0 {
            return;
        }
        let mut one = |k: u16, d: f64| match sub.iter().position(|&s| s == k as usize) {
            Some(q) => m[q] += d,
            None => self.ext[k as usize] -= d,
        };
        rx.inputs.iter().for_each(|&(k, n)| one(k, -(n as f64) * x));
        rx.outputs.iter().for_each(|&(k, n)| one(k, n as f64 * x));
        self.heat -= chem.dh_of(rx) * x;
    }
}

/// Book what a life of cell `i` (ground at `t` K), which held `e` J and was given `set_aside` J from its
/// aside node, took and gave in its month (`Books`): the light set aside for it from its aside node; gases from and to
/// the air (taken from its air node with their heat at its temperature and their latent heat, given into the aside
/// node with theirs, as bodies breathe); liquids and minerals from and to its ground, with their heat and the fusion
/// enthalpy of the share of them molten there (`World::molten_last`); its remains to its
/// ground. Last its heat comes to `target` J (its heat capacity at the temperature it ends at) with its ground. Returns
/// the heat it holds then, J.
pub fn book<const N: usize>(w: &World, moves: &mut Moves, (i, t): (usize, f64), (e, set_aside): (f64, f64), target: f64, b: &Books<N>) -> f64 {
    let (chem, j) = (&w.chem, w.mesh.up(i, 0, AIR));
    let (mut e, lift, ta, ground) = (e + set_aside + b.heat, w.lift(i), moves.t_air(w, j), Node::ground(i));
    moves.take(w, Node::aside(i), 0, 0.0, set_aside);
    let cap = moves.heat_cap_ground(w, i);
    let held = |kk: usize, m: f64| m * chem.subs[kk].carried(t, w.molten_last(kk, i));
    for (kk, (&ext, &died)) in b.ext.iter().zip(&b.dead).enumerate().take(chem.ns()) {
        let (m, s) = (ext - died, &chem.subs[kk]);
        if m == 0.0 {
            continue;
        }
        if chem.phase(kk, t) == Phase::Gas {
            moves.gas(w, kk, -m, lift);
            if m > 0.0 {
                moves.take(w, Node::air(j), 0, 0.0, m * s.cp * ta);
                e += m * s.carried_gas(ta);
            } else {
                moves.give(w, Node::aside(i), 0, 0.0, -m * s.cp * t);
                e += m * s.carried_gas(t);
            }
        } else if m > 0.0 {
            moves.take(w, ground, kk, m, 0.0);
            e += moves.take_from_ground(w, i, (0, 0.0, 0.0), held(kk, m), cap).1;
        } else {
            moves.give(w, ground, kk, -m, held(kk, -m));
            e -= held(kk, -m);
        }
    }
    if target > e {
        e += moves.take_from_ground(w, i, (0, 0.0, 0.0), target - e, cap).1;
    } else {
        moves.give(w, ground, 0, 0.0, e - target);
        e = target;
    }
    e
}
