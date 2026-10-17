//! Population nodes: the monthly merge of alike nodes in each cell (`crowd`), a body joining another (`merge`, `join`),
//! a node splitting in two (`split`), and members drawn from a node as individuals (`one`, `open`). Members, stores and
//! heat are kept exactly in every join and split.

use crate::body::{self, alike, alike_log, class, light_eater, most, noise, range, size_step, spread_at, spreads, steps, wall_room, within_steps, Body, Layout, REACH};
use crate::brain::{row_step, shake};
use crate::lineage::own;
use crate::rng::{Purpose, Rng};
use crate::life::Life;
use crate::World;
use std::f32::consts::FRAC_2_PI;

/// Past this many times `Config::crowd` bodies in a cell, unlike nodes merge too (the most alike first): a numerical
/// safeguard.
const UNLIKE_CROWD: usize = 2;

/// The monthly merge of nodes, a speed shortcut, not a law: it coarsens only how members are represented (as population
/// nodes), never how many there are or where. In each cell, among the bodies that hold and are inside nothing, a node
/// joins the first larger one alike to it (`join_alike`); then, while more than `Config::crowd` remain, the most alike
/// pairs merge (`merge_closest`). Stores and heat add up, genes and born rows become the means by count, and the spread
/// of both is kept (`merge`); lineage, brain group and memory are the larger's. A node that grew too varied splits
/// again at its next update (`split`). With `Config::nodes` off, eaters never merge and never count toward
/// `crowd`.
pub(crate) fn crowd(w: &mut World) {
    let (max, lay) = (w.cfg.crowd.max(1), Layout::of(&w.chem));
    for i in 0..w.grid.n() {
        if w.life.at[i].len() <= 1 {
            continue;
        }
        let left = join_alike(w, &lay, i);
        if left.len() > max {
            merge_closest(w, &lay, left, max);
        }
    }
}

/// Each node of cell `i`, largest first, joins the first larger one alike in genes (`body::alike`, on the means only, so
/// a split's halves stay apart for selection) and of its size class (`body::class`). Returns the bodies left.
fn join_alike(w: &mut World, lay: &Layout, i: usize) -> Vec<usize> {
    let b = &w.life.bodies;
    let mut list: Vec<(u64, usize)> = w.life.at[i].iter().map(|&s| s as usize).filter(|&s| b[s].hold == 0 && b[s].inside == 0 && may_be_node(w, &b[s].genes)).map(|s| (u64::MAX - b[s].n().to_bits(), s)).collect();
    list.sort_unstable();
    let mut head: Vec<usize> = (0..list.len()).collect();
    // Each one's class once, tested before the genes.
    let class: Vec<(f64, bool)> = list.iter().map(|&(_, s)| class(&w.chem, &b[s])).collect();
    for x in 0..list.len() {
        let a = &b[list[x].1];
        if a.n() > 1.0 {
            let lim = w.cfg.phys.alike;
            head[x] = (0..x).find(|&y| head[y] == y && class[y].1 == class[x].1 && alike_log(lay, lim, class[y].0, class[x].0) && alike(lay, lim, &b[list[y].1].genes, &a.genes)).unwrap_or(x);
        }
    }
    for x in (0..list.len()).filter(|&x| head[x] != x) {
        merge(w, list[x].1, list[head[x]].1);
    }
    (0..list.len()).filter(|&x| head[x] == x).map(|x| list[x].1).collect()
}

/// A body's genes in steps (`body::steps`), its spread (`body::spreads`) and its size class (`merge_closest`).
type Likeness = (Vec<f32>, Vec<f32>, (f64, bool));

