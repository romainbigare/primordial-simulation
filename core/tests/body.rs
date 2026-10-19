//! A body's own physics: its upkeep and how its tempo follows temperature, its heat, its liquid and dormancy, its size
//! and what it keeps, digestion, light and shade, its age, intake, upkeep and leaf by mass, its roots and enzymes.
//! Damage: `damage.rs`.

use pcore::body::{self, Body, Layout, COPY_MASS, DISPERSE, LIQ, PHASE, REPAIR, STRENGTH, WET};
use pcore::store::{Boundary, Class, Node};
use pcore::units::{DAY, DAY_S, T_FLOOR};
use pcore::{brain, change, World, AIR};
use worldgen::testkit::{add, brood, node_of, open_area, release, dry_land, eater, empty_cell, in_all, rock_outside_matter, children, died, dry_lands, founder, hold_ground, kg, moved, open_air, out, put, put_kg_m2, scale, small_world, start, took_from_ground, warm, Books, Brain, Line, DISSOLVED, FIX_GAS, FIX_OUT, LIQUID, MATTER, OPEN_CAP, OX, RICH, ROCK};

/// The default world's settings (its year: a reaction too slow to matter over it is frozen).
fn cfg() -> pcore::Config {
    pcore::Config::default()
}

/// A world with a lineage of brains that only rest, and the land cells with no liquid on them.
struct Set {
    w: World,
    line: Line,
    land: Vec<usize>,
}

fn set() -> Set {
    let mut w = small_world();
    let land = dry_lands(&w);
    let line = Line::new(&mut w, &Brain::only(brain::REST), 1);
    Set { w, line, land }
}

impl Set {
    /// A starting eater in land cell number `at`, resting, staying where it is, soaking no liquid, repair 0.
    fn eater(&mut self, at: usize) -> Body {
        let mut b = self.line.founder(&mut self.w, 1, self.land[at]);
        (b.genes[REPAIR], b.genes[LIQ], b.genes[DISPERSE]) = (0.0, -4.0, 0.0);
        b.prepare(&self.w.chem, &self.w.cfg.phys);
        b
    }

    /// The same body with its key liquid at the share `x` of the level it keeps, at the same temperature.
    fn dried(&self, b: &Body, x: f64) -> Body {
        let mut d = b.clone();
        let t = d.temp(&self.w.chem);
        d.store[LIQUID] = x * b.level(&self.w.chem, LIQUID, 0.0);
        d.e = d.energy_at(&self.w.chem, t);
        d
    }
}

/// The same body at the temperature `t`.
fn at(w: &World, b: &Body, t: f64) -> Body {
    let mut x = b.clone();
    x.e = x.energy_at(&w.chem, t);
    x
}

/// The tempo of a body follows the rate law of its reactions: its upkeep at a temperature is its upkeep at `t_cat`
/// times exp(-Ea (1/T - 1/T_ref)), Ea the barrier of the first reaction it burns with (`Consts::burn`), and what it
/// can take and build a day (`Body::supply`, by `Body::pace`) goes by the same factor: about 2.5 times for each 10 K at
/// 290 K. A body that burns nothing pays the same at every temperature.
#[test]
fn upkeep_and_supply_follow_the_rate_law_of_the_reactions() {
    let mut s = set();
    let b = s.eater(0);
    let (w, p) = (&s.w, s.w.cfg.phys);
    let (days, mass) = (b.interval(w) as f64, b.mass(&w.chem));
    let up = |x: &Body| pcore::body::upkeep(w, x, days, mass, x.temp(&w.chem));
    let ea = w.chem.reactions[b.k.burn[0].0].ea;
    let (b0, u0) = (at(w, &b, p.t_cat), up(&at(w, &b, p.t_cat)));
    assert!(u0 > 0.0);
    for t in [250.0, 270.0, 280.0, 285.0, 295.0, 300.0, 310.0] {
        let x = at(w, &b, t);
        let (want, got, supply) = ((-ea * (1.0 / t - 1.0 / p.t_cat)).exp(), up(&x) / u0, x.supply(&w.chem, &p, x.temp(&w.chem)) / b0.supply(&w.chem, &p, b0.temp(&w.chem)));
        println!("{t} K: upkeep x {got:.4}, supply x {supply:.4} (the law: {want:.4})");
        assert!((got / want - 1.0).abs() < 1e-9 && (supply / want - 1.0).abs() < 1e-9, "{t} K: upkeep {got}, supply {supply}, the law {want}");
    }
    // About 2.5 times per 10 K at 290 K (Ea 7,760 K).
    let q10 = up(&at(w, &b, p.t_cat + 10.0)) / u0;
    assert!(q10 > 2.0 && q10 < 3.0, "10 K warmer: x {q10}");
    // A body that burns nothing pays the same at every temperature.
    let mut dull = b.clone();
    dull.k.burn.clear();
    assert!((up(&at(w, &dull, 250.0)) / up(&at(w, &dull, 310.0)) - 1.0).abs() < 1e-12);
}

/// The age deaths of a day: in a copy of `w`, node `b` updates once; its wear over the day (the share that died of age
/// is 1 - e^-wear).
fn wear_of_a_day(w: &World, b: Body) -> f64 {
    let mut v = w.clone();
    let k = v.add(b, 1);
    let n = v.life.bodies[k].count;
    pcore::survey::survey(&mut v);
    v.time += DAY_S;
    v.events.clear();
    pcore::survey::day(&mut v);
    assert!(v.life.bodies[k].alive, "the node died");
    -(1.0 - died(&v, pcore::body::AGE) / n).ln()
}

/// A cold body wears slower than a warm one in the same cell, in the ratio of their upkeep (wear is the energy the
/// upkeep turns over): nodes of a thousand members lose that share of them to age in a day.
#[test]
fn a_cold_body_ages_slower() {
    let mut s = set();
    let (t_cold, t_warm) = (270.0, 300.0);
    let (mut cold, mut hot) = (s.eater(0), s.eater(0));
    warm(&mut s.w, &mut cold, t_cold);
    warm(&mut s.w, &mut hot, t_warm);
    let w = &mut s.w;
    let (cold, hot) = (node_of(w, cold, 1000.0), node_of(w, hot, 1000.0));
    let (mut cold, mut hot, days) = (cold, hot, 1.0);
    cold.prepare(&w.chem, &w.cfg.phys);
    hot.prepare(&w.chem, &w.cfg.phys);
    let mass = cold.mass(&w.chem);
    let (uc, uw) = (pcore::body::upkeep(w, &cold, days, mass, cold.temp(&w.chem)), pcore::body::upkeep(w, &hot, days, mass, hot.temp(&w.chem)));
    let (wc, wk) = (wear_of_a_day(w, cold), wear_of_a_day(w, hot));
    println!("wear in a day: cold (270 K) {wc:.4e}, warm (300 K) {wk:.4e}; upkeep {uc:.4e} and {uw:.4e} J");
    assert!(wc > 0.0 && wk > 10.0 * wc, "the warm body ages {} times as fast", wk / wc);
    assert!((wk / wc / (uw / uc) - 1.0).abs() < 1e-6, "wear {wk:e} / {wc:e} against the upkeep {uw:e} / {uc:e}");
}

/// A body of `f` times the stores and heat (an edit), at its temperature, prepared.
fn scaled(w: &mut World, b: &Body, f: f64) -> Body {
    let mut x = b.clone();
    scale(w, &mut x, f);
    x.prepare(&w.chem, &w.cfg.phys);
    x
}

/// The work a body pays for (`body::pay`: moving, handling, building) leaves heat in it, which leaves through its
/// surface by its conductance: spread over the interval to its next update, a cost Q warms it by Q / (g dt), g the
/// conductance of its genes, cover and walls and what it radiates, dt the interval. The heat that does not stay goes to
/// the ground, so matter and energy are kept. (A tenth of the cost is the latent heat of the gas it breathes out.)
#[test]
fn heat_made_by_work_stays_in_the_body_and_leaves_by_its_conductance() {
    let mut s = set();
    let b = s.eater(0);
    let w = &mut s.w;
    let books = Books::now(w);
    let (mut b, days) = (b, 10.0);
    let (temp0, mass) = (b.temp(&w.chem), b.mass(&w.chem));
    let cost = 2e6;
    assert!(moved(w, (b.cell as usize, 0), |w| pcore::body::pay(w, &mut b, cost, days)), "it can pay");
    let (temp1, g) = (b.temp(&w.chem), pcore::body::conductance_to_ground(w, &b, mass, temp0));
    let (cap, dt) = (b.heat_cap(&w.chem), b.interval(w) as f64 * DAY);
    let (rise, kept) = (temp1 - temp0, cap * (temp1 - temp0));
    println!("a cost of {cost:.1e} J warms a body of {mass:.0} kg by {rise:.4} K (g {g:.2} W/K, {:.1} h time constant): the rate law says {:.4} K", cap / g / 3600.0, cost / (g * dt));
    // Its time constant is hours, far under the days to the next update: the warmth is the heat made over the heat lost per K.
    assert!(cap / g < 0.1 * dt, "the body's heat does not relax within the interval");
    assert!(rise > 0.0 && (rise * g * dt / cost) > 0.85 && (rise * g * dt / cost) < 1.0, "it warms by {rise} K, the cost over g dt is {}", cost / (g * dt));
    // What it keeps is a small part of the heat made, the rest went to the ground (a bare body passes only what its still
    // air and radiation take: its time constant is about 3 h).
    assert!(kept < 0.02 * cost, "it keeps {kept} J of {cost}");
    books.assert_kept(w, 1e-12);
    // The same work again warms it by as much more.
    let mut c = b.clone();
    assert!(moved(w, (c.cell as usize, 0), |w| pcore::body::pay(w, &mut c, cost, days)));
    let again = c.temp(&w.chem) - temp1;
    assert!((again / rise - 1.0).abs() < 0.05, "the second cost warms by {again} K, the first {rise} K");
}

/// A bigger body of the same genes is warmer for the same work per kg: the heat it makes goes with its mass, and its
/// surface, which its heat leaves by, with the mass to the power 2/3; its boundary layer is thicker too (forced
/// convection goes with size^-1/2), so a bare body's air takes mass^(1/2) and its radiation mass^(2/3): 8 times the body
/// is 2 to 2.83 times warmer, by the ratio of its conductances.
#[test]
fn a_bigger_body_of_the_same_genes_is_warmer() {
    let mut s = set();
    let small = s.eater(0);
    let big = scaled(&mut s.w, &small, 8.0);
    let w = &mut s.w;
    let rise = |w: &mut World, mut b: Body, per_kg: f64| {
        let (t0, mass) = (b.temp(&w.chem), b.mass(&w.chem));
        assert!(moved(w, (b.cell as usize, 0), |w| pcore::body::pay(w, &mut b, per_kg * mass, 10.0)), "it can pay");
        b.temp(&w.chem) - t0
    };
    let g = |w: &World, b: &Body| pcore::body::conductance_to_ground(w, b, b.mass(&w.chem), b.temp(&w.chem));
    let want = 8.0 * g(w, &small) / g(w, &big);
    let (a, c) = (rise(w, small, 1e5), rise(w, big, 1e5));
    println!("warmth over the ground for the same work per kg: {a:.4} K, 8 times the body {c:.4} K ({:.3} times; 8 g / g' = {want:.3})", c / a);
    assert!(a > 0.0 && (c / a / want - 1.0).abs() < 0.03 && want > 2.0 && want < 2.83, "{a} K and {c} K");
}

/// `days` days of a world whose ground is held at `temps[k]` K in the cell `cells[k]` (booked as an edit).
fn held_days(w: &mut World, cells: &[usize], temps: &[f64], days: usize) {
    for _ in 0..days {
        for (&i, &t) in cells.iter().zip(temps) {
            hold_ground(w, i, t);
        }
        pcore::day(w);
    }
}

/// A day of a world whose ground is held at `t` K in `cells`.
fn held_day(w: &mut World, cells: &[usize], t: f64) {
    held_days(w, cells, &vec![t; cells.len()], 1);
}

