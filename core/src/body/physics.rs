//! Flow and Change on a body over the time since its last update: light, gas, reactions, liquids and heat (laws.md
//! 6.3, 6.4).

use super::exchange::{breath, film, surface, through, Film};
use super::live::{LiveTmp, Site};
use super::pay::breathes;
use super::{Body, Layout, COOL_T, DENATURE};
use crate::change::{send_gas, GasMove, Reactor};
use crate::ground::{heat_above_floor, share, Melting};
use crate::{chem::Phase, math::{mean_decay, one_minus_exp, ones}, life::Domain, store::Node, World};
use crate::units::{MAX_SUBSTANCES, P_REF};

/// The heat of an update as its stages go.
struct Heat {
    /// At the start: the body's mass, kg, temperature, K, conductance to the surface, W/K; its ground's heat capacity, J/K
    mass: f64,
    t0: f64,
    g: f64,
    gcap: f64,
    /// Heat released beside what its stores hold at their temperature, J; heat capacity of the gas put out, J/K
    heat: f64,
    c_out: f64,
    /// Light it absorbed, J; light set aside for it that it took from its bodies' node (`Class::Bodies`), J
    lit: f64,
    took: f64,
    /// The ground temperature below which what it evaporated would stop, K: the highest dew point of the air it breathes
    /// among the liquids it evaporated, or `COOL_T` where it evaporated to cool; the heat of what it would have
    /// evaporated with the heat its surface conducts in that its bodies' node did not hold for it, J
    stop: Option<f64>,
    asked: f64,
}

/// What the ground of a cell offers to soak up: its liquids that dissolve (bits), their sum, mol, and the rate, per day.
struct Soil {
    wets: u64,
    in_ground: f64,
    rate: f64,
}

/// The physics of a body of `mass` kg at the plateau state `th0` (`Body::thermal`) over its update, with the heat its
/// upkeep released. Returns the light it caught per kg per day, J.
#[allow(clippy::too_many_arguments)]
pub(super) fn physics(w: &World, d: &mut Domain, b: &mut Body, rc: &mut Reactor, tmp: &mut LiveTmp, s: &Site, mass: f64, upkeep_heat: f64, th0: Melting) -> f64 {
    let gcap = d.moves.heat_cap_ground(w, s.i);
    let (chem, cfg) = (&w.chem, &w.cfg);
    let fall = rain_caught(w, b, s);
    // It soaks through the mass that works; its conductance is taken at its temperature at the start.
    let wk = b.size(chem) / mass;
    // The stages move sensible heat; liquids that come and go bring and take their fusion enthalpy as heat, until the
    // stores hold theirs again at the end.
    let c0 = b.heat_cap(chem);
    let (t0, fusion0) = (th0.t, b.e - c0 * th0.t);
    // One boundary layer for its heat and its breath until its stores change.
    let f0 = film(chem, cfg, b, mass, s.cover);
    let g = surface(cfg, b, mass, &f0, s.walls, t0, s.tg);
    let (caught, lit) = catch_light(w, b, s, f0.leaf);
    let mut h = Heat { mass, t0, g, gcap, heat: 0.0, c_out: 0.0, lit, took: 0.0, stop: None, asked: 0.0 };
    let reach = gas_pool(w, d, b, s, &f0, tmp);
    let used = react(w, b, rc, tmp, s, &mut h, c0, upkeep_heat);
    settle_gas(w, d, b, &rc.dpool, s, &mut h);
    h.took = store_light(w, d, b, s, caught, used);
    let soil = soil(w, d, b, s, wk);
    let mass1 = b.mass(&w.chem);
    let (lost, moved) = evaporate(w, d, b, tmp, s, &mut h, (&soil, reach));
    let mass2 = if moved { b.mass(&w.chem) } else { mass1 };
    soak(w, d, b, s, h.gcap, &mut h.heat, &soil, &lost, fall, mass2);
    roots(w, d, b, s, &mut h, mass2, wk);
    let fusion1 = b.fused_of(&w.chem, &th0, &b.store);
    h.heat += fusion0 - fusion1;
    relax_heat(w, d, b, s, &h);
    b.e += fusion1;
    lit / mass / s.days
}

/// The rain that fell on its capture area since its last update, mol: the cell's bodies intercept 1 - exp(-x) of it, x
/// their capture area per ground area, shared by height as light is (`survey::shade`).
fn rain_caught(w: &World, b: &mut Body, s: &Site) -> f64 {
    let (i, World { rained, canopy, .. }) = (s.i, w);
    let fell = rained[i] - std::mem::replace(&mut b.rained, rained[i]);
    fell * b.vis[1] as f64 * b.n() * s.open * b.shade * mean_decay(canopy.get(i).map_or(0.0, |&c| c as f64))
}

