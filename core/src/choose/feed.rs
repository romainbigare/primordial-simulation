//! Taking and giving: from the cell's ground (`take_node`), to it (`put`), from another body (`take_body`), and to
//! another body (`give`), each by the intake law of bodies (`Body::wanted`).

use crate::body::{self, light_cat, Body, Layout, STRENGTH, TAKEN};
use crate::chem::Phase;
use crate::event::{Event, EventKind};
use crate::rng::Rng;
use crate::{bond, lanes, World};

/// A dealing counts in what a body remembers (`Body::note`) as this many times the share of its mass it moved: one of a
/// tenth of its mass counts fully.
const NOTE_SCALE: f64 = 10.0;
/// A giver keeps this many times the safe minimum of each store.
const GIVE_KEEPS: f64 = 2.0;

/// Mass of the amounts `m` that the body's reactions can burn, kg.
fn burnable(w: &World, b: &Body, m: &[f64]) -> f64 {
    let t = b.temp(&w.chem);
    m.iter().enumerate().filter(|&(k, _)| b.burns_at(&w.chem, k, t)).map(|(k, a)| a * w.chem.subs[k].molar_mass).sum()
}

/// Most a body takes per day per kg of its mass from amounts `from` (mol) spread over a cell: `Physics::take`, and no
/// more than the area it works per day per kg (`Physics::sweep`) holds of what it takes by its uptake: the mass per m²
/// of the cell of those substances. So a thin cover is as hard to find as it is thin, whatever the body could carry.
fn reach(w: &World, b: &Body, from: impl Fn(usize) -> f64) -> f64 {
    let dens: f64 = b.k.upt.iter().filter(|&&(k, _)| b.can_take(&w.chem, k)).map(|&(k, _)| from(k).max(0.0) * w.chem.subs[k].molar_mass).sum::<f64>() / w.grid.area();
    w.cfg.phys.take.min(w.cfg.phys.sweep * dens)
}

/// Take loose solids and liquids from the cell's ground, as `reach` allows for the mass it takes by (`Body::supply`; a
/// node's members each work their own area: it is the node's total), and no more than its stores have room for.
pub(super) fn take_node(w: &mut World, slot: usize, b: &mut Body) {
    let (i, n) = (b.cell as usize, w.grid.n());
    let tg = w.t_ground_last(i);
    let free = |k: usize| if w.chem.phase(k, tg) != Phase::Gas { w.g(k, i) } else { 0.0 };
    let mass = b.mass(&w.chem);
    let m = b.wanted(&w.chem, free, reach(w, b, free) * b.interval() as f64 * b.supply(&w.chem, &w.cfg.phys), mass);
    if !body::pay(w, b, w.cfg.phys.c_handle * w.chem.mass_of(&m), b.interval() as f64) {
        return;
    }
    let ev = EventKind::Took { by: b.id(slot), from: u64::MAX, burn: burnable(w, b, &m) as f32, cat: [light_cat(&w.chem, &b.genes), 0.0] };
    w.events.push(Event { day: w.day, kind: ev, node: b.cell, value: w.chem.mass_of(&m) });
    let mut matter = 0.0;
    for (k, &a) in m.iter().enumerate() {
        // The ground pays the heat that goes with what is taken; if it cannot, less is taken.
        let cp = w.chem.subs[k].cp;
        let (a, took) = w.take_paid(i, a, cp, a * (cp * tg + w.chem.fused_heat(k, tg, |j| w.g(j, i))));
        w.ground[k * n + i] -= a;
        b.store[k] += a;
        b.e += took;
        if Some(k) == w.chem.matter() {
            matter = a;
        }
    }
    b.digest(&w.chem, matter);
}

