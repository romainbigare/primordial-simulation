//! One lobe of the high stand (laws.md 0, `space::delta`): a mouth fills the liquid it runs into with its own load,
//! outward from where its channel comes in, in the order of least cost (Dijkstra): deeper liquid costs more to fill, so
//! a lobe follows a drowned valley and the shallows; where the waves are strong a step away from the coast costs more,
//! so they spread it along the shore; where they are weak a step on along the channel costs less, so the channel carries
//! it out (Wright and Coleman 1973; Galloway 1975). Each sample fills up to the lobe's top (`delta::top`), and a front
//! falls from the lobe's edge at the load's angle of rest (`shelf::FORESET`).

use super::delta::{Liquid, Mouth};
use super::grid::Grid;
use super::shelf::FORESET;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// The depth of liquid under a lobe's top over which filling costs it one step more, m.
const DEPTH_COST_M: f64 = 10.0;
/// How much more a step away from the coast costs where the waves are those of an open coast (β).
const ALONG_COAST: f64 = 3.0;
/// How much less a step on along the channel costs where no waves come (γ, under 1).
const DOWN_CHANNEL: f64 = 0.5;
/// Rounds that move the lobe's last-filled samples into its front until the two hold its load.
const FRONT_ROUNDS: usize = 3;

/// A sample waiting in a search: its key (a cost, a distance or a height) and the sample; the least first, then the
/// least index.
pub(super) struct Step(pub(super) f64, pub(super) usize);

impl Ord for Step {
    fn cmp(&self, o: &Self) -> Ordering {
        o.0.total_cmp(&self.0).then(o.1.cmp(&self.1))
    }
}
impl PartialOrd for Step {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl PartialEq for Step {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == Ordering::Equal
    }
}
impl Eq for Step {}

/// Of each sample, the search that last reached it (a stamp, so a search costs only the samples it reaches) and what
/// it reached it at.
pub(super) struct Seen {
    stamp: Vec<u32>,
    at: Vec<f64>,
}

impl Seen {
    /// No sample of `n` reached.
    pub(super) fn new(n: usize) -> Seen {
        Seen { stamp: vec![u32::MAX; n], at: vec![0.0; n] }
    }
    /// What search `m` reached sample `k` at (infinite where it did not).
    fn get(&self, m: u32, k: usize) -> f64 {
        if self.stamp[k] == m { self.at[k] } else { f64::INFINITY }
    }
    /// Search `m` reaches sample `k` at `at`.
    fn set(&mut self, m: u32, k: usize, at: f64) {
        (self.stamp[k], self.at[k]) = (m, at);
    }
}

/// A lobe: the samples of its top, in the order filled, and of its front, each with the m³ it got.
pub(super) struct Lobe {
    pub(super) top: Vec<(usize, f64)>,
    pub(super) front: Vec<(usize, f64)>,
}

/// The lobe of mouth `mouth` (the `m`th) on the heights `built`, m: from its entry, each sample of the open low ground
/// in the order of least cost (the module's doc) filled up to its top until the load is laid, the last in part; then
/// its front at `FORESET` down from its edge (`front`), the load it takes moved from the samples filled last, so the
/// lobe holds the load exactly (none where it finds no more room: what it laid is the lobe's). `seen` holds the
/// searches' marks.
pub(super) fn fill(g: &Grid, built: &mut [f64], (liquid, mouth, m): (&Liquid, &Mouth, u32), (seen, near): (&mut Seen, &mut Seen)) -> Lobe {
    let (top, own) = (mouth.top, liquid.body[mouth.entry]);
    let mut heap = BinaryHeap::from([Step(0.0, mouth.entry)]);
    seen.set(m, mouth.entry, 0.0);
    let (mut left, mut filled) = (mouth.load, Vec::new());
    while let Some(Step(c, k)) = heap.pop() {
        if left <= 0.0 {
            break;
        }
        if c > seen.get(m, k) {
            continue;
        }
        let put = ((top - built[k]).max(0.0) * g.area(k)).min(left);
        if put > 0.0 {
            built[k] += put / g.area(k);
            left -= put;
            filled.push((k, put));
        }
        for (j, d) in g.around(k).filter(|&(j, _)| liquid.body[j] == own) {
            let cost = c + d * step(g, (k, j), (liquid, mouth), (top - built[j]).max(0.0));
            if cost < seen.get(m, j) {
                seen.set(m, j, cost);
                heap.push(Step(cost, j));
            }
        }
    }
    let front = front(g, built, (liquid, top, m, own), &mut filled, near);
    Lobe { top: filled, front }
}

/// The samples of a channel a mouth's heading is read along, upstream of its mouth.
const HEADING_SAMPLES: usize = 4;

/// The way the channel at sample `k` comes into sample `entry` of grid `g`, a unit vector east and south: from the
/// sample `HEADING_SAMPLES` up its main channel (`up` of each sample, `NONE` for none) to the entry.
pub(super) fn heading(g: &Grid, up: impl Fn(usize) -> u32, (k, entry): (usize, usize)) -> [f64; 2] {
    let mut from = k;
    for _ in 0..HEADING_SAMPLES {
        if up(from) == super::network::NONE {
            break;
        }
        from = up(from) as usize;
    }
    let v = offset(g, from, entry);
    let len = v[0].hypot(v[1]);
    if len > 0.0 { [v[0] / len, v[1] / len] } else { [0.0; 2] }
}

