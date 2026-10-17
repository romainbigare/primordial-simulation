//! Intake: what a body takes in, by its uptake selectivity, within the room of its stores (`Body::wanted`); Choose
//! takes from the ground and from bodies by it, Bond from parts.

use super::{Body, Layout, MAX_TAKE_SHARE};
use crate::chem::Chemistry;

impl Body {
    /// Can the body take in substance `k`: body matter and the solids bodies build only if it breaks them
    /// (`Body::breaks`), as it digests what it can break and nothing else; a body without the catalyst takes none.
    pub(crate) fn can_take(&self, chem: &Chemistry, k: usize) -> bool {
        (chem.matter() != Some(k) && !chem.is_material(k)) || self.breaks(chem, k)
    }

    /// Amounts (mol) to take from `from` by the body's uptake selectivity, up to `kg` in all, for a body of `mass` kg,
    /// at most `MAX_TAKE_SHARE` of what is there, within the room of its stores (`Body::level`, `digest_room`).
    pub(crate) fn wanted(&self, chem: &Chemistry, from: impl Fn(usize) -> f64, kg: f64, mass: f64) -> Vec<f64> {
        let lay = Layout::of(chem);
        let from = |k: usize| from(k).max(0.0);
        let up = |k: usize| if from(k) > 0.0 && self.can_take(chem, k) { self.gene(lay.upt + k).max(0.0) } else { 0.0 };
        let sum: f64 = (0..chem.ns()).map(up).sum();
        // (A solid bodies build it takes past its own level: it holds it until its reactions break it down, and what is
        // left passes back to the ground at its update, `live::shed`.)
        let room = |k: usize| if chem.is_material(k) { f64::MAX } else { (self.level(chem, k, mass) - self.store[k]).max(0.0) };
        let mut m: Vec<f64> = (0..chem.ns()).map(|k| if up(k) > 0.0 { (kg * up(k) / sum / chem.subs[k].molar_mass).min(MAX_TAKE_SHARE * from(k)).min(room(k)) } else { 0.0 }).collect();
        self.digest_room(chem, &mut m, mass);
        m
    }

    /// Body matter taken in is digested into what it was built of as it comes in (`Body::digest`), so it fills the room
    /// of those stores (`Body::level`), not its own: amounts `m` (mol) for a body of `mass` kg are cut together until
    /// what is taken of the fuels and what the matter becomes fit their room (held only to its own room, an eater would
    /// fill its store of the fuel past its level, its mass would grow, and with it the room, without end).
    pub(crate) fn digest_room(&self, chem: &Chemistry, m: &mut [f64], mass: f64) {
        let (Some(bl), Some(km)) = (chem.build.as_ref(), chem.matter()) else { return };
        if m[km] <= 0.0 {
            return;
        }
        let room = |k: usize| (self.level(chem, k, mass) - self.store[k]).max(0.0);
        let per = bl.outputs[0].1 as f64;
        // (Only the fuels: the minerals it also makes of the matter it sheds when it updates, `live::shed`.)
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
