//! Taking and giving: from the cell's ground (`take_node`), to it (`put`), from another body (`take_body`), and to
//! another body (`give`), each by the intake law of bodies (`Body::wanted`).

use crate::body::{self, Body, STRENGTH, TAKEN};
use crate::chem::{Chemistry, Phase};
use crate::event::{Event, EventKind};
use crate::rng::Rng;
use crate::{bond, lanes, life::{Chunk, Handle}, store::Node, World};

/// A dealing counts in a body's memory (`Body::note`) as this many times the share of its mass moved.
const NOTE_SCALE: f64 = 10.0;

/// Mass of the amounts `m` that the body at `t` K uses (`Body::uses`), kg.
fn used_kg(w: &World, b: &Body, t: f64, m: &[f64]) -> f64 {
    m.iter().enumerate().filter(|&(k, _)| b.uses(&w.chem, k, t)).map(|(k, a)| a * w.chem.subs[k].molar_mass).sum()
}

/// The intake law of bodies, one for the ground and for other bodies: what body `b` can take in its update from what
/// lies over its cell, `from` mol of each substance. Per day per kg-equivalent of its supply it takes Holling's type II
/// response to what the area it works (`sweep`) holds of what it takes, s, up to `take`: take s / (take + s), so the
/// thinner it lies, the more of its time goes to finding it (Holling 1959). Only what it uses counts in s
/// (`Body::uses`). A node's members each work their own area. Returns that, kg, and the share of the cell its area
/// holds, which is the share of another body's members it holds and may be less than one member: of one individual
/// body, the chance it holds it. `t`: the body's temperature, K.
fn intake(w: &World, b: &Body, t: f64, from: impl Fn(usize) -> f64) -> (f64, f64) {
    let (p, area, days) = (&w.cfg.phys, w.mesh.cells().area()[b.cell as usize], w.cfg.update_days());
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
/// passes through it to the ground at once, with its heat (a gut passage of days, within the update's month: it never
/// becomes the body's own frame). What it takes of the cover it pays to break as a bite does (`Cover::breaking`).
pub(super) fn take_node(w: &World, d: &mut Chunk, b: &mut Body) {
    let (i, chem) = (b.cell as usize, &w.chem);
    let (tg, parts) = (w.t_ground_last(i), crate::cover::Parts::of(chem));
    let free = |k: usize| if chem.phase(k, tg) != Phase::Gas { d.moves.amount(w, Node::ground(i), k).max(0.0) } else { 0.0 };
    let cover: [f64; crate::units::MAX_SUBSTANCES] = parts.as_ref().map_or([0.0; crate::units::MAX_SUBSTANCES], |p| d.at(i).cover.amounts(p));
    let (lying, t) = (|k: usize| free(k) + cover[k], b.temp(chem));
    let gut = intake(w, b, t, lying).0;
    let mut m = b.wanted(chem, lying, gut, b.mass(chem));
    let mut grazed: Vec<f64> = m.iter().enumerate().map(|(k, &a)| if cover[k] > 0.0 { (a * cover[k] / lying(k)).min(a).min(cover[k]) } else { 0.0 }).collect();
    // The bulk: the parts of the cover it does not use, at the share of the cover its bite took of those it uses.
    let used = |k: usize| cover[k] > 0.0 && b.uses(chem, k, t);
    let usable: f64 = (0..chem.ns()).filter(|&k| used(k)).map(|k| cover[k] * chem.subs[k].molar_mass).sum();
    let bit = if usable > 0.0 { chem.mass_of(&grazed) / usable } else { 0.0 };
    let mut bulk: Vec<f64> = (0..chem.ns()).map(|k| if cover[k] > 0.0 && !used(k) { cover[k] * bit } else { 0.0 }).collect();
    // All it takes fits its gut.
    let full = chem.mass_of(&m) + chem.mass_of(&bulk);
    if full > gut {
        let f = gut / full;
        [&mut m, &mut grazed, &mut bulk].into_iter().for_each(|v| v.iter_mut().for_each(|a| *a *= f));
    }
    let (all, kg) = (parts.as_ref().map_or(0.0, |p| d.at(i).cover.amounts(p).iter().zip(&chem.subs).map(|(a, s)| a * s.molar_mass).sum()), chem.mass_of(&grazed) + chem.mass_of(&bulk));
    let breaking = parts.as_ref().filter(|_| kg > 0.0).map_or(0.0, |p| d.at(i).cover.breaking(chem, &w.cfg, p, (kg / all.max(1e-300)).min(1.0)));
    if !body::pay(w, d, b, w.cfg.phys.c_handle * (chem.mass_of(&m) + chem.mass_of(&bulk)) + breaking) {
        return;
    }
    for (from, x) in [(u64::MAX, chem.mass_of(&m) - chem.mass_of(&grazed)), (COVER, kg)].into_iter().filter(|e| e.1 > 0.0) {
        let part: Vec<f64> = m.iter().zip(&grazed).map(|(&a, &g)| if from == COVER { g } else { a - g }).collect();
        let ev = EventKind::Took { by: b.id, from, used: used_kg(w, b, t, &part) as f32 };
        d.events.push(Event { day: w.day(), kind: ev, node: b.cell, value: x });
    }
    if let Some(p) = parts.as_ref().filter(|_| kg > 0.0) {
        for (k, g) in grazed.iter().zip(&bulk).map(|(g, x)| g + x).enumerate().filter(|e| e.1 > 0.0) {
            b.e += d.cover(i).take(chem, p, k, g);
            b.store[k] += g;
        }
        d.counts.cover_lost[TAKEN as usize] += kg;
        if bulk.iter().any(|&x| x > 0.0) {
            let th = b.thermal(chem);
            body::to_ground(w, d, b, &th, &bulk);
        }
    }
    for k in 0..m.len() {
        let held = if chem.phase(k, tg) != Phase::Gas { d.moves.amount(w, Node::ground(i), k).max(0.0) } else { 0.0 };
        let want = (m[k] - grazed[k]).min(held);
        if want <= 0.0 {
            m[k] = grazed[k];
            continue;
        }
        // The ground pays the heat that goes with what is taken; if it cannot, less is taken.
        let cp = chem.subs[k].cp;
        let heat = want * (cp * tg + chem.fused_heat(k, tg, |j| d.moves.amount(w, Node::ground(i), j)));
        let (got, took) = d.moves.take_from_ground(w, i, (k, want, cp), heat, d.moves.heat_cap_ground(w, i));
        (b.store[k], b.e, m[k]) = (b.store[k] + got, b.e + took, got + grazed[k]);
    }
    b.digest(chem, chem.matter().map_or(0.0, |k| m[k]), &mut d.counts.rounded[0]);
}

/// Release stores above the level the body keeps (`Body::level`) to the cell's ground.
pub(super) fn put(w: &World, d: &mut Chunk, b: &mut Body) {
    let mass = b.mass(&w.chem);
    let m: Vec<f64> = (0..w.chem.ns()).map(|k| (b.store[k] - b.level(&w.chem, k, mass)).clamp(0.0, b.store[k])).collect();
    if !body::pay(w, d, b, w.cfg.phys.c_handle * w.chem.mass_of(&m)) {
        return;
    }
    let m: Vec<f64> = m.iter().zip(&b.store).map(|(&a, &held)| a.min(held)).collect();
    let th = b.thermal(&w.chem);
    body::to_ground(w, d, b, &th, &m);
}

/// Move amounts `m` (mol) from `from` to `to`, with their heat and their share of `from`'s brood; `to` digests the body
/// matter among them (`Body::digest`, its rounding counted in `rounded`).
fn transfer(chem: &Chemistry, rounded: &mut f64, from: &mut Body, to: &mut Body, m: &[f64]) {
    let q = from.heat_with(chem, &from.thermal(chem), m);
    from.lose(chem, m);
    for (k, &a) in m.iter().enumerate() {
        to.store[k] += a;
    }
    from.e -= q;
    to.e += q;
    to.digest(chem, chem.matter().map_or(0.0, |k| m[k]), rounded);
}

/// Take from another body in the cell by uptake selectivity, by the intake law (`intake`: its members spread over the
/// cell, a node is found as the ground is), as far as its stores' room and its force against the other's allow. It
/// reaches what the members its area holds hold (an individual, by any taker: all of it, with the chance its area
/// holds it). The bite tears each member it reaches (`body::wound`; laws.md 6.8). A bite that reaches
/// less than one member of a node meets one whole member with that chance: it takes, tears and pays what that meeting
/// would, times the chance, the expected outcome of an individual's draw.
pub fn take_body(w: &World, d: &mut Chunk, b: &mut Body, s: Handle, rng: &mut Rng) {
    let o = &d[s];
    let (mb, mo, tb) = (b.mass(&w.chem), o.mass(&w.chem), b.temp(&w.chem));
    let (fb, fo) = (b.gene(STRENGTH) * bond::lever(w, b) * b.built(&w.chem) / b.n(), o.gene(STRENGTH) * bond::lever(w, o) * o.built(&w.chem) / o.n());
    let (kg, swept) = intake(w, b, tb, |k| o.store[k]);
    // One draw finds an individual prey.
    let reached = if o.n() > 1.0 { swept } else if rng.f64() < swept { 1.0 } else { return };
    // The share of the members a meeting reaches (at least one whole member) and the chance of that meeting.
    let per = reached.max(1.0 / o.n());
    let chance = reached / per;
    let m = b.wanted(&w.chem, |k| per * o.store[k], kg * fb / (fb + fo + 1e-30), mb);
    let resist = 1.0 + lanes::thickness(w, w.life.sum(b.cell as usize)[1], w.mesh.cells().area()[b.cell as usize]);
    let hit = reached;
    // The bite tears this share of the body of each member it reaches; the stores they held spill in proportion, less
    // what the taker took (all they held, if they die).
    let (built, got) = (per * o.built(&w.chem), w.chem.mass_of(&m));
    let eaten = w.chem.matter().map_or(0.0, |k| if o.store[k] > 0.0 { m[k] / (per * o.store[k]) } else { 0.0 });
    let f = (got / built.max(1e-30)).max(eaten).min(1.0);
    // The taker pays to handle what it takes and to break the share it tears (`Body::breaking`).
    let cost = (w.cfg.phys.c_handle * got * resist + o.breaking(&w.chem, &w.cfg, f * per)) * chance;
    if !body::pay(w, d, b, cost) || hit <= 0.0 {
        return;
    }
    let mut o = std::mem::take(&mut d[s]);
    let m: Vec<f64> = m.iter().map(|a| a * hit / per).collect();
    let torn = if body::torn_to_death(&w.cfg.phys, f) { hit } else { f * hit };
    let mut rest: Vec<f64> = o.store.iter().zip(&m).map(|(s, a)| (torn * s - a).max(0.0)).collect();
    let got = w.chem.mass_of(&m);
    transfer(&w.chem, &mut d.counts.rounded[0], &mut o, b, &m);
    let ev = EventKind::Took { by: b.id, from: o.id, used: used_kg(w, b, tb, &m) as f32 };
    d.events.push(Event { day: w.day(), kind: ev, node: b.cell, value: got });
    if body::wound(w, d, s, o, (hit, f), &mut rest, TAKEN) {
        d[s].note(b.id, -(NOTE_SCALE * got / mo) as f32);
    }
}

/// Give to another body in the cell what it can spare (`Body::spare`, laws.md 5.3), up to what the
/// members it meets could take in the time and have room for (each of its members meets one of the other's, as many as
/// both have). The other remembers the giver.
pub(super) fn give(w: &World, d: &mut Chunk, b: &mut Body, s: Handle) {
    let chem = &w.chem;
    let (size, mass) = (b.size(chem), b.mass(chem));
    let spare = |k: usize| b.spare(chem, &w.cfg.phys, k, (size, mass));
    let all: f64 = (0..chem.ns()).map(|k| spare(k) * chem.subs[k].molar_mass).sum();
    let o = &d[s];
    let (mo, met, t) = (o.mass(chem), b.n().min(o.n()), o.temp(chem));
    let share = (w.cfg.phys.take * w.cfg.update_days() * o.supply(&w.chem, &w.cfg.phys, t) * met / o.n() / all.max(1e-9)).min(met / b.n());
    let mut m: Vec<f64> = (0..chem.ns()).map(|k| if o.uses(chem, k, t) { (spare(k) * share).min(o.room(chem, k, mo) * met / o.n()) } else { 0.0 }).collect();
    o.digest_room(chem, &mut m, mo);
    let got = w.chem.mass_of(&m);
    if got <= 0.0 || !body::pay(w, d, b, w.cfg.phys.c_handle * got) {
        return;
    }
    // Paying may have used some of what was to be put.
    let m: Vec<f64> = m.iter().zip(&b.store).map(|(a, s)| a.min(*s)).collect();
    let mut o = std::mem::take(&mut d[s]);
    transfer(&w.chem, &mut d.counts.rounded[0], b, &mut o, &m);
    o.note(b.id, (NOTE_SCALE * got / mo) as f32);
    o.stale = true;
    d.events.push(Event { day: w.day(), kind: EventKind::Gave { from: b.id, to: o.id }, node: b.cell, value: got });
    d[s] = o;
}
