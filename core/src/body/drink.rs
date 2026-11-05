//! Its key liquids: a body drinks them from its ground (`drink`, `sip`) and sweats or boils them off (`evaporate`)
//! (laws.md 6.3, 6.4).

use super::{Body, Layout};
use crate::chem::{Phase, Stores};
use crate::store::Node;
use crate::units::BODY_STORES;
use crate::{life::Chunk, World};

/// First in its sub-step, it drinks its key liquids from its cell's ground up to the level it keeps (`Body::level`), as
/// it drinks while it loses and builds over the time: its physics runs at the activity of what it could drink then
/// (`Body::activity`), whatever the length of the sub-step. Each day at most its uptake gene's share of that level
/// (`sip`); what that leaves of the sub-step's days, in `sips`, it drinks as it sweats (`evaporate`), so it sweats in a
/// sub-step what it would drink and sweat in shorter ones.
pub(super) fn drink(w: &World, d: &mut Chunk, b: &mut Body, days: f64, sips: &mut Stores) {
    let (chem, lay, mass) = (&w.chem, Layout::of(&w.chem), b.mass(&w.chem));
    *sips = [0.0; BODY_STORES];
    for s in b.wet_ks() {
        let (level, most) = (b.level(chem, s, mass), b.gene(lay.upt + s).max(0.0));
        let all = most * level * days;
        sips[s] = all - sip(w, d, b, s, (level - b.store[s]).max(0.0).min(all));
    }
}

/// It drinks up to `x` mol of its key liquid at store `s` from its cell's ground, as far as the ground holds it liquid,
/// paying to handle it (`c_handle` per kg); it comes with its heat. Returns the mol drunk.
fn sip(w: &World, d: &mut Chunk, b: &mut Body, s: usize, x: f64) -> f64 {
    let (chem, i) = (&w.chem, b.cell as usize);
    let (k, tg) = (chem.body().subs()[s], w.t_ground_last(i));
    let x = x.min(d.moves.amount(w, Node::ground(i), k));
    if x <= 0.0 || chem.phase(k, tg) != Phase::Liquid || !super::pay(w, d, b, w.cfg.phys.c_handle * x * chem.subs[k].molar_mass) {
        return 0.0;
    }
    let cp = chem.subs[k].cp;
    let want = x * (cp * tg + chem.subs[k].lf);
    let cap = d.moves.heat_cap_ground(w, i);
    let (got, heat) = d.moves.take_from_ground(w, i, (k, x, cp), want, cap);
    (b.store[s], b.e) = (b.store[s] + got, b.e + heat);
    got
}

/// It evaporates its key liquids to take `q` J away, at each one's vaporisation enthalpy, never below its safe minimum
/// when it sweats (`sips`: what it may still drink in the sub-step, which it drinks first as far as the sweat needs,
/// `sip`; boiling, none, takes all): the vapour goes to its cell's bodies' node with its heat and its latent heat
/// (`Moves::gas`). Returns the heat taken away, J.
pub(super) fn evaporate(w: &World, d: &mut Chunk, b: &mut Body, q: f64, mut sips: Option<&mut Stores>) -> f64 {
    let (chem, i) = (&w.chem, b.cell as usize);
    let (lay, size, lift) = (Layout::of(chem), b.size(chem), w.lift(i));
    let mut taken = 0.0;
    for p in b.wet_ks() {
        let k = chem.body().subs()[p];
        let s = &chem.subs[k];
        let kept = if sips.is_some() { b.gene(lay.min + p).max(0.0) * size / s.molar_mass } else { 0.0 };
        if let Some(left) = sips.as_deref_mut() {
            let more = ((q - taken) / s.lv - (b.store[p] - kept).max(0.0)).min(left[p]);
            if more > 0.0 {
                left[p] -= sip(w, d, b, p, more);
            }
        }
        let (t, free) = (b.temp(chem), (b.store[p] - kept).max(0.0));
        let x = free.min((q - taken) / s.lv);
        if x <= 0.0 {
            continue;
        }
        b.store[p] -= x;
        d.moves.gas(w, k, x, lift);
        d.moves.give(w, Node::bodies(i), 0, 0.0, x * s.cp * t);
        // (Its own heat held the liquid's sensible and fusion heat; the vapour takes its vaporisation heat too.)
        b.e -= x * (s.cp * t + s.latent_gas());
        taken += x * s.lv;
    }
    taken
}