/// A resting body in a cell whose ground is held settles at the temperature where the heat its upkeep makes is the heat
/// its surface loses: T = T_ground + heat rate / conductance (the heat step of its update solves the relaxation, whose
/// time constant of hours is far under its interval of days, and the rate law makes the heat it makes depend on T). So
/// a body of kilograms stands above the ground by a degree, and one of 8 times the stores, which makes 8 times the heat
/// through 4 times the surface, by more; and matter and energy are kept.
#[test]
fn a_resting_body_settles_where_the_heat_it_makes_is_the_heat_it_loses() {
    let mut s = set();
    s.w.cfg.phys.c_build = 0.0;
    // (Flow and the survey every 3 days: the balance below is the body's own, and at the 10-day steps its surroundings
    // move its rise by about 1% more.)
    (s.w.cfg.phys_days, s.w.cfg.air_hours) = (3, 72);
    let mut small = s.eater(0);
    // (In the dark: no light-capture area to absorb the sun, which warms a body too.)
    small.genes[pcore::body::LEAF] = -9.0;
    small.prepare(&s.w.chem, &s.w.cfg.phys);
    small.vis = small.visible(&s.w.chem, &s.w.cfg);
    // (And with no liquid, which would evaporate and cool it; what its burning makes is little.)
    let w = &mut s.w;
    let wet = kg(w, &small, LIQUID);
    add(w, &mut small, LIQUID, -wet);
    (small.store[LIQUID], small.wet) = (0.0, 0);
    // (Its living body is its matter at the liquid share of tissue, `Body::built`: a quarter of the matter keeps the body,
    // and so the upkeep that warms it, as small as a dry body of all the matter would be.)
    let cut = 0.75 * kg(w, &small, MATTER);
    add(w, &mut small, MATTER, -cut);
    let lay = pcore::body::Layout::of(&w.chem);
    (small.genes[lay.min + LIQUID], small.genes[lay.keep + LIQUID]) = (0.0, 0.0);
    // (No death of age: it lives on.)
    w.cfg.phys.e_life = 1e30;
    small.prepare(&s.w.chem, &s.w.cfg.phys);
    // (An eighth of it and that again times 8, 2.5 and 20 kg: a bare body much bigger has no steady state at this upkeep,
    // its warmth over the ground past T² / (Ea e), about 4 K, and runs away.)
    let small = scaled(&mut s.w, &small, 0.125);
    let big = scaled(&mut s.w, &small, 8.0);
    let w = &mut s.w;
    // Two cells of high ground (rain does not flood it) in other air nodes (a body would take the air the other breathes).
    let mut land = s.land.clone();
    land.sort_by(|&a, &b| w.life.env[b][2].total_cmp(&w.life.env[a][2]));
    let air = w.mesh.up(land[0], 0, AIR);
    let cells = [land[0], *land.iter().find(|&&i| w.mesh.up(i, 0, AIR) != air).expect("another air node")];
    let t_cat = w.cfg.phys.t_cat;
    let (mut small, mut big) = (small, big);
    small.cell = cells[0] as u32;
    big.cell = cells[1] as u32;
    for b in [&mut small, &mut big] {
        warm(w, b, t_cat);
    }
    let slots = [w.add(small, 1), w.add(big, 1)];
    let books = Books::now(w);
    // (The ground held is that under the bodies: they have brains and may step to a neighbour cell, and the law tested is
    // the heat balance with the ground a body stands on.)
    let held = |w: &World| slots.map(|s| w.life.bodies[s].cell as usize);
    for _ in 0..90 {
        held_day(w, &held(w), t_cat);
    }
    let mut rises = [0.0; 2];
    for (n, &slot) in slots.iter().enumerate() {
        // The world and the body as its next update finds them, and the body as that update leaves it.
        let next = w.life.bodies[slot].last as u64 + w.life.bodies[slot].interval(w);
        while w.day() < next {
            held_day(w, &held(w), t_cat);
        }
        let (w0, mut b) = (w.clone(), w.life.bodies[slot].clone());
        held_day(w, &held(w), t_cat);
        assert!(w.life.bodies[slot].alive, "it lives");
        let w0 = &w0;
        b.prepare(&w0.chem, &w0.cfg.phys);
        let (i, mass, days) = (b.cell as usize, b.mass(&w0.chem), b.interval(w0) as f64);
        let (tg, chem) = (w0.t_ground_last(i), &w0.chem);
        let (ns, na, j) = (chem.ns(), w0.mesh.level(AIR).n(), w0.mesh.up(i, 0, AIR));
        // The temperature where heat made = heat lost. The heat made is what its reactions release at that temperature to
        // their products where they go: the gases to the air with their latent heat, the rest held at its temperature (each
        // mol of `k` holds `h`). Its upkeep burns its fuel (`body::pay`), and the open air burns its stores bare at any
        // temperature (a sixth of what it makes). The liquid that makes, which it keeps none of, leaves: as vapour, with its
        // latent heat, as far as the air it exchanges takes it up to its vapour pressure, and the rest to the ground as
        // liquid with its heat (`live::shed`).
        let h = |k: usize, t: f64| {
            let (s, gas) = (&chem.subs[k], chem.phase(k, t) == pcore::chem::Phase::Gas);
            s.hf + if gas { s.lf + s.lv } else { s.cp * t + chem.fused_heat(k, t, |j| b.store[j]) }
        };
        // (The bare burning gets the burning gas its breath brings, not the open air's.)
        let reach = b.k.gas * open_area(&b, chem, &w0.cfg.phys, mass) * days;
        let gas: Vec<f64> = chem.air.iter().enumerate().map(|(k, x)| if k == FIX_GAS || k == OX { x * reach } else { 0.0 }).collect();
        let made = |t: f64| {
            let (x, mut w2) = (at(w0, &b, t), w0.clone());
            let mut y = x.clone();
            assert!(moved(&mut w2, (i, 0), |w2| pcore::body::pay(w2, &mut y, pcore::body::upkeep(w0, &x, days, mass, x.temp(&w0.chem)), days)));
            let (mut g, mut rc) = (gas.clone(), pcore::change::Reactor::new(chem, &w0.cfg));
            rc.react(chem, &mut y.store, Some(&mut g), &mut (t * 1e18), 1e18, None, 0.0, days * DAY);
            // (The gas it gives off waits in its cell's bodies' node for the air's step.)
            let n = w0.mesh.cells().n();
            let held = |w: &World, k: usize| w.store.amounts(Class::Air)[k * na + j] + w.store.amounts(Class::Bodies)[k * n + i];
            let air = |k: usize| held(w0, k) - held(&w2, k) + gas[k] - g[k];
            let wet = y.store[LIQUID] - x.store[LIQUID];
            let x_air = w0.a_seen(LIQUID, j) / (0..ns).map(|k| w0.a_seen(k, j)).sum::<f64>();
            let vapour = wet.min((chem.p_sat(LIQUID, t) / pcore::units::P_REF - x_air) * b.k.gas * open_area(&b, chem, &w0.cfg.phys, mass) * days);
            (0..ns).map(|k| (x.store[k] - y.store[k] + air(k)) * h(k, t)).sum::<f64>() - vapour * chem.subs[LIQUID].lv
        };
        let lost = |t: f64| (t - tg) * pcore::body::conductance_to_ground(w0, &b, mass, t) * days * DAY;
        let (mut lo, mut hi) = (tg, tg + 20.0);
        for _ in 0..80 {
            let mid = 0.5 * (lo + hi);
            if lost(mid) < made(mid) { lo = mid } else { hi = mid }
        }
        let want = 0.5 * (lo + hi) - tg;
        rises[n] = w.life.bodies[slot].temp(chem) - tg;
        println!("{:.0} kg: {:.3} K above the ground ({tg:.2} K); heat made = heat lost at {want:.3} K", mass, rises[n]);
        // (Within 2%: what is left is the update's steps, the body's mass at its start against its mean.)
        assert!(rises[n] > 0.0 && (rises[n] / want - 1.0).abs() < 0.02, "{} K above the ground, the heat balance says {want} K", rises[n]);
    }
    assert!(rises[1] > 1.5 * rises[0], "the bigger body is warmer: {rises:?}");
    books.assert_kept(w, 1e-9);
}

/// The kg a body takes from the ground at its decision goes by the pace of its temperature: two bodies alike, at 280 K
/// and at 300 K (their temperature when they decide), take in the ratio of their paces and upkeep, the sweep being out
/// of the way.
#[test]
fn a_body_takes_from_the_ground_at_the_pace_of_its_temperature() {
    use pcore::event::EventKind;
    let mut s = set();
    s.w.cfg.phys.sweep = 1e12;
    s.w.cfg.phys.c_build = 0.0;
    // A starting eater whose brain only takes from the ground.
    s.line = Line::new(&mut s.w, &Brain::only(brain::TAKE), 1);
    let mut took = vec![];
    let mut ups = vec![];
    let mut paces = vec![];
    for (n, t) in [280.0, 300.0].into_iter().enumerate() {
        let mut b = s.eater(n);
        let w = &mut s.w;
        // Little fuel, so that it has room to take; at the temperature `t`; 100 kg of the solid per m² on its cell (booked).
        let out = 0.8 * kg(w, &b, RICH);
        add(w, &mut b, RICH, -out);
        warm(w, &mut b, t);
        put_kg_m2(w, RICH, b.cell as usize, 100.0);
        let slot = w.add(b, 1);
        pcore::survey::survey(w);
        w.time += DAY_S;
        let mut b = std::mem::take(&mut w.life.bodies[slot]);
        b.prepare(&w.chem, &w.cfg.phys);
        let (days, mass) = (b.interval(w) as f64, b.mass(&w.chem));
        let (up, pace) = (pcore::body::upkeep(w, &b, days, mass, b.temp(&w.chem)), b.pace(&w.chem, &w.cfg.phys, b.temp(&w.chem)));
        w.events.clear();
        moved(w, (b.cell as usize, slot), |w| pcore::choose::act(w, slot, &mut b, &Default::default(), &mut Default::default()));
        let id = b.id(slot);
        took.push(w.events.iter().filter_map(|e| if let EventKind::Took { by, from: u64::MAX, .. } = e.kind { (by == id).then_some(e.value) } else { None }).sum::<f64>());
        ups.push(up);
        paces.push(pace);
        w.life.bodies[slot] = b;
    }
    println!("took {took:?} kg at 280 K and 300 K; pace {paces:?}; upkeep {ups:?}");
    assert!(took[0] > 0.0 && took[1] > 0.0);
    assert!((took[1] / took[0] / (paces[1] / paces[0]) - 1.0).abs() < 0.01, "taken {took:?}, pace {paces:?}");
    assert!((took[1] / took[0] / (ups[1] / ups[0]) - 1.0).abs() < 0.01, "taken {took:?}, upkeep {ups:?}");
}

