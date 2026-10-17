//! Liquid over and in the loose ground: one ordered sweep by head, the springs of the bed's
//! store, pore flow by Darcy's law, and the work moving liquid does on its bed.

use super::land::{darcy, solid, stand, t_now, table, wet};
use super::{bare_share, follow, range, year_weight, Scratch};
use crate::grid::DIRS;
use crate::math::one_minus_exp;
use crate::World;

/// The lower neighbours of a node in the sweep, and what it sends each.
#[derive(Default)]
struct Outlets {
    /// How many there are.
    cnt: usize,
    /// Each neighbour and the drop in head to it, m.
    nb: [(usize, f64); 8],
    /// Weight by slope (`lower`), then volume sent (`runoff`, `pore_flow`), m³.
    q: [f64; 8],
    /// How far the head may fall toward each, m.
    room: [f64; 8],
    /// Drop in head of what goes to each, for the power lost, m.
    fall: [f64; 8],
    /// Volume that runs off over the pores to each (the load goes with it alone), m³, and its sum.
    qs: [f64; 8],
    qss: f64,
    /// Sum of the weights; the farthest the head may fall, m; the steepest slope.
    wsum: f64,
    rmax: f64,
    qmax: f64,
}

/// Liquid by head, as one ordered sweep from the highest head down: each node passes liquid to
/// its lower neighbours, more to the steeper, so it can cross the map in one sweep while
/// depressions fill and spill. The head lost heats the node. Every few months (`work` > 0) the
/// power lost also carries loose solids up to a capacity, drops the rest, and breaks bonded
/// ground by power over bond strength.
pub(super) fn liquid(w: &mut World, s: &mut Scratch, dt: f64, work: f64) {
    let (n, area, ph) = (w.grid.n(), s.area, w.cfg.phys);
    let inv = DIRS.map(|d| 1.0 / w.grid.dist(d));
    let (tmin, tmax) = range(&s.tg);
    let (liq, sol) = (s.liquids(&w.chem, (tmin, tmax), true), s.solids(tmin));
    s.sed.resize(n * sol.len(), 0.0);
    // Highest head first (heads to 0.1 m, then by cell); kept between refreshes (any order is safe,
    // it only sets how far liquid gets in one call).
    if s.order.len() != n {
        s.order = (0..n).map(|i| (((2e4 - stand(s, i)).max(0.0) * 10.0) as u64) << 20 | i as u64).collect();
        s.order.sort_unstable();
    }
    s.done.fill(false);
    let ema = year_weight(w, dt);
    let mut heat = 0.0;
    let mut wa = vec![0.0; liq.len()];
    for r in 0..n {
        let i = (s.order[r] & 0xf_ffff) as usize;
        let (hd, zi) = (stand(s, i), s.z[i]);
        s.done[i] = true;
        let mut out = lower(s, i, &inv, ph.still);
        let mut hs = spring(w, s, i, out.qmax, dt, tmin);
        // Head above the surface is liquid standing here; liquid in the pores flows by Darcy's law.
        let damp = s.pd[i] > 0.0 && s.head[i] > zi;
        if !damp && work <= 0.0 && (out.cnt == 0 || (hd <= zi && hs <= 0.0)) {
            w.liquid_flux[i] *= 1.0 - ema;
            continue;
        }
        let lvol = mobile(w, s, i, &liq, tmin, &mut wa);
        let (pores, lv0) = (s.pd[i] * area, lvol - hs);
        // (The loose ground's share of the runoff `hs`: what its pores hold over their field capacity, at `seep`.)
        if out.qmax > 0.0 && damp {
            let lp = lv0.min(pores);
            hs += (lp - ph.field * pores).max(0.0).min(lp * one_minus_exp(ph.seep * out.qmax * dt / (ph.porosity * ph.hill)));
        }
        // Neighbours much less steep than the steepest get nothing.
        for qc in out.q[..out.cnt].iter_mut() {
            if *qc < w.cfg.phys.split * out.qmax {
                out.wsum -= *qc;
                *qc = 0.0;
            }
        }
        // What leaves carries the node's heat at its temperature now (so a flood of cooler liquid
        // from above is not sent on at the old, hotter one).
        let tcur = t_now(w, s, i);
        if lvol > 0.0 && out.cnt > 0 {
            runoff(&mut out, lv0 - pores, hs, area);
        }
        if lvol > 0.0 && damp {
            pore_flow(w, s, i, &mut out, dt);
        }
        let (power, sent) = if lvol > 0.0 && out.cnt > 0 { send(w, s, i, &out, (&liq, &wa), lvol, tcur) } else { (0.0, 0.0) };
        w.e_ground[i] += power;
        heat += power;
        follow(&mut w.liquid_flux[i], (sent / dt) as f32, ema);
        // The work, on the bed below the liquid left standing (which hides it).
        let bedwork = power * work * ph.hide[1] / (ph.hide[1] + (lvol - s.pd[i] * area - out.qss).max(0.0) / area);
        if work > 0.0 && work_on_bed(w, s, i, &out, &sol, bedwork, tcur) {
            carry(w, s, i, &out, &sol, &mut heat);
        }
    }
    w.ledger.gravity += heat;
}

