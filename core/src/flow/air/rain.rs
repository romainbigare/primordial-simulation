//! Rain (laws.md 1.5): where each air node's rain falls among the cells that see it (`wet`), a block's vapour through
//! a day with what its ground gives and what its nodes rain (`Pool`), and where what rained lands (`landed`).

use super::super::Scratch;
use super::daily::Mean;
use super::step::Active;
use super::{at_height, over_cell, touching};
use crate::{math::{mean_decay, mean_passed}, World, AIR};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// Where each air node's rain falls, as of the step's start (`wet`): per node its entries (`Seen`), each a place: a
/// cell alone (its index) or a patch of the low basin (`n` + its index in `patches`, `Place`); and per cell its place.
pub(super) struct Wet {
    pub(super) nodes: Vec<Seen>,
    pub(super) patches: Vec<Place>,
    place_of: Vec<u32>,
}

/// A patch of the low basin as its rain finds it, one surface (`surf::patches`): its cells with the share of its area
/// each holds, the air nodes its cells see with their weights by area (`Corners::by_area`), and how much colder than that
/// air the air touching it is, a mean by area, K (`Scratch::cool`).
pub(super) struct Place {
    cells: Vec<u32>,
    area: Vec<f64>,
    sees: Vec<(u32, f64)>,
    cool: f64,
}

impl Place {
    /// The patch of the cells `cells`.
    fn of(w: &World, s: &Scratch, cells: &[u32]) -> Place {
        let area = w.mesh.cells().area();
        let (all, sees) = (cells.iter().map(|&i| area[i as usize]).sum::<f64>(), s.corners.by_area(&s.wt, area, cells));
        let cool = cells.iter().map(|&i| s.cool[i as usize] * area[i as usize] / all).sum();
        Place { cells: cells.to_vec(), area: cells.iter().map(|&i| area[i as usize] / all).collect(), sees, cool }
    }
    /// The air touching it under air nodes at `t`, K (`touching`, by area).
    fn touching(&self, s: &Scratch, t: &[f64]) -> f64 {
        (s.corners.mean(&self.sees, t) - self.cool).max(1.0)
    }
}

/// One node's entries (`Wet`): the places that see it (`cell`), the share of the node's air over each, and per active gas the
/// node's saturation ratio (its amount over what saturates it at its mean ground) past which each rains (`[y * len +
/// e]`); and per active gas the entries in rising order of that ratio, each `[ratio, Σ share so far, Σ share × ratio so
/// far]` (`Seen::new`).
pub(super) struct Seen {
    pub(super) cell: Vec<u32>,
    pub(super) share: Vec<f64>,
    pub(super) ratio: Vec<f64>,
    steps: Vec<[f64; 3]>,
}

impl Seen {
    /// From its cells, their shares and per active gas their ratios.
    pub(super) fn new(cell: Vec<u32>, share: Vec<f64>, ratio: Vec<f64>) -> Seen {
        let mut steps = Vec::with_capacity(ratio.len());
        for by in ratio.chunks(cell.len().max(1)) {
            let mut order: Vec<(f64, f64)> = by.iter().copied().zip(share.iter().copied()).collect();
            order.sort_unstable_by(|p, q| p.0.total_cmp(&q.0));
            let mut sum = [0.0; 2];
            for (r, a) in order {
                sum = [sum[0] + a, sum[1] + a * r];
                steps.push([r, sum[0], sum[1]]);
            }
        }
        Seen { cell, share, ratio, steps }
    }
}

