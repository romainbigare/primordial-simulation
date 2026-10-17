//! Drainage of the loose ground into the store of the bed below, and the flow of the stores.

use super::land::{darcy, t_now, wet, HALF_WAY};
use super::{range, Scratch};
use crate::grid::DIRS;
use crate::World;

/// Drainage: the loose ground holds its liquid against gravity up to a share `field` of its pores; the
/// rest sinks, `seep` a second at most (Darcy's law under its own weight), into the store of the bed below
/// (`World::deep`: a zone `bed_depth` deep with pores `bed_porosity` of it). The stores flow to their
/// neighbours by Darcy's law as the pores do (`bed_seep` x the depth filled x the slope of the table, toward a
/// lower table, no more than half way to a level one); what reaches a full store comes out into its node's
/// loose ground (a spring, or standing liquid). Each move takes its share of every liquid and what is dissolved
/// in it, with its heat; the head it loses on the way (from a node's head or a store's table to where it goes)
/// heats it there, as the liquid above does (`liquid`). Bodies and Law 2 do not reach the store.
pub(super) fn drain(w: &mut World, s: &mut Scratch, dt: f64) {
    let (n, ns, area, p, g) = (w.grid.n(), w.chem.ns(), s.area, w.cfg.phys, w.cfg.gravity);
    if w.deep.len() != ns * n {
        (w.deep, w.e_deep) = (vec![0.0; ns * n], vec![0.0; n]);
    }
    let (tmin, tmax) = range(&s.tg);
    let liq = s.liquids(&w.chem, (tmin, tmax), true);
    let (full, sy) = (p.bed_porosity * p.bed_depth * area, p.bed_porosity * area);
    // Sink into the store below.
    for i in 0..n {
        let lvol: f64 = liq.iter().filter(|&&k| s.sp[k].boil > tmin).map(|&k| w.ground[k * n + i] * wet(s, k, i) * s.sp[k].v[1]).sum();
        let v = (lvol - p.field * s.pd[i] * area).min(full - store_volume(w, s, i, tmin)).min(p.seep * area * dt);
        if v <= 0.0 {
            continue;
        }
        let (tcur, fall) = (t_now(w, s, i), g * (s.head[i] - store_table(w, s, i, tmin)));
        for &k in &liq {
            let sp = s.sp[k];
            let m = w.ground[k * n + i] * if sp.boil <= tmin { 1.0 } else { wet(s, k, i) } * v / lvol;
            let (m, e) = crate::ground::take_paid(&mut w.e_ground[i], &mut w.counts, s.cap[i], m, sp.cp, m * (sp.cp * tcur + sp.lf));
            (w.deep[k * n + i], w.e_deep[i]) = (w.deep[k * n + i] + m, w.e_deep[i] + e + m * sp.mm * fall);
            w.potential -= m * sp.mm * fall;
            s.put(&mut w.ground, k, i, -m, true);
        }
    }
    // The stores flow to their neighbours; what a full store cannot take comes out above it.
    for i in 0..n {
        let nbr = s.nbr[i];
        for (d, &j) in nbr.iter().enumerate().filter(|p| *p.1 != u32::MAX) {
            let (j, held) = (j as usize, store_volume(w, s, i, tmin));
            let (ti, tj) = (store_table(w, s, i, tmin), store_table(w, s, j, tmin));
            let dh = ti - tj;
            if dh <= p.still || held <= 0.0 {
                continue;
            }
            let v = darcy(p.bed_seep * held / sy, &w.grid, DIRS[d], dh, dt).min(HALF_WAY / 2.0 * dh * sy).min(held);
            // The share of the store that moves, and the share of that the neighbour's store has room for.
            let (f, into) = (v / held, ((full - store_volume(w, s, j, tmin)) / v).clamp(0.0, 1.0));
            let (e, mut mass, head) = (w.e_deep[i] * f, 0.0, s.head[j]);
            for k in 0..ns {
                let m = w.deep[k * n + i] * f;
                if m <= 0.0 {
                    continue;
                }
                (w.deep[k * n + i], w.deep[k * n + j]) = (w.deep[k * n + i] - m, w.deep[k * n + j] + m * into);
                s.put(&mut w.ground, k, j, m * (1.0 - into), true);
                mass += m * s.sp[k].mm;
            }
            // Falling to the neighbour's table into its store, and to its head (or rising, a spring) out of it.
            let (to_store, to_node) = (mass * into * g * dh, mass * (1.0 - into) * g * (ti - head));
            (w.e_deep[i], w.e_deep[j], w.e_ground[j]) = (w.e_deep[i] - e, w.e_deep[j] + e * into + to_store, w.e_ground[j] + e * (1.0 - into) + to_node);
            w.potential -= to_store + to_node;
        }
    }
}

/// Volume of the liquid in the store under ground node `i`, m³ (what boils below the coldest node, `tmin`, is
/// dissolved gas and adds none).
fn store_volume(w: &World, s: &Scratch, i: usize, tmin: f64) -> f64 {
    let n = w.grid.n();
    (0..w.chem.ns()).map(|k| if s.sp[k].boil > tmin { w.deep[k * n + i] * s.sp[k].v[1] } else { 0.0 }).sum()
}

/// Height of the table of the store under ground node `i`, m: the bottom of its zone, `bed_depth` under the loose
/// ground (whose depth its pores tell, none without pores), plus the depth its liquid fills.
pub(super) fn store_table(w: &World, s: &Scratch, i: usize, tmin: f64) -> f64 {
    let p = w.cfg.phys;
    let loose = if p.porosity > 0.0 { s.pd[i] / p.porosity } else { 0.0 };
    s.z[i] - loose - p.bed_depth + store_volume(w, s, i, tmin) / (p.bed_porosity * s.area)
}