/// A body keeps its temperature at its `KEEP_T` gene: a resting eater with a thin coat (`HEAT` -0.3) that keeps 300 K on
/// ground held at 280 K stands at 300 K, where its twin that keeps 250 K stands a little above the ground, and the fuel
/// it burns in an update is the heat its surface loses at 300 K by contact and radiation (the latent heat of what it
/// breathes out and evaporates is the few % more); matter and energy are kept. With fur (-1.0) its outer surface
/// radiates, not its body: under 3 W/K in all for this body of about 100 kg (Herreid and Kessel 1967: 1.7).
#[test]
fn a_body_keeps_its_temperature_by_burning_the_heat_it_loses() {
    let mut s = set();
    s.w.cfg.phys.c_build = 0.0;
    let make = |s: &mut Set, keep: f32| {
        let mut b = s.eater(0);
        (b.genes[pcore::body::KEEP_T], b.genes[pcore::body::HEAT]) = (keep, -0.3);
        // Fuel for the test (an edit): at 30 K over its ground it burns about its own fuel in an update.
        let fuel = 4.0 * kg(&s.w, &b, RICH);
        add(&mut s.w, &mut b, RICH, fuel);
        b.prepare(&s.w.chem, &s.w.cfg.phys);
        b
    };
    let (keeps, twin) = (make(&mut s, 300.0), make(&mut s, 250.0));
    let (w, cell) = (&mut s.w, keeps.cell as usize);
    let slots = [w.add(keeps, 1), w.add(twin, 1)];
    let books = Books::now(w);
    held_days(w, &[cell], &[280.0], 21);
    // One update more: the fuel it burns against the heat it loses (by the mean of its conductance at its start and end).
    let g = |w: &World, b: &Body| pcore::body::conductance_to_ground(w, b, b.mass(&w.chem), b.temp(&w.chem));
    let (b, tg) = (w.life.bodies[slots[0]].clone(), 280.0);
    let mut fur = b.clone();
    fur.genes[pcore::body::HEAT] = -1.0;
    fur.prepare(&w.chem, &w.cfg.phys);
    let g_fur = g(w, &fur);
    assert!(g_fur < 3.0, "with fur {g_fur} W/K for {} kg", fur.mass(&w.chem));
    let (fuel0, g0, next) = (b.store[RICH], g(w, &b), b.last as u64 + b.interval(w));
    held_days(w, &[cell], &[tg], (next - w.day()) as usize + 1);
    let (y, twin) = (&w.life.bodies[slots[0]], &w.life.bodies[slots[1]]);
    assert!(y.alive && twin.alive && y.last == next as f64);
    let (t, t_twin) = (y.temp(&w.chem), twin.temp(&w.chem));
    let lost = (t - tg) * 0.5 * (g0 + g(w, y)) * y.interval(w) as f64 * DAY;
    let burned = (fuel0 - y.store[RICH]) * y.k.burn[0].1;
    println!("keeps 300 K: {t:.3} K (its twin {t_twin:.2} K) on {tg} K ground; burned {burned:.4e} J in an update, lost {lost:.4e} J ({:.3}); with fur {g_fur:.2} W/K", burned / lost);
    assert!((t - 300.0).abs() < 0.1 && t_twin < 290.0, "{t} K, its twin {t_twin} K");
    assert!(burned / lost > 1.0 && burned / lost < 1.15, "burned {burned:e} J, lost {lost:e} J");
    books.assert_kept(w, 1e-9);
}

/// A body keeps its temperature from above by evaporating: a bare resting eater (the founders' `HEAT`) that evaporates
/// above 300 K (`COOL_T`) on ground held at 295 K holds 300 K, the latent heat of what it evaporates carrying off what
/// its upkeep makes past what its surface passes at 5 K; its twin that does not (`COOL_T` 320 K, the founders') stands
/// warmer. Matter and energy are kept.
#[test]
fn a_body_keeps_its_temperature_from_above_by_evaporating() {
    let mut s = set();
    s.w.cfg.phys.c_build = 0.0;
    let make = |s: &mut Set, cool: f32| {
        let mut b = s.eater(0);
        b.genes[pcore::body::COOL_T] = cool;
        b.prepare(&s.w.chem, &s.w.cfg.phys);
        b
    };
    let (sweat, dry) = (make(&mut s, 300.0), make(&mut s, 320.0));
    let (w, cell) = (&mut s.w, sweat.cell as usize);
    let slots = [w.add(sweat, 1), w.add(dry, 1)];
    let books = Books::now(w);
    held_days(w, &[cell], &[295.0], 21);
    let (a, b) = (&w.life.bodies[slots[0]], &w.life.bodies[slots[1]]);
    assert!(a.alive && b.alive);
    let (t, t_dry) = (a.temp(&w.chem), b.temp(&w.chem));
    let kg = (b.store[LIQUID] - a.store[LIQUID]) * w.chem.subs[LIQUID].molar_mass;
    println!("on 295 K ground: evaporating above 300 K, {t:.2} K; not evaporating {t_dry:.2} K; {kg:.1} kg less liquid");
    assert!((t - 300.0).abs() < 0.5 && t_dry > t + 1.0, "{t} K, the twin {t_dry} K");
    assert!(kg > 0.0, "{kg} kg");
    books.assert_kept(w, 1e-9);
}

/// What a wet body evaporates past its store flows through it from the ground (`through`), and the ground's heat pays the
/// latent heat only after that liquid has left with its own: a node of 10^8 light-eaters on a cell whose ground is only
/// 10^10 mol of liquid at 300 K evaporates in one update no more than the ground can pay. The ground stays above
/// `T_FLOOR`, the body above zero energy, and matter and energy are kept.
#[test]
fn a_body_evaporates_no_more_than_the_ground_can_pay() {
    const MEMBERS: f64 = 1e8;
    const LIQUID_MOL: f64 = 1e10;
    let mut w = small_world();
    let (i, days) = (dry_land(&w), 36);
    empty_cell(&mut w, i);
    put(&mut w, LIQUID, i, LIQUID_MOL);
    hold_ground(&mut w, i, 300.0);
    pcore::flow::state(&mut w, true);
    let mut b = founder(&mut w, 0, i, &Brain::only(brain::REST));
    let f = MEMBERS / b.n();
    b.store.iter_mut().for_each(|a| *a *= f);
    (b.e, b.count, b.last) = (b.e * f, b.n() * f, w.day() as f64);
    let slot = w.add(b, days);
    let (books, rounded) = (Books::now(&w), w.counts.rounded);
    w.time += days * DAY_S;
    pcore::survey::survey(&mut w);
    pcore::survey::day(&mut w);
    let b = &w.life.bodies[slot];
    let floor = w.heat_cap_ground(i) * T_FLOOR;
    let e = w.store.heat(Class::Ground)[i] - floor;
    println!("the body at {:.1} K; the ground {e:.3e} J above the floor, {:.3e} of {LIQUID_MOL:e} mol of liquid left", b.temp(&w.chem), w.g(LIQUID, i));
    assert!(!b.alive || b.e >= 0.0, "the body ends at {} J", b.e);
    assert!(e >= 0.0, "the ground ends {e:e} J below the floor");
    assert_eq!(w.counts.rounded, rounded);
    books.assert_kept(&w, 1e-12);
}

/// Activity needs liquid: a body's chemistry runs in its liquid, so a dry body is nearly dormant. Activity is
/// min(1, l / l*)^n (`Body::activity`, `Physics::dormancy`): 1 at the level the body keeps and above it; a third of the
/// liquid is about 512 times less (Harrington's rule for seeds); a body with no key liquid, or no matter to keep a level
/// of, has nothing to dry.
#[test]
fn activity_goes_with_the_liquid_a_body_holds() {
    let mut s = set();
    let b = s.eater(0);
    let (chem, p) = (&s.w.chem, s.w.cfg.phys);
    assert!((b.activity(chem, &p) - 1.0).abs() < 1e-9, "a starting body holds what it keeps: {}", b.activity(chem, &p));
    for x in [0.9, 0.5, 0.2] {
        let a = s.dried(&b, x).activity(chem, &p);
        assert!((a / x.powf(p.dormancy) - 1.0).abs() < 1e-9, "{x} of the liquid: {a}");
    }
    let third = s.dried(&b, 1.0 / 3.0).activity(chem, &p);
    println!("a third of the liquid: activity 1/{:.0}", 1.0 / third);
    assert!((1.0 / third / 512.0 - 1.0).abs() < 0.05, "a 3-fold drop is about 512 times less: 1/{}", 1.0 / third);
    assert_eq!(s.dried(&b, 3.0).activity(chem, &p), 1.0, "more than it keeps is full activity");
    // No key liquid (its safe-minimum gene off, so none was liquid at birth), or no matter: nothing to dry.
    let mut none = s.dried(&b, 0.1);
    none.wet = 0;
    assert_eq!(none.activity(chem, &p), 1.0);
    let mut bare = s.dried(&b, 0.1);
    bare.store[chem.matter().expect("matter")] = 0.0;
    assert_eq!(bare.activity(chem, &p), 1.0);
    // A node's activity is that of its mean member (totals over totals).
    let mut node = s.dried(&b, 0.5);
    node.count = 7.0;
    node.store.iter_mut().for_each(|a| *a *= 7.0);
    assert!((node.activity(chem, &p) - s.dried(&b, 0.5).activity(chem, &p)).abs() < 1e-12);
}

/// A dry body pays its upkeep times its activity, the brain's cost included, and as its wear grows with the upkeep it
/// hardly ages.
#[test]
fn a_dry_body_pays_upkeep_times_activity_and_ages_slower() {
    let mut s = set();
    s.w.cfg.phys.c_build = 0.0;
    let wet = s.eater(0);
    let dry = s.dried(&wet, 0.5);
    let (d, m) = (wet.interval(&s.w) as f64, wet.mass(&s.w.chem));
    let a = dry.activity(&s.w.chem, &s.w.cfg.phys);
    let (uw, ud) = (pcore::body::upkeep(&s.w, &wet, d, m, wet.temp(&s.w.chem)), pcore::body::upkeep(&s.w, &dry, d, m, dry.temp(&s.w.chem)));
    println!("upkeep {uw:.4e} J wet, {ud:.4e} dry, activity {a:.4}");
    assert!((ud / uw / a - 1.0).abs() < 1e-9, "the same body, dry, pays its activity's share: {}", ud / uw);
    // The brain's decision is in it: with the brain costing a thousand times more, the dry body still pays its activity's share.
    s.w.cfg.phys.c_hidden *= 1000.0;
    let (uw2, ud2) = (pcore::body::upkeep(&s.w, &wet, d, m, wet.temp(&s.w.chem)), pcore::body::upkeep(&s.w, &dry, d, m, dry.temp(&s.w.chem)));
    assert!(uw2 > 2.0 * uw && (ud2 / uw2 / a - 1.0).abs() < 1e-9, "{uw2} {ud2}");
    s.w.cfg.phys.c_hidden /= 1000.0;
    // In a day the dry body's wear is a small share of the wet one's (nodes of a thousand members).
    let (wet, dry) = (node_of(&mut s.w, wet, 1000.0), node_of(&mut s.w, dry, 1000.0));
    let (ww, wd) = (wear_of_a_day(&s.w, wet), wear_of_a_day(&s.w, dry));
    println!("wear in a day: wet {ww:.3e}, dry {wd:.3e}");
    assert!(ww > 0.0 && wd < 0.1 * ww, "wear {wd} against {ww}");
}

/// The speed-up of a catalyst is 1 + (f - 1) a, a the activity: a body with no liquid has the bare reaction (as if the
/// catalyst gene were off but for its cost), half the liquid a small share of the speed-up, and a wet body all of it.
#[test]
fn a_dry_bodys_catalysed_rate_falls_toward_the_bare_rate() {
    let mut s = set();
    let wet = s.eater(0);
    let lay = pcore::body::Layout::of(&s.w.chem);
    let burn = (0..s.w.chem.reactions.len()).find(|&r| wet.genes[lay.cat + r] > 0.0 && s.w.chem.dh(r) < 0.0).expect("a catalyst for burning");
    let f = wet.dec(lay.cat + burn);
    // The same stores with the catalyst at the bare reaction (a gene of 1e-9: on, but factor 1).
    let bare = |b: &Body| {
        let mut x = b.clone();
        let on: Vec<usize> = (0..s.w.chem.reactions.len()).filter(|&r| x.genes[lay.cat + r] > 0.0 && s.w.chem.dh(r) < 0.0).collect();
        for r in on {
            x.genes[lay.cat + r] = 1e-9;
        }
        x.prepare(&s.w.chem, &s.w.cfg.phys);
        x
    };
    let rel = |x: f64| {
        let b = s.dried(&wet, x);
        // (A time short enough for the reaction to stay first order.)
        release(&s.w, &b, 0.01) / release(&s.w, &bare(&b), 0.01)
    };
    let (full, half, none) = (rel(1.0), rel(0.5), rel(0.0));
    let a = s.dried(&wet, 0.5).activity(&s.w.chem, &s.w.cfg.phys);
    println!("catalysed over bare: wet {full:.1} (f {f:.1}), half the liquid {half:.2} (1 + (f - 1) a = {:.2}), none {none:.4}", 1.0 + (f - 1.0) * a);
    assert!((full / f - 1.0).abs() < 0.02, "a wet body has all of its speed-up: {full} of {f}");
    assert!((half / (1.0 + (f - 1.0) * a) - 1.0).abs() < 0.03, "{half} against {}", 1.0 + (f - 1.0) * a);
    assert!((none - 1.0).abs() < 1e-6, "a body with no liquid has the bare rate: {none}");
}

