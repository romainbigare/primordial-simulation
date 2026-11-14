//! A month of the takers of each cell of a chunk (laws.md 3b): the slots find what they reach of the cover (`take`),
//! cut their work to the liquid they have (`thirst`), take one bite of each layer for all, shed the heat their coat
//! cannot pass as vapour, digest what they took and pay their upkeep (`gut`), and lose the members they could not keep:
//! unpaid, worn out, frozen, and those whose heat stays unshed even at rest (life's causes `SPENT`, `AGE`, `PHASE`,
//! `LACK`). Their remains go to the ground; everything they take and give is booked through the contract and the books
//! of the cell's life (`life::book`), with its heat.

use super::body::{body, Body};
use super::gut::{digest, Gut};
use super::thirst::{self, Thirst};
use super::{mm, sub, take, Fauna, FS, NF, N_WHOLE, REPAIR};
use crate::contract;
use crate::life::{book, Books, Chunk, Parts, MATTER, NO_ROOT, NP, WET_TISSUE};
use crate::math::one_minus_exp;
use crate::rng::{Purpose, Rng};
use crate::units::DAY;
use crate::World;

/// The month of the takers of every cell of chunk `d` that holds any, in the order of its cells.
pub fn step(w: &World, d: &mut Chunk) {
    let Some(p) = Parts::of(&w.chem) else { return };
    let gut = Gut::of(&w.chem, &p);
    for i in d.cells() {
        if d.at(i).fauna.any() {
            cell(w, d, (&p, &gut), i);
        }
    }
}

/// The month of the takers of cell `i` of chunk `d`.
fn cell(w: &World, d: &mut Chunk, (p, gut): (&Parts, &Gut), i: usize) {
    let (chem, dt, t) = (&w.chem, w.cfg.calendar().month as f64, contract::place(w, i).t);
    let (area, lv) = (w.mesh.cells().area()[i], chem.subs[p.liquid].lv);
    let (sub, mm) = (sub(p), mm(chem, p));
    contract::open(w, d, i);
    let bodies: [Body; FS] = {
        let f = &d.at(i).fauna;
        std::array::from_fn(|x| body(w, p, f, (x, &mm), (i, t), gut.per_wall(chem)))
    };
    // What they would take at full work, and the liquid of it: their work is cut to the liquid they have.
    let layers = take::layers(w, d, i);
    let (stock, breaking, share) = take::reachable(w, i, &layers, &bodies);
    let days = dt / DAY;
    let (full, searching) = take::scheme(&take::eaters(&bodies, &share, &[1.0; FS]), (&stock, &breaking), area, days);
    let wet: Vec<f64> = layers.iter().map(|f| f.shown.mol[p.sub[MATTER]] * chem.subs[p.sub[MATTER]].molar_mass / f.shown.kg * WET_TISSUE / (1.0 - WET_TISSUE) / chem.subs[p.liquid].molar_mass).collect();
    let walks: [f64; FS] = std::array::from_fn(|x| bodies[x].speed * searching[x] / DAY);
    let thirsts: [Thirst; FS] = std::array::from_fn(|x| Thirst::of(&bodies[x], walks[x], (dt, lv), full[x].iter().zip(&wet).map(|(k, l)| k * l).sum()));
    let found = contract::liquid(w, d, i);
    let given = thirst::shares(&thirsts, found.open + found.running + found.soft);
    let (work, lack) = thirst::work(&bodies, &thirsts, &walks, &given, (dt, lv));
    let (taken, searching) = take::scheme(&take::eaters(&bodies, &share, &work), (&stock, &breaking), area, days);
    let walks: [f64; FS] = std::array::from_fn(|x| bodies[x].speed * searching[x] / DAY);
    let (took, heat) = take::bite(w, d, i, &layers, &taken);
    // The vapour they shed leaves the cell's liquid (the open liquid, what ran over it and what they ate join it there),
    // then the soft solid; the liquid their guts split walls with leaves it too.
    let shed: f64 = (0..FS).map(|x| bodies[x].vapour(walks[x], work[x], lv) * bodies[x].n * dt * (1.0 - lack[x])).sum();
    let ground = found.open + found.pores;
    let drunk = shed.min(ground);
    let paid = contract::vapour(w, d, i, [drunk, (shed - drunk).min(found.soft)]);
    let mut liquid = ground - drunk;
    let mut b: Books<0> = Books::new(contract::air(w, d, i), []);
    let (f, moves, counts) = d.fauna_with(i);
    let e = f.heat() + heat.iter().sum::<f64>() - paid;
    for x in (0..FS).filter(|&x| bodies[x].n > 0.0) {
        let bd = &bodies[x];
        let burnt = bd.heat(walks[x], work[x]);
        let mut part = [0.0; NP];
        (0..NP).for_each(|q| part[q] = took[x][p.sub[q]]);
        let store = w.cfg.phys.store_most * bd.m * bd.n / mm[super::STORE];
        let spent = digest(chem, (p, gut), (f, x), (part, bd.gut * bd.n * dt, &mut liquid), (burnt * bd.n * dt, store), &mut b);
        rounded(f, x, &mut counts.rounded[0]);
        let frozen = w.cfg.phys.frozen((chem.subs[p.liquid].melt, w.cfg.phys.harden), bd.tb, bd.pace.min(1.0));
        let worn = one_minus_exp(bd.wear(&w.cfg.phys, f.genes[x][REPAIR], burnt, dt));
        let mut r = Rng::new(w.seed, w.month(), (i * FS + x) as u64, Purpose::Fauna);
        // The causes in order: unpaid, worn out, frozen, and the heat it could not shed even at rest.
        die(f, (x, &sub), [spent, worn, frozen, lack[x]], &mut r, &mut b);
        if f.adults[x] <= 0.0 {
            (0..NF).for_each(|q| b.dead[sub[q]] += f.mol[x][q]);
            (f.adults[x], f.mol[x], f.root[x]) = (0.0, [0.0; NF], NO_ROOT);
        }
    }
    // Each slot's heat ends at its members' temperature; what the burning released beyond it goes to the ground.
    let target: [f64; FS] = std::array::from_fn(|x| f.cap(chem, &sub, x) * bodies[x].tb);
    let all: f64 = target.iter().sum();
    let left = book(w, moves, (i, t), (e, 0.0), all, &b);
    let share = if all > 0.0 { left / all } else { 0.0 };
    f.e = target.map(|x| x * share);
}

/// What rounding left below zero of slot `x` of `f` is set to zero and counted in `rounded`, mol.
fn rounded(f: &mut Fauna, x: usize, rounded: &mut f64) {
    for m in f.mol[x].iter_mut().filter(|m| **m < 0.0) {
        *rounded -= std::mem::take(m);
    }
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