/// The most alike pair of `left` merges first, the smaller into the larger (ties: the earlier, larger ones), until `max`
/// are left or no pair is alike: here each within the other's spread, as children join a node (`body::within`), and of
/// one size class. Past `UNLIKE_CROWD` times `max`, unlike pairs merge too, the most alike first. A pair's key: unlike or
/// not, then the squared distance of their genes in steps.
fn merge_closest(w: &mut World, lay: &Layout, mut left: Vec<usize>, max: usize) {
    let (m, lim) = (left.len(), w.cfg.phys.alike);
    let steps = |w: &World, s: usize| (steps(lay, &w.life.bodies[s].genes), spreads(&w.life.bodies[s]), class(&w.chem, &w.life.bodies[s]));
    let mut v: Vec<Likeness> = left.iter().map(|&s| steps(w, s)).collect();
    let reach = lim as f64 * size_step(lay) as f64;
    let dist = |p: &Likeness, q: &Likeness| {
        let gap = |g: usize| (p.0[g] - q.0[g]).abs() > within_steps(lay, lim, p.1[g] + q.1[g], g);
        (p.2 .1 != q.2 .1 || (p.2 .0 - q.2 .0).abs() > reach || (0..p.0.len()).any(gap), p.0.iter().zip(&q.0).map(|(x, y)| (x - y) * (x - y)).sum::<f32>())
    };
    let mut d = vec![(true, f32::INFINITY); m * m];
    for p in 0..m {
        for q in p + 1..m {
            d[p * m + q] = dist(&v[p], &v[q]);
        }
    }
    for n in (max..m).rev() {
        let k = (0..m * m).fold(0, |k, x| if d[x] < d[k] { x } else { k });
        if d[k].1 == f32::INFINITY || d[k].0 && n < UNLIKE_CROWD * max {
            break;
        }
        let (p, q) = (k / m, k % m);
        let (s, t) = if w.life.bodies[left[q]].n() > w.life.bodies[left[p]].n() { (p, q) } else { (q, p) };
        merge(w, left[s], left[t]);
        (left[s], v[t]) = (usize::MAX, steps(w, left[t]));
        for o in 0..m {
            let (lo, hi) = (o.min(s), o.max(s));
            d[lo * m + hi] = (true, f32::INFINITY);
            if o != t && left[o] != usize::MAX {
                let (lo, hi) = (o.min(t), o.max(t));
                d[lo * m + hi] = dist(&v[o], &v[t]);
            }
        }
    }
}

/// A body of these genes may be or join a population node: any body with `Config::nodes`, else only a light-eater.
pub(crate) fn may_be_node(w: &World, genes: &[f32]) -> bool {
    w.cfg.nodes || light_eater(&w.chem, genes)
}

/// The body in slot `s` joins the body in slot `t` (`join`), its born rows carried over to the sizes of the other's brain
/// where they differ.
pub fn merge(w: &mut World, s: usize, t: usize) {
    let c = w.life.bodies[s].clone();
    let mut r = Rng::new(w.seed, w.day, c.id(s), Purpose::Merge);
    let rows = w.life.carry_rows(w.life.born(&c).to_vec(), c.lineage, w.life.bodies[t].lineage, &mut r, w.cfg.phys.new_noise);
    w.life.remove(s);
    join(w, t, c, &rows);
}

/// Members `c` (not placed, born with `rows`, in the shape of `t`'s brain) join body `t` (`take_in`).
pub fn join(w: &mut World, t: usize, c: Body, rows: &[f32]) {
    let mut o = std::mem::take(&mut w.life.bodies[t]);
    take_in(w, &mut o, c, Some(rows));
    w.life.bodies[t] = o;
}

/// Members `c` join body `o`, taken out of its slot: members, stores, brood and heat add up; genes and born rows become
/// the means by members (children of less than one member weigh what they are), and the spread what both held around
/// the new mean (each one's spread plus its gap from the new mean squared, by members; for the rows the mean over rows
/// of the gap squared; up to `most`), so the smaller one's genes stay in the spread, not only in the mean. `rows` are
/// the born rows of `c`; none: those of `o` (a member split off `o` coming back, `one`), which leaves its rows, their
/// spread and its group's as they are: it left with the rows' mean and comes back with it.
pub(crate) fn take_in(w: &mut World, o: &mut Body, c: Body, rows: Option<&[f32]>) {
    let (lay, p, a) = (Layout::of(&w.chem), &w.cfg.phys, (c.count / (o.count + c.count)) as f32);
    let mix = |x: &[f32], y: &[f32]| x.iter().zip(y).map(|(p, q)| p + a * (q - p)).collect::<Vec<f32>>();
    let pool = |v: f32, u: f32, d2: f32, step: f32| ((1.0 - a) * v + a * u + a * (1.0 - a) * d2).min(most(step, p.alike));
    o.spread = (0..o.genes.len()).map(|g| pool(spread_at(&o.spread, g), spread_at(&c.spread, g), (c.genes[g] - o.genes[g]).powi(2), range(&lay, g).0)).collect();
    let genes = mix(&o.genes, &c.genes);
    match rows {
        Some(rows) => {
            let born = w.life.born(o);
            let gap = born.iter().zip(rows).map(|(p, q)| (q - p) * (q - p)).sum::<f32>() / born.len().max(1) as f32;
            o.row_spread = pool(o.row_spread, c.row_spread, gap, row_step(p, body::mutation_size(&o.genes)));
            let rows = mix(born, rows);
            w.life.reborn(o, genes, rows);
        }
        None => w.life.regene(o, genes),
    }
    o.store.iter_mut().zip(&c.store).for_each(|(s, x)| *s += x);
    o.brood += c.brood;
    // A node has no wear: its members are of all ages (`body::live`).
    (o.e, o.wear) = (o.e + c.e, 0.0);
    w.life.set_count(o, o.count + c.count);
    o.vis = o.visible(&w.chem, &w.cfg.phys);
}

