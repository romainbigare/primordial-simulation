//! The cover's side of the contract (`contract`): what it stands on its cell as (`Cover::stand`), what a part of it
//! shows a taker (`Cover::shown`), and what a take and a press do to it (`Cover::take`, `lose`). All of it is read from
//! the cover's own numbers when asked; it keeps no copy.

use super::{Cover, Parts, NP, SLOTS, WALL};
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
    /// What its slots `of` show a taker (`contract::Shown`): their mass and what they hold of each substance.
    pub fn shown(&self, chem: &Chemistry, p: &Parts, of: impl Fn(usize) -> bool) -> Shown {
        let mm = p.mm(chem);
        let (mut kg, mut mol) = (0.0, [0.0; MAX_SUBSTANCES]);
        for x in (0..SLOTS).filter(|&x| of(x)) {
            kg += self.kg(&mm, x);
            (0..NP).for_each(|q| mol[p.sub[q]] += self.mol[x][q]);
        }
        Shown { kg, mol }
    }
    /// The height the plants of slot `x` stand at, m (`Cover::height`, its walls at their own density).
    pub fn stands_at(&self, chem: &Chemistry, ph: &crate::Physics, p: &Parts, x: usize) -> f64 {
        self.height(ph, &p.mm(chem), chem.subs[p.sub[WALL]].density[0], x)
    }
    /// The energy to break all of it, J: its soft tissue (with its liquid) and its solids by their breaking stresses
    /// (`life::solids_of`).
    pub fn breaking(&self, chem: &Chemistry, cfg: &crate::Config, p: &Parts) -> f64 {
        let sum = |q: usize| self.mol.iter().map(|m| m[q]).sum::<f64>();
        let mm = p.mm(chem);
        let living = (0..SLOTS).map(|x| self.living(&mm, x)).sum();
        crate::life::solids_of(chem, cfg, living, |s| p.sub.iter().position(|&q| q == s).map_or(0.0, sum), |_| 1.0)[1]
    }
    /// `mol` of substance `k` taken from its slots `of`, from each in proportion to what it holds of it: returns what is
    /// taken, mol (no more than they hold), and the heat that goes with it, J (at its temperature).
    pub(crate) fn take(&mut self, chem: &Chemistry, p: &Parts, (k, mol): (usize, f64), of: impl Fn(usize) -> bool) -> (f64, f64) {
        let Some(q) = p.sub.iter().position(|&s| s == k) else { return (0.0, 0.0) };
        let all = self.mol.iter().enumerate().filter(|e| of(e.0)).map(|(_, m)| m[q]).sum::<f64>();
        if all <= 0.0 || mol <= 0.0 {
            return (0.0, 0.0);
        }
        let (f, t) = ((mol / all).min(1.0), self.temp(chem, p));
        self.mol.iter_mut().enumerate().filter(|e| of(e.0)).for_each(|(_, m)| m[q] -= m[q] * f);
        let heat = (mol.min(all) * chem.subs[k].cp * t).min(self.e);
        self.e -= heat;
        (mol.min(all), heat)
    }
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
