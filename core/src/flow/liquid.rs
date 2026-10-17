use super::land::{molten, settle, stand, table, wet};
use super::{held, range, Scratch};
use crate::grid::DIRS;
use crate::math::one_minus_exp;
use crate::units::DAY;
use crate::World;

/// Liquid by head, as one ordered sweep from the highest head down: each node passes liquid to
/// its lower neighbours, more to the steeper, so it can cross the map in one sweep while
/// depressions fill and spill. The head lost heats the node. Every few months (`work` > 0) the
/// power lost also carries loose solids up to a capacity, drops the rest, and breaks bonded
/// ground by power over bond strength.
pub(super) fn liquid(w: &mut World, s: &mut Scratch, dt: f64, work: f64) {
    let (n, area, g, ph) = (w.grid.n(), s.area, w.cfg.gravity, w.cfg.phys);
    let inv = DIRS.map(|d| 1.0 / w.grid.dist(d));
    let (tmin, tmax) = range(&s.tg);
    // (With the gases dissolved in it, which are gas at every node: `air::dissolve`.)
    let liq: Vec<usize> = s.loose.iter().copied().filter(|&k| s.sp[k].melt <= tmax && (s.sp[k].boil > tmin || w.chem.subs[k].solubility > 0.0)).collect();
    let sol: Vec<usize> = s.loose.iter().copied().filter(|&k| s.sp[k].melt > tmin).collect();
    let ns = sol.len();
    s.sed.resize(n * ns, 0.0);
    // Highest head first (heads to 0.1 m, then by cell); kept between refreshes (any order is safe,
    // it only sets how far liquid gets in one call).
    if s.order.len() != n {
        s.order = (0..n).map(|i| (((2e4 - stand(s, i)).max(0.0) * 10.0) as u64) << 20 | i as u64).collect();
        s.order.sort_unstable();
    }
    s.done.fill(false);
    let ema = (dt / (w.days_per_year() as f64 * DAY)) as f32;
    let mut heat = 0.0;
    let mut wa = vec![0.0; liq.len()];
    for r in 0..n {
        let i = (s.order[r] & 0xf_ffff) as usize;
        let (t, hd, zi) = (s.tg[i], stand(s, i), s.z[i]);
        s.done[i] = true;
        // Lower neighbours and how far this head may fall for each: to one still to come (which
        // passes the liquid on) or to its own bed; half way to one already done, so heads never cross.
        // (Nothing stands on a node whose head is its solid surface: it may fall by none.)
        let (mut nb, mut q, mut room) = ([(0usize, 0.0f64); 8], [0.0f64; 8], [0.0f64; 8]);
        let (mut cnt, mut wsum, mut rmax, mut qmax) = (0, 0.0, 0.0f64, 0.0f64);
        for (d, &j) in s.nbr[i].iter().enumerate() {
            if j == u32::MAX {
                continue;
            }
            let (j, hj) = (j as usize, stand(s, j as usize));
            let (dh, r) = (hd - hj, if hd <= zi { 0.0 } else if s.done[j] { 0.5 * (hd - hj) } else { hd - hj.max(zi) });
            if dh > ph.still {
                (nb[cnt], room[cnt], q[cnt]) = ((j, dh), r, dh * inv[d]);
                (wsum, rmax, qmax) = (wsum + q[cnt], rmax.max(r), qmax.max(q[cnt]));
                cnt += 1;
            }
        }
        // Inside the node the liquid drains to its channels, `hill` from the divides, by Darcy's law down the
        // slope of its surface (`qmax`, to the steepest lower neighbour), and runs off (`hs`, m3): from the bed's
        // store, a share 1 - exp(-`bed_seep` x slope x time / (`bed_porosity` x `hill`)) of it, which comes up with
        // its heat (below, from the loose ground over its field capacity, at `seep` through its pores).
        let mut hs = 0.0;
        if qmax > 0.0 && w.deep.len() == w.chem.ns() * n {
            let f = one_minus_exp(ph.bed_seep * qmax * dt / (ph.bed_porosity * ph.hill));
            let e = w.e_deep[i] * f;
            (w.e_deep[i], w.e_ground[i]) = (w.e_deep[i] - e, w.e_ground[i] + e);
            for k in 0..w.chem.ns() {
                let (m, sp) = (w.deep[k * n + i] * f, s.sp[k]);
                if m > 0.0 {
                    (w.deep[k * n + i], w.ground[k * n + i], s.cap[i]) = (w.deep[k * n + i] - m, w.ground[k * n + i] + m, s.cap[i] + m * sp.cp);
                    s.hold[i][0] += if s.pk[i] != k { m * sp.lf } else { 0.0 };
                    settle(s, k, i, m);
                    hs += if sp.boil > tmin { m * sp.v[1] * wet(s, k, i) } else { 0.0 };
                }
            }
        }
        // Head above the surface is liquid standing here; liquid in the pores flows by Darcy's law.
        let damp = s.pd[i] > 0.0 && s.head[i] > zi;
        if !damp && work <= 0.0 && (cnt == 0 || (hd <= zi && hs <= 0.0)) {
            w.liquid_flux[i] *= 1.0 - ema;
            continue;
        }
        let mut lvol = 0.0;
        for (a, &k) in wa.iter_mut().zip(&liq) {
            // Dissolved gas moves with the liquid and adds no volume.
            let gas = s.sp[k].boil <= tmin;
            *a = w.ground[k * n + i] * if gas { 1.0 } else { wet(s, k, i) };
            lvol += if gas { 0.0 } else { *a * s.sp[k].v[1] };
        }
        let (pores, lv0) = (s.pd[i] * area, lvol - hs);
        if qmax > 0.0 && damp {
            let lp = lv0.min(pores);
            hs += (lp - ph.field * pores).max(0.0).min(lp * one_minus_exp(ph.seep * qmax * dt / (ph.porosity * ph.hill)));
        }
        // Neighbours much less steep than the steepest get nothing.
        for qc in q[..cnt].iter_mut() {
            if *qc < w.cfg.phys.split * qmax {
                wsum -= *qc;
                *qc = 0.0;
            }
        }
        // What leaves carries the node's heat at its temperature now: earlier radiation and
        // evaporation in this call may have cooled it below `tg`, and liquid from above mixed in
        // at its own temperature (so a flood of cooler liquid is not sent on at the old, hotter one).
        let tcur = t.min((w.e_ground[i] - s.hold[i][0]) / s.cap[i].max(1.0)).max(0.0);
        let (mut power, mut out) = (0.0, 0.0);
        // Only the liquid over the pores runs off (`qs`: the load goes with it alone); `fall`: each
        // neighbour's drop in head, for the power lost.
        let (mut fall, mut qs, mut qss) = ([0.0; 8], [0.0; 8], 0.0);
        if lvol > 0.0 && cnt > 0 {
            let (x, y) = ((lv0 - pores).max(0.0).min(area * rmax) / wsum, hs / wsum);
            for c in 0..cnt {
                q[c] = (x * q[c]).min(area * room[c]) + y * q[c];
                (fall[c], qs[c]) = (nb[c].1, q[c]);
                qss += q[c];
            }
        }
        // Pore liquid flows by Darcy's law: through the ground the table fills, at `seep` times the
        // slope of its head (toward a lower table, no more than half way to a level one).
        if lvol > 0.0 && damp {
            let por = w.cfg.phys.porosity;
            let ti = table(s, i, por);
            for (d, &j) in s.nbr[i].iter().enumerate() {
                if j == u32::MAX {
                    continue;
                }
                let j = j as usize;
                let dh = ti - table(s, j, por);
                let b = ti - (s.z[i] - s.pd[i] / por).max(s.z[j] - s.pd[j] / por);
                if dh <= w.cfg.phys.still || b <= 0.0 {
                    continue;
                }
                let (si, sj) = (area * if s.head[i] - zi >= s.pd[i] { 1.0 } else { por }, area * if s.head[j] - s.z[j] >= s.pd[j] { 1.0 } else { por });
                let v = (w.cfg.phys.seep * b * w.grid.cell_m * dh / w.grid.dist(DIRS[d]) * dt).min(0.5 * dh / (1.0 / si + 1.0 / sj));
                if v <= 0.0 {
                    continue;
                }
                let c = (0..cnt).find(|&c| nb[c].0 == j).unwrap_or_else(|| {
                    (nb[cnt], q[cnt], fall[cnt]) = ((j, 0.0), 0.0, 0.0);
                    cnt += 1;
                    cnt - 1
                });
                fall[c] = (fall[c] * q[c] + dh * v) / (q[c] + v);
                q[c] += v;
            }
        }
        if lvol > 0.0 && cnt > 0 {
            let qsum: f64 = q[..cnt].iter().sum();
            let (part, inv) = ((qsum / lvol).min(1.0), 1.0 / qsum);
            for (&a, &k) in wa.iter().zip(&liq).filter(|p| *p.0 > 0.0) {
                let (p, m) = (s.sp[k], (a * part).min(w.ground[k * n + i]));
                // The liquid leaves with its heat and fusion enthalpy; less leaves if the node cannot pay.
                let (m, e) = crate::ground::take_paid(&mut w.e_ground[i], &mut w.ledger, s.cap[i], m, p.cp, m * (p.cp * tcur + p.lf));
                (w.ground[k * n + i], s.cap[i]) = (w.ground[k * n + i] - m, s.cap[i] - m * p.cp);
                s.hold[i][0] -= if s.pk[i] != k { m * p.lf } else { 0.0 };
                settle(s, k, i, -m);
                out += m;
                for ((&(j, _), &dh), &q) in nb[..cnt].iter().zip(&fall).zip(&q).filter(|p| *p.1 > 0.0) {
                    let (m, e) = (m * q * inv, e * q * inv);
                    (w.ground[k * n + j], s.cap[j], w.e_ground[j]) = (w.ground[k * n + j] + m, s.cap[j] + m * p.cp, w.e_ground[j] + e);
                    s.hold[j][0] += if s.pk[j] != k { m * p.lf } else { 0.0 };
                    settle(s, k, j, m);
                    power += m * p.mm * g * dh;
                }
            }
        }
        w.e_ground[i] += power;
        heat += power;
        w.liquid_flux[i] += ((out / dt) as f32 - w.liquid_flux[i]) * ema;
        if work <= 0.0 {
            continue;
        }

        // The work, on the bed below the liquid left standing (which hides it): drop what is over
        // capacity (all of it where nothing flows on), else pick up loose solids and break bonded ground.
        let bedwork = power * work * w.cfg.phys.hide[1] / (w.cfg.phys.hide[1] + (lvol - s.pd[i] * area - qss).max(0.0) / area);
        let load: f64 = sol.iter().enumerate().map(|(si, &k)| s.sed[i * ns + si] * s.sp[k].v[0]).sum();
        let capv = w.cfg.carry * bedwork * held(w, i);
        if load <= 0.0 && capv < 1.0 {
            continue;
        }
        if load > capv || qss <= 0.0 {
            let f = if qss > 0.0 { 1.0 - capv / load } else { 1.0 };
            for (si, &k) in sol.iter().enumerate() {
                let m = s.sed[i * ns + si] * f;
                s.sed[i * ns + si] -= m;
                w.ground[k * n + i] += m;
                s.cap[i] += m * s.sp[k].cp;
                settle(s, k, i, m);
            }
            w.e_ground[i] += s.sed_e[i] * f;
            s.sed_e[i] -= s.sed_e[i] * f;
        } else if bedwork > 0.0 {
            let solid = |k: usize| w.ground[k * n + i] * (1.0 - molten(s, k, i));
            let avail: f64 = sol.iter().map(|&k| solid(k) * s.sp[k].v[0]).sum();
            let f = if avail > 0.0 { ((capv - load) / avail).min(0.5) } else { 0.0 };
            for (si, &k) in sol.iter().enumerate() {
                if f > 0.0 {
                    let m = w.ground[k * n + i] * (1.0 - molten(s, k, i)) * f;
                    // Picked up with its heat; less is picked up if the node cannot pay.
                    let (m, e) = w.take_paid(i, m, s.sp[k].cp, m * s.sp[k].cp * tcur);
                    w.ground[k * n + i] -= m;
                    s.sed[i * ns + si] += m;
                    s.cap[i] -= m * s.sp[k].cp;
                    s.sed_e[i] += e;
                    settle(s, k, i, -m);
                }
            }
            for bi in 0..s.bedk.len() {
                let (k, bv) = (s.bedk[bi], s.bed[i][1]);
                let b = w.bed[k * n + i];
                if b > 0.0 && bv > 0.0 {
                    let v = w.cfg.wear * bedwork * (b * s.sp[k].v[0] / bv) / w.chem.subs[k].strength.max(0.05);
                    let m = (v / s.sp[k].v[0]).min(0.5 * b);
                    w.bed[k * n + i] -= m;
                    w.ground[k * n + i] += m;
                    s.bed[i][1] -= m * s.sp[k].v[0];
                    s.cap[i] += m * s.sp[k].cp;
                    if !s.loose.contains(&k) {
                        s.loose.push(k);
                        s.vary.push(k);
                    }
                }
            }
        }
        if qss <= 0.0 {
            continue;
        }
        for (&(j, dh), &q) in nb[..cnt].iter().zip(&qs).filter(|p| *p.1 > 0.0) {
            let (f, mut mass) = (q / qss, 0.0);
            for (si, &k) in sol.iter().enumerate() {
                let m = s.sed[i * ns + si] * f;
                if m <= 0.0 {
                    continue;
                }
                mass += m * s.sp[k].mm;
                if s.done[j] {
                    w.ground[k * n + j] += m;
                    s.cap[j] += m * s.sp[k].cp;
                    settle(s, k, j, m);
                } else {
                    s.sed[j * ns + si] += m;
                }
            }
            let e = s.sed_e[i] * f + mass * g * dh;
            heat += mass * g * dh;
            if s.done[j] {
                w.e_ground[j] += e;
            } else {
                s.sed_e[j] += e;
            }
        }
        s.sed[i * ns..(i + 1) * ns].fill(0.0);
        s.sed_e[i] = 0.0;
    }
    w.ledger.gravity += heat;
}
