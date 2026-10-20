//! Taking and giving: from the cell's ground (`take_node`), to it (`put`), from another body (`take_body`), and to
//! another body (`give`), each by the intake law of bodies (`Body::wanted`).

use crate::body::{self, light_cat, Body, STRENGTH, TAKEN};
use crate::chem::{Chemistry, Phase};
use crate::event::{Event, EventKind};
use crate::rng::Rng;
use crate::{bond, lanes, World};

/// A dealing counts in a body's memory (`Body::note`) as this many times the share of its mass moved.
const NOTE_SCALE: f64 = 10.0;

/// Mass of the amounts `m` that the body uses (`Body::uses`), kg.
fn used_kg(w: &World, b: &Body, m: &[f64]) -> f64 {
    let t = b.temp(&w.chem);
    m.iter().enumerate().filter(|&(k, _)| b.uses(&w.chem, k, t)).map(|(k, a)| a * w.chem.subs[k].molar_mass).sum()
}

/// The intake law of bodies, one for the ground and for other bodies: what body `b` can take in its update from what
/// lies over its cell, `from` mol of each substance in `n` members (the ground: one). Per day per kg-equivalent of its
/// supply it takes Holling's type II response to what the area it works (`sweep`) holds of what it takes, s, up to
/// `take`: take s / (take + s), so the thinner it lies, the more of its time goes to finding it (Holling 1959). Only what it
/// uses counts in s (`Body::uses`). A node's members each work their own area. Returns that, kg,
/// and the share of the `n` members its area holds: of a node, what that area holds, which may be less than one
/// member; of one individual body (`n` 1), all of it.
fn intake(w: &World, b: &Body, from: impl Fn(usize) -> f64, n: f64) -> (f64, f64) {
    let (p, area, days) = (&w.cfg.phys, w.mesh.cells().area()[b.cell as usize], b.interval(w) as f64);
    let t = b.temp(&w.chem);
    let wants = b.k.upt.iter().filter(|&&(k, _)| b.uses(&w.chem, k, t));
    let lying: f64 = wants.map(|&(k, _)| from(k).max(0.0) * w.chem.subs[k].molar_mass).sum();
    let (found, supply) = (p.sweep * lying / area, b.supply(&w.chem, p, t));
    let held = (p.sweep * supply * days * n / area).min(n) / n;
    (p.take * found / (p.take + found) * days * supply, if n > 1.0 { held } else { 1.0 })
}

/// Take loose solids and liquids from the cell's ground by the intake law (`intake`), no more than its stores have room
/// for.
pub(super) fn take_node(w: &mut World, slot: usize, b: &mut Body) {
    let i = b.cell as usize;
    let tg = w.t_ground_last(i);
    let free = |k: usize| if w.chem.phase(k, tg) != Phase::Gas { w.g(k, i) } else { 0.0 };
    let mut m = b.wanted(&w.chem, b.paces(w), free, intake(w, b, free, 1.0).0, b.mass(&w.chem));
    if !body::pay(w, b, w.cfg.phys.c_handle * w.chem.mass_of(&m), b.interval(w) as f64) {
        return;
    }
    let ev = EventKind::Took { by: b.id(slot), from: u64::MAX, used: used_kg(w, b, &m) as f32, cat: [light_cat(&w.chem, &b.genes), 0.0] };
    w.events.push(Event { day: w.day(), kind: ev, node: b.cell, value: w.chem.mass_of(&m) });
    for (k, a) in m.iter_mut().enumerate() {
        // The ground pays the heat that goes with what is taken; if it cannot, less is taken.
        let cp = w.chem.subs[k].cp;
        let want = *a * (cp * tg + w.chem.fused_heat(k, tg, |j| w.g(j, i)));
        let took;
        (*a, took) = w.take_from_ground(i, (k, *a, cp), want, w.heat_cap_ground(i));
        b.store[k] += *a;
        b.e += took;
    }
    b.digest(&w.chem, w.chem.matter().map_or(0.0, |k| m[k]), &mut w.counts.rounded[0]);
}