/// Building (`copy::grow`) goes with the activity: the allowance `c_build` is times it.
#[test]
fn a_dry_body_builds_in_proportion_to_its_activity() {
    let mut s = set();
    // The allowance sets how much it builds (a tiny `c_build`, so nothing else binds).
    s.w.cfg.phys.c_build = 1e-2;
    let wet = s.eater(0);
    let mut dry = s.dried(&wet, 0.5);
    let mut wet = wet;
    let a = dry.activity(&s.w.chem, &s.w.cfg.phys);
    let m = s.w.chem.matter().expect("matter");
    let (m0, m1) = (wet.store[m], dry.store[m]);
    // (What it may build goes with its mass^0.75 too, and the dry body has the lighter liquid.)
    let eq = dry.supply(&s.w.chem, &s.w.cfg.phys, dry.temp(&s.w.chem)) / wet.supply(&s.w.chem, &s.w.cfg.phys, wet.temp(&s.w.chem));
    let grow = |w: &mut World, b: &mut Body| moved(w, (b.cell as usize, 0), |w| worldgen::testkit::grow(w, b, 10.0));
    assert!(grow(&mut s.w, &mut wet) && grow(&mut s.w, &mut dry));
    let (gw, gd) = (wet.store[m] - m0, dry.store[m] - m1);
    println!("matter built: wet {gw:.4e} mol, dry {gd:.4e}, activity {a:.4}");
    assert!(gw > 0.0 && (gd / gw / (a * eq) - 1.0).abs() < 0.02, "built {gd} against {gw} (activity {a}, mass {eq})");
}

/// A key liquid that froze kills the share of a node that is free liquid to freeze: its activity. A wet node dies of
/// PHASE, a dry one (half the liquid it keeps: a few % active) loses that share of its members at each update and
/// lives. Matter and energy are kept.
#[test]
fn frost_kills_in_proportion_to_activity() {
    let mut s = set();
    s.w.cfg.phys.c_build = 0.0;
    let count = 10_000.0;
    let node = |s: &mut Set, at: usize, x: f64| {
        let b = s.eater(at);
        let mut b = s.dried(&b, x);
        // A node of `count` such members (its matter and energy booked as an edit).
        scale(&mut s.w, &mut b, count);
        b.count = count;
        s.w.add(b, 1)
    };
    let (wet, dry) = (node(&mut s, 0, 1.0), node(&mut s, 1, 0.5));
    let a = s.w.life.bodies[dry].activity(&s.w.chem, &s.w.cfg.phys);
    let books = Books::now(&s.w);
    // The key liquid freezes at the temperature the bodies are at (they are at their cell's, and stay there as their
    // liquid gives up its fusion enthalpy: an edit).
    let temps = [wet, dry].map(|x| s.w.life.bodies[x].temp(&s.w.chem));
    s.w.chem.subs[LIQUID].melt = 400.0;
    for (x, t) in [wet, dry].into_iter().zip(temps) {
        let mut b = std::mem::take(&mut s.w.life.bodies[x]);
        warm(&mut s.w, &mut b, t);
        s.w.life.bodies[x] = b;
    }
    pcore::step(&mut s.w);
    let killed = died(&s.w, PHASE);
    let (cw, cd) = (s.w.life.bodies[wet].alive, s.w.life.bodies[dry].count);
    println!("wet node alive {cw}; dry node {cd:.0} of {count} left (activity {a:.4}); died of PHASE {killed:.0}");
    assert!(!cw, "a wet node frozen dies");
    assert!(s.w.life.bodies[dry].alive, "a dry node lives");
    // Three updates in the month, each takes the share `a` (of those left; some other deaths may add a few).
    let left = (1.0 - a).powi(3) * count;
    assert!((cd / left - 1.0).abs() < 0.02, "left {cd} of {count}, expected {left:.0}");
    assert!(killed >= count - cd - 1.0 && killed >= 0.9 * count, "died of PHASE {killed}");
    books.assert_kept(&s.w, 1e-9);
}

/// A body's molten stores hold their fusion enthalpy, as the ground's do: a body that takes 1 kg of liquid at its own
/// temperature with the heat the ground gives up for it (cp T and the fusion enthalpy) keeps its temperature, and the
/// ground it dies on, at the same temperature, keeps its own.
#[test]
fn liquid_taken_at_the_body_temperature_leaves_it_as_warm() {
    let mut s = set();
    let mut b = s.eater(0);
    let (chem, i) = (&s.w.chem, b.cell as usize);
    let t = b.temp(chem);
    assert!(t > b.melt(chem, LIQUID), "the key liquid is molten at {t} K");
    let (a, sub) = (1.0 / chem.subs[LIQUID].molar_mass, &chem.subs[LIQUID]);
    b.store[LIQUID] += a;
    b.e += a * (sub.cp * t + sub.lf);
    let t1 = b.temp(chem);
    println!("{t} K, after 1 kg of liquid {t1} K");
    assert!((t1 - t).abs() < 1e-9 * t, "1 kg of liquid at {t} K left the body at {t1} K");
    hold_ground(&mut s.w, i, t);
    let slot = s.w.add(b, 1);
    let b = std::mem::take(&mut s.w.life.bodies[slot]);
    moved(&mut s.w, (i, slot), |w| body::die(w, slot, b, body::LACK));
    let tg = s.w.t_ground(i);
    assert!((tg - t).abs() < 1e-6, "the ground at {t} K took the body's remains and went to {tg} K");
}

/// A body whose `WET` gene is below 0 gives its children less of its key liquid than of the rest, and they start as
/// warm as it is. A dry child pays almost nothing to be, so it waits for a long time where a wet one dies of want;
/// given liquid it soaks it up and wakes. Matter and energy are kept.
#[test]
fn a_dry_child_waits_and_wakes_when_it_soaks_liquid() {
    let wet_gene = -0.5f32;
    let run = |wet: f32| {
        let mut s = set();
        s.w.cfg.nodes = false;
        let books = Books::now(&s.w);
        let mut b = s.eater(0);
        (b.genes[WET], b.genes[DISPERSE], b.genes[STRENGTH]) = (wet, 0.0, 0.0);
        // A mature body (a body of 21 kg, its copy mass 1.6 kg) with a brood of three children.
        b.genes[COPY_MASS] = 0.2;
        b.prepare(&s.w.chem, &s.w.cfg.phys);
        brood(&s.w, &mut b, 3.0);
        let (before, t) = (b.store.clone(), b.temp(&s.w.chem));
        let (slot, id) = {
            let slot = s.w.add(b, 1);
            let mut b = std::mem::take(&mut s.w.life.bodies[slot]);
            assert!(pcore::copy::copy(&mut s.w, slot, &mut b));
            let id = b.id(slot);
            s.w.life.bodies[slot] = b;
            (slot, id)
        };
        let kids: Vec<usize> = children(&s.w, id).into_iter().filter(|&k| k != slot).collect();
        assert!(!kids.is_empty(), "no children");
        let c = &s.w.life.bodies[kids[0]];
        let (share_rich, share_liquid) = (c.store[RICH] / before[RICH], c.store[LIQUID] / before[LIQUID]);
        assert!((share_liquid / share_rich / 10f64.powf(wet as f64) - 1.0).abs() < 1e-9, "the child's liquid is 10^WET of its share: {share_liquid} {share_rich}");
        assert!((c.temp(&s.w.chem) / t - 1.0).abs() < 1e-9, "the child is as warm as the parent was");
        let a0 = c.activity(&s.w.chem, &s.w.cfg.phys);
        // Matter and energy are kept by the birth.
        books.assert_kept(&s.w, 1e-9);
        // A month with nothing to take in and no soaking (the children's gene for it is -4): the fuel they keep, and how active.
        let fuel = |s: &Set| kids.iter().filter(|&&k| s.w.life.bodies[k].alive).map(|&k| s.w.life.bodies[k].store[RICH]).sum::<f64>();
        let acts = |s: &Set| kids.iter().filter(|&&k| s.w.life.bodies[k].alive).map(|&k| s.w.life.bodies[k].activity(&s.w.chem, &s.w.cfg.phys)).collect::<Vec<_>>();
        let f0 = fuel(&s);
        pcore::step(&mut s.w);
        let (alive, a1, f1) = (acts(&s).len(), acts(&s).into_iter().fold(0.0, f64::max), fuel(&s) * kids.len() as f64 / (acts(&s).len().max(1)) as f64 / f0);
        // Then they can soak the liquid on the ground up (their gene for it is set, as a mutation would): they wake.
        for &k in &kids {
            let c = &mut s.w.life.bodies[k];
            c.genes[LIQ] = -0.5;
            c.prepare(&s.w.chem, &s.w.cfg.phys);
        }
        let mut a2: f64 = 0.0;
        for _ in 0..3 {
            pcore::step(&mut s.w);
            a2 = a2.max(acts(&s).into_iter().fold(0.0, f64::max));
        }
        println!("WET {wet}: {} children, activity at birth {a0:.2e}; after a month {alive} alive, most active {a1:.2e}, fuel kept {f1:.4}; most active in three months of soaking {a2:.3}", kids.len());
        (kids.len(), a0, alive, a1, f1, a2)
    };
    let (n_dry, a_dry, alive_dry, act_dry, fuel_dry, wake) = run(wet_gene);
    let (n_wet, a_wet, _, _, fuel_wet, _) = run(0.0);
    assert!(n_dry > 0 && n_wet > 0);
    assert!(a_wet > 0.3 && a_dry < 0.01, "activity at birth: {a_wet} against {a_dry}");
    assert!(alive_dry == n_dry && act_dry < 0.01, "dry children wait: {alive_dry} of {n_dry} alive, activity {act_dry}");
    // (A dry child keeps all its fuel but what rots bare in a month, at 0.27 a year: 1.1%.)
    assert!(fuel_dry > 0.985 && fuel_wet < 0.95, "a dry child pays almost nothing to be: it keeps {fuel_dry} of its fuel in a month, a wet one {fuel_wet}");
    // (What is asked is that it wakes, by orders of magnitude.)
    assert!(wake > 0.2 && wake > 100.0 * act_dry, "a dry child that soaks liquid wakes: activity {wake}");
}