impl Wet {
    /// Node `j`'s entries for active gas `x` in rising order of their ratio (`Seen`): past ratio `r` it holds
    /// `r Σ share - Σ share × ratio` of the gas over what saturates the cells that see it, as a share of what saturates
    /// it at its mean ground (the per-cell rain of laws.md 1.5, summed).
    fn steps(&self, x: usize, j: usize) -> &[[f64; 3]] {
        let (node, len) = (&self.nodes[j], self.nodes[j].cell.len());
        &node.steps[x * len..(x + 1) * len]
    }
    /// The air touching each place a node that rains falls on (`rain`, per active gas and node, `[y * na + j]`, mol)
    /// under air nodes at `t`, K (`touching`; 0 elsewhere and for a cell of a patch), by place: `rain_air` reads no other.
    pub(super) fn touched(&self, s: &Scratch, t: &[f64], rain: &[f64]) -> Vec<f64> {
        let (n, na) = (self.place_of.len(), self.nodes.len());
        let mut wet = vec![false; n + self.patches.len()];
        for (i, _) in rain.iter().enumerate().filter(|r| *r.1 > 0.0) {
            self.nodes[i % na].cell.iter().for_each(|&p| wet[p as usize] = true);
        }
        let mut v = crate::par::map_n(n, |i| if wet[i] { touching(s, t, i) } else { 0.0 });
        v.extend(self.patches.iter().zip(&wet[n..]).map(|(p, &on)| if on { p.touching(s, t) } else { 0.0 }));
        v
    }
}

/// A ratio at which a node's cell starts or stops raining (`Pool::day`): the block's vapour there, mol, signed so that
/// the nearest in the way the vapour moves is the greatest, and the node's place among the block's raining nodes.
#[derive(PartialEq)]
struct Break(f64, usize);

impl Eq for Break {}

impl PartialOrd for Break {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Break {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0).then(other.1.cmp(&self.1))
    }
}

/// A block's vapour of one active gas over a day (`daily::exchange`): what its ground gives, `c + s X` mol over the step for `X`
/// mol in the block; and per node of the block its share `v` of a change of the block's vapour (by volume, as the eddies
/// even it), its vapour apart from that, `off`, mol, and what saturates it at its mean ground, `full`, mol (none where
/// the gas does not rain).
pub(super) struct Pool {
    pub(super) c: f64,
    pub(super) s: f64,
    pub(super) v: Vec<f64>,
    pub(super) off: Vec<f64>,
    pub(super) full: Vec<f64>,
}

