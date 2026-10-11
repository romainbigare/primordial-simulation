//! Law 1: Flow. Stuff moves along edges from high push to low push, with loss.
//!
//! Ground nodes (here): liquid runs down its head in one ordered sweep; the head it loses
//! heats the ground and does work, carrying loose solids and breaking weak bonds; loose
//! solids creep downhill and heat conducts. Air nodes (`air`): radiation, contact heat,
//! evaporation and condensation, and gas moved by pressure, overturning and mixing.
//! Every move takes from one node exactly what it gives to another, with its heat, and
//! books matter lifted or dropped in the gravity ledger, so elements and energy are kept.

mod air;

use crate::grid::{DIRS, FORWARD};
use crate::World;

/// Thickness of a layer that hides half of what lies below it, m: [soft solid, liquid, loose solid].
const HIDE_M: [f64; 3] = [0.01, 0.5, 0.02];
/// A solid warmer than this share of its melting point is soft cover lying on top.
const SOFT: f64 = 0.75;

/// Numbers of one substance the laws read often: heat capacity, molar mass, molar volume
/// as [solid, liquid], melting and boiling point, reflectance as [solid, liquid].
#[derive(Clone, Copy, Debug, Default)]
struct Sp {
    cp: f64,
    mm: f64,
    v: [f64; 2],
    melt: f64,
    boil: f64,
    refl: [f64; 2],
}

/// Working memory of the law, rebuilt from the world when missing.
#[derive(Clone, Debug, Default)]
pub struct Scratch {
    month: Option<u64>,
    year: Option<u64>,
    area: f64,
    sp: Vec<Sp>,
    /// Substances present in ground, bonded ground and air nodes. Loose ones that stay solid
    /// and cannot evaporate at any ground temperature are `fixed`: their volume and volume ×
    /// reflectance per node are kept for a month in `fix`; the others `vary`.
    loose: Vec<usize>,
    fixed: Vec<usize>,
    vary: Vec<usize>,
    fix: Vec<[f64; 2]>,
    bedk: Vec<usize>,
    gas: Vec<usize>,
    // Ground nodes: bonded ground [heat capacity of the skin, volume, reflectance,
    // conductivity]; heat capacity, temperature, solid surface, head, reflectance.
    bed: Vec<[f64; 4]>,
    cap: Vec<f64>,
    tg: Vec<f64>,
    z: Vec<f64>,
    head: Vec<f64>,
    alb: Vec<f64>,
    /// Volume and volume × reflectance of [soft solid, liquid, loose solid].
    lay: Vec<[f64; 6]>,
    rain: Vec<f64>,
    order: Vec<u64>,
    done: Vec<bool>,
    /// Loose solids carried by liquid, [cell * carried kinds + kind], and their heat.
    sed: Vec<f64>,
    sed_e: Vec<f64>,
    mob: Vec<f64>,
    // Ground node to air: own node, four nearest nodes and weights, share of a node; the air
    // seen: [temperature touching the ground, heat-absorbing share, radiation down].
    air_of: Vec<u32>,
    near: Vec<[u32; 4]>,
    wt: Vec<[f64; 4]>,
    per: Vec<f64>,
    view: Vec<[f64; 3]>,
    snap: Vec<f64>,
    // Air nodes: ground cells, mean ground height, volume, amount, heat capacity, mass,
    // heat-absorbing share, temperature drop per metre, mean height of the air's mass,
    // temperature, pressure, warmth at equal height and its gradient.
    cells: Vec<f64>,
    za: Vec<f64>,
    vol: Vec<f64>,
    nt: Vec<f64>,
    capa: Vec<f64>,
    mass: Vec<f64>,
    fabs: Vec<f64>,
    gam: Vec<f64>,
    zair: Vec<f64>,
    ta: Vec<f64>,
    p: Vec<f64>,
    th: Vec<f64>,
    grad: Vec<[f64; 2]>,
    /// Per gas [g * na + j]: may it condense in the column; height its amount fills near the
    /// ground, m; share of it in the upper half of the column.
    cond: Vec<bool>,
    hk: Vec<f64>,
    split: Vec<f64>,
    // One line of air nodes: pushes [mix, overturn, even] per edge, and solver space.
    push: Vec<[f64; 3]>,
    al: Vec<f64>,
    be: Vec<f64>,
    x: Vec<f64>,
    cw: Vec<f64>,
    mflux: Vec<f64>,
    lh: Vec<[f64; 2]>,
    xs: Vec<f64>,
    fl: Vec<f64>,
    nt_new: Vec<f64>,
}

