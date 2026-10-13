//! Numbers computed from the world's raw facts: what counts as a light-eater, how far learning
//! has moved the brains, and how many kinds of bodies there are. The core knows none of this.

use pcore::body::{self, Body, Layout, COPY_MASS, INTERVAL, LEAF, STRENGTH};
use pcore::chem::Chemistry;
use pcore::World;

/// A body whose light reactions are sped up at least this much (log10) is a light-eater.
pub const LIGHT_EATER: f32 = 8.0;

pub fn light_eater(chem: &Chemistry, genes: &[f32]) -> bool {
    body::light_cat(chem, genes) >= LIGHT_EATER
}

/// Mean absolute learned offset of each live group's rows (its rows less the mean of its members'
/// born rows), averaged over groups by their members.
pub fn learned(w: &World) -> f64 {
    let (mut sum, mut members) = (0.0, 0.0);
    for g in w.life.groups.iter().filter(|g| g.members > 0 && !g.rows.is_empty()) {
        let d: f64 = g.learned().map(|x| x.abs() as f64).sum();
        (sum, members) = (sum + d / g.rows.len() as f64 * g.members as f64, members + g.members as f64);
    }
    sum / members.max(1.0)
}

/// What makes bodies kinds, one dimension each: log10 copy mass, log10 interval, strength, log10
/// light-capture area and the light-reaction catalyst (the first `FIXED` places, scales in `SCALE`),
/// then the diet: the catalyst of every other reaction (off counts as 0; `CAT_SCALE` decades) and
/// the uptake gene of every substance (off as 0; `UPT_SCALE`). The diet has one place per reaction
/// and per substance, not "the two largest": a charcoal-eater and a sugar-eater whose largest
/// catalysts are equal still differ in which reaction they are, and two bodies with the same diet
/// are equal whatever order their genes lie in. A kind differs by about one scale. The cost is
/// about 45 dimensions on a sample of `SAMPLE` bodies, a few milliseconds a year.
const FIXED: usize = 5;
const SCALE: [f32; FIXED] = [1.0, 0.3, 0.25, 1.0, 4.0];
const CAT_SCALE: f32 = 4.0;
const UPT_SCALE: f32 = 1.0;
/// Most kinds looked for; most bodies clustered (an even sample of the live bodies); rounds of
/// the clustering.
const MAX_K: usize = 6;
const SAMPLE: usize = 2000;
const ROUNDS: usize = 20;
/// Kinds are clearly separated if the nearest two centres are further apart than this many times
/// the widest kind's rms radius; a kind needs this share of the bodies (or members) and at
/// least 3 sampled bodies (1 when counting members: one node of 10^8 members is a kind).
const SEP: f32 = 3.0;
const MIN_SHARE: f64 = 0.01;

/// The genes that make the point of a body after the first `FIXED`, with the scale of each.
fn diet(chem: &Chemistry) -> Vec<(usize, f32)> {
    let lay = Layout::of(chem);
    let cat = (0..chem.reactions.len()).filter(|&r| !chem.reactions[r].light).map(|r| (lay.cat + r, CAT_SCALE));
    cat.chain((0..chem.ns()).map(|k| (lay.upt + k, UPT_SCALE))).collect()
}

fn point(chem: &Chemistry, diet: &[(usize, f32)], b: &Body) -> Vec<f32> {
    let g = &b.genes;
    let x = [g[COPY_MASS], g[INTERVAL], g[STRENGTH], g[LEAF], body::light_cat(chem, g)];
    (0..FIXED).map(|j| x[j] / SCALE[j]).chain(diet.iter().map(|&(i, s)| g[i].max(0.0) / s)).collect()
}

fn d2(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum()
}

fn nearest(c: &[Vec<f32>], p: &[f32]) -> usize {
    (0..c.len()).min_by(|&a, &b| d2(&c[a], p).total_cmp(&d2(&c[b], p))).unwrap_or(0)
}