impl Pool {
    /// The block's vapour over `dd` seconds of a step of `dt` from `x0` mol, its nodes `nodes` raining gas `y` by `wet`
    /// in `rain_time` `tau`: what its ground gives less what its nodes rain, each linear in the vapour between two of the
    /// ratios at which a cell starts or stops raining (`Wet::steps`), so followed exactly from one such ratio to the
    /// next, nearest first (`Break`); the vapour moves one way all day, toward the balance of the two. Its mean over the
    /// day, mol, returned; each node's rain, mol, and that times the ratio it falls at, added to `rain` (`[node of the
    /// block]`) and `ratio`.
    pub(super) fn day(&self, (wet, y, nodes): (&Wet, usize, &[u32]), x0: f64, (dd, dt, tau): (f64, f64, f64), (rain, ratio): (&mut [f64], &mut [f64])) -> f64 {
        let raining: Vec<usize> = (0..nodes.len()).filter(|&q| self.full[q] > 0.0).collect();
        let steps = |q: usize| wet.steps(y, nodes[q] as usize);
        let sums = |q: usize, k: usize| if k == 0 { [0.0; 2] } else { [steps(q)[k - 1][1], steps(q)[k - 1][2]] };
        // Each raining node's entries below its ratio now, a ratio it stands at counted below where the vapour rises; and
        // the rain then, (a x + b) / tau mol/s.
        let below = |up: bool| -> Vec<usize> {
            raining.iter().map(|&q| (q, (self.v[q] * x0 + self.off[q]) / self.full[q])).map(|(q, r)| steps(q).partition_point(|c| if up { c[0] <= r } else { c[0] < r })).collect()
        };
        let terms = |q: usize, k: usize| {
            let [s1, s2] = sums(q, k);
            [self.v[q] * s1, self.off[q] * s1 - self.full[q] * s2]
        };
        let rate = |x: f64, [a, b]: [f64; 2]| (self.c + self.s * x) / dt - (a * x + b) / tau;
        let (mut k, mut ab) = (below(true), [0.0; 2]);
        raining.iter().zip(&k).for_each(|(&q, &k)| terms(q, k).iter().zip(&mut ab).for_each(|(t, s)| *s += t));
        let up = rate(x0, ab) > 0.0;
        if !up {
            (k, ab) = (below(false), [0.0; 2]);
            raining.iter().zip(&k).for_each(|(&q, &k)| terms(q, k).iter().zip(&mut ab).for_each(|(t, s)| *s += t));
        }
        // The next ratio each node reaches, as the block's vapour there.
        let (sign, len) = (if up { -1.0 } else { 1.0 }, |q: usize| steps(q).len());
        let next = |n: usize, k: usize| {
            let q = raining[n];
            let at = if up { (k < len(q)).then(|| steps(q)[k][0]) } else { (k > 0).then(|| steps(q)[k - 1][0]) };
            at.map(|r| Break(sign * (r * self.full[q] - self.off[q]) / self.v[q], n))
        };
        let mut ahead: BinaryHeap<Break> = (0..raining.len()).filter_map(|n| next(n, k[n])).collect();
        // The time and the vapour's time integral so far, s and mol s, and each raining node's at its last change.
        let (mut x, mut time, mut sum) = (x0, 0.0, 0.0);
        let mut since = vec![[0.0; 2]; raining.len()];
        let mut flush = |n: usize, k: usize, (time, sum): (f64, f64), since: &mut [f64; 2]| {
            let (q, span, held) = (raining[n], time - since[0], sum - since[1]);
            if span > 0.0 {
                let [s1, s2] = sums(q, k);
                let m = (((self.v[q] * held + self.off[q] * span) * s1 - self.full[q] * s2 * span) / tau).max(0.0);
                rain[q] += m;
                ratio[q] += m * (self.v[q] * held / span + self.off[q]) / self.full[q];
            }
            *since = [time, sum];
        };
        while time < dd {
            let (f, left) = (rate(x, ab), dd - time);
            if (up && f <= 0.0) || (!up && f >= 0.0) {
                (time, sum) = (dd, sum + x * left);
                break;
            }
            if !up && x <= 0.0 {
                break;
            }
            // How the rate falls with the vapour, 1/s, and the time it takes to reach the nearest ratio (no lower than
            // none), s: x + f t mean_decay(beta t) reaches it where e^(-beta t) = 1 - beta (to - x) / f.
            let beta = -self.s / dt + ab[0] / tau;
            let to = ahead.peek().map_or(if up { f64::INFINITY } else { 0.0 }, |b| sign * b.0);
            let to = if up { to } else { to.max(0.0) };
            let (gap, z) = ((to - x) / f, beta * (to - x) / f);
            let reach = if !to.is_finite() || z >= 1.0 { f64::INFINITY } else if z == 0.0 { gap } else { -(-z).ln_1p() / z * gap };
            let piece = reach.min(left);
            (time, sum) = (time + piece, sum + (x + f * piece * mean_passed(beta * piece)) * piece);
            if piece < reach {
                x += f * piece * mean_decay(beta * piece);
                continue;
            }
            x = to;
            if let Some(n) = ahead.peek().filter(|b| sign * b.0 == to).map(|b| b.1) {
                ahead.pop();
                flush(n, k[n], (time, sum), &mut since[n]);
                let old = terms(raining[n], k[n]);
                k[n] = if up { k[n] + 1 } else { k[n] - 1 };
                let new = terms(raining[n], k[n]);
                (0..2).for_each(|c| ab[c] += new[c] - old[c]);
                ahead.extend(next(n, k[n]));
            }
        }
        (0..raining.len()).for_each(|n| flush(n, k[n], (time, sum), &mut since[n]));
        sum / dd
    }
}

