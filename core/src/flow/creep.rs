use super::land::{lift, melted, molten, rise};
use super::scratch::around;
use super::{held, range, Scratch};
use crate::grid::FORWARD;
use crate::World;

/// Yearly: loose solids creep to lower neighbours, softer ones faster and thicker layers more;
/// heat conducts between ground nodes; bodies spread back into ground cleared along edges
/// (`choose::regrow`, here only, so it closes at the documented speed).
pub(super) fn creep(w: &mut World, s: &mut Scratch, dt: f64) {
    let (n, area, g, cell) = (w.grid.n(), s.area, w.cfg.gravity, w.grid.cell_m);
    let (tmin, _) = range(&s.tg);
    let sol: Vec<usize> = s.loose.iter().copied().filter(|&k| s.sp[k].melt > tmin).collect();
    let soft: Vec<f64> = (0..w.chem.ns()).map(|k| 1.0 / (w.chem.subs[k].hardness + 0.05)).collect();
    s.mob.fill(0.0);
    for &k in &sol {
        for i in 0..n {
            s.mob[i] += w.ground[k * n + i] * (1.0 - molten(s, k, i)) * s.sp[k].v[0] * soft[k];
        }
    }
    let mut heat = 0.0;
    for i in 0..n {
        for (d, j) in around(&FORWARD, w.grid.w, w.grid.h, i).into_iter().enumerate() {
            if j == usize::MAX {
                continue;
            }
            let dist = w.grid.dist(FORWARD[d]);
            let (ti, tj) = (s.tg[i], s.tg[j]);
            let gk = (s.bed[i][3] + s.bed[j][3]) / 2.0 * w.cfg.skin_m * cell / dist;
            let qmax = 0.2 * (ti - tj).abs() * s.cap[i].min(s.cap[j]);
            let qh = (gk * (ti - tj) * dt).clamp(-qmax, qmax);
            // Heat goes from the warmer node; it gives no more than it holds above `T_FLOOR`.
            let (from, to) = if qh >= 0.0 { (i, j) } else { (j, i) };
            let took = w.take_heat(from, qh.abs());
            w.e_ground[to] += took;

            let (a, b) = if s.z[i] > s.z[j] { (i, j) } else { (j, i) };
            let dz = s.z[a] - s.z[b];
            if s.mob[a] <= 0.0 || dz <= 0.0 {
                continue;
            }
            let r = w.cfg.creep * dz / dist * cell * dt / area * held(w, a);
            let f = (area * dz / 4.0 / (r * s.mob[a])).min(1.0);
            let (ta, tb) = (s.tg[a], s.tg[b]);
            // What moves, summed over substances: heat capacity, mass, mobility, rise of each surface and head.
            let (mut c, mut mass, mut mob, mut ua, mut ub) = (0.0, 0.0, 0.0, [0.0; 2], [0.0; 2]);
            for &k in &sol {
                let x = w.ground[k * n + a] * (1.0 - molten(s, k, a));
                if x > 0.0 {
                    let (p, m) = (&s.sp[k], (x * r * f * soft[k]).min(0.25 * x));
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
                heat += drop;
            }
        }
    }
    w.ledger.gravity += heat;
    crate::choose::regrow(w, dt);
}
