//! Law 1 (Flow): heat and radiation, the air, gases dissolving, liquid in the pores, drainage and runoff, what the
//! liquid carries, erosion, and the edges that moving bodies clear.

use pcore::chem::Phase;
use pcore::store::{Boundary, Class, Node};
use pcore::units::DAY;
use pcore::{body, brain, choose, lanes, World, AIR};
use worldgen::testkit::{ground_of, hold_ground, loose_volume as loose, moved, put, set_amount, set_heat, Books, Brain, Line, DISSOLVED, LIQUID, MINERAL, OFF, RICH};

/// One step of radiation and the air.
const DT: f64 = 3.0 * DAY;
/// One liquid step.
const LIQUID_STEP: f64 = 15.0 * DAY;

/// The highest cell of `w`.
fn top(w: &World) -> usize {
    (0..w.mesh.cells().n()).max_by(|&a, &b| w.surface_z(a).total_cmp(&w.surface_z(b))).unwrap()
}

/// What the pores of the loose ground of cell `i` hold of the liquid, mol: the porosity times the volume of its loose
/// solids.
fn pore_mol(w: &World, i: usize) -> f64 {
    w.cfg.phys.porosity * loose(w, i) / w.chem.molar_volume(LIQUID, Phase::Liquid)
}

/// Two land cells side by side, both warm and dry: the higher first.
fn pair(w: &World) -> (usize, usize) {
    let (n, gw) = (w.mesh.cells().n(), w.map.w);
    let warm_land = |i: usize| w.surface_z(i) > 50.0 && w.t_ground(i) > 280.0;
    let (a, b) = (0..n).filter(|i| i % gw + 1 < gw).map(|i| (i, i + 1)).find(|&(a, b)| warm_land(a) && warm_land(b)).expect("two land cells");
    if w.surface_z(a) >= w.surface_z(b) { (a, b) } else { (b, a) }
}

/// `vol` m³ of the liquid poured on cell `i`, warm enough to be liquid.
fn pour(w: &mut World, i: usize, vol: f64) {
    let t = w.t_ground(i).max(w.chem.subs[LIQUID].melt + 5.0);
    put(w, LIQUID, i, vol / w.chem.molar_volume(LIQUID, Phase::Liquid));
    hold_ground(w, i, t);
}

/// The liquid on the ground of cell `i`, m³.
fn liquid_m3(w: &World, i: usize) -> f64 {
    w.store.amounts(Class::Ground)[LIQUID * w.mesh.cells().n() + i] * w.chem.molar_volume(LIQUID, Phase::Liquid)
}

/// A hot node held at a melting point with little fusion enthalpy left: radiating for a whole step at that
/// temperature would take more heat than it holds. It leaves the plateau and cools by its heat capacity, toward the
/// balance with sunlight and the air, not down to the floor.
#[test]
fn hot_plateau_cools_but_not_to_the_floor() {
    let mut w = worldgen::new_world(3, 16, 16);
    let (ns, area) = (w.chem.ns(), w.mesh.cells().area()[0]);
    let k = (0..ns).find(|&k| (700.0..1000.0).contains(&w.chem.subs[k].melt)).expect("a substance melting near 850 K");
    let i = top(&w);
    (0..ns).for_each(|x| set_amount(&mut w, Node::ground(i), x, 0.0));
    set_amount(&mut w, Node::ground(i), k, area);
    let melt = w.chem.subs[k].melt;
    // (At its melting point all of it holds its fusion enthalpy, `World::energy_ground`: half of it frozen.)
    let e = w.energy_ground(i, melt) - 0.5 * w.g(k, i) * w.chem.subs[k].lf;
    set_heat(&mut w, Node::ground(i), e);
    assert_eq!(w.thaw(i).1.map(|p| p.0), Some(k));
    let books = Books::now(&w);
    worldgen::testkit::flow_step(&mut w, DT);
    let t = w.t_ground(i);
    assert!(t > 150.0 && t < melt, "{t} K after a step from {melt} K");
    books.assert_within(&w, 1e-12, 1e-9);
}

/// A pool with a solute in it freezes lower than a pure one (`Chemistry::melt_in`): 1 mol of solute in a kg of the
/// liquid lowers its melting point by the textbook 1.86 K, and Flow holds each pool, half frozen, at its own melting
/// point.
#[test]
fn a_pool_with_a_solute_freezes_lower() {
    let mut w = worldgen::new_world(3, 16, 16);
    let (n, ns, k) = (w.mesh.cells().n(), w.chem.ns(), LIQUID);
    let solute = (0..ns).find(|&j| j != k && w.chem.subs[j].melt < w.chem.subs[k].melt && w.chem.subs[j].boil > 1000.0).expect("a solute");
    let (pure, mixed) = (0, n - 1);
    let pool = 1e4 * w.mesh.cells().area()[0] / w.chem.molar_volume(k, Phase::Liquid);
    for i in [pure, mixed] {
        (0..ns).for_each(|x| set_amount(&mut w, Node::ground(i), x, 0.0));
        set_amount(&mut w, Node::ground(i), k, pool);
    }
    let dissolved = w.g(k, mixed) * w.chem.subs[k].molar_mass;
    set_amount(&mut w, Node::ground(mixed), solute, dissolved);
    let tm = [pure, mixed].map(|i| w.melt(k, i));
    assert_eq!(tm[0], w.chem.subs[k].melt);
    let dt = (w.chem.subs[k].melt - tm[1]) / 1.86;
    assert!((dt - 1.0).abs() < 0.03, "{dt} of the textbook lowering");
    for (i, t) in [pure, mixed].into_iter().zip(tm) {
        let e = w.energy_ground(i, t) - 0.5 * w.g(k, i) * w.chem.subs[k].lf;
        set_heat(&mut w, Node::ground(i), e);
        let (at, plateau) = w.thaw(i);
        assert!(at == t && plateau.is_some_and(|(q, f)| q == k && (f - 0.5).abs() < 1e-9), "{at} K, {plateau:?}");
    }
    let books = Books::now(&w);
    worldgen::testkit::flow_step(&mut w, DT);
    // (Liquid ran off the pools in the step, and some evaporated: each is held at its melting point as the step left it.)
    for i in [pure, mixed] {
        let held = w.scratch.temps().0[i];
        assert!((held - w.melt(k, i)).abs() < MELT_HELD && w.thaw_last(i).1.is_some_and(|(q, _)| q == k), "{held} K, melting at {} K", w.melt(k, i));
    }
    assert!(w.scratch.temps().0[mixed] < w.scratch.temps().0[pure], "the pool with a solute is held lower");
    books.assert_within(&w, 1e-12, 1e-9);
}

