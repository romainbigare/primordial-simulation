//! A month of the takers of each cell of a chunk (laws.md 3b): the slots find what they reach of the cover (`take`),
//! cut their work to the liquid they have (`thirst`), take one bite of each layer for all, shed the heat their coat
//! cannot pass as vapour, digest what they took and pay their upkeep (`gut`), and lose the members they could not keep:
//! unpaid, worn out, frozen, and those whose heat stays unshed even at rest (life's causes `SPENT`, `AGE`, `PHASE`,
//! `LACK`). Their remains go to the ground; everything they take and give is booked through the contract and the books
//! of the cell's life (`life::book`), with its heat.

use super::body::{body, Body};
use super::gut::{digest, Gut};
use super::thirst::{self, Thirst};
use super::{mm, sub, take, way, Fauna, FS, NF, N_WHOLE, REPAIR};
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

/// The month of the takers of cell `i` of chunk `d`: what they take at the work their liquid allows (`plan`), one bite
/// of each layer for all, their vapour, then each slot's digestion and deaths (`settle`), and the books.
fn cell(w: &World, d: &mut Chunk, (p, gut): (&Parts, &Gut), i: usize) {
    let (chem, dt, t) = (&w.chem, w.cfg.calendar().month as f64, contract::place(w, i).t);
    let (area, lv, days) = (w.mesh.cells().area()[i], chem.subs[p.liquid].lv, dt / DAY);
    let mm = mm(chem, p);
    contract::open(w, d, i);
    let bodies: [Body; FS] = {
        let f = &d.at(i).fauna;
        std::array::from_fn(|x| body(w, p, f, (x, &mm), (i, t), gut.per_wall(chem)))
    };
    let layers = take::layers(w, d, i);
    let reach = take::reachable(w, i, &layers, &bodies);
    let found = contract::liquid(w, d, i);
    let plan = plan(w, p, (i, &bodies), (&layers, &reach), &found);
    let (taken, searching) = take::scheme(&take::eaters(&bodies, &reach.share, &plan.work), (&reach.stock, &reach.breaking), area, days);
    let walks: [f64; FS] = std::array::from_fn(|x| bodies[x].speed * searching[x] / DAY);
    let (took, heat) = take::bite(w, d, i, &layers, &taken);
    // The vapour they shed leaves the cell's liquid (the open liquid, what ran over it, what they ate and fetched join it
    // there), then the soft solid; the liquid their guts split walls with leaves it too.
    let shed: f64 = (0..FS).map(|x| bodies[x].vapour(walks[x], plan.work[x], lv) * bodies[x].n * dt * (1.0 - plan.lack[x])).sum();
    let ground = found.open + found.pores;
    let drunk = shed.min(ground);
    let paid = contract::vapour(w, d, i, [drunk, (shed - drunk).min(found.soft)]);
    let mut b: Books<0> = Books::new(contract::air(w, d, i), []);
    let (f, moves, counts) = d.fauna_with(i);
    let e = f.heat() + heat.iter().sum::<f64>() - paid;
    let mut liquid = ground - drunk;
    for x in (0..FS).filter(|&x| bodies[x].n > 0.0) {
        let burnt = bodies[x].heat(walks[x], plan.work[x]) + plan.way[x] / (bodies[x].n * dt);
        let r = Rng::new(w.seed, w.month(), (i * FS + x) as u64, Purpose::Fauna);
        settle(w, (p, gut), (f, x, &bodies[x]), (&took[x], burnt, plan.lack[x]), (&mut liquid, r, &mut b));
        rounded(f, x, &mut counts.rounded[0]);
    }
    // Each slot's heat ends at its members' temperature; what the burning released beyond it goes to the ground.
    let target: [f64; FS] = std::array::from_fn(|x| f.cap(chem, &sub(p), x) * bodies[x].tb);
    let all: f64 = target.iter().sum();
    let left = book(w, moves, (i, t), (e, 0.0), all, &b);
    let share = if all > 0.0 { left / all } else { 0.0 };
    f.e = target.map(|x| x * share);
}

/// What the slots of cell `i` do for liquid in the month: the share of their work each keeps, the share of its members
/// that die of thirst, and the work of their ways to the liquid, J.
struct Plan {
    work: [f64; FS],
    lack: [f64; FS],
    way: [f64; FS],
}

