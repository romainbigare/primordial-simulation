//! Flow and Change on a body over the time since its last update (`physics`): the light it caught, the gas it exchanged
//! and its catalysed reactions, the liquids it lost and soaked up, and its heat to the ground.

use super::exchange::{breath, film, surface, through};
use super::live::{LiveTmp, Site};
use super::pay::breathes;
use super::{light_eater, Body, Layout, COOL_T, DENATURE};
use crate::change::Reactor;
use crate::chem::Phase;
use crate::ground::{draw, heat_above_floor, share, take_paid};
use crate::math::{mean_decay, one_minus_exp};
use crate::survey::intercept;
use crate::units::{MAX_SUBSTANCES, P_REF};
use crate::{Diag, World};

/// The heat of an update as its stages go.
struct Heat {
    /// The body's mass, kg, its temperature, K, and its conductance to the surface it stands on, W/K (`surface`), at the
    /// start of the physics
    mass: f64,
    t0: f64,
    g: f64,
    /// The heat capacity of its ground node, J/K, as of the start of the physics
    gcap: f64,
    /// Heat released beside the heat its stores hold at their temperature, J
    heat: f64,
    /// Heat capacity of the gas its reactions put out, J/K: it stays with the body's heat until it leaves at the end
    c_out: f64,
    /// Light it absorbed, J, and the light set aside for it that it took (`World::light_pool`), J
    lit: f64,
    took: f64,
    /// It lives on light (for the tallies of the land's liquid)
    plant: bool,
}

/// What the ground of a cell offers to soak up: its liquids that dissolve (one bit per substance), their sum, mol, and
/// the rate a body soaks them at, per day (by solubility).
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

/// The physics of a body of `mass` kg over its update, with the heat `upkeep_heat` its upkeep released. Returns the light
/// it caught per kg per day, J.
#[allow(clippy::too_many_arguments)]
pub(super) fn physics(w: &mut World, slot: usize, b: &mut Body, rc: &mut Reactor, tmp: &mut LiveTmp, s: &Site, mass: f64, upkeep_heat: f64) -> f64 {
    let keep = w.light_keep.get(slot, b.gen) as f64;
    // (The ground's capacity is needed by the first liquid it soaks or loses; it is the same when taken first.)
    let gcap = crate::ground::cap_ground(&w.chem, &w.grid, &w.cfg, &w.ground, &w.bed, s.i);
    let (chem, cfg) = (&w.chem, &w.cfg);
    let plant = light_eater(chem, &b.genes);
    let fall = rain_caught(w, slot, b, s);
    // It soaks through the mass that works; its light-capture area is all members' (`visible`): of its body, by mass^0.75.
    let wk = b.size(chem) / mass;
    let leaf = b.equiv(&cfg.phys, b.built(chem)) * b.k.leaf;
    // Its conductance to the surface, at its temperature at the start of the update (and at the end, if it ends warmer).
    let c0 = b.heat_cap(chem);
    let t0 = b.e / c0;
    let g = surface(chem, cfg, b, mass, s.cover, s.walls, t0, s.tg);
    let (caught, lit) = catch_light(w, b, s, leaf);
    let mut h = Heat { mass, t0, g, gcap, heat: 0.0, c_out: 0.0, lit, took: 0.0, plant };
    let reach = gas_pool(w, b, s, mass, tmp);
    let used = react(w, b, rc, tmp, s, &mut h, c0, upkeep_heat);
    settle_gas(w, b, tmp, s, &mut h);
    h.took = store_light(w, slot, b, s, keep, caught, used);
    let soil = soil(w, b, s, wk);
    let mass1 = b.mass(&w.chem);
    let (lost, moved) = evaporate(w, b, tmp, s, &mut h, &soil, reach);
    let mass2 = if moved { b.mass(&w.chem) } else { mass1 };
    soak(w, b, s, &mut h, &soil, &lost, fall, mass2);
    roots(w, b, s, &h, mass2, wk);
    relax_heat(w, slot, b, s, &h);
    lit / mass / s.days
}

/// The rain that fell on its capture area, mol: the cell's bodies intercept 1 - exp(-x) of the rain, x their capture
/// area per ground area, shared out by height as the light is (`survey::shade`), at the cell's rain rate of the year.
fn rain_caught(w: &World, slot: usize, b: &Body, s: &Site) -> f64 {
    let (i, World { rain, life, canopy, .. }) = (s.i, w);
    rain.get(i).map_or(0.0, |&r| r as f64) * b.vis[1] as f64 * b.n() * s.dt * s.open * life.shade.get(slot).map_or(1.0, |&f| f) * intercept(canopy.get(i).map_or(0.0, |&c| c as f64))
}

