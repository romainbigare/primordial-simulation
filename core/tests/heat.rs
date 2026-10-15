//! The cost of living follows temperature, and the heat a body makes stays in it (15 Oct 2026, decided
//! 14 Oct): upkeep goes by the rate law of the body's reactions, exp(-Ea (1/T - 1/T_ref)),
//! T the body's temperature, Ea the barrier of its main burning reaction, T_ref = `Physics::t_cat`
//! where the costs are anchored (`body::upkeep`, `Body::pace`); and the heat its reactions and its
//! work release leaves through its surface by the conductance of its genes, cover and walls, so the
//! body stands above its ground by that heat over that conductance (`body::live`, `body::pay`).

use pcore::body::{Body, Lineage, DISPERSE, LIQ, REPAIR};
use pcore::rng::{Purpose, Rng};
use pcore::{brain, choose, World};

/// A world with a lineage of brains that only rest, and the land cells with no liquid on them.
struct Set {
    w: World,
    lineage: u32,
    rows: Vec<f32>,
    land: Vec<usize>,
}

fn set() -> Set {
    let mut w = worldgen::new_world(1, 16, 16);
    pcore::step(&mut w);
    let land: Vec<usize> = (0..w.grid.n()).filter(|&i| w.life.env[i][2] > 0.0 && w.life.env[i][1] < 1e-3).collect();
    let shape = choose::shape(w.chem.ns(), 12, 6);
    let mut rest = [-8.0; brain::FIXED];
    rest[0] = 8.0;
    let mut base = brain::random(shape, &mut Rng::new(1, 0, 0, Purpose::Body), 0.1, rest, -8.0, [-8.0; brain::VERBS + brain::OVERBS]);
    let rows = base.split_off(shape.base());
    let lineage = w.life.new_lineage(Lineage { shape, seen: 3, sight: 1, base, root: 1 });
    Set { w, lineage, rows, land }
}

impl Set {
    /// A starting eater (30 kg, 10 days between updates) in land cell number `at`, resting, soaking no liquid, repair 0.
    fn eater(&mut self, at: usize) -> Body {
        let g = self.w.life.new_genome(self.rows.clone());
        let mut b = worldgen::life::make(&mut self.w, 1, self.land[at], self.lineage, g).expect("body");
        (b.genes[REPAIR], b.genes[LIQ], b.genes[DISPERSE]) = (0.0, -4.0, 0.0);
        b.k = Default::default();
        b.prepare(&self.w.chem, &self.w.cfg.phys);
        b
    }
}

/// The same body at the temperature `t`.
fn at(w: &World, b: &Body, t: f64) -> Body {
    let mut x = b.clone();
    x.e = x.heat_cap(&w.chem) * t;
    x
}

/// Upkeep at a temperature is its upkeep at `t_cat` times exp(-Ea (1/T - 1/T_ref)), Ea the barrier of the
/// first reaction the body burns with (`Consts::burn`): the same rate law as the reactions that pay it, so
/// about 2.5 times more for each 10 K at 290 K for the solid's 7,760 K. No new number. A body that burns nothing
/// pays the same at every temperature. Wear goes with upkeep, so a cold body ages slower (`body::live`).
#[test]
fn upkeep_follows_the_rate_law_of_the_reactions() {
    let mut s = set();
    let b = s.eater(0);
    let (w, p) = (&s.w, s.w.cfg.phys);
    let (days, mass) = (b.interval() as f64, b.mass(&w.chem));
    let up = |x: &Body| pcore::body::upkeep(w, x, days, mass);
    let ea = w.chem.reactions[b.k.burn[0].0].ea;
    let u0 = up(&at(w, &b, p.t_cat));
    assert!(u0 > 0.0);
    for t in [250.0, 270.0, 280.0, 285.0, 295.0, 300.0, 310.0] {
        let (want, got) = ((-ea * (1.0 / t - 1.0 / p.t_cat)).exp(), up(&at(w, &b, t)) / u0);
        println!("{t} K: upkeep x {got:.4} (the law: {want:.4})");
        assert!((got / want - 1.0).abs() < 1e-9, "{t} K: {got} against {want}");
    }
    // About 2.5 times per 10 K at 290 K (Ea 7,760 K): the lives review found "about twice per 10 K".
    let q10 = up(&at(w, &b, p.t_cat + 10.0)) / u0;
    assert!(q10 > 2.0 && q10 < 3.0, "10 K warmer: x {q10}");
    // A body that burns nothing pays the same at every temperature.
    let mut dull = b.clone();
    dull.k.burn.clear();
    assert!((up(&at(w, &dull, 250.0)) / up(&at(w, &dull, 310.0)) - 1.0).abs() < 1e-12);
}