/// How near its melting point Flow holds a pool after a step, K: the step's evaporation changes what is dissolved.
const MELT_HELD: f64 = 1e-3;

/// A warmer air node gives its heat to its neighbours.
#[test]
fn warm_air_spreads() {
    let mut cold = worldgen::new_world(3, 16, 16);
    let mut warm = cold.clone();
    let j = cold.map.w.div_ceil(cold.air_scale) + 1;
    set_heat(&mut warm, Node::air(j), 1.2 * cold.store.heat(Class::Air)[j]);
    let books = Books::now(&warm);
    let excess = warm.t_air(j) - cold.t_air(j);
    worldgen::testkit::flow_step(&mut cold, DT);
    worldgen::testkit::flow_step(&mut warm, DT);
    assert!(warm.t_air(j) - cold.t_air(j) < 0.5 * excess, "warm node kept its heat");
    assert!(warm.t_air(j + 1) - cold.t_air(j + 1) > 0.5, "neighbour did not warm");
    books.assert_within(&warm, 1e-12, 1e-9);
}

/// Heat spreads sideways through flat, even ground as diffusion does: on the bed of one land cell everywhere with no
/// loose ground, a hot spot keeps its excess heat while its variance grows by 2 D t along each axis, D = k / (rho c) of
/// the bed (backward Euler on an open lattice gives just that growth at any step length).
#[test]
fn a_hot_spot_spreads_through_the_ground_at_its_diffusivity() {
    let mut w = worldgen::new_world(3, 16, 16);
    let (n, ns, g) = (w.mesh.cells().n(), w.chem.ns(), w.map);
    let land = (0..n).find(|&i| w.surface_z(i) > 0.0).expect("land");
    let (mut cp, mut vol, mut kv) = (0.0, 0.0, 0.0);
    for k in 0..ns {
        let (b, v) = (w.store.amounts(Class::Bed)[k * n + land], w.chem.molar_volume(k, Phase::Solid));
        (cp, vol, kv) = (cp + b * w.chem.subs[k].cp, vol + b * v, kv + b * v * w.chem.subs[k].conductivity);
        for i in 0..n {
            set_amount(&mut w, Node::new(Class::Bed, i), k, b);
            set_amount(&mut w, Node::ground(i), k, 0.0);
        }
    }
    let mid = g.idx(g.w / 2, g.h / 2);
    for i in 0..n {
        hold_ground(&mut w, i, if i == mid { 310.0 } else { 300.0 });
    }
    w.scratch = Default::default();
    let (books, base, hot) = (Books::now(&w), w.energy_ground(0, 300.0), w.store.heat(Class::Ground)[mid] - w.energy_ground(0, 300.0));
    // Spread to a standard deviation of one cell, in ten steps (the edges, 7 cells off, then reflect almost nothing).
    let d = kv / cp;
    let t = g.side * g.side / (2.0 * d);
    (0..10).for_each(|_| pcore::flow::conduct(&mut w, t / 10.0));
    let (mx, my) = g.xy(mid);
    let (mut sum, mut var) = (0.0, [0.0; 2]);
    for i in 0..n {
        let ((x, y), q) = (g.xy(i), w.store.heat(Class::Ground)[i] - base);
        sum += q;
        for (v, c) in var.iter_mut().zip([x as f64 - mx as f64, y as f64 - my as f64]) {
            *v += q * (c * g.side).powi(2);
        }
    }
    println!("diffusivity {d:.2e} m2/s over {:.0} years: variance {:.6} and {:.6} of 2 D t", t / DAY / 365.0, var[0] / sum / (2.0 * d * t), var[1] / sum / (2.0 * d * t));
    assert!((sum / hot - 1.0).abs() < 1e-9, "excess heat {sum} of {hot}");
    assert!(var.iter().all(|v| (v / sum / (2.0 * d * t) - 1.0).abs() < 1e-4), "variance {var:?} of {}", 2.0 * d * t * sum);
    books.assert_within(&w, 1e-12, 1e-12);
}

/// More heat-absorbing gas keeps warming the ground: under 240 W/m² of sunlight, with the air following the ground by
/// the lapse rate, the balance at optical depth 17 is clearly warmer than at 9 (one slab at the air's mean temperature
/// would give both the same).
#[test]
fn more_gas_warms_with_no_ceiling() {
    let ground = |x: f64| {
        let (mut lo, mut hi) = (150.0, 500.0);
        for _ in 0..60 {
            let t: f64 = (lo + hi) / 2.0;
            let up = (-x).exp() * 5.670e-8 * t.powi(4) + pcore::flow::glow(-(-x).exp_m1(), t, &pcore::flow::column(x, 0.0065, 1e4, 7600.0)[1]);
            (lo, hi) = if up > 240.0 { (lo, t) } else { (t, hi) };
        }
        lo
    };
    let (t1, t9, t17) = (ground(1.5), ground(9.0), ground(17.0));
    assert!(t9 > t1 + 20.0 && t17 > t9 + 3.0, "{t1} {t9} {t17}");
}

