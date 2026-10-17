//! A body's update: Flow and Change on it since its last update.

use super::exchange::{breath, cover_of, film, surface, through};
use super::pay::{hold, pay_at, plan, spend, upkeep, breathes};
use super::{light_eater, thin, Body, Layout, AGE, COOL_T, DENATURE, KEEP_T, LACK, PHASE, REPAIR, SPENT};
use crate::change::Reactor;
use crate::chem::Phase;
use crate::math::{mean_decay, one_minus_exp};
use crate::rng::{Purpose, Rng};
use crate::survey::intercept;
use crate::units::{DAY, MAX_SUBSTANCES, P_REF};
use crate::{bond, Diag, World};

/// Flow and Change on one body since its last update. Returns a death cause.

/// Working memory of `live`, kept from body to body; what it left for the body's decision (`choose::act`).
#[derive(Default)]
pub struct LiveTmp {
    pool: Vec<f64>,
    pool0: Vec<f64>,
    x: Vec<f64>,
    /// Light caught at the last update per kg per day, J
    pub lit: f64,
    /// At the last update: what its reactions could release over the time against its upkeep
    /// (counted up to 2); the catalyst factors of the reactions that run on their own
    pub spare: f64,
    cat: Vec<f64>,
    /// The amounts of a share of its members that die (`thin`)
    dead: Vec<f64>,
    /// Mass and heat capacity of the body as `live` left it (valid until its stores change)
    pub mass: f64,
    pub cap: f64,
}