/// Light caught since the last update through any openings, less what it reflects, and what liquid over it leaves, J.
/// It already warmed the surface as it fell, but for the share set aside for light reactions (`store_light`).
fn catch_light(w: &World, b: &mut Body, s: &Site, leaf: f64) -> (f64, f64) {
    let caught = leaf * (w.light[s.i] - b.mark) * (1.0 - b.reflectance()) * s.open;
    b.mark = w.light[s.i];
    let reach = (-s.depth / w.cfg.phys.light_depth).exp();
    (caught, caught * reach)
}

/// The air the body exchanges gas with in this time (`breath`), at the shares of the air it sees (`World::a_seen`), no
/// more than the node holds.
fn gas_pool(w: &World, d: &Domain, b: &Body, s: &Site, f: &Film, tmp: &mut LiveTmp) -> f64 {
    let (chem, j) = (&w.chem, s.j);
    let tot = d.moves.a_seen_all(w, j);
    let reach = breath(chem, &w.cfg, b, f, s.days, s.depth);
    let (seen, air) = (d.moves.seen(w, j), d.moves.amounts(w, Node::air(j)));
    tmp.x.clear();
    tmp.pool.clear();
    for k in 0..chem.ns() {
        let x = if tot > 0.0 { seen(k) / tot } else { 0.0 };
        tmp.x.push(x);
        tmp.pool.push((x * reach).min(air(k)));
    }
    reach
}

/// Catalysed reactions at the body's temperature, held by the surface as a store of the heat it can conduct into the
/// body over the time (`g` dt `tg`). Burnings ran on demand and go here at the bare rate. Returns the light used, J.
#[allow(clippy::too_many_arguments)]
fn react(w: &World, b: &mut Body, rc: &mut Reactor, tmp: &mut LiveTmp, s: &Site, h: &mut Heat, c0: f64, upkeep_heat: f64) -> f64 {
    let (chem, cfg, t0, dt) = (&w.chem, &w.cfg, h.t0, s.dt);
    let fixed = h.g * dt * s.tg / t0.max(1.0);
    let mut e = t0 * (fixed + c0);
    let act = b.activity(chem, &cfg.phys);
    tmp.cat.clear();
    tmp.cat.extend(b.k.cat.iter().enumerate().map(|(r, &f)| Body::catalyst(f, if f > 1.0 { act * b.inhibited(chem, r, h.mass, t0) } else { act })));
    for &(r, _) in b.k.burn.iter().filter(|p| breathes(chem, p.0, t0)) {
        tmp.cat[r] = 1.0;
    }
    let den = (&tmp.cat[..], b.gene(DENATURE), cfg.phys.unfold);
    let held = b.matter_mol(chem);
    let used = rc.react(chem, &mut b.store, Some(&mut tmp.pool), &mut e, fixed, Some(den), h.lit, dt);
    b.brood_share(chem, held);
    let c1 = b.heat_cap(chem);
    h.heat = e - t0 * (fixed + c1) + h.lit - used + upkeep_heat;
    b.e = t0 * c1;
    used
}

/// Gas taken brings its heat at the air's temperature and its latent heat; gas put out takes its latent heat, as far as
/// heat pays for it: the body's above `T_FLOOR`, what it released, and what the surface conducts in, no more than its
/// bodies' node holds for it.
fn settle_gas(w: &World, d: &mut Domain, b: &mut Body, dpool: &[f64], s: &Site, h: &mut Heat) {
    let (j, lift) = (s.j, w.lift(s.i));
    let gas = GasMove::of(&w.chem, dpool, d.moves.t_air(w, j));
    b.e += gas.q_in;
    d.moves.take(w, Node::air(j), 0, 0.0, gas.q_in);
    let heat = h.heat + gas.back;
    let conduct = (h.g * s.dt * s.tg).min(held_for(w, d, b, s.i));
    let paid = share((heat_above_floor(b.e, b.heat_cap(&w.chem)) + heat + conduct).max(0.0), gas.pay);
    send_gas(dpool, paid, |k, sent, kept| {
        b.store[k] += kept;
        if sent != 0.0 {
            d.moves.gas(w, k, sent, lift);
        }
    });
    (h.heat, h.c_out) = (heat - paid * gas.pay, paid * gas.c_out);
}

