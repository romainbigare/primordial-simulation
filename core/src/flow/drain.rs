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
/// in it, with its heat. Bodies and Law 2 do not reach the store.
pub(super) fn drain(w: &mut World, s: &mut Scratch, dt: f64) {
    let (n, ns, area, p) = (w.grid.n(), w.chem.ns(), s.area, w.cfg.phys);
    if w.deep.len() != ns * n {
        (w.deep, w.e_deep) = (vec![0.0; ns * n], vec![0.0; n]);
    }
    let (tmin, tmax) = range(&s.tg);
    let liq = s.liquids(&w.chem, (tmin, tmax), true);
    let (full, sy) = (p.bed_porosity * p.bed_depth * area, p.bed_porosity * area);
    // Volume of the liquid of a store (dissolved gas adds none), and its table.
    let vm: Vec<f64> = (0..ns).map(|k| if s.sp[k].boil > tmin { s.sp[k].v[1] } else { 0.0 }).collect();
    let dv = |w: &World, i: usize| (0..ns).map(|k| w.deep[k * n + i] * vm[k]).sum::<f64>();
    let table = |w: &World, s: &Scratch, i: usize| s.z[i] - s.pd[i] / p.porosity - p.bed_depth + dv(w, i) / sy;
    // Sink into the store below.
    for i in 0..n {
        let lvol: f64 = liq.iter().filter(|&&k| s.sp[k].boil > tmin).map(|&k| w.ground[k * n + i] * wet(s, k, i) * s.sp[k].v[1]).sum();
        let v = (lvol - p.field * s.pd[i] * area).min(full - dv(w, i)).min(p.seep * area * dt);
        if v <= 0.0 {
            continue;
        }
        let tcur = t_now(w, s, i);
        for &k in &liq {
            let sp = s.sp[k];
            let m = w.ground[k * n + i] * if sp.boil <= tmin { 1.0 } else { wet(s, k, i) } * v / lvol;
            let (m, e) = crate::ground::take_paid(&mut w.e_ground[i], &mut w.ledger, s.cap[i], m, sp.cp, m * (sp.cp * tcur + sp.lf));
            (w.deep[k * n + i], w.e_deep[i]) = (w.deep[k * n + i] + m, w.e_deep[i] + e);
            s.put(&mut w.ground, k, i, -m, true);
        }
    }
    // The stores flow to their neighbours; what a full store cannot take comes out above it.
    for i in 0..n {
        let nbr = s.nbr[i];
        for (d, &j) in nbr.iter().enumerate().filter(|p| *p.1 != u32::MAX) {
            let (j, held) = (j as usize, dv(w, i));
            let dh = table(w, s, i) - table(w, s, j);
            if dh <= p.still || held <= 0.0 {
                continue;
            }
            let v = darcy(p.bed_seep * held / sy, &w.grid, DIRS[d], dh, dt).min(HALF_WAY / 2.0 * dh * sy).min(held);
            // The share of the store that moves, and the share of that the neighbour's store has room for.
            let (f, into) = (v / held, ((full - dv(w, j)) / v).clamp(0.0, 1.0));
            let e = w.e_deep[i] * f;
            (w.e_deep[i], w.e_deep[j], w.e_ground[j]) = (w.e_deep[i] - e, w.e_deep[j] + e * into, w.e_ground[j] + e * (1.0 - into));
            for k in 0..ns {
                let m = w.deep[k * n + i] * f;
                if m <= 0.0 {
                    continue;
                }
                (w.deep[k * n + i], w.deep[k * n + j]) = (w.deep[k * n + i] - m, w.deep[k * n + j] + m * into);
                s.put(&mut w.ground, k, j, m * (1.0 - into), true);
            }
        }
    }
}