pub(crate) fn live(w: &mut World, slot: usize, b: &mut Body, rc: &mut Reactor, tmp: &mut LiveTmp) -> Option<u8> {
    let i = b.cell as usize;
    // The ground's temperature as of the last flow step (the monthly survey's is too old to feel
    // a cell that turned hot).
    let [tg, depth, _] = w.life.env[i];
    let tg = w.t_ground_last(i).unwrap_or(tg);
    let (stands, depth, walls) = (depth > 0.0, bond::depth(w, b, depth), bond::walls_around(w, b));
    let cover = cover_of(w, b);
    let open = 1.0 - walls.map_or(0.0, |x| x.0);
    let days = w.day.saturating_sub(b.last).max(1) as f64;
    let (mut mass, cap) = b.mass_cap(&w.chem);
    b.last = w.day;
    // With nothing left of a member there is no heat capacity to hold a temperature: it dies.
    if b.empty_at(mass) {
        return Some(LACK);
    }

    // Upkeep, paid by the reactions that breathe, as fast as they go at the body's temperature
    // over the time (at least an interval: a decision takes one); its heat is spread over the
    // time like the rest.
    let (upkeep, span) = (upkeep(w, b, days, mass), days.max(b.interval() as f64));
    tmp.spare = plan(w, b, 2.0 * upkeep, span, mass, cap).1 / upkeep.max(1e-300);
    // In a node, the members whose upkeep could not be paid die.
    let (e0, node) = (b.e, b.n() > 1.0);
    let short = pay_at(w, b, upkeep, span, mass, cap, node) / upkeep.max(1e-300);
    if short > 1e-9 && !node {
        return Some(SPENT);
    }
    // Living wears a body: by the energy its upkeep turns over per kg of the body that is built (the
    // tissue that wears, not the fuel it carries), over what it can turn over in its life
    // (`Physics::e_life`), so a body that lives fast wears fast. An individual dies of age when its
    // wear reaches 1. A node's members are of all ages: this share of them dies now (the death
    // rate of members who each live that long, so the same mean life).
    let worn = upkeep / (b.built(&w.chem).max(1e-9 * mass) * w.cfg.phys.e_life * b.dec(REPAIR));
    // The heat set aside is what the burning released less what its products take to stand at the body's temperature, so the
    // body keeps its temperature as its stores change (as in `pay`, and for its reactions below).
    let kept = e0 / cap.max(1e-9) * b.heat_cap(&w.chem);
    let upkeep = b.e - kept;
    b.e = kept;
    for (f, cause) in [(short, SPENT), (if node { worn.min(1.0) } else { 0.0 }, AGE)] {
        if f > 1e-9 {
            tmp.dead.clear();
            tmp.dead.extend(b.store.iter().map(|a| a * f));
            if !thin(w, b, f, &tmp.dead, cause) {
                b.e += upkeep;
                return Some(cause);
            }
            mass = b.mass(&w.chem);
        }
    }
    b.wear += if node { 0.0 } else { worn };
    if b.wear >= 1.0 {
        b.e += upkeep;
        return Some(AGE);
    }

    // Heat the ground node can give, J (its capacity as of now; it only falls while the body soaks
    // up liquid): found when first needed, but before the body soaks anything up.
    let keep = w.light_keep.get(slot, b.gen) as f64;
    let World { cfg, chem, grid, ground, bed, e_ground, air, e_air, ledger, light, light_pool, light_keep, diag, life, canopy, heat_pool, heat_keep, rain, .. } = w;
    let mut gcap = None;
    macro_rules! cap {
        () => {
            *gcap.get_or_insert_with(|| crate::ground::cap_ground(chem, grid, cfg, ground, bed, i))
        };
    }
    let (ns, n, na, j) = (chem.ns(), grid.n(), grid.na(), grid.air_of(i));
    let (dt, plant) = (days * DAY, light_eater(chem, &b.genes));
    // The rain that fell on its capture area, mol: the cell's bodies intercept 1 - exp(-x) of the rain, x their capture
    // area per ground area, shared out by height as the light is (`shade`), at the cell's rain rate of the year.
    let fall = rain.get(i).map_or(0.0, |&r| r as f64) * b.vis[1] as f64 * b.n() * dt * open * life.shade.get(slot).map_or(1.0, |&f| f) * intercept(canopy.get(i).map_or(0.0, |&c| c as f64));

    // Its conductance to the surface it stands on, W/K (for its reactions, evaporation and the heat below): through its
    // coat, then by contact and radiation from its outer surface (`surface`). Taken at its temperature at the start of
    // the update (and at the end, if it ends warmer).
    let work = b.size(chem);
    let wk = work / mass;
    // Its light-capture area, all members' (`visible`): of its body, by mass^0.75.
    let leaf = b.equiv(&cfg.phys, b.built(chem)) * b.k.leaf;
    let c0 = b.heat_cap(chem);
    let t0 = b.e / c0;
    let g = surface(chem, cfg, b, mass, cover, walls, t0, tg);

    // Light caught since the last update (through the openings of walls around it), less what
    // the body reflects; liquid over the body absorbs its share first. Its energy already
    // warmed the surface as it fell (`flow::air::radiation`), but for the share set aside for
    // what light reactions store: so up to `MAX_INTERVAL` days of sunlight neither reach the
    // ground in one go nor are drawn back from it in one go.
    let refl = (b.vis[3] + b.vis[4] + b.vis[5]) as f64 / 3.0;
    let got = leaf * (light[i] - b.mark) * (1.0 - refl) * open;
    b.mark = light[i];
    let reach = (-depth / cfg.phys.light_depth).exp();
    let lit = got * reach;
    tmp.lit = lit / mass / days;

    // The air the body exchanges gas with in this time (mol), from its air node; liquid over
    // the body slows it.
    let tot: f64 = (0..ns).map(|k| air[k * na + j]).sum();
    let reach_mol = breath(chem, cfg, b, mass, cover, days, depth);
    tmp.x.clear();
    tmp.pool.clear();
    for k in 0..ns {
        let (a, x) = (air[k * na + j], air[k * na + j] / tot);
        tmp.x.push(x);
        tmp.pool.push((x * reach_mol).min(0.5 * a));
    }
    tmp.pool0.clone_from(&tmp.pool);

    // Catalysed reactions, at the body's temperature held by the surface it stands on; their
    // heat is spread over the time below. The surface holds it as a store of the heat it can
    // conduct into the body over the time (`g` dt `tg`, at the body's temperature): a body
    // well bound to it reacts at a fixed temperature, a big one under cover warms with what
    // its reactions release and cools with what they take, and they cannot take more heat than
    // the body and that store hold. Those that breathe ran on demand (upkeep, actions) and go
    // here at the open world's rate. Above the DENATURE gene its catalysts lose their shape.
    let fixed = g * dt * tg / t0.max(1.0);
    let e_start = t0 * (fixed + c0);
    let mut e = e_start;
    // The catalysts work as far as the body is wet (`Body::activity`): factor 1 + (f - 1) a. A dry
    // plant's light reaction slows with the rest; each is slowed by its products (`Body::inhibited`).
    let act = b.activity(chem, &cfg.phys);
    tmp.cat.clear();
    tmp.cat.extend(b.k.cat.iter().enumerate().map(|(r, &f)| Body::catalyst(f, if f > 1.0 { act * b.inhibited(chem, r, mass, t0) } else { act })));
    for &(r, _) in b.k.burn.iter().filter(|p| breathes(chem, p.0, t0)) {
        tmp.cat[r] = 1.0;
    }
    // Gas they take from the air brings its heat (at the air's temperature) and gives up its
    // fusion and vaporisation enthalpy. Gas they put out takes that enthalpy, as far as half of
    // what the body's heat, the heat released and that store hold pays for it (the rest stays
    // in the body), and leaves with its heat at the body's last temperature (`c_out`, below).
    let ta = e_air[j] / (0..ns).map(|k| air[k * na + j] * chem.subs[k].cp).sum::<f64>().max(1.0);
    let den = (&tmp.cat[..], b.gene(DENATURE), cfg.phys.unfold);
    let used = rc.react(chem, &mut b.store, Some(&mut tmp.pool), &mut e, fixed, Some(den), lit, dt);
    // (Their products, too, take their heat at the body's temperature from what they release.)
    let c1 = b.heat_cap(chem);
    let mut heat = e - t0 * (fixed + c1) + lit - used + upkeep;
    b.e = t0 * c1;
    let (mut pay, mut q_in, mut c_out) = (0.0, 0.0, 0.0);
    for k in 0..ns {
        let d = tmp.pool[k] - tmp.pool0[k];
        (pay, heat, q_in) = (pay + d.max(0.0) * chem.subs[k].latent_gas(), heat - d.min(0.0) * chem.subs[k].latent_gas(), q_in - d.min(0.0) * chem.subs[k].cp * ta);
    }
    (b.e, e_air[j]) = (b.e + q_in, e_air[j] - q_in);
    let paid = crate::ground::share(0.5 * (b.e + heat + g * dt * tg).max(0.0), pay);
    for k in 0..ns {
        let d = tmp.pool[k] - tmp.pool0[k];
        let sent = if d > 0.0 { d * paid } else { d };
        (b.store[k], air[k * na + j]) = (b.store[k] + d - sent, air[k * na + j] + sent);
        (heat, c_out) = (heat - sent.max(0.0) * chem.subs[k].latent_gas(), c_out + sent.max(0.0) * chem.subs[k].cp);
    }
    // What its light reactions stored (`LIGHT_YIELD` of the light they used) is paid by the
    // light set aside for it, as far as its share goes (the share it stored last time, of the
    // light it caught); the difference is settled with the ground below. It keeps the share it
    // stored now. The heat released is its own (`own`) but for the light, which is the ground's
    // heat already.
    let stored = crate::change::LIGHT_YIELD * used;
    let took = (keep * got).min(light_pool[i]).max(0.0);
    light_pool[i] -= took;
    if got > 0.0 {
        light_keep.set(slot, b.gen, (stored / got) as f32);
    }

    // Liquids the body holds evaporate toward their vapour pressure at its temperature, with
    // their latent heat, through its pores (`breath`), which for the vapour open as far as it is wet (its activity: a dry
    // body closes them as it dries, `closing`; 16 Oct 2026. Not for its gases: the reactions that take them already go with its activity, and
    // their pool goes into mass action to its order, so it would count twice: dry bodies could not pay). The latent heat
    // is paid by the body's own heat released (not the light: the ground has it), and by what
    // the surface can conduct into a body that evaporating cools no lower than the vapour's
    // condensing point: through the same conductance as the heat below (cover and walls in
    // series), and once for all the liquids (what one spent is owed by the next), so the heat
    // step never needs more than the surface gives and the body never ends below that point.
    // (It catches no vapour: the rain its capture area catches is taken with what it soaks, below.)
    // **It keeps its temperature from above as from below (16 Oct 2026):** where its update would leave it warmer than its
    // `COOL_T` gene, it opens its surface to evaporation (pores, sweat, panting) as far as that holds it there: the latent heat
    // that brings the end of the heat step below to the gene (that step is linear in the heat, `e1 = A + B heat`). It can open
    // no further than its still air carries vapour away (the heat film's conductance over the air's molar heat capacity, the
    // Lewis relation, `film`), against the air's deficit of vapour, and spends no liquid it keeps below its safe minimum.
    // (The liquid in the pores of the ground is soaked up like the liquid that stands: whatever the cell holds of it
    // counts for the wetness, but a body afloat on standing liquid reaches the ground through none. The rate it soaks at,
    // below, is wanted here: what a body that soaks loses through its pores is what flows through it.)
    let standing = if stands && depth == 0.0 {
        0.0
    } else {
        (0..ns).filter(|&k| chem.phase_in(k, tg, |j| ground[j * n + i]) == Phase::Liquid).map(|k| ground[k * n + i] * chem.molar_volume(k, Phase::Liquid)).sum::<f64>() / grid.area()
    };
    let wet = standing / (standing + cfg.phys.wet_depth);
    let wets = (0..ns).filter(|&k| chem.phase_in(k, tg, |j| ground[j * n + i]) == Phase::Liquid && chem.subs[k].solubility > 0.0).fold(0u64, |m, k| m | 1 << k);
    let liq = |k: usize| wets >> k & 1 == 1;
    let in_ground: f64 = (0..ns).filter(|&k| liq(k)).map(|k| ground[k * n + i]).sum();
    if in_ground > 0.0 {
        let _ = cap!();
    }
    let rate = b.k.liq * wet * wk;
    let mass1 = b.mass(chem);
    let mut moved = false;
    let cpa: f64 = (0..ns).map(|k| tmp.x[k] * chem.subs[k].cp).sum();
    let wide = film(chem, cfg, b, mass, cover).0 / cpa.max(1e-9) * dt * open * (-depth / cfg.phys.gas_depth).exp();
    let (lay, size) = (Layout::of(chem), b.size(chem));
    let mut lost = [0.0f64; MAX_SUBSTANCES];
    for k in 0..ns {
        let push = (chem.p_sat(k, t0) / P_REF - tmp.x[k]).max(0.0);
        if push <= 0.0 || b.store[k] <= 0.0 || chem.phase(k, t0) == Phase::Gas || t0 < b.melt(chem, k) {
            continue;
        }
        let (own, room) = (heat - lit + took, crate::ground::heat_above_floor(e_ground[i], cap!()));
        let paid = (own + (g * dt * (tg - chem.t_sat(k, tmp.x[k])).max(0.0)).min(room)).max(0.0);
        let (ct, x) = (b.heat_cap(chem) + c_out, g * dt / (b.heat_cap(chem) + c_out));
        let bx = mean_decay(x);
        let e1 = one_minus_exp(x) * ct * tg + (-x).exp() * b.e + bx * heat;
        let need = ((e1 - ct * b.gene(COOL_T)) / bx / chem.subs[k].latent_gas()).max(0.0);
        let spare = if b.wet >> k & 1 == 1 { b.store[k] - b.gene(lay.min + k) * size / chem.subs[k].molar_mass } else { 0.0 };
        let r = if in_ground > 0.0 && liq(k) { rate * chem.subs[k].solubility * ground[k * n + i] / in_ground } else { 0.0 };
        let open_m = if b.wet >> k & 1 == 1 { through(b.store[k], b.level(chem, k, 0.0), push * reach_mol, r * days, cfg.phys.dormancy) } else { push * reach_mol };
        let m = open_m.max((push * wide).min(need).min(spare)).min(paid / chem.subs[k].latent_gas());
        // What flows through it past what it holds comes from the ground as it goes, as far as the ground has it (the ground pays
        // its heat and fusion enthalpy, as for what it soaks below, which then takes back the rest of what it lost).
        let d = if r > 0.0 && b.wet >> k & 1 == 1 { (m - b.store[k]).max(0.0).min(0.5 * ground[k * n + i]) } else { 0.0 };
        let (f, cp) = (chem.fused_heat(k, tg, |j| ground[j * n + i]), chem.subs[k].cp);
        let pre = crate::ground::take_paid(&mut e_ground[i], ledger, cap!(), d, cp, d * (cp * tg + f)).0;
        (b.store[k], ground[k * n + i], b.e, heat) = (b.store[k] + pre, ground[k * n + i] - pre, b.e + pre * chem.subs[k].cp * tg, heat + pre * f);
        let m = m.min(b.store[k]);
        diag.liquid_land[Diag::LAND_PLANTS_TOOK] += if plant { pre * chem.molar_volume(k, Phase::Liquid) } else { 0.0 };
        if m > 0.0 {
            moved = true;
            let q = m * chem.subs[k].cp * t0;
            (air[k * na + j], b.store[k], e_air[j], b.e) = (air[k * na + j] + m, b.store[k] - m, e_air[j] + q, b.e - q);
            heat -= m * chem.subs[k].latent_gas();
            lost[k] = (m - pre).max(0.0);
            diag.liquid_land[if plant { Diag::LAND_PLANTS } else { Diag::LAND_OTHERS }] += m * chem.molar_volume(k, Phase::Liquid);
        }
    }

    // Liquids standing on the ground soak in by solubility, faster where the ground is wet,
    // up to the level the body keeps, through the mass that works (`wk`). **A flow along its contact with the ground (16 Oct
    // 2026):** driven by how far the body is below its level, at the rate `LIQ` x wetness x `wk` x solubility per day, while
    // it loses what it evaporated in this time at a steady rate, solved exactly over the time (as Law 2's step): from the
    // store it had, it ends at the level less the loss over that rate, so over an update it takes back what it lost as far
    // as the soil holds and the contact allows. Before, it took only that share of the gap at the end. With them it takes the rain its capture area
    // caught since its last update (`fall`): that rain fell into the cell's liquid in this time, so the ground has that
    // much less.
    let mass2 = if moved { b.mass(chem) } else { mass1 };
    for k in (0..ns).filter(|&k| in_ground > 0.0 && liq(k)) {
        let (x, room) = (ground[k * n + i] / in_ground, b.level(chem, k, mass2) - b.store[k]);
        // (Rate r per day over `days`, the loss e a day: what it takes is (gap at the start) f + e days (1 - f / (r days)),
        // f = 1 - exp(-r days); all the gap and the loss when r days is large.)
        let r = rate * chem.subs[k].solubility;
        let f = one_minus_exp(r * days);
        let uptake = if r * days > 1e-9 { (room - lost[k]) * f + lost[k] * (1.0 - f / (r * days)) } else { 0.0 };
        let d = ((uptake + fall) * x).min(room).min(0.5 * ground[k * n + i]).max(0.0);
        // The ground pays the heat that goes with the liquid and its fusion enthalpy; if it
        // cannot pay it all, the body soaks up less.
        let (per, f) = (chem.subs[k].cp * tg, chem.fused_heat(k, tg, |j| ground[j * n + i]));
        let d = crate::ground::take_paid(&mut e_ground[i], ledger, cap!(), d, chem.subs[k].cp, d * (per + f)).0;
        b.store[k] += d;
        ground[k * n + i] -= d;
        b.e += d * per;
        heat += d * f;
        diag.liquid_land[Diag::LAND_PLANTS_TOOK] += if plant { d * chem.molar_volume(k, Phase::Liquid) } else { 0.0 };
    }
    // Roots take what is dissolved in the soil: the salts that live in the liquid of the ground
    // (liquid at the cell's warmth but for the common liquid itself, whose soaking is above) that
    // its uptake genes are on for, from the loose ground of its cell, up to the room its level
    // leaves, at the rate it soaks liquids (without the wetness of the surface: soil holds
    // moisture the surface does not show), through the mass that works, and no more than half of
    // what there is (the bodies of a cell share it).
    let soak = one_minus_exp(b.k.liq * days) * wk;
    for &(k, u) in &b.k.upt {
        if u <= 0.0 || chem.subs[k].solubility > 0.0 || chem.phase_in(k, tg, |j| ground[j * n + i]) != Phase::Liquid || ground[k * n + i] <= 0.0 {
            continue;
        }
        let d = ((b.level(chem, k, mass2) - b.store[k]).max(0.0) * soak).min(0.5 * ground[k * n + i]);
        let d = crate::ground::take_paid(&mut e_ground[i], ledger, cap!(), d, chem.subs[k].cp, d * chem.subs[k].cp * tg).0;
        b.store[k] += d;
        ground[k * n + i] -= d;
        b.e += d * chem.subs[k].cp * tg;
    }

    // Heat to the surface the body stands on (which radiates it and warms the air): T relaxes
    // toward the ground's temperature plus the warming of the heat released, over the time.
    // Heat taken beyond what the surface can conduct into it (down to 0 K) over the time is
    // taken from its own heat at once (its reactions took it at its temperature). The gas its
    // reactions put out (heat capacity `c_out`) goes the same way until it leaves at the end,
    // with its share of the heat.
    let (_, c) = b.mass_cap(chem);
    if c > 0.0 {
        let (r, ct, e0) = (heat.max(-g * dt * tg), c + c_out, b.e + heat - heat.max(-g * dt * tg));
        let relax = |g: f64| ct * (tg + r / dt / g + (e0 / ct - tg - r / dt / g) * (-g * dt / ct).exp());
        let e1 = relax(g);
        let e1 = if e1 > ct * t0 { relax(surface(chem, cfg, b, mass, cover, walls, e1 / ct, tg)) } else { e1 };
        // The ground pays the difference (the light is already in it, but for what was set aside
        // and taken); if it cannot, the body ends colder.
        let give = e1 - b.e - heat + lit - took;
        // What the cell held aside for it since its last update pays first (`survey`), as far as it goes the same way.
        let set = heat_keep.get(slot, b.gen) as f64 * dt;
        let pool = heat_pool.get(i).copied().unwrap_or(0.0);
        let from = if give * set > 0.0 && give * pool > 0.0 { give.signum() * give.abs().min(set.abs()).min(pool.abs()) } else { 0.0 };
        if from != 0.0 {
            heat_pool[i] -= from;
        }
        let got = from + crate::ground::draw(&mut e_ground[i], ledger, cap!(), give - from);
        heat_keep.set(slot, b.gen, (got / dt) as f32);
        let e1 = if got == give { e1 } else { e1 - (give - got) };
        let out = e1 * c_out / ct;
        (b.e, e_air[j]) = (e1 - out, e_air[j] + out);
    }
    // It keeps its temperature at its `KEEP_T` gene: if the update leaves it colder, it burns on demand, as for its upkeep, the
    // heat its surface sheds at that level over the time, so it ends there (the share `hold` of a heat released over the time
    // stays in it), as far as its reactions can release it. Twice, as its products take a part of the heat at its temperature.
    for _ in 0..2 {
        let (m, c) = b.mass_cap(&w.chem);
        let short = c * b.gene(KEEP_T) - b.e;
        if c <= 0.0 || short <= 1e-6 * b.e || spend(w, b, short / hold(w, b, m, b.e / c, dt), span, dt, true) > 0.0 {
            break;
        }
    }

    let (mut mass3, mut c) = b.mass_cap(&w.chem);
    if b.lacking_at(&w.chem, mass3) {
        if !(b.n() > 1.0 && mass3 >= 1e-3 * b.n() && starve(w, b, &mut tmp.dead)) {
            return Some(LACK);
        }
        (mass3, c) = b.mass_cap(&w.chem);
    }
    let chem = &w.chem;
    (tmp.mass, tmp.cap) = (mass3, c);
    let t = b.e / c.max(1e-9);
    // A key liquid that boiled kills the body. One that froze kills the share of it that is free
    // liquid to freeze: the frozen share of its key liquid (all of it, the body being at one
    // temperature) times its activity, as a dry body holds little liquid to freeze. A node loses
    // that share of its members, with their stores (`thin`); an individual dies with that chance.
    let key = |k: usize| b.wet >> k & 1 == 1;
    if (0..ns).any(|k| key(k) && chem.phase(k, t) == Phase::Gas) {
        return Some(PHASE);
    }
    if (0..ns).any(|k| key(k) && t < b.melt(chem, k)) {
        let f = b.activity(chem, &w.cfg.phys);
        if f >= 1.0 - 1e-9 {
            return Some(PHASE);
        }
        if b.n() > 1.0 {
            tmp.dead.clear();
            tmp.dead.extend(b.store.iter().map(|a| a * f));
            if !thin(w, b, f, &tmp.dead, PHASE) {
                return Some(PHASE);
            }
            (tmp.mass, tmp.cap) = b.mass_cap(&w.chem);
        } else if Rng::new(w.seed, w.day, b.id(slot), Purpose::Frost).f64() < f {
            return Some(PHASE);
        }
    }
    // Growth is building body matter: a member makes matter of the energy-rich solid, paid by
    // work, as far as its allowance goes (`copy::grow`). Below the size of maturity (its copy
    // mass) all of it is its own growth; past it a share `ALLOC` goes into the brood, matter held
    // for offspring. Offspring are made whenever the brood covers a child, by physics and not by
    // choice: at its update whatever its brain does (`copy::copy`).
    if crate::copy::grow(w, b, days) {
        (tmp.mass, tmp.cap) = b.mass_cap(&w.chem);
    }
    if b.brood > 0.0 && crate::copy::copy(w, slot, b) {
        (tmp.mass, tmp.cap) = b.mass_cap(&w.chem);
    }
    shed(w, b);
    None
}

