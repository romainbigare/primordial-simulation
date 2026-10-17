//! Soil liquid (15 Oct 2026, queued item 4 of 13.2, step 1): the loose ground has pores, the
//! porosity times the volume of its loose solids. Liquid soaks into them up to that volume and only
//! the liquid over it stands and runs off (`flow::liquid`); the liquid in the pores flows to a
//! neighbour by Darcy's law, at `Physics::seep` times the slope of the liquid table, through the
//! thickness of ground it fills; a body soaks it up like any liquid of its cell.

use pcore::chem::Phase;
use pcore::World;
use worldgen::testkit::{loose_volume as loose, Books};

/// 15 days: one liquid step.
const DT: f64 = 15.0 * 86_400.0;

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
        pcore::flow::step(&mut w, DT);
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
        pcore::flow::step(&mut w, DT);
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
        pcore::flow::step(&mut w, DT);
        books.assert_kept(&w, 1e-9);
        (w.deep[k * n + i] * mv, liquid_m3(&w, k, i))
    };
    let (hi, _) = pair(&w0);
    let s = slope(&w0, hi);
    let want = full * -(-p.bed_seep * s * DT / (p.bed_porosity * p.hill)).exp_m1();
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
    pcore::flow::step(&mut w, DT);
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
        pcore::flow::step(&mut w, DT);
        assert!(deep(&w, lo) <= full * 1.000001, "the low cell's store overfilled");
    }
    assert!(deep(&w, hi) < 0.9 * full, "the full store on the high cell did not drain: {} of {full} m³", deep(&w, hi));
    books.assert_kept(&w, 1e-9);
}
