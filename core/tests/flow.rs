//! Law 1 (Flow): liquid, pores and drainage, heat and radiation, the air, gases dissolving, erosion, and the edges that use clears.

use pcore::chem::Phase;
use pcore::units::DAY;
use pcore::{body, brain, choose, lanes, World};
use worldgen::testkit::{loose_volume as loose, Books, Brain, Line, DISSOLVED, LIQUID, OFF};

const DT: f64 = 3.0 * 86_400.0;

/// What the pores of the loose ground of cell `i` hold, mol of liquid `k` (the porosity times the volume of the loose solids).
fn pore_mol(w: &pcore::World, i: usize, k: usize) -> f64 {
    let t = w.t_ground(i);
    let solids: f64 = (0..w.chem.ns()).filter(|&q| w.chem.phase(q, t) == pcore::chem::Phase::Solid).map(|q| w.g(q, i) * w.chem.molar_volume(q, pcore::chem::Phase::Solid)).sum();
    w.cfg.phys.porosity * solids / w.chem.molar_volume(k, pcore::chem::Phase::Liquid)
}

#[test]
fn liquid_runs_downhill() {
    let (_, init) = worldgen::chemistry0();
    let mut w = worldgen::new_world(3, 16, 16);
    let (n, k) = (w.grid.n(), init.low);
    let top = (0..n).max_by(|&a, &b| w.surface_z(a).total_cmp(&w.surface_z(b))).unwrap();
    let t = w.t_ground(top).max(w.chem.subs[k].melt + 5.0);
    // Liquid soaks into the pores of the loose ground first (15 Oct 2026): what runs off is what is over them.
    let pores = pore_mol(&w, top, k);
    w.ground[k * n + top] += 1.5 * pores;
    w.e_ground[top] = w.energy_ground(top, t);
    let books = Books::now(&w);
    pcore::flow::step(&mut w, DT);
    let left = w.ground[k * n + top];
    // (It keeps its field capacity, `field` of its pores; the rest ran off or sank into the bed below, `flow::drain`.)
    assert!(left < 1.1 * pores && left > 0.55 * pores, "the cell holds {} of its pores' worth", left / pores);
    books.assert_within(&w, 1e-12, 1e-9);
}

/// A hot node held at a melting point with little fusion enthalpy left: radiating for a whole
/// step at that temperature would take more heat than it holds. It leaves the plateau and cools
/// by its heat capacity, toward the balance with sunlight and the air, not down to the floor.
#[test]
fn hot_plateau_cools_but_not_to_the_floor() {
    let mut w = worldgen::new_world(3, 16, 16);
    let (n, ns) = (w.grid.n(), w.chem.ns());
    let k = (0..ns).find(|&k| (700.0..1000.0).contains(&w.chem.subs[k].melt)).expect("a substance melting near 850 K");
    let i = (0..n).max_by(|&a, &b| w.surface_z(a).total_cmp(&w.surface_z(b))).unwrap();
    (0..ns).for_each(|x| w.ground[x * n + i] = 0.0);
    w.ground[k * n + i] = w.grid.area();
    let melt = w.chem.subs[k].melt;
    w.e_ground[i] = w.energy_ground(i, melt) + 0.5 * w.ground[k * n + i] * w.chem.subs[k].lf;
    assert_eq!(w.thaw(i).1.map(|p| p.0), Some(k));
    let books = Books::now(&w);
    pcore::flow::step(&mut w, DT);
    let t = w.t_ground(i);
    assert!(t > 150.0 && t < melt, "{t} K after a step from {melt} K");
    assert_eq!(w.counts.clamped_heat, 0.0);
    books.assert_within(&w, 1e-12, 1e-9);
}

/// A salty pool freezes lower than a fresh one (`Chemistry::melt_in`): 1 mol of a salt in a kg of the common liquid
/// lowers its melting point by the textbook 1.86 K, and Flow holds each pool, half frozen, at its own melting point.
#[test]
fn salty_pool_freezes_lower() {
    let (_, init) = worldgen::chemistry0();
    let mut w = worldgen::new_world(3, 16, 16);
    let (n, ns, k) = (w.grid.n(), w.chem.ns(), init.low);
    let salt = (0..ns).find(|&j| j != k && w.chem.subs[j].melt < w.chem.subs[k].melt && w.chem.subs[j].boil > 1000.0).expect("a salt");
    let (fresh, salty) = (0, n - 1);
    for i in [fresh, salty] {
        (0..ns).for_each(|x| w.ground[x * n + i] = 0.0);
        w.ground[k * n + i] = 1e4 * w.grid.area() / w.chem.molar_volume(k, pcore::chem::Phase::Liquid);
    }
    w.ground[salt * n + salty] = w.ground[k * n + salty] * w.chem.subs[k].molar_mass;
    let tm = [fresh, salty].map(|i| w.melt(k, i));
    assert_eq!(tm[0], w.chem.subs[k].melt);
    let dt = (w.chem.subs[k].melt - tm[1]) / 1.86;
    assert!((dt - 1.0).abs() < 0.03, "{dt} of the textbook lowering");
    for (i, t) in [fresh, salty].into_iter().zip(tm) {
        w.e_ground[i] = w.energy_ground(i, t) + 0.5 * w.ground[k * n + i] * w.chem.subs[k].lf;
        let (at, plateau) = w.thaw(i);
        assert!(at == t && plateau.is_some_and(|(q, f)| q == k && (f - 0.5).abs() < 1e-9), "{at} K, {plateau:?}");
    }
    let books = Books::now(&w);
    pcore::flow::step(&mut w, DT);
    assert_eq!([fresh, salty].map(|i| w.scratch.temps().0[i]), tm);
    books.assert_within(&w, 1e-12, 1e-9);
}

