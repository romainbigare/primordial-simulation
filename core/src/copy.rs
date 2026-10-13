//! Law 4: Copy. A body whose members have enough mass, if it can pay the cost, makes a child
//! per member: a share of each of its stores and of its heat, and its genes plus small noise
//! (now and then a change of the brain rows it was born with, else it shares its parent's); it
//! starts in its parent's brain group (or, if brains are not shared, in a group of its own with
//! what its parent learned). A child whose brain-size genes are a whole unit away from its
//! lineage's sizes starts a new lineage (and group) with the new sizes, its brain carried over
//! where the sizes overlap; rarely the base of its lineage's brain is copied with small noise,
//! which starts a new lineage too. A member past `phys.full` times its copy mass has as many
//! children as it grew past that.
//!
//! A population node (count > 1) keeps how its members differ (plan 7.1): its spread, the
//! variance of their genes around its genes, per gene, and of their born rows around its rows
//! (one number for all rows). Its children that stay are new members of it and widen the spread
//! by their share of a generation's change (`widen`). A share of them (the `DISPERSE` gene; none
//! if less than one member) starts in a random neighbour cell, drawn from the node: their mean
//! genes and rows are the node's plus noise of its spread and the change over their number, and
//! they keep the variance left among them (`vary`). There they join the first node alike in genes
//! with a brain of the same sizes, whatever its lineage (means by count, the spread pooled,
//! `join`), or start a node. An individual's children stay together, in a neighbour cell with
//! that chance (else in its own); there they join an alike node as a node's do, else they are a
//! new body. Only individuals' children start new lineages; a node whose mean brain-size genes
//! are a whole unit away from its lineage's sizes moves itself to a new lineage with them, its
//! brain and what its group learned carried over (`resize`, at its update).
//!
//! A node whose halves would no longer be alike splits in two (`split`), so selection acts
//! between them; a node does the actions of Bond through one member split off as an individual
//! (`one`, from `choose::act`). Members, stores and heat are kept exactly in every split and join.

use crate::body::{self, Body, Layout, Lineage};
use crate::{brain, choose};
use crate::event::{Event, EventKind};
use crate::grid::DIRS;
use crate::rng::{Purpose, Rng};
use crate::{Physics, World};
use std::f32::consts::{FRAC_2_PI, FRAC_PI_8};

/// Reach of the noise `noise` draws (a sum of four uniforms, centred and scaled): 2·sqrt(3).
const REACH: f32 = 3.464_101_6;

/// Mutation step (times the body's mutation size) and range of a gene. Catalysts, safe minimums
/// and uptake are off below 0 (they cost nothing and do nothing there), and their ranges go 20
/// steps below 0, so an off gene wanders as far as the noise of a node's spread reaches (about 10
/// steps, `most`) without meeting a wall, whose clamp would push it up (review 3, M1). A catalyst
/// step is 0.3 decades, so nodes alike within `phys.alike` steps differ by a factor 16 at most.
fn range(lay: &Layout, g: usize) -> (f32, f32, f32) {
    match g {
        body::GAS => (0.3, -1.0, 5.0),
        body::LIQ => (0.3, -4.0, 1.0),
        body::HEAT => (0.3, -3.0, 3.0),
        body::LEAF => (0.3, -4.0, 1.0),
        body::STRENGTH => (0.1, 0.0, 1.0),
        body::CHILD => (0.05, 0.05, 0.5),
        body::COPY_MASS => (0.2, -3.0, 4.0),
        body::INTERVAL => (0.2, 0.7, 2.556),
        body::MUT => (0.2, -3.0, 0.0),
        body::LEARN => (0.2, -4.0, 0.0),
        body::FORGET => (0.2, -4.0, -1.7),
        body::HIDDEN => (0.3, 4.0, 64.0),
        body::MEMORY => (0.15, 0.0, 12.0),
        body::SEEN => (0.1, 1.0, 6.0),
        body::SIGHT => (0.07, 1.0, 4.0),
        body::DISPERSE => (0.05, 0.0, 1.0),
        body::DENATURE => (5.0, 250.0, 450.0),
        g if g < lay.min => (0.3, -6.0, 18.0),
        g if g < lay.upt => (0.05, -1.0, 0.9),
        g if g < lay.keep => (0.2, -4.0, 2.0),
        _ => (0.2, -0.5, 2.0),
    }
}

