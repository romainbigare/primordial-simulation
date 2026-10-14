//! Law 4: Copy. A body whose members have enough mass pays the work of copying into its child
//! store as far as its reactions can release it now, and makes the children it covers, a child
//! per member: a share of each of its stores and of its heat, and its genes plus small noise
//! (now and then a change of the brain rows it was born with, else it shares its parent's); it
//! starts in its parent's brain group (or, if brains are not shared, in a group of its own with
//! what its parent learned). A child whose brain-size genes are a whole unit away from its
//! lineage's sizes starts a new lineage (and group) with the new sizes, its brain carried over
//! where the sizes overlap; rarely the base of its lineage's brain is copied with small noise,
//! which starts a new lineage too. A member past `phys.full` times its copy mass has as many
//! children as it grew past that, at its update whatever its brain does (`body::live`).
//!
//! A population node (count > 1) keeps how its members differ (plan 7.1): its spread, the
//! variance of their genes around its genes, per gene, and of their born rows around its rows
//! (one number for all rows). Its children that stay are new members of it and widen the spread
//! by their share of a generation's change (`widen`). A share of them (the `DISPERSE` gene; none
//! if less than one member) starts in a random neighbour cell, drawn from the node: their mean
//! genes and rows are the node's plus noise of its spread and the change over their number, and
//! they keep the variance left among them (`vary`). There they join the first node they belong
//! to, alike in genes within its spread and theirs (`within`), whatever its lineage and brain
//! sizes (means by count, the spread pooled, `join`), or start a node. An individual's children
//! stay together, in a neighbour cell with that chance (else in its own); there they join a node
//! as a node's do, else they are a new body. Only individuals' children start new lineages; a node whose mean brain-size genes
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
        body::COPY_MASS => (0.2, -3.0, 6.0),
        body::INTERVAL => (0.2, 0.7, 2.556),
        body::MUT => (0.2, -3.0, 0.0),
        body::LEARN => (0.2, -4.0, 0.0),
        body::FORGET => (0.2, -4.0, -1.7),
        // Down to no brain (review 3, Q2): below 0.5 it rounds to 0 hidden units; the room below 0
        // lets a node's mean reach it without meeting the wall.
        body::HIDDEN => (0.3, -1.0, 64.0),
        body::MEMORY => (0.15, 0.0, 12.0),
        body::SEEN => (0.1, 1.0, 6.0),
        body::SIGHT => (0.07, 1.0, 4.0),
        body::DISPERSE => (0.05, 0.0, 1.0),
        body::DENATURE => (5.0, 250.0, 450.0),
        body::REPAIR => (0.2, -3.0, 3.0),
        g if g < lay.min => (0.3, -6.0, 18.0),
        g if g < lay.upt => (0.05, -1.0, 0.9),
        g if g < lay.keep => (0.2, -4.0, 2.0),
        _ => (0.2, -0.5, 2.0),
    }
}

/// Genes no more than `limit` mutation steps apart in each gene (genes that are off count as 0;
/// the sizes of brain and senses are not compared, as `merge` carries a brain over to other sizes).
/// Nodes merge each month only if their means are alike (`body::crowd`), as a split's halves are
/// tested (`split`): so the halves stay apart for selection until they grow alike again.
pub fn alike(lay: &Layout, limit: f32, a: &[f32], b: &[f32]) -> bool {
    a.iter().zip(b).enumerate().all(|(g, (&x, &y))| (step(lay, g, x) - step(lay, g, y)).abs() <= limit)
}

/// Standard deviations of spread (of a node and of the children joining it) that the means of
/// children may lie from a node's beyond `phys.alike` steps and still belong to it (`within`).
/// Two: a lone child of a node held at the split threshold (2.7 steps) is then within its node in
/// a gene but for a chance of 1 in 1,000 (4 + 2 x 2.7 steps against its 2.9), so it nearly always
/// joins it whatever the number of genes; with `alike` alone the chance was 1 in 6 per gene, and
/// with about a hundred genes nearly every small group started a node of its own (review 3, M3).
/// A larger factor would let children join a node of another kind as wide.
pub const WITHIN: f32 = 2.0;