#[test]
fn warm_air_spreads() {
    let mut cold = worldgen::new_world(3, 16, 16);
    let mut warm = cold.clone();
    let j = cold.grid.aw() + 1;
    warm.e_air[j] *= 1.2;
    let books = Books::now(&warm);
    let excess = warm.t_air(j) - cold.t_air(j);
    pcore::flow::step(&mut cold, DT);
    pcore::flow::step(&mut warm, DT);
    assert!(warm.t_air(j) - cold.t_air(j) < 0.5 * excess, "warm node kept its heat");
    assert!(warm.t_air(j + 1) - cold.t_air(j + 1) > 0.5, "neighbour did not warm");
    books.assert_within(&warm, 1e-12, 1e-9);
}

/// More heat-absorbing gas keeps warming the ground: under 240 W/m² of sunlight, with the air
/// following the ground by the lapse rate, the balance at optical depth 17 is clearly warmer than
/// at 9 (two seeds' late worlds; one slab at the air's mean temperature gave both the same).
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

/// The most soluble gas of the air dissolves in the liquid of the deepest node toward Henry's
/// share (here with a fast liquid side, so within an exchange), and part of it comes out when that
/// liquid is warmed by 20 K (a fifth at balance); matter and energy are kept.
#[test]
fn gas_dissolves_and_comes_out_when_warmed() {
    let (_, init) = worldgen::chemistry0();
    let mut w = worldgen::new_world(3, 16, 16);
    (w.cfg.dissolve, w.cfg.exchange_days) = (1.0, 3);
    let (n, na, c) = (w.grid.n(), w.grid.na(), &w.chem);
    let k = init.air.iter().map(|p| p.0).filter(|&k| c.subs[k].boil < 250.0).max_by(|&a, &b| c.subs[a].solubility.total_cmp(&c.subs[b].solubility)).unwrap();
    let i = (0..n).max_by(|&a, &b| w.ground[init.low * n + a].total_cmp(&w.ground[init.low * n + b])).unwrap();
    let books = Books::now(&w);
    // (Gases dissolve at every fourth exchange, for all the time since.)
    (0..4).for_each(|_| pcore::flow::step(&mut w, DT));
    // Dissolved per mol of liquid, against Henry's share under the gas's share of the reference pressure.
    let held = |w: &pcore::World| w.ground[k * n + i] / w.ground[init.low * n + i];
    let j = w.grid.air_of(i);
    let x = w.air[k * na + j] / (0..w.chem.ns()).map(|q| w.air[q * na + j]).sum::<f64>();
    let (h1, want) = (held(&w), w.chem.henry(k, w.t_ground(i), x * pcore::units::P_REF));
    assert!(h1 > 0.5 * want && h1 < 2.0 * want, "{h1} dissolved per mol, Henry's share {want}");
    let e = w.energy_ground(i, w.t_ground(i) + 20.0) - w.e_ground[i];
    w.e_ground[i] += e;
    w.ledger.imposed_energy += e;
    (0..4).for_each(|_| pcore::flow::step(&mut w, DT));
    assert!(held(&w) < 0.9 * h1, "{} dissolved per mol after warming, {h1} before", held(&w));
    books.assert_within(&w, 1e-12, 1e-9);
}

