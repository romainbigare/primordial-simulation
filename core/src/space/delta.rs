//! The lobes the high stand's mouths build (laws.md 0, `space::stand`): each mouth fills the liquid it runs into with
//! its own load, outward from where its channel comes in, along the way that costs it least: deeper liquid costs more
//! to fill, so a lobe follows a drowned valley and the shallows; where the waves are strong a step away from the coast
//! costs more, so they spread it along the shore; where they are weak a step on along the channel costs less, so the
//! channel carries it out (Wright and Coleman 1973; Galloway 1975). Each sample fills up to the top a deposit builds to
//! (`top`: the channel's flood; the waves shape where the lobe goes, not how high: on a coast they build beaches up to
//! the level and over it), a front falls from the lobe's edge at the load's angle of rest (`shelf::FORESET`), and the
//! edge is rounded as the shelf's is (`shelf::round`).

use super::grid::Grid;
use super::shelf::{round, FORESET};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// The flood that sorts what settles and that a delta's top builds up to: its runoff from the area that drains
/// through a sample, m/s.
pub(crate) const FLOOD_RUNOFF: f64 = 1e-6;
/// The depth the waves of an open coast stir the bed to, m: filling liquid this much deeper costs a lobe one step more.
const WAVE_BASE_M: f64 = 10.0;
/// The run of open liquid at which the waves are those of an open coast, m; a longer run counts as this.
const FETCH_REF_M: f64 = 50e3;
/// How much more a step away from the coast costs where the waves are those of an open coast (β).
const ALONG_COAST: f64 = 3.0;
/// How much less a step on along the channel costs where no waves come (γ, under 1).
const DOWN_CHANNEL: f64 = 0.5;
/// Rounds that move the lobe's last-filled samples into its front until the two hold its load.
const FRONT_ROUNDS: usize = 3;

/// A mouth: the sample of the open low ground its channel comes into, the load it brings, m³, the area that drains to
/// that sample, m², and the way its channel comes in, a unit vector east and south.
pub(super) struct Mouth {
    pub(super) entry: usize,
    pub(super) load: f64,
    pub(super) drains: f64,
    pub(super) heading: [f64; 2],
}

/// How open to the waves each sample of grid `g` lies on the open low ground `open`, from none to 1: the run of open
/// liquid the waves come over (`Grid::fetches`) over `FETCH_REF_M`.
pub(super) fn exposures(g: &Grid, open: &[bool]) -> Vec<f64> {
    g.fetches(open, FETCH_REF_M).iter().map(|f| f / FETCH_REF_M).collect()
}

/// The height a deposit builds up to in liquid standing at `level`, m, where a channel that drains `drains` m² comes in:
/// the surface of the channel's flood there, as deep as a channel of the law `channel` (the depth at 1 m³/s, m, and its
/// power of the discharge; Leopold and Maddock 1953) carries `FLOOD_RUNOFF` from that area.
pub(super) fn top(level: f64, (drains, channel): (f64, [f64; 2])) -> f64 {
    level + channel[0] * (FLOOD_RUNOFF * drains).powf(channel[1])
}

/// A sample waiting in a lobe's fill: the cost to reach it and the sample; the cheapest first, then the least index.
struct Step(f64, usize);

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

/// What the lobes read of the open low ground: which samples are open, how open to the waves each lies, the way out
/// from the coast at each (a unit vector east and south along the rise of the distance from the land; none where it
/// has none), the level of its liquid, m, and the channel law of the terrain.
pub(super) struct Liquid<'a> {
    pub(super) open: &'a [bool],
    pub(super) exposed: &'a [f64],
    seaward: Vec<[f64; 2]>,
    pub(super) level: f64,
    pub(super) channel: [f64; 2],
}

