//! The cover's side of the contract (`contract`): what it stands on its cell as (`Cover::stand`), what a part of it and
//! the seeds lying show a taker (`Cover::shown`, `Cover::shown_lying`), and what a take and a press do to it
//! (`Cover::take`, `Cover::take_lying`, `lose`). All of it is read from
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
    /// What the share `of(x)` of each slot `x` shows a taker (`contract::Shown`): its mass and what it holds of each
    /// substance.
    pub fn shown(&self, chem: &Chemistry, p: &Parts, of: impl Fn(usize) -> f64) -> Shown {
        shown_of(p, &p.mm(chem), self.mol.iter().enumerate().map(|(x, m)| (m, of(x))))
    }
    /// What the share `of` of the seeds lying on its ground shows a taker: its mass and what it holds of each substance.
    pub fn shown_lying(&self, chem: &Chemistry, p: &Parts, of: f64) -> Shown {
        shown_of(p, &p.mm(chem), self.lying.mol.iter().map(|m| (m, of)))
    }
    /// The height the plants of slot `x` stand at, m (`Cover::height`, its walls at their own density).
    pub fn stands_at(&self, chem: &Chemistry, ph: &crate::Physics, p: &Parts, x: usize) -> f64 {
        self.height(ph, &p.mm(chem), chem.subs[p.sub[WALL]].density[0], x)
    }
    /// The energy to break all of it, J: each part's volume times its breaking stress, its living cover (soft tissue
    /// with its liquid) at `rho_body` as body matter (`Chemistry::matter`), and the solids it builds
    /// (`Chemistry::materials`).
    pub fn breaking(&self, chem: &Chemistry, cfg: &crate::Config, p: &Parts) -> f64 {
        let Some(m) = chem.matter() else { return 0.0 };
        let sum = |q: usize| self.mol.iter().map(|m| m[q]).sum::<f64>();
        let mm = p.mm(chem);
        let living: f64 = (0..SLOTS).map(|x| self.living(&mm, x)).sum();
        let mol = |k: usize| p.sub.iter().position(|&q| q == k).map_or(0.0, sum);
        let stress = |v: f64, k: usize| v * chem.subs[k].rupture;
        let soft = stress(living.max(0.0) / cfg.phys.rho_body, m);
        chem.materials().map(|k| stress(mol(k).max(0.0) * chem.subs[k].molar_mass / chem.subs[k].density[0], k)).fold(soft, |a, x| a + x)
    }
    /// `mol` of substance `k` taken from the share `of(x)` of each slot `x`, from each in proportion to what that share
    /// holds of it: returns what is taken, mol (no more than they hold), and the heat that goes with it, J (at its
    /// temperature).
    pub(crate) fn take(&mut self, chem: &Chemistry, p: &Parts, (k, mol): (usize, f64), of: impl Fn(usize) -> f64) -> (f64, f64) {
        let t = self.temp(chem, p);
        let (got, heat) = take_from(chem, p, (k, mol), t, self.mol.iter_mut().enumerate().map(|(x, m)| (m, of(x))));
        let heat = heat.min(self.e);
        self.e -= heat;
        (got, heat)
    }
    /// `mol` of substance `k` taken from the share `of` of the seeds lying on its ground, from each lot in proportion to
    /// what it holds of it: returns what is taken, mol, and its heat, J (`take`).
    pub(crate) fn take_lying(&mut self, chem: &Chemistry, p: &Parts, (k, mol): (usize, f64), of: f64) -> (f64, f64) {
        let t = self.temp(chem, p);
        let (got, heat) = take_from(chem, p, (k, mol), t, self.lying.mol.iter_mut().map(|m| (m, of)));
        let heat = heat.min(self.e);
        self.e -= heat;
        (got, heat)
    }
}

/// What the shares `of` (amounts, mol of each part, and the share of them shown) show a taker: their mass and what
/// they hold of each substance.
fn shown_of<'a>(p: &Parts, mm: &[f64; NP], of: impl Iterator<Item = (&'a [f64; NP], f64)>) -> Shown {
    let (mut kg, mut mol) = (0.0, [0.0; MAX_SUBSTANCES]);
    for (m, f) in of.filter(|e| e.1 > 0.0) {
        kg += (0..NP).map(|q| m[q] * mm[q]).sum::<f64>() * f;
        (0..NP).for_each(|q| mol[p.sub[q]] += m[q] * f);
    }
    Shown { kg, mol }
}

/// `mol` of substance `k` taken at `t` K from the shares `of` (amounts, mol of each part, and the share of them that
/// can be taken), from each in proportion to what its share holds of it: what is taken, mol (no more than the shares
/// hold), and the heat that goes with it, J.
fn take_from<'a>(chem: &Chemistry, p: &Parts, (k, mol): (usize, f64), t: f64, of: impl Iterator<Item = (&'a mut [f64; NP], f64)>) -> (f64, f64) {
    let Some(q) = p.sub.iter().position(|&s| s == k) else { return (0.0, 0.0) };
    let mut of: Vec<(&mut [f64; NP], f64)> = of.filter(|e| e.1 > 0.0).collect();
    let all: f64 = of.iter().map(|(m, f)| m[q] * f).sum();
    if all <= 0.0 || mol <= 0.0 {
        return (0.0, 0.0);
    }
    let g = (mol / all).min(1.0);
    of.iter_mut().for_each(|(m, f)| m[q] -= m[q] * *f * g);
    (mol.min(all), mol.min(all) * chem.subs[k].cp * t)
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
