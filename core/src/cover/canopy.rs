//! What the slots of a cell's cover share (laws.md 3): the ground they stand on, as rings of the shares their roots hold
//! (`rings`); the light, passing down through their crowns over those rings (`shade`); and the key liquid of each ring
//! (`drawn`). Each formula acts on the stand, so a kind cut into slots catches and draws as the whole.

use super::{Cover, SLOTS};
use crate::math::mean_decay;

/// The rings of ground that slots standing on nested shares `shares` of it make (a slot of a smaller share stands within
/// the ground of each larger one): each ring the share s its slots stand on at least, and its own share of the ground,
/// from the largest down; and how many rings.
pub(super) fn rings(shares: &[f64]) -> ([(f64, f64); SLOTS], usize) {
    let mut s = [0.0; SLOTS];
    s[..shares.len()].copy_from_slice(shares);
    s[..shares.len()].sort_by(|a, b| b.total_cmp(a));
    let mut n = 0;
    for k in 0..shares.len() {
        if n == 0 || s[k] != s[n - 1] {
            (s[n], n) = (s[k], n + 1);
        }
    }
    let (mut out, mut r) = ([(0.0, 0.0); SLOTS], 0);
    for k in 0..n {
        let ring = s[k] - if k + 1 < n { s[k + 1] } else { 0.0 };
        if ring > 0.0 {
            (out[r], r) = ((s[k], ring), r + 1);
        }
    }
    (out, r)
}

/// The share of the ground a slot of rooted share `s` stands on (no less than a tiny one).
fn share(s: f64) -> f64 {
    s.max(1e-300)
}