/// Size is the body: its matter over 1 - h (h the share of the liquid it keeps, its keep gene below 0.95), and no more
/// than its matter and all its liquid; the fuel it stores and liquid past the share it keeps are not body. Leaf
/// (`visible`) and the open surface follow it, and so does the height order of `shade`: a body of more matter and less
/// mass is the taller.
#[test]
fn size_goes_by_body_not_by_fuel_or_free_liquid() {
    let (mut w, i) = start();
    let lay = Layout::of(&w.chem);
    let mut b = eater(&mut w, i, brain::REST, 6.0);
    // 4 kg of matter and 12 kg of liquid: the liquid is the share 0.75 the keep gene sets, so a body of 16 kg.
    b.store.iter_mut().for_each(|x| *x = 0.0);
    b.store[MATTER] = 4.0 / w.chem.subs[MATTER].molar_mass;
    b.store[LIQUID] = 12.0 / w.chem.subs[LIQUID].molar_mass;
    b.genes[lay.keep + LIQUID] = 0.75;
    b.wet = 1 << LIQUID;
    let size = |w: &World, b: &Body| b.size(&w.chem);
    assert!((size(&w, &b) - 16.0).abs() < 1e-9, "matter 4 kg over 1 - 0.75: {}", size(&w, &b));
    // Fuel is not body, however much of it: nor is liquid past the share kept.
    add(&mut w, &mut b, RICH, 100.0);
    add(&mut w, &mut b, LIQUID, 50.0);
    assert!((size(&w, &b) - 16.0).abs() < 1e-9, "fuel and free liquid add no size: {}", size(&w, &b));
    assert!(b.mass(&w.chem) > 160.0);
    // A body short of liquid is as big as it is wet.
    add(&mut w, &mut b, LIQUID, -55.0);
    assert!((size(&w, &b) - 11.0).abs() < 1e-9, "matter 4 kg and 7 kg of liquid: {}", size(&w, &b));
    // The keep gene above 0.95 counts as 0.95: matter over 0.05.
    b.genes[lay.keep + LIQUID] = 1.0;
    add(&mut w, &mut b, LIQUID, 100.0);
    assert!((size(&w, &b) - 80.0).abs() < 1e-9, "matter 4 kg over 1 - 0.95: {}", size(&w, &b));
    // No key liquid: the body is its matter.
    b.wet = 0;
    assert!((size(&w, &b) - 4.0).abs() < 1e-9, "{}", size(&w, &b));
    // A chemistry with no body matter: the whole mass.
    let mut plain = w.chem.clone();
    plain.build = None;
    assert!((b.size(&plain) - b.mass(&plain)).abs() < 1e-9);

    // Leaf and open surface follow the size: the same body with 100 kg more fuel has the same leaf.
    let (mut a, mut c) = (eater(&mut w, i, brain::REST, 6.0), eater(&mut w, i, brain::REST, 6.0));
    (a.genes[body::LEAF], c.genes[body::LEAF]) = (0.0, 0.0);
    add(&mut w, &mut c, RICH, 100.0);
    a.prepare(&w.chem, &w.cfg.phys);
    c.prepare(&w.chem, &w.cfg.phys);
    let (va, vc) = (a.visible(&w.chem, &w.cfg), c.visible(&w.chem, &w.cfg));
    assert!(va[1] > 0.0 && va[1] == vc[1], "leaf {} and {}", va[1], vc[1]);
    assert!(vc[0] > 4.0 * va[0], "the mass differs: {} and {}", va[0], vc[0]);
    let ma = a.mass(&w.chem);
    assert!((open_area(&a, &w.chem, &w.cfg.phys, ma) - open_area(&c, &w.chem, &w.cfg.phys, ma)).abs() < 1e-9 * ma, "the open surface, with a mass of {ma} kg");

    // Height: two nodes in one cell, B of 3 times the matter of A but lighter (A holds 100 kg of fuel a member more): by
    // mass A is the taller, by body B. Leaf area is set so that each catches a good part of the cell's light and the
    // layers differ.
    let n = 100.0;
    let (mut ha, mut hb) = (eater(&mut w, i, brain::REST, 6.0), eater(&mut w, i, brain::REST, 6.0));
    (ha.genes[body::LEAF], hb.genes[body::LEAF]) = (4.0, 4.0);
    scale(&mut w, &mut hb, 3.0 * n);
    scale(&mut w, &mut ha, n);
    add(&mut w, &mut ha, RICH, 100.0 * n);
    (ha.count, hb.count) = (n, n);
    let (sa, sb) = (w.add(ha, 1), w.add(hb, 1));
    for s in [sa, sb] {
        let mut x = std::mem::take(&mut w.life.bodies[s]);
        x.prepare(&w.chem, &w.cfg.phys);
        x.vis = x.visible(&w.chem, &w.cfg);
        w.life.bodies[s] = x;
    }
    // (Not at a month's start: alike nodes of a cell merge then.)
    w.time += DAY_S;
    pcore::survey::survey(&mut w);
    let (a, b) = (&w.life.bodies[sa], &w.life.bodies[sb]);
    let (ma, mb) = (a.mass(&w.chem) / n, b.mass(&w.chem) / n);
    println!("shade: A (member mass {ma:.0} kg, body {:.1}) {:.3}, B (member mass {mb:.0} kg, body {:.1}) {:.3}; leaf per ground area {:.2} and {:.2}", a.size(&w.chem) / n, w.life.shade[sa], b.size(&w.chem) / n, w.life.shade[sb], a.vis[1] as f64 * n / w.mesh.cells().area()[0], b.vis[1] as f64 * n / w.mesh.cells().area()[0]);
    assert!(ma > 1.3 * mb && a.size(&w.chem) < b.size(&w.chem), "A is heavier and smaller in body");
    assert!(w.life.shade[sb] > 1.2 * w.life.shade[sa], "the body of more matter is the taller: shade {} against {}", w.life.shade[sb], w.life.shade[sa]);
}

/// What a body carries is bounded by its body, not its whole mass: the keep gene of the liquid is the share of the body
/// it makes up (so it holds h / (1 - h) of its matter), the keep gene of any other store (not body matter) is kg of it
/// per kg of body (`Body::size`). So more fuel does not raise the level of the liquid or of another store, and a body
/// that takes without limit stops at them: left on ground full of the solid and the liquid it never holds more than its
/// keep genes times its body, however long it takes.
#[test]
fn a_body_cannot_store_more_than_keep_times_body() {
    let (mut w, i) = start();
    let lay = Layout::of(&w.chem);
    let mut b = eater(&mut w, i, brain::TAKE, 6.0);
    let (mm_rich, mm_liq) = (w.chem.subs[RICH].molar_mass, w.chem.subs[LIQUID].molar_mass);
    let (keep_rich, h) = (b.genes[lay.keep + RICH] as f64, b.genes[lay.keep + LIQUID] as f64);
    let mass = b.mass(&w.chem);
    let size = b.size(&w.chem);
    // The levels: the solid keep x body, the liquid h / (1 - h) x matter.
    let (lv_rich, lv_liq) = (b.level(&w.chem, RICH, mass) * mm_rich, b.level(&w.chem, LIQUID, mass) * mm_liq);
    println!("body {size:.2} kg of {mass:.2}: solid level {lv_rich:.2} kg (keep {keep_rich:.2}), liquid level {lv_liq:.2} kg (h {h:.2})");
    assert!((lv_rich / (keep_rich * size) - 1.0).abs() < 1e-9, "{lv_rich} of {keep_rich} x {size}");
    assert!((lv_liq / (h / (1.0 - h) * kg(&w, &b, MATTER)) - 1.0).abs() < 1e-9, "{lv_liq}");
    // More fuel does not raise the other levels, nor the size; scaling the whole body raises all.
    add(&mut w, &mut b, RICH, 100.0);
    assert!((b.level(&w.chem, RICH, b.mass(&w.chem)) * mm_rich / lv_rich - 1.0).abs() < 1e-9 && (b.size(&w.chem) / size - 1.0).abs() < 1e-9);
    add(&mut w, &mut b, RICH, -100.0);
    let mut big = b.clone();
    scale(&mut w, &mut big, 4.0);
    assert!((big.level(&w.chem, RICH, big.mass(&w.chem)) * mm_rich / lv_rich - 4.0).abs() < 1e-9);
    // A long take from ground rich in both: the stores stop at their levels.
    w.cfg.phys.c_handle = 0.0;
    w.cfg.phys.c_build = 0.0;
    put_kg_m2(&mut w, RICH, i, 50.0);
    put_kg_m2(&mut w, LIQUID, i, 50.0);
    let rich0 = b.store[RICH];
    let s = w.add(b, 1);
    let (mut top_rich, mut top_liq) = (0.0f64, 0.0f64);
    for _ in 0..120 {
        pcore::day(&mut w);
        let b = &w.life.bodies[s];
        let mass = b.mass(&w.chem);
        let (x_rich, x_liq) = (b.store[RICH] * mm_rich / (b.level(&w.chem, RICH, mass) * mm_rich).max(1e-30), b.store[LIQUID] * mm_liq / (b.level(&w.chem, LIQUID, mass) * mm_liq).max(1e-30));
        top_rich = top_rich.max(x_rich);
        top_liq = top_liq.max(x_liq);
    }
    let b = &w.life.bodies[s];
    println!("after 120 days: solid {:.2} kg (was {:.2}), at most {top_rich:.3} of its level; liquid at most {top_liq:.3} of its level; body {:.2} kg, mass {:.2} kg", kg(&w, b, RICH), rich0 * mm_rich, b.size(&w.chem), b.mass(&w.chem));
    assert!(b.store[RICH] > rich0, "it took none of the solid");
    // (The liquid starts at its level, and burning the solid leaves liquid in the body, so it goes a little past; taking
    // without limit would take it to many times that.)
    assert!(top_rich < 1.0 + 1e-6 && top_liq < 1.1, "past its level: solid {top_rich}, liquid {top_liq}");
    assert!(b.mass(&w.chem) < 3.0 * b.size(&w.chem), "mass {} for a body of {}", b.mass(&w.chem), b.size(&w.chem));
}

/// The safe minimums (the min genes) are shares of the body, so a body is not killed by holding fuel: a starting eater
/// of 30 kg (body 21 kg, solid 9 kg, liquid 15.75 kg) with a minimum for the liquid of 0.7 of its body (14.7 kg, under
/// the 15.75 it has) would be short of it if it were 0.7 of its whole mass (21 kg), and still is not after it takes in
/// 100 kg of the solid.
#[test]
fn the_safe_minimum_is_a_share_of_the_body() {
    let (mut w, i) = start();
    let lay = Layout::of(&w.chem);
    let mut b = eater(&mut w, i, brain::REST, 6.0);
    b.genes[lay.min + LIQUID] = 0.7;
    let (size, mass) = (b.size(&w.chem), b.mass(&w.chem));
    println!("body {size:.2} kg of {mass:.2}: liquid {:.2} kg against a minimum of {:.2} kg", kg(&w, &b, LIQUID), 0.7 * size);
    assert!(kg(&w, &b, LIQUID) > 0.7 * size && kg(&w, &b, LIQUID) < 0.7 * mass, "the setup");
    assert!(!b.lacking(&w.chem, b.mass(&w.chem), b.temp(&w.chem)), "lacking a minimum that is met");
    add(&mut w, &mut b, RICH, 100.0);
    assert!(!b.lacking(&w.chem, b.mass(&w.chem), b.temp(&w.chem)), "killed by holding fuel");
    // And a minimum above what it holds still counts: 0.8 of the body is 16.8 kg of the liquid.
    b.genes[lay.min + LIQUID] = 0.8;
    assert!(b.lacking(&w.chem, b.mass(&w.chem), b.temp(&w.chem)));
}

/// Digestion: body matter that a body takes in from outside is broken into the energy-rich solid and the minerals as it
/// comes in (the reverse of growth), so it enters as fuel, never as body, and the body builds its own matter from it at
/// the cost of work. Elements and energy are kept. A body whose catalyst for burning matter is off takes none.
#[test]
fn matter_taken_from_outside_is_digested_into_the_solid() {
    let take_from_ground = |digests: bool| {
        let (mut w, i) = start();
        w.cfg.phys.c_handle = 0.0;
        w.cfg.phys.c_build = 0.0;
        // 3 kg of body matter per m² on its ground (booked as an edit).
        put_kg_m2(&mut w, MATTER, i, 3.0);
        let mut b = eater(&mut w, i, brain::TAKE, 6.0);
        let lay = Layout::of(&w.chem);
        let (_, burn_matter) = w.chem.matter_roles().expect("matter");
        if !digests {
            b.genes[lay.cat + burn_matter] = -1.0;
        }
        // Only matter on the ground to take: it takes neither the liquid nor the solid.
        (b.genes[lay.upt + LIQUID], b.genes[lay.upt + RICH]) = (-0.1, -0.1);
        let (rich0, matter0, size0) = (b.store[RICH], b.store[MATTER], b.size(&w.chem));
        let s = w.add(b, 1);
        let books = Books::now(&w);
        // (What the body took from the ground, by the events, not what left it: dead matter on the ground also rots.)
        let mut took = 0.0;
        for _ in 0..10 {
            w.events.clear();
            pcore::day(&mut w);
            took += took_from_ground(&w);
        }
        let b = &w.life.bodies[s];
        books.assert_kept(&w, 1e-9);
        (b.store[RICH] - rich0, b.store[MATTER] - matter0, took / w.chem.subs[MATTER].molar_mass, b.size(&w.chem) - size0)
    };
    let (rich, matter, taken, size) = take_from_ground(true);
    println!("an eater took {taken:.2} mol of matter from the ground: solid +{rich:.2} mol, its own matter {matter:+.3} mol, body {size:+.3} kg");
    assert!(taken > 0.5, "it took none");
    // The solid rose by 118 per mol taken, less what it burnt in ten days; its own matter did not rise.
    assert!(rich > 20.0 * taken, "solid +{rich} mol for {taken} mol of matter");
    assert!(matter <= 1e-9 && size <= 1e-9, "matter entered as body: {matter} mol, body {size} kg");
    let (_, _, none, _) = take_from_ground(false);
    assert!(none < 1e-12, "a body that cannot burn matter took {none} mol of it");
    // The reverse of growth, exactly, called directly.
    let (mut w, i) = start();
    let mut b = eater(&mut w, i, brain::REST, 6.0);
    add(&mut w, &mut b, MATTER, 34.56);
    let (s, mut b) = out(&mut w, b);
    let (books, before) = (Books::now(&w), b.store.clone());
    let x = b.digest(&w.chem, 10.0, &mut 0.0);
    assert!((x - 10.0).abs() < 1e-12);
    // 10 of matter and the liquid growth makes with it become what growth is made of, 10 times its counts.
    let growth = w.chem.build.as_ref().expect("growth");
    for (side, sign) in [(&growth.inputs, 1.0), (&growth.outputs, -1.0)] {
        for p in side.iter() {
            let k = p.0 as usize;
            assert!((sign * (b.store[k] - before[k]) - 10.0 * p.1 as f64).abs() < 1e-9, "substance {k}: {} -> {}", before[k], b.store[k]);
        }
    }
    w.life.bodies[s] = b;
    books.assert_kept(&w, 1e-12);
}