/// A solid that dissolves in the liquid (the nitrogen mineral, 15 Oct 2026) is in the liquid of the pores, and moves with the
/// liquid that runs off the cell: the share of the mineral that leaves is the share of the liquid that leaves, down the slope,
/// and it is deposited nowhere as sediment. Liquid that fits in the pores does not run off and the mineral stays with it; a dry
/// cell keeps its mineral. Matter and energy are kept.
#[test]
fn a_dissolved_solid_moves_with_the_liquid_that_runs_off_and_stays_with_the_pores() {
    let (_, init) = worldgen::chemistry0();
    let base = worldgen::new_world(3, 16, 16);
    let (n, k) = (base.grid.n(), init.low);
    let mineral = base.chem.minerals()[0].0;
    let top = (0..n).max_by(|&a, &b| base.surface_z(a).total_cmp(&base.surface_z(b))).unwrap();
    let pores = pore_mol(&base, top, k);
    // A world with `x` times the pores' worth of liquid on the highest cell, and 1e6 mol of the mineral there.
    let scene = |x: f64| {
        let mut w = base.clone();
        let t = w.t_ground(top).max(w.chem.subs[k].melt + 5.0);
        w.ground[k * n + top] += x * pores;
        w.e_ground[top] = w.energy_ground(top, t);
        let e = 1e6 * w.chem.subs[mineral].cp * w.t_ground(top);
        w.ground[mineral * n + top] += 1e6;
        w.e_ground[top] += e;
        w.chem.add_elems(mineral, 1e6, &mut w.ledger.imposed_elems);
        w.ledger.imposed_energy += e + 1e6 * w.chem.subs[mineral].hf;
        w
    };
    // (No evaporation and no vapour in the air to rain, which would add or take liquid and not the mineral: the liquid there is
    // all there is, and what it does is the pores' and the slope's.)
    let run = |x: f64, steps: usize| {
        let mut w = scene(x);
        w.cfg.evap = 0.0;
        let na = w.grid.na();
        w.air[k * na..(k + 1) * na].fill(0.0);
        let books = Books::now(&w);
        let liquid0 = w.ground[k * n + top];
        for _ in 0..steps {
            pcore::flow::step(&mut w, DT);
        }
        books.assert_kept(&w, 1e-9);
        let downstream: f64 = (0..n).filter(|&i| i != top).map(|i| w.ground[mineral * n + i]).sum::<f64>() + w.deep.get(mineral * n..(mineral + 1) * n).map_or(0.0, |v| v.iter().sum::<f64>());
        assert!((downstream + w.ground[mineral * n + top] - 1e6).abs() < 1e-6 * 1e6, "the mineral was lost: {} mol", downstream + w.ground[mineral * n + top]);
        (1.0 - w.ground[mineral * n + top] / 1e6, 1.0 - w.ground[k * n + top] / liquid0)
    };
    // Half the pores' worth: nothing runs off over the ground; what moves is Darcy's flow, a little (the highest cell of the map,
    // 600 m above its neighbours: 3.3% in 8 steps at the loam's `seep`, 0.9% at the old 1e-6), and the mineral goes with the
    // liquid in the same share.
    let (m_in, l_in) = run(0.5, 8);
    assert!(m_in < 0.1 && l_in < 0.1 && (m_in / l_in - 1.0).abs() < 0.05, "liquid that fits in the pores moved: mineral {m_in}, liquid {l_in}");
    // Twice: the half over the pores runs off, and the mineral with it, in the same share.
    let (m_out, l_out) = run(2.0, 8);
    println!("pores half full: {:.2}% of the mineral and {:.2}% of the liquid moved; twice full: {:.1}% of the mineral and {:.1}% of the liquid left the cell in 8 steps", 100.0 * m_in, 100.0 * l_in, 100.0 * m_out, 100.0 * l_out);
    assert!(l_out > 0.3 && m_out > 0.3, "only {m_out} of the mineral and {l_out} of the liquid ran off");
    assert!((m_out / l_out - 1.0).abs() < 0.05, "the mineral {m_out} and the liquid {l_out} ran off in different shares");
    // No liquid: nothing carries it.
    let mut dry = scene(0.0);
    dry.cfg.evap = 0.0;
    pcore::flow::step(&mut dry, DT);
    // (A dry cell of the highest ground keeps all but what Darcy's flow takes of the film it holds: under 1% in a step.)
    assert!(dry.ground[mineral * n + top] > 0.99e6, "the mineral moved on dry ground: {}", dry.ground[mineral * n + top]);
}

