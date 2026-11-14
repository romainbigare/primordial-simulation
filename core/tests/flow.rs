//! Law 1 (Flow): heat and radiation, the air, gases dissolving, liquid in the pores, drainage and runoff, what the
//! liquid carries, and erosion.

use pcore::chem::Phase;
use pcore::store::{Boundary, Class, Node};
use pcore::units::DAY;
use pcore::{World, AIR};
use worldgen::testkit::{flow_step, ground_of, hold_ground, loose_volume as loose, put, set_amount, set_heat, Books, DISSOLVED, LIQUID, MINERAL};

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

/// Two land cells side by side, both warm, away from the low basin: their own lowest ground and that of the cells round
/// them above 0 m, so the basin's liquid reaches none of them (liquid that one gains under the ground comes from the
/// other or from rain), and neither in a body of standing liquid (what runs off them leaves): the higher first. None
/// where the ground has no such two (`testkit::island_with`).
fn pair(w: &World) -> Option<(usize, usize)> {
    let (n, gw) = (w.mesh.cells().n(), w.map.w);
    let high = |i: usize| pcore::flow::lowest(w, i) > 0.0;
    let warm_land = |i: usize| w.surface_z(i) > 50.0 && w.t_ground(i) > 280.0 && high(i) && !w.in_body(i) && w.mesh.cells().around(i).1.iter().all(|&j| high(j as usize));
    let (a, b) = (0..n).filter(|i| i % gw + 1 < gw).map(|i| (i, i + 1)).find(|&(a, b)| warm_land(a) && warm_land(b))?;
    Some(if w.surface_z(a) >= w.surface_z(b) { (a, b) } else { (b, a) })
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

/// A hot node held at a melting point far above the air's with little fusion enthalpy left: radiating for a whole step
/// at that temperature would take more heat than it holds. It leaves the plateau and cools by its heat capacity, toward the
/// balance with sunlight and the air, not down to the floor.
#[test]
fn hot_plateau_cools_but_not_to_the_floor() {
    let mut w = worldgen::testkit::squares(3, 16, 16);
    let (ns, area) = (w.chem.ns(), w.mesh.cells().area()[0]);
    let k = (0..ns).find(|&k| (400.0..1000.0).contains(&w.chem.subs[k].melt)).expect("a substance melting far above the air");
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
    books.assert_within(&w, 1e-9, 1e-9);
}

/// A pool with a solute in it freezes lower than a pure one (`Chemistry::melt_in`): 1 mol of solute in a kg of the
/// liquid lowers its melting point by the textbook 1.86 K, and Flow holds each pool, half frozen, at its own melting
/// point. Each pool is shallower than the liquid's top layer (`h_mix`), so it is one node (laws.md 1.1).
#[test]
fn a_pool_with_a_solute_freezes_lower() {
    let mut w = worldgen::testkit::squares(3, 16, 16);
    let (n, ns, k) = (w.mesh.cells().n(), w.chem.ns(), LIQUID);
    let solute = (0..ns).find(|&j| j != k && w.chem.subs[j].melt < w.chem.subs[k].melt && w.chem.subs[j].boil > 1000.0).expect("a solute");
    let (pure, mixed) = (0, n - 1);
    let pool = 0.5 * w.cfg.phys.h_mix * w.mesh.cells().area()[0] / w.chem.molar_volume(k, Phase::Liquid);
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
    // An air step: some of each pool evaporates, and each is held at its melting point as the step leaves it (a liquid
    // step would run the liquid off its ice, which holds none).
    pcore::flow::state(&mut w, true);
    pcore::flow::air(&mut w, DT);
    pcore::flow::state(&mut w, false);
    for i in [pure, mixed] {
        let held = w.t_ground_last(i);
        assert!((held - w.melt(k, i)).abs() < MELT_HELD && w.thaw_last(i).1.is_some_and(|(q, _)| q == k), "{held} K, melting at {} K", w.melt(k, i));
    }
    assert!(w.t_ground_last(mixed) < w.t_ground_last(pure), "the pool with a solute is held lower");
    books.assert_within(&w, 1e-9, 1e-9);
}

/// How near its melting point Flow holds a pool after a step, K: the step's evaporation changes what is dissolved.
const MELT_HELD: f64 = 1e-3;

/// A ground node on its melting plateau that one long air step melts all of (under the sun, its ice a cover whose
/// melting point caps the surface) or freezes all of (in the dark under air 10% colder) leaves it within the step, on
/// each of the first `PLATEAU_CELLS` cells of dry land: no plateau is left and the books are kept. (How long it stays
/// on it and where it goes on from is the balance's own law, `air::radiation`'s test.)
#[test]
fn a_plateau_used_up_in_one_long_step_is_left_with_the_books_kept() {
    let base = worldgen::testkit::small_world();
    for &i in worldgen::testkit::dry_lands(&base).iter().take(PLATEAU_CELLS) {
        let mut w0 = base.clone();
        // (On flat ground, under the sun of its latitude.)
        w0.rise_south[i] = 0.0;
        let pool = PLATEAU_M * w0.mesh.cells().area()[i] / w0.chem.molar_volume(LIQUID, Phase::Liquid);
        for (sun, air, frozen) in [(1.0, 1.0, 0.9), (0.0, 0.9, 0.1)] {
            let mut w = w0.clone();
            // (No rain: what falls in a step is snow or rain by the mean air of the step.)
            (w.cfg.sun, w.cfg.rain_time) = (w.cfg.sun * sun, f64::INFINITY);
            for j in 0..w.mesh.level(AIR).n() {
                let e = air * w.store.heat(Class::Air)[j];
                set_heat(&mut w, Node::air(j), e);
            }
            set_amount(&mut w, Node::ground(i), LIQUID, pool);
            let e = w.energy_ground(i, w.melt(LIQUID, i)) - frozen * pool * w.chem.subs[LIQUID].lf;
            set_heat(&mut w, Node::ground(i), e);
            assert!(w.thaw(i).1.is_some(), "cell {i}, sun x{sun}: on the plateau");
            let books = Books::now(&w);
            pcore::flow::state(&mut w, true);
            pcore::flow::air(&mut w, DT);
            books.assert_within(&w, 1e-9, 1e-9);
            assert!(w.thaw(i).1.is_none(), "cell {i}, sun x{sun}: a plateau left at {} K", w.t_ground(i));
        }
    }
}

/// The depth of the liquid on the plateau, m, and the cells of dry land it is tried on.
const PLATEAU_M: f64 = 0.02;
const PLATEAU_CELLS: usize = 11;

/// A ground node over a liquid's boiling point boils of it only what its heat over that point pays: a deep pool 2 K
/// over boils a small share in an air step, keeps its books, and stays near the boiling point.
#[test]
fn a_hot_pool_boils_only_what_its_heat_pays() {
    let mut w = worldgen::testkit::squares(3, 16, 16);
    let (k, i) = (LIQUID, 0);
    let pool = 100.0 * w.mesh.cells().area()[i] / w.chem.molar_volume(k, Phase::Liquid);
    set_amount(&mut w, Node::ground(i), k, pool);
    let e = w.energy_ground(i, w.chem.subs[k].boil + 2.0);
    set_heat(&mut w, Node::ground(i), e);
    let books = Books::now(&w);
    pcore::flow::state(&mut w, true);
    pcore::flow::air(&mut w, DT);
    books.assert_within(&w, 1e-9, 1e-9);
    let (left, t) = (w.g(k, i) / pool, w.t_ground(i));
    assert!(left > 0.99 && t > w.chem.subs[k].boil - 5.0, "{left} of the pool left, at {t} K");
}

/// A land node far over a liquid's boiling point (`FAR_K`), its pool (`POOL_M` deep) most of its heat capacity, gives
/// its air no more heat than it holds, where boiling all the pool away with its own heat would take more (laws.md 0,
/// "No overdraw"): no heat is rounded away and the books are kept.
#[test]
fn a_pool_far_over_its_boiling_point_gives_no_more_than_it_holds() {
    const FAR_K: f64 = 9000.0;
    const POOL_M: f64 = 10.0;
    let ((mut w, i), k) = (worldgen::testkit::start(), LIQUID);
    let pool = POOL_M * w.mesh.cells().area()[i] / w.chem.molar_volume(k, Phase::Liquid);
    set_amount(&mut w, Node::ground(i), k, pool);
    let e = w.energy_ground(i, w.chem.subs[k].boil + FAR_K);
    set_heat(&mut w, Node::ground(i), e);
    let (books, rounded) = (Books::now(&w), w.counts.rounded[1]);
    pcore::flow::state(&mut w, true);
    pcore::flow::air(&mut w, DT);
    books.assert_within(&w, 1e-9, 1e-9);
    let lost = w.counts.rounded[1] - rounded;
    assert!(lost <= 1e-9 * e, "{lost:e} J of {e:e} J rounded away");
}

/// A melting snowpack on a slope (`SNOW_M` of the liquid's solid, half molten), on a cell flat inside (so its node is
/// one, its ground all under its meltwater or none of it, laws.md 1.1): Flow's phases are those of its heat after the
/// air's step, which melts more of it, and after the liquid's step, whose meltwater runs off down the slope and leaves
/// the ice (laws.md 1.2): what the world reads as molten and standing is what the heat holds, within the share the heat
/// of the lifts and falls of the matter booked after moves (its meltwater falls at most from the snow's top to its bed's
/// top or its lowest ground, the lower: g × molar mass × that fall / fusion enthalpy), and none stands there.
#[test]
fn meltwater_runs_off_and_the_snow_keeps_the_share_its_heat_holds() {
    const SNOW_M: f64 = 2.0;
    let (mut w, (i, _)) = worldgen::testkit::island_with(3, pair);
    worldgen::testkit::flatten(&mut w, i);
    let snow = SNOW_M * w.mesh.cells().area()[i] / w.chem.molar_volume(LIQUID, Phase::Liquid);
    set_amount(&mut w, Node::ground(i), LIQUID, snow);
    let e = w.energy_ground(i, w.melt(LIQUID, i)) - 0.5 * snow * w.chem.subs[LIQUID].lf;
    set_heat(&mut w, Node::ground(i), e);
    let bed = (0..w.chem.ns()).map(|k| w.b(k, i) * w.chem.molar_volume(k, Phase::Solid)).sum::<f64>() / w.mesh.cells().area()[i];
    let sub = &w.chem.subs[LIQUID];
    let fall = w.surface_z(i) - (w.base_z + bed).min(pcore::flow::lowest(&w, i));
    let share = w.cfg.gravity * sub.molar_mass * fall / sub.lf;
    let books = Books::now(&w);
    pcore::flow::state(&mut w, true);
    let molten = |w: &World| (w.thaw_last(i).1.expect("melting").1, w.thaw(i).1.expect("melting").1);
    pcore::flow::air(&mut w, DT);
    let (read, held) = molten(&w);
    assert!((read - held).abs() < share, "after the air's step: molten share {read} read, {held} held (within {share})");
    pcore::flow::liquid(&mut w, LIQUID_STEP);
    books.assert_within(&w, 1e-9, 1e-9);
    let ((read, held), stands) = (molten(&w), pcore::flow::means(&w, &[i]).unwrap()[0].stands);
    assert!((read - held).abs() < share && stands < 1e-9, "molten share {read} read, {held} held (within {share}); {stands} m stands");
}

/// A warmer air node gives its heat to its neighbours.
#[test]
fn warm_air_spreads() {
    let mut cold = worldgen::testkit::squares(3, 16, 16);
    let mut warm = cold.clone();
    let j = cold.map.w.div_ceil(cold.map.scale) + 1;
    set_heat(&mut warm, Node::air(j), 1.2 * cold.store.heat(Class::Air)[j]);
    let books = Books::now(&warm);
    let excess = warm.t_air(j) - cold.t_air(j);
    worldgen::testkit::flow_step(&mut cold, DT);
    worldgen::testkit::flow_step(&mut warm, DT);
    assert!(warm.t_air(j) - cold.t_air(j) < 0.5 * excess, "warm node kept its heat");
    assert!(warm.t_air(j + 1) - cold.t_air(j + 1) > 0.5, "neighbour did not warm");
    books.assert_within(&warm, 1e-9, 1e-9);
}

/// Heat spreads sideways through flat, even ground as diffusion does: on the bed of one land cell everywhere with no
/// loose ground, a hot spot keeps its excess heat while its variance grows by 2 D t along each axis, D = k / (rho c) of
/// the bed (backward Euler on an open lattice gives just that growth at any step length).
#[test]
fn a_hot_spot_spreads_through_the_ground_at_its_diffusivity() {
    let mut w = worldgen::testkit::squares(3, 16, 16);
    let (n, ns, g) = (w.mesh.cells().n(), w.chem.ns(), w.map.clone());
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
    (0..10).for_each(|_| worldgen::testkit::creep(&mut w, t / 10.0));
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
    books.assert_within(&w, 1e-9, 1e-9);
}

/// Heat crosses the border between cells of two sizes by the border's width over the distance across it: on even bed
/// everywhere with no loose ground, ground warmer to the east at a fixed rate (as much heat flows into each cell as
/// out) stays so in the cells away from the map's edges, in squares, in cells of 2 x 2 squares and across their
/// borders, while cells that read a square's width or the distance between centres would not. An even temperature
/// stays even.
#[test]
fn heat_crosses_the_border_between_cells_of_two_sizes_by_its_width() {
    const GRAD: f64 = 1e-3;
    // Squares from the north edge of the map to the south between cells of 2 x 2 squares: their borders run north to
    // south, across the warming.
    let island = worldgen::terrain::island(24, 16);
    let (scale, square) = (island.scale, island.square_depth());
    let map = island.by_blocks(|bx, _| if (8..16).contains(&(bx * scale)) { square } else { square - 1 });
    let mut w = worldgen::terrain::new_map(3, map.clone());
    let (n, ns, area) = (w.mesh.cells().n(), w.chem.ns(), w.mesh.cells().area().to_vec());
    let land = (0..n).find(|&i| w.surface_z(i) > 0.0 && area[i] == map.side * map.side).expect("land");
    // Per m² of the bed: heat capacity, volume, and volume times conductivity.
    let (mut cp, mut vol, mut kv) = (0.0, 0.0, 0.0);
    for k in 0..ns {
        let (b, v) = (w.store.amounts(Class::Bed)[k * n + land] / area[land], w.chem.molar_volume(k, Phase::Solid));
        (cp, vol, kv) = (cp + b * w.chem.subs[k].cp, vol + b * v, kv + b * v * w.chem.subs[k].conductivity);
        for (i, &a) in area.iter().enumerate() {
            set_amount(&mut w, Node::new(Class::Bed, i), k, b * a);
            set_amount(&mut w, Node::ground(i), k, 0.0);
        }
    }
    // A step short against the time heat takes to cross a square, and the heat that crosses a side in it, J.
    let x = |w: &World, i: usize| w.mesh.cells().centre()[i][0];
    let dt = 0.01 * map.side * map.side * cp / kv;
    let side_heat = kv / vol * w.cfg.skin_m * map.side * GRAD * dt;
    let away = |w: &World, i: usize| (4.0 * map.side..=20.0 * map.side).contains(&x(w, i));
    for even in [true, false] {
        for i in 0..n {
            let t = 300.0 + if even { 0.0 } else { GRAD * x(&w, i) };
            hold_ground(&mut w, i, t);
        }
        w.scratch = Default::default();
        let (books, before) = (Books::now(&w), w.store.heat(Class::Ground).to_vec());
        worldgen::testkit::creep(&mut w, dt);
        let off = (0..n).filter(|&i| even || away(&w, i)).map(|i| (w.store.heat(Class::Ground)[i] - before[i]).abs()).fold(0.0, f64::max);
        println!("even {even}: the most any cell changed, {:.3e} of the heat across a side", off / side_heat);
        assert!(off <= 1e-6 * side_heat, "a cell changed by {off:.3e} J, against {side_heat:.3e} J across a side");
        books.assert_within(&w, 1e-9, 1e-9);
    }
}

/// More heat-absorbing gas keeps warming the ground: under 240 W/m² of sunlight, with the air following the ground by
/// the lapse rate, the balance at optical depth 17 in every band is clearly warmer than at 9 (one slab at the air's
/// mean temperature would give both the same).
#[test]
fn more_gas_warms_with_no_ceiling() {
    let (chem, _) = worldgen::chemistry0();
    let ground = |x: f64| {
        let heat = pcore::flow::Heat::of([(x, 7600.0); pcore::chem::HEAT_BANDS], (0.0, 7600.0), (0.0065, 1e4), 1.0);
        let (mut lo, mut hi) = (150.0, 500.0);
        for _ in 0..60 {
            let t: f64 = (lo + hi) / 2.0;
            let up = (1.0 - heat.absorbs(&chem, t)) * pcore::units::SIGMA * t.powi(4) + heat.glow(&chem, t, pcore::flow::UP)[0];
            (lo, hi) = if up > 240.0 { (lo, t) } else { (t, hi) };
        }
        lo
    };
    let (t1, t9, t17) = (ground(1.5), ground(9.0), ground(17.0));
    assert!(t9 > t1 + 20.0 && t17 > t9 + 3.0, "{t1} {t9} {t17}");
}

/// The most soluble gas of the air dissolves in the liquid of the deepest node toward Henry's share (here with a fast
/// liquid side, so within an exchange), and part of it comes out when the ground is warmed by 20 K (a fifth at balance;
/// all of it, as the low basin mixes in a month); matter and energy are kept.
#[test]
fn gas_dissolves_and_comes_out_when_warmed() {
    let (_, init) = worldgen::chemistry0();
    let mut w = worldgen::testkit::squares(3, 16, 16);
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
    for c in 0..n {
        let t = w.t_ground(c) + 20.0;
        hold_ground(&mut w, c, t);
    }
    (0..4).for_each(|_| worldgen::testkit::flow_step(&mut w, DT));
    assert!(held(&w) < 0.9 * h1, "{} dissolved per mol after warming, {h1} before", held(&w));
    books.assert_within(&w, 1e-9, 1e-9);
}

/// The pore space of a cell is the porosity times the volume of its loose solids; the liquid in it does not stand:
/// what stands on the cell (`flow::means`, as Flow's state of the world now has it) is only the liquid over it.
#[test]
fn pore_space_is_the_porosity_times_the_loose_solids() {
    let (mut w, (i, _)) = worldgen::testkit::island_with(3, pair);
    let (pores, area) = (w.cfg.phys.porosity * loose(&w, i), w.mesh.cells().area()[i]);
    // The loose layer of the generated land is what the wear left (`Samples::loose`), the top `skin_m` of it at most,
    // 0.4 of it pores (random packing: 0.36 to 0.44).
    let (lat, skin) = ((w.cfg.lat_south, w.cfg.lat_north), w.cfg.skin_m);
    let start = w.terrain.over(w.terrain.seed, &w.map, w.map.leaves()[i], lat, |s, k| s.loose[k].min(skin));
    assert!((pores / area / (0.4 * start) - 1.0).abs() < 0.05, "{} m of pore depth under {start} m of loose ground", pores / area);
    let depth = |w: &mut World| {
        pcore::flow::state(w, true);
        pcore::flow::means(w, &[i]).expect("Flow's state")[0].stands
    };
    assert_eq!(depth(&mut w), 0.0);
    pour(&mut w, i, 0.5 * pores);
    assert_eq!(depth(&mut w), 0.0, "liquid that fits in the pores does not stand");
    pour(&mut w, i, pores);
    let stands = depth(&mut w) * area;
    assert!((stands / (0.5 * pores) - 1.0).abs() < 1e-6, "{stands} m³ stands, {} over the pores", 0.5 * pores);
}

/// On flat ground liquid soaks in up to the pores and stays (a sheet runs off where the ground has none), and only what
/// is over them runs off (on uneven ground its hollows fill first, and run off sooner). Matter and energy are kept.
#[test]
fn liquid_soaks_into_the_pores_and_only_the_rest_runs_off() {
    let (base, (i, _)) = worldgen::testkit::island_with(3, pair);
    let run = |porosity: f64, share: f64| {
        let mut w = base.clone();
        w.cfg.phys.porosity = porosity;
        worldgen::testkit::flatten(&mut w, i);
        let pores = 0.4 * loose(&w, i);
        pour(&mut w, i, share * pores);
        let books = Books::now(&w);
        let v0 = liquid_m3(&w, i);
        worldgen::testkit::flow_step(&mut w, LIQUID_STEP);
        books.assert_within(&w, 1e-9, 1e-9);
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

/// The liquid in the bed's store under cell `i`, m³.
fn store_m3(w: &World, i: usize) -> f64 {
    w.store.amounts(Class::Deep)[LIQUID * w.mesh.cells().n() + i] * w.chem.molar_volume(LIQUID, Phase::Liquid)
}

/// Months from a world's start to its summer.
const SUMMER_MONTHS: usize = 5;

/// Liquid moves once a month, at its start: the sink into the bed's store after each of the air's steps, then the rest
/// of the liquid. Liquid poured on a slope (four times what the pores hold, more over field capacity than the store has
/// room for) after a month's start stays on the ground, the store unchanged, until the next month: then the store under
/// it gains (and gives its neighbours their share of it) and the rest runs off.
#[test]
fn liquid_sinks_into_the_bed_and_runs_off_once_a_month() {
    let mut w = worldgen::testkit::squares(3, 16, 16);
    // Into the summer, so the poured liquid stays liquid.
    (0..SUMMER_MONTHS).for_each(|_| pcore::step(&mut w));
    pcore::day_with_month_at_start(&mut w);
    let (i, _) = pair(&w).expect("two warm land cells in the summer");
    let p = w.cfg.phys;
    let (pores, full) = (p.porosity * loose(&w, i), p.bed_porosity * p.bed_depth * w.mesh.cells().area()[i]);
    pour(&mut w, i, 4.0 * pores);
    let (v0, d0) = (liquid_m3(&w, i), store_m3(&w, i));
    assert!(v0 - p.field * pores > full - d0, "more over field capacity ({}) than the store's room ({})", v0 - p.field * pores, full - d0);
    (1..w.cfg.days_per_month).for_each(|_| pcore::day_with_month_at_start(&mut w));
    assert_eq!((liquid_m3(&w, i), store_m3(&w, i)), (v0, d0), "nothing moves inside the month");
    pcore::day_with_month_at_start(&mut w);
    let d = store_m3(&w, i);
    println!("the store {d0:.4e} -> {d:.4e} m³ (room {:.4e}), the ground {v0:.4e} -> {:.4e} m³", full - d0, liquid_m3(&w, i));
    assert!(d > d0, "the store gains: {d0} -> {d} m³");
    assert!(liquid_m3(&w, i) < 0.5 * v0, "{} of {v0} m³ stayed after the month's liquid", liquid_m3(&w, i));
}

/// Darcy: the pore liquid of a cell flows to a neighbour whose liquid table is lower, at `seep` x the slope x the
/// thickness it fills x the width, in the time: what the neighbour gains against the same world with none poured in the
/// cell (its other neighbours may hold the starting liquid). The flow goes with `seep`, with the liquid table's slope
/// and with the thickness; pores under their field capacity (`field`, 0.6) give none; matter and energy are kept.
#[test]
fn pore_liquid_flows_down_the_liquid_table_at_darcys_rate() {
    // One liquid step with the liquid of cell `i` filling the share `share` of its pores and `j` dry: what `j` gained,
    // m³, the drop of the liquid table and the thickness that carries the flow.
    let (base, (i, j)) = worldgen::testkit::island_with(3, pair);
    let gained = |seep: f64, share: f64| {
        let mut w = base.clone();
        w.cfg.phys.seep = seep;
        // (No evaporation, which would lower the liquid table before the liquid flows, no rain, which would fill the dry
        // cell's pores, no channels inside the cell, and a full store under `i`, so none of its liquid sinks before it
        // flows.)
        (w.cfg.evap, w.cfg.rain_time, w.cfg.phys.hill) = (0.0, f64::INFINITY, 1e15);
        let (p, mv) = (w.cfg.phys, w.chem.molar_volume(LIQUID, Phase::Liquid));
        let full = p.bed_porosity * p.bed_depth * w.mesh.cells().area()[i] / mv;
        set_amount(&mut w, Node::new(Class::Deep, i), LIQUID, full);
        let warm = full * (w.chem.subs[LIQUID].cp * 290.0 + w.chem.subs[LIQUID].lf);
        set_heat(&mut w, Node::new(Class::Deep, i), warm);
        let pores = w.cfg.phys.porosity * loose(&w, i);
        pour(&mut w, i, share * pores);
        let books = Books::now(&w);
        worldgen::testkit::flow_step(&mut w, LIQUID_STEP);
        books.assert_within(&w, 1e-9, 1e-9);
        let (area, depth) = (w.mesh.cells().area()[0], loose(&w, i) / w.mesh.cells().area()[0]);
        // The liquid table of `i` stood at its bottom plus the depth the liquid fills; `j` was dry on its own bottom.
        let (bi, bj) = (w.surface_z(i) - depth, w.surface_z(j) - loose(&w, j) / area);
        let ti = bi + share * depth;
        (liquid_m3(&w, j), ti - bj, ti - bi.max(bj))
    };
    let unpoured = |seep: f64| gained(seep, 0.0).0;
    let (some, dh, b) = gained(1e-6, 0.8);
    let (none, none10) = (unpoured(1e-6), unpoured(1e-5));
    let want = 1e-6 * b * dh * LIQUID_STEP;
    println!("slope {dh:.2} m over one cell, thickness {b:.3} m: Darcy {want:.3} m³, gained {:.3} m³", some - none);
    assert!(dh > 0.0 && b > 0.0, "the table of the wet cell is higher: {dh} {b}");
    assert!(((some - none) / want - 1.0).abs() < 0.05, "gained {} m³, Darcy says {want}", some - none);
    // Ten times the conductivity, ten times the flow.
    let (more, ..) = gained(1e-5, 0.8);
    assert!(((more - none10) / (some - none) / 10.0 - 1.0).abs() < 0.05, "x10 gave x{}", (more - none10) / (some - none));
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
    let (base, (hi, _)) = worldgen::testkit::island_with(3, pair);
    let mut w0 = base.clone();
    pcore::flow::state(&mut w0, true);
    let (n, area, p) = (w0.mesh.cells().n(), w0.mesh.cells().area()[0], w0.cfg.phys);
    let year = w0.days_per_year() as f64 * DAY;
    assert!(p.seep * w0.map.side * 1e-3 * year / (p.porosity * area) < 2e-4);
    let mv = w0.chem.molar_volume(LIQUID, Phase::Liquid);
    let full = p.bed_porosity * p.bed_depth * area;
    let stands = |w: &World, i: usize| w.surface_z(i) + pcore::contract::env_of(w, i)[1];
    // The slope of the steepest way down (`Level::steepest`) to the neighbours lower by more than `still`, as the sweep
    // reads it.
    let slope = |w: &World, i: usize| {
        let cells = w.mesh.cells();
        let drop = |k: usize| Some(stands(w, i) - stands(w, cells.around(i).1[k] as usize)).filter(|&d| d > p.still);
        cells.steepest(i, drop).map_or(0.0, |way| way.0)
    };
    // One liquid step with the store of `i` full and warm, with channels `hill` apart: the store and the ground liquid
    // after. (No pore liquid sinks into the stores, `seep` 0, so the neighbours' stores do not change with `hill`.)
    let run = |i: usize, hill: f64| {
        let mut w = base.clone();
        (w.cfg.evap, w.cfg.phys.hill, w.cfg.phys.seep) = (0.0, hill, 0.0);
        let e = full / mv * (w.chem.subs[LIQUID].cp * 290.0 + w.chem.subs[LIQUID].lf);
        set_amount(&mut w, Node::new(Class::Deep, i), LIQUID, full / mv);
        set_heat(&mut w, Node::new(Class::Deep, i), e);
        let books = Books::now(&w);
        worldgen::testkit::flow_step(&mut w, LIQUID_STEP);
        books.assert_kept(&w, 1e-9);
        (w.store.amounts(Class::Deep)[LIQUID * n + i] * mv, liquid_m3(&w, i))
    };
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
    let under = (0..n).find(|&i| pcore::contract::env_of(&w0, i)[1] > 20.0 && slope(&w0, i) == 0.0).expect("a cell under deep liquid");
    let ((d1, _), (d0, _)) = (run(under, p.hill), run(under, 1e15));
    assert!((d1 - d0).abs() <= 1e-9 * d0, "{d1} against {d0}");
}

/// Drainage (`flow::drain`): liquid over the share `field` of the pores sinks into the bed's store, no more than it
/// holds (`bed_porosity` x `bed_depth`) nor `seep` a second; a full store on a high cell passes its liquid down the
/// slope, and none overfills. Matter and energy are kept.
#[test]
fn liquid_over_field_capacity_drains_into_the_bed_and_flows_down() {
    let (mut w, (hi, lo)) = worldgen::testkit::island_with(3, pair);
    let (n, area, p) = (w.mesh.cells().n(), w.mesh.cells().area()[0], w.cfg.phys);
    let (pores, full, mv) = (p.porosity * loose(&w, hi), p.bed_porosity * p.bed_depth * area, w.chem.molar_volume(LIQUID, Phase::Liquid));
    // No rain: what sinks is only the liquid poured.
    (w.cfg.evap, w.cfg.rain_time) = (0.0, f64::INFINITY);
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
    let mut w = worldgen::testkit::squares(3, 16, 16);
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
    let base = worldgen::testkit::squares(3, 16, 16);
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
    // (Liquid steps alone: no air's step to evaporate, rain or freeze, which would add or take liquid and not the
    // mineral: the liquid there is all there is, and what it does is the pores' and the slope's.)
    let run = |x: f64, steps: usize| {
        let mut w = scene(x);
        let books = Books::now(&w);
        let liquid0 = w.store.amounts(Class::Ground)[LIQUID * n + top];
        for _ in 0..steps {
            pcore::flow::state(&mut w, true);
            pcore::flow::liquid(&mut w, DT);
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
/// dry air, in a pit dug in dry land that holds no part of a body of standing liquid, among dry land that holds none
/// either (lower than all its neighbours, so nothing runs off it, and no pore liquid flows into it).
#[test]
fn a_dissolved_mineral_stays_when_its_liquid_evaporates() {
    let near = |w: &World, i: usize| Some(w.mesh.cells().around(i).1).filter(|c| c.len() == 8).map(|c| c.iter().map(|&j| w.surface_z(j as usize)).collect::<Vec<f64>>());
    let dry = |w: &World, i: usize| w.store.amounts(Class::Ground)[LIQUID * w.mesh.cells().n() + i] == 0.0 && w.surface_z(i) > 20.0 && !w.in_body(i);
    let among = |w: &World| (0..w.mesh.cells().n()).find(|&i| dry(w, i) && near(w, i).is_some() && w.mesh.cells().around(i).1.iter().all(|&j| dry(w, j as usize)));
    let (mut w, i) = worldgen::testkit::island_with(3, among);
    let (n, na, ns) = (w.mesh.cells().n(), w.mesh.level(AIR).n(), w.chem.ns());
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

/// The diffusivity of a metre of the starting loose ground is the measured one: creep x softness x 1 m = 3e-3 m²/yr
/// (hillslope soil diffusivities are 1e-3 to 1e-2 m²/yr), within a tenth.
#[test]
fn the_starting_ground_creeps_at_a_measured_diffusivity() {
    let w = worldgen::testkit::squares(1, 16, 16);
    let cell = (0..w.mesh.cells().n()).find(|&i| w.surface_z(i) > 0.0).expect("land");
    let (h, _, soft) = ground_of(&w, cell);
    let d = w.cfg.creep * soft * w.days_per_year() as f64 * DAY;
    println!("loose ground {h:.2} m thick, softness {soft:.2}: diffusivity of a metre {d:.2e} m2/yr");
    assert!(h > 0.0, "{h} m of loose ground");
    assert!(d > 2.7e-3 && d < 3.3e-3, "{d} m2/yr");
}

/// The two rates of running liquid are the measured yield: the sediment it carries to the low ground, about 15 to 20
/// Gt a year, is 7e9 m³, and the potential energy of the runoff, 4e13 m³ a year falling 840 m, is 3.4e20 J: 2e-11 m³
/// per J. A bed of the starting rock breaks at the same.
#[test]
fn liquid_carries_and_breaks_at_the_measured_yield_per_joule() {
    let w = worldgen::testkit::squares(1, 16, 16);
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
        let mut w = worldgen::testkit::squares(1, 32, 32);
        (w.cfg.phys.porosity, w.cfg.phys.seep, w.cfg.evap) = (0.0, 0.0, 0.0);
        let (n, area) = (w.mesh.cells().n(), w.mesh.cells().area()[0]);
        let low: Vec<usize> = (0..n).filter(|&i| w.surface_z(i) < 0.0).collect();
        let apart = |i: usize, j: usize| (w.map.xy(i).0.abs_diff(w.map.xy(j).0)).max(w.map.xy(i).1.abs_diff(w.map.xy(j).1));
        let far: Vec<usize> = (0..n).filter(|&i| low.iter().all(|&j| apart(i, j) > FAR)).collect();
        far.iter().for_each(|&i| pour(&mut w, i, rain * area));
        let books = Books::now(&w);
        worldgen::testkit::flow_step(&mut w, LIQUID_STEP);
        books.assert_within(&w, 1e-9, 1e-9);
        (low.iter().map(|&i| liquid_m3(&w, i)).sum::<f64>(), rain * area * far.len() as f64)
    };
    let ((wet, fell), (dry, _)) = (low_after(0.05), low_after(0.0));
    println!("{:.1}% of the rain on the land far from the low ground reached it in one step", 100.0 * (wet - dry) / fell);
    assert!(fell > 0.0 && wet - dry > 0.1 * fell, "{} of {fell} m³ reached the low ground", wet - dry);
}

/// A pit's liquid over its rim runs out by its way out (laws.md 1.2): on an island with pits in its land (every
/// `PIT_EVERY`th land cell inland, its neighbours raised by rock to `RIM_M` over it at least), liquid poured on each pit to `COLUMN_M`
/// over its rim leaves the pits and their rims in one liquid step but for `KEPT` of what is over the rims: none is sent
/// back to a cell whose liquid runs into the pit, where the next step would send it back again. Matter and energy are
/// kept.
#[test]
fn a_pit_over_its_rim_spills_by_its_way_out() {
    const PIT_EVERY: usize = 9;
    const RIM_M: f64 = 20.0;
    const COLUMN_M: f64 = 100.0;
    const KEPT: f64 = 0.1;
    let mut w = worldgen::testkit::squares(1, 32, 32);
    (w.cfg.phys.porosity, w.cfg.phys.seep, w.cfg.evap) = (0.0, 0.0, 0.0);
    let (n, area) = (w.mesh.cells().n(), w.mesh.cells().area()[0]);
    let around = |w: &World, i: usize| w.mesh.cells().around(i).1.iter().map(|&j| j as usize).collect::<Vec<usize>>();
    let inland: Vec<usize> = (0..n).filter(|&i| w.surface_z(i) > 0.0 && around(&w, i).iter().all(|&j| w.surface_z(j) > 0.0)).collect();
    // Pits whose rims touch no other pit's.
    let mut pits: Vec<usize> = Vec::new();
    for &i in inland.iter().step_by(PIT_EVERY) {
        let near = |a: usize, b: usize| a == b || around(&w, a).contains(&b) || around(&w, a).iter().any(|&c| around(&w, c).contains(&b));
        if pits.iter().all(|&p| !near(p, i)) {
            pits.push(i);
        }
    }
    let rock_per_m3 = 1.0 / w.chem.molar_volume(worldgen::testkit::ROCK, Phase::Solid);
    for (i, j) in pits.iter().flat_map(|&i| around(&w, i).into_iter().map(move |j| (i, j))).collect::<Vec<(usize, usize)>>() {
        let (t, rise) = (w.t_ground(j), RIM_M + (w.surface_z(i) - w.surface_z(j)).max(0.0));
        put(&mut w, worldgen::testkit::ROCK, j, rise * area * rock_per_m3);
        hold_ground(&mut w, j, t);
    }
    assert!(pits.len() > 2, "{} pits", pits.len());
    let near: Vec<usize> = pits.iter().flat_map(|&i| around(&w, i).into_iter().chain([i])).collect();
    let held = |w: &World| near.iter().map(|&i| liquid_m3(w, i)).sum::<f64>();
    let rim = |w: &World, i: usize| (around(w, i).iter().map(|&j| w.surface_z(j)).fold(f64::MAX, f64::min) - w.surface_z(i)) * area;
    let (before, rims): (f64, Vec<f64>) = (held(&w), pits.iter().map(|&i| rim(&w, i)).collect());
    pits.iter().zip(&rims).for_each(|(&i, r)| pour(&mut w, i, r + COLUMN_M * area));
    let (over, rims) = (COLUMN_M * area * pits.len() as f64, rims.iter().sum::<f64>());
    let books = Books::now(&w);
    worldgen::testkit::flow_step(&mut w, LIQUID_STEP);
    books.assert_within(&w, 1e-9, 1e-9);
    let kept = held(&w) - before - rims;
    println!("{:.1}% of the liquid over the rims stayed around {} pits", 100.0 * kept / over, pits.len());
    assert!(kept < KEPT * over, "{kept:e} of {over:e} m³ over the rims stayed around the pits");
}


/// The height the liquid of cell `i` stands at, m: the level the liquid over what its pores hold reaches over its own
/// ground (its lowest ground where none stands).
fn liquid_top(w: &World, i: usize) -> f64 {
    pcore::flow::levels(w, &[i]).expect("Flow's state")[0][1]
}

/// The low basin of `w` under the level `top`, m: the lowest cell and the cells joined to it through cells whose solid
/// surface lies under it; and that lowest cell.
fn basin_under(w: &World, top: impl Fn(&World, usize) -> f64) -> (Vec<usize>, usize) {
    let n = w.mesh.cells().n();
    let lowest = (0..n).min_by(|&a, &b| w.surface_z(a).total_cmp(&w.surface_z(b))).unwrap();
    let level = top(w, lowest);
    let (mut basin, mut todo) = (vec![false; n], vec![lowest]);
    basin[lowest] = true;
    while let Some(i) = todo.pop() {
        for &j in w.mesh.cells().around(i).1 {
            if !basin[j as usize] && w.surface_z(j as usize) < level {
                basin[j as usize] = true;
                todo.push(j as usize);
            }
        }
    }
    ((0..n).filter(|&i| basin[i]).collect(), lowest)
}

/// The cells of `cells` whose ground lies wholly under the level of their liquid: every sample of the terrain in them
/// under it, so they hold no land (laws.md 1.2: a cell with land in it faces the air with two surfaces and does not mix
/// as one with its body).
fn wholly_under(w: &World, cells: &[usize]) -> Vec<usize> {
    let (s, leaves) = (w.terrain_samples(), w.map.leaves());
    cells.iter().copied().filter(|&i| leaves[i].split(pcore::space::terrain::SAMPLE_DEPTH).all(|f| s.h[s.index(&f)] < liquid_top(w, i))).collect()
}

/// The low basin of each air block mixes its heat in a month (laws.md 1.2): an island's low basin (one block) with
/// every other cell warmed by 3 K ends a liquid step with its surface at one temperature, within a millikelvin, over
/// most of its cells that hold no land (those off their plateaus, with no cover, the same phases). Matter and energy
/// are kept.
#[test]
fn a_blocks_low_basin_mixes_its_heat_in_a_month() {
    let mut w = worldgen::testkit::small_world();
    let cells = wholly_under(&w, &basin_under(&w, liquid_top).0);
    for &i in cells.iter().step_by(2) {
        let t = w.t_ground(i) + 3.0;
        hold_ground(&mut w, i, t);
    }
    pcore::flow::state(&mut w, true);
    let books = Books::now(&w);
    pcore::flow::liquid(&mut w, LIQUID_STEP);
    books.assert_within(&w, 1e-9, 1e-9);
    let mut t: Vec<f64> = cells.iter().map(|&i| w.t_surface_last(i)).collect();
    t.sort_by(f64::total_cmp);
    let mid = t[t.len() / 2];
    let one = t.iter().filter(|&&x| (x - mid).abs() < 1e-3).count();
    assert!(one > 3 * t.len() / 4, "{one} of {} cells at {mid} K: {t:?}", t.len());
}

/// The low basin under one air node takes up a gas as one liquid (laws.md 1.5): with none of it dissolved, after a
/// liquid step its cells under that node that mix as one each hold the same share of their liquid (most of the cells of
/// the node's basin that hold no land), so a cell of little liquid does not reach balance before a deep one. Matter and
/// energy are kept.
#[test]
fn the_low_basin_under_one_air_node_takes_up_gas_as_one_liquid() {
    let (_, init) = worldgen::chemistry0();
    let mut w = worldgen::testkit::small_world();
    w.cfg.dissolve = 1.0;
    let (n, c) = (w.mesh.cells().n(), &w.chem);
    let k = init.air.iter().map(|p| p.0).filter(|&k| c.subs[k].boil < 250.0).max_by(|&a, &b| c.subs[a].solubility.total_cmp(&c.subs[b].solubility)).unwrap();
    let basin = basin_under(&w, liquid_top).0;
    let cells = wholly_under(&w, &basin);
    let j = (0..w.mesh.level(AIR).n()).max_by_key(|&j| cells.iter().filter(|&&i| w.mesh.up(i, 0, AIR) == j).count()).unwrap();
    let under: Vec<usize> = cells.iter().copied().filter(|&i| w.mesh.up(i, 0, AIR) == j).collect();
    assert!(under.len() > 4, "{} cells of the basin under one node", under.len());
    // None dissolved anywhere: the liquid that runs into the basin brings none.
    for i in 0..n {
        set_amount(&mut w, Node::ground(i), k, 0.0);
        set_amount(&mut w, Node::new(Class::Deep, i), k, 0.0);
    }
    pcore::flow::state(&mut w, true);
    let books = Books::now(&w);
    pcore::flow::liquid(&mut w, DT);
    books.assert_within(&w, 1e-9, 1e-9);
    // Each cell's liquid, mol: what is molten in its node and below its boiling point.
    let liquid: Vec<f64> = under
        .iter()
        .map(|&i| {
            let (t, plateau) = w.thaw_node_last(i);
            (0..w.chem.ns()).filter(|&q| t < w.chem.subs[q].boil).map(|q| w.g(q, i) * w.molten(q, i, t, plateau)).sum()
        })
        .collect();
    let mut share: Vec<f64> = under.iter().zip(&liquid).map(|(&i, &l)| w.store.amounts(Class::Ground)[k * n + i] / l).collect();
    share.sort_by(f64::total_cmp);
    let mid = share[share.len() / 2];
    let one = share.iter().filter(|&&x| (x / mid - 1.0).abs() < 1e-9).count();
    assert!(mid > 0.0 && one > share.len() / 2, "{one} of {} cells at {mid:e} per mol: {share:?}", share.len());
}

/// Under the low basin the loose ground's pores are full and the bed's store is not drained: over a year of Flow no
/// liquid sinks into, flows through or comes up from the stores of its cells (those a metre under its level), while the
/// stores of the land take rain and drain.
#[test]
fn nothing_sinks_or_seeps_under_the_low_basin() {
    let mut w = worldgen::testkit::small_world();
    let (cells, _) = basin_under(&w, |w, i| liquid_top(w, i) - 1.0);
    let land: Vec<usize> = (0..w.mesh.cells().n()).filter(|&i| w.is_land(i)).collect();
    let (under, on_land) = (cells.iter().map(|&i| store_m3(&w, i)).collect::<Vec<f64>>(), land.iter().map(|&i| store_m3(&w, i)).collect::<Vec<f64>>());
    for _ in 0..12 {
        worldgen::testkit::flow_step(&mut w, LIQUID_STEP);
    }
    assert!(cells.iter().zip(&under).all(|(&i, &v)| store_m3(&w, i) == v), "a store under the basin changed");
    assert!(land.iter().zip(&on_land).any(|(&i, &v)| store_m3(&w, i) != v), "no store on the land changed");
}

/// Moving liquid wears its bed only with the work its load leaves free (the cover effect, Sklar and Dietrich 2004).
/// Once rain runs off (the pores of the land are full after three months), the land under its loose ground keeps its
/// bed in the next liquid step; stripped of its loose solids (at the temperature it had), over a full store, it breaks
/// bonded ground at each of the next eight.
#[test]
fn liquid_wears_its_bed_with_the_work_its_load_leaves_free() {
    let mut w = worldgen::testkit::squares(1, 16, 16);
    (0..3).for_each(|_| pcore::step(&mut w));
    let n = w.mesh.cells().n();
    let land: Vec<usize> = (0..n).filter(|&i| w.surface_z(i) > 0.0).collect();
    let bed = |w: &World| land.iter().map(|&i| (0..w.chem.ns()).map(|k| w.store.amounts(Class::Bed)[k * n + i]).sum::<f64>()).sum::<f64>();
    let mut covered = w.clone();
    flow_step(&mut covered, LIQUID_STEP);
    assert_eq!(bed(&covered), bed(&w), "the land under its loose ground wore its bed");
    for &i in &land {
        let t = w.t_ground(i);
        let solids: Vec<usize> = (0..w.chem.ns()).filter(|&k| w.chem.phase(k, t) == Phase::Solid).collect();
        solids.into_iter().for_each(|k| set_amount(&mut w, Node::ground(i), k, 0.0));
        hold_ground(&mut w, i, t);
        // A full store under it, so the rain runs off rather than sinking.
        let full = w.cfg.phys.bed_porosity * w.cfg.phys.bed_depth * w.mesh.cells().area()[i] / w.chem.molar_volume(LIQUID, Phase::Liquid);
        set_amount(&mut w, Node::new(Class::Deep, i), LIQUID, full);
        let warm = full * (w.chem.subs[LIQUID].cp * t + w.chem.subs[LIQUID].lf);
        set_heat(&mut w, Node::new(Class::Deep, i), warm);
    }
    for step in 0..8 {
        let before = bed(&w);
        flow_step(&mut w, LIQUID_STEP);
        assert!(bed(&w) < before, "step {step}: the bed on bare land kept {} of {before} mol", bed(&w));
    }
}

/// A small island, ten years: the land loses rock (the rock-making solids of its loose ground and bed, over its
/// cells) at the real rate of denudation, 0.01 to 1 mm a year.
#[test]
#[ignore = "slow: ten years of a 32 x 32 world"]
fn an_island_loses_ground_at_a_real_rate() {
    let mut w = worldgen::testkit::squares(1, 32, 32);
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

/// A small globe without life (8 rows, from pole to pole) runs 10 years whole and keeps its books, with liquid, heat
/// and wind crossing the wrap of its rows.
#[test]
fn a_globe_keeps_its_books_round_the_wrap() {
    let mut w = worldgen::terrain::new_globe(1, 8, [0; 4]);
    let books = Books::now(&w);
    for month in 0..10 * w.cfg.months_per_year {
        pcore::step(&mut w);
        assert_eq!(pcore::check::broken(&w), None, "month {month}");
    }
    books.assert_kept(&w, 1e-9);
}

/// An island that stands for a wide band of latitudes keeps its books with the boundary counted, its air stretched and
/// open on the planet's air (`World::planet`); the planet carries heat across it as across its band, in at the warm
/// south edge and out at the cold north one, and its southern air is warmer than its northern.
#[test]
fn an_island_in_the_planets_air_keeps_its_books_and_carries_heat_north() {
    const BAND: (f64, f64) = (30.0, 55.0);
    let mut w = worldgen::terrain::new_island(1, (16, 16), pcore::space::terrain::Shape::One, worldgen::terrain::banded(BAND));
    worldgen::planet::stand(&mut w);
    let books = Books::now(&w);
    (0..3 * w.cfg.months_per_year).for_each(|_| pcore::step(&mut w));
    books.assert_kept(&w, 1e-9);
    let [north, south, ..] = w.counts.edges;
    assert!(south > 0.0 && north < 0.0, "heat in at the north {north:e} J, at the south {south:e} J");
    let air = w.mesh.level(AIR);
    let mean = |south: bool| {
        let rows: Vec<usize> = (0..air.n()).filter(|&j| (air.lat()[j] < (BAND.0 + BAND.1) / 2.0) == south).collect();
        rows.iter().map(|&j| w.t_air(j)).sum::<f64>() / rows.len() as f64
    };
    assert!(mean(true) > mean(false) + 2.0, "south {} K, north {} K", mean(true), mean(false));
}

/// A globe of 32 rows without life keeps its books to rounding over 100 years, through its ice and its soft covers: its
/// air nodes and cells take no more than they hold (the air's days and the cells' exchange are one law), so nothing is
/// set to zero.
#[test]
#[ignore = "slow: 100 years of a 32-row globe"]
fn a_globe_keeps_its_books_to_rounding_for_a_hundred_years() {
    let mut w = worldgen::terrain::new_globe(1, 32, [0; 4]);
    let books = Books::now(&w);
    (0..100 * w.cfg.months_per_year).for_each(|_| pcore::step(&mut w));
    books.assert_kept(&w, 1e-9);
}

/// The sun is right at every latitude, on a circular orbit: the yearly mean at a pole is S sin ε / π and at the equator
/// 2 S E(sin ε) / π² (E the complete elliptic integral of the second kind; Berger 1978); at every season the whole
/// sphere's mean is S / 4 (the disc it shows the sun over its area); the south has the north's seasons half a year on.
#[test]
fn the_sun_is_right_at_every_latitude() {
    use pcore::flow::{declination, insolation};
    use std::f64::consts::{FRAC_PI_2, PI};
    const SEASONS: usize = 720;
    const BANDS: usize = 4000;
    let (s0, tilt) = (1361.0, 23.4);
    let yearly = |lat: f64| (0..SEASONS).map(|k| insolation(s0, lat, declination(tilt, (k as f64 + 0.5) / SEASONS as f64))).sum::<f64>() / SEASONS as f64;
    let k = tilt.to_radians().sin();
    // E(k) = π/2 (1 - Σ ((2n-1)!! / (2n)!!)² k^2n / (2n - 1)).
    let (mut e, mut term) = (1.0, 1.0);
    for n in 1..60 {
        term *= ((2 * n - 1) as f64 / (2 * n) as f64).powi(2) * k * k;
        e -= term / (2 * n - 1) as f64;
    }
    let (pole, equator) = (s0 * k / PI, 2.0 * s0 * e * FRAC_PI_2 / (PI * PI));
    for (lat, want) in [(FRAC_PI_2, pole), (-FRAC_PI_2, pole), (0.0, equator)] {
        let got = yearly(lat);
        assert!((got / want - 1.0).abs() < 1e-5, "latitude {lat}: {got} W/m2, {want} analytic");
    }
    for season in [0.0, 0.1, 0.25, 0.4] {
        let decl = declination(tilt, season);
        let band = |b: usize| -FRAC_PI_2 + PI * (b as f64 + 0.5) / BANDS as f64;
        let mean = (0..BANDS).map(|b| insolation(s0, band(b), decl) * band(b).cos()).sum::<f64>() * PI / BANDS as f64 / 2.0;
        assert!((mean / (s0 / 4.0) - 1.0).abs() < 1e-5, "season {season}: the sphere's mean {mean} W/m2");
        for lat in [0.3, 0.9, 1.4] {
            let (north, south) = (insolation(s0, lat, decl), insolation(s0, -lat, declination(tilt, season + 0.5)));
            assert!((north - south).abs() < 1e-9 * s0, "season {season}, latitude {lat}: north {north}, south {south}");
        }
    }
}

/// The sun on a slope is Lambert's cosine law summed over the day: on ground of slope (∂z/∂x, ∂z/∂y), x east and y
/// south, the beam per m² of plan is S (s · N), N = (−∂z/∂x, −∂z/∂y, 1), summed over the hours the sun is up (its own
/// sunrise and sunset, which a face turned away pays as the shadow it casts) by small steps of the hour angle; the
/// east-west slope adds nothing over a day, and the sun on a slope (`on_slope`) is that sum wherever it is above zero.
#[test]
fn the_sun_on_a_slope_is_lamberts_law_over_the_day() {
    use pcore::flow::{declination, on_slope, sun_by_latitude};
    use std::f64::consts::PI;
    const HOURS: usize = 200_000;
    let (s0, tilt) = (1361.0, 23.4);
    for (lat, season) in [(0.9f64, 0.0), (0.9, 0.25), (-0.5, 0.6), (1.3, 0.45), (0.1, 0.1)] {
        let decl: f64 = declination(tilt, season);
        for (east, south) in [(0.0, 0.0), (0.3, 0.0), (0.0, -0.4), (-0.2, 0.25), (0.5, 0.5)] {
            let day = (0..HOURS)
                .map(|k| {
                    let h = -PI + 2.0 * PI * (k as f64 + 0.5) / HOURS as f64;
                    let up = lat.sin() * decl.sin() + lat.cos() * decl.cos() * h.cos();
                    let (to_east, to_north) = (-decl.cos() * h.sin(), lat.cos() * decl.sin() - lat.sin() * decl.cos() * h.cos());
                    if up > 0.0 { s0 * (up - east * to_east + south * to_north) } else { 0.0 }
                })
                .sum::<f64>()
                / HOURS as f64;
            let law = on_slope(sun_by_latitude(s0, lat, decl), south);
            assert!((law - day.max(0.0)).abs() < 1e-4 * s0, "latitude {lat}, season {season}, slope ({east}, {south}): {law} W/m2 by the law, {day} summed");
        }
    }
}

/// The sun on a square of an island is the sum of the sun on its 16 cells (`on_slope`, at one latitude, so only the
/// slopes tell them apart): a square's rise toward the south is its cells' mean by area (`Terrain::rise_south`), and
/// the sun is linear in it, so the two differ only by what a cell, or the square, whose day sums below zero does not
/// take: at most the sum of those parts below zero, by area, at every latitude and season.
#[test]
fn the_sun_on_a_square_is_the_sum_over_its_cells() {
    use pcore::flow::{declination, on_slope, sun_by_latitude};
    let w = worldgen::testkit::squares(1, 16, 16);
    let (lat, map) = ((w.cfg.lat_south, w.cfg.lat_north), &w.map);
    let rise = |l: pcore::space::Leaf| w.terrain.rise_south(w.seed, map, l, lat);
    let (mut turned, mut steepest) = (0, 0.0f64);
    for &square in map.leaves() {
        let cells: Vec<(f64, f64)> = square.split(square.depth + 2).map(|l| (map.place_of(&l, lat.0, lat.1).0, rise(l))).collect();
        let (area, whole) = (cells.iter().map(|c| c.0).sum::<f64>(), rise(square));
        let mean = cells.iter().map(|c| c.0 * c.1).sum::<f64>() / area;
        steepest = cells.iter().fold(steepest, |m, c| m.max(c.1.abs()));
        assert!((mean - whole).abs() < 1e-12 * (1.0 + whole.abs()), "square {square:?}: its rise {whole}, its cells' mean {mean}");
        for phi in [0.0f64, 35.0, 60.0, 75.0] {
            for season in (0..12).map(|m| (m as f64 + 0.5) / 12.0) {
                let sun = sun_by_latitude(w.cfg.sun, phi.to_radians(), declination(w.cfg.tilt, season));
                let below = |r: f64| (-(sun[0] + r * sun[1])).max(0.0);
                let parts = cells.iter().map(|c| c.0 * on_slope(sun, c.1)).sum::<f64>();
                let bound = cells.iter().map(|c| c.0 * below(c.1)).sum::<f64>() + area * below(whole);
                turned += (bound > 0.0) as usize;
                let off = (parts - area * on_slope(sun, whole)).abs();
                assert!(off <= bound + 1e-12 * area * w.cfg.sun, "square {square:?}, latitude {phi}, season {season}: {off:e} W off, {bound:e} W turned away");
            }
        }
    }
    assert!(steepest > 0.05 && turned > 0, "the island has slopes ({steepest}) and faces turned away ({turned})");
}

/// The frozen part of the common liquid on the ground of cell `i`, mol.
fn frozen(w: &World, i: usize) -> f64 {
    let (t, plateau) = w.thaw_last(i);
    w.g(LIQUID, i) * (1.0 - w.molten(LIQUID, i, t, plateau))
}

/// Latitudes between which the two cells lie, with snow in their spring, degrees; those of the south and north edges of
/// their island, which reaches the low latitudes, so its air brings the cells heat from there; the months it runs first,
/// to late winter; the snow put on the cells, kg/m²; how far they rise toward the south or fall, m per m; and the most
/// steps of `DT` that the spring is followed for.
const SNOW_LAT: [f64; 2] = [50.0, 52.3];
const SNOW_WORLD_LAT: [f64; 2] = [10.0, 72.3];
const TO_SPRING: usize = 2;
const SNOW_KG: f64 = 30.0;
const FACING: f64 = 0.3;
const MELT_STEPS: usize = 60;

/// Of two cells alike but for the way they face (the lowest two dry land cells side by side between `SNOW_LAT` under one
/// air node that hold no part of a body of standing liquid, on the smallest island that has them,
/// `testkit::island_in_with`, flat amid flat land at one height, `testkit::flat_round`), at one latitude
/// under snow at the end of winter, the one that faces the equator takes more sun (`flow::on_slope`): it loses its snow
/// first and is warmer meanwhile, with no other change. Each faces each way in turn, so nothing else tells them apart.
/// Their island reaches the low latitudes
/// (`SNOW_WORLD_LAT`), as a real place at 50° N gets much of its winter heat from lower latitudes through the air.
#[test]
fn a_slope_facing_the_sun_loses_its_snow_first() {
    let cfg = pcore::Config { lat_south: SNOW_WORLD_LAT[0], lat_north: SNOW_WORLD_LAT[1], ..Default::default() };
    // The lowest such pair at the end of winter (flat at the height of the first, `flat_round`): its spring comes first.
    let pair = |w: &pcore::World| {
        let mut w = w.clone();
        (0..TO_SPRING).for_each(|_| pcore::step(&mut w));
        let (gw, lat) = (w.map.w, |i: usize| w.mesh.cells().lat()[i]);
        let dry: Vec<usize> = worldgen::testkit::dry_lands(&w).into_iter().filter(|&i| (SNOW_LAT[0]..SNOW_LAT[1]).contains(&lat(i)) && !w.in_body(i)).collect();
        let parent = w.mesh.cells().parent();
        let pairs = dry.iter().copied().filter(|&i| i % gw + 1 < gw && dry.contains(&(i + 1)) && parent[i] == parent[i + 1]);
        pairs.min_by(|&x, &y| w.surface_z(x).total_cmp(&w.surface_z(y)))
    };
    let (mut w0, a) = worldgen::testkit::island_in_with(1, cfg, pair);
    (0..TO_SPRING).for_each(|_| pcore::step(&mut w0));
    let b = a + 1;
    worldgen::testkit::flat_round(&mut w0, a);
    for &i in &[a, b] {
        worldgen::testkit::put_kg_m2(&mut w0, LIQUID, i, SNOW_KG);
        let cold = w0.chem.subs[LIQUID].melt - 5.0;
        hold_ground(&mut w0, i, cold);
    }
    for (sunny, shaded) in [(a, b), (b, a)] {
        let mut w = w0.clone();
        (w.rise_south[sunny], w.rise_south[shaded]) = (-FACING, FACING);
        // The step each loses the last of its snow, and the sum of its temperatures until then.
        let (mut gone, mut warmth) = ([None; 2], [0.0; 2]);
        for step in 0..MELT_STEPS {
            flow_step(&mut w, DT);
            w.time += DT as u64;
            for (c, &i) in [sunny, shaded].iter().enumerate() {
                warmth[c] += w.t_ground(i);
                if gone[c].is_none() && frozen(&w, i) <= 0.0 {
                    gone[c] = Some(step);
                }
            }
            if gone[1].is_some() {
                break;
            }
        }
        let [Some(first), Some(last)] = gone else { panic!("the snow of both melts in spring: {gone:?}") };
        assert!(first < last && warmth[0] > warmth[1], "facing the sun: gone at step {first}, {} K; facing away: step {last}, {} K", warmth[0], warmth[1]);
    }
}

/// The potential energy the store books (`Store::lifted`) is the change of the matter's potential energy: over five
/// years of a 16 x 16 island with the roster's life its rain, runoff, stores, creep, the air's columns, Change and the
/// matter its life takes and gives and carries from cell to cell move it, and the two stay within 1e-6 of what it
/// gained (what is left: gas lifted after the month's later laws moved the heads Flow's booking found).
#[test]
fn the_lifted_energy_is_what_the_matter_gained() {
    let mut w = worldgen::testkit::squares(1, 16, 16);
    pcore::step(&mut w);
    for &k in &worldgen::life::ROSTER {
        worldgen::life::seed(&mut w, k);
    }
    let (p0, l0, heat) = (pcore::flow::potential(&w).expect("Flow's state"), w.store.lifted(), pcore::check::totals(&w).thermal);
    for _ in 0..5 * w.cfg.months_per_year {
        pcore::step(&mut w);
    }
    assert!(pcore::cover::sums(&w).iter().any(|s| s.slots > 0.0), "nothing lives");
    let (gained, lifted) = (pcore::flow::potential(&w).expect("Flow's state") - p0, w.store.lifted() - l0);
    assert!(lifted.abs() > 1e-7 * heat, "the matter moved: {lifted:e} J");
    assert!((gained - lifted).abs() < 1e-6 * lifted.abs(), "gained {gained:e} J, lifted {lifted:e} J");
}

/// A cell whose own ground lies partly under its liquid is two surfaces under one air, each with its own heat (laws.md
/// 1.1): on the island of most tests, through the second year, the land part of each cell that stays two parts all year
/// (its land at another temperature than its node every month) swings between its warmest and its coldest month by more
/// than the node as a whole, whose heat capacity is mostly its liquid's, far more per m² than the land's skin; and the
/// books keep.
#[test]
fn a_coast_cells_land_swings_more_than_the_cell() {
    let mut w = worldgen::testkit::small_world();
    for _ in 0..w.cfg.months_per_year {
        pcore::step(&mut w);
    }
    let (books, n) = (Books::now(&w), w.mesh.cells().n());
    // Per cell its land's and its node's temperature each month, K.
    let mut year = Vec::new();
    for _ in 0..w.cfg.months_per_year {
        pcore::step(&mut w);
        year.push((0..n).map(|i| [w.t_ground_last(i), w.thaw_node_last(i).0]).collect::<Vec<[f64; 2]>>());
    }
    let two: Vec<usize> = (0..n).filter(|&i| year.iter().all(|m| m[i][0] != m[i][1])).collect();
    assert!(two.len() >= 3, "cells of two parts all year: {two:?}");
    let swing = |i: usize, p: usize| year.iter().map(|m| m[i][p]).fold(f64::MIN, f64::max) - year.iter().map(|m| m[i][p]).fold(f64::MAX, f64::min);
    let mean = |p: usize| two.iter().map(|&i| swing(i, p)).sum::<f64>() / two.len() as f64;
    let (land, node) = (mean(0), mean(1));
    assert!(land > node, "the land swings {land:.2} K, the node {node:.2} K, over {} cells", two.len());
    books.assert_kept(&w, 1e-9);
}

/// A year of the small world, whose shores hold cells of parts (land beside its liquid's top, each facing the
/// air, so the surface's temperature is not its land's), with its air's steps and their rain: the books are kept to
/// rounding and no node is overdrawn (`Books::assert_kept`, `check::broken`): the rain of a step wets the bucket of each
/// cell's own ground, never its liquid's top as well (`soak`).
#[test]
fn a_year_of_rain_on_cells_of_parts_keeps_the_books() {
    let mut w = worldgen::testkit::small_world();
    let (books, n) = (Books::now(&w), w.mesh.cells().n());
    let mut two_faces = false;
    for _ in 0..w.cfg.months_per_year {
        pcore::step(&mut w);
        two_faces |= (0..n).any(|i| w.t_surface_last(i) != w.t_ground_last(i));
    }
    assert!(two_faces, "no cell of parts with its land and its liquid's top facing the air");
    books.assert_kept(&w, 1e-9);
}