/// What its light reactions stored is paid by the light set aside for it in its bodies' node (by `Body::keep`, as far
/// as the node holds it); it keeps the share it stored now. Returns the light it took, J.
fn store_light(w: &World, d: &mut Domain, b: &mut Body, s: &Site, caught: f64, used: f64) -> f64 {
    let stored = w.cfg.phys.light_yield * used;
    let took = (b.keep as f64 * caught).min(d.moves.heat(w, Node::bodies(s.i))).max(0.0);
    d.moves.take(w, Node::bodies(s.i), 0, 0.0, took);
    if caught > 0.0 {
        b.keep = (stored / caught) as f32;
    }
    took
}

/// The liquids of the cell's ground a body soaks up (`Soil`); a body afloat reaches none.
fn soil(w: &World, d: &Domain, b: &Body, s: &Site, wk: f64) -> Soil {
    let (chem, i, tg) = (&w.chem, s.i, s.tg);
    let (ns, g) = (chem.ns(), d.moves.amounts(w, Node::ground(i)));
    let liquid = (0..ns).filter(|&k| chem.phase_in(k, tg, &g) == Phase::Liquid).fold(0u64, |m, k| m | 1 << k);
    let vol: f64 = ones(liquid).map(|k| g(k) * chem.molar_volume(k, Phase::Liquid)).sum();
    let standing = if s.stands && s.depth == 0.0 { 0.0 } else { vol / w.mesh.cells().area()[i] };
    let wet = standing / (standing + w.cfg.phys.wet_depth);
    let wets = ones(liquid).filter(|&k| chem.subs[k].solubility > 0.0).fold(0u64, |m, k| m | 1 << k);
    let in_ground: f64 = ones(wets).map(&g).sum();
    Soil { wets, in_ground, rate: b.k.liq * wet * wk }
}

/// Its liquids evaporate toward their vapour pressure through its pores (`breath`; for a key liquid as far as it is wet),
/// paid by its heat and what the surface can conduct into it from the ground, no more than its bodies' node holds for it
/// (`heat_bound`; the ground gave it over the air's steps, laws.md 1.4). Above `COOL_T` it evaporates more, as far as
/// that cools it to the gene, its still air carries the vapour (the Lewis relation) and its liquid stays above its safe
/// minimum. Returns what each lost past what flowed in.
fn evaporate(w: &World, d: &mut Domain, b: &mut Body, tmp: &LiveTmp, s: &Site, h: &mut Heat, (soil, reach): (&Soil, f64)) -> ([f64; MAX_SUBSTANCES], bool) {
    let lift = w.lift(s.i);
    let (ns, i, tg, dt, days) = (w.chem.ns(), s.i, s.tg, s.dt, s.days);
    let (g, t0) = (h.g, h.t0);
    // Heat that warming what flowed in to the body's temperature takes from its energy, J (the vapour leaves at it).
    let (mut moved, mut warmed) = (false, 0.0);
    let cpa: f64 = (0..ns).map(|k| tmp.x[k] * w.chem.subs[k].cp).sum();
    let wide = film(&w.chem, &w.cfg, b, h.mass, s.cover).g / cpa.max(1e-9) * dt * s.open * (-s.depth / w.cfg.phys.gas_depth).exp();
    let (lay, size, held) = (Layout::of(&w.chem), b.size(&w.chem), held_for(w, d, b, i));
    let mut lost = [0.0f64; MAX_SUBSTANCES];
    for (k, lost) in lost.iter_mut().enumerate().take(ns) {
        let (chem, cfg) = (&w.chem, &w.cfg);
        if b.store[k] <= 0.0 || chem.phase(k, t0) == Phase::Gas {
            continue;
        }
        let push = (chem.p_sat(k, t0) / P_REF - tmp.x[k]).max(0.0);
        if push <= 0.0 || t0 < b.melt(chem, k) {
            continue;
        }
        let ct = b.heat_cap(chem) + h.c_out;
        let x = g * dt / ct;
        let bx = mean_decay(x);
        let e1 = one_minus_exp(x) * ct * tg + (-x).exp() * b.e + bx * h.heat;
        let need = ((e1 - ct * b.gene(COOL_T)) / bx / chem.subs[k].latent_gas()).max(0.0);
        let spare = if b.wet >> k & 1 == 1 { b.store[k] - b.gene(lay.min + k) * size / chem.subs[k].molar_mass } else { 0.0 };
        let ground = d.moves.amounts(w, Node::ground(i));
        let gk = ground(k);
        let soak_rate = if soil.in_ground > 0.0 && soil.wets >> k & 1 == 1 { soil.rate * chem.subs[k].solubility * gk / soil.in_ground } else { 0.0 };
        let open_m = if b.wet >> k & 1 == 1 { through(b.store[k], b.level(chem, k, 0.0), push * reach, soak_rate * days, cfg.phys.dormancy) } else { push * reach };
        let (cp, latent, flows) = (chem.subs[k].cp, chem.subs[k].latent_gas(), soak_rate > 0.0 && b.wet >> k & 1 == 1);
        let fused = chem.fused_heat(k, tg, ground);
        let (own, conduct) = (h.heat - h.lit + h.took - warmed, g * dt * (tg - chem.t_sat(k, tmp.x[k])).max(0.0));
        let (per, cool) = ((latent, fused - cp * (t0 - tg)), (push * wide).min(need).min(spare));
        let want = open_m.max(cool);
        let m = want.min(heat_bound(own + conduct.min(held), per, b.store[k], flows));
        // What the surface could conduct in beyond that, it asks of the ground until its next update (`survey::Draw`).
        h.asked += (want.min(heat_bound(own + conduct, per, b.store[k], flows)) - m).max(0.0) * latent;
        // What flows through it comes from the ground, which pays its heat and fusion enthalpy.
        let inflow = if flows { (m - b.store[k]).max(0.0).min(gk) } else { 0.0 };
        let pre = d.moves.take_from_ground(w, i, (k, inflow, cp), inflow * (cp * tg + fused), h.gcap).0;
        (b.store[k], b.e, h.heat, h.gcap) = (b.store[k] + pre, b.e + pre * cp * tg, h.heat + pre * fused, h.gcap - pre * cp);
        warmed += pre * cp * (t0 - tg);
        let m = m.min(b.store[k]);
        if m > 0.0 {
            moved = true;
            let q = m * cp * t0;
            (b.store[k], b.e) = (b.store[k] - m, b.e - q);
            d.moves.give(w, Node::air(s.j), 0, 0.0, q);
            d.moves.gas(w, k, m, lift);
            let stop = if cool > open_m { b.gene(COOL_T) } else { w.chem.t_sat(k, tmp.x[k]) };
            (h.heat, h.stop) = (h.heat - m * latent, Some(h.stop.map_or(stop, |x| x.max(stop))));
            *lost = (m - pre).max(0.0);
        }
    }
    (lost, moved)
}