/// Genes no more than `limit` mutation steps apart in each gene (genes that are off count as 0;
/// the sizes of brain and senses are not compared, as `merge` carries a brain over to other sizes).
pub fn alike(lay: &Layout, limit: f32, a: &[f32], b: &[f32]) -> bool {
    a.iter().zip(b).enumerate().all(|(g, (&x, &y))| (step(lay, g, x) - step(lay, g, y)).abs() <= limit)
}

/// A gene in mutation steps (off as 0; the sizes of brain and senses as 0): two bodies are as
/// alike as their genes are near in these (`alike`, `body::crowd`).
fn step(lay: &Layout, g: usize, x: f32) -> f32 {
    if (body::HIDDEN..=body::SIGHT).contains(&g) { 0.0 } else { (if g < lay.cat { x } else { x.max(0.0) }) / range(lay, g).0 }
}
pub fn steps(lay: &Layout, a: &[f32]) -> Vec<f32> {
    a.iter().enumerate().map(|(g, &x)| step(lay, g, x)).collect()
}

/// Parent genes plus noise, kept in range.
pub fn mutate(lay: &Layout, genes: &[f32], r: &mut Rng) -> Vec<f32> {
    vary(lay, genes, &[], 1.0, 0.0, r).0
}

/// Noise near a standard normal: a sum of four uniforms, centred and scaled (within `REACH`).
fn noise(r: &mut Rng) -> f32 {
    (((0..4).map(|_| r.f64()).sum::<f64>() - 2.0) * 3f64.sqrt()) as f32
}

/// Most variance a node keeps along a gene (or its rows) of mutation step `step`, in its units
/// squared: the split threshold, where its halves (means `2·sqrt(2/π)` standard deviations apart)
/// are `alike` steps apart, `(alike · step)² π / 8` (about 6.3 step² with `alike` 4), plus one
/// step², so a node held there is past it and splits (`split`), while one that cannot split along
/// it (a gene that is off, or near a wall) widens no further. Nothing else takes variance out of
/// a node, so without it the spread grew with every generation and merge (review 3, M1).
fn most(step: f32, alike: f32) -> f32 {
    step * step * (alike * alike * FRAC_PI_8 + 1.0)
}

/// The genes of `m` members drawn from a node of mean `genes` and variance `spread` per gene
/// (empty: none), each changed by a generation's mutation: their mean (the node's, plus noise of
/// that variance over `m`), and the variance left among them ((1 - 1/m) of it, up to `most` for
/// `alike`; none for one). The spread's share of the noise is shrunk where it could cross a wall,
/// so the mean drawn is never clamped: a clamp would move the mean of a node's children away
/// from the wall, a drift no selection made (review 3, M1). A generation's own change may still
/// be clamped (one small step; a gene at a wall can still move off it).
pub fn vary(lay: &Layout, genes: &[f32], spread: &[f32], m: f64, alike: f32, r: &mut Rng) -> (Vec<f32>, Vec<f32>) {
    let (size, m, mut left) = (10f32.powf(genes[body::MUT]), m.max(1.0) as f32, Vec::new());
    let genes = (genes.iter().enumerate())
        .map(|(g, &v)| {
            let (step, lo, hi) = range(lay, g);
            let (u2, s2) = ((step * size).powi(2), spread.get(g).map_or(0.0, |s| s.max(0.0)));
            if m > 1.0 {
                left.push(((u2 + s2) * (1.0 - 1.0 / m)).min(most(step, alike)));
            }
            let room = (v - lo).min(hi - v).max(0.0) / REACH;
            let s2 = s2.min((room * room * m - u2).max(0.0));
            (v + ((u2 + s2) / m).sqrt() * noise(r)).clamp(lo, hi)
        })
        .collect();
    (genes, left)
}

