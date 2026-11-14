//! A month of the takers of each cell of a chunk (laws.md 3b): the slots find what they reach of the cover (`take`),
//! cut their work to the liquid they have (`thirst`), take one bite of each layer for all, shed the heat their coat
//! cannot pass as vapour, digest what they took and pay their upkeep (`gut`), and lose the members they could not keep:
//! unpaid, worn out, frozen, and those whose heat stays unshed even at rest (life's causes `SPENT`, `AGE`, `PHASE`,
//! `LACK`); then each slot grows and bears young (`grow`). Their remains go to the ground; everything they take and give is booked through the contract and the books
//! of the cell's life (`life::book`), with its heat.

use super::body::{body, Body};
use super::grow::{grow, whole, Surplus};
use super::lodge::{lodge, merge_alike, Packet};
use super::gut::{digest, Digested, Gut};
use super::thirst::{self, Thirst};
use super::{mm, sub, take, way, Fauna, FS, GROUPS, KEEP, NF, N_WHOLE, REPAIR};
use crate::contract;
use crate::life::{book, Books, Chunk, Parts, AGE, LACK, MATTER, NO_ROOT, NP, PHASE, SPENT, WET_TISSUE};
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
/// of each layer for all, their vapour, then each slot's groups' digestion and deaths (`settle`), its growth and births
/// (`grow`), and the books.
fn cell(w: &World, d: &mut Chunk, (p, gut): (&Parts, &Gut), i: usize) {
    let (chem, dt, t) = (&w.chem, w.cfg.calendar().month as f64, contract::place(w, i).t);
    let (area, lv, days) = (w.mesh.cells().area()[i], chem.subs[p.liquid].lv, dt / DAY);
    let mm = mm(chem, p);
    contract::open(w, d, i);
    // What lies on its ground, kg a m², and each member's share of the cell's: each sits in its own heap.
    let (remains, members) = (contract::remains(w, d, i), d.at(i).fauna.n.iter().sum::<f64>());
    let share = remains * area / members.max(1.0);
    let bodies: [Body; GROUPS] = {
        let f = &d.at(i).fauna;
        std::array::from_fn(|g| body(w, p, f, (g, &mm), (i, t), (gut.per_wall(chem), remains, share)))
    };
    let layers = take::layers(w, d, i);
    let reach = take::reachable(w, i, &layers, &bodies);
    let found = contract::liquid(w, d, i);
    let plan = plan(w, p, (i, &bodies), (&layers, &reach), &found);
    let (taken, searching) = take::scheme(&take::eaters(&bodies, &reach.share, &plan.work), (&reach.stock, &reach.breaking), area, days);
    let walks: [f64; GROUPS] = std::array::from_fn(|g| bodies[g].speed * searching[g] / DAY);
    let (took, heat) = take::bite(w, d, i, &layers, &taken);
    // The vapour they shed leaves the cell's liquid (the open liquid, what ran over it, what they ate and fetched join it
    // there), then the soft solid; the liquid their guts split walls with leaves it too.
    let shed: f64 = (0..GROUPS).map(|g| bodies[g].vapour(walks[g], plan.work[g], lv) * bodies[g].n * dt * (1.0 - plan.lack[g])).sum();
    let ground = found.open + found.pores;
    let drunk = shed.min(ground);
    let paid = contract::vapour(w, d, i, [drunk, (shed - drunk).min(found.soft)]);
    let mut b: Books<0> = Books::new(contract::air(w, d, i), []);
    let (f, moves, counts) = d.fauna_with(i);
    let e = f.heat() + heat.iter().sum::<f64>() - paid;
    let mut liquid = ground - drunk;
    let (held, year): (Vec<usize>, f32) = (f.held().collect(), w.cfg.months_per_year as f32);
    let mut packets = Vec::new();
    for x in held {
        // One stream of draws per slot and month, drawn in a fixed order: deaths of its adults, of its young, its
        // maturing, its births, the mutation of its newborns.
        let mut r = Rng::new(w.seed, w.month(), (i * FS + x) as u64, Purpose::Fauna);
        let (mut surplus, mut balance) = (Surplus { gained: [0.0; 2], matter: 0.0 }, 0.0);
        for (c, g) in [x, FS + x].into_iter().enumerate().filter(|&(_, g)| bodies[g].n > 0.0) {
            let burnt = bodies[g].heat(walks[g], plan.work[g]) + plan.way[g] / (bodies[g].n * dt);
            let (died, kept) = settle(w, (p, gut), (f, g, &bodies[g]), (&took[g], burnt, plan.lack[g]), (&mut liquid, &mut r, &mut b));
            counts.takers_died.iter_mut().zip(died).for_each(|(a, d)| *a += d);
            (surplus.gained[c], surplus.matter, balance) = (kept.gained, surplus.matter + kept.matter, balance + kept.balance);
        }
        let (n, body) = grow(w, (p, gut), (f, x, (&bodies[x], &bodies[FS + x])), &surplus, (&mut r, &mut b));
        tidy(f, x, &sub(p), &mut b);
        [x, FS + x].into_iter().for_each(|g| rounded(f, g, &mut counts.rounded[0]));
        if f.root[x] != NO_ROOT {
            f.net[x] += ((balance / f.living(&mm, x).max(1e-300)) as f32 - f.net[x]) / year;
        }
        if n > 0.0 {
            packets.push((Packet { x, n, body }, r));
        }
    }
    // The newborns lodge in the order of their slots, then slots alike in every gene join.
    let waiting: Vec<usize> = packets.iter().map(|q| q.0.x).collect();
    for (k, (packet, mut r)) in packets.into_iter().enumerate() {
        lodge(w, (f, &packet), (&sub(p), &mm), (&mut r, &mut b, &mut counts.takers_died[SPENT as usize]), &waiting[k + 1..]);
    }
    merge_alike(w, f, &mm);
    // Each slot's heat ends at its members' temperature; what the burning released beyond it goes to the ground.
    let target: [f64; FS] = std::array::from_fn(|x| f.cap(chem, &sub(p), x) * (f.genes[x][KEEP] as f64).max(t));
    let all: f64 = target.iter().sum();
    let left = book(w, moves, (i, t), (e, 0.0), all, &b);
    let share = if all > 0.0 { left / all } else { 0.0 };
    f.e = target.map(|x| x * share);
}

