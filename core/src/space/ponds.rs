//! The closed basins of the terrain's samples (laws.md 0 and 8): the samples under a basin's level (`space::drain`),
//! where liquid stands at one level before it runs over the basin's lowest pass, and the load the wear at the world's
//! start (`space::wear`) carries into them, which fills them from the floor up. A basin the load fills is a plain;
//! one it leaves is a pond (`samples::Pond`). The parts of the open low ground are ponds too, of no pass (`open_parts`).

use super::drain::Flood;
use super::grid::Grid;
use super::network::NONE;
use super::samples::Pond;
use crate::math::ways_down;

/// The least depth under its level of a closed basin's deepest sample that the floods over a plain leave, m.
const GRADED_M: f64 = 1.0;

/// The parts of the open low ground `open` on grid `g` joined through their samples, as ponds after those in `ponds`
/// (`samples::Pond`: no pass, an infinite level), each sample's in `pond`.
pub(super) fn open_parts(g: &Grid, open: &[bool], (pond, ponds): (&mut [u32], &mut Vec<Pond>)) {
    for k in 0..g.n() {
        if !open[k] || pond[k] != NONE {
            continue;
        }
        let id = ponds.len() as u32;
        ponds.push(Pond { pass: NONE, level: f64::INFINITY });
        let mut todo = vec![k];
        pond[k] = id;
        while let Some(i) = todo.pop() {
            for (j, _) in g.around(i) {
                if open[j] && pond[j] == NONE {
                    pond[j] = id;
                    todo.push(j);
                }
            }
        }
    }
}

/// The closed basins of the flood `f` over heights `z` (`Flood::under`): of each sample the basin it lies in
/// (`NONE` where none), and of each basin its samples, the first the one the flood reached first: samples under a
/// level beside each other are one basin, as they stand at one level.
pub(super) fn basins(g: &Grid, z: &[f64], f: &Flood) -> (Vec<u32>, Vec<Vec<u32>>) {
    let mut of = vec![NONE; g.n()];
    let mut all: Vec<Vec<u32>> = Vec::new();
    for &k in &f.order {
        if of[k as usize] != NONE || !f.under(z, k as usize) {
            continue;
        }
        let id = all.len() as u32;
        let mut members = vec![k];
        of[k as usize] = id;
        let mut next = 0;
        while let Some(&i) = members.get(next) {
            next += 1;
            for (j, _) in g.around(i as usize) {
                if of[j] == NONE && f.under(z, j) {
                    of[j] = id;
                    members.push(j as u32);
                }
            }
        }
        all.push(members);
    }
    (of, all)
}

/// The closed basins `basins` of the flood `f` over heights `h` whose deepest sample lies less than `GRADED_M` under
/// their level, filled to it: the floods that spread a plain's load leave it graded, with no hollow so shallow (the
/// flood's ways over it stay).
pub(super) fn grade(h: &mut [f64], f: &Flood, basins: &[Vec<u32>]) {
    for members in basins {
        let level = f.level[members[0] as usize];
        if members.iter().all(|&k| level - h[k as usize] < GRADED_M) {
            members.iter().for_each(|&k| h[k as usize] = level);
        }
    }
}

/// Where the load of a step of `dt` years settles (each sample's rise at `up` less how far it rose from heights `was`,
/// m³; none on the open low ground): in the flood's order from the last reached, each sample's load spreads down as
/// the liquid does (`math::ways_down`, on the flood's surface), so it reaches every hollow below what was cut; a closed
/// basin keeps all that comes into it, and once all is in, at its first sample, fills its floor from the lowest up
/// toward the height its pass has now (`fill`), and what it cannot hold runs on over the pass. What reaches the open
/// low ground leaves the land.
pub(super) fn settle(g: &Grid, h: &mut [f64], f: &Flood, was: &[f64], (up, open): (&[f64], &[bool]), dt: f64) {
    let (of, basins) = basins(g, was, f);
    let mut load: Vec<f64> = (0..g.n()).map(|k| if open[k] { 0.0 } else { (was[k] + up[k] * dt - h[k]).max(0.0) * g.area(k) }).collect();
    let mut into = vec![0.0; basins.len()];
    for &k in f.order.iter().rev() {
        let k = k as usize;
        if of[k] != NONE {
            let b = of[k] as usize;
            into[b] += load[k];
            if basins[b][0] as usize != k {
                continue;
            }
            load[k] = fill(g, h, &basins[b], into[b], f.to[k]);
        }
        let (ways, sum) = ways_down(&f.surface, k, |k| f.earlier(g, k));
        for &(q, w) in ways.iter().filter(|w| !open[w.0]) {
            load[q] += load[k] * w / sum;
        }
    }
}

/// Fill the floor of the basin of samples `members` of heights `h` with `volume` m³ from its lowest up, as one level
/// over them (`flow::level_of`), no higher than its pass (sample `pass`; `NONE`: no pass), and return what it cannot
/// hold, m³.
fn fill(g: &Grid, h: &mut [f64], members: &[u32], volume: f64, pass: u32) -> f64 {
    let top = if pass == NONE { f64::INFINITY } else { h[pass as usize] };
    let room: f64 = members.iter().map(|&k| g.area(k as usize) * (top - h[k as usize]).max(0.0)).sum();
    let level = if volume >= room {
        top
    } else {
        let mut floors: Vec<(f64, f64)> = members.iter().map(|&k| (h[k as usize], g.area(k as usize))).collect();
        crate::flow::level_of(&mut floors, volume)
    };
    members.iter().for_each(|&k| h[k as usize] = h[k as usize].max(level));
    (volume - room).max(0.0)
}