/// A node short of key stores: the members poorest in them die (members differ around the
/// mean), so the survivors keep those stores and the dead take their share of the rest. The
/// share that dies is the least that lifts every key store to its safe minimum; false if that
/// is all of them.
fn starve(w: &mut World, b: &mut Body, dead: &mut Vec<f64>) -> bool {
    let (chem, lay) = (&w.chem, Layout::of(&w.chem));
    let x = |k: usize| b.store[k] * chem.subs[k].molar_mass;
    // (Safe minimums are shares of the body; the survivors, a share `live` of the members, have that share of it.)
    let size = b.size(chem);
    let short = |&k: &usize| !b.burns(chem, k) && x(k) < b.gene(lay.min + k) * size;
    let live = (0..chem.ns()).filter(short).map(|k| x(k) / (b.gene(lay.min + k) * size).max(1e-30)).fold(1.0, f64::min);
    let f = 1.0 - live.max(0.0);
    dead.clear();
    dead.extend((0..chem.ns()).map(|k| if short(&k) { 0.0 } else { b.store[k] * f }));
    f < 1.0 && thin(w, b, f, dead, LACK) && !b.lacking(&w.chem)
}

/// A body is not a quarry: of what it holds besides its liquid, matter, gases and the fuel it burns, the minerals
/// (`Chemistry::minerals`) and the solids it builds (`Chemistry::materials`: what it took of one and did not break passes
/// through) go back to the ground past the level it keeps (`Body::level`), and anything else (a solid left by a salt
/// it held breaking down) all of it, with its share of the heat. Digestion (`Body::digest`) and its own burning of matter give it more
/// than it builds with: an eater that kept it would grow without end and, its room full, stop taking
/// matter. A plant takes only up to the level (`choose::wanted`), so for it the minerals shed nothing.
/// The key liquid past its level drains out the same way (15 Oct 2026, review 7 R11): its burning makes it in the body, and
/// before only evaporation took it out, so eaters held 40 to 65% of their liquid past the level (30 to 48% of their mass).
/// It goes to the ground with its heat; what is dissolved in it goes by its own level in this same loop (the minerals are
/// kept by the body's matter, not by its liquid), and gases leave by breath.
fn shed(w: &mut World, b: &mut Body) {
    let (i, n, mass, minerals) = (b.cell as usize, w.grid.n(), b.mass(&w.chem), w.chem.minerals());
    for k in 0..w.chem.ns() {
        let kept = b.wet >> k & 1 == 1 || minerals.iter().any(|m| m.0 == k) || w.chem.is_material(k);
        let over = b.store[k] - if kept { b.level(&w.chem, k, mass) } else { 0.0 };
        if over <= 0.0 || w.chem.gas_at_surface(k) || w.chem.matter() == Some(k) || b.burns(&w.chem, k) {
            continue;
        }
        let q = b.e * over * w.chem.subs[k].cp / b.heat_cap(&w.chem).max(1e-9);
        b.store[k] -= over;
        w.ground[k * n + i] += over;
        b.e -= q;
        w.e_ground[i] += q;
    }
}