/// The light a node's light reactions store is set aside in its cell as it falls (not drawn back from the ground's heat
/// at its update): a node a thousand times as large as seeded that decides once a year takes it from there, no heat
/// goes ungiven, and matter and energy are kept.
#[test]
fn a_big_node_stores_light_set_aside_as_it_fell() {
    let mut w = worldgen::new_world(1, 16, 16);
    pcore::step(&mut w);
    worldgen::life::seed(&mut w, 0);
    for _ in 0..3 {
        pcore::step(&mut w);
    }
    let s = (0..w.life.bodies.len()).filter(|&s| w.life.bodies[s].alive).max_by(|&a, &b| w.life.bodies[a].count.total_cmp(&w.life.bodies[b].count)).expect("a node");
    let b = &mut w.life.bodies[s];
    b.count *= 1e3;
    b.store.iter_mut().for_each(|a| *a *= 1e3);
    b.e *= 1e3;
    // It decides once a year: a year's stored light at once.
    b.genes[pcore::body::INTERVAL] = 2.556;
    b.prepare(&w.chem, &w.cfg.phys);
    let (books, h0) = (Books::now(&w), w.counts.clamped_heat);
    for _ in 0..14 {
        pcore::step(&mut w);
        assert_eq!(pcore::check::broken(&w), None);
    }
    assert!(w.store.heat(Class::Bodies).iter().sum::<f64>() > 0.0 && w.counts.clamped_heat == h0, "heat not given {:e} J", w.counts.clamped_heat - h0);
    books.assert_kept(&w, 1e-9);
}

/// Light passes through a cell's bodies from the tallest member down (`survey::shade`): of two light-eater nodes with the
/// same capture area, the one of 100 times heavier members catches more light per m² of it (e^(its capture area per
/// ground area) times more), the two together catch what the cell's canopy intercepts, and matter and energy are kept.
#[test]
fn a_tall_member_shades_a_short_one() {
    let mut w = worldgen::new_world(1, 16, 16);
    pcore::step(&mut w);
    worldgen::life::seed(&mut w, 0);
    let short = (0..w.life.bodies.len()).filter(|&s| w.life.bodies[s].alive).max_by(|&a, &b| w.life.bodies[a].count.total_cmp(&w.life.bodies[b].count)).expect("a node");
    let (i, area) = (w.life.bodies[short].cell as usize, w.mesh.cells().area()[0]);
    // The cell's other nodes move next door.
    for s in w.life.at[i].clone().into_iter().map(|s| s as usize).filter(|&s| s != short) {
        let j = i ^ 1;
        w.life.relocate(s, i, j);
        (w.life.bodies[s].cell, w.life.bodies[s].mark) = (j as u32, w.light[j]);
    }
    // Two nodes of capture area 0.5 per ground area each, the same but for member mass (and the copy mass gene, so they
    // are not alike and stay apart).
    let b = &mut w.life.bodies[short];
    let f = 0.5 * area / (b.vis[1] as f64 * b.n());
    b.store.iter_mut().for_each(|a| *a *= f);
    (b.e, b.count) = (b.e * f, b.n() * f);
    let mut tall = b.clone();
    // Leaf goes with mass^0.75: 100 times fewer members of 100 times the mass have 100^-0.25 of the leaf, so the tall
    // one's LEAF gene is raised by 0.5 to make the areas equal.
    (tall.count, tall.genes[pcore::body::COPY_MASS]) = (b.n() / 100.0, b.genes[pcore::body::COPY_MASS] + 2.0);
    tall.genes[pcore::body::LEAF] += 0.5;
    tall.prepare(&w.chem, &w.cfg.phys);
    (b.vis, tall.vis) = (b.visible(&w.chem, &w.cfg), tall.visible(&w.chem, &w.cfg));
    let slots = [w.add(tall, 1), short];
    let books = Books::now(&w);
    pcore::survey::survey(&mut w);
    let (light0, dt) = (w.light[i], w.cfg.phys_days as f64 * DAY);
    worldgen::testkit::flow_step(&mut w, dt);
    let bs = &w.life.bodies;
    let per = |s: usize| w.light[i] - bs[s].mark;
    let cap = |s: usize| bs[s].vis[1] as f64 * bs[s].n();
    let (t, sh) = (per(slots[0]), per(slots[1]));
    println!("light per m² of capture area: tall {t:.4e}, short {sh:.4e} J");
    assert!(sh > 0.0 && (t / sh / (cap(slots[0]) / area).exp() - 1.0).abs() < 1e-6, "tall {t:e}, short {sh:e}");
    let (all, cell) = (cap(slots[0]) * t + cap(slots[1]) * sh, (w.light[i] - light0) * w.canopy[i] as f64 * area);
    assert!((all / cell - 1.0).abs() < 1e-6, "bodies caught {all:e} J, the canopy {cell:e} J");
    for _ in 0..3 {
        pcore::step(&mut w);
        assert_eq!(pcore::check::broken(&w), None);
    }
    books.assert_kept(&w, 1e-9);
}

/// Living wears a body: its members die at the rate of the energy their upkeep turns over per kg, over `e_life` x
/// 10^REPAIR, so they live that life on average: a node loses the share 1 - e^-wear of its members at each update, an
/// individual dies with that chance. More repair: a longer life (10 times per unit) at a higher upkeep (the repair
/// share, a quarter at gene 0, doubles per unit). Matter and energy are kept.
#[test]
fn bodies_die_of_age_by_their_rate_of_living() {
    use pcore::body::{AGE, COPY_MASS, DISPERSE, LEARN, REPAIR, STRENGTH};
    use pcore::brain;
    const ONES: usize = 100;
    let mut w = small_world();
    // Bodies that neither grow nor soak liquid up (their mass, which the wear goes by, stays): no building work, and no
    // liquid exchange with the ground. Their copy mass is half their body: the matter that rots is not starving. No one
    // ages until they have settled, and bodies with a brain do not merge into nodes (`--individuals`).
    (w.cfg.phys.c_build, w.cfg.phys.e_life, w.cfg.nodes) = (0.0, 1e30, false);
    // Upkeep goes with the temperature: the bodies stand on ground held at the temperature the costs are anchored to
    // (`Physics::t_cat`, booked as an edit), so that they pay what the costs say, in two cells of high ground (rain
    // does not flood it) with other air nodes.
    let mut land = dry_lands(&w);
    land.sort_by(|&a, &b| w.life.env[b][2].total_cmp(&w.life.env[a][2]));
    let air = w.mesh.up(land[0], 0, AIR);
    let cells = [land[0], *land.iter().find(|&&i| w.mesh.up(i, 0, AIR) != air).expect("another air node")];
    // Starting eaters whose brain only rests, weak so their stores last.
    let line = Line::new(&mut w, &Brain::only(brain::REST), 1);
    let make = |w: &mut pcore::World, i: usize, repair: f32, count: f64| {
        let mut b = line.founder(w, 1, i);
        // (And they do not learn: a body that learns from its well-being may learn to walk away in the months of the test.)
        (b.genes[STRENGTH], b.genes[DISPERSE], b.genes[REPAIR], b.genes[pcore::body::LIQ], b.genes[LEARN]) = (0.0, 0.0, repair, -4.0, -6.0);
        b.genes[COPY_MASS] -= std::f32::consts::LOG10_2;
        // A node of `count` such members (its matter and energy booked as an edit), with three times the fuel.
        let fuel = 2.0 * kg(w, &b, RICH);
        add(w, &mut b, RICH, fuel);
        scale(w, &mut b, count);
        b.count = count;
        b.prepare(&w.chem, &w.cfg.phys);
        w.add(b, 1)
    };
    // A node of members at repair 0 alone in one cell; one at repair 0.5 and a hundred individuals at repair 0 in the
    // other (an individual's death is of one member, a node's of a share).
    let (sa, sb) = (make(&mut w, cells[1], 0.0, 1000.0), make(&mut w, cells[0], 0.5, 1000.0));
    let ones: Vec<usize> = (0..ONES).map(|_| make(&mut w, cells[0], 0.0, 1.0)).collect();
    // A day with the ground held; the age deaths of the day, as (cell, day, members).
    let t_cat = w.cfg.phys.t_cat;
    let day = |w: &mut pcore::World, deaths: &mut Vec<(usize, u64, f64)>| {
        for &i in &cells {
            hold_ground(w, i, t_cat);
        }
        w.events.clear();
        pcore::day(w);
        deaths.extend(w.events.iter().filter(|e| matches!(e.kind, pcore::event::EventKind::Died { cause: AGE, .. })).map(|e| (e.node as usize, e.day, e.value)));
    };
    // Two months for the bodies to settle at the temperature their heat and the ground give them, then their upkeep per
    // kg of body and day.
    let mut settle = Vec::new();
    for _ in 0..60 {
        day(&mut w, &mut settle);
    }
    let per_kg = |w: &mut pcore::World, s: usize| {
        let mut b = std::mem::take(&mut w.life.bodies[s]);
        b.prepare(&w.chem, &w.cfg.phys);
        let (d, m) = (b.interval(w) as f64, b.mass(&w.chem));
        let (u, act) = (pcore::body::upkeep(w, &b, d, m, b.temp(&w.chem)) / (d * b.built(&w.chem)), b.activity(&w.chem, &w.cfg.phys));
        w.life.bodies[s] = b;
        (u, act)
    };
    assert!(ones.iter().all(|&s| w.life.bodies[s].alive), "an individual died while settling");
    let ((ua, aa), (ub, ab), (ui, ai)) = (per_kg(&mut w, sa), per_kg(&mut w, sb), per_kg(&mut w, ones[0]));
    println!("upkeep per kg of body and day: {ua:.1}, {ub:.1}, an individual {ui:.1} J");
    // (The repair gene costs 0.75 + 0.25 x 2^0.5 times as much at 0.5.)
    assert!(((ub / ab) / (ua / aa) / (0.75 + 0.25 * 2f64.sqrt()) - 1.0).abs() < 0.03 && ((ui / ai) / (ua / aa) - 1.0).abs() < 0.07, "upkeep per kg {ua} {ub} {ui}");
    // A life of 38 days at gene 0, 10^0.5 ua / ub times that at 0.5. Each counts its days from its last update.
    let (life, x) = (38.0, 10f64.sqrt() * ua / ub);
    w.cfg.phys.e_life = life * ua;
    let (books, n0, every) = (Books::now(&w), w.life.bodies[sa].count, w.life.bodies[sa].interval(&w) as f64);
    let from: Vec<f64> = [sa, sb].iter().chain(&ones).map(|&s| w.life.bodies[s].last).collect();
    let day0 = w.day() as f64;
    let mut deaths = Vec::new();
    for _ in 0..240 {
        day(&mut w, &mut deaths);
    }
    // A node's mean life: the days its members lived over those who died; it loses a share at each update of members
    // who die evenly between updates, so over a span they live dead / ln(n / (n - dead)) times its days.
    let node_life = |cell: usize, t0: f64| {
        let (mut n, mut t, mut lived) = (n0, t0, 0.0);
        for &(_, d, dead) in deaths.iter().filter(|x| x.0 == cell && x.2 != 1.0) {
            (lived, n, t) = (lived + dead * (d as f64 - t) / (n / (n - dead).max(f64::MIN_POSITIVE)).ln(), n - dead, d as f64);
        }
        lived / (n0 - n)
    };
    let (la, lb) = (node_life(cells[1], from[0]), node_life(cells[0], from[1]));
    // The individuals dead of age in one life: each dies at an update with the chance its wear since gives, so by day
    // `life` one that updated last at u has died with the chance 1 - e^-((u - its first day) / its life).
    let (li, by) = (life * ua / ui, day0 + life);
    let chance = |f: f64| -(-((by - f) / every).floor() * every / li).exp_m1();
    let expected: f64 = from[2..].iter().map(|&f| chance(f)).sum();
    let sd = from[2..].iter().map(|&f| chance(f) * (1.0 - chance(f))).sum::<f64>().sqrt();
    let dead = deaths.iter().filter(|x| x.0 == cells[0] && x.2 == 1.0 && (x.1 as f64) <= by).count() as f64;
    println!("lives, days: nodes {la:.1} and {lb:.1} at genes 0 and 0.5 (predicted {life} and {:.0}); individuals dead in a life {dead} of {ONES} (expected {expected:.1} +- {sd:.1})", life * x);
    assert!((la / life - 1.0).abs() < 0.1 && (lb / (life * x) - 1.0).abs() < 0.1, "node lives {la} and {lb}");
    assert!((dead - expected).abs() < 3.5 * sd, "individuals dead in a life: {dead}, expected {expected} +- {sd}");
    books.assert_kept(&w, 1e-9);
}