/// Where each node's rain falls, as of the step's start (`Wet`): on each place that sees the node (`lerp`; a patch of
/// the low basin `patches` as one), by the share `a` of the node's air over it (its weight there times its area over
/// the area it sees), past the node's saturation ratio `e` at which the air touching it saturates (colder on higher
/// ground). The node rains what its places hold over their saturation, summed (laws.md 1.5).
pub(super) fn wet(w: &World, s: &Scratch, act: &[Active], t0: &[f64], patches: &[Vec<u32>]) -> Wet {
    let (cells, air) = (w.mesh.cells(), w.mesh.level(AIR));
    let (n, na, nx) = (cells.n(), air.n(), act.len());
    // Per node and active gas what saturates its air at its mean ground, per K (`[q * nx + y]`); per place what saturates
    // the air touching it, per K, times the area of the air it sees.
    let node: Vec<f64> = (0..na)
        .flat_map(|q| {
            let ts = at_height(t0[q], s.gam[q], s.za[q], s.zair[q]);
            act.iter().map(move |ac| (s.sat[ac.k].at(ts) / ts).max(f64::MIN_POSITIVE))
        })
        .collect();
    let mut place_of: Vec<u32> = (0..n as u32).collect();
    for (p, c) in patches.iter().enumerate() {
        c.iter().for_each(|&i| place_of[i as usize] = (n + p) as u32);
    }
    let places: Vec<Place> = patches.iter().map(|c| Place::of(w, s, c)).collect();
    let mut sat = vec![0.0; (n + places.len()) * nx];
    crate::par::rows(&mut sat[..n * nx], nx, || (), |_, i, row| {
        if place_of[i] as usize == i {
            let (tl, seen) = (touching(s, t0, i), s.seen[i]);
            act.iter().zip(row).for_each(|(ac, r)| *r = s.sat[ac.k].at(tl) / tl * seen);
        }
    });
    for (p, row) in places.iter().zip(sat[n * nx..].chunks_exact_mut(nx)) {
        let (tl, seen) = (p.touching(s, t0), p.cells.iter().zip(&p.area).map(|(&i, a)| s.seen[i as usize] * a).sum::<f64>());
        act.iter().zip(row).for_each(|(ac, r)| *r = s.sat[ac.k].at(tl) / tl * seen);
    }
    // The cells that see each node, in the order of the cells: their entries `at[q]..at[q + 1]` (cell, corner); a
    // patch's at its first.
    let (at, by) = (&s.corners.by_at, &s.corners.by);
    let nodes = crate::par::map_n(na, |q| {
        let (mut place, mut share, mut first) = (Vec::new(), Vec::new(), Vec::<(u32, usize)>::new());
        for &(i, c) in &by[at[q]..at[q + 1]] {
            let (p, a) = (place_of[i as usize], s.wt[i as usize][c as usize] * over_cell(w, s, i as usize));
            if p as usize >= n {
                if let Some(&(_, e)) = first.iter().find(|f| f.0 == p) {
                    share[e] += a;
                    continue;
                }
                first.push((p, place.len()));
            }
            place.push(p);
            share.push(a);
        }
        let (sat, node, places) = (&sat, &node, &place);
        let ratio = (0..nx).flat_map(|y| places.iter().map(move |&p| sat[p as usize * nx + y] / air.area()[q] / node[q * nx + y])).collect();
        Seen::new(place, share, ratio)
    });
    Wet { nodes, patches: places, place_of }
}

/// What rains of one gas on each cell from each node it sees over the step: (cell, node, mol), in the order of the nodes.
pub(super) type Landed = Vec<(u32, u32, f64)>;

/// The air the rain of active gas `y` of node `j` meets on a day it rains at its saturation ratio `ratio`: the air
/// touching the places it falls on (`touched`, per place, `Wet::touched` under the day's air), each by its share of the
/// rain as it lands (`landed`), K; none where it falls on no place.
pub(super) fn rain_air(wet: &Wet, (y, j, ratio): (usize, usize, f64), touched: &[f64]) -> Option<f64> {
    let node = &wet.nodes[j];
    let len = node.cell.len();
    let over = |e: usize| node.share[e] * (ratio - node.ratio[y * len + e]).max(0.0);
    let (sum, all) = (0..len).map(|e| (e, over(e))).filter(|p| p.1 > 0.0).fold((0.0, 0.0), |(sum, all), (e, o)| (sum + o * touched[node.cell[e] as usize], all + o));
    (all > 0.0).then(|| sum / all)
}