/// What the slots of cell `i` do for liquid in the month: the share of their work each keeps, the share of its members
/// that die of thirst, and the work of their ways to the liquid, J.
struct Plan {
    work: [f64; GROUPS],
    lack: [f64; GROUPS],
    way: [f64; GROUPS],
}

/// The plan of the slots `bodies` of cell `i` for liquid: what they would take of `layers` at full work and the
/// liquid of it (`WET_TISSUE` of the living mass of the body matter), their need at that work (`Thirst`), their shares of
/// the open liquid `found`, what they fetch by the way to the nearest open liquid of the rest (`way::fetch`), and the
/// work each keeps (`thirst::work`).
fn plan(w: &World, p: &Parts, (i, bodies): (usize, &[Body; GROUPS]), (layers, reach): (&[take::Found], &take::Reach), found: &contract::Liquid) -> Plan {
    let (chem, dt) = (&w.chem, w.cfg.calendar().month as f64);
    let (lv, liquid_mm, days) = (chem.subs[p.liquid].lv, chem.subs[p.liquid].molar_mass, dt / DAY);
    let (full, searching) = take::scheme(&take::eaters(bodies, &reach.share, &[1.0; GROUPS]), (&reach.stock, &reach.breaking), w.mesh.cells().area()[i], days);
    let matter = chem.subs[p.sub[MATTER]].molar_mass;
    let wet: Vec<f64> = layers.iter().map(|f| f.shown.mol[p.sub[MATTER]] * matter / f.shown.kg * WET_TISSUE / (1.0 - WET_TISSUE) / liquid_mm).collect();
    let walks: [f64; GROUPS] = std::array::from_fn(|x| bodies[x].speed * searching[x] / DAY);
    let thirsts: [Thirst; GROUPS] = std::array::from_fn(|x| Thirst::of(&bodies[x], walks[x], (dt, lv), full[x].iter().zip(&wet).map(|(k, l)| k * l).sum()));
    let mut given = thirst::shares(&thirsts, found.open + found.running + found.soft);
    let mut work_of_way = [0.0; GROUPS];
    for x in (0..GROUPS).filter(|&x| bodies[x].n > 0.0) {
        let (b, short) = (&bodies[x], (thirsts[x].asks() - given[x] - thirsts[x].eaten).max(0.0));
        let got = way::fetch((b.n, b.m, b.speed, b.per_m), (short, liquid_mm, days), contract::to_liquid(w, i, short), w.cfg.phys.dry_most);
        (given[x], work_of_way[x]) = (given[x] + got.mol, got.work);
    }
    let (work, lack) = thirst::work(bodies, &thirsts, &walks, &given, (dt, lv));
    Plan { work, lack, way: work_of_way }
}

