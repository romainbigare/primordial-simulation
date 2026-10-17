//! Drainage of the loose ground into the store of the bed below, and the flow of the stores.

use super::land::{darcy, t_now, wet, HALF_WAY};
use super::{range, Scratch};
use crate::grid::DIRS;
use crate::World;

/// Drainage: the loose ground holds its liquid against gravity up to a share `field` of its pores; the
/// rest sinks, `seep` a second at most (Darcy's law under its own weight), into the store of the bed below
/// (`World::deep`: a zone `bed_depth` deep with pores `bed_porosity` of it). The stores flow to their
/// neighbours by Darcy's law as the pores do (`stores`). Each move takes its share of every liquid and what is dissolved
/// in it, with its heat; the head it loses on the way (from a node's head or a store's table to where it goes)
/// heats it there, as the liquid above does (`liquid`). Bodies and Law 2 do not reach the store.
pub(super) fn drain(w: &mut World, s: &mut Scratch, dt: f64) {
    let (n, ns, area, p, g) = (w.grid.n(), w.chem.ns(), s.area, w.cfg.phys, w.cfg.gravity);
    if w.deep.len() != ns * n {
        (w.deep, w.e_deep) = (vec![0.0; ns * n], vec![0.0; n]);
    }
    let (tmin, tmax) = range(&s.tg);
    let liq = s.liquids(&w.chem, (tmin, tmax), true);
    let full = p.bed_porosity * p.bed_depth * area;
    // Sink into the store below.
    for i in 0..n {
        let lvol: f64 = liq.iter().filter(|&&k| s.sp[k].boil > tmin).map(|&k| w.ground[k * n + i] * wet(s, k, i) * s.sp[k].v[1]).sum();
        let v = (lvol - p.field * s.pd[i] * area).min(full - store_volume(w, s, i, tmin)).min(p.seep * area * dt);
        if v <= 0.0 {
            continue;
        }
        // The share of its liquid that sinks: at most 1, so no product below takes more than the node holds.
        let (tcur, fall, sinks) = (t_now(w, s, i), g * (s.head[i] - store_table(w, s, i, tmin)), v / lvol);
        for &k in &liq {
            let sp = s.sp[k];
            let m = w.ground[k * n + i] * if sp.boil <= tmin { 1.0 } else { wet(s, k, i) } * sinks;
            let (m, e) = crate::ground::take_paid(&mut w.e_ground[i], &mut w.counts, s.cap[i], m, sp.cp, m * (sp.cp * tcur + sp.lf));
            (w.deep[k * n + i], w.e_deep[i]) = (w.deep[k * n + i] + m, w.e_deep[i] + e + m * sp.mm * fall);
            w.potential -= m * sp.mm * fall;
            s.put(&mut w.ground, k, i, -m);
        }
    }
    stores(w, s, dt, tmin);
}

/// The stores flow to their neighbours by Darcy's law (`bed_seep` x the depth filled x the slope of the table, toward
/// a lower table, no more than half way to a level one), all from the stores as they are before (so no direction
/// goes first), each giving no more than it holds. A store takes what it has room for (with what it gives); the
/// rest comes out into its node's loose ground (a spring, or standing liquid).
fn stores(w: &mut World, s: &mut Scratch, dt: f64, tmin: f64) {
    let (n, ns, p, g) = (w.grid.n(), w.chem.ns(), w.cfg.phys, w.cfg.gravity);
    let (full, sy) = (p.bed_porosity * p.bed_depth * s.area, p.bed_porosity * s.area);
    let (table, held): (Vec<f64>, Vec<f64>) = (0..n).map(|i| (store_table(w, s, i, tmin), store_volume(w, s, i, tmin))).unzip();
    // Each edge's share of the store it leaves; the share leaving each store, and the volume coming into each.
    let (mut moves, mut out, mut inflow) = (Vec::new(), vec![0.0; n], vec![0.0; n]);
    for i in (0..n).filter(|&i| held[i] > 0.0) {
        for (d, &j) in s.nbr[i].iter().enumerate().filter(|p| *p.1 != u32::MAX) {
            let (j, dh) = (j as usize, table[i] - table[j as usize]);
            if dh <= p.still {
                continue;
            }
            let v = darcy(p.bed_seep * held[i] / sy, &w.grid, DIRS[d], dh, dt).min(HALF_WAY / 2.0 * dh * sy).min(held[i]);
            if v > 0.0 {
                moves.push((i, j, v / held[i]));
                out[i] += v / held[i];
            }
        }
    }
    for m in &mut moves {
        m.2 /= out[m.0].max(1.0);
        inflow[m.1] += m.2 * held[m.0];
    }
    let (deep0, e0, head) = (w.deep.clone(), w.e_deep.clone(), s.head.clone());
    for i in (0..n).filter(|&i| out[i] > 0.0) {
        let keep = 1.0 - out[i].min(1.0);
        for k in 0..ns {
            w.deep[k * n + i] *= keep;
        }
        w.e_deep[i] *= keep;
    }
    for (i, j, f) in moves {
        let into = ((full - held[j] * (1.0 - out[j].min(1.0))) / inflow[j]).clamp(0.0, 1.0);
        let (e, mut mass) = (e0[i] * f, 0.0);
        for k in (0..ns).filter(|&k| deep0[k * n + i] > 0.0) {
            let m = deep0[k * n + i] * f;
            w.deep[k * n + j] += m * into;
            if into < 1.0 {
                s.put(&mut w.ground, k, j, m * (1.0 - into));
            }
            mass += m * s.sp[k].mm;
        }
        // Falling to the neighbour's table into its store, and to its head (or rising, a spring) out of it.
        let (to_store, to_node) = (mass * into * g * (table[i] - table[j]), mass * (1.0 - into) * g * (table[i] - head[j]));
        (w.e_deep[j], w.e_ground[j]) = (w.e_deep[j] + e * into + to_store, w.e_ground[j] + e * (1.0 - into) + to_node);
        w.potential -= to_store + to_node;
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