/// The most soluble gas of the air dissolves in the liquid of the deepest node toward Henry's share (here with a fast
/// liquid side, so within an exchange), and part of it comes out when that liquid is warmed by 20 K (a fifth at
/// balance); matter and energy are kept.
#[test]
fn gas_dissolves_and_comes_out_when_warmed() {
    let (_, init) = worldgen::chemistry0();
    let mut w = worldgen::new_world(3, 16, 16);
    w.cfg.dissolve = 1.0;
    let (n, na, c) = (w.mesh.cells().n(), w.mesh.level(AIR).n(), &w.chem);
    let k = init.air.iter().map(|p| p.0).filter(|&k| c.subs[k].boil < 250.0).max_by(|&a, &b| c.subs[a].solubility.total_cmp(&c.subs[b].solubility)).unwrap();
    let i = (0..n).max_by(|&a, &b| w.store.amounts(Class::Ground)[LIQUID * n + a].total_cmp(&w.store.amounts(Class::Ground)[LIQUID * n + b])).unwrap();
    let books = Books::now(&w);
    (0..4).for_each(|_| worldgen::testkit::flow_step(&mut w, DT));
    // Dissolved per mol of liquid, against Henry's share under the gas's share of the reference pressure.
    let held = |w: &World| w.store.amounts(Class::Ground)[k * n + i] / w.store.amounts(Class::Ground)[LIQUID * n + i];
    let j = w.mesh.up(i, 0, AIR);
    let x = w.store.amounts(Class::Air)[k * na + j] / (0..w.chem.ns()).map(|q| w.store.amounts(Class::Air)[q * na + j]).sum::<f64>();
    let (h1, want) = (held(&w), w.chem.henry(k, w.t_ground(i), x * pcore::units::P_REF));
    assert!(h1 > 0.5 * want && h1 < 2.0 * want, "{h1} dissolved per mol, Henry's share {want}");
    let t = w.t_ground(i) + 20.0;
    hold_ground(&mut w, i, t);
    (0..4).for_each(|_| worldgen::testkit::flow_step(&mut w, DT));
    assert!(held(&w) < 0.9 * h1, "{} dissolved per mol after warming, {h1} before", held(&w));
    books.assert_within(&w, 1e-12, 1e-9);
}

/// The pore space of a cell is the porosity times the volume of its loose solids; the liquid in it does not stand:
/// what the bodies feel as liquid depth (`env_of`) is only the liquid over it.
#[test]
fn pore_space_is_the_porosity_times_the_loose_solids() {
    let mut w = worldgen::new_world(3, 16, 16);
    let (i, _) = pair(&w);
    let (pores, area) = (w.cfg.phys.porosity * loose(&w, i), w.mesh.cells().area()[0]);
    // The loose layer of the generated land is 1 m of a mix of loose solids, 0.4 of it pores (random packing: 0.36 to 0.44).
    assert!((pores / area / 0.4 - 1.0).abs() < 0.05, "{} m of pore depth under a metre of loose ground", pores / area);
    let depth = |w: &World| pcore::survey::env_of(w, i)[1];
    assert_eq!(depth(&w), 0.0);
    pour(&mut w, i, 0.5 * pores);
    assert_eq!(depth(&w), 0.0, "liquid that fits in the pores does not stand");
    pour(&mut w, i, pores);
    let stands = depth(&w) * area;
    assert!((stands / (0.5 * pores) - 1.0).abs() < 1e-6, "{stands} m³ stands, {} over the pores", 0.5 * pores);
}

/// Liquid soaks in up to the pores and stays (a sheet runs off where the ground has none), and only what is over them
/// runs off. Matter and energy are kept.
#[test]
fn liquid_soaks_into_the_pores_and_only_the_rest_runs_off() {
    let run = |porosity: f64, share: f64| {
        let mut w = worldgen::new_world(3, 16, 16);
        w.cfg.phys.porosity = porosity;
        let (i, _) = pair(&w);
        let pores = 0.4 * loose(&w, i);
        pour(&mut w, i, share * pores);
        let books = Books::now(&w);
        let v0 = liquid_m3(&w, i);
        worldgen::testkit::flow_step(&mut w, LIQUID_STEP);
        books.assert_within(&w, 1e-12, 1e-9);
        (liquid_m3(&w, i) / v0, liquid_m3(&w, i) / pores)
    };
    // Half the pores' volume: it all stays (but for what the open air takes); with no pores the same sheet runs off.
    let (kept, _) = run(0.4, 0.5);
    let (kept_bare, _) = run(0.0, 0.5);
    assert!(kept > 0.7, "the ground kept {kept} of its liquid");
    assert!(kept_bare < 0.1, "with no pores {kept_bare} of it stayed");
    // Twice the pores' volume: the rest ran off, and the pores keep their field capacity (`field`, 0.6) as the rest sinks.
    let (_, full) = run(0.4, 2.0);
    assert!(full < 1.05 && full > 0.55, "the ground holds {full} of its pore volume after the excess ran off");
}

/// Each part of Flow runs on its own period on the world's clock: with the air every 3 days and liquid every 15,
/// liquid poured on a slope (four times what the pores hold) after the first day stays through the steps of the air
/// and runs off at the 15-day step.
#[test]
fn liquid_runs_on_its_own_period_between_steps_of_the_air() {
    let mut w = worldgen::new_world(3, 16, 16);
    (w.cfg.air_hours, w.cfg.flow_days) = (72, 15);
    pcore::day(&mut w);
    let (i, _) = pair(&w);
    let pores = w.cfg.phys.porosity * loose(&w, i);
    pour(&mut w, i, 4.0 * pores);
    let v0 = liquid_m3(&w, i);
    (1..15).for_each(|_| pcore::day(&mut w));
    assert!(liquid_m3(&w, i) > 0.5 * v0, "{} of {v0} m³ stayed through the steps of the air", liquid_m3(&w, i));
    pcore::day(&mut w);
    assert!(liquid_m3(&w, i) < 0.5 * v0, "{} of {v0} m³ stayed after the liquid step", liquid_m3(&w, i));
}