/// The heat its cell's bodies' node holds for body `b` (`Draw::owed`), J: the ground's heat it can reach in an update.
fn held_for(w: &World, d: &Domain, b: &Body, i: usize) -> f64 {
    b.draw.owed.min(d.moves.heat(w, Node::bodies(i))).max(0.0)
}

/// The most of a liquid a body holding `held` mol can evaporate with `pay` J, mol: per mol it takes its latent heat
/// `latent`, J. Past its store the liquid flows in from the ground (if `flows`), each mol bringing the body `brings` J
/// (its fusion enthalpy less its warming to the body): (held + u) latent <= pay + u brings.
fn heat_bound(pay: f64, (latent, brings): (f64, f64), held: f64, flows: bool) -> f64 {
    if !flows || pay <= held * latent {
        return pay.max(0.0) / latent;
    }
    let cost = latent - brings;
    held + if cost > 0.0 { (pay - held * latent) / cost } else { f64::INFINITY }
}

/// Liquids of the ground soak in up to the body's room (`Body::room`) at the rate of `Soil` × solubility, while it loses
/// what it evaporated (`lost`), solved exactly over the time, with the rain it caught (`fall`). The ground pays their
/// heat and fusion enthalpy, or the body soaks less.
#[allow(clippy::too_many_arguments)]
fn soak(w: &World, d: &mut Domain, b: &mut Body, s: &Site, gcap: f64, heat: &mut f64, soil: &Soil, lost: &[f64], fall: f64, mass2: f64) {
    let paces = b.paces(w);
    let (i, tg, days) = (s.i, s.tg, s.days);
    for k in ones(soil.wets).filter(|_| soil.in_ground > 0.0) {
        let (chem, gk) = (&w.chem, d.moves.amounts(w, Node::ground(i))(k));
        let (x, room) = (gk / soil.in_ground, b.room(chem, paces, k, mass2));
        // At rate r per day over `days` with a loss e a day it takes gap f + e days (1 - f / (r days)), f = 1 - exp(-r days).
        let rate = soil.rate * chem.subs[k].solubility;
        let f = one_minus_exp(rate * days);
        let uptake = if rate * days > 1e-9 { (room - lost[k]) * f + lost[k] * (1.0 - f / (rate * days)) } else { 0.0 };
        let want = ((uptake + fall) * x).min(room).min(gk).max(0.0);
        soak_in(w, d, b, (i, tg, gcap), (k, want), heat);
    }
}