/// The salt moves with the liquid that runs off a high cell, by the liquid's share, and stays when
/// the liquid evaporates (it is no gas).
#[test]
fn the_salt_runs_with_the_liquid_and_stays_when_the_liquid_leaves() {
    let mut w = worldgen::new_world(3, 16, 16);
    let (n, na, area) = (w.grid.n(), w.grid.na(), w.grid.area());
    let top = (0..n).max_by(|&a, &b| w.surface_z(a).total_cmp(&w.surface_z(b))).unwrap();
    let t = w.t_ground(top).max(w.chem.subs[LIQUID].melt + 5.0);
    // (Liquid soaks into the pores of the loose ground first, 15 Oct 2026; what runs off is what is over them: three times their volume.)
    let pores: f64 = (0..w.chem.ns()).filter(|&k| w.chem.phase(k, w.t_ground(top)) == pcore::chem::Phase::Solid).map(|k| w.g(k, top) * w.chem.molar_volume(k, pcore::chem::Phase::Solid)).sum::<f64>() * w.cfg.phys.porosity / w.chem.molar_volume(LIQUID, pcore::chem::Phase::Liquid);
    let (liquid, salt) = (3.0 * pores, 1e5);
    (w.ground[LIQUID * n + top], w.ground[DISSOLVED * n + top]) = (liquid, salt);
    w.e_ground[top] = w.energy_ground(top, t);
    let all = |w: &pcore::World, k: usize| w.ground[k * n..(k + 1) * n].iter().sum::<f64>() + w.air[k * na..(k + 1) * na].iter().sum::<f64>() + w.deep.get(k * n..(k + 1) * n).map_or(0.0, |v| v.iter().sum::<f64>());
    let (salt0, liq0) = (all(&w, DISSOLVED), all(&w, LIQUID));
    pcore::flow::step(&mut w, 3.0 * DAY);
    let (left_liq, left_salt) = (w.ground[LIQUID * n + top], w.ground[DISSOLVED * n + top]);
    assert!(left_liq < 0.5 * liquid && left_salt < 0.5 * salt, "stayed: liquid {left_liq}, salt {left_salt}");
    // The same share leaves: the salt left over the liquid left is the start's, within a tenth.
    assert!(((left_salt / salt) / (left_liq / liquid) - 1.0).abs() < 0.1, "left {} of the liquid, {} of the salt", left_liq / liquid, left_salt / salt);
    assert!((all(&w, DISSOLVED) - salt0).abs() < 1e-9 * salt0 && (all(&w, LIQUID) - liq0).abs() < 1e-6 * liq0);
    // A thin film on warm ground in dry air: the liquid evaporates, the salt stays where it is.
    let mut w = worldgen::new_world(3, 16, 16);
    // (A pit of dry land, lower than all its neighbours: nothing runs off it.)
    let pit = |w: &pcore::World, i: usize| pcore::grid::DIRS.iter().all(|&d| w.grid.step(i, d).is_some_and(|j| w.surface_z(j) > w.surface_z(i)));
    let i = (0..n).find(|&i| w.ground[LIQUID * n + i] == 0.0 && w.surface_z(i) > 20.0 && pit(&w, i)).expect("a pit of dry land");
    (w.ground[LIQUID * n + i], w.ground[DISSOLVED * n + i]) = (0.05 * area / 0.018e-3 * 1.0, 1e3);
    w.e_ground[i] = w.energy_ground(i, 330.0);
    w.air[LIQUID * na..(LIQUID + 1) * na].fill(0.0);
    let (liq0, salt0) = (w.ground[LIQUID * n + i], w.ground[DISSOLVED * n + i]);
    for _ in 0..4 {
        pcore::flow::step(&mut w, 3.0 * DAY);
    }
    assert!(w.ground[LIQUID * n + i] < 0.99 * liq0, "the liquid did not evaporate: {} of {liq0}", w.ground[LIQUID * n + i]);
    assert_eq!(w.ground[DISSOLVED * n + i], salt0);
    assert_eq!(w.air[DISSOLVED * na..(DISSOLVED + 1) * na].iter().sum::<f64>(), 0.0);
}

#[test]
fn take_heat_never_takes_a_node_below_the_floor() {
    let mut w = worldgen::new_world(1, 32, 32);
    let (i, e0) = (5, w.e_ground[5]);
    let floor = w.heat_cap_ground(i).max(1.0) * pcore::units::T_FLOOR;
    assert_eq!(w.take_heat(i, 0.25 * e0), 0.25 * e0);
    assert_eq!(w.counts.clamped_heat, 0.0);
    // Asking for more than there is gives what is above the floor and counts the rest.
    let took = w.take_heat(i, 10.0 * e0);
    assert!((w.e_ground[i] - floor).abs() <= 1e-9 * e0, "{} vs {floor}", w.e_ground[i]);
    assert!((took + w.counts.clamped_heat - 10.0 * e0).abs() <= 1e-9 * e0);
    assert!(pcore::check::broken(&w).is_none());
    // A negative request gives heat and is returned whole.
    assert_eq!(w.take_heat(i, -5.0), -5.0);
    // Heat that leaves with matter of most of the node's heat capacity may take the node down
    // to the floor of the heat capacity that stays.
    let (c, e1) = (0.9 * w.heat_cap_ground(i), w.e_ground[i]);
    let took = w.take_heat_with(i, e1, c);
    assert!((e1 - took - (w.heat_cap_ground(i) - c) * pcore::units::T_FLOOR).abs() <= 1e-9 * e1, "{took} of {e1}");
}

/// 15 days: one liquid step.
const LIQUID_STEP: f64 = 15.0 * 86_400.0;