/// What a member takes and can build goes with its mass to the power `intake_exp`: 1 kg counts as 1 kg, 10 t takes 180
/// times what 10 kg does (not 1,000), and a node of n members counts n times a member. `equiv` counts the mass it is
/// given; `Body::supply` gives it the body that is built, not the fuel stored.
#[test]
fn equiv_goes_with_mass_to_the_power() {
    use pcore::body::{Body, COPY_MASS};
    let p = pcore::Physics::default();
    let member = |count: f64, copy: f32| {
        let mut genes = vec![0.0f32; 64];
        genes[COPY_MASS] = copy;
        Body { count, genes, ..Default::default() }
    };
    let take = |kg: f64| member(1.0, 6.0).equiv(&p, kg);
    assert!((take(1.0) - 1.0).abs() < 1e-12, "a member of 1 kg counts as 1 kg");
    assert!((take(1e4) / take(10.0) - 177.8).abs() < 0.1, "10 t takes 180 times 10 kg: {}", take(1e4) / take(10.0));
    assert!((member(4.0, 6.0).equiv(&p, 4.0 * 30.0) / take(30.0) - 4.0).abs() < 1e-9, "a node counts n members' worth");
    let m = member(1.0, 1.0);
    assert!(m.equiv(&p, 2000.0) > m.equiv(&p, 20.0), "equiv counts the mass it is given");
    // What a body takes and builds counts its body, not the fuel it stores (`Body::supply`): the same body holding three
    // times the fuel has the same supply.
    let w = small_world();
    let mut b = Body { count: 1.0, genes: vec![0.0; Layout::of(&w.chem).len], store: vec![0.0; w.chem.ns()], ..Default::default() };
    b.genes[COPY_MASS] = 1.0;
    (b.store[MATTER], b.store[RICH]) = (10.0, 20.0);
    let thin = b.supply(&w.chem, &w.cfg.phys, b.temp(&w.chem));
    b.store[RICH] *= 3.0;
    assert!(thin > 0.0 && (b.supply(&w.chem, &w.cfg.phys, b.temp(&w.chem)) / thin - 1.0).abs() < 1e-12, "fuel changed the supply");
    b.store[MATTER] *= 8.0;
    assert!((b.supply(&w.chem, &w.cfg.phys, b.temp(&w.chem)) / thin - 8f64.powf(0.75)).abs() < 1e-9, "supply goes with the body to the power 0.75");
}

/// The cost of keeping tissue goes with the kg of the body, mass^1.0, at every mass (the supply of a body goes with
/// mass^0.75, `equiv_goes_with_mass_to_the_power`): the same genes, a body 1, 10, 100 and 10,000 times the size pay
/// upkeep in proportion to their body, so per kg of body the same, with basal, catalysts, strength and learning all in
/// it; across kinds it goes by the size a kind matures at (Kleiber), and a node pays for each member. The brain's
/// decision (mass^0.25 per member) and the leaf (by area) are left out here (set to nothing).
#[test]
fn upkeep_per_kg_of_body_is_the_same_at_every_size() {
    use pcore::body::{LEARN, REPAIR, STRENGTH};
    let (mut w, i) = start();
    // A starting eater: strength 0.6, a burning catalyst, learning on.
    let mut b = founder(&mut w, 1, i, &Brain::neutral());
    (b.genes[STRENGTH], b.genes[LEARN], b.genes[REPAIR]) = (0.6, -1.0, 0.0);
    b.prepare(&w.chem, &w.cfg.phys);
    // At the temperature the costs are anchored to, where upkeep is what the costs say.
    b.e = b.energy_at(&w.chem, w.cfg.phys.t_cat);
    let p = &mut w.cfg.phys;
    (p.c_brain, p.c_hidden, p.c_memory, p.c_seen, p.c_sight, p.c_leaf) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    let days = 10.0;
    let up = |w: &pcore::World, x: &pcore::body::Body| pcore::body::upkeep(w, x, days, x.mass(&w.chem), x.temp(&w.chem));
    let per_kg: Vec<f64> = [1.0, 10.0, 100.0, 1e4]
        .iter()
        .map(|&f| {
            let mut x = b.clone();
            x.store.iter_mut().for_each(|a| *a *= f);
            x.e = x.energy_at(&w.chem, w.cfg.phys.t_cat);
            up(&w, &x) / x.built(&w.chem)
        })
        .collect();
    println!("upkeep per day and kg of body at 21 kg x 1, 10, 100, 10,000: {:?}", per_kg.iter().map(|u| u / days).collect::<Vec<_>>());
    assert!(per_kg[0] > 0.0);
    for u in &per_kg {
        assert!((u / per_kg[0] - 1.0).abs() < 1e-9, "upkeep per kg of body differs with size: {per_kg:?}");
    }
    // Strength, catalysts, learning and basal are each in it, by the kg of body (at the temperature the costs are anchored to).
    let p = w.cfg.phys;
    let kind = (b.dec(pcore::body::COPY_MASS) / p.m_ref).powf(p.intake_exp - 1.0);
    let want = days * kind * (p.c_basal + p.c_cat * b.k.cats + p.c_strength * b.gene(STRENGTH) + p.c_learn * b.learn_rate() as f64);
    assert!((per_kg[0] / want - 1.0).abs() < 1e-9, "per kg of body {} against the costs summed {want}", per_kg[0]);
    // Across kinds by the size a kind matures at: the same body of a kind that matures 16 times larger pays half as much
    // per kg.
    let mut big = b.clone();
    big.genes[pcore::body::COPY_MASS] += 16f32.log10();
    assert!((up(&w, &big) / up(&w, &b) - 0.5).abs() < 1e-6, "a kind 16 times larger pays {} of the upkeep per kg", up(&w, &big) / up(&w, &b));
    // A node of 5 such members pays for each member.
    let mut node = b.clone();
    node.count = 5.0;
    node.store.iter_mut().for_each(|a| *a *= 5.0);
    node.e = node.energy_at(&w.chem, w.cfg.phys.t_cat);
    assert!((up(&w, &node) / up(&w, &b) / 5.0 - 1.0).abs() < 1e-9, "a node of 5 pays {} times a member's upkeep", up(&w, &node) / up(&w, &b));
}

/// Leaf goes with mass^0.75 like intake: the same `LEAF` gene gives a member of 1 kg 10^LEAF m² per kg, a member of
/// 1,000 kg 1000^0.75 times that in all (178 m², not 1,000), so 5.6 times less per kg; a node counts its members; and
/// it goes by all the body, with no cap. The open surface follows (plus its skin).
#[test]
fn leaf_area_per_kg_falls_with_mass() {
    use pcore::body::{Body, COPY_MASS, LEAF};
    let w = small_world();
    let (chem, p) = (w.chem.clone(), w.cfg.phys);
    // A member of `kg` (that copies at 10^`copy` kg), 0.5 m² of leaf per kg of 1 kg.
    let member = |count: f64, kg: f64, copy: f32| {
        let mut b = Body { count, genes: vec![0.0; pcore::body::Layout::of(&chem).len], store: vec![0.0; chem.ns()], ..Default::default() };
        (b.genes[LEAF], b.genes[COPY_MASS]) = (0.5f32.log10(), copy);
        // Body matter for a living body of `kg` per member (`Body::built`: matter over 1 - `WET_TISSUE`; no key liquid).
        let m = chem.matter().expect("body matter");
        b.store[m] = count * kg * (1.0 - pcore::body::WET_TISSUE) / chem.subs[m].molar_mass;
        b.prepare(&chem, &p);
        b
    };
    let leaf = |b: &Body| b.visible(&chem, &w.cfg)[1] as f64;
    let (one, tonne) = (member(1.0, 1.0, 6.0), member(1.0, 1000.0, 6.0));
    println!("leaf of a member: 1 kg {:.3} m², 1,000 kg {:.3} m²", leaf(&one), leaf(&tonne));
    assert!((leaf(&one) - 0.5).abs() < 1e-6, "1 kg: {}", leaf(&one));
    assert!((leaf(&tonne) / leaf(&one) / 1000f64.powf(0.75) - 1.0).abs() < 1e-6, "1,000 kg: {}", leaf(&tonne));
    assert!((leaf(&tonne) / 1000.0 / (leaf(&one) / 1.0) - 1000f64.powf(-0.25)).abs() < 1e-6, "per kg it falls");
    // A node of 10 members of 1,000 kg: the open surface is 10 members' leaf plus their skin.
    let node = member(10.0, 1000.0, 6.0);
    let m = node.mass(&chem);
    let skin = 10.0 * 6.0 * (m / 10.0 / p.rho_body).powf(2.0 / 3.0);
    assert!((open_area(&node, &chem, &p, m) - (10.0 * leaf(&tonne) + skin)).abs() < 1e-6 * (10.0 * leaf(&tonne) + skin), "open area {}", open_area(&node, &chem, &p, m));
    // Nothing caps the mass that works (a body is mature from its copy mass but keeps growing): a member of 10 kg (its
    // copy mass) and one of 1,000 kg have leaf in the ratio of their bodies to the power 0.75.
    let (at, past) = (member(1.0, 10.0, 1.0), member(1.0, 1000.0, 1.0));
    assert!((leaf(&past) / leaf(&at) / 100f64.powf(0.75) - 1.0).abs() < 1e-5 && (leaf(&at) / 0.5 / 10f64.powf(0.75) - 1.0).abs() < 1e-5, "past maturity: {} and {}", leaf(&at), leaf(&past));
}

/// A small world after a year, and a land cell with no liquid standing, whose pores hold liquid, and the dissolved
/// mineral in it.
fn start_wet() -> (World, usize) {
    let mut w = worldgen::new_world(1, 16, 16);
    for _ in 0..12 {
        pcore::step(&mut w);
    }
    let n = w.mesh.cells().n();
    let i = (0..n).find(|&i| w.life.env[i][2] > 0.0 && w.life.env[i][1] < 1e-3 && w.store.amounts(Class::Ground)[DISSOLVED * n + i] > 1e5).expect("wet land");
    (w, i)
}