/// Children `c` (one or a group drawn from a node, with the spread left among them) belong to
/// node `o`: in each gene their mean is within `limit` steps of its mean plus `WITHIN` standard
/// deviations of both spreads together (each gene alone, so kinds that differ in a few genes
/// stay apart). A node's children differ from its mean by its spread over their number, so
/// with `alike` alone a group of a few rarely joined even the node it was drawn from.
fn within(lay: &Layout, limit: f32, o: &Body, c: &Body) -> bool {
    let s = |b: &Body, g: usize| b.spread.get(g).map_or(0.0, |&v| v.max(0.0));
    o.genes.iter().zip(&c.genes).enumerate().all(|(g, (&x, &y))| (step(lay, g, x) - step(lay, g, y)).abs() <= limit + WITHIN * (s(o, g) + s(c, g)).sqrt() / range(lay, g).0)
}

/// A gene in mutation steps (off as 0; the sizes of brain and senses as 0): two bodies are as
/// alike as their genes are near in these (`alike`, `body::crowd`). A split measures the sizes in
/// their steps (`split`), so selection can move them in a node.
fn step(lay: &Layout, g: usize, x: f32) -> f32 {
    if (body::HIDDEN..=body::SIGHT).contains(&g) { 0.0 } else { (if g < lay.cat { x } else { x.max(0.0) }) / range(lay, g).0 }
}
pub fn steps(lay: &Layout, a: &[f32]) -> Vec<f32> {
    a.iter().enumerate().map(|(g, &x)| step(lay, g, x)).collect()
}
/// A body's spread per gene in mutation steps squared (none for the sizes of brain and senses).
pub fn spread_steps(lay: &Layout, b: &Body) -> Vec<f32> {
    (0..b.genes.len()).map(|g| if (body::HIDDEN..=body::SIGHT).contains(&g) { 0.0 } else { b.spread.get(g).map_or(0.0, |&v| v.max(0.0)) / range(lay, g).0.powi(2) }).collect()
}

/// Parent genes plus noise, kept in range.
pub fn mutate(lay: &Layout, genes: &[f32], r: &mut Rng) -> Vec<f32> {
    vary(lay, genes, &[], 1.0, 0.0, r).0
}