/// A share `f` of a node's members are new, born of it with a generation's change (they stay):
/// its spread grows by `f` times that change's variance (variances add), up to `most`. Per gene,
/// its mutation's (`vary`); for the rows, a share `row_chance` of children changed by a step of
/// `row_noise` × size / sqrt(row_chance).
fn widen(lay: &Layout, p: &Physics, b: &mut Body, f: f32) {
    let size = 10f32.powf(b.genes[body::MUT]);
    b.spread.resize(b.genes.len(), 0.0);
    for (g, s) in b.spread.iter_mut().enumerate() {
        let step = range(lay, g).0;
        *s = (*s + f * (step * size).powi(2)).min(most(step, p.alike));
    }
    b.row_spread = (b.row_spread + f * (p.row_noise * size).powi(2)).min(most(row_step(p, size), p.alike));
}

/// The step of a node's born rows: that of a child whose rows changed, for mutation size `size`.
fn row_step(p: &Physics, size: f32) -> f32 {
    p.row_noise * size / (p.row_chance as f32).sqrt()
}

/// Add noise near a normal of spread `size` to each weight (a sum of four 16-bit uniforms from
/// one draw, centred and scaled: cheap, as a brain has many weights).
pub fn shake(w: &mut [f32], r: &mut Rng, size: f32) {
    let k = size * 3f32.sqrt() / 65536.0;
    for x in w {
        let u = r.u64();
        let s = (u & 0xffff) + (u >> 16 & 0xffff) + (u >> 32 & 0xffff) + (u >> 48);
        *x += k * (s as f32 - 131070.0);
    }
}