/// Advance flow by `dt` seconds.
pub fn step(w: &mut World, dt: f64) {
    let mut s = std::mem::take(&mut w.scratch);
    let month = w.day / w.cfg.days_per_month as u64;
    let monthly = s.month != Some(month);
    if s.cap.len() != w.grid.n() {
        refresh(w, &mut s);
    }
    ground_state(w, &mut s);
    air::state(w, &mut s);
    air::radiation(w, &mut s, dt);
    air::exchange(w, &mut s, dt);
    air::wind(w, &mut s, dt);
    liquid(w, &mut s, dt, if monthly { w.cfg.days_per_month as f64 * 86_400.0 / dt } else { 0.0 });
    let year = w.day / w.days_per_year();
    if s.year != Some(year) {
        if s.year.is_some() {
            creep(w, &mut s, w.days_per_year() as f64 * 86_400.0);
        }
        s.year = Some(year);
    }
    if monthly {
        refresh(w, &mut s);
        s.month = Some(month);
    }
    w.scratch = s;
}

/// Is substance `k` liquid at temperature `t`?
#[inline]
fn wet(s: &Scratch, k: usize, t: f64) -> bool {
    t >= s.sp[k].melt && t < s.sp[k].boil
}

/// Raise (or with negative `m`, lower) the surface of ground node `i` for `m` mol of `k`.
#[inline]
fn settle(s: &mut Scratch, k: usize, i: usize, m: f64) {
    let p = s.sp[k];
    if s.tg[i] < p.melt {
        let dz = m * p.v[0] / s.area;
        s.z[i] += dz;
        s.head[i] += dz;
    } else if s.tg[i] < p.boil {
        s.head[i] += m * p.v[1] / s.area;
    }
}

/// Move `m` mol of substance `k` from ground node `i` to `j`, with its heat at `t`.
#[inline]
fn shift(w: &mut World, s: &mut Scratch, k: usize, i: usize, j: usize, m: f64, t: f64) {
    let (n, cp) = (w.grid.n(), s.sp[k].cp);
    w.ground[k * n + i] -= m;
    w.ground[k * n + j] += m;
    s.cap[i] -= m * cp;
    s.cap[j] += m * cp;
    w.e_ground[i] -= m * cp * t;
    w.e_ground[j] += m * cp * t;
    settle(s, k, i, -m);
    settle(s, k, j, m);
}

fn range(v: &[f64]) -> (f64, f64) {
    v.iter().fold((f64::MAX, f64::MIN), |(a, b), &x| (a.min(x), b.max(x)))
}

/// Neighbours of cell `i` along `dirs`, or `usize::MAX` off the map.
#[inline]
fn around<const N: usize>(dirs: &[(i32, i32); N], gw: usize, gh: usize, i: usize) -> [usize; N] {
    let (x, y) = ((i % gw) as i32, (i / gw) as i32);
    let inside = x > 0 && y > 0 && x + 1 < gw as i32 && y + 1 < gh as i32;
    dirs.map(|(dx, dy)| {
        let (nx, ny) = (x + dx, y + dy);
        if inside || (nx >= 0 && ny >= 0 && (nx as usize) < gw && (ny as usize) < gh) {
            ny as usize * gw + nx as usize
        } else {
            usize::MAX
        }
    })
}