/// Share `f` of node `b`'s members as a new body (not placed): that share of each store, of the brood and of the heat;
/// lineage, group, genome and memory as the node's; holding nothing, inside nothing.
fn part(life: &mut Life, b: &mut Body, f: f64) -> Body {
    // (The trace and the constants are not the part's: set aside, not cloned.)
    let (trace, k) = (std::mem::take(&mut b.trace), std::mem::take(&mut b.k));
    let mut c = Body { count: f * b.count, e: f * b.e, brood: f * b.brood, hold: 0, inside: 0, ..b.clone() };
    (b.trace, b.k) = (trace, k);
    for (a, x) in b.store.iter_mut().zip(c.store.iter_mut()) {
        *x = f * *a;
        *a -= *x;
    }
    (b.e, b.brood) = (b.e - c.e, b.brood - c.brood);
    life.set_count(b, b.count - c.count);
    c
}

/// Genes as wide as the widest but for this share (rounding) tie for the split.
const TIE: f32 = 1e-4;

/// At its update, a node whose two halves would not be alike splits in two, so selection acts between them. The halves
/// of a normal spread have means `d = sqrt(2/π)` standard deviations from the mean: it splits when their gap passes
/// `Physics::alike` steps along the gene where it is widest in its mutation steps (a gene that is off counts as 0; among
/// genes as wide, a keyed random pick). `d` is shrunk to the room left to the gene's walls, so the mean is kept, and
/// the variance along it falls by `d²`, so the spread of both together is kept. Each half has half the members, stores
/// and heat. Halves of a brain size are alike (likeness counts the sizes by whole units), so the monthly merge takes
/// them back, each having paid for its own size; a mean a whole unit from its lineage's size moves the node to a
/// lineage of that size (`lineage::resize`). It splits only while its cell keeps a free place after it (fewer than
/// `Config::crowd` - 1 bodies): a node held at `most` splits at every update, and without the limit the nodes of a cell
/// would double each time. The spread stops at `most` and a split keeps the mean, so the limit does not bias the genes.
pub fn split(w: &mut World, slot: usize, b: &mut Body) {
    if b.count < 2.0 || b.spread.is_empty() || w.life.at[b.cell as usize].len() + 1 >= w.cfg.crowd {
        return;
    }
    if !may_be_node(w, &b.genes) {
        return;
    }
    let (lay, p) = (Layout::of(&w.chem), &w.cfg.phys);
    // Per gene the halves' shift from the mean (within the walls) and their gap in steps (one look-up of the range:
    // most updates do not split).
    let half = |g: usize| {
        let ((u, lo, hi), m) = (range(&lay, g), b.genes[g]);
        let d = (FRAC_2_PI * b.spread[g].max(0.0)).sqrt().min(wall_room(m, lo, hi));
        let (x, y) = if g < lay.cat { (m + d, m - d) } else { ((m + d).max(0.0), (m - d).max(0.0)) };
        (d, x / u - y / u)
    };
    let top = (0..b.spread.len()).fold(0.0f32, |a, g| a.max(half(g).1));
    if top <= p.alike {
        return;
    }
    let mut gap: Vec<(usize, f32)> = (0..b.spread.len()).map(|g| (g, half(g).1)).collect();
    gap.retain(|x| x.1 >= top * (1.0 - TIE));
    let mut r = Rng::new(w.seed, w.day, b.id(slot), Purpose::Split);
    let g = gap[r.below(gap.len())].0;
    let d = half(g).0;
    let mut c = part(&mut w.life, b, 0.5);
    let (mut genes, rows) = (b.genes.clone(), w.life.born(b).to_vec());
    // Which half keeps the slot (and so is first in the order of updates) is drawn.
    let d = if r.below(2) == 0 { d } else { -d };
    (genes[g], c.genes[g]) = (genes[g] + d, genes[g] - d);
    b.spread[g] = (b.spread[g] - d * d).max(0.0);
    c.spread[g] = b.spread[g];
    w.life.reborn(b, genes, rows);
    own(w, b, &mut c);
    w.counts.splits[0] += 1.0;
    w.place(c, Some(w.light_keep.get(slot, b.gen)));
}