impl<'a> Liquid<'a> {
    /// The open low ground `open` of grid `g`, its samples as open to the waves as `exposed`, its liquid at `level`, m,
    /// under the channel law `channel`.
    pub(super) fn new(g: &Grid, (open, exposed): (&'a [bool], &'a [f64]), level: f64, channel: [f64; 2]) -> Liquid<'a> {
        let edge = g.edge_distance(open);
        let seaward = (0..g.n()).map(|k| {
            let [(e, dx), (w, _), (north, dy), (s, _)] = g.sides(k);
            let v = [(edge[e] - edge[w]) / (2.0 * dx), (edge[s] - edge[north]) / (2.0 * dy)];
            let len = v[0].hypot(v[1]);
            if open[k] && len.is_finite() && len > 0.0 { [v[0] / len, v[1] / len] } else { [0.0; 2] }
        }).collect();
        Liquid { open, exposed, seaward, level, channel }
    }
}

/// The lobes of the `mouths` (largest load first, then by entry) on grid `g` of heights `h`, laid over the open low
/// ground `liquid` for `years`: each fills its own load (`fill`), on the heights the lobes before it left; then the
/// hollows they leave, in the liquid and in the valleys behind them (`behind`: ground that drains to a mouth and lies
/// under the highest top), are filled (`hollows`) from the lobes' fronts, each by its share of the load, thinned (and
/// where a front holds too little, from the samples its lobe filled last, `unfill`); their edges rounded
/// (`shelf::round`), they are laid on the heights and the loose ground `loose`, m. Returns the volume laid, m³, and of
/// each sample what was laid, m, and how long ago its top was laid, years.
pub(super) fn lobes(g: &Grid, (h, loose): (&mut [f64], &mut [f64]), (liquid, behind): (&Liquid, &[bool]), mouths: &[Mouth], years: f64) -> (f64, Vec<f64>, Vec<f64>) {
    let n = g.n();
    let mut built = h.to_vec();
    let (mut reach, mut age) = (vec![f64::NEG_INFINITY; n], vec![0.0; n]);
    let mut seen = Seen { stamp: vec![u32::MAX; n], cost: vec![0.0; n] };
    let mut filled = Vec::with_capacity(mouths.len());
    for (m, mouth) in mouths.iter().enumerate() {
        filled.push(fill(g, (&mut built, &mut reach, &mut age), (liquid, mouth, m as u32), &mut seen, years));
    }
    let volume = |built: &[f64]| (0..n).map(|k| (built[k] - h[k]).max(0.0) * g.area(k)).sum::<f64>();
    let target = volume(&built);
    hollows(g, (&mut built, h), (&mut reach, &mut age), (liquid, behind), years);
    let more = volume(&built) - target;
    let all: f64 = mouths.iter().map(|m| m.load).sum();
    for ((lobe, ring), mouth) in filled.into_iter().zip(mouths) {
        unfill(g, &mut built, (lobe, ring), more * mouth.load / all);
    }
    let thick: Vec<f64> = (0..n).map(|k| (built[k] - h[k]).max(0.0)).collect();
    let room: Vec<f64> = (0..n).map(|k| (reach[k] - h[k]).max(0.0)).collect();
    let ground: Vec<bool> = (0..n).map(|k| liquid.open[k] || behind[k]).collect();
    // (Rounded over the ground the lobes may lie on alone, so the land does not thin their edges.)
    let over: Vec<f64> = ground.iter().map(|&o| if o { 1.0 } else { 0.0 }).collect();
    let was = h.to_vec();
    let laid = round(g, (h, loose), (&ground, &over), (&thick, &room, target));
    let laid_on: Vec<f64> = (0..n).map(|k| (h[k] - was[k]).max(0.0)).collect();
    (laid, laid_on, age)
}