/// A cold body wears slower than a warm one in the same cell, in the ratio of their upkeep (wear is the energy the
/// upkeep turns over), and the update keeps matter and energy exactly.
#[test]
fn a_cold_body_ages_slower() {
    let mut s = set();
    let (cold, warm) = (s.eater(0), s.eater(0));
    let (t_cold, t_warm) = (270.0, 300.0);
    let (mut cold, mut warm) = (at(&s.w, &cold, t_cold), at(&s.w, &warm, t_warm));
    // (The temperatures are edits.)
    let fix = |w: &mut World, b: &mut Body, t: f64| {
        let e = t * b.heat_cap(&w.chem);
        w.ledger.imposed_energy += e - b.e;
        b.e = e;
    };
    fix(&mut s.w, &mut cold, t_cold);
    fix(&mut s.w, &mut warm, t_warm);
    let w = &mut s.w;
    let (days, mass) = (1.0, cold.mass(&w.chem));
    cold.prepare(&w.chem, &w.cfg.phys);
    warm.prepare(&w.chem, &w.cfg.phys);
    let (uc, uw) = (pcore::body::upkeep(w, &cold, days, mass), pcore::body::upkeep(w, &warm, days, mass));
    let (c, k) = (w.life.add(cold, w.day, 1), w.life.add(warm, w.day, 1));
    let (t0, l0) = (pcore::check::totals(w), w.ledger.clone());
    pcore::body::survey(w);
    w.day += 1;
    pcore::body::day(w);
    assert!(w.life.bodies[c].alive && w.life.bodies[k].alive);
    let (wc, wk) = (w.life.bodies[c].wear, w.life.bodies[k].wear);
    println!("wear after a day: cold (270 K) {wc:.4e}, warm (300 K) {wk:.4e}; upkeep {uc:.4e} and {uw:.4e} J");
    assert!(wc > 0.0 && wk > 10.0 * wc, "the warm body ages {} times as fast", wk / wc);
    assert!((wk / wc / (uw / uc) - 1.0).abs() < 1e-6, "wear {wk:e} / {wc:e} against the upkeep {uw:e} / {uc:e}");
    let (de, dq) = pcore::check::drift(w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}

/// A body of `f` times the stores and heat (an edit), at its temperature.
fn scaled(w: &mut World, b: &Body, f: f64) -> Body {
    let mut x = b.clone();
    for k in 0..w.chem.ns() {
        w.chem.add_elems(k, (f - 1.0) * x.store[k], &mut w.ledger.imposed_elems);
        w.ledger.imposed_energy += (f - 1.0) * x.store[k] * w.chem.subs[k].hf;
        x.store[k] *= f;
    }
    w.ledger.imposed_energy += (f - 1.0) * x.e;
    x.e *= f;
    x.k = Default::default();
    x.prepare(&w.chem, &w.cfg.phys);
    x
}

/// The work a body pays for (`body::pay`: moving, handling, building) leaves heat in it, which leaves through its surface
/// by its conductance (before, all of it went to the ground within the action): spread over the interval to its next update,
/// a cost Q warms it by Q / (g dt), g the conductance of its genes, cover and walls and what it radiates, dt the interval. The
/// heat that does not stay goes to the ground, so matter and energy are kept exactly. (A tenth of the cost is the latent
/// heat of the gas it breathes out.)
#[test]
fn heat_made_by_work_stays_in_the_body_and_leaves_by_its_conductance() {
    let mut s = set();
    let b = s.eater(0);
    let w = &mut s.w;
    let (t0, l0) = (pcore::check::totals(w), w.ledger.clone());
    let (mut b, days) = (b, 10.0);
    let (temp0, mass) = (b.temp(&w.chem), b.mass(&w.chem));
    let cost = 2e6;
    assert!(pcore::body::pay(w, &mut b, cost, days), "it can pay");
    let (temp1, g) = (b.temp(&w.chem), pcore::body::conductance_to_ground(w, &b, mass, temp0));
    let (cap, dt) = (b.heat_cap(&w.chem), b.interval() as f64 * 86_400.0);
    let (rise, kept) = (temp1 - temp0, cap * (temp1 - temp0));
    println!("a cost of {cost:.1e} J warms a body of {mass:.0} kg by {rise:.4} K (g {g:.2} W/K, {:.1} h time constant): the rate law says {:.4} K", cap / g / 3600.0, cost / (g * dt));
    // Its time constant is hours, far under the days to the next update: the warmth is the heat made over the heat lost per K.
    assert!(cap / g < 0.1 * dt, "the body's heat does not relax within the interval");
    assert!(rise > 0.0 && (rise * g * dt / cost) > 0.85 && (rise * g * dt / cost) < 1.0, "it warms by {rise} K, the cost over g dt is {}", cost / (g * dt));
    // What it keeps is a small part of the heat made (the rest went to the ground).
    assert!(kept < 0.01 * cost, "it keeps {kept} J of {cost}");
    let (de, dq) = pcore::check::drift(w, &t0, &l0);
    assert!(de < 1e-12 && dq < 1e-12, "element drift {de:e}, energy drift {dq:e}");
    // The same work again warms it by as much more.
    let mut c = b.clone();
    assert!(pcore::body::pay(w, &mut c, cost, days));
    let again = c.temp(&w.chem) - temp1;
    assert!((again / rise - 1.0).abs() < 0.05, "the second cost warms by {again} K, the first {rise} K");
}

/// A bigger body of the same genes is warmer for the same work per kg: the heat it makes goes with its mass and its surface,
/// which its heat leaves by, with the mass to the power 2/3, so it stands above its ground by (mass)^(1/3) times as much (8 times
/// the body, twice the warmth).
#[test]
fn a_bigger_body_of_the_same_genes_is_warmer() {
    let mut s = set();
    let small = s.eater(0);
    let big = scaled(&mut s.w, &small, 8.0);
    let w = &mut s.w;
    let rise = |w: &mut World, mut b: Body, per_kg: f64| {
        let (t0, mass) = (b.temp(&w.chem), b.mass(&w.chem));
        assert!(pcore::body::pay(w, &mut b, per_kg * mass, 10.0), "it can pay");
        b.temp(&w.chem) - t0
    };
    let (a, c) = (rise(w, small, 1e5), rise(w, big, 1e5));
    println!("warmth over the ground for the same work per kg: {a:.4} K, 8 times the body {c:.4} K");
    assert!(a > 0.0 && (c / a / 2.0 - 1.0).abs() < 0.03, "{a} K and {c} K");
}

/// Days of a world whose ground is held at `temps[k]` K in the cell `cells[k]` (an edit): the flow and change steps when due, then
/// the bodies due.
fn held_days(w: &mut World, cells: &[usize], temps: &[f64], days: usize) {
    for _ in 0..days {
        for (&i, &t) in cells.iter().zip(temps) {
            let e = w.energy_ground(i, t);
            w.ledger.imposed_energy += e - w.e_ground[i];
            w.e_ground[i] = e;
        }
        if (w.day % w.cfg.days_per_month as u64).is_multiple_of(w.cfg.phys_days as u64) {
            let dt = w.cfg.phys_days as f64 * 86_400.0;
            pcore::body::survey(w);
            pcore::flow::step(w, dt);
            pcore::change::step(w, dt);
        }
        w.events.clear();
        pcore::body::day(w);
        w.day += 1;
    }
}

fn held_day(w: &mut World, cells: &[usize], t: f64) {
    held_days(w, cells, &vec![t; cells.len()], 1);
}

/// A resting body in a cell whose ground is held settles at the temperature where the heat its upkeep makes is the heat its
/// surface loses: T = T_ground + heat rate / conductance (the heat step of its update solves the relaxation, whose time constant
/// of hours is far under its interval of days, and the rate law makes the heat it makes depend on T). So the body of
/// kilograms stands above the ground by a degree, and one of 8 times the stores, which makes 8 times the heat through
/// 4 times the surface, by more (not twice: its heat, and so its upkeep, rises with its warmth); and matter and energy are kept exactly.
#[test]
fn a_resting_body_settles_where_the_heat_it_makes_is_the_heat_it_loses() {
    let mut s = set();
    s.w.cfg.phys.c_build = 0.0;
    let mut small = s.eater(0);
    // (In the dark: no light-capture area to absorb the sun, which warms a body too.)
    small.genes[pcore::body::LEAF] = -9.0;
    small.vis = small.visible(&s.w.chem, &s.w.cfg.phys);
    // (And with no liquid, which would evaporate and cool it; what its burning makes is little.)
    let liquid = worldgen::life::roles(&s.w.chem).expect("roles").2;
    let w = &mut s.w;
    w.chem.add_elems(liquid, -small.store[liquid], &mut w.ledger.imposed_elems);
    w.ledger.imposed_energy -= small.store[liquid] * (w.chem.subs[liquid].hf + w.chem.subs[liquid].cp * small.temp(&w.chem));
    small.e -= small.store[liquid] * w.chem.subs[liquid].cp * small.temp(&w.chem);
    small.store[liquid] = 0.0;
    small.wet = 0;
    let lay = pcore::body::Layout::of(&w.chem);
    (small.genes[lay.min + liquid], small.genes[lay.keep + liquid]) = (0.0, 0.0);
    small.k = Default::default();
    small.prepare(&s.w.chem, &s.w.cfg.phys);
    let big = scaled(&mut s.w, &small, 8.0);
    let w = &mut s.w;
    // Two cells of high ground (rain does not flood it) in other air nodes (a body would take the air the other breathes).
    let mut land = s.land.clone();
    land.sort_by(|&a, &b| w.life.env[b][2].total_cmp(&w.life.env[a][2]));
    let air = w.grid.air_of(land[0]);
    let cells = [land[0], *land.iter().find(|&&i| w.grid.air_of(i) != air).expect("another air node")];
    let t_cat = w.cfg.phys.t_cat;
    let (mut small, mut big) = (small, big);
    small.cell = cells[0] as u32;
    big.cell = cells[1] as u32;
    for b in [&mut small, &mut big] {
        let e = t_cat * b.heat_cap(&w.chem);
        w.ledger.imposed_energy += e - b.e;
        b.e = e;
    }
    let slots = [w.life.add(small, w.day, 1), w.life.add(big, w.day, 1)];
    let (t0, l0) = (pcore::check::totals(w), w.ledger.clone());
    for _ in 0..90 {
        held_day(w, &cells, t_cat);
    }
    let mut rises = [0.0; 2];
    for (n, &slot) in slots.iter().enumerate() {
        // The world and the body as its next update finds them, and the body as that update leaves it.
        let next = w.life.bodies[slot].last + w.life.bodies[slot].interval();
        while w.day < next {
            held_day(w, &cells, t_cat);
        }
        let (w0, mut b) = (w.clone(), w.life.bodies[slot].clone());
        held_day(w, &cells, t_cat);
        assert!(w.life.bodies[slot].alive, "it lives");
        let w0 = &w0;
        b.prepare(&w0.chem, &w0.cfg.phys);
        let (i, mass, days) = (b.cell as usize, b.mass(&w0.chem), b.interval() as f64);
        let (tg, chem) = (w0.scratch.temps().0[i], &w0.chem);
        let (ns, na, j) = (chem.ns(), w0.grid.na(), w0.grid.air_of(i));
        // The temperature where heat made = heat lost. The heat made is what its reactions release at that temperature to their
        // products where they go: the gases to the air with their latent heat, the rest held at its temperature (each mol of `k`
        // holds `h`). Its upkeep burns its fuel (`body::pay`), and the open air burns its stores bare at any temperature (since
        // the litter constants of 15 Oct 2026, a sixth of what it makes). The liquid that makes, which it keeps none of, leaves:
        // as vapour, with its latent heat, as far as the air it exchanges takes it up to its vapour pressure, and the rest to the
        // ground as liquid with its heat (`body::shed`).
        let h = |k: usize, t: f64| {
            let (s, gas) = (&chem.subs[k], chem.phase(k, t) == pcore::chem::Phase::Gas);
            s.hf + if gas { s.lf + s.lv } else { s.cp * t }
        };
        let gas: Vec<f64> = chem.air.iter().enumerate().map(|(k, x)| if k == 1 || k == 2 { x * 1e12 } else { 0.0 }).collect();
        let made = |t: f64| {
            let (x, mut w2) = (at(w0, &b, t), w0.clone());
            let mut y = x.clone();
            assert!(pcore::body::pay(&mut w2, &mut y, pcore::body::upkeep(w0, &x, days, mass), days));
            let (mut g, mut rc) = (gas.clone(), pcore::change::Reactor::new(chem));
            rc.react(chem, &mut y.store, Some(&mut g), &mut (t * 1e18), 1e18, None, 0.0, days * 86_400.0);
            let air = |k: usize| w0.air[k * na + j] - w2.air[k * na + j] + gas[k] - g[k];
            let wet = y.store[liquid] - x.store[liquid];
            let x_air = w0.air[liquid * na + j] / (0..ns).map(|k| w0.air[k * na + j]).sum::<f64>();
            let vapour = wet.min((chem.p_sat(liquid, t) / pcore::chem::P_REF - x_air) * b.k.gas * b.open_area(chem, &w0.cfg.phys, mass) * days);
            (0..ns).map(|k| (x.store[k] - y.store[k] + air(k)) * h(k, t)).sum::<f64>() - vapour * (chem.subs[liquid].lf + chem.subs[liquid].lv)
        };
        let lost = |t: f64| (t - tg) * pcore::body::conductance_to_ground(w0, &b, mass, t) * days * 86_400.0;
        let (mut lo, mut hi) = (tg, tg + 3.0);
        for _ in 0..80 {
            let mid = 0.5 * (lo + hi);
            if lost(mid) < made(mid) { lo = mid } else { hi = mid }
        }
        let want = 0.5 * (lo + hi) - tg;
        rises[n] = w.life.bodies[slot].temp(chem) - tg;
        println!("{:.0} kg: {:.3} K above the ground ({tg:.2} K); heat made = heat lost at {want:.3} K", mass, rises[n]);
        // (Within 2%, 0.2% and 0.5% here: what is left is the update's steps, the body's mass at its start against its mean. It was
        // 13% under, held within 15%, after the smooth rate of Law 2 met the rock nutrient, from two errors (each change alone had
        // been within 2%, not measured again here): `body::live` ran its reactions after its upkeep 9 K below the body, its new liquid coming in with no heat (the
        // bare burning gave 4.9e5 J where 1.1e6 is due, 9% of the heat; fixed, 15 Oct 2026), and this count took 0.95 of the
        // heat where the latent heat of the liquid the burning makes takes 8% more.)
        assert!(rises[n] > 0.0 && (rises[n] / want - 1.0).abs() < 0.02,"{} K above the ground, the heat balance says {want} K", rises[n]);
    }
    assert!(rises[1] > 1.5 * rises[0], "the bigger body is warmer: {rises:?}");
    let (de, dq) = pcore::check::drift(w, &t0, &l0);
    assert!(de < 1e-9 && dq < 1e-9, "element drift {de:e}, energy drift {dq:e}");
}

/// The whole tempo of a body goes with its temperature, not only what it pays to live (15 Oct 2026, after A made upkeep alone
/// follow it, which left summer upkeep 2.4 times with the same intake): what it can take per day (`take`, the sweep, gifts)
/// and the `c_build` allowance per day are by `Body::supply`, which goes by the same factor as upkeep, `Body::pace`. Not
/// the light, nor the catalysed reactions (they follow T by their own rate), nor the cost of a metre.
#[test]
fn what_a_body_takes_and_builds_per_day_scales_with_upkeep() {
    let mut s = set();
    let b = s.eater(0);
    let (w, p) = (&s.w, s.w.cfg.phys);
    let (days, mass) = (b.interval() as f64, b.mass(&w.chem));
    let (s0, u0) = (b.supply(&w.chem, &p), pcore::body::upkeep(w, &b, days, mass));
    for t in [250.0, 270.0, 285.0, 300.0, 310.0] {
        let x = at(w, &b, t);
        let (take, build, up) = (p.take * days * x.supply(&w.chem, &p), p.c_build * x.supply(&w.chem, &p) / p.m_ref * days, pcore::body::upkeep(w, &x, days, mass));
        let (take0, build0) = (p.take * days * s0, p.c_build * s0 / p.m_ref * days);
        println!("{t} K: take x {:.4}, build x {:.4}, upkeep x {:.4}", take / take0, build / build0, up / u0);
        assert!((take / take0 / (up / u0) - 1.0).abs() < 1e-9 && (build / build0 / (up / u0) - 1.0).abs() < 1e-9);
    }
}

/// The kg a body takes from the ground at its decision goes by the same factor: two bodies alike, at 280 K and at 300 K
/// (their temperature when they decide), take in the ratio of their paces and upkeep, the sweep and satiety being out of the way.
#[test]
fn a_body_takes_from_the_ground_at_the_pace_of_its_temperature() {
    use pcore::event::EventKind;
    let mut s = set();
    s.w.cfg.phys.sweep = 1e12;
    s.w.cfg.phys.c_build = 0.0;
    // A starting eater whose brain only takes from the ground.
    let shape = choose::shape(s.w.chem.ns(), 12, 6);
    let mut fixed = [-8.0; brain::FIXED];
    fixed[3] = 8.0;
    let mut base = brain::random(shape, &mut Rng::new(1, 0, 0, Purpose::Body), 0.1, fixed, -8.0, [-8.0; brain::VERBS + brain::OVERBS]);
    s.rows = base.split_off(shape.base());
    s.lineage = s.w.life.new_lineage(Lineage { shape, seen: 3, sight: 1, base, root: 1 });
    let rich = worldgen::life::roles(&s.w.chem).expect("roles").1;
    let mut took = vec![];
    let mut ups = vec![];
    let mut paces = vec![];
    for (n, t) in [280.0, 300.0].into_iter().enumerate() {
        let mut b = s.eater(n);
        let w = &mut s.w;
        // Little fuel, so that it has room to take; at the temperature `t`; a kg of the solid per m² (booked) on its cell.
        let out = 0.8 * b.store[rich];
        w.chem.add_elems(rich, -out, &mut w.ledger.imposed_elems);
        w.ledger.imposed_energy -= out * w.chem.subs[rich].hf;
        b.store[rich] -= out;
        let e = t * b.heat_cap(&w.chem);
        w.ledger.imposed_energy += e - b.e;
        b.e = e;
        let (i, tg, nn) = (b.cell as usize, w.t_ground(b.cell as usize), w.grid.n());
        let a = 100.0 * w.grid.area() / w.chem.subs[rich].molar_mass;
        w.ground[rich * nn + i] += a;
        w.chem.add_elems(rich, a, &mut w.ledger.imposed_elems);
        let en = w.energy_ground(i, tg);
        w.ledger.imposed_energy += a * w.chem.subs[rich].hf + en - w.e_ground[i];
        w.e_ground[i] = en;
        let slot = w.life.add(b, w.day, 1);
        pcore::body::survey(w);
        w.day += 1;
        let mut b = std::mem::take(&mut w.life.bodies[slot]);
        b.prepare(&w.chem, &w.cfg.phys);
        let (days, mass) = (b.interval() as f64, b.mass(&w.chem));
        let (up, pace) = (pcore::body::upkeep(w, &b, days, mass), b.pace(&w.chem, &w.cfg.phys, b.temp(&w.chem)));
        w.events.clear();
        pcore::choose::act(w, slot, &mut b, &mut pcore::body::Tmp::default());
        let id = b.id(slot);
        took.push(w.events.iter().filter_map(|e| if let EventKind::Took { by, from: u64::MAX, .. } = e.kind { (by == id).then_some(e.value) } else { None }).sum::<f64>());
        (ups.push(up), paces.push(pace));
        w.life.bodies[slot] = b;
    }
    println!("took {took:?} kg at 280 K and 300 K; pace {paces:?}; upkeep {ups:?}");
    assert!(took[0] > 0.0 && took[1] > 0.0);
    assert!((took[1] / took[0] / (paces[1] / paces[0]) - 1.0).abs() < 0.01, "taken {took:?}, pace {paces:?}");
    assert!((took[1] / took[0] / (ups[1] / ups[0]) - 1.0).abs() < 0.01, "taken {took:?}, upkeep {ups:?}");
}
