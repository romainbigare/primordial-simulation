//! Taking and giving: from the cell's ground (`take_node`), to it (`put`), from another body (`take_body`), and to
//! another body (`give`), each by the intake law of bodies (`Body::wanted`).

use crate::body::{self, Body, STRENGTH, TAKEN};
use crate::chem::{Chemistry, Phase, Stores};
use crate::event::{Event, EventKind};
use crate::rng::Rng;
use crate::units::{BODY_STORES, MAX_SUBSTANCES};
use crate::{bond, lanes, life::{Chunk, Handle}, store::Node, World};

/// Mass of the amounts `m` (mol of each store) that the body at `t` K uses (`Body::uses`), kg.
fn used_kg(w: &World, b: &Body, t: f64, m: &Stores) -> f64 {
    let bc = w.chem.body();
    bc.subs().iter().zip(m).enumerate().filter(|&(_, (&k, _))| b.uses(&w.chem, k, t)).map(|(s, (_, a))| a * bc.mm[s]).sum()
}

/// The intake law of bodies, one for the ground and for other bodies: what body `b` can take over `days` d (the time to
/// its next update, its cell's step, which what it takes feeds) from what lies over its cell, `from` mol of each substance, per m² of its
/// cell. Per day per kg-equivalent of its supply it takes Holling's type II
/// response to what the area it works (`sweep`) holds of what it takes, s, up to `take`: take s / (take + s), so the
/// thinner it lies, the more of its time goes to finding it (Holling 1959). Only what it uses counts in s
/// (`Body::uses`). A node's members each work their own area. Returns that, kg, and the share of the cell its area
/// holds, which is the share of another body's members it holds and may be less than one member: of one individual
/// body, the chance it holds it. `t`: the body's temperature, K.
fn intake(w: &World, b: &Body, (t, days): (f64, f64), from: impl Fn(usize) -> f64) -> (f64, f64) {
    let (p, area) = (&w.cfg.phys, w.mesh.cells().area()[b.cell as usize]);
    let wants = b.k.upt.iter().filter(|&&(k, _)| b.uses(&w.chem, k, t));
    let lying: f64 = wants.map(|&(k, _)| from(k).max(0.0) * w.chem.subs[k].molar_mass).sum();
    let (found, supply) = (p.sweep * lying / area, b.supply(&w.chem, p, t));
    let held = (p.sweep * supply * days / area).min(1.0);
    (p.take * found / (p.take + found) * days * supply, held)
}

/// The id a take from the cover of a cell reads in its event (`EventKind::Took`; the ground's is `u64::MAX`).
pub const COVER: u64 = u64::MAX - 1;

