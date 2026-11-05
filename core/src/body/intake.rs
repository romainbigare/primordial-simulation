//! Intake: what a body takes in by its uptake selectivity, within the room of its stores over its update
//! (`Body::wanted`).

use super::{Body, Layout};
use crate::chem::{Chemistry, Stores};
use crate::units::BODY_STORES;
use crate::World;

impl Body {
    /// Does it use substance `k` at `t`, so that it takes it in: body matter and built solids if a catalyst of its own
    /// breaks them (`Consts::broken`: it digests them), and what it burns (`burns_at`) or builds of
    /// (`Chemistry::minerals`)? A solid none of its reactions takes is of no use to it, nor are the key liquids it does
    /// not burn: it drinks them (`live::drink`). All it uses are among its stores (`Chemistry::body`).
    pub(crate) fn uses(&self, chem: &Chemistry, k: usize, t: f64) -> bool {
        let built = chem.matter() == Some(k) || chem.is_material(k);
        if built { self.k.broken >> k & 1 == 1 } else { self.burns_at(chem, k, t) || chem.minerals().iter().any(|m| m.0 == k) }
    }

    /// The room of its store `s` for the time a meal feeds it, mol (laws.md 6.7): up to its level and, of a store it
    /// burns, what it burns of it over that time (`ahead`, J, its upkeep then, `body::ahead`), as the short steps of the
    /// same time each eat what they burnt. So what a long meal takes at once is what those take.
    pub(crate) fn room(&self, chem: &Chemistry, s: usize, mass: f64, ahead: f64) -> f64 {
        let (k, t) = (chem.body().subs()[s], self.temp(chem));
        let burnt = if ahead > 0.0 && self.burns_at(chem, k, t) { ahead / self.heat_of(chem, k, t).max(1e-300) } else { 0.0 };
        (self.level(chem, s, mass) - self.store[s]).max(0.0) + burnt
    }

    /// Amounts (mol of each store) of what it uses (`uses`) to take from `from` (mol of each substance) by uptake
    /// selectivity, up to `kg` in all, no more than is there and within the room of its stores for the `days` d what it
    /// takes feeds it (`room`).
    pub(crate) fn wanted(&self, w: &World, from: impl Fn(usize) -> f64, (kg, days): (f64, f64), mass: f64) -> Stores {
        let chem = &w.chem;
        let (lay, t, bc, from) = (Layout::of(chem), self.temp(chem), chem.body(), |k: usize| from(k).max(0.0));
        let up = |s: usize, k: usize| if from(k) > 0.0 && self.uses(chem, k, t) { self.gene(lay.upt + s).max(0.0) } else { 0.0 };
        let mut m = [0.0; BODY_STORES];
        bc.subs().iter().zip(&mut m).enumerate().for_each(|(s, (&k, x))| *x = up(s, k));
        let sum: f64 = m.iter().sum();
        let ahead = super::ahead(w, self, mass, days);
        // A built solid it takes past its own level until its reactions break it down; the rest it sheds.
        let room = |s: usize| if chem.is_material(bc.subs()[s]) { f64::MAX } else { self.room(chem, s, mass, ahead) };
        for (s, x) in m.iter_mut().enumerate().filter(|x| *x.1 > 0.0) {
            let k = bc.subs()[s];
            *x = (kg * *x / sum / bc.mm[s]).min(from(k)).min(room(s));
        }
        self.digest_room(chem, &mut m, mass, ahead);
        m
    }

    /// Body matter taken in is digested into what it was built of, so it fills those stores' room (`room`, with what it
    /// burns for the time the meal feeds it, `ahead` J): amounts `m` (mol of each store) are cut together until the
    /// fuels and what the matter becomes fit (else an eater's room would grow without end).
    pub(crate) fn digest_room(&self, chem: &Chemistry, m: &mut Stores, mass: f64, ahead: f64) {
        let (Some(bl), Some(km)) = (chem.build.as_ref(), Body::matter_at(chem).filter(|&km| m[km] > 0.0)) else { return };
        let (room, bc) = (|s: usize| self.room(chem, s, mass, ahead), chem.body());
        let per = bl.outputs[0].1 as f64;
        // Only the fuels: the minerals it makes of the matter it sheds.
        let minerals = chem.minerals();
        let fuel = |k: u16| !minerals.iter().any(|m| m.0 == k as usize);
        let at = |k: u16| bc.place(k as usize).expect("growth's inputs are stores");
        let f = bl.inputs.iter().filter(|p| fuel(p.0)).map(|&(k, nu)| room(at(k)) / (m[at(k)] + nu as f64 / per * m[km]).max(1e-300)).fold(1.0, f64::min);
        if f < 1.0 {
            for &(k, _) in bl.inputs.iter().filter(|p| fuel(p.0)) {
                m[at(k)] *= f;
            }
            m[km] *= f;
        }
    }
}
