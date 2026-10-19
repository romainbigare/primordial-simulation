//! Flow and Change on a body over the time since its last update: light, gas, reactions, liquids and heat (laws.md
//! 6.3, 6.4).

use super::exchange::{breath, film, surface, through, Film};
use super::live::{LiveTmp, Site};
use super::pay::breathes;
use super::{Body, Layout, COOL_T, DENATURE};
use crate::change::{send_gas, GasMove, Reactor};
use crate::chem::Phase;
use crate::ground::{heat_above_floor, share, Melting};
use crate::math::{mean_decay, one_minus_exp};
use crate::survey::intercept;
use crate::store::Node;
use crate::units::{MAX_SUBSTANCES, P_REF};
use crate::World;

/// The most share of the heat the body has and can get from the surface that pays the enthalpy of the gas it puts out.
const GAS_HEAT_SHARE: f64 = 0.5;

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
}

/// What the ground of a cell offers to soak up: its liquids that dissolve (bits), their sum, mol, and the rate, per day.
struct Soil {
    wets: u64,
    in_ground: f64,
    rate: f64,
}

impl Soil {
    fn liquid(&self, k: usize) -> bool {
        self.wets >> k & 1 == 1
    }
}

/// The physics of a body of `mass` kg at the plateau state `th0` (`Body::thermal`) over its update, with the heat its
/// upkeep released. Returns the light it caught per kg per day, J.
#[allow(clippy::too_many_arguments)]
pub(super) fn physics(w: &mut World, slot: usize, b: &mut Body, rc: &mut Reactor, tmp: &mut LiveTmp, s: &Site, mass: f64, upkeep_heat: f64, th0: Melting) -> f64 {
    let gcap = w.heat_cap_ground(s.i);
    let (chem, cfg) = (&w.chem, &w.cfg);
    let fall = rain_caught(w, slot, b, s);
    // It soaks through the mass that works; its conductance is taken at its temperature at the start.
    let wk = b.size(chem) / mass;
    let leaf = b.leaf_area(chem, &cfg.phys);
    // The stages move sensible heat; liquids that come and go bring and take their fusion enthalpy as heat, until the
    // stores hold theirs again at the end.
    let c0 = b.heat_cap(chem);
    let (t0, fusion0) = (th0.t, b.e - c0 * th0.t);
    // One boundary layer for its heat and its breath until its stores change.
    let f0 = film(chem, cfg, b, mass, s.cover);
    let g = surface(cfg, b, mass, &f0, s.walls, t0, s.tg);
    let (caught, lit) = catch_light(w, b, s, leaf);
    let mut h = Heat { mass, t0, g, gcap, heat: 0.0, c_out: 0.0, lit, took: 0.0 };
    let reach = gas_pool(w, b, s, &f0, tmp);
    let used = react(w, b, rc, tmp, s, &mut h, c0, upkeep_heat);
    settle_gas(w, b, tmp, s, &mut h);
    h.took = store_light(w, b, s, caught, used);
    let soil = soil(w, b, s, wk);
    let mass1 = b.mass(&w.chem);
    let (lost, moved) = evaporate(w, b, tmp, s, &mut h, &soil, reach);
    let mass2 = if moved { b.mass(&w.chem) } else { mass1 };
    soak(w, b, s, h.gcap, &mut h.heat, &soil, &lost, fall, mass2);
    roots(w, b, s, &mut h, mass2, wk);
    let fusion1 = b.fusion_in(&w.chem, &th0);
    h.heat += fusion0 - fusion1;
    relax_heat(w, b, s, &h);
    b.e += fusion1;
    lit / mass / s.days
}

/// The rain that fell on its capture area since its last update, mol: the cell's bodies intercept 1 - exp(-x) of it, x
/// their capture area per ground area, shared by height as light is (`survey::shade`).
fn rain_caught(w: &World, slot: usize, b: &mut Body, s: &Site) -> f64 {
    let (i, World { rained, life, canopy, .. }) = (s.i, w);
    let fell = rained[i] - std::mem::replace(&mut b.rained, rained[i]);
    fell * b.vis[1] as f64 * b.n() * s.open * life.shade.get(slot).map_or(1.0, |&f| f) * intercept(canopy.get(i).map_or(0.0, |&c| c as f64))
}