/// Lower neighbours of node `i` and how far its head may fall for each: to one still to come
/// (which passes the liquid on) or to its own bed; half way to one already done, so heads never
/// cross. (Nothing stands on a node whose head is its solid surface: it may fall by none.)
fn lower(s: &Scratch, i: usize, inv: &[f64; 8], still: f64) -> Outlets {
    let (hd, zi) = (stand(s, i), s.z[i]);
    let mut o = Outlets::default();
    for (d, &j) in s.nbr[i].iter().enumerate() {
        if j == u32::MAX {
            continue;
        }
        let (j, hj) = (j as usize, stand(s, j as usize));
        let (dh, r) = (hd - hj, if hd <= zi { 0.0 } else if s.done[j] { 0.5 * (hd - hj) } else { hd - hj.max(zi) });
        if dh > still {
            let c = o.cnt;
            (o.nb[c], o.room[c], o.q[c]) = ((j, dh), r, dh * inv[d]);
            (o.wsum, o.rmax, o.qmax) = (o.wsum + o.q[c], o.rmax.max(r), o.qmax.max(o.q[c]));
            o.cnt += 1;
        }
    }
    o
}

/// Inside node `i` the liquid drains to its channels, `hill` from the divides, by Darcy's law down
/// the slope of its surface (`qmax`, to the steepest lower neighbour), and runs off. From the
/// bed's store a share 1 - exp(-`bed_seep` x slope x time / (`bed_porosity` x `hill`)) comes up
/// into the node with its heat (a spring); returns the volume of it that runs off, m³.
fn spring(w: &mut World, s: &mut Scratch, i: usize, qmax: f64, dt: f64, tmin: f64) -> f64 {
    let (n, ph) = (w.grid.n(), w.cfg.phys);
    let mut hs = 0.0;
    if qmax > 0.0 && w.deep.len() == w.chem.ns() * n {
        let f = one_minus_exp(ph.bed_seep * qmax * dt / (ph.bed_porosity * ph.hill));
        let e = w.e_deep[i] * f;
        (w.e_deep[i], w.e_ground[i]) = (w.e_deep[i] - e, w.e_ground[i] + e);
        for k in 0..w.chem.ns() {
            let (m, sp) = (w.deep[k * n + i] * f, s.sp[k]);
            if m > 0.0 {
                w.deep[k * n + i] -= m;
                s.put(&mut w.ground, k, i, m, true);
                hs += if sp.boil > tmin { m * sp.v[1] * wet(s, k, i) } else { 0.0 };
            }
        }
    }
    hs
}