/// Two land cells side by side, both warm and dry: the higher first.
fn pair(w: &World) -> (usize, usize) {
    let (n, gw) = (w.grid.n(), w.grid.w);
    let (a, b) = (0..n).filter(|i| i % gw + 1 < gw).map(|i| (i, i + 1)).find(|&(a, b)| w.surface_z(a) > 50.0 && w.surface_z(b) > 50.0 && w.t_ground(a) > 280.0 && w.t_ground(b) > 280.0).expect("two land cells");
    if w.surface_z(a) >= w.surface_z(b) { (a, b) } else { (b, a) }
}

/// Give cell `i` `vol` m³ of the world's liquid, warm.
fn pour(w: &mut World, k: usize, i: usize, vol: f64) {
    let n = w.grid.n();
    let t = w.t_ground(i).max(w.chem.subs[k].melt + 5.0);
    w.ground[k * n + i] += vol / w.chem.molar_volume(k, Phase::Liquid);
    w.e_ground[i] = w.energy_ground(i, t);
}

fn liquid_m3(w: &World, k: usize, i: usize) -> f64 {
    w.ground[k * w.grid.n() + i] * w.chem.molar_volume(k, Phase::Liquid)
}

/// The pore space of a cell is the porosity times the volume of its loose solids; the liquid in it does not stand:
/// what the bodies feel as liquid depth (`env_of`) is only the liquid over it.
#[test]
fn pore_space_is_the_porosity_times_the_loose_solids() {
    let (_, init) = worldgen::chemistry0();
    let mut w = worldgen::new_world(3, 16, 16);
    let (i, _) = pair(&w);
    let (pores, area) = (w.cfg.phys.porosity * loose(&w, i), w.grid.area());
    // The loose layer of the generated land is 1 m of a mix of grains, 0.4 of it pores (anchor: random packing, 0.36 to 0.44).
    assert!((pores / area / 0.4 - 1.0).abs() < 0.05, "{} m of pore depth under a metre of loose ground", pores / area);
    let depth = |w: &World| pcore::survey::env_of(w, i)[1];
    assert_eq!(depth(&w), 0.0);
    pour(&mut w, init.low, i, 0.5 * pores);
    assert_eq!(depth(&w), 0.0, "liquid that fits in the pores does not stand");
    pour(&mut w, init.low, i, pores);
    let stands = depth(&w) * area;
    assert!((stands / (0.5 * pores) - 1.0).abs() < 1e-6, "{stands} m³ stands, {} over the pores", 0.5 * pores);
}

/// Liquid soaks in up to the pores and stays (a sheet that runs off where the ground has none), and only what is over
/// them runs off. Matter and energy are kept.
#[test]
fn liquid_soaks_into_the_pores_and_only_the_rest_runs_off() {
    let (_, init) = worldgen::chemistry0();
    let k = init.low;
    let run = |porosity: f64, share: f64| {
        let mut w = worldgen::new_world(3, 16, 16);
        w.cfg.phys.porosity = porosity;
        let (i, _) = pair(&w);
        let pores = 0.4 * loose(&w, i);
        pour(&mut w, k, i, share * pores);
        let books = Books::now(&w);
        let v0 = liquid_m3(&w, k, i);
        pcore::flow::step(&mut w, LIQUID_STEP);
        books.assert_within(&w, 1e-12, 1e-9);
        (liquid_m3(&w, k, i) / v0, liquid_m3(&w, k, i) / pores)
    };
    // Half the pores' volume: it all stays (but for what the open air takes), where the same sheet ran off before.
    let (kept, _) = run(0.4, 0.5);
    let (kept_old, _) = run(0.0, 0.5);
    assert!(kept > 0.7, "the ground kept {kept} of its liquid");
    assert!(kept_old < 0.1, "with no pores {kept_old} of it stayed");
    // Twice the pores' volume: the rest ran off, and the pores keep their field capacity (`field`, 0.6) as the rest sinks.
    let (_, full) = run(0.4, 2.0);
    assert!(full < 1.05 && full > 0.55, "the ground holds {full} of its pore volume after the excess ran off");
}