pub fn copy(w: &mut World, slot: usize, b: &mut Body) -> bool {
    let lay = Layout::of(&w.chem);
    let (mass, n, mc) = (b.mass(&w.chem), b.n(), b.dec(body::COPY_MASS));
    if mass < mc * n {
        return false;
    }
    let id = b.id(slot);
    let mut r = Rng::new(w.seed, w.day, id, Purpose::Mutate);
    let frac = b.gene(body::CHILD).clamp(0.05, 0.5);
    // A member past `phys.full` times its copy mass gives all it grew past that to children of the
    // size it would give at that mass: share `s` of its stores, `k` children.
    let (m1, full) = (mass / n, w.cfg.phys.full * mc);
    let (s, k) = if m1 > full { (1.0 - (1.0 - frac) * full / m1, (m1 / full - 1.0 + frac) / frac) } else { (frac, 1.0) };
    // A node's members copy as far as its reactions can release the cost now (a share of them);
    // an individual all or nothing.
    let (cost, days) = (w.cfg.phys.c_copy * s * mass, b.interval() as f64);
    let p = if n > 1.0 { (0.999 * body::release(w, b, days) / cost).min(1.0) } else { 1.0 };
    if p <= 0.0 || !body::pay(w, b, cost * p, days) {
        return false;
    }
    // An individual's children are whole.
    let (i, births) = (b.cell as usize, if n > 1.0 { n * k * p } else { k.round() });
    let cat = body::light_cat(&w.chem, &b.genes);
    w.events.push(Event { day: w.day, kind: EventKind::Born { cat }, node: i as u32, value: births });
    // Children that start in a neighbour cell (off the map: none): of a node, a share (none if
    // less than one member); an individual's go together, by that chance.
    let (node, d) = (n > 1.0, b.gene(body::DISPERSE).clamp(0.0, 1.0));
    let m = if node { if d * births >= 1.0 { d * births } else { 0.0 } } else { (r.f64() < d) as u8 as f64 };
    let to = if m > 0.0 { w.grid.step(i, DIRS[r.below(8)]) } else { None };
    let (cell, m) = match to {
        _ if !node => (to.unwrap_or(i), births),
        Some(j) => (j, m),
        None => (i, 0.0),
    };
    if node {
        b.count += births - m;
        widen(&lay, &w.cfg.copy_phys(), b, ((births - m) / b.n()) as f32);
        if m == 0.0 {
            return true;
        }
    }
    let share = s * m / births;
    let (genes, spread) = vary(&lay, &b.genes, &b.spread, m, w.cfg.phys.alike, &mut r);
    let mut c = Body {
        cell: cell as u32,
        count: m,
        lineage: b.lineage,
        parent: id,
        born: w.day,
        mark: w.light[cell],
        e: b.e * share,
        store: b.store.iter().map(|a| a * share).collect(),
        genes,
        spread,
        genome: b.genome,
        group: b.group,
        near: u32::MAX,
        ..Default::default()
    };
    b.e -= c.e;
    for (a, x) in b.store.iter_mut().zip(&c.store) {
        *a -= x;
    }
    // Born rows: a share `row_chance` of children change theirs by a step (bigger by 1 /
    // sqrt(row_chance)), else share their parent's. Children drawn from a node whose rows differ,
    // or a group of them, get the mean of the group drawn (noise of that variance and the
    // generation's change, over `m`) and keep the variance left among them.
    let (size, mm) = (10f32.powf(b.genes[body::MUT]), m.max(1.0) as f32);
    let p = w.cfg.copy_phys();
    let (var, changed) = (b.row_spread + (p.row_noise * size).powi(2), r.f64() < p.row_chance);
    let sd = if w.cfg.null { 0.0 } else if b.row_spread > 0.0 || mm > 1.0 { (var / mm).sqrt() } else { changed as u8 as f32 * p.row_noise * size / (p.row_chance as f32).sqrt() };
    c.row_spread = var * (1.0 - 1.0 / mm);
    let mut rows = (sd > 0.0).then(|| {
        let mut v = w.life.born(b).to_vec();
        shake(&mut v, &mut r, sd);
        v
    });
    // A node's children start no new lineage: a change of the brain's base in one child is lost in
    // the mean of many (the laws in average form, plan 7.1); a node whose mean sizes moved moves
    // itself (`resize`).
    if let Some(lin) = w.life.brains[b.lineage as usize].as_ref().filter(|_| !node) {
        let fresh = if let Some(l) = resized(lin, &c.genes, w.chem.ns(), &mut r, w.cfg.phys.new_noise) {
            rows = Some(brain::rerow(rows.as_deref().unwrap_or(w.life.born(b)), lin.shape, l.shape, &mut r, w.cfg.phys.new_noise));
            Some(l)
        } else if r.f64() < w.cfg.phys.new_lineage {
            let mut l = lin.clone();
            shake(&mut l.base, &mut r, p.base_noise);
            Some(l)
        } else {
            None
        };
        if let Some(l) = fresh {
            c.lineage = w.life.new_lineage(l);
            c.group = w.life.new_group(c.lineage, rows.clone().unwrap_or_else(|| w.life.born(b).to_vec()));
        }
    }
    // Children (of a node or of an individual, one or many) join a node there alike in genes
    // with a brain of the same sizes (of any lineage: being of the same one is no reason).
    let shape = |l: u32| w.life.brains[l as usize].as_ref().map(|x| (x.shape, x.seen, x.sight));
    let joins = |o: &Body| o.alive && o.n() > 1.0 && shape(o.lineage) == shape(c.lineage) && alike(&lay, w.cfg.phys.alike, &o.genes, &c.genes);
    if c.lineage == b.lineage {
        if let Some(t) = w.life.at[cell].iter().map(|&s| s as usize).find(|&s| joins(&w.life.bodies[s])) {
            let rows = rows.unwrap_or_else(|| w.life.born(b).to_vec());
            join(w, t, c, &rows);
            return true;
        }
    }
    if let Some(v) = rows {
        c.genome = w.life.new_genome(v);
    }
    own(w, b, &mut c);
    (c.wet, c.vis) = (c.key_liquids(&w.chem), c.visible(&w.chem));
    let delay = c.interval();
    let s = w.life.add(c, w.day, delay);
    w.set_keep(s, w.life.bodies[s].gen, w.keep_of(slot, b.gen));
    true
}

/// Without shared brains a body born or split from `b` starts a group of its own: its born rows
/// plus what `b`'s group learned.
fn own(w: &mut World, b: &Body, c: &mut Body) {
    if !w.cfg.share_brains && c.group == b.group {
        let gr = &w.life.groups[b.group as usize];
        let rows = w.life.born(c).iter().zip(gr.learned()).map(|(r, l)| r + l).collect();
        let (var, ms) = (gr.var, gr.ms.clone());
        c.group = w.life.new_group(c.lineage, rows);
        (w.life.groups[c.group as usize].var, w.life.groups[c.group as usize].ms) = (var, ms);
    }
}

