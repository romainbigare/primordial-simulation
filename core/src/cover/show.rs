//! The cover's side of the contract (`contract`): what it stands on its cell as (`Cover::stand`), what a part of it and
//! the seeds lying show a taker (`Cover::shown`, `Cover::shown_lying`), and what a take and a press do to it
//! (`Cover::take`, `Cover::take_lying`, `lose`). All of it is read from
//! the cover's own numbers when asked; it keeps no copy.

use super::{Cover, Parts, NP, WALL};
use crate::chem::Chemistry;
use crate::contract::{Shown, Stand};
use crate::units::MAX_SUBSTANCES;

impl Cover {
    /// What it stands on its cell as (`contract::Stand`): over its slots that hold cover its mass, capture area, that
    /// times the share of light it absorbs (`reflectance`) and that times the share set aside for its light reaction
    /// (`Cover::keep`).
    pub fn stand(&self, chem: &Chemistry, p: &Parts) -> Stand {
        let (mut s, mm) = (Stand::default(), p.mm(chem));
        for x in self.held() {
            let (leaf, abs) = (self.leaf(&mm, x), self.leaf(&mm, x) * (1.0 - self.reflectance(chem, p, x)));
            (s.mass, s.leaf, s.absorbed, s.kept) = (s.mass + self.kg(&mm, x), s.leaf + leaf, s.absorbed + abs, s.kept + abs * self.keep as f64);
        }
        s
    }
    /// What the shares `of(x)` of each part of each slot `x` show a taker (`contract::Shown`): its mass, what it holds of
    /// each substance and the energy to break it.
    pub fn shown(&self, chem: &Chemistry, ph: &crate::Physics, p: &Parts, of: impl Fn(usize) -> [f64; NP]) -> Shown {
        shown_of(chem, ph, p, self.mol.iter().enumerate().map(|(x, m)| (m, of(x))))
    }
    /// What the share `of` of the seeds lying on its ground shows a taker: its mass and what it holds of each substance.
    pub fn shown_lying(&self, chem: &Chemistry, ph: &crate::Physics, p: &Parts, of: f64) -> Shown {
        shown_of(chem, ph, p, self.lying.mol.iter().map(|m| (m, [of; NP])))
    }
    /// The height the plants of slot `x` stand at, m (`Cover::height`, its walls at their own density).
    pub fn stands_at(&self, chem: &Chemistry, ph: &crate::Physics, p: &Parts, x: usize) -> f64 {
        self.height(ph, &p.mm(chem), chem.subs[p.sub[WALL]].density[0], x)
    }
    /// The share `g` of the shares `of(x)` of each part of each slot `x` taken whole, each part in its own proportion (a
    /// bite): returns what is taken of each substance, mol, and the heat that goes with it, J (at its temperature).
    pub(crate) fn take(&mut self, chem: &Chemistry, p: &Parts, g: f64, of: impl Fn(usize) -> [f64; NP]) -> ([f64; MAX_SUBSTANCES], f64) {
        let t = self.temp(chem, p);
        let (got, heat) = take_from(chem, p, g, t, self.mol.iter_mut().enumerate().map(|(x, m)| (m, of(x))));
        let heat = heat.min(self.e);
        self.e -= heat;
        (got, heat)
    }
    /// The share `g` of the share `of` of the seeds lying on its ground taken whole, from each lot alike: returns what is
    /// taken, mol of each substance, and its heat, J (`take`).
    pub(crate) fn take_lying(&mut self, chem: &Chemistry, p: &Parts, g: f64, of: f64) -> ([f64; MAX_SUBSTANCES], f64) {
        let t = self.temp(chem, p);
        let (got, heat) = take_from(chem, p, g, t, self.lying.mol.iter_mut().map(|m| (m, [of; NP])));
        let heat = heat.min(self.e);
        self.e -= heat;
        (got, heat)
    }
}

/// What the shares `of` (amounts, mol of each part, and the share of each part shown) show a taker: their mass and
/// what they hold of each substance, and the energy to break them (`life::breaking`).
fn shown_of<'a>(chem: &Chemistry, ph: &crate::Physics, p: &Parts, of: impl Iterator<Item = (&'a [f64; NP], [f64; NP])>) -> Shown {
    let (mut kg, mut mol, mm) = (0.0, [0.0; MAX_SUBSTANCES], p.mm(chem));
    for (m, f) in of.filter(|e| e.1.iter().any(|&f| f > 0.0)) {
        kg += (0..NP).map(|q| m[q] * mm[q] * f[q]).sum::<f64>();
        (0..NP).for_each(|q| mol[p.sub[q]] += m[q] * f[q]);
    }
    Shown { kg, mol, breaking: crate::life::breaking(chem, ph, &mol), width: 0.0 }
}

/// The share `g` (at most all) of the shares `of` (amounts, mol of each part, and the share of each part that can be
/// taken) taken whole at `t` K: what is taken of each substance, mol, and the heat that goes with it, J.
fn take_from<'a>(chem: &Chemistry, p: &Parts, g: f64, t: f64, of: impl Iterator<Item = (&'a mut [f64; NP], [f64; NP])>) -> ([f64; MAX_SUBSTANCES], f64) {
    let (g, mut got) = (g.clamp(0.0, 1.0), [0.0; MAX_SUBSTANCES]);
    for (m, f) in of.filter(|e| e.1.iter().any(|&f| f > 0.0)) {
        for q in 0..NP {
            let x = m[q].max(0.0) * f[q] * g;
            (m[q], got[p.sub[q]]) = (m[q] - x, got[p.sub[q]] + x);
        }
    }
    let heat = (0..NP).map(|q| got[p.sub[q]] * chem.subs[p.sub[q]].cp).sum::<f64>() * t;
    (got, heat)
}

/// The share `f` of the cover of cell `i` of `d` breaks: it goes to the cell's ground as remains, with its heat,
/// through the chunk's moves (opened by the caller for the cover's node).
pub(crate) fn lose(w: &crate::World, d: &mut crate::life::Chunk, p: &Parts, i: usize, f: f64) {
    let c = d.cover(i);
    let t = c.temp(&w.chem, p);
    let mut dead = [0.0; NP];
    for m in &mut c.mol {
        (0..NP).for_each(|q| dead[q] += m[q] * f);
        m.iter_mut().for_each(|a| *a -= *a * f);
    }
    let heat = ((0..NP).map(|q| dead[q] * w.chem.subs[p.sub[q]].cp).sum::<f64>() * t).min(c.e);
    c.e -= heat;
    let ground = crate::store::Node::ground(i);
    (0..NP).for_each(|q| d.moves.give(w, ground, p.sub[q], dead[q], 0.0));
    d.moves.give(w, ground, 0, 0.0, heat);
}