/// Take loose solids and liquids from the cell's ground and from its cover (grazing) by the intake law (`intake`, one
/// search over both), no more than its stores have room for, each substance from the two in proportion to what each
/// holds of it. A bite of cover takes all its parts in the field's proportions: what the body does not use (`Body::uses`:
/// the frame without the enzyme that splits it) is bulk, which counts toward what it can take (`intake`, the gut) and
/// passes through it to the ground at once, with its heat (a gut passage of days, within the update's step: it never
/// becomes the body's own frame). What it takes of the cover it pays to break as a bite does (`Cover::breaking`), less
/// what a held edge saves (`bond::Edge`), which wears by the work it does. `days`: the time to its next update.
pub(super) fn take_node(w: &World, d: &mut Chunk, b: &mut Body, days: f64) {
    let (i, chem, bc) = (b.cell as usize, &w.chem, w.chem.body());
    let (tg, parts) = (w.t_ground_last(i), crate::cover::Parts::of(chem));
    let cover: [f64; MAX_SUBSTANCES] = parts.as_ref().map_or([0.0; MAX_SUBSTANCES], |p| d.at(i).cover.amounts(p));
    // (The cover's parts are among the stores of bodies: its fuel, matter and built solids.)
    let (of_cover, t) = (|s: usize| bc.subs().get(s).map_or(0.0, |&k| cover[k]), b.temp(chem));
    // What it would take and of it what it would graze, by what lies on the ground (one look-up of the ground's view)
    // and in the cover.
    let (gut, mut m, mut grazed) = {
        let ground = d.moves.amounts(w, Node::ground(i));
        let lying = |k: usize| (if chem.phase(k, tg) != Phase::Gas { ground(k).max(0.0) } else { 0.0 }) + cover[k];
        let gut = intake(w, b, (t, days), lying).0;
        let m = b.wanted(w, lying, gut, b.mass(chem));
        let grazed: Stores = std::array::from_fn(|s| if of_cover(s) > 0.0 { (m[s] * of_cover(s) / lying(bc.subs()[s])).min(m[s]).min(of_cover(s)) } else { 0.0 });
        (gut, m, grazed)
    };
    // The bulk: the parts of the cover it does not use, at the share of the cover its bite took of those it uses.
    let used: [bool; BODY_STORES] = std::array::from_fn(|s| of_cover(s) > 0.0 && b.uses(chem, bc.subs()[s], t));
    let usable: f64 = (0..bc.n()).filter(|&s| used[s]).map(|s| of_cover(s) * bc.mm[s]).sum();
    let bit = if usable > 0.0 { bc.mass_of(&grazed) / usable } else { 0.0 };
    let mut bulk: Stores = std::array::from_fn(|s| if of_cover(s) > 0.0 && !used[s] { of_cover(s) * bit } else { 0.0 });
    // All it takes fits its gut.
    let full = bc.mass_of(&m) + bc.mass_of(&bulk);
    if full > gut {
        let f = gut / full;
        [&mut m, &mut grazed, &mut bulk].into_iter().for_each(|v| v.iter_mut().for_each(|a| *a *= f));
    }
    // (The cover has not changed since `cover` was read.)
    let (all, kg) = (parts.as_ref().map_or(0.0, |_| cover.iter().zip(&chem.subs).map(|(a, s)| a * s.molar_mass).sum()), bc.mass_of(&grazed) + bc.mass_of(&bulk));
    let breaking = parts.as_ref().filter(|_| kg > 0.0).map_or(0.0, |p| d.at(i).cover.breaking(chem, &w.cfg, p, (kg / all.max(1e-300)).min(1.0)));
    let edge = bond::edge(d.at(i), w, b).filter(|_| breaking > 0.0);
    let area = w.mesh.cells().area()[i];
    let (left, through) = match (&edge, &parts) {
        (Some(e), Some(p)) => bond::edge_leaves(b.n(), d.at(i).cover.cut_by(chem, &w.cfg, p, e, area)),
        _ => (1.0, 0.0),
    };
    if !body::pay(w, d, b, w.cfg.phys.c_handle * (bc.mass_of(&m) + bc.mass_of(&bulk)) + left * breaking) {
        return;
    }
    if let Some(e) = edge.filter(|_| through > 0.0) {
        bond::blunt(d, &e, through * breaking);
        d.counts.parts.edged += kg;
    }
    for (from, x) in [(u64::MAX, bc.mass_of(&m) - bc.mass_of(&grazed)), (COVER, kg)].into_iter().filter(|e| e.1 > 0.0) {
        let part: Stores = std::array::from_fn(|s| if from == COVER { grazed[s] } else { m[s] - grazed[s] });
        let ev = EventKind::Took { by: b.id, from, used: used_kg(w, b, t, &part) as f32 };
        d.events.push(Event { day: w.day(), kind: ev, node: b.cell, value: x });
    }
    if let Some(p) = parts.as_ref().filter(|_| kg > 0.0) {
        for (s, g) in grazed.iter().zip(&bulk).map(|(g, x)| g + x).enumerate().filter(|e| e.1 > 0.0) {
            b.e += d.cover(i).take(chem, p, bc.subs()[s], g);
            b.store[s] += g;
        }
        d.counts.cover_lost[TAKEN as usize] += kg;
        if bulk.iter().any(|&x| x > 0.0) {
            body::to_ground(w, d, b, &bulk);
        }
    }
    for (s, &k) in bc.subs().iter().enumerate() {
        let held = if chem.phase(k, tg) != Phase::Gas { d.moves.amount(w, Node::ground(i), k).max(0.0) } else { 0.0 };
        let want = (m[s] - grazed[s]).min(held);
        if want <= 0.0 {
            m[s] = grazed[s];
            continue;
        }
        // The ground pays the heat that goes with what is taken; if it cannot, less is taken.
        let (got, took) = d.moves.take_loose(w, i, (k, want), tg);
        (b.store[s], b.e, m[s]) = (b.store[s] + got, b.e + took, got + grazed[s]);
    }
    b.digest(chem, Body::matter_at(chem).map_or(0.0, |s| m[s]), &mut d.counts.rounded[0]);
}