/// Release stores above the level the body keeps (`Body::level`) to the cell's ground.
pub(super) fn put(w: &mut World, b: &mut Body) {
    let mass = b.mass(&w.chem);
    let m: Vec<f64> = (0..w.chem.ns()).map(|k| (b.store[k] - b.level(&w.chem, k, mass)).clamp(0.0, b.store[k])).collect();
    if !body::pay(w, b, w.cfg.phys.c_handle * w.chem.mass_of(&m), b.interval() as f64) {
        return;
    }
    let m: Vec<f64> = m.iter().zip(&b.store).map(|(&a, &held)| a.min(held)).collect();
    body::to_ground(w, b, &m);
}

/// Move amounts `m` (mol) from `from` to `to`, with their heat (`Body::heat_with`) and their share of `from`'s brood
/// (`Body::lose`).
fn transfer(w: &World, from: &mut Body, to: &mut Body, m: &[f64]) {
    let q = from.heat_with(&w.chem, m);
    from.lose(&w.chem, m);
    for (k, &a) in m.iter().enumerate() {
        to.store[k] += a;
    }
    from.e -= q;
    to.e += q;
}

/// Take from another body in the cell, by uptake selectivity, as much as its mass (`Body::supply`), its stores' room and
/// its force against the other's (a member's force: strength x what a held part adds x its body, `Body::built`, as
/// `bond::force`: a hurt body is weaker) allow, through the bodies of the cell (`lanes::thickness`). The quicker body
/// acts first: one that decides more often than the taker has acted again within part of the taker's interval and gets
/// away with chance 1 - its interval / the taker's (the taker has paid). From a node (its members lie spread over the
/// cell) the taker also reaches only the area it works (`reach`); an individual it perceives is already found.
///
/// A bite breaks body matter: the kg taken tear that share of the other's body (`Body::built`), or the share of its
/// body matter the taker ate if that is more (matter eaten is body torn): its body matter and the stores that matter
/// held, in proportion (`body::wound`). The taker keeps what it took (digested, `Body::digest`); what is torn off and
/// not taken goes to the ground. An individual lives on smaller (its force falls with its body; it rebuilds the matter
/// as it grows) and dies when half of its body or more is torn or a store falls below its safe minimum. A bite of less
/// than a node's member's body grazes: every member loses that share of its matter and lives; one of a member's body or
/// more takes whole members, the same share of them, so the members killed carry the bite. What the taker takes of a
/// store beyond the share those members held comes from the members who live.
pub fn take_body(w: &mut World, slot: usize, b: &mut Body, s: usize, rng: &mut Rng) {
    let o = &w.life.bodies[s];
    let (mb, mo) = (b.mass(&w.chem), o.mass(&w.chem));
    let (fb, fo) = (b.gene(STRENGTH) * bond::lever(w, b) * b.built(&w.chem) / b.n(), o.gene(STRENGTH) * bond::lever(w, o) * o.built(&w.chem) / o.n());
    let rate = if o.n() > 1.0 { reach(w, b, |k| o.store[k]).min(w.cfg.phys.take * fb / (fb + fo + 1e-30)) } else { w.cfg.phys.take * fb / (fb + fo + 1e-30) };
    let m = b.wanted(&w.chem, |k| o.store[k], rate * b.interval() as f64 * b.supply(&w.chem, &w.cfg.phys), mb);
    let resist = 1.0 + lanes::thickness(w, w.life.sum[b.cell as usize][1]);
    let away = rng.f64() < 1.0 - o.interval() as f64 / b.interval() as f64;
    // The bite tears this share of the body; the stores it held spill in proportion, less what the taker took of them (a
    // taker that takes more of a store than the share holds takes it from the rest).
    let (built0, matter0, got) = (o.built(&w.chem), o.matter_kg(&w.chem), w.chem.mass_of(&m));
    let eaten = w.chem.matter().map_or(0.0, |k| if o.store[k] > 0.0 { m[k] / o.store[k] } else { 0.0 });
    let f = (got / built0.max(1e-30)).max(eaten).min(1.0);
    // The taker pays to handle what it takes and to break the share it tears (`Body::breaking`).
    let cost = w.cfg.phys.c_handle * got * resist + o.breaking(&w.chem, &w.cfg, f);
    if !body::pay(w, b, cost, b.interval() as f64) || away {
        return;
    }
    let mut o = std::mem::take(&mut w.life.bodies[s]);
    let rest: Vec<f64> = o.store.iter().zip(&m).map(|(s, a)| (f * s - a).max(0.0)).collect();
    transfer(w, &mut o, b, &m);
    b.digest(&w.chem, w.chem.matter().map_or(0.0, |k| m[k]));
    let ev = EventKind::Took { by: b.id(slot), from: o.id(s), burn: burnable(w, b, &m) as f32, cat: [light_cat(&w.chem, &b.genes), light_cat(&w.chem, &o.genes)] };
    w.events.push(Event { day: w.day, kind: ev, node: b.cell, value: got });
    if body::wound(w, s, o, f, &rest, TAKEN, built0, matter0) {
        w.life.bodies[s].note(b.id(slot), -(NOTE_SCALE * got / mo) as f32);
    }
}