/// The hollows the lobes leave on the heights `built` over the heights before them `before`, filled to where liquid
/// would spill out of them, as the fine load fills a lagoon or a dammed valley over the stand: a flood from the open low
/// ground of `liquid` (its largest part still under the level) over what the lobes raised, the liquid they cut off and the
/// ground behind them (`behind`), each sample up to
/// the lowest height its liquid could leave it by (Barnes, Lehman and Mulla 2014); one the flood does not reach (shut in
/// by higher ground), up to the lowest ground round it. What it takes the lobes give (`shelf::round` thins them to hold
/// their load); its samples are rounded up to that height (`reach`), and what lies in a lagoon was laid half the stand
/// ago (`age`, of `years`).
fn hollows(g: &Grid, (built, before): (&mut [f64], &[f64]), (reach, age): (&mut [f64], &mut [f64]), (liquid, behind): (&Liquid, &[bool]), years: f64) {
    let n = g.n();
    let wet: Vec<bool> = (0..n).map(|k| liquid.open[k] && built[k] < liquid.level).collect();
    let main = largest(g, &wet);
    let laid: Vec<bool> = (0..n).map(|k| built[k] > before[k] || (wet[k] && !main[k]) || behind[k]).collect();
    let mut done: Vec<bool> = (0..n).map(|k| !laid[k]).collect();
    let mut heap: BinaryHeap<Step> = (0..n).filter(|&k| !done[k] && g.around(k).any(|(j, _)| main[j])).map(|k| Step(built[k].max(liquid.level), k)).collect();
    while let Some(Step(at, k)) = heap.pop() {
        if done[k] {
            continue;
        }
        done[k] = true;
        lift(k, at, (built, reach, age), (wet[k] || behind[k], years));
        for (j, _) in g.around(k) {
            if !done[j] {
                heap.push(Step(built[j].max(at), j));
            }
        }
    }
    for k in (0..n).filter(|&k| !done[k]) {
        let rim = g.around(k).filter(|&(j, _)| !laid[j]).map(|(j, _)| built[j]).fold(f64::INFINITY, f64::min);
        if rim.is_finite() {
            lift(k, rim.max(liquid.level), (built, reach, age), (wet[k] || behind[k], years));
        }
    }
}

/// Sample `k` raised to `at`, m, if it stands lower: its height in `built`, the height it may be rounded to in `reach`,
/// and, where it lay under liquid or behind a lobe (`wet`), its age, half the `years`.
fn lift(k: usize, at: f64, (built, reach, age): (&mut [f64], &mut [f64], &mut [f64]), (wet, years): (bool, f64)) {
    if at > built[k] {
        (built[k], reach[k]) = (at, reach[k].max(at));
        if wet {
            age[k] = 0.5 * years;
        }
    }
}

/// The samples of the largest part of `wet` joined through their sides and corners (by the least index where two tie).
fn largest(g: &Grid, wet: &[bool]) -> Vec<bool> {
    let n = g.n();
    let mut part = vec![u32::MAX; n];
    let mut sizes: Vec<usize> = Vec::new();
    for k in (0..n).filter(|&k| wet[k]) {
        if part[k] != u32::MAX {
            continue;
        }
        let id = sizes.len() as u32;
        let (mut todo, mut size) = (vec![k], 0);
        part[k] = id;
        while let Some(i) = todo.pop() {
            size += 1;
            for (j, _) in g.around(i) {
                if wet[j] && part[j] == u32::MAX {
                    part[j] = id;
                    todo.push(j);
                }
            }
        }
        sizes.push(size);
    }
    let main = (0..sizes.len()).max_by_key(|&p| (sizes[p], std::cmp::Reverse(p))).map_or(u32::MAX, |p| p as u32);
    (0..n).map(|k| part[k] == main).collect()
}