/// Release stores above the level the body keeps (`Body::level`) to the cell's ground.
pub(super) fn put(w: &World, d: &mut Chunk, b: &mut Body) {
    let (mass, bc) = (b.mass(&w.chem), w.chem.body());
    let m: Stores = std::array::from_fn(|s| if s < bc.n() { (b.store[s] - b.level(&w.chem, s, mass)).clamp(0.0, b.store[s]) } else { 0.0 });
    if !body::pay(w, d, b, w.cfg.phys.c_handle * bc.mass_of(&m)) {
        return;
    }
    let m: Stores = std::array::from_fn(|s| m[s].min(b.store[s]));
    body::to_ground(w, d, b, &m);
}

/// Move amounts `m` (mol of each store) from `from` to `to`, with their heat and their share of `from`'s brood; `to`
/// digests the body matter among them (`Body::digest`, its rounding counted in `rounded`).
fn transfer(chem: &Chemistry, rounded: &mut f64, from: &mut Body, to: &mut Body, m: &Stores) {
    let q = from.heat_with(chem, m);
    from.lose(chem, m);
    for (s, &a) in m.iter().enumerate() {
        to.store[s] += a;
    }
    from.e -= q;
    to.e += q;
    to.digest(chem, Body::matter_at(chem).map_or(0.0, |s| m[s]), rounded);
}

