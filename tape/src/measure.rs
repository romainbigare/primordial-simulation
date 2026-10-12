//! Numbers computed from the world's raw facts: what counts as a light-eater, how far learning
//! has moved the brains, and how many kinds of bodies there are. The core knows none of this.

use pcore::body::{self, Body, COPY_MASS, INTERVAL, LEAF, STRENGTH};
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

/// Scales of the five genes that make bodies kinds: log10 copy mass, log10 interval, strength,
/// log10 light-capture area, and the light-reaction catalyst. A kind differs by about one scale.
const SCALE: [f32; 5] = [1.0, 0.3, 0.25, 1.0, 4.0];
/// Most kinds looked for; most bodies clustered (an even sample of the live bodies); rounds of
/// the clustering.
const MAX_K: usize = 6;
const SAMPLE: usize = 2000;
const ROUNDS: usize = 20;
/// Kinds are clearly separated if the nearest two centres are further apart than this many times
/// the widest kind's rms radius; a kind needs this share of the bodies (and at least 3).
const SEP: f32 = 3.0;
const MIN_SHARE: f64 = 0.01;

fn point(chem: &Chemistry, b: &Body) -> [f32; 5] {
    let g = &b.genes;
    let x = [g[COPY_MASS], g[INTERVAL], g[STRENGTH], g[LEAF], body::light_cat(chem, g)];
    [0, 1, 2, 3, 4].map(|j| x[j] / SCALE[j])
}

fn d2(a: &[f32; 5], b: &[f32; 5]) -> f32 {
    a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum()
}

fn nearest(c: &[[f32; 5]], p: &[f32; 5]) -> usize {
    (0..c.len()).min_by(|&a, &b| d2(&c[a], p).total_cmp(&d2(&c[b], p))).unwrap_or(0)
}

/// k-means with `k` centres: the first is the point nearest the mean, each next the point
/// furthest from the centres so far (ties to the lower index); then `ROUNDS` rounds of Lloyd.
/// Returns the centres and the kind of each point.
fn kmeans(p: &[[f32; 5]], k: usize) -> (Vec<[f32; 5]>, Vec<usize>) {
    let mean = p.iter().fold([0f32; 5], |m, x| [0, 1, 2, 3, 4].map(|j| m[j] + x[j] / p.len() as f32));
    let first = (0..p.len()).min_by(|&a, &b| d2(&p[a], &mean).total_cmp(&d2(&p[b], &mean))).unwrap_or(0);
    let mut c = vec![p[first]];
    while c.len() < k {
        let far = (0..p.len()).max_by(|&a, &b| d2(&p[a], &c[nearest(&c, &p[a])]).total_cmp(&d2(&p[b], &c[nearest(&c, &p[b])])).then(b.cmp(&a)));
        c.push(p[far.unwrap_or(0)]);
    }
    let mut of = vec![0; p.len()];
    for _ in 0..ROUNDS {
        for (o, x) in of.iter_mut().zip(p) {
            *o = nearest(&c, x);
        }
        let (mut sum, mut cnt) = (vec![[0f32; 5]; k], vec![0f32; k]);
        for (x, &o) in p.iter().zip(&of) {
            sum[o] = [0, 1, 2, 3, 4].map(|j| sum[o][j] + x[j]);
            cnt[o] += 1.0;
        }
        for j in (0..k).filter(|&j| cnt[j] > 0.0) {
            c[j] = sum[j].map(|v| v / cnt[j]);
        }
    }
    for (o, x) in of.iter_mut().zip(p) {
        *o = nearest(&c, x);
    }
    (c, of)
}

/// Kinds of the live bodies (in the order of `World::life.alive()`): the kind of each, and the
/// number of kinds. The largest k from 1 to `MAX_K` whose kinds are clearly separated (see
/// `SEP`, `MIN_SHARE`) is used; one kind if none is.
pub fn kinds(w: &World) -> (Vec<usize>, usize) {
    let all: Vec<[f32; 5]> = w.life.alive().map(|b| point(&w.chem, b)).collect();
    let stride = all.len().div_ceil(SAMPLE).max(1);
    let sample: Vec<[f32; 5]> = all.iter().step_by(stride).copied().collect();
    if sample.len() < 6 {
        return (vec![0; all.len()], 1);
    }
    for k in (2..=MAX_K).rev() {
        let (c, of) = kmeans(&sample, k);
        let mut cnt = vec![0usize; k];
        let mut r2 = vec![0f32; k];
        for (x, &o) in sample.iter().zip(&of) {
            cnt[o] += 1;
            r2[o] += d2(x, &c[o]);
        }
        let need = (MIN_SHARE * sample.len() as f64).ceil().max(3.0) as usize;
        let wide = (0..k).map(|j| (r2[j] / cnt[j].max(1) as f32).sqrt()).fold(0f32, f32::max);
        let gap = (0..k).flat_map(|a| (a + 1..k).map(move |b| (a, b))).map(|(a, b)| d2(&c[a], &c[b]).sqrt()).fold(f32::MAX, f32::min);
        if cnt.iter().all(|&n| n >= need) && gap > SEP * wide {
            return (all.iter().map(|x| nearest(&c, x)).collect(), k);
        }
    }
    (vec![0; all.len()], 1)
}