/// What `volume` m³ a lobe gives back from the heights `built`: its front (`ring`, each sample with what it got) thinned
/// evenly, and what that cannot give from the samples of its top `filled` (each with what it got, in the order
/// filled), the last filled first.
fn unfill(g: &Grid, built: &mut [f64], (mut filled, ring): Lobe, mut volume: f64) {
    let front: f64 = ring.iter().map(|r| r.1).sum();
    if front > 0.0 {
        let share = (volume / front).min(1.0);
        ring.iter().for_each(|&(k, put)| built[k] -= put * share / g.area(k));
        volume -= front * share;
    }
    while volume > 0.0 {
        let Some((k, put)) = filled.pop() else { break };
        let back = put.min(volume);
        built[k] -= back / g.area(k);
        volume -= back;
    }
}

/// A lobe: the samples of its top, in the order filled, and of its front, each with what it got, m³.
type Lobe = (Vec<(usize, f64)>, Vec<(usize, f64)>);

/// Of each sample, the mouth that last reached it (a stamp) and the cost it reached it at.
struct Seen {
    stamp: Vec<u32>,
    cost: Vec<f64>,
}

/// The lobe of mouth `mouth` (the `m`th) on the heights `built`, m: from its entry, each sample in the order of least
/// cost (the module's doc) filled up to its top (`top`) until the load is laid, the last in part; then its front at
/// `FORESET` down from its edge (`front`), the load it takes moved from the samples filled last, so the lobe holds the
/// load exactly. Raises `reach`, the height each sample may be rounded up to, m (the top of the lobe's sample it lies in
/// or nearest), and sets the age of what each sample got: the share of the load laid after it, times `years` (the
/// front's, none). Returns the samples of its top that it filled, in the order filled, and of its front, each with what
/// it got, m³.
fn fill(g: &Grid, (built, reach, age): (&mut [f64], &mut [f64], &mut [f64]), (liquid, mouth, m): (&Liquid, &Mouth, u32), seen: &mut Seen, years: f64) -> Lobe {
    let top = top(liquid.level, (mouth.drains, liquid.channel));
    let mut heap = BinaryHeap::from([Step(0.0, mouth.entry)]);
    (seen.stamp[mouth.entry], seen.cost[mouth.entry]) = (m, 0.0);
    let (mut left, mut filled) = (mouth.load, Vec::new());
    while let Some(Step(c, k)) = heap.pop() {
        if c > seen.cost[k] || left <= 0.0 {
            if left <= 0.0 {
                break;
            }
            continue;
        }
        let put = ((top - built[k]).max(0.0) * g.area(k)).min(left);
        if put > 0.0 {
            built[k] += put / g.area(k);
            left -= put;
            filled.push((k, put));
        }
        for (j, d) in g.around(k) {
            if !liquid.open[j] {
                continue;
            }
            let cost = c + d * step(g, (k, j), (liquid, mouth), (top - built[j]).max(0.0));
            if seen.stamp[j] != m || cost < seen.cost[j] {
                (seen.stamp[j], seen.cost[j]) = (m, cost);
                heap.push(Step(cost, j));
            }
        }
    }
    let ring = front(g, built, (liquid, top), &mut filled);
    let mut laid = 0.0;
    for &(k, put) in &filled {
        laid += put;
        age[k] = years * (1.0 - laid / mouth.load).max(0.0);
        reach[k] = reach[k].max(top);
    }
    for &(k, put) in &ring {
        age[k] = 0.0;
        reach[k] = reach[k].max(top);
        built[k] += put / g.area(k);
    }
    (filled, ring)
}

/// The cost of a step from sample `k` to sample `j` of grid `g` per m of it, for the lobe of `mouth` over `liquid`, where
/// `j` lies `depth` m under its top: one, and more by the depth over `WAVE_BASE_M`; more by `ALONG_COAST` times how
/// open `j` lies to the waves times the square of how far the step leads away from the coast; less by `DOWN_CHANNEL`
/// times how sheltered `j` lies times how far the step leads on along the channel.
fn step(g: &Grid, (k, j): (usize, usize), (liquid, mouth): (&Liquid, &Mouth), depth: f64) -> f64 {
    let (rk, ck, rj, cj) = ((k / g.across) as f64, (k % g.across) as f64, (j / g.across) as f64, (j % g.across) as f64);
    let s = [(cj - ck) * g.dx[j / g.across], (rj - rk) * g.dy];
    let len = s[0].hypot(s[1]);
    let s = [s[0] / len, s[1] / len];
    let (out, on) = (s[0] * liquid.seaward[j][0] + s[1] * liquid.seaward[j][1], s[0] * mouth.heading[0] + s[1] * mouth.heading[1]);
    let e = liquid.exposed[j];
    (1.0 + depth / WAVE_BASE_M) * (1.0 + ALONG_COAST * e * out * out) * (1.0 - DOWN_CHANNEL * (1.0 - e) * on.max(0.0))
}