/// Light caught since the last update through the openings of walls around it, less what it reflects, and what of it
/// liquid over the body leaves it, J. Its energy already warmed the surface as it fell (`flow::air::radiation`), but for
/// the share set aside for what light reactions store: so up to `MAX_INTERVAL` days of sunlight neither reach the ground
/// in one go nor are drawn back from it in one go.
fn catch_light(w: &World, b: &mut Body, s: &Site, leaf: f64) -> (f64, f64) {
    let refl = (b.vis[3] + b.vis[4] + b.vis[5]) as f64 / 3.0;
    let caught = leaf * (w.light[s.i] - b.mark) * (1.0 - refl) * s.open;
    b.mark = w.light[s.i];
    let reach = (-s.depth / w.cfg.phys.light_depth).exp();
    (caught, caught * reach)
}

/// The air the body exchanges gas with in this time, mol (`breath`), from its air node, at the air's shares; each gas
/// at most half of what the node holds. Returns that air.
fn gas_pool(w: &World, b: &Body, s: &Site, mass: f64, tmp: &mut LiveTmp) -> f64 {
    let (chem, air, na, j) = (&w.chem, &w.air, w.grid.na(), s.j);
    let tot: f64 = (0..chem.ns()).map(|k| air[k * na + j]).sum();
    let reach = breath(chem, &w.cfg, b, mass, s.cover, s.days, s.depth);
    tmp.x.clear();
    tmp.pool.clear();
    for k in 0..chem.ns() {
        let (a, x) = (air[k * na + j], air[k * na + j] / tot);
        tmp.x.push(x);
        tmp.pool.push((x * reach).min(0.5 * a));
    }
    tmp.pool0.clone_from(&tmp.pool);
    reach
}

/// Catalysed reactions, at the body's temperature held by the surface it stands on. The surface holds it as a store of
/// the heat it can conduct into the body over the time (`g` dt `tg`, at the body's temperature): a body well bound to it
/// reacts at a fixed temperature, a big one under cover warms with what its reactions release and cools with what they
/// take, and they cannot take more heat than the body and that store hold. The catalysts work as far as the body is wet
/// (`Body::activity`) and each is slowed by its products (`Body::inhibited`); those that breathe ran on demand (upkeep,
/// actions) and go here at the open world's rate; above the `DENATURE` gene they lose their shape. Their products take
/// their heat at the body's temperature from what they release. Returns the light the reactions used, J.
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
    let used = rc.react(chem, &mut b.store, Some(&mut tmp.pool), &mut e, fixed, Some(den), h.lit, dt);
    let c1 = b.heat_cap(chem);
    h.heat = e - t0 * (fixed + c1) + h.lit - used + upkeep_heat;
    b.e = t0 * c1;
    used
}

/// The gas its reactions took from the air brings its heat (at the air's temperature) and gives up its fusion and
/// vaporisation enthalpy. Gas they put out takes that enthalpy, as far as half of what the body's heat, the heat
/// released and the surface's store hold pays for it (the rest stays in the body), and leaves with its heat at the
/// body's last temperature (`Heat::c_out`).
fn settle_gas(w: &mut World, b: &mut Body, tmp: &LiveTmp, s: &Site, h: &mut Heat) {
    let World { chem, grid, air, e_air, .. } = w;
    let (ns, na, j) = (chem.ns(), grid.na(), s.j);
    let ta = e_air[j] / (0..ns).map(|k| air[k * na + j] * chem.subs[k].cp).sum::<f64>().max(1.0);
    let (mut heat, mut pay, mut q_in, mut c_out) = (h.heat, 0.0, 0.0, 0.0);
    for k in 0..ns {
        let d = tmp.pool[k] - tmp.pool0[k];
        (pay, heat, q_in) = (pay + d.max(0.0) * chem.subs[k].latent_gas(), heat - d.min(0.0) * chem.subs[k].latent_gas(), q_in - d.min(0.0) * chem.subs[k].cp * ta);
    }
    (b.e, e_air[j]) = (b.e + q_in, e_air[j] - q_in);
    let paid = share(0.5 * (b.e + heat + h.g * s.dt * s.tg).max(0.0), pay);
    for k in 0..ns {
        let d = tmp.pool[k] - tmp.pool0[k];
        let sent = if d > 0.0 { d * paid } else { d };
        (b.store[k], air[k * na + j]) = (b.store[k] + d - sent, air[k * na + j] + sent);
        (heat, c_out) = (heat - sent.max(0.0) * chem.subs[k].latent_gas(), c_out + sent.max(0.0) * chem.subs[k].cp);
    }
    (h.heat, h.c_out) = (heat, c_out);
}