/// Release stores above the level the body keeps (`Body::level`) to the cell's ground.
pub(super) fn put(w: &mut World, b: &mut Body) {
    let mass = b.mass(&w.chem);
    let m: Vec<f64> = (0..w.chem.ns()).map(|k| (b.store[k] - b.level(&w.chem, k, mass)).clamp(0.0, b.store[k])).collect();
    if !body::pay(w, b, w.cfg.phys.c_handle * w.chem.mass_of(&m), b.interval(w) as f64) {
        return;
    }
    let m: Vec<f64> = m.iter().zip(&b.store).map(|(&a, &held)| a.min(held)).collect();
    let th = b.thermal(&w.chem);
    body::to_ground(w, b, &th, &m);
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
/// reaches what the members its area holds hold, and a quicker body gets away with chance 1 - its interval / the
/// taker's (the taker has paid; of a node's bites, that share). The bite tears each member it reaches (`body::wound`;
/// laws.md 6.8).
pub fn take_body(w: &mut World, slot: usize, b: &mut Body, s: usize, rng: &mut Rng) {
    let o = &w.life.bodies[s];
    let (mb, mo) = (b.mass(&w.chem), o.mass(&w.chem));
    let (fb, fo) = (b.gene(STRENGTH) * bond::lever(w, b) * b.built(&w.chem) / b.n(), o.gene(STRENGTH) * bond::lever(w, o) * o.built(&w.chem) / o.n());
    let (kg, reached) = intake(w, b, |k| o.store[k], o.n());
    let m = b.wanted(&w.chem, b.paces(w), |k| reached * o.store[k], kg * fb / (fb + fo + 1e-30), mb);
    let resist = 1.0 + lanes::thickness(w, w.life.sum[b.cell as usize][1], w.mesh.cells().area()[b.cell as usize]);
    let away = (1.0 - o.interval(w) as f64 / b.interval(w) as f64).max(0.0);
    let hit = reached * if b.n() > 1.0 { 1.0 - away } else { (rng.f64() >= away) as u8 as f64 };
    // The bite tears this share of the body of each member it reaches; the stores they held spill in proportion, less
    // what the taker took (all they held, if they die).
    let (built, got) = (reached * o.built(&w.chem), w.chem.mass_of(&m));
    let eaten = w.chem.matter().map_or(0.0, |k| if o.store[k] > 0.0 { m[k] / (reached * o.store[k]) } else { 0.0 });
    let f = (got / built.max(1e-30)).max(eaten).min(1.0);
    // The taker pays to handle what it takes and to break the share it tears (`Body::breaking`).
    let cost = w.cfg.phys.c_handle * got * resist + o.breaking(&w.chem, &w.cfg, f * reached);
    if !body::pay(w, b, cost, b.interval(w) as f64) || hit <= 0.0 {
        return;
    }
    let mut o = std::mem::take(&mut w.life.bodies[s]);
    let m: Vec<f64> = m.iter().map(|a| a * hit / reached).collect();
    let torn = if body::torn_to_death(&w.cfg.phys, f) { hit } else { f * hit };
    let rest: Vec<f64> = o.store.iter().zip(&m).map(|(s, a)| (torn * s - a).max(0.0)).collect();
    let got = w.chem.mass_of(&m);
    transfer(&w.chem, &mut w.counts.rounded[0], &mut o, b, &m);
    let ev = EventKind::Took { by: b.id(slot), from: o.id(s), used: used_kg(w, b, &m) as f32, cat: [light_cat(&w.chem, &b.genes), light_cat(&w.chem, &o.genes)] };
    w.events.push(Event { day: w.day(), kind: ev, node: b.cell, value: got });
    if body::wound(w, s, o, (hit, f), &rest, TAKEN) {
        w.life.bodies[s].note(b.id(slot), -(NOTE_SCALE * got / mo) as f32);
    }
}

/// Give to another body in the cell what it can spare (`Body::spare`, laws.md 5.3), up to what the
/// members it meets could take in the time and have room for (each of its members meets one of the other's, as many as
/// both have). The other remembers the giver.
pub(super) fn give(w: &mut World, slot: usize, b: &mut Body, s: usize) {
    let chem = &w.chem;
    let (size, mass) = (b.size(chem), b.mass(chem));
    let spare = |k: usize| b.spare(chem, &w.cfg.phys, k, (size, mass));
    let all: f64 = (0..chem.ns()).map(|k| spare(k) * chem.subs[k].molar_mass).sum();
    let o = &w.life.bodies[s];
    let (mo, paces, met, t) = (o.mass(chem), o.paces(w), b.n().min(o.n()), o.temp(chem));
    let share = (w.cfg.phys.take * b.interval(w) as f64 * o.supply(&w.chem, &w.cfg.phys, t) * met / o.n() / all.max(1e-9)).min(met / b.n());
    let mut m: Vec<f64> = (0..chem.ns()).map(|k| if o.uses(chem, k, t) { (spare(k) * share).min(o.room(chem, paces, k, mo) * met / o.n()) } else { 0.0 }).collect();
    o.digest_room(chem, paces, &mut m, mo);
    let got = w.chem.mass_of(&m);
    if got <= 0.0 || !body::pay(w, b, w.cfg.phys.c_handle * got, b.interval(w) as f64) {
        return;
    }
    // Paying may have used some of what was to be put.
    let m: Vec<f64> = m.iter().zip(&b.store).map(|(a, s)| a.min(*s)).collect();
    let mut o = std::mem::take(&mut w.life.bodies[s]);
    transfer(&w.chem, &mut w.counts.rounded[0], b, &mut o, &m);
    o.note(b.id(slot), (NOTE_SCALE * got / mo) as f32);
    w.see(&mut o);
    w.events.push(Event { day: w.day(), kind: EventKind::Gave { from: b.id(slot), to: o.id(s) }, node: b.cell, value: got });
    w.life.bodies[s] = o;
}