/// Up to `want` mol of liquid `k` of ground node `i` go into the body with their heat at `tg` and their fusion enthalpy
/// (into `heat`), which the ground pays, or the body takes less.
fn soak_in(w: &World, d: &mut Domain, b: &mut Body, (i, tg, gcap): (usize, f64, f64), (k, want): (usize, f64), heat: &mut f64) {
    let (cp, fused) = (w.chem.subs[k].cp, w.chem.fused_heat(k, tg, d.moves.amounts(w, Node::ground(i))));
    let per = cp * tg;
    let took = d.moves.take_from_ground(w, i, (k, want, cp), want * (per + fused), gcap).0;
    (b.store[k], b.e, *heat) = (b.store[k] + took, b.e + took * per, *heat + took * fused);
}

/// After growth and children the key liquids soak in again at the same update: new matter raises their level and
/// children took their share, so the activity does not lag one update (and fall with the length of updates).
pub(super) fn soak_again(w: &World, d: &mut Domain, b: &mut Body, s: &Site) {
    let mass = b.mass(&w.chem);
    let soil = soil(w, d, b, s, b.size(&w.chem) / mass);
    let soil = Soil { wets: soil.wets & b.wet, ..soil };
    let (gcap, mut heat) = (d.moves.heat_cap_ground(w, s.i), 0.0);
    soak(w, d, b, s, gcap, &mut heat, &soil, &[0.0; MAX_SUBSTANCES], 0.0, mass);
    b.e += heat;
}

/// Roots take what is liquid in the ground but does not dissolve itself, for the uptake genes that are on, up to its room,
/// at the rate it soaks liquids (without the surface's wetness); the ground pays their heat, or the body takes less.
fn roots(w: &World, d: &mut Domain, b: &mut Body, s: &Site, h: &mut Heat, mass2: f64, wk: f64) {
    let paces = b.paces(w);
    let (i, tg) = (s.i, s.tg);
    let soak = one_minus_exp(b.k.liq * s.days) * wk;
    for x in 0..b.k.upt.len() {
        let (k, u) = b.k.upt[x];
        let (chem, ground) = (&w.chem, d.moves.amounts(w, Node::ground(i)));
        let gk = ground(k);
        if u <= 0.0 || chem.subs[k].solubility > 0.0 || chem.phase_in(k, tg, ground) != Phase::Liquid || gk <= 0.0 {
            continue;
        }
        let want = (b.room(chem, paces, k, mass2) * soak).min(gk);
        soak_in(w, d, b, (i, tg, h.gcap), (k, want), &mut h.heat);
    }
}

/// Heat to the surface: its temperature relaxes toward the ground's plus the warming of the heat released. The ground
/// pays the difference, first from what its bodies' node holds for it (`Draw::owed`); if it cannot, the body ends
/// colder; what it draws until its next update follows from this (`survey::Draw`). The gas put out leaves at the end
/// with its share of the heat.
fn relax_heat(w: &World, d: &mut Domain, b: &mut Body, s: &Site, h: &Heat) {
    let (i, j, tg, dt, heat) = (s.i, s.j, s.tg, s.dt, h.heat);
    let (_, c) = b.mass_cap(&w.chem);
    if c <= 0.0 {
        return;
    }
    let (released, ct, e_start) = (heat.max(-h.g * dt * tg), c + h.c_out, b.e + heat - heat.max(-h.g * dt * tg));
    let relax = |g: f64| ct * (tg + released / dt / g + (e_start / ct - tg - released / dt / g) * (-g * dt / ct).exp());
    let (chem, cfg) = (&w.chem, &w.cfg);
    let e0 = relax(h.g);
    let g = if e0 > ct * h.t0 { surface(cfg, b, h.mass, &film(chem, cfg, b, h.mass, s.cover), s.walls, e0 / ct, tg) } else { h.g };
    let e1 = relax(g);
    let give = e1 - b.e - heat + h.lit - h.took;
    let from = give.min(held_for(w, d, b, i)).max(0.0);
    d.moves.take(w, Node::bodies(i), 0, 0.0, from);
    let drawn = from + d.moves.take_from_ground(w, i, (0, 0.0, 0.0), give - from, h.gcap).1;
    b.draw = crate::survey::Draw::after((drawn + h.asked) / dt, g, tg, h.stop);
    let e1 = if drawn == give { e1 } else { e1 - (give - drawn) };
    let out = e1 * h.c_out / ct;
    b.e = e1 - out;
    d.moves.give(w, Node::air(j), 0, 0.0, out);
}