/// Darcy: the pore liquid of a cell flows to a neighbour whose liquid table is lower, at `seep` x the slope x the
/// thickness it fills x the width, in the time. With `seep` at 0 the neighbour gets none; the flow goes with `seep`,
/// with the liquid table's slope and with the thickness; pores under their field capacity (`field`, 0.6) give none;
/// matter and energy are kept.
#[test]
fn pore_liquid_flows_down_the_liquid_table_at_darcys_rate() {
    // One liquid step with the liquid of cell `i` filling the share `share` of its pores and `j` dry: what `j` gained,
    // m³, the drop of the liquid table and the thickness that carries the flow.
    let gained = |seep: f64, share: f64| {
        let mut w = worldgen::new_world(3, 16, 16);
        let (i, j) = pair(&w);
        w.cfg.phys.seep = seep;
        // (No evaporation, which would lower the liquid table before the liquid flows, and no channels inside the cell.)
        (w.cfg.evap, w.cfg.phys.hill) = (0.0, 1e15);
        let pores = w.cfg.phys.porosity * loose(&w, i);
        pour(&mut w, i, share * pores);
        let books = Books::now(&w);
        worldgen::testkit::flow_step(&mut w, LIQUID_STEP);
        books.assert_within(&w, 1e-12, 1e-9);
        let (area, depth) = (w.mesh.cells().area()[0], loose(&w, i) / w.mesh.cells().area()[0]);
        // The liquid table of `i` stood at its bottom plus the depth the liquid fills; `j` was dry on its own bottom.
        let (bi, bj) = (w.surface_z(i) - depth, w.surface_z(j) - loose(&w, j) / area);
        let ti = bi + share * depth;
        (liquid_m3(&w, j), ti - bj, ti - bi.max(bj))
    };
    let (none, ..) = gained(0.0, 0.8);
    let (some, dh, b) = gained(1e-6, 0.8);
    let want = 1e-6 * b * dh * LIQUID_STEP;
    println!("slope {dh:.2} m over one cell, thickness {b:.3} m: Darcy {want:.3} m³, gained {:.3} m³", some - none);
    assert!(dh > 0.0 && b > 0.0, "the table of the wet cell is higher: {dh} {b}");
    assert!(((some - none) / want - 1.0).abs() < 0.05, "gained {} m³, Darcy says {want}", some - none);
    // Ten times the conductivity, ten times the flow.
    let (more, ..) = gained(1e-5, 0.8);
    assert!(((more - none) / (some - none) / 10.0 - 1.0).abs() < 0.05, "x10 gave x{}", (more - none) / (some - none));
    // More liquid, a thicker flow: the thickness that carries it, and the slope, are bigger.
    let (full, dh2, b2) = gained(1e-6, 0.95);
    let want2 = 1e-6 * b2 * dh2 * LIQUID_STEP;
    assert!(b2 > b && dh2 > dh);
    assert!(((full - none) / want2 - 1.0).abs() < 0.05, "gained {} m³, Darcy says {want2}", full - none);
    // Under field capacity the ground holds its pore liquid against gravity.
    let (held, ..) = gained(1e-6, 0.5);
    assert_eq!(held, none, "under field capacity the pores gave {} m³", held - none);
}

/// Darcy's law across a 2 km cell moves under 0.02% of its pores a year (`seep`, a metre of ground, a slope of 1 in
/// 1,000), but the liquid drains inside the cell to its channels, `hill` from the divides: a full store of the bed on a
/// sloping cell loses a share 1 - exp(-`bed_seep` x slope x time / (`bed_porosity` x `hill`)) of it in a step, which runs
/// off (it does not stay in the cell); under deep standing liquid (no slope) it loses none that way.
#[test]
fn liquid_drains_inside_a_cell_to_its_channels() {
    let w0 = worldgen::new_world(3, 16, 16);
    let (n, gw, area, p) = (w0.mesh.cells().n(), w0.map.w, w0.mesh.cells().area()[0], w0.cfg.phys);
    let year = w0.days_per_year() as f64 * DAY;
    assert!(p.seep * w0.map.side * 1e-3 * year / (p.porosity * area) < 2e-4);
    let mv = w0.chem.molar_volume(LIQUID, Phase::Liquid);
    let full = p.bed_porosity * p.bed_depth * area;
    let stands = |w: &World, i: usize| w.surface_z(i) + pcore::survey::env_of(w, i)[1];
    // The slope down to the steepest lower neighbour, as the sweep reads it.
    let slope = |w: &World, i: usize| {
        let (x, y) = ((i % gw) as i64, (i / gw) as i64);
        let mut s = 0.0f64;
        for (dx, dy) in [(-1i64, -1i64), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
            let (nx, ny) = (x + dx, y + dy);
            if nx >= 0 && ny >= 0 && nx < gw as i64 && ny < (n / gw) as i64 {
                let dh = stands(w, i) - stands(w, (ny * gw as i64 + nx) as usize);
                if dh > p.still {
                    s = s.max(dh / (w.map.side * if dx != 0 && dy != 0 { 2f64.sqrt() } else { 1.0 }));
                }
            }
        }
        s
    };
    // One liquid step with the store of `i` full and warm, with channels `hill` apart: the store and the ground liquid after.
    let run = |i: usize, hill: f64| {
        let mut w = worldgen::new_world(3, 16, 16);
        (w.cfg.evap, w.cfg.phys.hill) = (0.0, hill);
        let e = full / mv * (w.chem.subs[LIQUID].cp * 290.0 + w.chem.subs[LIQUID].lf);
        set_amount(&mut w, Node::new(Class::Deep, i), LIQUID, full / mv);
        set_heat(&mut w, Node::new(Class::Deep, i), e);
        let books = Books::now(&w);
        worldgen::testkit::flow_step(&mut w, LIQUID_STEP);
        books.assert_kept(&w, 1e-9);
        (w.store.amounts(Class::Deep)[LIQUID * n + i] * mv, liquid_m3(&w, i))
    };
    let (hi, _) = pair(&w0);
    let s = slope(&w0, hi);
    let want = full * -(-p.bed_seep * s * LIQUID_STEP / (p.bed_porosity * p.hill)).exp_m1();
    // The control: channels so far apart that none drain. What it loses is Darcy's flow across the cell, which goes
    // with what the store holds after the channels took their share.
    let ((d1, g1), (d0, g0)) = (run(hi, p.hill), run(hi, 1e15));
    let drained = full - d1 / (d0 / full);
    println!("slope {s:.3}: {drained:.0} m³ drained to the channels, the law says {want:.0}; Darcy across the cell {:.0}", full - d0);
    assert!(s > 1e-3 && (drained / want - 1.0).abs() < 0.02, "drained {drained} m³, want {want}");
    assert!(g1 - g0 < 0.05 * want, "{} m³ stayed in the cell", g1 - g0);
    // Under the deep liquid, level with its neighbours: nothing drains to channels.
    let under = (0..n).find(|&i| pcore::survey::env_of(&w0, i)[1] > 20.0 && slope(&w0, i) == 0.0).expect("a cell under deep liquid");
    let ((d1, _), (d0, _)) = (run(under, p.hill), run(under, 1e15));
    assert!((d1 - d0).abs() <= 1e-9 * d0, "{d1} against {d0}");
}

