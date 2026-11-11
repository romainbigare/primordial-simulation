//! Bodies of standing liquid and the runoff down the terrain's network (laws.md 1.2): a body's cells end a month at one
//! level; what a body holds over its pass arrives in the bodies below it; what runs through a sample is what runs into
//! it from the samples above.

use pcore::chem::Phase;
use pcore::space::network::NONE;
use pcore::space::terrain::SAMPLE_DEPTH;
use pcore::store::Class;
use pcore::units::DAY;
use pcore::World;
use worldgen::testkit::{flow_step, hold_ground, put, Books, LIQUID};

/// One liquid step, s.
const STEP: f64 = 30.0 * DAY;
/// The volume poured over a body's pass, m³; and how far over its melting point a cell is warm enough that no part of
/// it holds its liquid frozen, K (the poured liquid is held there).
const POURED_M3: f64 = 1e9;
const WARM_K: f64 = 5.0;
/// How far the cells of a body may stand from its level at the month's end, a share of what it holds: the state of a
/// cell's ground found after the pass (its surface and pores, `land::ground_state`) and the laws after it in the month
/// (the gases that dissolve) move a cell's liquid by millimetres.
const LEVEL: f64 = 1e-6;

/// Months of the scene.
const MONTHS: usize = 12;

/// The scene: the small island of a mild climate (`testkit::small_world`) after `MONTHS` liquid steps, so its bodies hold
/// the liquid the steps left them, with no load carried (the ground stays where the pass found it).
fn world() -> World {
    let mut w = worldgen::testkit::small_world();
    w.cfg.carry = 0.0;
    (0..MONTHS).for_each(|_| flow_step(&mut w, STEP));
    w
}

/// The liquid of `cells`, in their ground and in the bed's store under them, mol.
fn held(w: &World, cells: &[u32]) -> f64 {
    let n = w.mesh.cells().n();
    let of = |c: Class| cells.iter().map(|&i| w.store.amounts(c)[LIQUID * n + i as usize]).sum::<f64>();
    of(Class::Ground) + of(Class::Deep)
}

/// Every body of two cells or more that holds liquid over its pores ends a month at one level: each of its cells whose
/// liquid neither freezes nor melts after the pass (`WARM_K` over the liquid's melting point) holds over its pores what
/// its part's samples hold under the body's level (`Pool::stands`, `Pool::under`), within `LEVEL` of what the body
/// holds, the open low ground too.
#[test]
fn the_cells_of_a_body_end_a_month_at_one_level() {
    let w = world();
    let warm = w.chem.subs[LIQUID].melt + WARM_K;
    let mut seen = 0;
    for pool in w.pools().iter().filter(|l| l.cells.len() > 1 && l.volume > 0.0) {
        for (c, (a, b)) in pool.stands.iter().zip(&pool.under).enumerate().filter(|p| w.t_ground(pool.cells[p.0] as usize) > warm) {
            assert!((a - b).abs() <= LEVEL * pool.volume, "body of pond {}, cell {}: {a} m³ over its pores, {b} m³ under its level, of {} m³", pool.pond, pool.cells[c], pool.volume);
            seen += 1;
        }
    }
    println!("{seen} cells checked");
    assert!(seen > 0, "no cell of a body of two cells or more holds liquid");
}

/// What a body holds over its pass leaves over it and arrives in the bodies down its way in the same month: liquid
/// poured on a body with a pass, far more than it holds under its pass, leaves it at its pass's level, and all the
/// rest of what was poured is in the bodies down its way (against the same world with none poured), within a thousandth:
/// the other laws of the month (the poured cell's pores, the flow under the ground) differ a little between the two.
#[test]
fn what_leaves_a_body_over_its_pass_arrives_in_the_body_below() {
    let w0 = world();
    let pools = w0.pools();
    let (x, pool) = pools.iter().enumerate().find(|(_, l)| l.next.is_some()).expect("a body with a pass");
    let pass = w0.terrain_samples().ponds[pool.pond].level;
    let mut down = Vec::new();
    let mut at = pool.next;
    while let Some(j) = at {
        down.extend_from_slice(&pools[j].cells);
        at = pools[j].next;
    }
    let mut w = w0.clone();
    let mol = POURED_M3 / w.chem.molar_volume(LIQUID, Phase::Liquid);
    let i = pool.cells[0] as usize;
    put(&mut w, LIQUID, i, mol);
    let warm = w.chem.subs[LIQUID].melt + WARM_K;
    let t = w.t_ground(i).max(warm);
    hold_ground(&mut w, i, t);
    let books = Books::now(&w);
    let mut control = w0.clone();
    flow_step(&mut w, STEP);
    flow_step(&mut control, STEP);
    books.assert_within(&w, 1e-9, 1e-9);
    let after = &w.pools()[x];
    assert!((after.level - pass).abs() < 1e-9 * pass.abs().max(1.0), "the body ends at {} m, its pass at {pass} m", after.level);
    let (src, below) = (held(&w, &pool.cells) - held(&control, &pool.cells), held(&w, &down) - held(&control, &down));
    assert!(((src + below) / mol - 1.0).abs() < 1e-3, "of {mol:e} mol poured, {src:e} stayed and {below:e} reached the bodies below");
    assert!(below > 0.5 * mol, "{below:e} of {mol:e} mol went on over the pass");
}

/// What runs through a sample is what the samples above it pass on (those whose liquid runs to it, but those of a body,
/// which keeps what reaches it) and its own share of its cell's runoff, by its share of the cell's area: on the
/// island's equal samples, every sample of a cell in no body that lies in no body and is no body's pass runs what runs
/// through those of its samples that nothing runs into (all alike) and what runs into it, to the rounding of the
/// moving averages' 32-bit numbers.
#[test]
fn the_discharge_at_a_sample_is_what_the_samples_above_it_pass_on() {
    let w = world();
    let s = w.terrain_samples();
    let pools = w.pools();
    let mut body = vec![false; s.ponds.len()];
    pools.iter().for_each(|l| body[l.pond] = true);
    let in_body = |k: usize| s.pond[k] != NONE && body[s.pond[k] as usize];
    let mut pass = vec![false; s.h.len()];
    pools.iter().filter(|l| l.next.is_some()).for_each(|l| pass[s.ponds[l.pond].pass as usize] = true);
    let mut from = vec![0.0f64; s.h.len()];
    for (k, &j) in s.to.iter().enumerate().filter(|p| *p.1 != NONE && !in_body(p.0)) {
        from[j as usize] += w.discharge[k] as f64;
    }
    let mut checked = 0;
    for (i, l) in w.map.leaves().iter().enumerate().filter(|p| !w.in_body(p.0)) {
        let ks: Vec<usize> = l.split(SAMPLE_DEPTH).map(|f| s.index(&f)).filter(|&k| !in_body(k) && !pass[k]).collect();
        let heads: Vec<f64> = ks.iter().filter(|&&k| from[k] == 0.0).map(|&k| w.discharge[k] as f64).collect();
        let Some(&own) = heads.first().filter(|&&x| x > 0.0) else { continue };
        assert!(heads.iter().all(|&x| (x / own - 1.0).abs() < 1e-6), "cell {i}: its first samples send {heads:?}");
        for &k in &ks {
            let d = w.discharge[k] as f64;
            assert!((d - from[k] - own).abs() <= 1e-5 * d, "cell {i}, sample {k}: {d} through it, {} from above, {own} its own", from[k]);
        }
        checked += 1;
    }
    println!("{checked} cells checked");
    assert!(checked >= 10, "{checked} cells checked");
}