/// What its light reactions stored (`LIGHT_YIELD` of the light they `used`) is paid by the light set aside for it, as
/// far as its share goes (the share it stored last time, `keep`, of the light it `caught`); the difference is settled
/// with the ground (`relax_heat`). It keeps the share it stored now. Returns the light set aside it took, J.
fn store_light(w: &mut World, slot: usize, b: &Body, s: &Site, keep: f64, caught: f64, used: f64) -> f64 {
    let stored = crate::change::LIGHT_YIELD * used;
    let took = (keep * caught).min(w.light_pool[s.i]).max(0.0);
    w.light_pool[s.i] -= took;
    if caught > 0.0 {
        w.light_keep.set(slot, b.gen, (stored / caught) as f32);
    }
    took
}

/// The liquids of the cell's ground a body soaks up, and its rate (`Soil`). The liquid in the pores of the ground is
/// soaked up like the liquid that stands: whatever the cell holds of it counts for the wetness, but a body afloat on
/// standing liquid reaches the ground through none.
fn soil(w: &World, b: &Body, s: &Site, wk: f64) -> Soil {
    let (chem, ground, n, i, tg) = (&w.chem, &w.ground, w.grid.n(), s.i, s.tg);
    let ns = chem.ns();
    let standing = if s.stands && s.depth == 0.0 {
        0.0
    } else {
        (0..ns).filter(|&k| chem.phase_in(k, tg, |j| ground[j * n + i]) == Phase::Liquid).map(|k| ground[k * n + i] * chem.molar_volume(k, Phase::Liquid)).sum::<f64>() / w.grid.area()
    };
    let wet = standing / (standing + w.cfg.phys.wet_depth);
    let wets = (0..ns).filter(|&k| chem.phase_in(k, tg, |j| ground[j * n + i]) == Phase::Liquid && chem.subs[k].solubility > 0.0).fold(0u64, |m, k| m | 1 << k);
    let in_ground: f64 = (0..ns).filter(|&k| wets >> k & 1 == 1).map(|k| ground[k * n + i]).sum();
    Soil { wets, in_ground, rate: b.k.liq * wet * wk }
}