/// The plan of the slots `bodies` of cell `i` for liquid: what they would take of `layers` at full work and the
/// liquid of it (`WET_TISSUE` of the living mass of the body matter), their need at that work (`Thirst`), their shares of
/// the open liquid `found`, what they fetch by the way to the nearest open liquid of the rest (`way::fetch`), and the
/// work each keeps (`thirst::work`).
fn plan(w: &World, p: &Parts, (i, bodies): (usize, &[Body; FS]), (layers, reach): (&[take::Found], &take::Reach), found: &contract::Liquid) -> Plan {
    let (chem, dt) = (&w.chem, w.cfg.calendar().month as f64);
    let (lv, liquid_mm, days) = (chem.subs[p.liquid].lv, chem.subs[p.liquid].molar_mass, dt / DAY);
    let (full, searching) = take::scheme(&take::eaters(bodies, &reach.share, &[1.0; FS]), (&reach.stock, &reach.breaking), w.mesh.cells().area()[i], days);
    let matter = chem.subs[p.sub[MATTER]].molar_mass;
    let wet: Vec<f64> = layers.iter().map(|f| f.shown.mol[p.sub[MATTER]] * matter / f.shown.kg * WET_TISSUE / (1.0 - WET_TISSUE) / liquid_mm).collect();
    let walks: [f64; FS] = std::array::from_fn(|x| bodies[x].speed * searching[x] / DAY);
    let thirsts: [Thirst; FS] = std::array::from_fn(|x| Thirst::of(&bodies[x], walks[x], (dt, lv), full[x].iter().zip(&wet).map(|(k, l)| k * l).sum()));
    let mut given = thirst::shares(&thirsts, found.open + found.running + found.soft);
    let mut work_of_way = [0.0; FS];
    for x in (0..FS).filter(|&x| bodies[x].n > 0.0) {
        let (b, short) = (&bodies[x], (thirsts[x].asks() - given[x] - thirsts[x].eaten).max(0.0));
        let got = way::fetch((b.n, b.m, b.speed, b.per_m), (short, liquid_mm, days), contract::to_liquid(w, i, short), w.cfg.phys.dry_most);
        (given[x], work_of_way[x]) = (given[x] + got.mol, got.work);
    }
    let (work, lack) = thirst::work(bodies, &thirsts, &walks, &given, (dt, lv));
    Plan { work, lack, way: work_of_way }
}

/// Slot `x` of `f`, its members `b`, digests `took` (mol of each substance) for the energy it burns, `burnt` W a member
/// (`gut::digest`, its gut splitting walls with the cell's `liquid`), and loses the members it could not keep, drawn
/// from `r`: unpaid, worn out, frozen, and `lack` of them for the heat they could not shed even at rest; an empty slot's
/// amounts go to the remains.
fn settle(w: &World, (p, gut): (&Parts, &Gut), (f, x, b): (&mut Fauna, usize, &Body), (took, burnt, lack): (&[f64; crate::units::MAX_SUBSTANCES], f64, f64), (liquid, mut r, books): (&mut f64, Rng, &mut Books<0>)) {
    let (chem, ph, dt) = (&w.chem, &w.cfg.phys, w.cfg.calendar().month as f64);
    let (sub, mm) = (sub(p), mm(chem, p));
    let part: [f64; NP] = std::array::from_fn(|q| took[p.sub[q]]);
    let store = ph.store_most * b.m * b.n / mm[super::STORE];
    let spent = digest(chem, (p, gut), (f, x), (part, b.gut * b.n * dt, liquid), (burnt * b.n * dt, store), books);
    let frozen = ph.frozen((chem.subs[p.liquid].melt, ph.harden), b.tb, b.pace.min(1.0));
    let worn = one_minus_exp(b.wear(ph, f.genes[x][REPAIR], burnt, dt));
    die(f, (x, &sub), [spent, worn, frozen, lack], &mut r, books);
    if f.adults[x] <= 0.0 {
        (0..NF).for_each(|q| books.dead[sub[q]] += f.mol[x][q]);
        (f.adults[x], f.mol[x], f.root[x]) = (0.0, [0.0; NF], NO_ROOT);
    }
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