/// The east and south of a step of grid `g` from sample `k` to sample `j`, m: across a globe's seam the short way.
pub(super) fn offset(g: &Grid, k: usize, j: usize) -> [f64; 2] {
    let (rk, ck, rj, cj) = ((k / g.across) as i64, (k % g.across) as i64, (j / g.across) as i64, (j % g.across) as i64);
    let mut dc = cj - ck;
    if g.wrap && 2 * dc.abs() > g.across as i64 {
        dc -= dc.signum() * g.across as i64;
    }
    [dc as f64 * g.dx[j / g.across], (rj - rk) as f64 * g.dy]
}

/// The cost of a step from sample `k` to sample `j` of grid `g` per m of it, for the lobe of `mouth` over `liquid`, where
/// `j` lies `depth` m under its top: one, and more by the depth over `DEPTH_COST_M`; more by `ALONG_COAST` times how
/// open `j` lies to the waves times the square of how far the step leads away from the coast; less by `DOWN_CHANNEL`
/// times how sheltered `j` lies times how far the step leads on along the channel.
fn step(g: &Grid, (k, j): (usize, usize), (liquid, mouth): (&Liquid, &Mouth), depth: f64) -> f64 {
    let s = offset(g, k, j);
    let len = s[0].hypot(s[1]);
    let s = [s[0] / len, s[1] / len];
    let (out, on) = (s[0] * liquid.seaward[j][0] + s[1] * liquid.seaward[j][1], s[0] * mouth.heading[0] + s[1] * mouth.heading[1]);
    let e = liquid.exposed[j];
    (1.0 + depth / DEPTH_COST_M) * (1.0 + ALONG_COAST * e * out * out) * (1.0 - DOWN_CHANNEL * (1.0 - e) * on.max(0.0))
}

/// The front of the lobe `m` whose samples got `filled` (each with the m³ it got, in the order filled) on the heights
/// `built`, its top at `top`, m: round its edge the open liquid of its body `own` falls from the heights the lobe's
/// samples reach at `FORESET` (`slope`); what it takes is moved from the samples filled last, the entry's too (a lobe
/// of one sample has a front), a few rounds; it is then thinned or thickened to hold exactly what they gave,
/// never over the top (what it cannot hold goes back to the sample filled last). Lays it on `built` and returns its
/// samples, each with the m³ it got.
fn front(g: &Grid, built: &mut [f64], (liquid, top, m, own): (&Liquid, f64, u32, u32), filled: &mut Vec<(usize, f64)>, near: &mut Seen) -> Vec<(usize, f64)> {
    let lobe: f64 = filled.iter().map(|f| f.1).sum();
    let mut ring = Vec::new();
    for round in 0..FRONT_ROUNDS as u32 {
        ring = slope(g, built, (liquid, m * FRONT_ROUNDS as u32 + round, own), filled, near);
        let mut want: f64 = ring.iter().map(|r: &(usize, f64)| r.1 * g.area(r.0)).sum::<f64>() - (lobe - filled.iter().map(|f| f.1).sum::<f64>());
        while want > 0.0 && !filled.is_empty() {
            let last = filled.len() == 1;
            let (k, put) = filled.last_mut().expect("a filled sample");
            let back = put.min(want);
            (*put, want) = (*put - back, want - back);
            built[*k] -= back / g.area(*k);
            // (The entry stays the lobe's last sample, even emptied: what the front cannot hold goes back to it.)
            if last {
                break;
            }
            if *put <= 0.0 {
                filled.pop();
            }
        }
    }
    let wanted: f64 = ring.iter().map(|r| r.1 * g.area(r.0)).sum();
    let scale = if wanted > 0.0 { (lobe - filled.iter().map(|f| f.1).sum::<f64>()) / wanted } else { 0.0 };
    let laid: Vec<(usize, f64)> = ring.into_iter().map(|(k, t)| (k, (t * scale).min((top - built[k]).max(0.0)) * g.area(k))).filter(|r| r.1 > 0.0).collect();
    laid.iter().for_each(|&(k, put)| built[k] += put / g.area(k));
    let left = lobe - filled.iter().map(|f| f.1).sum::<f64>() - laid.iter().map(|r| r.1).sum::<f64>();
    if let Some((k, put)) = filled.last_mut() {
        built[*k] += left / g.area(*k);
        *put += left;
    }
    laid
}

/// The samples of the liquid of the body `own` round a lobe whose samples are `filled` on the heights `built` that its
/// front raises, each with how much, m: from the lobe out, each to the height of the lobe's sample it is reached from
/// less `FORESET` times how far it lies, the highest such height (search `id` of `near`, which holds the heights,
/// negated), down to where that meets the bed.
fn slope(g: &Grid, built: &[f64], (liquid, id, own): (&Liquid, u32, u32), filled: &[(usize, f64)], near: &mut Seen) -> Vec<(usize, f64)> {
    filled.iter().for_each(|&(k, _)| near.set(id, k, -built[k]));
    let mut heap: BinaryHeap<Step> = filled.iter().map(|&(k, _)| Step(-built[k], k)).collect();
    let mut lobe: Vec<usize> = filled.iter().map(|f| f.0).collect();
    lobe.sort_unstable();
    let mut ring = Vec::new();
    while let Some(Step(c, k)) = heap.pop() {
        if c > near.get(id, k) {
            continue;
        }
        let at = -c;
        for (j, len) in g.around(k) {
            let hj = at - FORESET * len;
            if liquid.body[j] == own && hj > built[j] && -hj < near.get(id, j) {
                near.set(id, j, -hj);
                heap.push(Step(-hj, j));
            }
        }
        if lobe.binary_search(&k).is_err() {
            ring.push((k, (at - built[k]).max(0.0)));
        }
    }
    ring.sort_by_key(|r| r.0);
    ring.dedup_by_key(|r| r.0);
    ring
}