/// Liquids the body holds evaporate toward their vapour pressure at its temperature, with their latent heat, through its
/// pores (`breath`), which for the vapour open as far as it is wet (its activity: a dry body closes them as it dries,
/// `closing`; not for its gases, whose reactions already go with its activity). The latent heat is paid by the body's own
/// heat released (not the light: the ground has it), and by what the surface can conduct into a body that evaporating
/// cools no lower than the vapour's condensing point, once for all the liquids (what one spent is owed by the next).
/// Where its update would leave it warmer than its `COOL_T` gene, it opens its surface (pores, sweat, panting) as far as
/// that holds it there: the latent heat that brings the end of the heat step (`relax_heat`, linear in the heat) to the
/// gene, no further than its still air carries vapour away (`film` over the air's molar heat capacity, the Lewis
/// relation), and spending no liquid it keeps below its safe minimum. What flows through a body that soaks past what it
/// holds comes from the ground as it goes (`through`). Returns what each liquid lost past that, mol, and whether any left.
fn evaporate(w: &mut World, b: &mut Body, tmp: &LiveTmp, s: &Site, h: &mut Heat, soil: &Soil, reach: f64) -> ([f64; MAX_SUBSTANCES], bool) {
    let World { cfg, chem, grid, ground, e_ground, air, e_air, ledger, diag, .. } = w;
    let (ns, n, na, i, j, tg, dt, days) = (chem.ns(), grid.n(), grid.na(), s.i, s.j, s.tg, s.dt, s.days);
    let (g, t0) = (h.g, h.t0);
    let mut moved = false;
    let cpa: f64 = (0..ns).map(|k| tmp.x[k] * chem.subs[k].cp).sum();
    let wide = film(chem, cfg, b, h.mass, s.cover).0 / cpa.max(1e-9) * dt * s.open * (-s.depth / cfg.phys.gas_depth).exp();
    let (lay, size) = (Layout::of(chem), b.size(chem));
    let mut lost = [0.0f64; MAX_SUBSTANCES];
    for k in 0..ns {
        let push = (chem.p_sat(k, t0) / P_REF - tmp.x[k]).max(0.0);
        if push <= 0.0 || b.store[k] <= 0.0 || chem.phase(k, t0) == Phase::Gas || t0 < b.melt(chem, k) {
            continue;
        }
        let (own, room) = (h.heat - h.lit + h.took, heat_above_floor(e_ground[i], h.gcap));
        let heat_paid = (own + (g * dt * (tg - chem.t_sat(k, tmp.x[k])).max(0.0)).min(room)).max(0.0);
        let (ct, x) = (b.heat_cap(chem) + h.c_out, g * dt / (b.heat_cap(chem) + h.c_out));
        let bx = mean_decay(x);
        let e1 = one_minus_exp(x) * ct * tg + (-x).exp() * b.e + bx * h.heat;
        let need = ((e1 - ct * b.gene(COOL_T)) / bx / chem.subs[k].latent_gas()).max(0.0);
        let spare = if b.wet >> k & 1 == 1 { b.store[k] - b.gene(lay.min + k) * size / chem.subs[k].molar_mass } else { 0.0 };
        let soak_rate = if soil.in_ground > 0.0 && soil.liquid(k) { soil.rate * chem.subs[k].solubility * ground[k * n + i] / soil.in_ground } else { 0.0 };
        let open_m = if b.wet >> k & 1 == 1 { through(b.store[k], b.level(chem, k, 0.0), push * reach, soak_rate * days, cfg.phys.dormancy) } else { push * reach };
        let m = open_m.max((push * wide).min(need).min(spare)).min(heat_paid / chem.subs[k].latent_gas());
        // What flows through it comes from the ground, which pays its heat and fusion enthalpy as for what it soaks.
        let inflow = if soak_rate > 0.0 && b.wet >> k & 1 == 1 { (m - b.store[k]).max(0.0).min(0.5 * ground[k * n + i]) } else { 0.0 };
        let (fused, cp) = (chem.fused_heat(k, tg, |j| ground[j * n + i]), chem.subs[k].cp);
        let pre = take_paid(&mut e_ground[i], ledger, h.gcap, inflow, cp, inflow * (cp * tg + fused)).0;
        (b.store[k], ground[k * n + i], b.e, h.heat) = (b.store[k] + pre, ground[k * n + i] - pre, b.e + pre * chem.subs[k].cp * tg, h.heat + pre * fused);
        let m = m.min(b.store[k]);
        diag.liquid_land[Diag::LAND_PLANTS_TOOK] += if h.plant { pre * chem.molar_volume(k, Phase::Liquid) } else { 0.0 };
        if m > 0.0 {
            moved = true;
            let q = m * chem.subs[k].cp * t0;
            (air[k * na + j], b.store[k], e_air[j], b.e) = (air[k * na + j] + m, b.store[k] - m, e_air[j] + q, b.e - q);
            h.heat -= m * chem.subs[k].latent_gas();
            lost[k] = (m - pre).max(0.0);
            diag.liquid_land[if h.plant { Diag::LAND_PLANTS } else { Diag::LAND_OTHERS }] += m * chem.molar_volume(k, Phase::Liquid);
        }
    }
    (lost, moved)
}

/// Liquids of the ground soak in by solubility, faster where the ground is wet, up to the level the body keeps, through
/// the mass that works: a flow along its contact with the ground, driven by how far the body is below its level, at the
/// rate `LIQ` x wetness x the working share x solubility per day, while it loses what it evaporated (`lost`) at a steady
/// rate, solved exactly over the time: it takes back what it lost as far as the soil holds and the contact allows. With
/// them it takes the rain its capture area caught (`fall`), which fell into the cell's liquid in this time. The ground
/// pays the heat that goes with the liquid and its fusion enthalpy; if it cannot pay it all, the body soaks up less.
#[allow(clippy::too_many_arguments)]
fn soak(w: &mut World, b: &mut Body, s: &Site, h: &mut Heat, soil: &Soil, lost: &[f64], fall: f64, mass2: f64) {
    let World { chem, grid, ground, e_ground, ledger, diag, .. } = w;
    let (n, i, tg, days) = (grid.n(), s.i, s.tg, s.days);
    for k in (0..chem.ns()).filter(|&k| soil.in_ground > 0.0 && soil.liquid(k)) {
        let (x, room) = (ground[k * n + i] / soil.in_ground, b.level(chem, k, mass2) - b.store[k]);
        // (Rate r per day over `days`, the loss e a day: it takes (gap at the start) f + e days (1 - f / (r days)),
        // f = 1 - exp(-r days); all the gap and the loss when r days is large.)
        let rate = soil.rate * chem.subs[k].solubility;
        let f = one_minus_exp(rate * days);
        let uptake = if rate * days > 1e-9 { (room - lost[k]) * f + lost[k] * (1.0 - f / (rate * days)) } else { 0.0 };
        let want = ((uptake + fall) * x).min(room).min(0.5 * ground[k * n + i]).max(0.0);
        let (per, fused) = (chem.subs[k].cp * tg, chem.fused_heat(k, tg, |j| ground[j * n + i]));
        let d = take_paid(&mut e_ground[i], ledger, h.gcap, want, chem.subs[k].cp, want * (per + fused)).0;
        b.store[k] += d;
        ground[k * n + i] -= d;
        b.e += d * per;
        h.heat += d * fused;
        diag.liquid_land[Diag::LAND_PLANTS_TOOK] += if h.plant { d * chem.molar_volume(k, Phase::Liquid) } else { 0.0 };
    }
}

