//! The yearly part of Flow: creep of loose solids, and the regrowth of bodies into cleared ground.

use super::land::{lift, melted, rise, solid};
use super::{bare_share, range, Scratch};
use crate::space::square::FORWARD;
use crate::World;

/// Added to a substance's hardness before its softness (1 / hardness) is taken.
const HARD_MIN: f64 = 0.05;
/// Most volume that creeps over an edge per call, as a share of the step between the surfaces
/// times the cell's area (the higher node goes no further than half way to level).
const CREEP_LEVEL: f64 = 0.25;
/// Most of each loose solid that creeps over an edge per call.
const CREEP_MOST: f64 = 0.25;

/// Yearly: loose solids creep to lower neighbours, softer ones faster and thicker layers more;
/// bodies spread back into ground cleared along edges
/// (`lanes::regrow`, here only, so it closes at the documented speed).
pub(super) fn creep(w: &mut World, s: &mut Scratch, dt: f64) {
    let n = w.grid.n();
    let sol = s.solids(range(&s.tg).0);
    let soft: Vec<f64> = (0..w.chem.ns()).map(|k| 1.0 / (w.chem.subs[k].hardness + HARD_MIN)).collect();
    s.mob.fill(0.0);
    for &k in &sol {
        for i in 0..n {
            s.mob[i] += solid(w, s, k, i) * s.sp[k].v[0] * soft[k];
        }
    }
    let mut heat = 0.0;
    for i in 0..n {
        for d in FORWARD {
            let Some(j) = w.grid.step(i, d) else { continue };
            let (a, b) = if s.z[i] > s.z[j] { (i, j) } else { (j, i) };
            slide(w, s, (a, b), d, (&sol, &soft), dt, &mut heat);
        }
    }
    w.potential -= heat;
    crate::lanes::regrow(w, dt);
}

/// Loose solids `sol` (with softness `soft`) creep from the higher node `a` to its neighbour `b`
/// (along `d`) with their heat; the mass's fall heats `b` and is counted in `heat`.
fn slide(w: &mut World, s: &mut Scratch, (a, b): (usize, usize), d: (i32, i32), (sol, soft): (&[usize], &[f64]), dt: f64, heat: &mut f64) {
    let (n, area, g, cell, dist) = (w.grid.n(), s.area, w.cfg.gravity, w.grid.cell_m, w.grid.dist(d));
    let dz = s.z[a] - s.z[b];
    if s.mob[a] <= 0.0 || dz <= 0.0 {
        return;
    }
    let r = w.cfg.creep * dz / dist * cell * dt / area * bare_share(w, a);
    let f = (area * dz * CREEP_LEVEL / (r * s.mob[a])).min(1.0);
    let (ta, tb) = (s.tg[a], s.tg[b]);
    // What moves, summed over substances: heat capacity, mass, mobility, rise of each surface and head.
    let (mut c, mut mass, mut mob, mut ua, mut ub) = (0.0, 0.0, 0.0, [0.0; 2], [0.0; 2]);
    for &k in sol {
        let x = solid(w, s, k, a);
        if x > 0.0 {
            let (p, m) = (&s.sp[k], (x * r * f * soft[k]).min(CREEP_MOST * x));
            w.ground[k * n + a] -= m;
            w.ground[k * n + b] += m;
            let (da, db) = (rise(p, ta, melted(s, k, a), -m, area), rise(p, tb, melted(s, k, b), m, area));
            (c, mass, mob) = (c + m * p.cp, mass + m * p.mm, mob + m * p.v[0] * soft[k]);
            (ua, ub) = ([ua[0] + da[0], ua[1] + da[1]], [ub[0] + db[0], ub[1] + db[1]]);
        }
    }
    if c > 0.0 {
        let drop = mass * g * dz / 2.0;
        (s.cap[a], s.cap[b], s.mob[a], s.mob[b]) = (s.cap[a] - c, s.cap[b] + c, s.mob[a] - mob, s.mob[b] + mob);
        // The heat of what moved goes with it (no more than the node holds above `T_FLOOR`).
        let took = w.take_heat(a, c * ta);
        w.e_ground[b] += took + drop;
        lift(s, a, ua);
        lift(s, b, ub);
        *heat += drop;
    }
}
