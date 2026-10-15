//! Soil liquid (15 Oct 2026, queued item 4 of 13.2, step 1): the loose ground has pores, the
//! porosity times the volume of its loose solids. Liquid soaks into them up to that volume and only
//! the liquid over it stands and runs off (`flow::liquid`); the liquid in the pores flows to a
//! neighbour by Darcy's law, at `Physics::seep` times the slope of the liquid table, through the
//! thickness of ground it fills; a body soaks it up like any liquid of its cell.

use pcore::chem::Phase;
use pcore::check::{drift, totals};
use pcore::World;

/// 15 days: one liquid step.
const DT: f64 = 15.0 * 86_400.0;

/// Volume of the loose solids of land cell `i`, m³, and the cell's liquid (the key liquid of chemistry 0), mol per m³.
fn loose(w: &World, i: usize) -> f64 {
    let t = w.t_ground(i);
    (0..w.chem.ns()).filter(|&k| w.chem.phase(k, t) == Phase::Solid).map(|k| w.g(k, i) * w.chem.molar_volume(k, Phase::Solid)).sum()
}

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
    let depth = |w: &World| pcore::body::env_of(w, i)[1];
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
        let (t0, l0) = (totals(&w), w.ledger.clone());
        let v0 = liquid_m3(&w, k, i);
        pcore::flow::step(&mut w, DT);
        let (de, dq) = drift(&w, &t0, &l0);
        assert!(de < 1e-12 && dq < 1e-9, "drift {de} {dq}");
        (liquid_m3(&w, k, i) / v0, liquid_m3(&w, k, i) / pores)
    };
    // Half the pores' volume: it all stays (but for what the open air takes), where the same sheet ran off before.
    let (kept, _) = run(0.4, 0.5);
    let (kept_old, _) = run(0.0, 0.5);
    assert!(kept > 0.7, "the ground kept {kept} of its liquid");
    assert!(kept_old < 0.1, "with no pores {kept_old} of it stayed");
    // Twice the pores' volume: the pores are full and the rest ran off.
    let (_, full) = run(0.4, 2.0);
    assert!(full < 1.05 && full > 0.7, "the ground holds {full} of its pore volume after the excess ran off");
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
        // (No evaporation, which would lower the liquid table before the liquid flows.)
        w.cfg.evap = 0.0;
        let pores = w.cfg.phys.porosity * loose(&w, i);
        pour(&mut w, k, i, share * pores);
        let (t0, l0) = (totals(&w), w.ledger.clone());
        pcore::flow::step(&mut w, DT);
        let (de, dq) = drift(&w, &t0, &l0);
        assert!(de < 1e-12 && dq < 1e-9, "drift {de} {dq}");
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
    let want = 1e-6 * b * w.grid.cell_m * dh / w.grid.cell_m * DT;
    println!("slope {dh:.2} m over one cell, thickness {b:.3} m: Darcy {want:.3} m³, gained {:.3} m³", some - none);
    assert!(dh > 0.0 && b > 0.0, "the table of the wet cell is higher: {dh} {b}");
    assert!(((some - none) / want - 1.0).abs() < 0.05, "gained {} m³, Darcy says {want}", some - none);
    // Ten times the conductivity, ten times the flow.
    let (more, ..) = gained(1e-5, 0.5);
    assert!(((more - none) / (some - none) / 10.0 - 1.0).abs() < 0.05, "x10 gave x{}", (more - none) / (some - none));
    // More liquid, a thicker flow: the thickness that carries it, and the slope, are bigger.
    let (full, dh2, b2) = gained(1e-6, 0.9);
    let want2 = 1e-6 * b2 * dh2 * DT;
    assert!(b2 > b && dh2 > dh);
    assert!(((full - none) / want2 - 1.0).abs() < 0.05, "gained {} m³, Darcy says {want2}", full - none);
}

/// The rate is physical: 1e-6 m/s through a metre of ground at a slope of 1 in 1000 moves a cubic metre in a few weeks
/// across a kilometre of width, nowhere near enough to move the liquid of a cell (1.6 million m³ of pores) across it.
#[test]
fn pore_liquid_barely_moves_across_a_cell() {
    let w = worldgen::new_world(3, 16, 16);
    let (k, slope, thick) = (w.cfg.phys.seep, 1e-3, 1.0);
    let per_year = k * thick * w.grid.cell_m * slope * 365.0 * 86_400.0;
    let pores = w.cfg.phys.porosity * 1.0 * w.grid.area();
    assert!(per_year / pores < 1e-4, "{per_year} m³ a year against {pores} m³ of pores");
}
