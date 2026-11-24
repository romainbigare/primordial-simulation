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

/// Of each pond of a sample's `pond` (`ponds` many), its key: the first of its samples, which no other pond holds.
pub(super) fn keys(pond: &[u32], ponds: usize) -> Vec<u32> {
    let mut key = vec![NONE; ponds];
    for (k, &p) in pond.iter().enumerate().filter(|(_, &p)| p != NONE) {
        if key[p as usize] == NONE {
            key[p as usize] = k as u32;
        }
    }
    key
}

/// The ponds of a sample's `pond` and their list `ponds` numbered as near the ponds whose keys were `was` (`keys`) as
/// they can be: a pond of a key in `was` keeps its place in it; a new pond takes the place of a pond gone, in order, or
/// one after the last; a place left empty takes the last pond. So an edit renumbers no pond it did not reach, and a
/// list numbered after its own keys stays as it is.
pub(super) fn renumber(pond: &mut [u32], ponds: &mut Vec<Pond>, was: &[u32]) {
    let key = keys(pond, ponds.len());
    let place: std::collections::HashMap<u32, u32> = was.iter().enumerate().map(|(i, &k)| (k, i as u32)).collect();
    let mut to: Vec<u32> = key.iter().map(|k| place.get(k).copied().unwrap_or(NONE)).collect();
    let mut taken = vec![false; was.len()];
    to.iter().filter(|&&t| t != NONE).for_each(|&t| taken[t as usize] = true);
    let mut gone = (0..was.len() as u32).filter(|&i| !taken[i as usize]);
    let mut next = was.len() as u32;
    for t in to.iter_mut().filter(|t| **t == NONE) {
        *t = gone.next().unwrap_or_else(|| {
            next += 1;
            next - 1
        });
    }
    // The places left empty under the count of ponds, lowest first, each taken by the pond of the highest place over it.
    let n = ponds.len() as u32;
    let empty = gone.filter(|&i| i < n);
    let mut over: Vec<usize> = (0..to.len()).filter(|&p| to[p] >= n).collect();
    over.sort_by_key(|&p| std::cmp::Reverse(to[p]));
    over.into_iter().zip(empty).for_each(|(p, e)| to[p] = e);
    let mut list = vec![Pond { pass: NONE, level: f64::NAN }; ponds.len()];
    ponds.iter().zip(&to).for_each(|(p, &t)| list[t as usize] = *p);
    pond.iter_mut().filter(|p| **p != NONE).for_each(|p| *p = to[*p as usize]);
    *ponds = list;
}

/// The closed basins of the flood `f` over heights `z` (`Flood::under`): of each sample the basin it lies in
/// (`NONE` where none), and of each basin its samples, the first the one the flood reached first: samples under a
/// level beside each other across a side are one basin, as they stand at one level (the flood joins them so).
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
            for (j, _) in g.sides(i as usize) {
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
        if shallow(h, f, members) {
            let level = f.level[members[0] as usize];
            members.iter().for_each(|&k| h[k as usize] = level);
        }
    }
}

/// Is the closed basin of samples `members` of the flood `f` over heights `h` too shallow to hold a pond: its deepest
/// sample less than `GRADED_M` under its level (the floods over a plain flow through it)?
pub(super) fn shallow(h: &[f64], f: &Flood, members: &[u32]) -> bool {
    let level = f.level[members[0] as usize];
    members.iter().all(|&k| level - h[k as usize] < GRADED_M)
}

/// Where the load of a step of `dt` years settles (each sample's rise at `up` less how far it rose from heights `was`,
/// m³; none on the open low ground): in the flood's order from the last reached, each sample's load spreads down as
/// the liquid does (`math::ways_down`, on the flood's surface), so it reaches every hollow below what was cut; a closed
/// basin keeps all that comes into it, and once all is in, at its first sample, fills its floor from the lowest up
/// toward the height its pass has now (`fill`), and what it cannot hold runs on over the pass. What reaches the open
/// low ground is added to `offshore` there, m³ (`space::shelf`).
pub(super) fn settle(g: &Grid, h: &mut [f64], f: &Flood, was: &[f64], (up, open): (&[f64], &[bool]), (dt, offshore): (f64, &mut [f64])) {
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
        for &(q, w) in &ways {
            *if open[q] { &mut offshore[q] } else { &mut load[q] } += load[k] * w / sum;
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
