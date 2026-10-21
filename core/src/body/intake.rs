//! Intake: what a body takes in by its uptake selectivity, within the room of its stores (`Body::wanted`).

use super::{Body, Layout};
use crate::chem::Chemistry;

impl Body {
    /// Does it use `k` at `t`, so that it takes it in: body matter and built solids if a catalyst of its own breaks
    /// them (`Consts::broken`: it digests them), and what it burns (`burns_at`) or builds of (`Chemistry::minerals`)? A solid none of its
    /// reactions takes is of no use to it, nor are the key liquids it does not burn: it drinks them (`physics::soak`).
    pub(crate) fn uses(&self, chem: &Chemistry, k: usize, t: f64) -> bool {
        let built = chem.matter() == Some(k) || chem.is_material(k);
        if built { self.k.broken >> k & 1 == 1 } else { self.burns_at(chem, k, t) || chem.minerals().iter().any(|m| m.0 == k) }
    }

    /// The room of its store of `k` in one update, mol: up to its level once for each of the `paces` intervals of its
    /// gene, so a store in use refills at a level an interval (laws.md 4.4); a key liquid's once, as `physics::soak`
    /// solves its losses over the time.
    pub(crate) fn room(&self, chem: &Chemistry, paces: f64, k: usize, mass: f64) -> f64 {
        let gap = (self.level(chem, k, mass) - self.store[k]).max(0.0);
        if self.wet >> k & 1 == 1 { gap } else { gap * paces }
    }

    /// Amounts (mol) of what it uses (`uses`) to take from `from` by uptake selectivity, up to `kg` in all, no more
    /// than is there and within the room of its stores.
    pub(crate) fn wanted(&self, chem: &Chemistry, paces: f64, from: impl Fn(usize) -> f64, kg: f64, mass: f64) -> Vec<f64> {
        let (lay, t, from) = (Layout::of(chem), self.temp(chem), |k: usize| from(k).max(0.0));
        let up = |k: usize| if from(k) > 0.0 && self.uses(chem, k, t) { self.gene(lay.upt + k).max(0.0) } else { 0.0 };
        let mut m: Vec<f64> = (0..chem.ns()).map(up).collect();
        let sum: f64 = m.iter().sum();
        // A built solid it takes past its own level until its reactions break it down; the rest it sheds.
        let room = |k: usize| if chem.is_material(k) { f64::MAX } else { self.room(chem, paces, k, mass) };
        for (k, x) in m.iter_mut().enumerate().filter(|x| *x.1 > 0.0) {
            *x = (kg * *x / sum / chem.subs[k].molar_mass).min(from(k)).min(room(k));
        }
        self.digest_room(chem, paces, &mut m, mass);
        m
    }

    /// Body matter taken in is digested into what it was built of, so it fills those stores' room: amounts `m` are cut
    /// together until the fuels and what the matter becomes fit (else an eater's room would grow without end).
    pub(crate) fn digest_room(&self, chem: &Chemistry, paces: f64, m: &mut [f64], mass: f64) {
        let (Some(bl), Some(km)) = (chem.build.as_ref(), chem.matter()) else { return };
        if m[km] <= 0.0 {
            return;
        }
        let room = |k: usize| self.room(chem, paces, k, mass);
        let per = bl.outputs[0].1 as f64;
        // Only the fuels: the minerals it makes of the matter it sheds.
        let minerals = chem.minerals();
        let fuel = |k: u16| !minerals.iter().any(|m| m.0 == k as usize);
        let f = bl.inputs.iter().filter(|p| fuel(p.0)).map(|&(k, nu)| room(k as usize) / (m[k as usize] + nu as f64 / per * m[km]).max(1e-300)).fold(1.0, f64::min);
        if f < 1.0 {
            for &(k, _) in bl.inputs.iter().filter(|p| fuel(p.0)) {
                m[k as usize] *= f;
            }
            m[km] *= f;
        }
    }
}
