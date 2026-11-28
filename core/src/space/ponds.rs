//! The closed basins of the terrain's samples (laws.md 0 and 5): the samples under a basin's level (`space::drain`),
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

/// The ponds of a sample's `pond` and their list `ponds` numbered after the ponds whose keys were `was` (`keys`), so
/// that a number never comes to name a pond elsewhere while a picture made before still reads it: a pond takes the place
/// of the pond of `was` whose key is its own, else the first place of `was` whose key it holds (a pond grown over
/// another), else the place after the last; a place no pond takes stays empty (no pass, no level). So an edit
/// renumbers no pond it did not reach, a pond that grew over another keeps a number of one of them, and a list
/// numbered after its own keys stays as it is.
pub(super) fn renumber(pond: &mut [u32], ponds: &mut Vec<Pond>, was: &[u32]) {
    let key = keys(pond, ponds.len());
    let mut to = vec![NONE; ponds.len()];
    let mut taken = vec![false; was.len()];
    // (First each pond whose own key a place holds, then each pond over the keys of the places left.)
    for own in [true, false] {
        for (i, &k) in was.iter().enumerate() {
            let p = if k == NONE || taken[i] { NONE } else { pond.get(k as usize).copied().unwrap_or(NONE) };
            if p != NONE && to[p as usize] == NONE && (!own || key[p as usize] == k) {
                (to[p as usize], taken[i]) = (i as u32, true);
            }
        }
    }
    let mut next = was.len() as u32;
    for t in to.iter_mut().filter(|t| **t == NONE) {
        (*t, next) = (next, next + 1);
    }
    let mut list = vec![Pond { pass: NONE, level: f64::NAN }; next as usize];
    ponds.iter().zip(&to).for_each(|(p, &t)| list[t as usize] = *p);
    pond.iter_mut().filter(|p| **p != NONE).for_each(|p| *p = to[*p as usize]);
    *ponds = list;
}

/// The share of the places of the ponds' list that may be empty: past it, a build of the years drops them (`compact`).
const EMPTY_SHARE: f64 = 0.25;

/// The keys `was` of the places of the ponds `ponds` (`Relief::ponds`) with the empty places dropped (no pond holds
/// them: no level, `renumber`) where they are more than `EMPTY_SHARE` of the list, else as they are: so the list
/// does not only grow, and the ponds after an empty place dropped take lower numbers at the next build.
pub(super) fn compact(was: &[u32], ponds: &[Pond]) -> Vec<u32> {
    let empty = |p: usize| ponds.get(p).is_some_and(|q| q.level.is_nan());
    let n = (0..was.len()).filter(|&p| empty(p)).count();
    if n as f64 <= EMPTY_SHARE * was.len() as f64 {
        return was.to_vec();
    }
    was.iter().enumerate().filter(|&(p, _)| !empty(p)).map(|(_, &k)| k).collect()
}

/// The key of each place of the ponds `pond` (`ponds` many) numbered after the keys `was` (`renumber`): its pond's
/// (`keys`), and for an empty place the key it had, so a pond that holds that sample later takes its place again,
/// and a picture that still reads the place finds the pond that holds it now.
pub(super) fn kept_keys(pond: &[u32], ponds: usize, was: &[u32]) -> Vec<u32> {
    keys(pond, ponds).iter().enumerate().map(|(p, &k)| if k == NONE { was.get(p).copied().unwrap_or(NONE) } else { k }).collect()
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

#[cfg(test)]
mod tests {
    use super::{compact, Pond, NONE};

    /// The list of ponds does not only grow: its empty places are dropped once they are more than `EMPTY_SHARE` of it,
    /// and kept, with their keys, while they are fewer.
    #[test]
    fn empty_places_are_dropped_once_too_many() {
        let (full, empty) = (Pond { pass: 3, level: 1.0 }, Pond { pass: NONE, level: f64::NAN });
        let keys = [10, 20, 30, 40];
        assert_eq!(compact(&keys, &[full, empty, full, full]), keys);
        assert_eq!(compact(&keys, &[full, empty, empty, full]), [10, 40]);
    }
}