/// Drainage (`flow::drain`): liquid over the share `field` of the pores sinks into the bed's store, no more than it
/// holds (`bed_porosity` x `bed_depth`) nor `seep` a second; a full store on a high cell passes its liquid down the
/// slope, and none overfills. Matter and energy are kept.
#[test]
fn liquid_over_field_capacity_drains_into_the_bed_and_flows_down() {
    let mut w = worldgen::new_world(3, 16, 16);
    let (hi, lo) = pair(&w);
    let (n, area, p) = (w.mesh.cells().n(), w.mesh.cells().area()[0], w.cfg.phys);
    let (pores, full, mv) = (p.porosity * loose(&w, hi), p.bed_porosity * p.bed_depth * area, w.chem.molar_volume(LIQUID, Phase::Liquid));
    w.cfg.evap = 0.0;
    pour(&mut w, hi, 0.9 * pores);
    let books = Books::now(&w);
    worldgen::testkit::flow_step(&mut w, LIQUID_STEP);
    let deep = |w: &World, i: usize| w.store.amounts(Class::Deep)[LIQUID * n + i] * mv;
    // Down: what is over the field capacity, 0.3 of the pores (0.9 - 0.6), less what Darcy's flow took sideways.
    assert!(deep(&w, hi) > 0.1 * pores && deep(&w, hi) <= (0.3 * pores).min(full) * 1.000001, "{} m³ sank of {pores} m³ of pores", deep(&w, hi));
    assert!(liquid_m3(&w, hi) >= 0.95 * 0.6 * pores, "the loose ground kept {} of {pores} m³", liquid_m3(&w, hi));
    books.assert_kept(&w, 1e-9);
    // Across: the high cell's store, full and warm, drains to its neighbours; the low one's never holds more than it can.
    let e = full / mv * (w.chem.subs[LIQUID].cp * 290.0 + w.chem.subs[LIQUID].lf);
    set_amount(&mut w, Node::new(Class::Deep, hi), LIQUID, full / mv);
    set_heat(&mut w, Node::new(Class::Deep, hi), e);
    let books = Books::now(&w);
    for _ in 0..20 {
        worldgen::testkit::flow_step(&mut w, LIQUID_STEP);
        assert!(deep(&w, lo) <= full * 1.000001, "the low cell's store overfilled");
    }
    assert!(deep(&w, hi) < 0.9 * full, "the full store on the high cell did not drain: {} of {full} m³", deep(&w, hi));
    books.assert_kept(&w, 1e-9);
}

/// The stores of the bed flow from where they stand before the step, so no direction goes first: in one liquid step
/// the full store of a cell (among empty ones, with nothing sinking into them) reaches its neighbours and no further.
#[test]
fn a_store_reaches_only_its_neighbours_in_a_step() {
    let mut w = worldgen::new_world(3, 16, 16);
    w.cfg.phys.seep = 0.0;
    worldgen::testkit::flow_step(&mut w, LIQUID_STEP);
    let (n, gw, p) = (w.mesh.cells().n(), w.map.w, w.cfg.phys);
    let (x, y) = (gw / 2, w.map.h / 2);
    let i = y * gw + x;
    for j in 0..n {
        (0..w.chem.ns()).for_each(|k| set_amount(&mut w, Node::new(Class::Deep, j), k, 0.0));
        set_heat(&mut w, Node::new(Class::Deep, j), 0.0);
    }
    let (mv, sub) = (w.chem.molar_volume(LIQUID, Phase::Liquid), w.chem.subs[LIQUID].clone());
    let m = p.bed_porosity * p.bed_depth * w.mesh.cells().area()[0] / mv;
    set_amount(&mut w, Node::new(Class::Deep, i), LIQUID, m);
    set_heat(&mut w, Node::new(Class::Deep, i), m * (sub.cp * 290.0 + sub.lf));
    worldgen::testkit::flow_step(&mut w, LIQUID_STEP);
    let near = |j: usize| (j % gw).abs_diff(x) <= 1 && (j / gw).abs_diff(y) <= 1;
    let reached: Vec<usize> = (0..n).filter(|&j| j != i && w.store.amounts(Class::Deep)[LIQUID * n + j] > 0.0).collect();
    assert!(!reached.is_empty() && reached.iter().all(|&j| near(j)), "reached {reached:?} from {i}");
}