/// Group `g` of `f`, its members `b`, digests `took` (mol of each substance) for the energy it burns, `burnt` W a member
/// (`gut::digest`, its gut splitting walls with the cell's `liquid`), and loses the members it could not keep, drawn
/// from `r`: unpaid, worn out, frozen, and `lack` of them for the heat they could not shed even at rest. Returns the
/// members that died by cause (`life::LACK` to `AGE`) and what it kept for growth.
fn settle(w: &World, (p, gut): (&Parts, &Gut), (f, g, b): (&mut Fauna, usize, &Body), (took, burnt, lack): (&[f64; crate::units::MAX_SUBSTANCES], f64, f64), (liquid, r, books): (&mut f64, &mut Rng, &mut Books<0>)) -> ([f64; 6], Digested) {
    let (chem, ph, dt) = (&w.chem, &w.cfg.phys, w.cfg.calendar().month as f64);
    let sub = sub(p);
    let part: [f64; NP] = std::array::from_fn(|q| took[p.sub[q]]);
    let kept = digest(chem, (p, gut), (f, g), (part, b.gut * b.n * dt, liquid), burnt * b.n * dt, books);
    let frozen = ph.frozen((chem.subs[p.liquid].melt, ph.harden), b.tb, b.pace.min(1.0));
    let worn = one_minus_exp(b.wear(ph, f.genes[g % FS][REPAIR], burnt, dt));
    let causes = [(kept.unpaid, SPENT), (worn, AGE), (frozen, PHASE), (lack, LACK)];
    let dead = die(f, (g, &sub), causes.map(|c| c.0), r, books);
    // The dead shared by the causes as they combine, each on what the one before left.
    let (mut left, mut by) = (1.0, [0.0; 6]);
    let all = 1.0 - causes.iter().map(|c| 1.0 - c.0.clamp(0.0, 1.0)).product::<f64>();
    for (share, cause) in causes {
        by[cause as usize] = if all > 0.0 { dead * share.clamp(0.0, 1.0) * left / all } else { 0.0 };
        left *= 1.0 - share.clamp(0.0, 1.0);
    }
    (by, kept)
}

/// The groups of slot `x` of `f` (its parts the substances `sub`) after its month: a group of no members gives its
/// amounts to the slot's other group (matter built into newborns none of whom was drawn stays in their mother); a slot
/// of no members is empty, and all it held goes to the remains (`Books::dead`).
fn tidy(f: &mut Fauna, x: usize, sub: &[usize; NF], b: &mut Books<0>) {
    for (from, to) in [(FS + x, x), (x, FS + x)] {
        if f.n[from] <= 0.0 && f.n[to] > 0.0 {
            for q in 0..NF {
                (f.mol[to][q], f.mol[from][q]) = (f.mol[to][q] + f.mol[from][q], 0.0);
            }
        }
    }
    if f.members(x) <= 0.0 {
        for g in [x, FS + x] {
            (0..NF).for_each(|q| b.dead[sub[q]] += f.mol[g][q]);
            (f.n[g], f.mol[g]) = (0.0, [0.0; NF]);
        }
        f.root[x] = NO_ROOT;
    }
}

/// What rounding left below zero of group `x` of `f` is set to zero and counted in `rounded`, mol.
fn rounded(f: &mut Fauna, x: usize, rounded: &mut f64) {
    for m in f.mol[x].iter_mut().filter(|m| **m < 0.0) {
        *rounded -= std::mem::take(m);
    }
}

/// The members of group `x` of `f` (its parts the substances `sub`) die by the shares `causes`, each on what the one
/// before left (as the cover's causes combine); their share of its body matter and store goes to the cell's remains
/// (`Books::dead`). Below `N_WHOLE` members the count is whole: the dead are a binomial draw from `r`, and a count that
/// falls below it becomes whole by a draw that keeps its mean (it is never rounded up as a rule). Returns the dead.
fn die(f: &mut Fauna, (x, sub): (usize, &[usize; NF]), causes: [f64; 4], r: &mut Rng, b: &mut Books<0>) -> f64 {
    let n = f.n[x];
    let left: f64 = causes.iter().map(|c| 1.0 - c.clamp(0.0, 1.0)).product();
    let dead = if n < N_WHOLE { r.binomial(n as u32, 1.0 - left) as f64 } else { n * (1.0 - left) };
    let after = whole(n - dead, r);
    let gone = if n > 0.0 { dead / n } else { 0.0 };
    for (a, &k) in f.mol[x].iter_mut().zip(sub) {
        let m = *a * gone;
        *a -= m;
        b.dead[k] += m;
    }
    f.n[x] = after;
    dead
}