/// Take from another body in the cell by uptake selectivity, by the intake law (`intake`: its members spread over the
/// cell, a node is found as the ground is), as far as its stores' room and its force against the other's allow. It
/// reaches what the members its area holds hold (an individual, by any taker: all of it, with the chance its area
/// holds it). The bite tears each member it reaches (`body::wound`; laws.md 6.8). A bite that reaches
/// less than one member of a node meets one whole member with that chance: it takes, tears and pays what that meeting
/// would, times the chance, the expected outcome of an individual's draw. A held edge (`bond::Edge`) leaves the share
/// `left` of the force and the energy the bite needs to break the other, and wears by the work done through it. `days`:
/// the time to its next update.
pub fn take_body(w: &World, d: &mut Chunk, b: &mut Body, (s, days): (Handle, f64), rng: &mut Rng) {
    let (o, bc) = (&d[s], w.chem.body());
    let (mb, tb) = (b.mass(&w.chem), b.temp(&w.chem));
    let edge = bond::edge(d.at(b.cell as usize), w, b);
    let (left, through) = edge.as_ref().map_or((1.0, 0.0), |e| bond::edge_leaves(b.n(), o.cut_by(&w.chem, &w.cfg, e)));
    // (Its force goes as far as `1 / left` of it would bluntly.)
    let (fb, fo) = (b.gene(STRENGTH) * b.built(&w.chem) / b.n() / left, o.gene(STRENGTH) * o.built(&w.chem) / o.n());
    let (kg, swept) = intake(w, b, (tb, days), |k| o.amount(bc, k));
    // One draw finds an individual prey.
    let reached = if o.n() > 1.0 { swept } else if rng.f64() < swept { 1.0 } else { return };
    // The share of the members a meeting reaches (at least one whole member) and the chance of that meeting.
    let per = reached.max(1.0 / o.n());
    let chance = reached / per;
    let m = b.wanted(w, |k| per * o.amount(bc, k), kg * fb / (fb + fo + 1e-30), mb);
    let resist = 1.0 + lanes::thickness(w, w.life.sum(b.cell as usize)[1], w.mesh.cells().area()[b.cell as usize]);
    let hit = reached;
    // The bite tears this share of the body of each member it reaches; the stores they held spill in proportion, less
    // what the taker took (all they held, if they die).
    let (built, got) = (per * o.built(&w.chem), bc.mass_of(&m));
    let eaten = Body::matter_at(&w.chem).map_or(0.0, |k| if o.store[k] > 0.0 { m[k] / (per * o.store[k]) } else { 0.0 });
    let f = (got / built.max(1e-30)).max(eaten).min(1.0);
    // The taker pays to handle what it takes and to break the share it tears (`Body::breaking`).
    let breaking = o.breaking(&w.chem, &w.cfg, f * per);
    let cost = (w.cfg.phys.c_handle * got * resist + left * breaking) * chance;
    if !body::pay(w, d, b, cost) || hit <= 0.0 {
        return;
    }
    if let Some(e) = edge.filter(|_| through > 0.0) {
        bond::blunt(d, &e, through * breaking * chance);
        d.counts.parts.edged += got * hit / per;
    }
    let mut o = std::mem::take(&mut d[s]);
    let m = m.map(|a| a * hit / per);
    let torn = if body::torn_to_death(&w.cfg.phys, f) { hit } else { f * hit };
    let mut rest: Stores = std::array::from_fn(|q| (torn * o.store[q] - m[q]).max(0.0));
    let got = bc.mass_of(&m);
    transfer(&w.chem, &mut d.counts.rounded[0], &mut o, b, &m);
    let ev = EventKind::Took { by: b.id, from: o.id, used: used_kg(w, b, tb, &m) as f32 };
    d.events.push(Event { day: w.day(), kind: ev, node: b.cell, value: got });
    body::wound(w, d, s, o, (hit, f), &mut rest, TAKEN);
}

/// Give to another body in the cell what it can spare (`Body::spare`, laws.md 5.3), up to what the members it meets could
/// take in the time to its next update, `days`, and have room for (each of its members meets one of the other's, as
/// many as both have).
pub(super) fn give(w: &World, d: &mut Chunk, b: &mut Body, (s, days): (Handle, f64)) {
    let (chem, bc) = (&w.chem, w.chem.body());
    let (size, mass) = (b.size(chem), b.mass(chem));
    let spare = |q: usize| b.spare(chem, &w.cfg.phys, q, (size, mass));
    let all: f64 = (0..bc.n()).map(|q| spare(q) * bc.mm[q]).sum();
    let o = &d[s];
    let (mo, met, t) = (o.mass(chem), b.n().min(o.n()), o.temp(chem));
    let ahead = body::ahead(w, o, mo);
    let share = (w.cfg.phys.take * days * o.supply(&w.chem, &w.cfg.phys, t) * met / o.n() / all.max(1e-9)).min(met / b.n());
    let mut m: Stores = std::array::from_fn(|q| if q < bc.n() && o.uses(chem, bc.subs()[q], t) { (spare(q) * share).min(o.room(chem, q, mo, ahead) * met / o.n()) } else { 0.0 });
    o.digest_room(chem, &mut m, mo, ahead);
    let got = bc.mass_of(&m);
    if got <= 0.0 || !body::pay(w, d, b, w.cfg.phys.c_handle * got) {
        return;
    }
    // Paying may have used some of what was to be put.
    let m: Stores = std::array::from_fn(|q| m[q].min(b.store[q]));
    let mut o = std::mem::take(&mut d[s]);
    transfer(&w.chem, &mut d.counts.rounded[0], b, &mut o, &m);
    o.stale = true;
    d.events.push(Event { day: w.day(), kind: EventKind::Gave { from: b.id, to: o.id }, node: b.cell, value: got });
    d[s] = o;
}