/// Roots take what is dissolved in the soil: what is liquid at the cell's warmth but does not dissolve itself (the
/// liquids that do are soaked above), that its uptake genes are on for, from the loose ground of its cell, up to the room
/// its level leaves, at the rate it soaks liquids (without the wetness of the surface: soil holds moisture the surface
/// does not show), through the mass that works (`wk`), and no more than half of what there is (the bodies of a cell share it).
fn roots(w: &mut World, b: &mut Body, s: &Site, h: &Heat, mass2: f64, wk: f64) {
    let World { chem, grid, ground, e_ground, ledger, .. } = w;
    let (n, i, tg) = (grid.n(), s.i, s.tg);
    let soak = one_minus_exp(b.k.liq * s.days) * wk;
    for &(k, u) in &b.k.upt {
        if u <= 0.0 || chem.subs[k].solubility > 0.0 || chem.phase_in(k, tg, |j| ground[j * n + i]) != Phase::Liquid || ground[k * n + i] <= 0.0 {
            continue;
        }
        let want = ((b.level(chem, k, mass2) - b.store[k]).max(0.0) * soak).min(0.5 * ground[k * n + i]);
        let d = take_paid(&mut e_ground[i], ledger, h.gcap, want, chem.subs[k].cp, want * chem.subs[k].cp * tg).0;
        b.store[k] += d;
        ground[k * n + i] -= d;
        b.e += d * chem.subs[k].cp * tg;
    }
}

/// Heat to the surface the body stands on (which radiates it and warms the air): its temperature relaxes toward the
/// ground's plus the warming of the heat released, over the time. Heat taken beyond what the surface can conduct into it
/// (down to 0 K) over the time is taken from its own heat at once. The gas its reactions put out goes the same way until
/// it leaves at the end, with its share of the heat. The ground pays the difference (the light is already in it, but for
/// what was set aside and taken), first from what the cell held aside for it since its last update (`survey`); if it
/// cannot, the body ends colder.
fn relax_heat(w: &mut World, slot: usize, b: &mut Body, s: &Site, h: &Heat) {
    let World { cfg, chem, e_ground, e_air, ledger, heat_pool, heat_keep, .. } = w;
    let (i, j, tg, dt, g, heat) = (s.i, s.j, s.tg, s.dt, h.g, h.heat);
    let (_, c) = b.mass_cap(chem);
    if c <= 0.0 {
        return;
    }
    let (released, ct, e_start) = (heat.max(-g * dt * tg), c + h.c_out, b.e + heat - heat.max(-g * dt * tg));
    let relax = |g: f64| ct * (tg + released / dt / g + (e_start / ct - tg - released / dt / g) * (-g * dt / ct).exp());
    let e1 = relax(g);
    let e1 = if e1 > ct * h.t0 { relax(surface(chem, cfg, b, h.mass, s.cover, s.walls, e1 / ct, tg)) } else { e1 };
    let give = e1 - b.e - heat + h.lit - h.took;
    let set = heat_keep.get(slot, b.gen) as f64 * dt;
    let pool = heat_pool.get(i).copied().unwrap_or(0.0);
    let from = if give * set > 0.0 && give * pool > 0.0 { give.signum() * give.abs().min(set.abs()).min(pool.abs()) } else { 0.0 };
    if from != 0.0 {
        heat_pool[i] -= from;
    }
    let drawn = from + draw(&mut e_ground[i], ledger, h.gcap, give - from);
    heat_keep.set(slot, b.gen, (drawn / dt) as f32);
    let e1 = if drawn == give { e1 } else { e1 - (give - drawn) };
    let out = e1 * h.c_out / ct;
    (b.e, e_air[j]) = (e1 - out, e_air[j] + out);
}