/// Light passing down through the crowns of slots over the ground they share (Beer's law): `l` holds per slot its
/// crown's base and top, m, its capture area per ground area x, the share s of the ground it stands on, and the slot. A
/// slot stands on its share s, the shares nested (a kind stands where the loose ground holds its roots, and ground that
/// holds deep roots holds shallow ones), with capture area x / s per unit of it spread evenly through its crown (a crown of
/// no depth a sheet at its top); so the ground falls into rings (`rings`), each held by the slots of a share at least
/// its own, and in each the light passes down through the capture area of all the crowns at each height, whatever slot
/// holds it, absorbing 1 - exp(-y) of what reaches it over capture area y per unit of ground (with every s 1, Beer's law
/// through the canopy). `set(slot, f, bottom, depth)` gets each slot's light per unit of capture area over the mean of
/// all (weighted by capture area, the factors have mean 1), the light at its crown's base per unit of capture area over
/// the same mean, and the capture area per unit of ground the light passes through its crown (ln of the light at its top
/// over that at its base): each the mean over its rings by its capture area in each.
pub(super) fn shade(l: &[(f64, f64, f64, f64, usize)], mut set: impl FnMut(usize, f64, f64, f64)) {
    let mut got = [[0.0; 3]; SLOTS];
    // The slots on a ring, the heights of their crowns, and the light each slot's capture area gets and what the light
    // has passed at each one's top and base there.
    let (mut on, mut heights) = ([0; SLOTS], [0.0; 2 * SLOTS]);
    let (mut lit, mut top, mut base) = ([0.0; SLOTS], [0.0; SLOTS], [0.0; SLOTS]);
    let mut shares = [0.0; SLOTS];
    l.iter().zip(shares.iter_mut()).for_each(|(m, s)| *s = share(m.3));
    let (rings, count) = rings(&shares[..l.len()]);
    for &(at_least, ring) in &rings[..count] {
        let mut n_on = 0;
        for q in (0..l.len()).filter(|&q| share(l[q].3) >= at_least) {
            (on[n_on], n_on) = (q, n_on + 1);
        }
        let on = &on[..n_on];
        let mut n_h = 0;
        for &q in on {
            (heights[n_h], heights[n_h + 1], n_h) = (l[q].0, l[q].1, n_h + 2);
        }
        heights[..n_h].sort_by(|a, b| b.total_cmp(a));
        let mut n_dist = 0;
        for k in 0..n_h {
            if n_dist == 0 || heights[k] != heights[n_dist - 1] {
                (heights[n_dist], n_dist) = (heights[k], n_dist + 1);
            }
        }
        let heights = &heights[..n_dist];
        [&mut lit, &mut top, &mut base].into_iter().for_each(|v| v.fill(0.0));
        // The capture area per unit of ground the light has passed.
        let mut tau = 0.0f64;
        for (k, &h) in heights.iter().enumerate() {
            on.iter().filter(|&&q| l[q].1 == h).for_each(|&q| top[q] = tau);
            // The crowns of no depth at this height: one sheet.
            let sheet = |q: usize| l[q].0 == h && l[q].1 == h;
            if on.iter().any(|&q| sheet(q)) {
                let y: f64 = on.iter().filter(|&&q| sheet(q)).map(|&q| l[q].2 / share(l[q].3)).sum();
                let in_sheet = (-tau).exp() * mean_decay(y);
                on.iter().filter(|&&q| sheet(q)).for_each(|&q| lit[q] += in_sheet);
                tau += y;
                on.iter().filter(|&&q| sheet(q)).for_each(|&q| base[q] = tau);
            }
            let Some(&below) = heights.get(k + 1) else { break };
            // The crowns through the span down to the next height, each its capture area spread over its depth.
            let through = |q: usize| l[q].0 <= below && l[q].1 >= h && l[q].1 > l[q].0;
            let part = |q: usize| (h - below) / (l[q].1 - l[q].0);
            let y: f64 = on.iter().filter(|&&q| through(q)).map(|&q| l[q].2 / share(l[q].3) * part(q)).sum();
            let (reach, mean) = ((-tau).exp(), mean_decay(y));
            on.iter().filter(|&&q| through(q)).for_each(|&q| lit[q] += part(q) * reach * mean);
            tau += y;
            on.iter().filter(|&&q| l[q].0 == below && l[q].1 > below).for_each(|&q| base[q] = tau);
        }
        for &q in on {
            let part = ring / share(l[q].3);
            got[q] = [got[q][0] + part * lit[q], got[q][1] + part * (-base[q]).exp(), got[q][2] + part * (base[q] - top[q])];
        }
    }
    let (sum_x, sum_a) = got.iter().zip(l.iter()).fold((0.0, 0.0), |(sx, sa), (g, m)| (sx + m.2, sa + m.2 * g[0]));
    let all = if sum_x > 0.0 { sum_a / sum_x } else { 1.0 };
    got.iter().zip(l.iter()).for_each(|(g, m)| set(m.4, g[0] / all.max(1e-300), g[1] / all.max(1e-300), g[2]));
}

/// The units of the light reaction `x` the slots in use of cover `c` would run, cut to the key liquid their roots reach:
/// the liquid of each ring of its ground (`rings`, by the shares the slots' plants hold, `Cover::on`) is the slots'
/// that stand on it, `units` of the reaction a unit of the ground in all; where they would draw more there, each draws
/// it in proportion to what it would draw there. So the slots of a cell draw on the ground they share, whatever slots
/// hold them.
pub(super) fn drawn(c: &Cover, x: &mut [f64], units: f64) {
    let rooted = |q: usize| share(c.on(q));
    let mut shares = [0.0; SLOTS];
    (0..x.len()).for_each(|q| shares[q] = rooted(q));
    let mut got = [0.0; SLOTS];
    let (rings, count) = rings(&shares[..x.len()]);
    for &(at_least, ring) in &rings[..count] {
        let on = || (0..x.len()).filter(|&q| rooted(q) >= at_least);
        let f = (units * ring / on().map(|q| x[q] * ring / rooted(q)).sum::<f64>().max(1e-300)).min(1.0);
        on().for_each(|q| got[q] += x[q] * ring / rooted(q) * f);
    }
    x.copy_from_slice(&got[..x.len()]);
}