/// Weighted k-means with `k` centres: the first is the point nearest the mean, each next the
/// point furthest from the centres so far (ties to the lower index); then `ROUNDS` rounds of
/// Lloyd with each point counting its weight. Returns the centres and the kind of each point.
fn kmeans(p: &[Vec<f32>], wt: &[f64], k: usize) -> (Vec<Vec<f32>>, Vec<usize>) {
    let (dim, tot) = (p[0].len(), wt.iter().sum::<f64>());
    let mean: Vec<f32> = (0..dim).map(|j| (p.iter().zip(wt).map(|(x, &w)| x[j] as f64 * w).sum::<f64>() / tot) as f32).collect();
    let first = (0..p.len()).min_by(|&a, &b| d2(&p[a], &mean).total_cmp(&d2(&p[b], &mean))).unwrap_or(0);
    let mut c = vec![p[first].clone()];
    while c.len() < k {
        let far = (0..p.len()).max_by(|&a, &b| d2(&p[a], &c[nearest(&c, &p[a])]).total_cmp(&d2(&p[b], &c[nearest(&c, &p[b])])).then(b.cmp(&a)));
        c.push(p[far.unwrap_or(0)].clone());
    }
    let mut of = vec![0; p.len()];
    for _ in 0..ROUNDS {
        for (o, x) in of.iter_mut().zip(p) {
            *o = nearest(&c, x);
        }
        let (mut sum, mut cnt) = (vec![vec![0f64; dim]; k], vec![0f64; k]);
        for ((x, &o), &w) in p.iter().zip(&of).zip(wt) {
            sum[o].iter_mut().zip(x).for_each(|(s, &v)| *s += v as f64 * w);
            cnt[o] += w;
        }
        for j in (0..k).filter(|&j| cnt[j] > 0.0) {
            c[j] = sum[j].iter().map(|&v| (v / cnt[j]) as f32).collect();
        }
    }
    for (o, x) in of.iter_mut().zip(p) {
        *o = nearest(&c, x);
    }
    (c, of)
}

/// The centres of the kinds of the live bodies (none if they are not clearly separated, or too
/// few to tell): the largest k from 2 to `MAX_K` whose kinds are clearly separated (see `SEP`,
/// `MIN_SHARE`). Each sampled body counts once, or, `by_members`, as many times as it has
/// members, so a node of 10^8 and a lone individual do not weigh the same.
fn fit(w: &World, by_members: bool) -> Vec<Vec<f32>> {
    let diet = diet(&w.chem);
    let stride = w.life.alive().count().div_ceil(SAMPLE).max(1);
    let (sample, wt): (Vec<Vec<f32>>, Vec<f64>) = w.life.alive().step_by(stride).map(|b| (point(&w.chem, &diet, b), if by_members { b.n() } else { 1.0 })).unzip();
    if sample.len() < 6 {
        return vec![];
    }
    let (total, min_pts) = (wt.iter().sum::<f64>(), if by_members { 1 } else { 3 });
    for k in (2..=MAX_K).rev() {
        let (c, of) = kmeans(&sample, &wt, k);
        let (mut cnt, mut pts, mut r2) = (vec![0f64; k], vec![0usize; k], vec![0f64; k]);
        for ((x, &o), &m) in sample.iter().zip(&of).zip(&wt) {
            (cnt[o], pts[o]) = (cnt[o] + m, pts[o] + 1);
            r2[o] += m * d2(x, &c[o]) as f64;
        }
        let wide = (0..k).map(|j| (r2[j] / cnt[j].max(1e-9)).sqrt() as f32).fold(0f32, f32::max);
        let gap = (0..k).flat_map(|a| (a + 1..k).map(move |b| (a, b))).map(|(a, b)| d2(&c[a], &c[b]).sqrt()).fold(f32::MAX, f32::min);
        if (0..k).all(|j| cnt[j] >= MIN_SHARE * total && pts[j] >= min_pts) && gap > SEP * wide {
            return c;
        }
    }
    vec![]
}

/// Kinds of the live bodies (in the order of `World::life.alive()`): the kind of each, and the
/// number of kinds, each body counting once.
pub fn kinds(w: &World) -> (Vec<usize>, usize) {
    let c = fit(w, false);
    if c.is_empty() {
        return (vec![0; w.life.alive().count()], 1);
    }
    let diet = diet(&w.chem);
    (w.life.alive().map(|b| nearest(&c, &point(&w.chem, &diet, b))).collect(), c.len())
}

/// The number of kinds when each body counts as many times as it has members.
pub fn kinds_by_members(w: &World) -> usize {
    fit(w, true).len().max(1)
}