/// Darcy: the pore liquid of a cell flows to a neighbour whose liquid table is lower, at `seep` x the slope x the thickness
/// it fills x the width, in the time. With `seep` at 0 the neighbour gets none; the flow goes with `seep`, with the
/// liquid table's slope and with the thickness; a level table does not flow; matter and energy are kept.
#[test]
fn pore_liquid_flows_down_the_liquid_table_at_darcys_rate() {
    let (_, init) = worldgen::chemistry0();
    let k = init.low;
    // One liquid step with the liquid of cell `i` half filling its pores and `j` dry: what `j` gained, m³, with `seep`.
    let gained = |seep: f64, share: f64| {
        let mut w = worldgen::new_world(3, 16, 16);
        let (i, j) = pair(&w);
        w.cfg.phys.seep = seep;
        // (No evaporation, which would lower the liquid table before the liquid flows, and no channels inside the cell.)
        (w.cfg.evap, w.cfg.phys.hill) = (0.0, 1e15);
        let pores = w.cfg.phys.porosity * loose(&w, i);
        pour(&mut w, k, i, share * pores);
        let books = Books::now(&w);
        pcore::flow::step(&mut w, LIQUID_STEP);
        books.assert_within(&w, 1e-12, 1e-9);
        let (area, depth) = (w.grid.area(), loose(&w, i) / w.grid.area());
        // The liquid table of `i` stood at its bottom plus the depth the liquid fills; `j` was dry on its own bottom.
        let (zi, zj) = (w.surface_z(i), w.surface_z(j));
        let (bi, bj) = (zi - depth, zj - loose(&w, j) / area);
        let ti = bi + share * depth;
        let dh = ti - bj;
        let b = ti - bi.max(bj);
        (liquid_m3(&w, k, j), dh, b)
    };
    let (none, ..) = gained(0.0, 0.5);
    let (some, dh, b) = gained(1e-6, 0.5);
    let w = worldgen::new_world(3, 16, 16);
    let want = 1e-6 * b * w.grid.cell_m * dh / w.grid.cell_m * LIQUID_STEP;
    println!("slope {dh:.2} m over one cell, thickness {b:.3} m: Darcy {want:.3} m³, gained {:.3} m³", some - none);
    assert!(dh > 0.0 && b > 0.0, "the table of the wet cell is higher: {dh} {b}");
    assert!(((some - none) / want - 1.0).abs() < 0.05, "gained {} m³, Darcy says {want}", some - none);
    // Ten times the conductivity, ten times the flow.
    let (more, ..) = gained(1e-5, 0.5);
    assert!(((more - none) / (some - none) / 10.0 - 1.0).abs() < 0.05, "x10 gave x{}", (more - none) / (some - none));
    // More liquid, a thicker flow: the thickness that carries it, and the slope, are bigger.
    let (full, dh2, b2) = gained(1e-6, 0.9);
    let want2 = 1e-6 * b2 * dh2 * LIQUID_STEP;
    assert!(b2 > b && dh2 > dh);
    assert!(((full - none) / want2 - 1.0).abs() < 0.05, "gained {} m³, Darcy says {want2}", full - none);
}

/// Darcy's law across a 2 km cell moves under 0.02% of its pores a year (`seep`, a metre of ground, a slope of 1 in
/// 1,000), but the liquid drains inside the cell to its channels, `hill` (250 m) from the divides: a full store of the
/// bed on a sloping cell loses a share 1 - exp(-`bed_seep` x slope x time / (`bed_porosity` x `hill`)) of it in a step,
/// which runs off (it does not stay in the cell); under deep standing liquid (no slope) it loses none that way.
#[test]
fn liquid_drains_inside_a_cell_to_its_channels() {
    let (_, init) = worldgen::chemistry0();
    let k = init.low;
    let w0 = worldgen::new_world(3, 16, 16);
    let (n, gw, area, p) = (w0.grid.n(), w0.grid.w, w0.grid.area(), w0.cfg.phys);
    assert!(p.seep * w0.grid.cell_m * 1e-3 * 3.156e7 / (p.porosity * area) < 2e-4);
    let mv = w0.chem.molar_volume(k, Phase::Liquid);
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
                    s = s.max(dh / (w.grid.cell_m * if dx != 0 && dy != 0 { 2f64.sqrt() } else { 1.0 }));
                }
            }
        }
        s
    };
    // One liquid step with the store of `i` full and warm, with channels `hill` apart: the store and the ground liquid after.
    let run = |i: usize, hill: f64| {
        let mut w = worldgen::new_world(3, 16, 16);
        (w.cfg.evap, w.cfg.phys.hill) = (0.0, hill);
        if w.deep.len() != w.chem.ns() * n {
            (w.deep, w.e_deep) = (vec![0.0; w.chem.ns() * n], vec![0.0; n]);
        }
        w.deep[k * n + i] = full / mv;
        w.e_deep[i] = full / mv * (w.chem.subs[k].cp * 290.0 + w.chem.subs[k].lf);
        let books = Books::now(&w);
        pcore::flow::step(&mut w, LIQUID_STEP);
        books.assert_kept(&w, 1e-9);
        (w.deep[k * n + i] * mv, liquid_m3(&w, k, i))
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
    let (_, init) = worldgen::chemistry0();
    let mut w = worldgen::new_world(3, 16, 16);
    let (k, (hi, lo)) = (init.low, pair(&w));
    let (n, area, p) = (w.grid.n(), w.grid.area(), w.cfg.phys);
    let (pores, full, mv) = (p.porosity * loose(&w, hi), p.bed_porosity * p.bed_depth * area, w.chem.molar_volume(k, Phase::Liquid));
    w.cfg.evap = 0.0;
    pour(&mut w, k, hi, 0.9 * pores);
    let books = Books::now(&w);
    pcore::flow::step(&mut w, LIQUID_STEP);
    let deep = |w: &World, i: usize| w.deep[k * n + i] * mv;
    // Down: what is over the field capacity, 0.3 of the pores (0.9 - 0.6), less what Darcy's flow took sideways.
    assert!(deep(&w, hi) > 0.1 * pores && deep(&w, hi) <= (0.3 * pores).min(full) * 1.000001, "{} m³ sank of {pores} m³ of pores", deep(&w, hi));
    assert!(liquid_m3(&w, k, hi) >= 0.95 * 0.6 * pores, "the loose ground kept {} of {pores} m³", liquid_m3(&w, k, hi));
    books.assert_kept(&w, 1e-9);
    // Across: the high cell's store, full and warm, drains to its neighbours; the low one's never holds more than it can.
    w.deep[k * n + hi] = full / mv;
    w.e_deep[hi] = full / mv * (w.chem.subs[k].cp * 290.0 + w.chem.subs[k].lf);
    let books = Books::now(&w);
    for _ in 0..20 {
        pcore::flow::step(&mut w, LIQUID_STEP);
        assert!(deep(&w, lo) <= full * 1.000001, "the low cell's store overfilled");
    }
    assert!(deep(&w, hi) < 0.9 * full, "the full store on the high cell did not drain: {} of {full} m³", deep(&w, hi));
    books.assert_kept(&w, 1e-9);
}

