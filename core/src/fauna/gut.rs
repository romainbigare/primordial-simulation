//! What a slot of takers does with what it took (laws.md 3b), by the chemistry's own reactions: its gut splits walls
//! into fuel up to its capacity (the reaction that makes fuel of the walls alone, taking the key liquid), it burns the
//! fuel it took and split for its upkeep first, then the body matter it took (the burning of body matter, which gives
//! back its mineral at once), then its store; the fuel it took beyond its need goes to its store and the body matter
//! to its growth (`grow`); the walls it did not split pass to the ground.

use super::{Fauna, NF, STORE};
use crate::chem::Chemistry;
use crate::life::{Books, Parts, FUEL, MATTER, NP, WALL};

/// The reactions a gut runs, found from the chemistry: the splitting of walls into fuel (if any) and the fuel it makes
/// a unit, the burning of the fuel and the fuel a unit takes, and the burning of body matter (if any).
pub(super) struct Gut {
    pub(super) split: Option<(usize, f64)>,
    burn: (usize, f64),
    matter: Option<usize>,
}

impl Gut {
    /// The gut of life of parts `p` in `chem`.
    pub(super) fn of(chem: &Chemistry, p: &Parts) -> Gut {
        let has = |v: &[(u16, u16)], k: usize| v.iter().find(|q| q.0 as usize == k).map(|q| q.1 as f64);
        let split = chem.reactions.iter().position(|r| has(&r.inputs, p.sub[WALL]).is_some() && has(&r.outputs, p.sub[FUEL]).is_some());
        let split = split.map(|k| (k, has(&chem.reactions[k].outputs, p.sub[FUEL]).unwrap_or(0.0) / has(&chem.reactions[k].inputs, p.sub[WALL]).unwrap_or(1.0)));
        let burn = (p.burn, has(&chem.reactions[p.burn].inputs, p.sub[FUEL]).unwrap_or(1.0));
        Gut { split, burn, matter: chem.matter_roles().map(|r| r.1) }
    }
    /// What a mol of fuel frees burnt, J.
    pub(super) fn fuel(&self, chem: &Chemistry) -> f64 {
        -chem.dh(self.burn.0) / self.burn.1
    }
    /// The fuel a mol of walls split makes, J of it burnt (none without a splitting reaction).
    pub(super) fn per_wall(&self, chem: &Chemistry) -> f64 {
        self.split.map_or(0.0, |s| s.1 * self.fuel(chem))
    }
}

/// What a group of takers kept of what it took in a month (`digest`): the share of its energy unpaid, the body matter
/// it took and did not burn, mol, and the fuel its store gained, mol (none where it drew on its store).
pub(super) struct Digested {
    pub(super) unpaid: f64,
    pub(super) matter: f64,
    pub(super) gained: f64,
}

/// Group `x` of `f` digests `took` (mol of life's parts) over the month: its gut splits up to `most` mol of walls, each
/// unit taking the key liquid the cell's ground still holds (`liquid`, mol, lessened by it); it burns for `energy` J the
/// fuel it took and split, then the body matter it took, then its store, each no more than the gas the cell's share of
/// its air node offers; the fuel it took and did not burn goes to its store; the walls it did not split go to the
/// remains (`Books::dead`). The body matter left is for growth (`grow`).
pub(super) fn digest(chem: &Chemistry, (p, gut): (&Parts, &Gut), (f, x): (&mut Fauna, usize), (mut took, most, liquid): ([f64; NP], f64, &mut f64), energy: f64, b: &mut Books<0>) -> Digested {
    if let Some((k, _)) = gut.split {
        let rx = &chem.reactions[k];
        let per_liquid = rx.inputs.iter().find(|q| q.0 as usize == p.liquid).map_or(0.0, |q| q.1 as f64);
        let per_wall = rx.inputs.iter().find(|q| q.0 as usize == p.sub[WALL]).map_or(1.0, |q| q.1 as f64);
        let units = (took[WALL].min(most) / per_wall).min(if per_liquid > 0.0 { *liquid / per_liquid } else { f64::INFINITY }).max(0.0);
        b.run(chem, &p.sub, &mut took, rx, units);
        *liquid -= units * per_liquid;
    }
    let sub = super::sub(p);
    let mut eaten = [took[FUEL].max(0.0), 0.0];
    let mut left = burn(chem, gut, (&sub, &mut eaten), energy, b);
    if let Some(k) = gut.matter {
        let rx = &chem.reactions[k];
        let per = rx.inputs.iter().find(|q| q.0 as usize == p.sub[MATTER]).map_or(1.0, |q| q.1 as f64);
        let units = (took[MATTER].max(0.0) / per).min(left / -chem.dh(k)).min(b.room(&p.sub, rx));
        b.run(chem, &p.sub, &mut took, rx, units);
        left -= units * -chem.dh(k);
    }
    let before = f.mol[x][STORE];
    f.mol[x][STORE] += eaten[STORE];
    let unpaid = burn(chem, gut, (&sub, &mut f.mol[x]), left, b);
    b.dead[p.sub[WALL]] += took[WALL].max(0.0);
    let unpaid = if energy > 0.0 { unpaid / energy } else { 0.0 };
    Digested { unpaid, matter: took[MATTER].max(0.0), gained: (f.mol[x][STORE] - before).max(0.0) }
}

/// Amounts `mol` (of the taker's parts, the substances `sub`) burn their store for `energy` J (the burning of the fuel,
/// `life::Books::run`), no more than the store and the gas the cell's share of its air node offers. Returns the energy
/// unpaid, J.
pub(super) fn burn(chem: &Chemistry, gut: &Gut, (sub, mol): (&[usize; NF], &mut [f64; NF]), energy: f64, b: &mut Books<0>) -> f64 {
    let (rx, per) = (&chem.reactions[gut.burn.0], gut.burn.1);
    let units = energy.max(0.0) / -chem.dh(gut.burn.0);
    let burnt = (units * per).min(mol[STORE].max(0.0)).min(b.room(sub, rx) * per);
    b.run(chem, sub, mol, rx, burnt / per);
    if units > 0.0 { energy * (1.0 - burnt / (units * per)) } else { 0.0 }
}