/// A starting body of kind `which` (0 light-eaters, 1 eaters) of a brain that only does fixed action `act`, weak,
/// staying where it is, soaking no liquid up, repair 0.
fn eater_of(w: &mut World, which: usize, i: usize, act: usize) -> Body {
    let mut b = founder(w, which, i, &Brain::only(act));
    (b.genes[DISPERSE], b.genes[STRENGTH], b.genes[LIQ], b.genes[REPAIR]) = (0.0, 0.0, -4.0, 0.0);
    b.prepare(&w.chem, &w.cfg.phys);
    b
}

/// Fixing the air's gas: in the open it is nothing, and a body whose genes speed the reaction up makes the mineral from
/// the air's gas and the energy-rich solid it burns, and gives off the reaction's gas, at the cost of the reaction: it
/// burns 14 units of its fuel for each mol of the mineral, and their heat pays none of its upkeep (it burns that much
/// more than its twin), and the enzyme costs upkeep by its capacity like the others. A body that has none fixes none.
/// Elements and energy are kept.
#[test]
fn a_body_that_fixes_makes_the_mineral_at_the_cost_of_the_reaction() {
    let (chem, init) = worldgen::chemistry0();
    let (mineral, _) = chem.minerals()[0];
    let fix = chem.fixing().expect("fixing");
    let (_, rich, _, _) = chem.life_roles().expect("roles");
    // In the open: ten years at 290 K, the air in plenty and the liquid and the fuel there.
    let mut rc = change::Reactor::new(&chem, &cfg());
    let (mut n, mut gas, mut e) = open_air(&chem, &init, 290.0);
    (n[LIQUID], n[rich]) = (1e6, 1e6);
    rc.react(&chem, &mut n, Some(&mut gas), &mut e, OPEN_CAP, None, 0.0, 10.0 * 365.0 * DAY);
    println!("in the open, ten years: {:.3e} mol of the mineral", n[mineral]);
    assert!(n[mineral] < 1e-6 * 1e6, "the open world fixed {} mol with 10^6 mol of the fuel there", n[mineral]);
    // A plant that fixes (the enzyme speeds the reaction up by 10^FIX_GENE) and its twin that does not, 60 days.
    const FIX: f32 = worldgen::life::FIX_GENE;
    let run = |cat: f32| {
        let (mut w, i) = start();
        let lay = Layout::of(&w.chem);
        let mut b = eater_of(&mut w, 0, i, brain::REST);
        // Its own mineral just short of nothing: what it holds past its level goes to the ground and is counted there.
        b.genes[lay.cat + fix] = cat;
        // (None of the dissolved mineral, so neither builds: the two differ by the fixing alone. Their copy mass a tenth
        // below their body, so the matter that rots unrebuilt in the 60 days is not starving.)
        b.store[w.chem.minerals()[1].0] = 0.0;
        b.genes[pcore::body::COPY_MASS] += 0.9f32.log10();
        let s = w.add(b, 1);
        let books = Books::now(&w);
        let na = w.mesh.level(AIR).n();
        let n = w.mesh.cells().n();
        // (What it gives off waits in the bodies' nodes for the air's step.)
        let air = |w: &World, k: usize| [(Class::Air, na), (Class::Bodies, n)].iter().map(|&(c, len)| w.store.amounts(c)[k * len..(k + 1) * len].iter().sum::<f64>()).sum::<f64>();
        let before = (in_all(&w, mineral), air(&w, FIX_OUT));
        let mut up = 0.0;
        for _ in 0..60 {
            pcore::day(&mut w);
            let mut b = w.life.bodies[s].clone();
            b.prepare(&w.chem, &w.cfg.phys);
            // Its upkeep over that of the same body with the gene off.
            let mut off = b.clone();
            off.genes[lay.cat + fix] = -1.0;
            off.prepare(&w.chem, &w.cfg.phys);
            up = body::upkeep(&w, &b, 1.0, b.mass(&w.chem), b.temp(&w.chem)) / body::upkeep(&w, &off, 1.0, b.mass(&w.chem), off.temp(&w.chem));
        }
        assert!(w.life.bodies[s].alive);
        books.assert_kept(&w, 1e-9);
        let after = (in_all(&w, mineral), air(&w, FIX_OUT));
        let b = &w.life.bodies[s];
        (after.0 - before.0, after.1 - before.1, up, b.k.cats, b.store[rich], b.mass(&w.chem))
    };
    let (fixed, co, up_with, cats_with, fuel_with, mass) = run(FIX);
    let (none, co_none, up_without, cats_without, fuel_without, _) = run(-1.0);
    println!("60 days: the fixing plant made {fixed:.3e} mol of the mineral and put out {co:.3e} mol of the gas fixing gives off, upkeep {up_with:.4} times that with the gene off ({up_without:.4}); the other {none:.3e} mol; fuel {fuel_with:.4e} against {fuel_without:.4e}");
    // (What the enzyme made is the difference.)
    let made = fixed - none;
    assert!(made > 1.0, "the plant fixed {made} mol");
    println!("it fixed {:.2e} mol of the mineral a kg of plant and second", made / (mass * 60.0 * DAY));
    // One mol of the gas fixing gives off for each mol of the mineral made; it is where the energy of the reaction went.
    assert!((co - co_none) / made > 0.9 && (co - co_none) / made < 1.1, "{} mol of the gas for {made} of the mineral", co - co_none);
    // And 14 mol of its fuel for each, the units the reaction burns (and the enzyme's upkeep, a little more): their heat
    // paid no upkeep.
    let units = chem.reactions[fix].inputs.iter().find(|p| p.0 as usize == rich).expect("the fuel").1 as f64;
    assert!((fuel_without - fuel_with) / made > 0.95 * units && (fuel_without - fuel_with) / made < 1.2 * units, "{} mol of fuel for {made} of the mineral", fuel_without - fuel_with);
    // The enzyme costs by its capacity: the plant that fixes pays more upkeep than it would with the gene off.
    assert!(cats_with > cats_without && up_with > 1.0 && up_without == 1.0, "capacity {cats_with} against {cats_without}, upkeep {up_with} against {up_without}");
}

/// Roots take each mineral from the ground by the uptake genes, up to the level the body keeps and no more; a body
/// whose gene is off takes none. What a body holds of one past its level goes back to the ground at its update, with
/// its heat (an eater that digested matter, a plant that burned its own); the dissolved mineral, which falls out of the
/// liquid in a body in days as the rock, goes back as the rock past the level the body keeps of it (the rock is a solid
/// bodies build, kept at its share, `Body::level`). Elements and energy are kept.
#[test]
fn roots_take_the_minerals_up_to_the_level_and_the_excess_goes_back() {
    let (w0, _) = start();
    for (q, &(mineral, _)) in w0.chem.minerals().iter().enumerate() {
        // (The dissolved mineral lives only in the liquid of the pores, which the rock and the gas make it in: none is put
        // on this ground, it has its own.)
        let (mut w, i) = start_wet();
        let lay = Layout::of(&w.chem);
        let n = w.mesh.cells().n();
        // A plant with its store of this mineral emptied, the ground full of it.
        let mut plant = eater_of(&mut w, 0, i, brain::REST);
        // (It soaks as a plant does, 1 a day, the rate roots take what is dissolved at; with no capture area it catches no
        // rain, whose liquid would bring what is dissolved in the ground with it.)
        (plant.genes[LIQ], plant.genes[pcore::body::LEAF]) = (0.0, -9.0);
        let mut idle = plant.clone();
        for b in [&mut plant, &mut idle] {
            let a = b.store[mineral];
            w.cross((Boundary::Outside, Node::new(Class::Body, 0)), [(mineral, -a)], 0.0);
            b.store[mineral] = 0.0;
        }
        idle.genes[lay.upt + mineral] = -0.1;
        assert!(plant.genes[lay.upt + mineral] > 0.0);
        if q == 0 {
            put(&mut w, mineral, i, 1e9);
        }
        let level = |w: &World, b: &Body| b.level(&w.chem, mineral, b.mass(&w.chem));
        let (sp, si) = (w.add(plant, 1), w.add(idle, 1));
        let books = Books::now(&w);
        // (The most it held in the days, at the end of an update: the dissolved mineral falls out as the rock between them.)
        let mut most = 0.0f64;
        for _ in 0..40 {
            pcore::day(&mut w);
            most = most.max(w.life.bodies[sp].store[mineral]);
        }
        let (took, lvl) = (w.life.bodies[sp].store[mineral], level(&w, &w.life.bodies[sp]));
        println!("substance {mineral}: a plant with the gene on held {took:.3e} mol (at most {most:.3e}), its level {lvl:.3e}, the rock {:.3e}; the other {:.3e}", w.life.bodies[sp].store[ROCK], w.life.bodies[si].store[mineral]);
        assert!(most > 0.1 * lvl && most <= lvl * (1.0 + 1e-6), "held at most {most}, level {lvl}");
        let kept = |w: &World, s: usize| w.life.bodies[s].store[ROCK] <= w.life.bodies[s].level(&w.chem, ROCK, 0.0) * (1.0 + 1e-9);
        assert!(kept(&w, sp) && kept(&w, si), "the rock past its level stayed in a body");
        assert!(w.life.bodies[si].store[mineral] < 1e-9 * lvl, "a body whose gene is off took substance {mineral}");
        books.assert_kept(&w, 1e-9);
        // The excess: give the plant ten times its level, and it is back on the ground at its next update (as the rock for
        // the dissolved mineral: the rock element in the ground, in all its forms, rises by what was over).
        let mut twin = w.clone();
        let more = 10.0 * lvl;
        w.cross((Boundary::Outside, Node::new(Class::Body, sp)), [(mineral, more)], more * w.chem.subs[mineral].cp * w.life.bodies[sp].temp(&w.chem));
        w.life.bodies[sp].e += more * w.chem.subs[mineral].cp * w.life.bodies[sp].temp(&w.chem);
        w.life.bodies[sp].store[mineral] += more;
        for _ in 0..40 {
            twin.events.clear();
            pcore::day(&mut twin);
            w.events.clear();
            pcore::day(&mut w);
        }
        let (held, lvl2) = (w.life.bodies[sp].store[mineral], level(&w, &w.life.bodies[sp]));
        // (Against its twin, the same world without the excess: the fixed mineral by itself, the rock element in all its
        // forms.)
        let back = if q == 0 { w.store.amounts(Class::Ground)[mineral * n + i] - twin.g(mineral, i) } else { rock_outside_matter(&w, i) - rock_outside_matter(&twin, i) };
        println!("with ten times its level more: it holds {held:.3e} against a level of {lvl2:.3e}, the rock {:.3e}; the ground gained {back:.3e} (the excess {more:.3e})", w.life.bodies[sp].store[ROCK]);
        assert!(held <= lvl2 * 1.05, "held {held} past the level {lvl2}");
        assert!(kept(&w, sp), "the rock past its level stayed in the body");
        assert!(back > 0.5 * more, "the excess did not go back to the ground: {back} of {more}");
        books.assert_kept(&w, 1e-9);
    }
}

/// End-product inhibition: a body's catalyst is slowed by its store of each product it keeps a level of, by
/// 1 / (1 + store / level): with none held it runs at full speed, at its level at half. Gases and the key liquid do not
/// count, so a burning (gas and the liquid) is not slowed.
#[test]
fn a_catalyst_is_slowed_by_the_product_the_body_holds() {
    let (mut w, i) = start();
    let (mineral, _) = w.chem.minerals()[0];
    let fix = w.chem.fixing().expect("fixing");
    let (light, rich, _, burn) = w.chem.life_roles().expect("roles");
    let b = eater_of(&mut w, 0, i, brain::REST);
    let (_, mut b) = out(&mut w, b);
    let (t, mass) = (b.temp(&w.chem), b.mass(&w.chem));
    for (r, k) in [(fix, mineral), (light, rich)] {
        b.store[k] = 0.0;
        let none = b.inhibited(&w.chem, r, mass, t);
        b.store[k] = b.level(&w.chem, k, mass);
        let full = b.inhibited(&w.chem, r, mass, t);
        assert!(none == 1.0 && (full - 0.5).abs() < 1e-9, "reaction {r}: {none} with none held, {full} at its level");
    }
    assert_eq!(b.inhibited(&w.chem, burn, mass, t), 1.0, "a burning makes gas and the liquid");
}
