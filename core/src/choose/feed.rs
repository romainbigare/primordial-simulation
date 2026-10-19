//! Taking and giving: from the cell's ground (`take_node`), to it (`put`), from another body (`take_body`), and to
//! another body (`give`), each by the intake law of bodies (`Body::wanted`).

use crate::body::{self, light_cat, Body, Layout, STRENGTH, TAKEN};
use crate::chem::Phase;
use crate::event::{Event, EventKind};
use crate::rng::Rng;
use crate::{bond, lanes, World};

/// A dealing counts in a body's memory (`Body::note`) as this many times the share of its mass moved.
const NOTE_SCALE: f64 = 10.0;
/// A giver keeps this many times the safe minimum of each store.
const GIVE_KEEPS: f64 = 2.0;

/// Mass of the amounts `m` that the body's reactions can burn, kg.
fn burnable(w: &World, b: &Body, m: &[f64]) -> f64 {
    let t = b.temp(&w.chem);
    m.iter().enumerate().filter(|&(k, _)| b.burns_at(&w.chem, k, t)).map(|(k, a)| a * w.chem.subs[k].molar_mass).sum()
}

/// Most a body takes per day per kg of its mass from amounts `from` spread over a cell: `take`, and no more than the
/// area it works (`sweep`) holds of what it takes, so a thin cover is as hard to find as it is thin.
fn reach(w: &World, b: &Body, from: impl Fn(usize) -> f64) -> f64 {
    let dens: f64 = b.k.upt.iter().filter(|&&(k, _)| b.can_take(&w.chem, k)).map(|&(k, _)| from(k).max(0.0) * w.chem.subs[k].molar_mass).sum::<f64>() / w.mesh.cells().area()[b.cell as usize];
    w.cfg.phys.take.min(w.cfg.phys.sweep * dens)
}