/// A lineage with the sizes of brain and senses that `genes` ask for, if one of them is a whole
/// unit away from lineage `lin`'s (the others kept), its base carried over where the sizes
/// overlap (`brain::rebase`, new weights of spread `noise`).
fn resized(lin: &Lineage, genes: &[f32], ns: usize, r: &mut Rng, noise: f32) -> Option<Lineage> {
    let (old, want) = (choose::sizes_of(lin), choose::sizes(genes));
    let gene = [body::HIDDEN, body::MEMORY, body::SEEN, body::SIGHT].map(|g| genes[g]);
    let new: [usize; 4] = std::array::from_fn(|k| if (gene[k] - old[k] as f32).abs() >= 1.0 { want[k] } else { old[k] });
    let to = choose::shape(ns, new[0], new[1]);
    (new != old).then(|| Lineage { shape: to, seen: new[2], sight: new[3], base: brain::rebase(&lin.base, lin.shape, to, r, noise) })
}

/// At its update, a body whose brain-size genes (a node's: their mean) are a whole unit away from
/// its lineage's sizes moves itself to a new lineage with those sizes, its brain carried over
/// (`resized`; its born rows by `brain::rerow`), in a new group of its own whose rows are its
/// group's shared rows carried over with the same new weights (so what it learned goes with it).
/// An individual's child does this at birth (`copy`); a node's children start no lineage, so
/// without this a node's size genes drifted but did nothing (review 3, M4). Nodes join and merge
/// with others only of the same brain sizes (`copy`), or carry their rows over to the larger's
/// sizes (`merge`). Moves no matter and no energy.
pub fn resize(w: &mut World, slot: usize, b: &mut Body) {
    let (ns, noise, id) = (w.chem.ns(), w.cfg.phys.new_noise, b.id(slot));
    let mut r = Rng::new(w.seed, w.day, id, Purpose::Body);
    let Some(lin) = w.life.brains[b.lineage as usize].as_ref() else { return };
    let Some(l) = resized(lin, &b.genes, ns, &mut r, noise) else { return };
    // The born and the shared rows get the same new weights (two streams of one key, the body's
    // id turned over).
    let rerow = |x: &[f32]| brain::rerow(x, lin.shape, l.shape, &mut Rng::new(w.seed, w.day, !id, Purpose::Body), noise);
    let gr = &w.life.groups[b.group as usize];
    let (born, shared, var, ms) = (rerow(w.life.born(b)), rerow(&gr.rows), gr.var, gr.ms.clone());
    let lineage = w.life.new_lineage(l);
    let group = w.life.new_group(lineage, shared);
    (w.life.groups[group as usize].var, w.life.groups[group as usize].ms) = (var, ms);
    w.life.move_to(b, lineage, group, born);
}

/// The body in slot `s` joins the body in slot `t` (`join`), its born rows carried over to the
/// sizes of the other's brain where they differ.
pub fn merge(w: &mut World, s: usize, t: usize) {
    let (life, c) = (&w.life, w.life.bodies[s].clone());
    let shape = |b: &Body| life.brains[b.lineage as usize].as_ref().map(|l| l.shape);
    let rows = match (shape(&c), shape(&life.bodies[t])) {
        (Some(a), Some(b)) if a != b => brain::rerow(life.born(&c), a, b, &mut Rng::new(w.seed, w.day, c.id(s), Purpose::Mutate), w.cfg.phys.new_noise),
        _ => life.born(&c).to_vec(),
    };
    w.life.remove(s);
    join(w, t, c, &rows);
}

/// Members `c` (not placed, born with `rows`, in the shape of `t`'s brain) join body `t`:
/// stores and heat add up; genes and born rows become the means by count, and the spread what
/// both held around the new mean (each one's spread plus its gap from the new mean squared, by
/// count; for the rows the gap squared is the mean over rows; up to `most`), so the smaller one's
/// genes stay in the spread, not only in the mean.
pub fn join(w: &mut World, t: usize, c: Body, rows: &[f32]) {
    let mut o = std::mem::take(&mut w.life.bodies[t]);
    take_in(w, &mut o, c, rows);
    w.life.bodies[t] = o;
}

