//! The yearly part of Flow: creep of loose solids, and the regrowth of bodies into cleared ground.

use super::land::{lift, melted, molten, rise};
use super::{bare_share, range, Scratch};
use crate::store::{apply, Batch, Class};
use crate::World;

/// Added to a substance's hardness before its softness (1 / hardness) is taken.
const HARD_MIN: f64 = 0.05;
/// Most volume that creeps over an edge per call, as a share of the step between the surfaces times the cell's area.
const CREEP_LEVEL: f64 = 0.25;
/// Most of each loose solid that creeps over an edge per call.
const CREEP_MOST: f64 = 0.25;

/// Yearly: loose solids creep to lower neighbours, link by link, each from what the links before it left, booked in one
/// call; then bodies spread back into cleared links (`lanes::regrow`). Over each link they creep from the higher node
/// `a` to `b` with their heat, and the fall heats `b`; they cross a border as wide as the side of `a`, across a corner
/// too.
pub(super) fn creep(w: &mut World, s: &mut Scratch, dt: f64) {
    let (n, area, g) = (w.mesh.cells().n(), w.mesh.cells().area(), w.cfg.gravity);
    let sol = s.solids(range(&s.state.tg).0);
    let soft: Vec<f64> = sol.iter().map(|&k| 1.0 / (w.chem.subs[k].hardness + HARD_MIN)).collect();
    // Each solid in each node, mol (`[q * n + i]`), and the volume able to creep by softness, m³, as the links so far
    // leave them; what moves over each link with its heat, and the fall that heats the lower node.
    let mut amt: Vec<f64> = sol.iter().flat_map(|&k| w.store.amounts(Class::Ground)[k * n..(k + 1) * n].iter().copied()).collect();
    let mut mob = vec![0.0; n];
    for (q, &k) in sol.iter().enumerate() {
        for (i, m) in mob.iter_mut().enumerate() {
            *m += amt[q * n + i] * (1.0 - molten(s, k, i)) * s.sp[k].v[0] * soft[q];
        }
    }
    let ks: Vec<u16> = sol.iter().map(|&k| k as u16).collect();
    let (mut moved, mut fall) = (Batch::new([Class::Ground; 2], &ks), Batch::falls(Class::Ground));
    let cells = w.mesh.cells();
    for (l, (&i, &j)) in cells.a().iter().zip(cells.b()).enumerate() {
        let (i, j) = (i as usize, j as usize);
        let (a, b) = if s.z[i] > s.z[j] { (i, j) } else { (j, i) };
        let dz = s.z[a] - s.z[b];
        if mob[a] <= 0.0 || dz <= 0.0 {
            continue;
        }
        let r = w.cfg.creep * dz / cells.dist()[l] * area[a].sqrt() * dt / area[a] * bare_share(w, a);
        let f = (area[a] * dz * CREEP_LEVEL / (r * mob[a])).min(1.0);
        let (ta, tb) = (s.state.tg[a], s.state.tg[b]);
        // What moves, summed over substances: heat capacity, mass, mobility, rise of each surface and head.
        let (mut c, mut mass, mut slid, mut ua, mut ub) = (0.0, 0.0, 0.0, [0.0; 2], [0.0; 2]);
        let mut out = vec![0.0; sol.len()];
        for (q, &k) in sol.iter().enumerate() {
            let x = amt[q * n + a] * (1.0 - molten(s, k, a));
            if x > 0.0 {
                let (p, m) = (&s.sp[k], (x * r * f * soft[q]).min(CREEP_MOST * x));
                (amt[q * n + a], amt[q * n + b], out[q]) = (amt[q * n + a] - m, amt[q * n + b] + m, m);
                let (da, db) = (rise(p, ta, melted(s, k, a), -m, area[a]), rise(p, tb, melted(s, k, b), m, area[b]));
                (c, mass, slid) = (c + m * p.cp, mass + m * p.mm, slid + m * p.v[0] * soft[q]);
                (ua, ub) = ([ua[0] + da[0], ua[1] + da[1]], [ub[0] + db[0], ub[1] + db[1]]);
            }
        }
        if c > 0.0 {
            (s.cap[a], s.cap[b], mob[a], mob[b]) = (s.cap[a] - c, s.cap[b] + c, mob[a] - slid, mob[b] + slid);
            // The heat of what moves goes with it.
            let u = moved.link((a, b), c * ta);
            sol.iter().zip(&out).for_each(|(&k, &m)| moved.carry(u, k as u16, m));
            fall.fall(b, mass * g * dz / 2.0);
            lift(s, a, ua);
            lift(s, b, ub);
        }
    }
    apply(w, &[moved.flux(), fall.flux()], &[], &[]);
    crate::lanes::regrow(w, dt);
}
