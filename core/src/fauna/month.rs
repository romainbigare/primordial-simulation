//! A month of the takers of each cell of a chunk (laws.md 3b): each slot pays its upkeep and keeps its body's heat by
//! burning its store, sheds the heat its coat cannot pass as vapour of the open liquid of its cell, and loses the
//! members it could not keep: unpaid, worn out, frozen and short of liquid (life's causes `SPENT`, `AGE`, `PHASE`,
//! `LACK`). Its
//! remains go to its ground; everything it takes and gives is booked through the contract and the books of the cell's
//! life (`life::book`), with its heat.

use super::body::{need, Need};
use super::{mm, sub, Fauna, FS, NF, N_WHOLE, STORE};
use crate::contract;
use crate::life::{book, Books, Chunk, Parts, NO_ROOT};
use crate::math::one_minus_exp;
use crate::rng::{Purpose, Rng};
use crate::World;

/// The month of the takers of every cell of chunk `d` that holds any, in the order of its cells.
pub fn step(w: &World, d: &mut Chunk) {
    let Some(p) = Parts::of(&w.chem) else { return };
    for i in d.cells() {
        if d.at(i).fauna.any() {
            cell(w, d, &p, i);
        }
    }
}

/// The month of the takers of cell `i` of chunk `d`.
fn cell(w: &World, d: &mut Chunk, p: &Parts, i: usize) {
    let (chem, dt, t) = (&w.chem, w.cfg.calendar().month as f64, contract::place(w, i).t);
    let (sub, mm) = (sub(p), mm(chem, p));
    contract::open(w, d, i);
    let needs: [Need; FS] = {
        let f = &d.at(i).fauna;
        std::array::from_fn(|x| need(w, p, f, (x, &mm), (t, dt)))
    };
    let met = thirst(w, d, i, &needs);
    let mut b: Books<0> = Books::new(contract::air(w, d, i), []);
    let (f, moves, counts) = d.fauna_with(i);
    let e = f.heat() - met.1;
    for x in (0..FS).filter(|&x| needs[x].mass > 0.0) {
        let spent = burn(w, p, f, (x, &sub), needs[x].burn, &mut b);
        let m = f.mol[x][STORE];
        if m < 0.0 {
            // (What rounding left below zero is set to zero and counted.)
            (f.mol[x][STORE], counts.rounded[0]) = (0.0, counts.rounded[0] - m);
        }
        let frozen = w.cfg.phys.frozen((chem.subs[p.liquid].melt, w.cfg.phys.harden), needs[x].tb, needs[x].pace.min(1.0));
        let mut r = Rng::new(w.seed, w.month(), (i * FS + x) as u64, Purpose::Fauna);
        // The causes in order: unpaid, worn out, frozen, and the heat it could not shed for want of liquid.
        let lack = if needs[x].vapour > 0.0 { 1.0 - met.0 } else { 0.0 };
        die(f, (x, &sub), [spent, one_minus_exp(needs[x].wear), frozen, lack], &mut r, &mut b);
        if f.adults[x] <= 0.0 {
            (0..NF).for_each(|q| b.dead[sub[q]] += f.mol[x][q]);
            (f.adults[x], f.mol[x], f.root[x]) = (0.0, [0.0; NF], NO_ROOT);
        }
    }
    // Each slot's heat ends at its members' temperature; what the burning released beyond it goes to the ground.
    let target: [f64; FS] = std::array::from_fn(|x| f.cap(chem, &sub, x) * needs[x].tb);
    let all: f64 = target.iter().sum();
    let left = book(w, moves, (i, t), (e, 0.0), all, &b);
    let share = if all > 0.0 { left / all } else { 0.0 };
    f.e = target.map(|x| x * share);
}

/// The key liquid the slots of cell `i` need in the month (`Need`): their vapour and the least they pass through,
/// which the open liquid and the soft solid lying on the ground (`contract::liquid`) meet by a share common to all
/// slots. The vapour that share sheds goes to the air (`contract::vapour`), the open liquid first; the rest of what
/// they drink goes back to the cell in the month. Of a slot that must shed heat as vapour, the members whose heat that
/// share leaves unshed die (`LACK`). Returns the share met and the heat the vapour took from the bodies, J.
fn thirst(w: &World, d: &mut Chunk, i: usize, needs: &[Need; FS]) -> (f64, f64) {
    let (vapour, least): (f64, f64) = (needs.iter().map(|n| n.vapour).sum(), needs.iter().map(|n| n.least).sum());
    if vapour + least <= 0.0 {
        return (1.0, 0.0);
    }
    let found = contract::liquid(w, d, i);
    let met = ((found.open + found.soft) / (vapour + least)).min(1.0);
    let shed = vapour * met;
    let open = shed.min(found.open);
    (met, contract::vapour(w, d, i, [open, (shed - open).min(found.soft)]))
}

/// Slot `x` of `f` (its parts the substances `sub`) burns its store for `energy` J (`life::Books::run` of the burning), no
/// more than its store and the gas the cell's share of its air node offers. Returns the share of the energy unpaid.
fn burn(w: &World, p: &Parts, f: &mut Fauna, (x, sub): (usize, &[usize; NF]), energy: f64, b: &mut Books<0>) -> f64 {
    let rx = &w.chem.reactions[p.burn];
    let per = rx.inputs.iter().find(|q| q.0 as usize == sub[STORE]).map_or(1.0, |q| q.1 as f64);
    let units = energy / -w.chem.dh(p.burn);
    let burnt = (units * per).min(f.mol[x][STORE].max(0.0)).min(b.room(sub, rx) * per);
    b.run(&w.chem, sub, &mut f.mol[x], rx, burnt / per);
    if units > 0.0 { 1.0 - burnt / (units * per) } else { 0.0 }
}

/// The members of slot `x` of `f` (its parts the substances `sub`) die by the shares `causes`, each on what the one
/// before left (as the cover's causes combine); their share of its body matter and store goes to the cell's remains
/// (`Books::dead`). Below `N_WHOLE` members the count is whole: the dead are a binomial draw from `r`, and a count that
/// falls below it becomes whole by a draw that keeps its mean (it is never rounded up as a rule).
fn die(f: &mut Fauna, (x, sub): (usize, &[usize; NF]), causes: [f64; 4], r: &mut Rng, b: &mut Books<0>) {
    let n = f.adults[x];
    let left: f64 = causes.iter().map(|c| 1.0 - c.clamp(0.0, 1.0)).product();
    let dead = if n < N_WHOLE { r.binomial(n as u32, 1.0 - left) as f64 } else { n * (1.0 - left) };
    let mut after = n - dead;
    if after < N_WHOLE && after.fract() != 0.0 {
        after = after.floor() + (r.f64() < after.fract()) as u8 as f64;
    }
    let gone = if n > 0.0 { dead / n } else { 0.0 };
    for (a, &k) in f.mol[x].iter_mut().zip(sub) {
        let m = *a * gone;
        *a -= m;
        b.dead[k] += m;
    }
    f.adults[x] = after;
}