/// Light caught since the last update through any openings, less what it reflects, and what liquid over it leaves, J.
/// It already warmed the surface as it fell, but for the share set aside for light reactions (`store_light`).
fn catch_light(w: &World, b: &mut Body, s: &Site, leaf: f64) -> (f64, f64) {
    let caught = leaf * (w.light[s.i] - b.mark) * (1.0 - b.reflectance()) * s.open;
    b.mark = w.light[s.i];
    let reach = (-s.depth / w.cfg.phys.light_depth).exp();
    (caught, caught * reach)
}

/// The air the body exchanges gas with in this time (`breath`), at the air's shares, no more than the node holds.
fn gas_pool(w: &World, b: &Body, s: &Site, f: &Film, tmp: &mut LiveTmp) -> f64 {
    let (chem, j) = (&w.chem, s.j);
    let tot: f64 = (0..chem.ns()).map(|k| w.a(k, j)).sum();
    let reach = breath(chem, &w.cfg, b, f, s.days, s.depth);
    tmp.x.clear();
    tmp.pool.clear();
    for k in 0..chem.ns() {
        let a = w.a(k, j);
        let x = if tot > 0.0 { a / tot } else { 0.0 };
        tmp.x.push(x);
        tmp.pool.push((x * reach).min(a));
    }
    tmp.pool0.clone_from(&tmp.pool);
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
/// `GAS_HEAT_SHARE` of the heat pays for it.
fn settle_gas(w: &mut World, b: &mut Body, tmp: &LiveTmp, s: &Site, h: &mut Heat) {
    let (j, lift) = (s.j, w.lift(s.i));
    let gas = GasMove::of(&w.chem, &tmp.pool, &tmp.pool0, w.t_air(j));
    b.e += gas.q_in;
    w.moves.take(&w.chem, Node::air(j), 0, 0.0, gas.q_in);
    let heat = h.heat + gas.back;
    let paid = share(GAS_HEAT_SHARE * (b.e + heat + h.g * s.dt * s.tg).max(0.0), gas.pay);
    send_gas(&tmp.pool, &tmp.pool0, paid, |k, sent, kept| {
        b.store[k] += kept;
        if sent != 0.0 {
            w.moves.gas(&w.chem, k, sent, lift);
        }
    });
    (h.heat, h.c_out) = (heat - paid * gas.pay, paid * gas.c_out);
}

/// What its light reactions stored is paid by the light set aside for it in its bodies' node (by `Body::keep`, as far
/// as the node holds it); it keeps the share it stored now. Returns the light it took, J.
fn store_light(w: &mut World, b: &mut Body, s: &Site, caught: f64, used: f64) -> f64 {
    let stored = w.cfg.phys.light_yield * used;
    let took = (b.keep as f64 * caught).min(w.heat_now(Node::bodies(s.i))).max(0.0);
    w.moves.take(&w.chem, Node::bodies(s.i), 0, 0.0, took);
    if caught > 0.0 {
        b.keep = (stored / caught) as f32;
    }
    took
}

/// The liquids of the cell's ground a body soaks up (`Soil`); a body afloat reaches none.
fn soil(w: &World, b: &Body, s: &Site, wk: f64) -> Soil {
    let (chem, i, tg) = (&w.chem, s.i, s.tg);
    let (ns, g) = (chem.ns(), |k: usize| w.g(k, i));
    let standing = if s.stands && s.depth == 0.0 {
        0.0
    } else {
        (0..ns).filter(|&k| chem.phase_in(k, tg, g) == Phase::Liquid).map(|k| g(k) * chem.molar_volume(k, Phase::Liquid)).sum::<f64>() / w.mesh.cells().area()[i]
    };
    let wet = standing / (standing + w.cfg.phys.wet_depth);
    let wets = (0..ns).filter(|&k| chem.phase_in(k, tg, g) == Phase::Liquid && chem.subs[k].solubility > 0.0).fold(0u64, |m, k| m | 1 << k);
    let in_ground: f64 = (0..ns).filter(|&k| wets >> k & 1 == 1).map(g).sum();
    Soil { wets, in_ground, rate: b.k.liq * wet * wk }
}

/// Its liquids evaporate toward their vapour pressure through its pores (`breath`; for a key liquid as far as it is wet),
/// paid by its heat and what the surface can conduct into it. Above `COOL_T` it evaporates more, as far as that cools it
/// to the gene, its still air carries the vapour (the Lewis relation) and its liquid stays above its safe minimum.
/// Returns what each lost past what flowed in.
fn evaporate(w: &mut World, b: &mut Body, tmp: &LiveTmp, s: &Site, h: &mut Heat, soil: &Soil, reach: f64) -> ([f64; MAX_SUBSTANCES], bool) {
    let lift = w.lift(s.i);
    let (ns, i, tg, dt, days) = (w.chem.ns(), s.i, s.tg, s.dt, s.days);
    let (g, t0) = (h.g, h.t0);
    let mut moved = false;
    let cpa: f64 = (0..ns).map(|k| tmp.x[k] * w.chem.subs[k].cp).sum();
    let wide = film(&w.chem, &w.cfg, b, h.mass, s.cover).g / cpa.max(1e-9) * dt * s.open * (-s.depth / w.cfg.phys.gas_depth).exp();
    let (lay, size) = (Layout::of(&w.chem), b.size(&w.chem));
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
        let (own, room) = (h.heat - h.lit + h.took, heat_above_floor(w.heat_now(Node::ground(i)), h.gcap));
        let heat_paid = (own + (g * dt * (tg - chem.t_sat(k, tmp.x[k])).max(0.0)).min(room)).max(0.0);
        let (ct, x) = (b.heat_cap(chem) + h.c_out, g * dt / (b.heat_cap(chem) + h.c_out));
        let bx = mean_decay(x);
        let e1 = one_minus_exp(x) * ct * tg + (-x).exp() * b.e + bx * h.heat;
        let need = ((e1 - ct * b.gene(COOL_T)) / bx / chem.subs[k].latent_gas()).max(0.0);
        let spare = if b.wet >> k & 1 == 1 { b.store[k] - b.gene(lay.min + k) * size / chem.subs[k].molar_mass } else { 0.0 };
        let gk = w.g(k, i);
        let soak_rate = if soil.in_ground > 0.0 && soil.liquid(k) { soil.rate * chem.subs[k].solubility * gk / soil.in_ground } else { 0.0 };
        let open_m = if b.wet >> k & 1 == 1 { through(b.store[k], b.level(chem, k, 0.0), push * reach, soak_rate * days, cfg.phys.dormancy) } else { push * reach };
        let m = open_m.max((push * wide).min(need).min(spare)).min(heat_paid / chem.subs[k].latent_gas());
        // What flows through it comes from the ground, which pays its heat and fusion enthalpy.
        let inflow = if soak_rate > 0.0 && b.wet >> k & 1 == 1 { (m - b.store[k]).max(0.0).min(gk) } else { 0.0 };
        let (cp, latent) = (chem.subs[k].cp, chem.subs[k].latent_gas());
        let fused = w.chem.fused_heat(k, tg, |j| w.g(j, i));
        let pre = w.take_from_ground(i, (k, inflow, cp), inflow * (cp * tg + fused), h.gcap).0;
        (b.store[k], b.e, h.heat) = (b.store[k] + pre, b.e + pre * cp * tg, h.heat + pre * fused);
        let m = m.min(b.store[k]);
        if m > 0.0 {
            moved = true;
            let q = m * cp * t0;
            (b.store[k], b.e) = (b.store[k] - m, b.e - q);
            w.moves.give(&w.chem, Node::air(s.j), 0, 0.0, q);
            w.moves.gas(&w.chem, k, m, lift);
            h.heat -= m * latent;
            *lost = (m - pre).max(0.0);
        }
    }
    (lost, moved)
}