/// Thickness of rock-making solids (those that melt above 1000 K, but body matter, which chars before it melts) in the loose ground
/// of cell `i` and bed below it, m, and the mean softness (1 / (hardness + 0.05)) of the loose ones, by volume. Counting the dead
/// matter that piles up on the land once life starts made the first decade of a run look slow (0.02 mm a year or less, not 0.08).
fn ground_of(w: &World, i: usize) -> (f64, f64, f64) {
    let (n, area) = (w.grid.n(), w.grid.area());
    let (mut loose, mut bed, mut soft) = (0.0, 0.0, 0.0);
    for k in (0..w.chem.ns()).filter(|&k| w.chem.subs[k].melt > 1000.0 && Some(k) != w.chem.matter()) {
        let v = w.chem.molar_volume(k, Phase::Solid);
        loose += w.ground[k * n + i] * v / area;
        bed += w.bed[k * n + i] * v / area;
        soft += w.ground[k * n + i] * v / area / (w.chem.subs[k].hardness + 0.05);
    }
    (loose, bed, soft / loose.max(1e-300))
}

/// The diffusivity of the starting loose ground is the measured one: creep x softness x thickness = 3e-3 m²/yr (hillslope soil
/// diffusivities are 1e-3 to 1e-2 m²/yr), within a tenth.
#[test]
fn the_starting_ground_creeps_at_a_measured_diffusivity() {
    let w = worldgen::new_world(1, 16, 16);
    let cell = (0..w.grid.n()).find(|&i| w.surface_z(i) > 0.0).expect("land");
    let (h, _, soft) = ground_of(&w, cell);
    let d = w.cfg.creep * soft * h * w.days_per_year() as f64 * 86_400.0;
    println!("loose ground {h:.2} m thick, softness {soft:.2}: diffusivity {d:.2e} m2/yr");
    assert!(h > 0.5 && h < 1.5, "{h} m of loose ground");
    assert!(d > 2.7e-3 && d < 3.3e-3, "{d} m2/yr");
}

/// The two rates of liquid are the measured yield: the sediment that running liquid carries to the low ground, about 15 to 20 Gt a year, is 7e9 m³, and the potential
/// energy of their runoff, 4e13 m³ a year falling 840 m, is 3.4e20 J: 2e-11 m³ per J. A bed of the starting rock (bond strength 0.7)
/// breaks at the same.
#[test]
fn liquid_carries_and_breaks_at_the_measured_yield_per_joule() {
    let w = worldgen::new_world(1, 16, 16);
    let yield_per_j = 7e9 / (4e13 * 840.0 * 1000.0 * 9.81);
    println!("measured yield {yield_per_j:.2e} m3/J; carry {:.1e}, wear {:.1e}", w.cfg.carry, w.cfg.wear);
    assert!((w.cfg.carry / yield_per_j - 1.0).abs() < 0.2, "carry {}", w.cfg.carry);
    // The bed breaks at wear / strength for each substance, by its share of the bed's volume.
    let (n, area) = (w.grid.n(), w.grid.area());
    let cell = (0..n).find(|&i| w.surface_z(i) > 0.0).expect("land");
    let (mut vol, mut inv) = (0.0, 0.0);
    for k in (0..w.chem.ns()).filter(|&k| w.bed[k * n + cell] > 0.0) {
        let v = w.bed[k * n + cell] * w.chem.molar_volume(k, Phase::Solid) / area;
        vol += v;
        inv += v / w.chem.subs[k].strength.max(0.05);
    }
    let per_j = w.cfg.wear * inv / vol;
    println!("bed breaks at {per_j:.2e} m3/J");
    assert!((per_j / yield_per_j - 1.0).abs() < 0.2, "{per_j}");
}