/// Monthly: which substances are present, the slow-changing bonded ground and fixed solids,
/// and (once) the map from ground cells to air nodes.
fn refresh(w: &World, s: &mut Scratch) {
    let (n, na, ns, g) = (w.grid.n(), w.grid.na(), w.chem.ns(), &w.grid);
    let mean = |v: [f32; 3]| v.iter().map(|&x| x as f64).sum::<f64>() / 3.0;
    s.area = g.area();
    s.sp = (w.chem.subs.iter())
        .map(|b| Sp {
            cp: b.cp,
            mm: b.molar_mass,
            v: [b.molar_mass / b.density[0], b.molar_mass / b.density[1]],
            melt: b.melt,
            boil: b.boil,
            refl: [mean(b.refl[0]), mean(b.refl[1])],
        })
        .collect();
    let any = |v: &[f64], len: usize, k: usize| v[k * len..(k + 1) * len].iter().any(|&x| x > 0.0);
    let tmax = (0..n).map(|i| w.t_ground(i)).fold(0.0, f64::max) + 20.0;
    let up = |k: usize| w.chem.p_sat(k, tmax) > 1.0;
    s.loose = (0..ns).filter(|&k| any(&w.ground, n, k)).collect();
    s.fixed = s.loose.iter().copied().filter(|&k| SOFT * s.sp[k].melt > tmax && !up(k)).collect();
    s.vary = s.loose.iter().copied().filter(|k| !s.fixed.contains(k)).collect();
    s.bedk = (0..ns).filter(|&k| any(&w.bed, n, k)).collect();
    s.gas = (0..ns).filter(|&k| any(&w.air, na, k) || (s.loose.contains(&k) && up(k))).collect();
    s.fix = vec![[0.0; 2]; n];
    for &k in &s.fixed {
        let (v, r) = (s.sp[k].v[0], s.sp[k].refl[0]);
        for (f, &a) in s.fix.iter_mut().zip(&w.ground[k * n..(k + 1) * n]) {
            f[0] += a * v;
            f[1] += a * v * r;
        }
    }
    s.bed = (0..n)
        .map(|i| {
            let (mut c, mut v, mut r, mut kc) = (0.0, 0.0, 0.0, 0.0);
            for &k in &s.bedk {
                let (b, sub) = (w.bed[k * n + i], &w.chem.subs[k]);
                c += b * sub.cp;
                v += b * s.sp[k].v[0];
                r += b * s.sp[k].v[0] * s.sp[k].refl[0];
                kc += b * s.sp[k].v[0] * sub.conductivity;
            }
            let c = if v > 0.0 { c / v * w.cfg.skin_m * s.area } else { 0.0 };
            [c, v, r / v.max(1e-9), kc / v.max(1e-9)]
        })
        .collect();
    if s.air_of.len() == n {
        return;
    }
    s.air_of = (0..n).map(|i| g.air_of(i) as u32).collect();
    let axis = |c: usize, len: usize| {
        let f = ((c as f64 + 0.5) / g.air_scale as f64 - 0.5).clamp(0.0, (len - 1) as f64);
        let a = (f as usize).min(len.saturating_sub(2));
        (a, (a + 1).min(len - 1), f - a as f64)
    };
    (s.near, s.wt) = (0..n)
        .map(|i| {
            let (x, y) = g.xy(i);
            let ((x0, x1, tx), (y0, y1, ty)) = (axis(x, g.aw()), axis(y, g.ah()));
            let j = |x: usize, y: usize| (y * g.aw() + x) as u32;
            ([j(x0, y0), j(x1, y0), j(x0, y1), j(x1, y1)], [(1.0 - tx) * (1.0 - ty), tx * (1.0 - ty), (1.0 - tx) * ty, tx * ty])
        })
        .unzip();
    s.cells = vec![0.0; na];
    for &j in &s.air_of {
        s.cells[j as usize] += 1.0;
    }
    s.per = (0..n).map(|i| 1.0 / air::lerp(s, &s.cells, i)).collect();
    for v in [&mut s.cap, &mut s.tg, &mut s.z, &mut s.head, &mut s.alb, &mut s.rain, &mut s.sed_e, &mut s.mob] {
        *v = vec![0.0; n];
    }
    for v in [&mut s.za, &mut s.vol, &mut s.nt, &mut s.capa, &mut s.mass, &mut s.fabs, &mut s.gam, &mut s.zair] {
        *v = vec![0.0; na];
    }
    for v in [&mut s.ta, &mut s.p, &mut s.th] {
        *v = vec![0.0; na];
    }
    (s.lay, s.view, s.done, s.grad) = (vec![[0.0; 6]; n], vec![[0.0; 3]; n], vec![false; n], vec![[0.0; 2]; na]);
}