/// Liquids of the ground soak in up to the body's room (`Body::room`) at the rate of `Soil` × solubility, while it loses
/// what it evaporated (`lost`), solved exactly over the time, with the rain it caught (`fall`). The ground pays their
/// heat and fusion enthalpy, or the body soaks less.
#[allow(clippy::too_many_arguments)]
fn soak(w: &mut World, b: &mut Body, s: &Site, gcap: f64, heat: &mut f64, soil: &Soil, lost: &[f64], fall: f64, mass2: f64) {
    let paces = b.paces(w);
    let (i, tg, days) = (s.i, s.tg, s.days);
    for k in (0..w.chem.ns()).filter(|&k| soil.in_ground > 0.0 && soil.liquid(k)) {
        let (chem, gk) = (&w.chem, w.g(k, i));
        let (x, room) = (gk / soil.in_ground, b.room(chem, paces, k, mass2));
        // At rate r per day over `days` with a loss e a day it takes gap f + e days (1 - f / (r days)), f = 1 - exp(-r days).
        let rate = soil.rate * chem.subs[k].solubility;
        let f = one_minus_exp(rate * days);
        let uptake = if rate * days > 1e-9 { (room - lost[k]) * f + lost[k] * (1.0 - f / (rate * days)) } else { 0.0 };
        let want = ((uptake + fall) * x).min(room).min(gk).max(0.0);
        soak_in(w, b, (i, tg, gcap), (k, want), heat);
    }
}