/// Put to another body in the cell: from the stores held above `GIVE_KEEPS` times their safe minimum (never its matter
/// nor a solid bodies build, nor a key liquid below its level), by their shares, up to what the other could take by its
/// own size in the time (`Physics::take` x interval x its `Body::supply`), as far as it has room. A node gives as one
/// member: a share 1/count of its spare (its members cannot all reach one body). The other remembers this of the giver.
pub(super) fn give(w: &mut World, slot: usize, b: &mut Body, s: usize) {
    // The receiver's numbers (`supply` goes by its pace, `can_take` by its genes' catalysts) need `prepare`: a body of a
    // world just loaded has not had its update yet, so without this a gift to it differs from the same gift in the run
    // that was not saved.
    let World { chem, cfg, life, .. } = &mut *w;
    life.bodies[s].prepare(chem, &cfg.phys);
    let (chem, lay) = (&w.chem, Layout::of(&w.chem));
    let (size, mass) = (b.size(chem), b.mass(chem));
    let keep = |k: usize| if b.wet >> k & 1 == 1 { b.level(chem, k, mass) } else { 0.0 };
    let spare = |k: usize| if chem.matter() == Some(k) || chem.is_material(k) { 0.0 } else { (b.store[k] - keep(k).max(GIVE_KEEPS * b.gene(lay.min + k).max(0.0) * size / chem.subs[k].molar_mass)).max(0.0) };
    let all: f64 = (0..chem.ns()).map(|k| spare(k) * chem.subs[k].molar_mass).sum();
    let o = &w.life.bodies[s];
    let mo = o.mass(chem);
    let share = (w.cfg.phys.take * b.interval() as f64 * o.supply(&w.chem, &w.cfg.phys) / all.max(1e-9)).min(1.0 / b.n());
    let mut m: Vec<f64> = (0..chem.ns()).map(|k| if o.can_take(chem, k) { (spare(k) * share).min((o.level(chem, k, mo) - o.store[k]).max(0.0)) } else { 0.0 }).collect();
    o.digest_room(chem, &mut m, mo);
    let got = w.chem.mass_of(&m);
    if got <= 0.0 || !body::pay(w, b, w.cfg.phys.c_handle * got, b.interval() as f64) {
        return;
    }
    // Paying may have used some of what was to be put.
    let m: Vec<f64> = m.iter().zip(&b.store).map(|(a, s)| a.min(*s)).collect();
    let mut o = std::mem::take(&mut w.life.bodies[s]);
    transfer(w, b, &mut o, &m);
    o.digest(&w.chem, w.chem.matter().map_or(0.0, |k| m[k]));
    o.note(b.id(slot), (NOTE_SCALE * got / mo) as f32);
    o.vis = o.visible(&w.chem, &w.cfg.phys);
    w.events.push(Event { day: w.day, kind: EventKind::Gave { from: b.id(slot), to: o.id(s) }, node: b.cell, value: got });
    w.life.bodies[s] = o;
}