/// Heat capacity, temperature, surface, head and reflectance of every ground node.
fn ground_state(w: &World, s: &mut Scratch) {
    let (n, area) = (w.grid.n(), s.area);
    for i in 0..n {
        s.cap[i] = s.bed[i][0];
    }
    for &k in &s.loose {
        let cp = s.sp[k].cp;
        for (c, &a) in s.cap.iter_mut().zip(&w.ground[k * n..(k + 1) * n]) {
            *c += a * cp;
        }
    }
    for i in 0..n {
        s.tg[i] = w.e_ground[i] / s.cap[i].max(1.0);
        s.z[i] = w.base_z + (s.bed[i][1] + s.fix[i][0]) / area;
        s.lay[i] = [0.0, 0.0, 0.0, 0.0, s.fix[i][0], s.fix[i][1]];
    }
    for &k in &s.vary {
        let p = s.sp[k];
        for (i, &a) in w.ground[k * n..(k + 1) * n].iter().enumerate() {
            let (t, l) = (s.tg[i], &mut s.lay[i]);
            if a <= 0.0 || t >= p.boil {
                continue;
            }
            let (ph, o) = if t >= p.melt { (1, 2) } else if t > SOFT * p.melt { (0, 0) } else { (0, 4) };
            l[o] += a * p.v[ph];
            l[o + 1] += a * p.v[ph] * p.refl[ph];
            if ph == 0 {
                s.z[i] += a * p.v[0] / area;
            }
        }
    }
    for i in 0..n {
        let l = s.lay[i];
        s.head[i] = s.z[i] + l[2] / area;
        let (mut a, mut seen) = (0.0, 1.0);
        for c in 0..3 {
            if l[2 * c] > 0.0 {
                let d = l[2 * c] / area;
                let cover = d / (d + HIDE_M[c]);
                a += seen * cover * l[2 * c + 1] / l[2 * c];
                seen *= 1.0 - cover;
            }
        }
        s.alb[i] = a + seen * s.bed[i][2];
    }
}