/// A small island, ten years: the land loses rock (the rock-making solids of its loose ground and bed, over its cells) at the real rate
/// of denudation, 0.01 to 1 mm a year, and not the 10 mm a year it lost when the numbers were first guesses.
#[test]
fn an_island_loses_ground_at_a_real_rate() {
    let mut w = worldgen::new_world(1, 32, 32);
    let land: Vec<usize> = (0..w.grid.n()).filter(|&i| w.surface_z(i) > 0.0).collect();
    let rock = |w: &World| land.iter().map(|&i| ground_of(w, i)).map(|g| g.0 + g.1).sum::<f64>() / land.len() as f64;
    let before = rock(&w);
    let years = 10;
    for _ in 0..12 * years {
        pcore::step(&mut w);
    }
    let rate = (before - rock(&w)) / years as f64 * 1000.0;
    println!("{} land cells lost {rate:.4} mm a year of rock", land.len());
    assert!(rate > 0.003 && rate < 1.0, "{rate} mm a year");
}

/// Cover per area in the two cells, kg/m².
const COVER: f64 = 20.0;

fn lane_scene() -> (World, usize, usize, usize) {
    let (chem, init) = worldgen::chemistry0();
    let cfg = pcore::Config { lat_south: 5.0, lat_north: 10.0, ..Default::default() };
    let mut w = worldgen::terrain::build(3, 12, 8, chem, &init, cfg);
    pcore::step(&mut w);
    let n = w.grid.n();
    let a = (0..n - 1).find(|&i| w.life.env[i][2] > 0.0 && w.life.env[i + 1][2] > 0.0 && i % 12 < 11).expect("two land cells");
    let brain = Brain { verbs: [OFF; brain::VERBS + brain::OVERBS], seed: 3, ..Brain::neutral() };
    let line = Line::new(&mut w, &brain, 0);
    for cell in [a, a + 1] {
        let mut p = line.founder(&mut w, 0, cell);
        // Scale the node to the cover wanted (stores are totals).
        let k = COVER * w.grid.area() / p.mass(&w.chem);
        p.store.iter_mut().for_each(|x| *x *= k);
        p.count *= k;
        w.life.add(p, w.day, 1);
    }
    let m = line.founder(&mut w, 1, a);
    let slot = w.life.add(m, w.day, 1);
    (w, a, a + 1, slot)
}

#[test]
fn going_back_and_forth_clears_the_edge_and_cover_closes_it() {
    let (mut w, a, b, slot) = lane_scene();
    pcore::survey::survey(&mut w);
    assert!((lanes::eased(&w, a, 0) - 1.0).abs() < 1e-9);
    let (rich, mut crossed) = (w.chem.life_roles().expect("roles").1, 0);
    // Moving needs force (review 7 fix 7): with the strength of the starting plants (0.05: a force of 0.025 of its weight)
    // the mover cannot give the push of a mover (`bump`, 0.05 of its weight) and stays; with its own (0.6) it goes, below.
    let mut weak = w.life.bodies[slot].clone();
    weak.genes[body::STRENGTH] = 0.05;
    choose::go(&mut w, slot, &mut weak, 0, b);
    assert_eq!(weak.cell as usize, a, "a body too weak to push walked");
    for t in 0..5000 {
        // Fuel for the trip is given each time (the lane_scene is not about eating),
        // given with its heat at the body's temperature, so the body does not cool.
        let mut m = std::mem::take(&mut w.life.bodies[slot]);
        m.e += (1e3 - m.store[rich]) * w.chem.subs[rich].cp * m.temp(&w.chem);
        m.store[rich] = 1e3;
        let (from, d, to) = if t % 2 == 0 { (a, 0, b) } else { (b, 4, a) };
        assert_eq!(m.cell as usize, from);
        choose::go(&mut w, slot, &mut m, d, to);
        crossed += (m.cell as usize == to) as u32;
        w.life.bodies[slot] = m;
        pcore::survey::survey(&mut w);
        if lanes::eased(&w, a, 0) > 3.0 {
            break;
        }
    }
    let worn = lanes::eased(&w, a, 0);
    assert!(crossed > 10 && worn > 3.0, "{crossed} crossings, edge {worn} times easier");
    // Moving stops: the cover grows back into the cleared ground over some decades.
    let year = 360.0 * 86_400.0;
    let mut left = vec![worn];
    for _ in 0..40 {
        lanes::regrow(&mut w, year);
        left.push(lanes::eased(&w, a, 0));
    }
    assert!(left[1] > 1.2, "recovers slowly: {left:?}");
    assert!(left.windows(2).all(|p| p[1] <= p[0] + 1e-9) && left[40] < 1.01, "and fully: {left:?}");
}