/// `join` into body `o`, taken out of its slot.
pub fn take_in(w: &mut World, o: &mut Body, c: Body, rows: &[f32]) {
    let (lay, p, a) = (Layout::of(&w.chem), &w.cfg.phys, (c.n() / (o.n() + c.n())) as f32);
    let mix = |x: &[f32], y: &[f32]| x.iter().zip(y).map(|(p, q)| p + a * (q - p)).collect::<Vec<f32>>();
    let pool = |v: f32, u: f32, d2: f32, step: f32| ((1.0 - a) * v + a * u + a * (1.0 - a) * d2).min(most(step, p.alike));
    let s = |b: &Body, g: usize| b.spread.get(g).copied().unwrap_or(0.0);
    o.spread = (0..o.genes.len()).map(|g| pool(s(o, g), s(&c, g), (c.genes[g] - o.genes[g]).powi(2), range(&lay, g).0)).collect();
    let born = w.life.born(o);
    let gap = born.iter().zip(rows).map(|(p, q)| (q - p) * (q - p)).sum::<f32>() / born.len().max(1) as f32;
    o.row_spread = pool(o.row_spread, c.row_spread, gap, row_step(p, 10f32.powf(o.genes[body::MUT])));
    let (genes, born) = (mix(&o.genes, &c.genes), mix(born, rows));
    w.life.reborn(o, genes, born);
    o.store.iter_mut().zip(&c.store).for_each(|(s, x)| *s += x);
    (o.e, o.count) = (o.e + c.e, o.n() + c.n());
    o.vis = o.visible(&w.chem);
}

/// Share `f` of node `b`'s members as a new body (not placed): that share of each store and of
/// the heat; lineage, group, genome, memory as the node's; holding nothing, inside nothing.
fn part(b: &mut Body, f: f64) -> Body {
    let mut c = Body { count: f * b.count, e: f * b.e, trace: Vec::new(), k: body::Consts::default(), hold: 0, inside: 0, ..b.clone() };
    for (a, x) in b.store.iter_mut().zip(c.store.iter_mut()) {
        *x = f * *a;
        *a -= *x;
    }
    (b.e, b.count) = (b.e - c.e, b.count - c.count);
    c
}