/// One member of node `b` (at least two) split off as an individual, to do what only a single body can (the actions of
/// Bond, `choose::act`): its genes the node's plus noise of their spread (shrunk near a wall, so it is never clamped,
/// as in `body::vary`), a share 1/count of its stores and heat (`part`). The node's genes and spread become those of the
/// others, so the member going back (`take_in`) gives them back. Its wear is drawn uniform in [0, 1): members who each
/// live as long are of all ages evenly, so members split off die at the rate the node's members die (`body::live`).
/// Placed; returns its slot.
pub fn one(w: &mut World, b: &mut Body, r: &mut Rng) -> usize {
    w.counts.splits[1] += 1.0;
    draw(w, b, r, None, false)
}

/// A count this close to a whole number is that number.
pub(crate) const COUNT_SLACK: f64 = 1e-9;

/// `n` members of the node in `slot` as individuals, each drawn as `one` draws a member and with born rows of its own,
/// drawn as its genes are from their spread (`Body::row_spread`), placed in the node's cell: the close view of a node,
/// and what the check of nodes against individuals compares. Each draw leaves the others' genes and rows, so the mean
/// of all is kept. The node keeps at least one member (draws stop when it has fewer than two). Returns the members'
/// slots.
pub fn open(w: &mut World, slot: usize, n: usize) -> Vec<usize> {
    let mut b = std::mem::take(&mut w.life.bodies[slot]);
    let (mut r, keep) = (Rng::new(w.seed, w.day, b.id(slot), Purpose::Open), w.light_keep.get(slot, b.gen));
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let whole = b.count.round();
        if (b.count - whole).abs() < COUNT_SLACK {
            w.life.set_count(&mut b, whole);
        }
        if b.count < 2.0 {
            break;
        }
        out.push(draw(w, &mut b, &mut r, Some(keep), true));
    }
    w.life.bodies[slot] = b;
    out
}

/// One member drawn from node `b` (`one`), placed with the light-keep share `keep`; with `own_rows`, born with rows of
/// its own (`member_rows`), else with the node's.
fn draw(w: &mut World, b: &mut Body, r: &mut Rng, keep: Option<f32>, own_rows: bool) -> usize {
    let (lay, n) = (Layout::of(&w.chem), b.count as f32);
    // (The member keeps no spread: it is left out of the clone.)
    let spread = std::mem::take(&mut b.spread);
    let (mut c, mut genes) = (part(&mut w.life, b, 1.0 / b.count), b.genes.clone());
    b.spread = spread;
    for (g, x) in c.genes.iter_mut().enumerate() {
        let ((_, lo, hi), v) = (range(&lay, g), spread_at(&b.spread, g));
        *x += v.sqrt().min(wall_room(*x, lo, hi) / REACH) * noise(r);
        let d = *x - genes[g];
        genes[g] = (genes[g] - d / (n - 1.0)).clamp(lo, hi);
        if let Some(s) = b.spread.get_mut(g) {
            *s = ((n * v - d * d * n / (n - 1.0)) / (n - 1.0)).max(0.0);
        }
    }
    if own_rows && b.row_spread > 0.0 {
        let (mine, left, spread) = member_rows(w.life.born(b), b.row_spread, n, r);
        (c.genome, b.row_spread) = (w.life.new_genome(mine), spread);
        w.life.reborn(b, genes, left);
    } else {
        w.life.regene(b, genes);
    }
    (c.count, c.spread, c.row_spread, c.wear) = (1.0, Vec::new(), 0.0, r.f64());
    own(w, b, &mut c);
    w.place(c, keep)
}

/// The born rows of one member drawn from `n` members whose rows have mean `rows` and variance `v` (`brain::shake`),
/// the mean of the others' rows, and the variance left among them, as `draw` draws genes.
fn member_rows(rows: &[f32], v: f32, n: f32, r: &mut Rng) -> (Vec<f32>, Vec<f32>, f32) {
    let mut mine = rows.to_vec();
    shake(&mut mine, r, v.sqrt());
    let d2 = mine.iter().zip(rows).map(|(x, m)| (x - m) * (x - m)).sum::<f32>() / rows.len().max(1) as f32;
    let left = rows.iter().zip(&mine).map(|(m, x)| m - (x - m) / (n - 1.0)).collect();
    (mine, left, ((n * v - d2 * n / (n - 1.0)) / (n - 1.0)).max(0.0))
}