/// The front of a lobe whose samples got `filled` (each with the m³ it got, in the order filled) on the heights `built`,
/// its top at `top`, m: round its edge the open liquid falls from the top at `FORESET` (`slope`); what it takes is moved
/// from the samples filled last, a few rounds, and it is then thinned or thickened to hold exactly what they gave.
/// Returns its samples, each with the m³ it gets.
fn front(g: &Grid, built: &mut [f64], (liquid, top): (&Liquid, f64), filled: &mut Vec<(usize, f64)>) -> Vec<(usize, f64)> {
    let lobe: f64 = filled.iter().map(|f| f.1).sum();
    let mut ring = Vec::new();
    for _ in 0..FRONT_ROUNDS {
        ring = slope(g, built, (liquid, top), filled);
        let mut want: f64 = ring.iter().map(|r: &(usize, f64)| r.1 * g.area(r.0)).sum::<f64>() - (lobe - filled.iter().map(|f| f.1).sum::<f64>());
        while want > 0.0 && filled.len() > 1 {
            let (k, put) = filled.last_mut().expect("a filled sample");
            let back = put.min(want);
            (*put, want) = (*put - back, want - back);
            built[*k] -= back / g.area(*k);
            if *put <= 0.0 {
                filled.pop();
            }
        }
    }
    let kept: f64 = filled.iter().map(|f| f.1).sum();
    let wanted: f64 = ring.iter().map(|r| r.1 * g.area(r.0)).sum();
    if wanted <= 0.0 {
        // (No front: what the rounds took goes back to the last sample.)
        if let Some((k, put)) = filled.last_mut() {
            built[*k] += (lobe - kept) / g.area(*k);
            *put += lobe - kept;
        }
        return Vec::new();
    }
    let scale = (lobe - kept) / wanted;
    ring.into_iter().map(|(k, t)| (k, t * scale * g.area(k))).collect()
}

/// The samples of the open low ground round the lobe `filled` on the heights `built` that its front raises, each with
/// how much, m: from the lobe out, by the shortest way through the open low ground, each to the lobe's top `top` less
/// `FORESET` times how far it lies, down to where that meets the bed.
fn slope(g: &Grid, built: &[f64], (liquid, top): (&Liquid, f64), filled: &[(usize, f64)]) -> Vec<(usize, f64)> {
    let mut near: std::collections::HashMap<usize, f64> = filled.iter().map(|&(k, _)| (k, 0.0)).collect();
    let mut heap: BinaryHeap<Step> = filled.iter().map(|&(k, _)| Step(0.0, k)).collect();
    let mut ring = Vec::new();
    while let Some(Step(d, k)) = heap.pop() {
        if d > near[&k] {
            continue;
        }
        for (j, len) in g.around(k) {
            let dj = d + len;
            if liquid.open[j] && top - FORESET * dj > built[j] && near.get(&j).is_none_or(|&was| dj < was) {
                near.insert(j, dj);
                heap.push(Step(dj, j));
            }
        }
        if d > 0.0 {
            ring.push((k, (top - FORESET * d - built[k]).max(0.0)));
        }
    }
    ring.sort_by_key(|r| r.0);
    ring.dedup_by_key(|r| r.0);
    ring
}