/// Up to `want` mol of liquid `k` of ground node `i` go into the body with their heat at `tg` and their fusion enthalpy
/// (into `heat`), which the ground pays, or the body takes less. Returns the mol taken.
fn soak_in(w: &mut World, b: &mut Body, (i, tg, gcap): (usize, f64, f64), (k, want): (usize, f64), heat: &mut f64) -> f64 {
    let (cp, fused) = (w.chem.subs[k].cp, w.chem.fused_heat(k, tg, |j| w.g(j, i)));
    let per = cp * tg;
    let d = w.take_from_ground(i, (k, want, cp), want * (per + fused), gcap).0;
    (b.store[k], b.e, *heat) = (b.store[k] + d, b.e + d * per, *heat + d * fused);
    d
}

/// After growth and children the key liquids soak in again at the same update: new matter raises their level and
/// children took their share, so the activity does not lag one update (and fall with the length of updates).
pub(super) fn soak_again(w: &mut World, b: &mut Body, s: &Site) {
    let mass = b.mass(&w.chem);
    let soil = soil(w, b, s, b.size(&w.chem) / mass);
    let soil = Soil { wets: soil.wets & b.wet, ..soil };
    let (gcap, mut heat) = (w.heat_cap_ground(s.i), 0.0);
    soak(w, b, s, gcap, &mut heat, &soil, &[0.0; MAX_SUBSTANCES], 0.0, mass);
    b.e += heat;
}

/// Roots take what is liquid in the ground but does not dissolve itself, for the uptake genes that are on, up to its room,
/// at the rate it soaks liquids (without the surface's wetness); the ground pays their heat, or the body takes less.
fn roots(w: &mut World, b: &mut Body, s: &Site, h: &mut Heat, mass2: f64, wk: f64) {
    let paces = b.paces(w);
    let (i, tg) = (s.i, s.tg);
    let soak = one_minus_exp(b.k.liq * s.days) * wk;
    for x in 0..b.k.upt.len() {
        let (k, u) = b.k.upt[x];
        let (chem, gk) = (&w.chem, w.g(k, i));
        if u <= 0.0 || chem.subs[k].solubility > 0.0 || chem.phase_in(k, tg, |j| w.g(j, i)) != Phase::Liquid || gk <= 0.0 {
            continue;
        }
        let want = (b.room(chem, paces, k, mass2) * soak).min(gk);
        soak_in(w, b, (i, tg, h.gcap), (k, want), &mut h.heat);
    }
}

/// Heat to the surface: its temperature relaxes toward the ground's plus the warming of the heat released. The ground
/// pays the difference, first from what its bodies' node holds for it; if it cannot, the body ends colder. The gas put
/// out leaves at the end with its share of the heat.
fn relax_heat(w: &mut World, b: &mut Body, s: &Site, h: &Heat) {
    let (i, j, tg, dt, g, heat) = (s.i, s.j, s.tg, s.dt, h.g, h.heat);
    let (_, c) = b.mass_cap(&w.chem);
    if c <= 0.0 {
        return;
    }
    let (released, ct, e_start) = (heat.max(-g * dt * tg), c + h.c_out, b.e + heat - heat.max(-g * dt * tg));
    let relax = |g: f64| ct * (tg + released / dt / g + (e_start / ct - tg - released / dt / g) * (-g * dt / ct).exp());
    let e1 = relax(g);
    let (chem, cfg) = (&w.chem, &w.cfg);
    let e1 = if e1 > ct * h.t0 { relax(surface(cfg, b, h.mass, &film(chem, cfg, b, h.mass, s.cover), s.walls, e1 / ct, tg)) } else { e1 };
    let give = e1 - b.e - heat + h.lit - h.took;
    let from = give.min(b.drew as f64 * dt).min(w.heat_now(Node::bodies(i))).max(0.0);
    w.moves.take(&w.chem, Node::bodies(i), 0, 0.0, from);
    let drawn = from + w.take_from_ground(i, (0, 0.0, 0.0), give - from, h.gcap).1;
    b.drew = (drawn.max(0.0) / dt) as f32;
    let e1 = if drawn == give { e1 } else { e1 - (give - drawn) };
    let out = e1 * h.c_out / ct;
    b.e = e1 - out;
    w.moves.give(&w.chem, Node::air(j), 0, 0.0, out);
}