/// What rains on each cell from each node it sees over the step (`Landed`), per active gas: each node's rain shared
/// among the places that see it by what each would hold over its own saturation at the ratio the node rained at
/// (`Wet`), a patch's among its cells by their areas.
pub(super) fn landed(act: &[Active], wet: &Wet, mean: &Mean) -> Vec<Landed> {
    let (na, n) = (wet.nodes.len(), wet.place_of.len());
    (0..act.len())
        .map(|x| {
            let mut out = Vec::new();
            for (j, node) in wet.nodes.iter().enumerate().filter(|(j, _)| mean.rain[x * na + j] > 0.0) {
                let len = node.cell.len();
                let over = |e: usize| node.share[e] * (mean.ratio[x * na + j] - node.ratio[x * len + e]).max(0.0);
                let all: f64 = (0..len).map(over).sum();
                for e in (0..len).filter(|&e| over(e) > 0.0) {
                    let (p, m) = (node.cell[e] as usize, mean.rain[x * na + j] * over(e) / all);
                    match p.checked_sub(n).map(|p| &wet.patches[p]) {
                        None => out.push((p as u32, j as u32, m)),
                        Some(patch) => out.extend(patch.cells.iter().zip(&patch.area).map(|(&i, a)| (i, j as u32, m * a))),
                    }
                }
            }
            out
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Pool, Seen, Wet};

    /// A block's day (`Pool::day`) is what small steps of the same rates give: its vapour rises through the ratios at
    /// which its nodes' cells start to rain (two nodes, three cells) toward the balance of what its ground gives and
    /// what rains.
    #[test]
    fn a_block_follows_its_ground_and_its_rain_through_the_day() {
        let wet = Wet { nodes: vec![Seen::new(vec![0, 1], vec![0.3, 0.7], vec![1.1, 0.9]), Seen::new(vec![2], vec![1.0], vec![0.95])], patches: vec![], place_of: vec![0, 1, 2] };
        let pool = Pool { c: 3.0, s: -0.1, v: vec![0.6, 0.4], off: vec![0.0, 0.0], full: vec![1.0, 0.8] };
        let (dd, dt, tau, x0) = (1e5, 1e5, 2e4, 1.0);
        let (mut rain, mut ratio) = (vec![0.0; 2], vec![0.0; 2]);
        let mean = pool.day((&wet, 0, &[0, 1]), x0, (dd, dt, tau), (&mut rain, &mut ratio));
        // The same by small explicit steps.
        const STEPS: usize = 1_000_000;
        let (h, mut x, mut sum, mut fell) = (dd / STEPS as f64, x0, 0.0, [0.0; 2]);
        // What node `q` rains a second: each cell's share of what the node holds over its ratio.
        let over = |q: usize, x: f64| {
            let (u, cells): (f64, &[(f64, f64)]) = (pool.v[q] * x / pool.full[q], if q == 0 { &[(1.1, 0.3), (0.9, 0.7)] } else { &[(0.95, 1.0)] });
            cells.iter().map(|&(r, a)| a * (u - r).max(0.0)).sum::<f64>() * pool.full[q] / tau
        };
        for _ in 0..STEPS {
            let (r0, r1) = (over(0, x), over(1, x));
            sum += x * h;
            (fell[0], fell[1]) = (fell[0] + r0 * h, fell[1] + r1 * h);
            x += h * ((pool.c + pool.s * x) / dt - r0 - r1);
        }
        assert!((mean / (sum / dd) - 1.0).abs() < 1e-4, "mean {mean} against {}", sum / dd);
        for q in 0..2 {
            assert!(fell[q] > 0.1 && (rain[q] / fell[q] - 1.0).abs() < 1e-3, "node {q}: rain {} against {}", rain[q], fell[q]);
        }
    }
}