/// Take loose solids and liquids from the cell's ground, as `reach` allows for its supply (a node's members each work
/// their own area), no more than its stores have room for.
pub(super) fn take_node(w: &mut World, slot: usize, b: &mut Body) {
    let i = b.cell as usize;
    let tg = w.t_ground_last(i);
    let free = |k: usize| if w.chem.phase(k, tg) != Phase::Gas { w.g(k, i) } else { 0.0 };
    let mass = b.mass(&w.chem);
    let m = b.wanted(&w.chem, b.paces(w), free, reach(w, b, free) * b.interval(w) as f64 * b.supply(&w.chem, &w.cfg.phys, b.temp(&w.chem)), mass);
    if !body::pay(w, b, w.cfg.phys.c_handle * w.chem.mass_of(&m), b.interval(w) as f64) {
        return;
    }
    let ev = EventKind::Took { by: b.id(slot), from: u64::MAX, burn: burnable(w, b, &m) as f32, cat: [light_cat(&w.chem, &b.genes), 0.0] };
    w.events.push(Event { day: w.day(), kind: ev, node: b.cell, value: w.chem.mass_of(&m) });
    let mut matter = 0.0;
    for (k, &a) in m.iter().enumerate() {
        // The ground pays the heat that goes with what is taken; if it cannot, less is taken.
        let cp = w.chem.subs[k].cp;
        let want = a * (cp * tg + w.chem.fused_heat(k, tg, |j| w.g(j, i)));
        let (a, took) = w.take_from_ground(i, (k, a, cp), want, w.heat_cap_ground(i));
        b.store[k] += a;
        b.e += took;
        if Some(k) == w.chem.matter() {
            matter = a;
        }
    }
    b.digest(&w.chem, matter, &mut w.counts.rounded[0]);
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

/// Move amounts `m` (mol) from `from` to `to`, with their heat and their share of `from`'s brood.
fn transfer(w: &World, from: &mut Body, to: &mut Body, m: &[f64]) {
    let q = from.heat_with(&w.chem, &from.thermal(&w.chem), m);
    from.lose(&w.chem, m);
    for (k, &a) in m.iter().enumerate() {
        to.store[k] += a;
    }
    from.e -= q;
    to.e += q;
}

/// Take from another body in the cell by uptake selectivity, as far as its supply, its stores' room and its force
/// against the other's allow. A quicker body gets away with chance 1 - its interval / the taker's (the taker has paid);
/// from a node the taker reaches only the area it works. The bite tears body matter (`body::wound`; laws.md 6.8).
pub fn take_body(w: &mut World, slot: usize, b: &mut Body, s: usize, rng: &mut Rng) {
    let o = &w.life.bodies[s];
    let (mb, mo) = (b.mass(&w.chem), o.mass(&w.chem));
    let (fb, fo) = (b.gene(STRENGTH) * bond::lever(w, b) * b.built(&w.chem) / b.n(), o.gene(STRENGTH) * bond::lever(w, o) * o.built(&w.chem) / o.n());
    let rate = if o.n() > 1.0 { reach(w, b, |k| o.store[k]).min(w.cfg.phys.take * fb / (fb + fo + 1e-30)) } else { w.cfg.phys.take * fb / (fb + fo + 1e-30) };
    let m = b.wanted(&w.chem, b.paces(w), |k| o.store[k], rate * b.interval(w) as f64 * b.supply(&w.chem, &w.cfg.phys, b.temp(&w.chem)), mb);
    let resist = 1.0 + lanes::thickness(w, w.life.sum[b.cell as usize][1], w.mesh.cells().area()[b.cell as usize]);
    let away = rng.f64() < 1.0 - o.interval(w) as f64 / b.interval(w) as f64;
    // The bite tears this share of the body; the stores it held spill in proportion, less what the taker took.
    let (built0, matter0, got) = (o.built(&w.chem), o.matter_kg(&w.chem), w.chem.mass_of(&m));
    let eaten = w.chem.matter().map_or(0.0, |k| if o.store[k] > 0.0 { m[k] / o.store[k] } else { 0.0 });
    let f = (got / built0.max(1e-30)).max(eaten).min(1.0);
    // The taker pays to handle what it takes and to break the share it tears (`Body::breaking`).
    let cost = w.cfg.phys.c_handle * got * resist + o.breaking(&w.chem, &w.cfg, f);
    if !body::pay(w, b, cost, b.interval(w) as f64) || away {
        return;
    }
    let mut o = std::mem::take(&mut w.life.bodies[s]);
    let rest: Vec<f64> = o.store.iter().zip(&m).map(|(s, a)| (f * s - a).max(0.0)).collect();
    transfer(w, &mut o, b, &m);
    b.digest(&w.chem, w.chem.matter().map_or(0.0, |k| m[k]), &mut w.counts.rounded[0]);
    let ev = EventKind::Took { by: b.id(slot), from: o.id(s), burn: burnable(w, b, &m) as f32, cat: [light_cat(&w.chem, &b.genes), light_cat(&w.chem, &o.genes)] };
    w.events.push(Event { day: w.day(), kind: ev, node: b.cell, value: got });
    if body::wound(w, s, o, f, &rest, TAKEN, built0, matter0) {
        w.life.bodies[s].note(b.id(slot), -(NOTE_SCALE * got / mo) as f32);
    }
}

/// Give to another body in the cell from stores above `GIVE_KEEPS` times their safe minimum (laws.md 5.3), up to what the
/// other could take in the time and has room for. A node gives as one member. The other remembers the giver.
pub(super) fn give(w: &mut World, slot: usize, b: &mut Body, s: usize) {
    let (chem, lay) = (&w.chem, Layout::of(&w.chem));
    let (size, mass) = (b.size(chem), b.mass(chem));
    let keep = |k: usize| if b.wet >> k & 1 == 1 { b.level(chem, k, mass) } else { 0.0 };
    let spare = |k: usize| if chem.matter() == Some(k) || chem.is_material(k) { 0.0 } else { (b.store[k] - keep(k).max(GIVE_KEEPS * b.gene(lay.min + k).max(0.0) * size / chem.subs[k].molar_mass)).max(0.0) };
    let all: f64 = (0..chem.ns()).map(|k| spare(k) * chem.subs[k].molar_mass).sum();
    let o = &w.life.bodies[s];
    let (mo, paces) = (o.mass(chem), o.paces(w));
    let share = (w.cfg.phys.take * b.interval(w) as f64 * o.supply(&w.chem, &w.cfg.phys, o.temp(&w.chem)) / all.max(1e-9)).min(1.0 / b.n());
    let mut m: Vec<f64> = (0..chem.ns()).map(|k| if o.can_take(chem, k) { (spare(k) * share).min(o.room(chem, paces, k, mo)) } else { 0.0 }).collect();
    o.digest_room(chem, paces, &mut m, mo);
    let got = w.chem.mass_of(&m);
    if got <= 0.0 || !body::pay(w, b, w.cfg.phys.c_handle * got, b.interval(w) as f64) {
        return;
    }
    // Paying may have used some of what was to be put.
    let m: Vec<f64> = m.iter().zip(&b.store).map(|(a, s)| a.min(*s)).collect();
    let mut o = std::mem::take(&mut w.life.bodies[s]);
    transfer(w, b, &mut o, &m);
    o.digest(&w.chem, w.chem.matter().map_or(0.0, |k| m[k]), &mut w.counts.rounded[0]);
    o.note(b.id(slot), (NOTE_SCALE * got / mo) as f32);
    w.see(&mut o);
    w.events.push(Event { day: w.day(), kind: EventKind::Gave { from: b.id(slot), to: o.id(s) }, node: b.cell, value: got });
    w.life.bodies[s] = o;
}