/// The mobile amount of each liquid `liq` in node `i` into `wa`, mol (its liquid share; all of a
/// dissolved gas); returns their volume, m³ (dissolved gas adds none).
fn mobile(w: &World, s: &Scratch, i: usize, liq: &[usize], tmin: f64, wa: &mut [f64]) -> f64 {
    let n = w.grid.n();
    let mut lvol = 0.0;
    for (a, &k) in wa.iter_mut().zip(liq) {
        let gas = s.sp[k].boil <= tmin;
        *a = w.ground[k * n + i] * if gas { 1.0 } else { wet(s, k, i) };
        lvol += if gas { 0.0 } else { *a * s.sp[k].v[1] };
    }
    lvol
}

/// Share the liquid over the pores (`over`, m³, up to how far the head may fall) and the runoff
/// `hs` (m³) among the outlets by their weights.
fn runoff(o: &mut Outlets, over: f64, hs: f64, area: f64) {
    let (x, y) = (over.max(0.0).min(area * o.rmax) / o.wsum, hs / o.wsum);
    for c in 0..o.cnt {
        o.q[c] = (x * o.q[c]).min(area * o.room[c]) + y * o.q[c];
        (o.fall[c], o.qs[c]) = (o.nb[c].1, o.q[c]);
        o.qss += o.q[c];
    }
}

/// Pore liquid of node `i` flows by Darcy's law: through the ground the table fills, at `seep`
/// times the slope of its head (toward a lower table, no more than half way to a level one).
fn pore_flow(w: &World, s: &Scratch, i: usize, o: &mut Outlets, dt: f64) {
    let (area, zi, por) = (s.area, s.z[i], w.cfg.phys.porosity);
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
        let v = darcy(w.cfg.phys.seep * b, &w.grid, DIRS[d], dh, dt).min(0.5 * dh / (1.0 / si + 1.0 / sj));
        if v <= 0.0 {
            continue;
        }
        let c = (0..o.cnt).find(|&c| o.nb[c].0 == j).unwrap_or_else(|| {
            (o.nb[o.cnt], o.q[o.cnt], o.fall[o.cnt]) = ((j, 0.0), 0.0, 0.0);
            o.cnt += 1;
            o.cnt - 1
        });
        o.fall[c] = (o.fall[c] * o.q[c] + dh * v) / (o.q[c] + v);
        o.q[c] += v;
    }
}

/// Send the mobile liquid `wa` of the substances `liq` (of volume `lvol`) from node `i` to its
/// outlets, with its heat at `tcur` and its fusion enthalpy; less leaves if the node cannot pay.
/// Returns the power lost on the way, J, and the mol sent.
fn send(w: &mut World, s: &mut Scratch, i: usize, o: &Outlets, (liq, wa): (&[usize], &[f64]), lvol: f64, tcur: f64) -> (f64, f64) {
    let (n, g) = (w.grid.n(), w.cfg.gravity);
    let (mut power, mut sent) = (0.0, 0.0);
    let qsum: f64 = o.q[..o.cnt].iter().sum();
    let (part, inv) = ((qsum / lvol).min(1.0), 1.0 / qsum);
    for (&a, &k) in wa.iter().zip(liq).filter(|p| *p.0 > 0.0) {
        let (p, m) = (s.sp[k], (a * part).min(w.ground[k * n + i]));
        let (m, e) = crate::ground::take_paid(&mut w.e_ground[i], &mut w.ledger, s.cap[i], m, p.cp, m * (p.cp * tcur + p.lf));
        s.put(&mut w.ground, k, i, -m, true);
        sent += m;
        for ((&(j, _), &dh), &q) in o.nb[..o.cnt].iter().zip(&o.fall).zip(&o.q).filter(|p| *p.1 > 0.0) {
            let (m, e) = (m * q * inv, e * q * inv);
            w.e_ground[j] += e;
            s.put(&mut w.ground, k, j, m, true);
            power += m * p.mm * g * dh;
        }
    }
    (power, sent)
}