/// A solid that dissolves in the liquid (the fixed gas's mineral) is in the liquid of the pores, and moves with the
/// liquid that runs off the cell: the share of the mineral that leaves is the share of the liquid that leaves, down
/// the slope, and it is deposited nowhere as sediment. Liquid that fits in the pores does not run off and the mineral
/// stays with it; a dry cell keeps its mineral. Matter and energy are kept.
#[test]
fn a_dissolved_solid_moves_with_the_liquid_that_runs_off_and_stays_with_the_pores() {
    let base = worldgen::new_world(3, 16, 16);
    let (n, top) = (base.mesh.cells().n(), top(&base));
    let pores = pore_mol(&base, top);
    // A world with `x` times the pores' worth of liquid on the highest cell, and 1e6 mol of the mineral there.
    let scene = |x: f64| {
        let mut w = base.clone();
        let t = w.t_ground(top).max(w.chem.subs[LIQUID].melt + 5.0);
        put(&mut w, LIQUID, top, x * pores);
        hold_ground(&mut w, top, t);
        let e = 1e6 * w.chem.subs[MINERAL].cp * w.t_ground(top);
        w.cross((Boundary::Outside, Node::ground(top)), [(MINERAL, 1e6)], e);
        w
    };
    // (No evaporation and no vapour in the air to rain, which would add or take liquid and not the mineral: the liquid
    // there is all there is, and what it does is the pores' and the slope's.)
    let run = |x: f64, steps: usize| {
        let mut w = scene(x);
        w.cfg.evap = 0.0;
        let na = w.mesh.level(AIR).n();
        (0..na).for_each(|j| set_amount(&mut w, Node::air(j), LIQUID, 0.0));
        let books = Books::now(&w);
        let liquid0 = w.store.amounts(Class::Ground)[LIQUID * n + top];
        for _ in 0..steps {
            worldgen::testkit::flow_step(&mut w, DT);
        }
        books.assert_kept(&w, 1e-9);
        let downstream: f64 = (0..n).filter(|&i| i != top).map(|i| w.store.amounts(Class::Ground)[MINERAL * n + i]).sum::<f64>() + w.store.amounts(Class::Deep).get(MINERAL * n..(MINERAL + 1) * n).map_or(0.0, |v| v.iter().sum::<f64>());
        assert!((downstream + w.store.amounts(Class::Ground)[MINERAL * n + top] - 1e6).abs() < 1e-6 * 1e6, "the mineral was lost: {} mol", downstream + w.store.amounts(Class::Ground)[MINERAL * n + top]);
        (1.0 - w.store.amounts(Class::Ground)[MINERAL * n + top] / 1e6, 1.0 - w.store.amounts(Class::Ground)[LIQUID * n + top] / liquid0)
    };
    // Half the pores' worth, under their field capacity: the ground holds all of it against gravity, and the mineral.
    let (m_in, l_in) = run(0.5, 8);
    assert!(m_in.abs() < 1e-12 && l_in.abs() < 1e-12, "liquid that fits in the pores moved: mineral {m_in}, liquid {l_in}");
    // Twice: the half over the pores runs off, and the mineral with it, in the same share.
    let (m_out, l_out) = run(2.0, 8);
    println!("pores half full: {:.2}% of the mineral and {:.2}% of the liquid moved; twice full: {:.1}% of the mineral and {:.1}% of the liquid left the cell in 8 steps", 100.0 * m_in, 100.0 * l_in, 100.0 * m_out, 100.0 * l_out);
    assert!(l_out > 0.3 && m_out > 0.3, "only {m_out} of the mineral and {l_out} of the liquid ran off");
    assert!((m_out / l_out - 1.0).abs() < 0.05, "the mineral {m_out} and the liquid {l_out} ran off in different shares");
    // No liquid: nothing carries it, but for what Darcy's flow takes of the film a dry cell holds.
    let mut dry = scene(0.0);
    dry.cfg.evap = 0.0;
    worldgen::testkit::flow_step(&mut dry, DT);
    assert!(dry.g(MINERAL, top) > 0.99e6, "the mineral moved on dry ground: {}", dry.g(MINERAL, top));
}

/// How far below its lowest neighbour the pit of the evaporation test is dug, m.
const PIT_M: f64 = 10.0;

/// The dissolved mineral stays where it is when its liquid evaporates (it is no gas): a thin film on warm ground in
/// dry air, in a pit dug in dry land (lower than all its neighbours, so nothing runs off it).
#[test]
fn a_dissolved_mineral_stays_when_its_liquid_evaporates() {
    let mut w = worldgen::new_world(3, 16, 16);
    let (n, na, ns) = (w.mesh.cells().n(), w.mesh.level(AIR).n(), w.chem.ns());
    let near = |w: &World, i: usize| pcore::space::square::DIRS.iter().map(|&d| w.map.step(i, d).map(|j| w.surface_z(j))).collect::<Option<Vec<f64>>>();
    let dry = |w: &World, i: usize| w.store.amounts(Class::Ground)[LIQUID * n + i] == 0.0 && w.surface_z(i) > 20.0;
    let i = (0..n).find(|&i| dry(&w, i) && near(&w, i).is_some_and(|z| z.iter().all(|&z| z > 20.0))).expect("dry land among dry land");
    let (area, lowest) = (w.mesh.cells().area()[i], near(&w, i).unwrap().into_iter().fold(f64::INFINITY, f64::min));
    let bed: f64 = (0..ns).map(|k| w.store.amounts(Class::Bed)[k * n + i] * w.chem.molar_volume(k, Phase::Solid)).sum::<f64>() / area;
    let keep = 1.0 - (w.surface_z(i) - lowest + PIT_M) / bed;
    for k in 0..ns {
        let mol = keep * w.store.amounts(Class::Bed)[k * n + i];
        set_amount(&mut w, Node::new(Class::Bed, i), k, mol);
    }
    assert!(near(&w, i).unwrap().iter().all(|&z| z > w.surface_z(i)), "a pit");
    // 5 cm of the liquid.
    let film = 0.05 * area / w.chem.molar_volume(LIQUID, Phase::Liquid);
    set_amount(&mut w, Node::ground(i), LIQUID, film);
    set_amount(&mut w, Node::ground(i), DISSOLVED, 1e3);
    hold_ground(&mut w, i, 330.0);
    (0..na).for_each(|j| set_amount(&mut w, Node::air(j), LIQUID, 0.0));
    let (liquid0, mineral0) = (w.store.amounts(Class::Ground)[LIQUID * n + i], w.store.amounts(Class::Ground)[DISSOLVED * n + i]);
    for _ in 0..4 {
        worldgen::testkit::flow_step(&mut w, DT);
    }
    assert!(w.store.amounts(Class::Ground)[LIQUID * n + i] < 0.99 * liquid0, "the liquid did not evaporate: {} of {liquid0}", w.store.amounts(Class::Ground)[LIQUID * n + i]);
    assert_eq!(w.store.amounts(Class::Ground)[DISSOLVED * n + i], mineral0);
    assert_eq!(w.store.amounts(Class::Air)[DISSOLVED * na..(DISSOLVED + 1) * na].iter().sum::<f64>(), 0.0);
}

