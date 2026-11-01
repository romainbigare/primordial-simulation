//! The yearly part of Flow: creep of loose solids, and the regrowth of bodies into cleared ground.

use super::{bare_share, land::{lift, melted, molten, rise}, on_land, potential::apply, range, Scratch};
use crate::{store::{Batch, Class}, World};

/// Yearly: loose solids creep to lower neighbours, link by link, each from what the links before it left, booked in one
/// call, out of land cells (`on_land`) down their slopes; then bodies spread back into cleared links (`lanes::regrow`). Over each link they creep from the higher node
/// `a` to `b` with their heat, and the fall heats `b` (`potential`); they cross a border as wide as the side of the smaller of the two
/// cells (a quarter's beside a refined square), across a corner too.
pub(super) fn creep(w: &mut World, s: &mut Scratch, dt: f64) {
    let (n, area) = (w.mesh.cells().n(), w.mesh.cells().area());
    let sol = s.solids(range(&s.state.tg).0);
    let soft: Vec<f64> = sol.iter().map(|&k| 1.0 / (w.chem.subs[k].hardness + w.cfg.hard_min)).collect();
    // Each solid in each node, mol (`[q * n + i]`), and the volume able to creep by softness, m³, as the links so far
    // leave them; and what moves over each link with its heat.
    let mut amt: Vec<f64> = sol.iter().flat_map(|&k| w.store.amounts(Class::Ground)[k * n..(k + 1) * n].iter().copied()).collect();
    let (from, mut mob): (Vec<bool>, _) = ((0..n).map(|i| on_land(s, i)).collect(), vec![0.0; n]);
    for (q, &k) in sol.iter().enumerate() {
        mob.iter_mut().enumerate().filter(|p| from[p.0]).for_each(|(i, m)| *m += amt[q * n + i] * (1.0 - molten(s, k, i)) * s.sp[k].v[0] * soft[q]);
    }
    let ks: Vec<u16> = sol.iter().map(|&k| k as u16).collect();
    let mut moved = Batch::new([Class::Ground; 2], &ks);
    let cells = w.mesh.cells();
    for (l, (&i, &j)) in cells.a().iter().zip(cells.b()).enumerate() {
        let (i, j) = (i as usize, j as usize);
        let (a, b) = if s.z[i] > s.z[j] { (i, j) } else { (j, i) };
        let dz = s.z[a] - s.z[b];
        if !from[a] || mob[a] <= 0.0 || dz <= 0.0 {
            continue;
        }
        let r = w.cfg.creep * dz / cells.dist()[l] * area[a].min(area[b]).sqrt() * dt / area[a] * bare_share(w, a);
        // At most what levels the two surfaces: creep goes down the slope, never past flat.
        let f = (dz * area[a] * area[b] / (area[a] + area[b]) / (r * mob[a])).min(1.0);
        let (ta, tb) = (s.state.tg[a], s.state.tg[b]);
        // What moves, summed over substances: heat capacity, mobility, rise of each surface and head.
        let (mut c, mut slid, mut ua, mut ub) = (0.0, 0.0, [0.0; 2], [0.0; 2]);
        let mut out = vec![0.0; sol.len()];
        for (q, &k) in sol.iter().enumerate() {
            let x = amt[q * n + a] * (1.0 - molten(s, k, a));
            if x > 0.0 {
                let (p, m) = (&s.sp[k], (x * r * f * soft[q]).min(x));
                (amt[q * n + a], amt[q * n + b], out[q]) = (amt[q * n + a] - m, amt[q * n + b] + m, m);
                let (da, db) = (rise(p, ta, melted(s, k, a), -m, area[a]), rise(p, tb, melted(s, k, b), m, area[b]));
                (c, slid) = (c + m * p.cp, slid + m * p.v[0] * soft[q]);
                (ua, ub) = ([ua[0] + da[0], ua[1] + da[1]], [ub[0] + db[0], ub[1] + db[1]]);
            }
        }
        if c > 0.0 {
            (s.cap[a], s.cap[b], mob[a], mob[b]) = (s.cap[a] - c, s.cap[b] + c, mob[a] - slid, mob[b] + slid);
            // The heat of what moves goes with it.
            let u = moved.link((a, b), c * ta);
            sol.iter().zip(&out).for_each(|(&k, &m)| moved.carry(u, k as u16, m));
            lift(s, a, ua);
            lift(s, b, ub);
        }
    }
    apply(w, s, &[moved.flux()], &[]);
    crate::lanes::regrow(w, dt);
}