/// The work of the liquid on the bed of node `i` (`bedwork`, J): drop what is over capacity (all
/// of it where nothing flows on), else pick up loose solids `sol` and break bonded ground.
/// Returns whether there is a load to `carry` on to the outlets.
fn work_on_bed(w: &mut World, s: &mut Scratch, i: usize, o: &Outlets, sol: &[usize], bedwork: f64, tcur: f64) -> bool {
    let ns = sol.len();
    let load: f64 = sol.iter().enumerate().map(|(si, &k)| s.sed[i * ns + si] * s.sp[k].v[0]).sum();
    let capv = w.cfg.carry * bedwork * bare_share(w, i);
    if load <= 0.0 && capv < 1.0 {
        return false;
    }
    if load > capv || o.qss <= 0.0 {
        let f = if o.qss > 0.0 { 1.0 - capv / load } else { 1.0 };
        drop_load(w, s, i, sol, f);
    } else if bedwork > 0.0 {
        pick_up(w, s, i, sol, capv - load, tcur);
        wear(w, s, i, bedwork);
    }
    o.qss > 0.0
}

/// Drop a share `f` of the load carried into node `i`, with its heat.
fn drop_load(w: &mut World, s: &mut Scratch, i: usize, sol: &[usize], f: f64) {
    let ns = sol.len();
    for (si, &k) in sol.iter().enumerate() {
        let m = s.sed[i * ns + si] * f;
        s.sed[i * ns + si] -= m;
        s.put(&mut w.ground, k, i, m, false);
    }
    w.e_ground[i] += s.sed_e[i] * f;
    s.sed_e[i] -= s.sed_e[i] * f;
}

/// Pick up loose solids of node `i` up to `room` m³ more of load (no more than half of each),
/// with their heat at `tcur`; less is picked up if the node cannot pay.
fn pick_up(w: &mut World, s: &mut Scratch, i: usize, sol: &[usize], room: f64, tcur: f64) {
    let ns = sol.len();
    let avail: f64 = sol.iter().map(|&k| solid(w, s, k, i) * s.sp[k].v[0]).sum();
    let f = if avail > 0.0 { (room / avail).min(0.5) } else { 0.0 };
    for (si, &k) in sol.iter().enumerate() {
        if f > 0.0 {
            let m = solid(w, s, k, i) * f;
            let (m, e) = w.take_paid(i, m, s.sp[k].cp, m * s.sp[k].cp * tcur);
            s.put(&mut w.ground, k, i, -m, false);
            s.sed[i * ns + si] += m;
            s.sed_e[i] += e;
        }
    }
}

/// Break bonded ground of node `i` into loose by `bedwork` over bond strength (`wear`), each
/// substance by its share of the bed's volume, no more than half of it.
fn wear(w: &mut World, s: &mut Scratch, i: usize, bedwork: f64) {
    let n = w.grid.n();
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
            s.add_loose(k, false);
        }
    }
}

/// Carry the load of node `i` on with the runoff, its heat and the head it loses: into the load of
/// an outlet still to come, or onto the ground of one already done.
fn carry(w: &mut World, s: &mut Scratch, i: usize, o: &Outlets, sol: &[usize], heat: &mut f64) {
    let (ns, g) = (sol.len(), w.cfg.gravity);
    for (&(j, dh), &q) in o.nb[..o.cnt].iter().zip(&o.qs).filter(|p| *p.1 > 0.0) {
        let (f, mut mass) = (q / o.qss, 0.0);
        for (si, &k) in sol.iter().enumerate() {
            let m = s.sed[i * ns + si] * f;
            if m <= 0.0 {
                continue;
            }
            mass += m * s.sp[k].mm;
            if s.done[j] {
                s.put(&mut w.ground, k, j, m, false);
            } else {
                s.sed[j * ns + si] += m;
            }
        }
        let e = s.sed_e[i] * f + mass * g * dh;
        *heat += mass * g * dh;
        if s.done[j] {
            w.e_ground[j] += e;
        } else {
            s.sed_e[j] += e;
        }
    }
    s.sed[i * ns..(i + 1) * ns].fill(0.0);
    s.sed_e[i] = 0.0;
}