/// At its update, a node whose members differ so much that its two halves would not be alike
/// splits in two (plan 7.1). The halves of a normal spread have means `d = sqrt(2/π)` standard
/// deviations from the mean, so it splits when the halves' gap passes `phys.alike` steps along
/// the gene where it is widest, as `step` counts it (so not along a gene that is off in both
/// halves, nor the brain's sizes), or, with `cfg.split_rows`, of the born rows in row steps (the
/// step of a changed child); among genes as wide (a spread held at `most` is as wide in every
/// gene), a keyed random pick. `d` is shrunk to the room left to the gene's walls, so the mean is
/// kept exactly (clamped halves moved it away from the wall, review 3, M1). Each half has half the
/// members, stores and heat; their means are the mean ± `d`, and the variance along it falls by
/// `d²`, so the spread of both together is kept.
/// The rows, whose spread is one number for all, split all at once along a random direction of
/// signs; the new half gets a genome of its own. Selection then acts between the halves until
/// the next month's merge (`body::crowd`): the one that fits grows, the other shrinks. Halves of
/// a gene are not alike, so the merge leaves them apart (it merges only alike nodes) until they
/// grow alike again; halves of the rows are alike (likeness is in genes), so it takes them back,
/// the mean moved toward the one that grew, and the node splits again while its rows still differ
/// that much: a monthly round of selection. The cell's count does not block a split (review 3,
/// M3): halves of neutral genes are pulled back together by the children that come and go
/// between neighbours and merge, so the count stays small (a test with every spread held at
/// `most`: 4 to 10 nodes a cell).
pub fn split(w: &mut World, slot: usize, b: &mut Body) {
    if b.count < 2.0 || b.spread.is_empty() && b.row_spread <= 0.0 {
        return;
    }
    let (lay, p) = (Layout::of(&w.chem), &w.cfg.phys);
    let row = row_step(p, 10f32.powf(b.genes[body::MUT]));
    // Per gene the halves' shift from the mean (within the walls) and their gap in steps.
    let half = |g: usize| {
        let ((_, lo, hi), m) = (range(&lay, g), b.genes[g]);
        let d = (FRAC_2_PI * b.spread[g].max(0.0)).sqrt().min(m - lo).min(hi - m).max(0.0);
        (d, step(&lay, g, m + d) - step(&lay, g, m - d))
    };
    let mut gap: Vec<(usize, f32)> = (0..b.spread.len()).map(|g| (g, half(g).1)).collect();
    gap.push((usize::MAX, if w.cfg.split_rows { 2.0 * (FRAC_2_PI * b.row_spread).sqrt() / row } else { 0.0 }));
    let top = gap.iter().fold(0.0f32, |a, x| a.max(x.1));
    if top <= p.alike {
        return;
    }
    // As wide but for rounding (a spread held at `most` is the same in steps in every gene).
    gap.retain(|x| x.1 >= top * (1.0 - 1e-4));
    let mut r = Rng::new(w.seed, w.day, b.id(slot), Purpose::Split);
    let g = gap[r.below(gap.len())].0;
    let d = if g == usize::MAX { (FRAC_2_PI * b.row_spread).sqrt() } else { half(g).0 };
    let mut c = part(b, 0.5);
    let (mut genes, mut rows) = (b.genes.clone(), w.life.born(b).to_vec());
    if g == usize::MAX {
        let mut bits = 0;
        let mut theirs = rows.clone();
        for (k, (x, y)) in rows.iter_mut().zip(theirs.iter_mut()).enumerate() {
            bits = if k % 64 == 0 { r.u64() } else { bits >> 1 };
            let s = if bits & 1 == 1 { d } else { -d };
            (*x, *y) = (*x + s, *y - s);
        }
        c.genome = w.life.new_genome(theirs);
        b.row_spread *= 1.0 - FRAC_2_PI;
        c.row_spread = b.row_spread;
    } else {
        // Which half keeps the slot (and so is first in the order of updates) is drawn.
        let d = if r.below(2) == 0 { d } else { -d };
        (genes[g], c.genes[g]) = (genes[g] + d, genes[g] - d);
        b.spread[g] = (b.spread[g] - d * d).max(0.0);
        c.spread[g] = b.spread[g];
    }
    w.life.reborn(b, genes, rows);
    own(w, b, &mut c);
    w.diag.splits[0] += 1.0;
    let delay = c.interval();
    let s = w.life.add(c, w.day, delay);
    w.set_keep(s, w.life.bodies[s].gen, w.keep_of(slot, b.gen));
}

/// One member of node `b` (at least two) split off as an individual, to do what only a single
/// body can (plan 7.1): its genes the node's plus noise of their spread (shrunk near a wall, so
/// it is never clamped, as in `vary`), a share 1/count of its stores and heat (`part`). The
/// node's genes and spread become those of the others (so the member going back, `take_in`,
/// gives them back). Placed; returns its slot.
pub fn one(w: &mut World, b: &mut Body, r: &mut Rng) -> usize {
    let (lay, n) = (Layout::of(&w.chem), b.count as f32);
    let (mut c, mut genes) = (part(b, 1.0 / b.count), b.genes.clone());
    for (g, x) in c.genes.iter_mut().enumerate() {
        let ((_, lo, hi), v) = (range(&lay, g), b.spread.get(g).map_or(0.0, |s| s.max(0.0)));
        *x += v.sqrt().min((*x - lo).min(hi - *x).max(0.0) / REACH) * noise(r);
        let d = *x - genes[g];
        genes[g] = (genes[g] - d / (n - 1.0)).clamp(lo, hi);
        if let Some(s) = b.spread.get_mut(g) {
            *s = ((n * v - d * d * n / (n - 1.0)) / (n - 1.0)).max(0.0);
        }
    }
    let rows = w.life.born(b).to_vec();
    w.life.reborn(b, genes, rows);
    (c.count, c.spread, c.row_spread) = (1.0, Vec::new(), 0.0);
    own(w, b, &mut c);
    w.diag.splits[1] += 1.0;
    let delay = c.interval();
    w.life.add(c, w.day, delay)
}