/// The diffusivity of the starting loose ground is the measured one: creep x softness x thickness = 3e-3 m²/yr
/// (hillslope soil diffusivities are 1e-3 to 1e-2 m²/yr), within a tenth.
#[test]
fn the_starting_ground_creeps_at_a_measured_diffusivity() {
    let w = worldgen::new_world(1, 16, 16);
    let cell = (0..w.mesh.cells().n()).find(|&i| w.surface_z(i) > 0.0).expect("land");
    let (h, _, soft) = ground_of(&w, cell);
    let d = w.cfg.creep * soft * h * w.days_per_year() as f64 * DAY;
    println!("loose ground {h:.2} m thick, softness {soft:.2}: diffusivity {d:.2e} m2/yr");
    assert!(h > 0.5 && h < 1.5, "{h} m of loose ground");
    assert!(d > 2.7e-3 && d < 3.3e-3, "{d} m2/yr");
}

/// The two rates of running liquid are the measured yield: the sediment it carries to the low ground, about 15 to 20
/// Gt a year, is 7e9 m³, and the potential energy of the runoff, 4e13 m³ a year falling 840 m, is 3.4e20 J: 2e-11 m³
/// per J. A bed of the starting rock breaks at the same.
#[test]
fn liquid_carries_and_breaks_at_the_measured_yield_per_joule() {
    let w = worldgen::new_world(1, 16, 16);
    let yield_per_j = 7e9 / (4e13 * 840.0 * 1000.0 * 9.81);
    println!("measured yield {yield_per_j:.2e} m3/J; carry {:.1e}, wear {:.1e}", w.cfg.carry, w.cfg.wear);
    assert!((w.cfg.carry / yield_per_j - 1.0).abs() < 0.2, "carry {}", w.cfg.carry);
    // The bed breaks at wear / strength for each substance, by its share of the bed's volume.
    let (n, area) = (w.mesh.cells().n(), w.mesh.cells().area()[0]);
    let cell = (0..n).find(|&i| w.surface_z(i) > 0.0).expect("land");
    let (mut vol, mut inv) = (0.0, 0.0);
    for k in (0..w.chem.ns()).filter(|&k| w.store.amounts(Class::Bed)[k * n + cell] > 0.0) {
        let v = w.store.amounts(Class::Bed)[k * n + cell] * w.chem.molar_volume(k, Phase::Solid) / area;
        vol += v;
        inv += v / w.chem.subs[k].strength.max(0.05);
    }
    let per_j = w.cfg.wear * inv / vol;
    println!("bed breaks at {per_j:.2e} m3/J");
    assert!((per_j / yield_per_j - 1.0).abs() < 0.2, "{per_j}");
}

/// Liquid runs down the cells in the order of their heads in one pass: rain on bare land (no pores, nothing sinking)
/// more than `FAR` cells from the low ground under 0 m reaches it in one liquid step, where a step from cell to cell
/// would take it no more than one cell and bring it none; the pits of the island keep the rest (they fill and spill over
/// the following steps). Matter and energy are kept.
#[test]
fn rain_crosses_the_land_in_one_step() {
    const FAR: usize = 3;
    let low_after = |rain: f64| {
        let mut w = worldgen::new_world(1, 32, 32);
        (w.cfg.phys.porosity, w.cfg.phys.seep, w.cfg.evap) = (0.0, 0.0, 0.0);
        let (n, area) = (w.mesh.cells().n(), w.mesh.cells().area()[0]);
        let low: Vec<usize> = (0..n).filter(|&i| w.surface_z(i) < 0.0).collect();
        let apart = |i: usize, j: usize| (w.map.xy(i).0.abs_diff(w.map.xy(j).0)).max(w.map.xy(i).1.abs_diff(w.map.xy(j).1));
        let far: Vec<usize> = (0..n).filter(|&i| low.iter().all(|&j| apart(i, j) > FAR)).collect();
        far.iter().for_each(|&i| pour(&mut w, i, rain * area));
        let books = Books::now(&w);
        worldgen::testkit::flow_step(&mut w, LIQUID_STEP);
        books.assert_within(&w, 1e-12, 1e-9);
        (low.iter().map(|&i| liquid_m3(&w, i)).sum::<f64>(), rain * area * far.len() as f64)
    };
    let ((wet, fell), (dry, _)) = (low_after(0.05), low_after(0.0));
    println!("{:.1}% of the rain on the land far from the low ground reached it in one step", 100.0 * (wet - dry) / fell);
    assert!(fell > 0.0 && wet - dry > 0.1 * fell, "{} of {fell} m³ reached the low ground", wet - dry);
}

/// Moving liquid works on its bed at every liquid step: once rain runs off (the pores of the land are full after
/// three months), each of the next eight liquid steps breaks bonded ground on the land.
#[test]
fn liquid_wears_its_bed_at_every_liquid_step() {
    let mut w = worldgen::new_world(1, 16, 16);
    (0..3).for_each(|_| pcore::step(&mut w));
    let n = w.mesh.cells().n();
    let land: Vec<usize> = (0..n).filter(|&i| w.surface_z(i) > 0.0).collect();
    let bed = |w: &World| land.iter().map(|&i| (0..w.chem.ns()).map(|k| w.store.amounts(Class::Bed)[k * n + i]).sum::<f64>()).sum::<f64>();
    for step in 0..8 {
        let before = bed(&w);
        worldgen::testkit::flow_step(&mut w, LIQUID_STEP);
        assert!(bed(&w) < before, "step {step}: the bed on land kept {} of {before} mol", bed(&w));
    }
}