/// Liquid by head, as one ordered sweep from the highest head down: each node passes liquid
/// to its lower neighbours, more to the steeper, so it can cross the map in one sweep while
/// depressions fill and spill. The head lost heats the node. Once a month (`work` > 0, the
/// month's share of this call) the power lost also does work: it carries loose solids up to
/// a capacity, drops the rest, and breaks bonded ground by power over bond strength.
fn liquid(w: &mut World, s: &mut Scratch, dt: f64, work: f64) {
    let (n, gw, gh, area, g) = (w.grid.n(), w.grid.w, w.grid.h, s.area, w.cfg.gravity);
    let dist = DIRS.map(|d| w.grid.dist(d));
    let (tmin, tmax) = range(&s.tg);
    let liq: Vec<usize> = s.loose.iter().copied().filter(|&k| s.sp[k].melt <= tmax && s.sp[k].boil > tmin).collect();
    let sol: Vec<usize> = s.loose.iter().copied().filter(|&k| s.sp[k].melt > tmin).collect();
    let ns = sol.len();
    s.sed.resize(n * ns, 0.0);
    // Highest head first (heads to 0.1 m, then by cell). The order is kept for a month: any
    // order is safe, it only sets how far liquid gets in one call.
    if work > 0.0 || s.order.len() != n {
        s.order = s.head.iter().enumerate().map(|(i, &h)| (((2e4 - h).max(0.0) * 10.0) as u64) << 20 | i as u64).collect();
        s.order.sort_unstable();
    }
    s.done.fill(false);
    let ema = (dt / (w.days_per_year() as f64 * 86_400.0)) as f32;
    let mut heat = 0.0;
    for r in 0..n {
        let i = (s.order[r] & 0xf_ffff) as usize;
        let t = s.tg[i];
        s.done[i] = true;
        let lvol: f64 = liq.iter().filter(|&&k| wet(s, k, t)).map(|&k| w.ground[k * n + i] * s.sp[k].v[1]).sum();
        if lvol <= 0.0 && work <= 0.0 {
            w.diag.liquid_flux[i] *= 1.0 - ema;
            continue;
        }
        // Lower neighbours, and how far this node's head may fall for each: down to a
        // neighbour still to come (which passes the liquid on) or to its own bed; half way
        // to one already done, so heads never cross.
        let (mut nb, mut q, mut room) = ([(0usize, 0.0f64); 8], [0.0f64; 8], [0.0f64; 8]);
        let (mut cnt, mut wsum, mut rmax) = (0, 0.0, 0.0f64);
        for (d, j) in around(&DIRS, gw, gh, i).into_iter().enumerate() {
            let dh = if j == usize::MAX { 0.0 } else { s.head[i] - s.head[j] };
            if dh > 0.0 {
                nb[cnt] = (j, dh);
                room[cnt] = if s.done[j] { 0.5 * dh } else { s.head[i] - s.head[j].max(s.z[i]) };
                q[cnt] = dh / dist[d];
                wsum += q[cnt];
                rmax = rmax.max(room[cnt]);
                cnt += 1;
            }
        }
        let tcur = w.e_ground[i] / s.cap[i];
        let (mut qsum, mut power, mut out) = (0.0, 0.0, 0.0);
        if lvol > 0.0 && cnt > 0 {
            let x = lvol.min(area * rmax);
            for c in 0..cnt {
                q[c] = (x * q[c] / wsum).min(area * room[c]);
                qsum += q[c];
            }
            for &k in &liq {
                let a = w.ground[k * n + i];
                if a <= 0.0 || !wet(s, k, t) {
                    continue;
                }
                for (&(j, dh), &q) in nb[..cnt].iter().zip(&q) {
                    let m = (a * q / lvol).min(w.ground[k * n + i]);
                    shift(w, s, k, i, j, m, tcur);
                    power += m * s.sp[k].mm * g * dh;
                    out += m;
                }
            }
        }
        w.e_ground[i] += power;
        heat += power;
        w.diag.liquid_flux[i] += ((out / dt) as f32 - w.diag.liquid_flux[i]) * ema;
        if work <= 0.0 {
            continue;
        }

        // The month's work, on the bed below the liquid left standing (which hides it): drop
        // what is over capacity (all of it where nothing flows on), else pick up loose solids
        // and break bonded ground; pass the load on with the flow.
        let bedwork = power * work * HIDE_M[1] / (HIDE_M[1] + (lvol - qsum).max(0.0) / area);
        let load: f64 = sol.iter().enumerate().map(|(si, &k)| s.sed[i * ns + si] * s.sp[k].v[0]).sum();
        let capv = w.cfg.carry * bedwork;
        if load > capv || qsum <= 0.0 {
            let f = if qsum > 0.0 { 1.0 - capv / load } else { 1.0 };
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
            let avail: f64 = sol.iter().filter(|&&k| t < s.sp[k].melt).map(|&k| w.ground[k * n + i] * s.sp[k].v[0]).sum();
            let f = if avail > 0.0 { ((capv - load) / avail).min(0.5) } else { 0.0 };
            for (si, &k) in sol.iter().enumerate() {
                if t < s.sp[k].melt && f > 0.0 {
                    let m = w.ground[k * n + i] * f;
                    w.ground[k * n + i] -= m;
                    s.sed[i * ns + si] += m;
                    s.cap[i] -= m * s.sp[k].cp;
                    w.e_ground[i] -= m * s.sp[k].cp * tcur;
                    s.sed_e[i] += m * s.sp[k].cp * tcur;
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
        if qsum <= 0.0 {
            continue;
        }
        for (&(j, dh), &q) in nb[..cnt].iter().zip(&q) {
            let (f, mut mass) = (q / qsum, 0.0);
            for (si, &k) in sol.iter().enumerate() {
                let m = s.sed[i * ns + si] * f;
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

/// Yearly: loose solids creep to lower neighbours, softer ones faster and thicker layers
/// more; heat conducts between neighbouring ground nodes.
fn creep(w: &mut World, s: &mut Scratch, dt: f64) {
    let (n, area, g, cell) = (w.grid.n(), s.area, w.cfg.gravity, w.grid.cell_m);
    let (tmin, _) = range(&s.tg);
    let sol: Vec<usize> = s.loose.iter().copied().filter(|&k| s.sp[k].melt > tmin).collect();
    let soft: Vec<f64> = (0..w.chem.ns()).map(|k| 1.0 / (w.chem.subs[k].hardness + 0.05)).collect();
    s.mob.fill(0.0);
    for &k in &sol {
        for i in 0..n {
            if s.tg[i] < s.sp[k].melt {
                s.mob[i] += w.ground[k * n + i] * s.sp[k].v[0] * soft[k];
            }
        }
    }
    let mut heat = 0.0;
    for i in 0..n {
        for (d, j) in around(&FORWARD, w.grid.w, w.grid.h, i).into_iter().enumerate() {
            if j == usize::MAX {
                continue;
            }
            let dist = w.grid.dist(FORWARD[d]);
            let (ti, tj) = (w.e_ground[i] / s.cap[i], w.e_ground[j] / s.cap[j]);
            let gk = (s.bed[i][3] + s.bed[j][3]) / 2.0 * w.cfg.skin_m * cell / dist;
            let qmax = 0.2 * (ti - tj).abs() * s.cap[i].min(s.cap[j]);
            let qh = (gk * (ti - tj) * dt).clamp(-qmax, qmax);
            w.e_ground[i] -= qh;
            w.e_ground[j] += qh;

            let (a, b) = if s.z[i] > s.z[j] { (i, j) } else { (j, i) };
            let dz = s.z[a] - s.z[b];
            if s.mob[a] <= 0.0 || dz <= 0.0 {
                continue;
            }
            let r = w.cfg.creep * dz / dist * cell * dt / area;
            let f = (area * dz / 4.0 / (r * s.mob[a])).min(1.0);
            let ta = w.e_ground[a] / s.cap[a];
            for &k in &sol {
                let x = w.ground[k * n + a];
                if x > 0.0 && s.tg[a] < s.sp[k].melt {
                    let m = (x * r * f * soft[k]).min(0.25 * x);
                    shift(w, s, k, a, b, m, ta);
                    let drop = m * s.sp[k].mm * g * dz / 2.0;
                    w.e_ground[b] += drop;
                    heat += drop;
                    s.mob[a] -= m * s.sp[k].v[0] * soft[k];
                    s.mob[b] += m * s.sp[k].v[0] * soft[k];
                }
            }
        }
    }
    w.ledger.gravity += heat;
}