/// Noise near a standard normal: a sum of four 16-bit uniforms from one draw, centred and scaled
/// (within `REACH`), as `shake` draws (four draws of 53 bits each cost a node's dispersing children
/// as much as the rest of their genes).
fn noise(r: &mut Rng) -> f32 {
    let u = r.u64();
    (((u & 0xffff) + (u >> 16 & 0xffff) + (u >> 32 & 0xffff) + (u >> 48)) as f32 - 131070.0) * (3f32.sqrt() / 65536.0)
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
    let (size, m, mut left) = (10f32.powf(genes[body::MUT]), m.max(1.0) as f32, Vec::with_capacity(genes.len()));
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

/// Add noise near a normal of spread `size` to each weight: a sum of four 8-bit uniforms, centred
/// and scaled (steps of 0.7% of `size`), two weights from one draw. Cheap, as a brain has many
/// weights and a node's dispersing children shake them all (it was a third of their cost).
pub fn shake(w: &mut [f32], r: &mut Rng, size: f32) {
    const B: u64 = 0x00ff_00ff_00ff_00ff;
    let k = size * 3f32.sqrt() / 256.0;
    // Bytes summed in pairs (16-bit lanes), then lanes in pairs: one sum per 32 bits.
    let sums = |u: u64| {
        let a = (u & B) + (u >> 8 & B);
        [((a & 0xffff) + (a >> 16 & 0xffff)) as f32 - 510.0, ((a >> 32 & 0xffff) + (a >> 48)) as f32 - 510.0]
    };
    let mut pairs = w.chunks_exact_mut(2);
    for p in &mut pairs {
        let s = sums(r.u64());
        (p[0], p[1]) = (p[0] + k * s[0], p[1] + k * s[1]);
    }
    if let [x] = pairs.into_remainder() {
        *x += k * sums(r.u64())[0];
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
    // The work of making them, `c_copy` per kg given, is paid into the child store (`Body::paid`)
    // as far as its reactions can release it now (review 3, P6): copying is a process, so a body
    // need not release a child's whole cost in one interval, nor a member far past `full` all
    // its children's at once (all or nothing, it could never pay and grew without end, M2). Past
    // `full` it must (physics, `body::live`); before, by its brain's choice, it pays only what it
    // can spare: its fuel `f` (what its reactions could release from its stores) as far as, less
    // what it pays, it can still release its upkeep `u` over an interval at today's rate `r`
    // (first order in the fuel): (f - paid) r / f >= u. (Sparing past `full` too, a body short of
    // fuel for its size never copied, and grew to 10^5 kg.) It has the children the store covers:
    // a node a share of them, an individual whole ones; the rest later. The work is for what the
    // children are built of: the liquid they carry is only moved (counted, a member of mostly
    // liquid could not pay for a child, never copied and swelled to 30 times its copy mass).
    let t = b.temp(&w.chem);
    let wet: f64 = (0..w.chem.ns()).filter(|&k| w.chem.phase(k, t) == crate::chem::Phase::Liquid).map(|k| b.store[k] * w.chem.subs[k].molar_mass).sum();
    let (owe, days) = (w.cfg.phys.c_copy * s * (mass - wet).max(0.0), b.interval() as f64);
    let more = if owe > b.paid {
        let r = 0.999 * body::release(w, b, days);
        let spare = if m1 >= full { r } else { b.fuel(&w.chem, b.temp(&w.chem)) * (1.0 - body::upkeep(w, b, days, mass) / r.max(1e-300)) };
        (owe - b.paid).min(spare).min(r)
    } else {
        0.0
    };
    if more > 0.0 && body::pay(w, b, more, days) {
        b.paid += more;
    }
    let p = if owe > 0.0 { (b.paid / owe).min(1.0) } else { 1.0 };
    let (i, births) = (b.cell as usize, if n > 1.0 { n * k * p } else { (k * p + 1e-9).floor() });
    if births <= 0.0 {
        return false;
    }
    b.paid = (b.paid - owe * births / (n * k)).max(0.0);
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
    // Each child gets share `s / k` of a member's stores (a node's children were 1 / p too big
    // when it paid for a share p of them).
    let share = s * m / (n * k);
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
    // Children (of a node or of an individual, one or many) join the first node there they
    // belong to (`within`; of any lineage: being of the same one is no reason), their rows carried
    // over to its brain's sizes where they differ, as the month's merge would (`merge`; joining
    // only a node of the same sizes, groups that met only nodes of other sizes started half the
    // new nodes of one 128x128 world at year 100). Children that started a new lineage stay
    // apart.
    let shape = |l: u32| w.life.brains[l as usize].as_ref().map(|x| x.shape);
    if c.lineage == b.lineage {
        if let Some(t) = w.life.at[cell].iter().map(|&s| s as usize).find(|&s| w.life.bodies[s].alive && w.life.bodies[s].n() > 1.0 && within(&lay, w.cfg.phys.alike, &w.life.bodies[s], &c)) {
            let rows = rows.unwrap_or_else(|| w.life.born(b).to_vec());
            let rows = match (shape(c.lineage), shape(w.life.bodies[t].lineage)) {
                (Some(x), Some(y)) if x != y => brain::rerow(&rows, x, y, &mut r, w.cfg.phys.new_noise),
                _ => rows,
            };
            join(w, t, c, &rows);
            return true;
        }
    }
    if let Some(v) = rows {
        c.genome = w.life.new_genome(v);
    }
    own(w, b, &mut c);
    (c.wet, c.vis) = (c.key_liquids(&w.chem), c.visible(&w.chem, w.cfg.phys.full));
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
    (new != old).then(|| Lineage { shape: to, seen: new[2], sight: new[3], base: brain::rebase(&lin.base, lin.shape, to, r, noise), root: lin.root })
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
    // With no brain it has no decision to learn from, no memory and no signal.
    if w.life.brainless(b) {
        (b.trace, b.mem, b.sig, b.wb) = (Vec::new(), Vec::new(), Default::default(), 0.0);
    }
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
/// stores, heat and child stores add up; genes and born rows become the means by count, and the spread what
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
    // A node has no wear: its members are of all ages (`body::live`).
    (o.e, o.count, o.paid, o.wear) = (o.e + c.e, o.n() + c.n(), o.paid + c.paid, 0.0);
    o.vis = o.visible(&w.chem, w.cfg.phys.full);
}

/// Share `f` of node `b`'s members as a new body (not placed): that share of each store, of the
/// heat and of the child store; lineage, group, genome, memory as the node's; holding nothing, inside nothing.
fn part(b: &mut Body, f: f64) -> Body {
    let mut c = Body { count: f * b.count, e: f * b.e, paid: f * b.paid, trace: Vec::new(), k: body::Consts::default(), hold: 0, inside: 0, ..b.clone() };
    for (a, x) in b.store.iter_mut().zip(c.store.iter_mut()) {
        *x = f * *a;
        *a -= *x;
    }
    (b.e, b.count, b.paid) = (b.e - c.e, b.count - c.count, b.paid - c.paid);
    c
}

/// At its update, a node whose members differ so much that its two halves would not be alike
/// splits in two (plan 7.1). The halves of a normal spread have means `d = sqrt(2/π)` standard
/// deviations from the mean, so it splits when the halves' gap passes `phys.alike` steps along
/// the gene where it is widest, in its steps (so not along a gene that is off in both halves; the
/// brain's sizes count here, though not in likeness), or, with `cfg.split_rows`, of the born rows in row steps (the
/// step of a changed child); among genes as wide (a spread held at `most` is as wide in every
/// gene), a keyed random pick. `d` is shrunk to the room left to the gene's walls, so the mean is
/// kept exactly (clamped halves moved it away from the wall, review 3, M1). Each half has half the
/// members, stores and heat; their means are the mean ± `d`, and the variance along it falls by
/// `d²`, so the spread of both together is kept.
/// The rows, whose spread is one number for all, split all at once along a random direction of
/// signs; the new half gets a genome of its own. Selection then acts between the halves until
/// the next month's merge (`body::crowd`): the one that fits grows, the other shrinks. Halves of
/// a gene are not alike, so the merge leaves them apart (it merges only nodes whose means are
/// alike, not within their spreads as children join, `within`: each half lies within the other's
/// spread, so a merge by spread would take them back every month) until they grow alike again;
/// halves of the rows are alike (likeness is in genes), so it takes them back,
/// the mean moved toward the one that grew, and the node splits again while its rows still differ
/// that much: a monthly round of selection. Halves of a brain size are alike too (likeness leaves
/// the sizes out, `alike`), so they go the same round, each paying for its own size (`Lineage::cost`
/// goes with the body's genes), and a mean that reaches a whole unit from its lineage's size moves
/// the node to a lineage of that size (`resize`). Before (review 4, finding 1) a node never split
/// along a size, nothing else selects a gene inside a node, and a merge only mixes means that all
/// started at 12: the size genes were frozen in nodes (`hidden` 12 for 300 years in every run), so
/// a brain could neither grow nor, in light-eaters, shrink to none. A node splits only while its cell keeps a free place
/// after it (fewer than `cfg.crowd` - 1 bodies): a node held at `most` in its neutral genes is
/// past the threshold at every update, and its halves (unlike, so never merged back) split again,
/// so with no such limit the nodes of a cell doubled at each update (one 64x64 world reached
/// about 146,000 in one cell). Blocking it no longer biases the genes (review 3, M1): the spread
/// stops at `most` and a split keeps the mean.
pub fn split(w: &mut World, slot: usize, b: &mut Body) {
    if b.count < 2.0 || b.spread.is_empty() && b.row_spread <= 0.0 || w.life.at[b.cell as usize].len() + 1 >= w.cfg.crowd {
        return;
    }
    let (lay, p) = (Layout::of(&w.chem), &w.cfg.phys);
    let row = row_step(p, 10f32.powf(b.genes[body::MUT]));
    // Per gene the halves' shift from the mean (within the walls) and their gap in steps (as
    // `step` counts them, with one look-up of the range: most updates do not split).
    let half = |g: usize| {
        let ((u, lo, hi), m) = (range(&lay, g), b.genes[g]);
        let d = (FRAC_2_PI * b.spread[g].max(0.0)).sqrt().min(m - lo).min(hi - m).max(0.0);
        let (x, y) = if g < lay.cat { (m + d, m - d) } else { ((m + d).max(0.0), (m - d).max(0.0)) };
        (d, x / u - y / u)
    };
    let rows = if w.cfg.split_rows { 2.0 * (FRAC_2_PI * b.row_spread).sqrt() / row } else { 0.0 };
    let top = (0..b.spread.len()).fold(0.0f32, |a, g| a.max(half(g).1)).max(rows);
    if top <= p.alike {
        return;
    }
    let mut gap: Vec<(usize, f32)> = (0..b.spread.len()).map(|g| (g, half(g).1)).chain([(usize::MAX, rows)]).collect();
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
/// gives them back). Its wear is drawn uniform in [0, 1): members who each live as long are of
/// all ages evenly, so members split off die at the rate the node's members die (`body::live`). Placed; returns its slot.
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
    (c.count, c.spread, c.row_spread, c.wear) = (1.0, Vec::new(), 0.0, r.f64());
    own(w, b, &mut c);
    w.diag.splits[1] += 1.0;
    let delay = c.interval();
    w.life.add(c, w.day, delay)
}