/// A small island, ten years: the land loses rock (the rock-making solids of its loose ground and bed, over its
/// cells) at the real rate of denudation, 0.01 to 1 mm a year.
#[test]
#[ignore = "slow: ten years of a 32 x 32 world"]
fn an_island_loses_ground_at_a_real_rate() {
    let mut w = worldgen::new_world(1, 32, 32);
    let land: Vec<usize> = (0..w.mesh.cells().n()).filter(|&i| w.surface_z(i) > 0.0).collect();
    let rock = |w: &World| land.iter().map(|&i| ground_of(w, i)).map(|g| g.0 + g.1).sum::<f64>() / land.len() as f64;
    let before = rock(&w);
    let years = 10;
    for _ in 0..w.cfg.months_per_year * years {
        pcore::step(&mut w);
    }
    let rate = (before - rock(&w)) / years as f64 * 1000.0;
    println!("{} land cells lost {rate:.4} mm a year of rock", land.len());
    assert!(rate > 0.003 && rate < 1.0, "{rate} mm a year");
}

/// Cover per area in the two cells of the lane scene, kg/m².
const COVER: f64 = 20.0;

/// Two land cells side by side in a warm world, each under `COVER` of light-eaters, and an eater in the first: the
/// world, the two cells and the eater's slot.
fn lane_scene() -> (World, usize, usize, usize) {
    let (chem, init) = worldgen::chemistry0();
    let cfg = pcore::Config { lat_south: 5.0, lat_north: 10.0, ..Default::default() };
    let mut w = worldgen::terrain::build(3, worldgen::terrain::island(12, 8), chem, &init, cfg);
    pcore::step(&mut w);
    let n = w.mesh.cells().n();
    let a = (0..n - 1).find(|&i| w.life.env[i][2] > 0.0 && w.life.env[i + 1][2] > 0.0 && i % 12 < 11).expect("two land cells");
    let brain = Brain { verbs: [OFF; brain::VERBS + brain::OVERBS], seed: 3, ..Brain::neutral() };
    let line = Line::new(&mut w, &brain, 0);
    for cell in [a, a + 1] {
        let mut p = line.founder(&mut w, 0, cell);
        // The node scaled to the cover wanted (stores are totals).
        let k = COVER * w.mesh.cells().area()[0] / p.mass(&w.chem);
        p.store.iter_mut().for_each(|x| *x *= k);
        p.count *= k;
        w.add(p, 1);
    }
    let m = line.founder(&mut w, 1, a);
    let slot = w.add(m, 1);
    (w, a, a + 1, slot)
}

/// Edges remember use: a body going back and forth through cover between two cells clears the ground along their
/// edge (it gets cheaper to cross), and the cover grows back over decades once the moving stops. A body too weak to
/// give the push of a mover does not move.
#[test]
fn going_back_and_forth_clears_the_edge_and_cover_closes_it() {
    let (mut w, a, b, slot) = lane_scene();
    let link = w.mesh.cells().link(a, b).expect("the cells are neighbours");
    pcore::survey::survey(&mut w);
    assert!((lanes::eased(&w, link) - 1.0).abs() < 1e-9);
    let mut crossed = 0;
    // Moving needs force: with the strength of the starting light-eaters (0.05: a force of 0.025 of its weight) the
    // body cannot give the push of a mover (`bump`, 0.05 of its weight) and stays; with its own (0.6) it goes.
    let mut weak = w.life.bodies[slot].clone();
    weak.genes[body::STRENGTH] = 0.05;
    moved(&mut w, (a, slot), |w| choose::go(w, slot, &mut weak, b));
    assert_eq!(weak.cell as usize, a, "a body too weak to push walked");
    for t in 0..5000 {
        // Fuel for the trip each time (the scene is not about eating), with its heat at the body's temperature.
        let mut m = std::mem::take(&mut w.life.bodies[slot]);
        m.e += (1e3 - m.store[RICH]) * w.chem.subs[RICH].cp * m.temp(&w.chem);
        m.store[RICH] = 1e3;
        let (from, to) = if t % 2 == 0 { (a, b) } else { (b, a) };
        assert_eq!(m.cell as usize, from);
        moved(&mut w, (from, slot), |w| choose::go(w, slot, &mut m, to));
        crossed += (m.cell as usize == to) as u32;
        w.life.bodies[slot] = m;
        pcore::survey::survey(&mut w);
        if lanes::eased(&w, link) > 3.0 {
            break;
        }
    }
    let worn = lanes::eased(&w, link);
    assert!(crossed > 10 && worn > 3.0, "{crossed} crossings, edge {worn} times easier");
    // Moving stops: the cover grows back into the cleared ground over some decades.
    let year = 360.0 * DAY;
    let mut left = vec![worn];
    for _ in 0..40 {
        lanes::regrow(&mut w, year);
        left.push(lanes::eased(&w, link));
    }
    assert!(left[1] > 1.2, "recovers slowly: {left:?}");
    assert!(left.windows(2).all(|p| p[1] <= p[0] + 1e-9) && left[40] < 1.01, "and fully: {left:?}");
}

/// A small globe without life (8 rows, from pole to pole) runs 10 years whole and keeps its books, with liquid, heat
/// and wind crossing the wrap of its rows.
#[test]
fn a_globe_keeps_its_books_round_the_wrap() {
    let mut w = worldgen::terrain::new_globe(1, 8);
    let books = Books::now(&w);
    for month in 0..10 * w.cfg.months_per_year {
        pcore::step(&mut w);
        assert_eq!(pcore::check::broken(&w), None, "month {month}");
    }
    books.assert_kept(&w, 1e-12);
}
