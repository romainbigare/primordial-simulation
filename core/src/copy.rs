//! Law 4: Copy. Reproduction is a share of a surplus (14 Oct 2026, the decision to reproduce by surplus; before, a body
//! copied when its body reached twice its copy mass, giving a share of every store: no organism splits
//! in half at full size). A body is mature when its body (`Body::built` per member) is at or past its
//! copy mass, the size of maturity. A mature body splits what it builds in an update (`grow`) into the
//! share `ALLOC` for offspring, held as a brood (`Body::brood`, matter built and paid for, not yet
//! body), and the rest for its own growth; an immature one grows. Whenever the brood covers the matter
//! of a child, a child is made (`copy`): its body is 10^`CHILD` of its parent's copy mass (seeds and
//! eggs far smaller than their parents, calves near half), of matter from the brood, with provisions of
//! fuel and liquid in the proportion its parent's body holds them (the key liquid times 10^`WET`), its
//! share of the heat, and its genes plus small noise (now and then a change of the brain rows it was born
//! with, else it shares its parent's); it starts in its parent's brain group (or, if brains are not
//! shared, in a group of its own with what its parent learned). The parent keeps its body. A child whose
//! brain-size genes are a whole unit away from its lineage's sizes starts a new lineage (and group) with
//! the new sizes, its brain carried over where the sizes overlap; rarely the base of its lineage's brain
//! is copied with small noise, which starts a new lineage too. It happens at the body's update whatever
//! its brain does (`body::live`).
//!
//! Growth is building (14 Oct 2026, decision after watch report 7): a member
//! turns the energy-rich solid into body matter (`Chemistry::build`) and pays `c_copy` per kg of
//! matter made in work (`grow`), as far as its allowance goes (`c_build`), its reactions can spare
//! and its stores reach. Storing fuel costs no building work. A child costs its parent no further
//! work: the matter it is given was built and paid for as brood (there is no child store).
//!
//! A population node (count > 1) keeps how its members differ (plan 7.1): its spread, the
//! variance of their genes around its genes, per gene, and of their born rows around its rows
//! (one number for all rows). Its children never join it while it is mature (they are below their
//! copy mass, and a mature node's mean must stay at or past maturity for its members to reproduce): they
//! form a group of their own, with the variance left among them (`vary`), who join the first immature
//! node of their kind and size class where they start (`near`, `emit`), or start one; a young node joins
//! and merges with the adult only once mature (the monthly merge, `body::crowd`). A child is of another
//! size class than a node when its body differs from the node's mean member by more than `alike` steps
//! of the copy-mass gene (`alike_size`).
//! A share of a node's children (the `DISPERSE` gene; none
//! if less than one member) starts in a random neighbour cell, drawn from the node: their mean
//! genes and rows are the node's plus noise of its spread and the change over their number, and
//! they keep the variance left among them (`vary`). There they join the first node they belong
//! to, alike in genes within its spread and theirs (`within`; brain sizes within a few whole
//! units), whatever its lineage (means by count, the spread pooled, `join`), or start a node. An individual's children
//! stay together, in a neighbour cell with that chance (else in its own); there they join a node
//! as a node's do, else they are a new body. Only individuals' children start new lineages; a node whose mean brain-size genes
//! are a whole unit away from its lineage's sizes moves itself to a new lineage with them, its
//! brain and what its group learned carried over (`resize`, at its update).
//!
//! A node whose halves would no longer be alike splits in two (`split`), so selection acts
//! between them; a node does the actions of Bond through one member split off as an individual
//! (`one`, from `choose::act`), and `open` draws several members as individuals at once (the
//! close view of a node). Members, stores and heat are kept exactly in every split and join.
//!
//! With `Config::nodes` off the animals (bodies that are not light-eaters, `body::light_eater`) are
//! never nodes: an animal's children start as individuals and join nothing (`copy`), and none
//! splits (`split`); `body::crowd` leaves them alone. Light-eaters are as before.

use crate::body::{self, Body, Layout, Lineage};
use crate::{brain, choose};
use crate::event::{Event, EventKind};
use crate::grid::DIRS;
use crate::rng::{Purpose, Rng};
use crate::{Physics, World};
use std::f32::consts::{FRAC_2_PI, FRAC_PI_8};

/// Reach of the noise `noise` draws (a sum of four uniforms, centred and scaled): 2·sqrt(3).
const REACH: f32 = 3.464_101_6;

/// log10 of one half: the largest child, a share of its parent's copy mass (`body::CHILD`).
const LOG_HALF: f32 = -0.301_029_995_7;

/// Mutation step (times the body's mutation size) and range of a gene. Catalysts, safe minimums
/// and uptake are off below 0 (they cost nothing and do nothing there), and their ranges go 20
/// steps below 0, so an off gene wanders as far as the noise of a node's spread reaches (about 10
/// steps, `most`) without meeting a wall, whose clamp would push it up (review 3, M1). A catalyst
/// step is 0.3 decades, so nodes alike within `phys.alike` steps differ by a factor 16 at most.
fn range(lay: &Layout, g: usize) -> (f32, f32, f32) {
    match g {
        g if g < body::NSCALAR => SCALARS[g],
        g if g < lay.min => (0.3, -6.0, 18.0),
        g if g < lay.upt => (0.05, -1.0, 0.9),
        g if g < lay.keep => (0.2, -4.0, 2.0),
        _ => (0.2, -0.5, 6.0),
    }
}

/// `range` of the scalar genes as a table: it is asked a hundred times in each likeness test and
/// each draw of children, and a look-up is cheaper than the match.
const SCALARS: [(f32, f32, f32); body::NSCALAR] = {
    let (mut t, mut g) = ([(0.0, 0.0, 0.0); body::NSCALAR], 0);
    while g < body::NSCALAR {
        t[g] = scalar(g);
        g += 1;
    }
    t
};

const fn scalar(g: usize) -> (f32, f32, f32) {
    match g {
        body::GAS => (0.3, -1.0, 5.0),
        body::LIQ => (0.3, -4.0, 1.0),
        body::HEAT => (0.3, -2.5, 3.5),
        body::LEAF => (0.3, -4.0, 1.0),
        body::STRENGTH => (0.1, 0.0, 1.0),
        body::CHILD => (0.1, -6.0, LOG_HALF),
        body::WET => (0.05, -3.0, 0.5),
        body::ALLOC => (0.05, 0.0, 1.0),
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
        _ => (0.3, -6.0, 18.0),
    }
}

/// Genes no more than `limit` steps apart in each gene (`step`: genes that are off count as 0, the
/// sizes of brain and senses by whole units).
/// Nodes merge each month only if their means are alike (`body::crowd`), as a split's halves are
/// tested (`split`): so the halves stay apart for selection until they grow alike again.
pub fn alike(lay: &Layout, limit: f32, a: &[f32], b: &[f32]) -> bool {
    a.iter().zip(b).enumerate().all(|(g, (&x, &y))| (step(lay, g, x) - step(lay, g, y)).abs() <= limit)
}

/// Standard deviations of spread (of a node and of the children joining it) that the means of
/// children may lie from a node's beyond `phys.alike` steps and still belong to it (`within`).
/// A half (14 Oct 2026; two before). Copying is asexual: in individuals the genes of two kinds
/// never mix, so a node, which only saves time, must not mix them either. With two, a node and
/// a group of children both at the spread's cap (2.7 steps each) took anything within
/// 4 + 2 x sqrt(2 x 7.3) = 11.6 steps in every gene, and the monthly merge of a full cell
/// (`body::crowd`) the same: the eaters (strength 0.6) and the eaters of remains (strength 0, 6
/// steps away; copy mass 1.6 to 9 steps away) were one kind for the nodes, and the children of
/// the most numerous, smallest kind swamped the others' nodes (a 64x64 roster world, seed 1: the
/// eaters' copy mass 51 kg at year 26, 3 kg at year 30; at year 100 the four animal kinds had the
/// same gene means and copy masses of tens of grams). With a half the bound is 5.9 steps and the
/// kinds mostly stay apart: at year 60 the eaters held 51 to 55 kg on seeds 2, 3 and 4 (0.04 to
/// 51 kg with two), and 12.5 kg on seed 1, where the 111 placed eaters still took the eaters of
/// remains' genes in their first two years. A pair of children of a node held at the split
/// threshold still joins it 199 times in 200 (`tests/nodes.rs`). Kinds less than 6 steps apart in
/// every gene still blend. With `alike` alone (0) a lone child left its own node 1 time in 6 per
/// gene, the bodies of a 64x64 roster world tripled, the safeguard merge past twice `crowd` mixed
/// the kinds again and the eaters ended at 0.3 to 2 kg (review 3, M3; 14 Oct 2026).
pub const WITHIN: f32 = 0.5;

/// Children `c` (one or a group drawn from a node, with the spread left among them) belong to
/// node `o`: in each gene their mean is within `limit` steps of its mean plus `WITHIN` standard
/// deviations of both spreads together (each gene alone, so kinds that differ in a few genes
/// stay apart). A node's children differ from its mean by its spread over their number, so
/// with `alike` alone a group of a few rarely joined even the node it was drawn from.
fn within(lay: &Layout, limit: f32, o: &Body, c: &Body) -> bool {
    let s = |b: &Body, g: usize| b.spread.get(g).map_or(0.0, |&v| v.max(0.0));
    o.genes.iter().zip(&c.genes).enumerate().all(|(g, (&x, &y))| (step(lay, g, x) - step(lay, g, y)).abs() <= limit + WITHIN * (s(o, g) + s(c, g)).sqrt() / like(lay, g))
}

/// The step gene `g` is measured in for likeness: its mutation step, but a whole unit for the
/// sizes of brain and senses. Their mutation steps (0.07 to 0.3) would make a split's halves
/// (1.3 units apart) unlike, and not counting them at all let ordinary eaters' children join a
/// placed 48-unit kind and average its brain to 12 within a year (watch report 5, review 5): in
/// whole units a 48-unit brain and a 12-unit one are not one kind, the halves of a split are.
fn like(lay: &Layout, g: usize) -> f32 {
    if (body::HIDDEN..=body::SIGHT).contains(&g) { 1.0 } else { range(lay, g).0 }
}

/// A gene in steps of `like` (off as 0): two bodies are as alike as their genes are near in these
/// (`alike`, `within`, `body::crowd`). A split measures the sizes in their mutation steps
/// (`split`), so selection can move them in a node.
fn step(lay: &Layout, g: usize, x: f32) -> f32 {
    (if g < lay.cat { x } else { x.max(0.0) }) / like(lay, g)
}
pub fn steps(lay: &Layout, a: &[f32]) -> Vec<f32> {
    a.iter().enumerate().map(|(g, &x)| step(lay, g, x)).collect()
}
/// A body's spread per gene in steps of `like`, squared.
pub fn spread_steps(lay: &Layout, b: &Body) -> Vec<f32> {
    (0..b.genes.len()).map(|g| b.spread.get(g).map_or(0.0, |&v| v.max(0.0)) / like(lay, g).powi(2)).collect()
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
    // Bytes summed in pairs (16-bit lanes), then lanes in pairs: one sum per 32 bits (the first
    // and the third lane of `a + (a >> 16)`).
    let sums = |u: u64| {
        let a = (u & B) + (u >> 8 & B);
        let t = a + (a >> 16);
        [(t & 0xffff) as f32 - 510.0, (t >> 32 & 0xffff) as f32 - 510.0]
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

/// Growth is building body matter (14 Oct 2026, decision after watch report 7), and past the size
/// of maturity a share of it is offspring (14 Oct 2026, the decision to reproduce by surplus). A member makes
/// matter (`Chemistry::build`: 6 of the energy-rich solid make 1 of matter and 1 of liquid,
/// element-exact and thermoneutral in chemistry 0) and pays `c_copy` per kg of matter made, through
/// `body::pay` (the work leaves as heat), as far as: (1) its allowance, `c_build` x its `equiv` /
/// `m_ref` x the days of its interval x its activity (building takes time, review 5); (2) its spare:
/// after what is paid and what is converted leave its fuel, its reactions must still release its
/// upkeep over an interval at today's rate; (3) what its reactions can release now; (4) the stores it
/// converts, which must also cover the fuel the work burns. Storing fuel costs no building work. There
/// is no size at which it stops: a body below its copy mass (the size of maturity) puts all it builds
/// into its own growth, up to that size; of what it builds past it a share `ALLOC` goes into the
/// brood (`Body::brood`, matter held back for offspring, not yet body) and the rest into its own
/// growth, so with `ALLOC` 1 it stops at maturity and with less it grows all its life. The liquid that
/// goes with new matter comes by soaking (`live`). A kg of matter costs `c_copy` of work and 1.11 kg
/// of the solid converted (6 x 30 g for 162 g) besides, 1.75 kg of it with the fuel the work burns
/// (62% efficient, as growth in life). Returns whether it built.
pub fn grow(w: &mut World, b: &mut Body, days: f64) -> bool {
    let (chem, p) = (&w.chem, &w.cfg.phys);
    let Some(bl) = chem.build.as_ref() else { return false };
    let m = bl.outputs[0].0 as usize;
    // (A unit of growth makes `nu0` mol of matter; chemistry 0: 1.)
    let (n, nu0, mm) = (b.n(), bl.outputs[0].1 as f64, chem.subs[m].molar_mass);
    b.brood = b.brooded(chem);
    // The matter a body of the size of maturity is made of: the body is its matter over 1 - h (`Body::size`).
    let (_, h) = b.wet_h(chem);
    let full = b.dec(body::COPY_MASS) * n * (1.0 - h);
    let needs = if b.adult(chem) { 0.0 } else { (full - b.matter_kg(chem)).max(0.0) };
    let (ivl, t, mass) = (b.interval() as f64, b.temp(chem), b.mass(chem));
    // (Building is chemistry: it goes with the body's activity, `Body::activity`; a dry body hardly grows.)
    let room = p.c_build * b.supply(chem, p) / p.m_ref * days.min(ivl) * b.activity(chem, p);
    let r = 0.999 * body::release(w, b, ivl);
    if room <= 0.0 || r <= 0.0 {
        return false;
    }
    // The fuel it spares: what is paid and the solid converted leave its fuel, less the matter's own
    // fuel if it burns that too. Per kg of matter made, J.
    let (ins, outs) = (&bl.inputs, &bl.outputs);
    let per_kg = p.c_copy + b.build_fuel(chem, t) - b.heat_of(chem, m, t) / mm;
    let spare = b.fuel(chem, t) * (1.0 - body::upkeep(w, b, ivl, mass) / r);
    // The stores it converts must also cover the fuel the work burns (each input that burns), and
    // leave the safe minimum of each (a body is not built of its last fuel: below its minimum it dies).
    let lay = Layout::of(chem);
    let size = b.size(chem);
    let floors: Vec<f64> = ins.iter().map(|&(k, _)| b.gene(lay.min + k as usize).max(0.0) * size / chem.subs[k as usize].molar_mass).collect();
    let reach = ins.iter().zip(&floors).map(|(&(k, nu), f)| {
        let heat = b.heat_of(chem, k as usize, t);
        (b.store[k as usize] - f).max(0.0) / (nu as f64 / (mm * nu0) + if heat > 0.0 { p.c_copy / heat } else { 0.0 })
    });
    let kg = (room / p.c_copy).min(r / p.c_copy).min(spare / per_kg.max(1e-300)).min(reach.fold(f64::MAX, f64::min));
    if kg <= 1e-12 * full.max(1e-30) {
        return false;
    }
    let (ins, outs, dh, cost) = (ins.clone(), outs.clone(), chem.dh_build(), p.c_copy * kg);
    if !body::pay(w, b, cost, ivl) {
        return false;
    }
    // What was paid may have burnt some of the solid: convert what is left of it, to its minimum.
    let ext = ins.iter().zip(&floors).fold(kg / (mm * nu0), |e, (&(k, nu), f)| e.min((b.store[k as usize] - f).max(0.0) / nu as f64));
    for &(k, nu) in &ins {
        // (A store used up to its last mol is zero, not a rounding below it.)
        b.store[k as usize] = (b.store[k as usize] - nu as f64 * ext).max(0.0);
    }
    for &(k, nu) in &outs {
        b.store[k as usize] += nu as f64 * ext;
    }
    b.e -= dh * ext;
    // What it made: up to what its maturity lacks it is body, of the rest the share `ALLOC` is brood.
    let made = nu0 * ext * mm;
    b.brood += (made - made.min(needs)) * b.gene(body::ALLOC).clamp(0.0, 1.0) / mm;
    true
}

/// A mature body makes offspring from its brood (14 Oct 2026, the decision to reproduce by surplus; before, a
/// body copied when its body reached `full` times its copy mass, giving a share of every store). A
/// child has a body of 10^`CHILD` of its parent's copy mass: matter of 1 - h of that (`Body::size`),
/// taken from the brood, and provisions of every other store, in the proportion the parent's body
/// holds them to its matter (the key liquid times 10^`WET`, at most what the parent holds), with its
/// share of the heat. A child is made whenever the brood covers one: an individual has whole children,
/// a node (members who all do the same) fractional members, as many as the brood makes. The parent
/// keeps its body: what it gives is brood and provisions. A body that is not mature (`Body::adult`)
/// makes none (its brood waits). Children are always immature (below their parents' copy mass), so
/// they never join their parent node, which is mature: they join a node of their kind and size class
/// that is immature too, in their parent's cell or, if they disperse, a neighbour's, or start one
/// (`emit`), and a node's mean never falls under maturity for its own children (`near`).
pub fn copy(w: &mut World, slot: usize, b: &mut Body) -> bool {
    let (chem, n, mc) = (&w.chem, b.n(), b.dec(body::COPY_MASS));
    let Some(mi) = chem.matter() else { return false };
    if !b.adult(chem) {
        return false;
    }
    let mm = chem.subs[mi].molar_mass;
    let (_, h) = b.wet_h(chem);
    b.brood = b.brooded(chem);
    // A child's body (kg) and its matter (mol); the births the brood covers.
    let body = 10f64.powf(b.gene(body::CHILD).clamp(-6.0, LOG_HALF as f64)) * mc;
    let kid = Kid { body, matter: body * (1.0 - h) / mm };
    let k = b.brood / kid.matter;
    let node = n > 1.0;
    let births = if node { k } else { (k + 1e-9).floor() };
    if births <= 0.0 {
        return false;
    }
    let id = b.id(slot);
    let mut r = Rng::new(w.seed, w.day, id, Purpose::Mutate);
    let i = b.cell as usize;
    // Children that start in a neighbour cell (off the map: none): of a node, a share (none if
    // less than one member); an individual's go together, by that chance.
    let d = b.gene(body::DISPERSE).clamp(0.0, 1.0);
    // An animal with `Config::nodes` off: its children join no node and are all individuals.
    let alone = !w.cfg.nodes && !body::light_eater(&w.chem, &b.genes);
    let m = if node { if d * births >= 1.0 { d * births } else { 0.0 } } else { (r.f64() < d) as u8 as f64 };
    let to = if m > 0.0 { w.grid.step(i, DIRS[r.below(8)]) } else { None };
    let (cell, m) = match to {
        _ if !node => (to.unwrap_or(i), births),
        Some(j) => (j, m),
        None => (i, 0.0),
    };
    let mut made = false;
    if m > 0.0 {
        made |= emit(w, slot, b, cell, m, &kid, node, alone, id, &mut r);
    }
    // A node's children who stay: a group of their own (less than one member waits in the brood
    // unless there is a node of theirs to join).
    if node && births - m > 0.0 {
        made |= emit(w, slot, b, i, births - m, &kid, node, alone, id, &mut r);
    }
    made
}

/// A child's body, kg, and its matter, mol (its body is that over 1 - h).
struct Kid {
    body: f64,
    matter: f64,
}

/// The step of the copy-mass gene that sizes are alike in (`alike_size`, `near`).
pub fn size_step(lay: &Layout) -> f32 {
    like(lay, body::COPY_MASS)
}

/// A body's class for `near`: log10 of its member's body (`Body::built` per member), which size
/// classes compare, and whether it is mature. Taken once per body where it meets many (`body::crowd`).
pub fn class(chem: &crate::chem::Chemistry, b: &Body) -> (f64, bool) {
    let x = b.member_body(chem);
    (x.max(1e-30).log10(), b.mature(x))
}

/// Two member bodies (kg) are in one size class: within `alike` steps of the copy-mass gene of each
/// other, in log10 (0.2 decades a step: with `alike` 4 about a factor 6.3). A node holds members of
/// one size class: a child far smaller than its parent node's mean member starts a node of its own
/// (`copy`), and nodes join and merge only within a class (`near`, `body::crowd`), so young and adult
/// nodes stay apart until the young have grown.
pub fn alike_size(lay: &Layout, alike: f32, x: f64, y: f64) -> bool {
    alike_log(lay, alike, x.max(1e-30).log10(), y.max(1e-30).log10())
}

/// `alike_size` of two bodies given as log10 (`class`).
pub fn alike_log(lay: &Layout, alike: f32, x: f64, y: f64) -> bool {
    (x - y).abs() <= (alike * size_step(lay)) as f64 + 1e-9
}

/// Bodies `a` and `b` are in one class: alike in size (their members' bodies, `Body::built` per member,
/// `alike_size`) and in maturity (both mature, `Body::adult`, or both not: a node's mean must stay at or
/// past its copy mass for its members to reproduce, so the young join and merge with the young until
/// they have grown).
pub fn near(lay: &Layout, chem: &crate::chem::Chemistry, alike: f32, a: &Body, b: &Body) -> bool {
    let (x, y) = (class(chem, a), class(chem, b));
    x.1 == y.1 && alike_log(lay, alike, x.0, y.0)
}

/// `m` children of `b` (each as `kid` says: its matter taken from the brood, provisions of every other
/// store in the proportion the parent's body holds them to its matter, its share of the heat) start in
/// `cell`: they join a node there they belong to (in genes, in size and immature, as they are: `near`),
/// or start a body. Less than one member of a node's children that has no node to join waits in the
/// brood (nothing is made; false).
#[allow(clippy::too_many_arguments)]
fn emit(w: &mut World, slot: usize, b: &mut Body, cell: usize, m: f64, kid: &Kid, node: bool, alone: bool, id: u64, r: &mut Rng) -> bool {
    let lay = Layout::of(&w.chem);
    let (genes, spread) = vary(&lay, &b.genes, &b.spread, m, w.cfg.phys.alike, r);
    let mi = w.chem.matter().unwrap_or(0);
    // The matter of all m children, and their provisions as a share of the parent's body matter; of the
    // key liquid 10^`WET` times that (a child of a parent whose `WET` is below 0 is dry, and nearly dormant).
    let take = (m * kid.matter).min(b.store[mi]);
    let prov = take / (b.store[mi] - b.brooded(&w.chem)).max(1e-300);
    let wet_prov = prov * 10f64.powf(b.gene(body::WET));
    let mut c = Body {
        cell: cell as u32,
        count: m,
        lineage: b.lineage,
        parent: id,
        born: w.day,
        mark: w.light[cell],
        store: b.store.iter().enumerate().map(|(k, a)| if k == mi { take } else { a * if b.wet >> k & 1 == 1 { wet_prov } else { prov }.min(1.0) }).collect(),
        genes,
        spread,
        genome: b.genome,
        group: b.group,
        near: u32::MAX,
        wet: b.wet,
        ..Default::default()
    };
    // The node a child joins: of its size and immature (children are below their parents' copy
    // mass, and a mature node must keep its mean there), and of its genes (`within`, tested last:
    // it reads a hundred genes).
    let target = |w: &World, c: &Body| w.life.at[cell].iter().map(|&s| s as usize).find(|&s| {
        let t = &w.life.bodies[s];
        if !t.alive || t.n() <= 1.0 {
            return false;
        }
        let x = t.member_body(&w.chem);
        !t.mature(x) && alike_size(&lay, w.cfg.phys.alike, kid.body, x) && within(&lay, w.cfg.phys.alike, t, c)
    });
    // Nothing between here and the join changes the nodes of the cell or the child's genes, so a
    // search made now holds for the join.
    let mut hit = None;
    if node && m < 1.0 {
        hit = if alone { None } else { target(w, &c) };
        if hit.is_none() {
            return false;
        }
    }
    let (cat, root) = (body::light_cat(&w.chem, &b.genes), w.life.root(b));
    w.events.push(Event { day: w.day, kind: EventKind::Born { cat, root }, node: b.cell, value: m });
    // Its heat is that of its parent's temperature on the heat capacity it got, so it starts as warm
    // as its parent (a dry child would else be much hotter, a wet one colder); the parent keeps the rest.
    c.e = b.e * c.heat_cap(&w.chem) / b.heat_cap(&w.chem).max(1e-9);
    b.e -= c.e;
    for (a, x) in b.store.iter_mut().zip(&c.store) {
        *a -= x;
    }
    b.brood = (b.brood - take).max(0.0);
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
        shake(&mut v, r, sd);
        v
    });
    // A node's children start no new lineage: a change of the brain's base in one child is lost in
    // the mean of many (the laws in average form, plan 7.1); a node whose mean sizes moved moves
    // itself (`resize`).
    if let Some(lin) = w.life.brains[b.lineage as usize].as_ref().filter(|_| !node) {
        let fresh = if let Some(l) = resized(lin, &c.genes, w.chem.ns(), r, w.cfg.phys.new_noise) {
            rows = Some(brain::rerow(rows.as_deref().unwrap_or(w.life.born(b)), lin.shape, l.shape, r, w.cfg.phys.new_noise));
            Some(l)
        } else if r.f64() < w.cfg.phys.new_lineage {
            let mut l = lin.clone();
            shake(&mut l.base, r, p.base_noise);
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
    if c.lineage == b.lineage && !alone {
        if let Some(t) = hit.or_else(|| target(w, &c)) {
            let rows = rows.unwrap_or_else(|| w.life.born(b).to_vec());
            let rows = match (shape(c.lineage), shape(w.life.bodies[t].lineage)) {
                (Some(x), Some(y)) if x != y => brain::rerow(&rows, x, y, r, w.cfg.phys.new_noise),
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
    // Its key liquids first: `visible` reads the body (`Body::size`) through them. (Set together in
    // one tuple, the leaf of a newborn was that of its matter alone until its first update.)
    // (A parent that survived a freezing (`body::live`) is still wet in the sense of its key liquids: its
    // child's are kept though it is born frozen.)
    c.wet = c.key_liquids(&w.chem) | (b.wet & c.key_mask(&w.chem));
    c.vis = c.visible(&w.chem, &w.cfg.phys);
    let delay = c.interval();
    let s = w.life.add(c, w.day, delay);
    w.set_keep(s, w.life.bodies[s].gen, w.keep_of(slot, b.gen));
    // Several children together (a member far past `full` has more than one): drawn from the group
    // as individuals, newborn (wear 0), the last one the group's body itself.
    if alone && w.life.bodies[s].count > 1.5 {
        let k = w.life.bodies[s].count.round() as usize - 1;
        for m in open(w, s, k) {
            w.life.bodies[m].wear = 0.0;
        }
        let c = &mut w.life.bodies[s];
        (c.count, c.spread, c.row_spread) = (1.0, Vec::new(), 0.0);
    }
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
/// stores and heat add up; genes and born rows become the means by count, and the spread what
/// both held around the new mean (each one's spread plus its gap from the new mean squared, by
/// count; for the rows the gap squared is the mean over rows; up to `most`), so the smaller one's
/// genes stay in the spread, not only in the mean.
pub fn join(w: &mut World, t: usize, c: Body, rows: &[f32]) {
    let mut o = std::mem::take(&mut w.life.bodies[t]);
    // A tally for the tape: the members joined whose means are not alike (the monthly merge only
    // joins alike nodes below twice `crowd`; children join within their spreads).
    if !alike(&Layout::of(&w.chem), w.cfg.phys.alike, &o.genes, &c.genes) {
        w.diag.wide += c.n();
    }
    take_in(w, &mut o, c, Some(rows));
    w.life.bodies[t] = o;
}

/// `join` into body `o`, taken out of its slot. `rows` are the born rows of `c`; none: those of `o`
/// (a member split off `o` coming back), which leaves its rows and its group's as they are.
pub fn take_in(w: &mut World, o: &mut Body, c: Body, rows: Option<&[f32]>) {
    let (lay, p, a) = (Layout::of(&w.chem), &w.cfg.phys, (c.n() / (o.n() + c.n())) as f32);
    let mix = |x: &[f32], y: &[f32]| x.iter().zip(y).map(|(p, q)| p + a * (q - p)).collect::<Vec<f32>>();
    let pool = |v: f32, u: f32, d2: f32, step: f32| ((1.0 - a) * v + a * u + a * (1.0 - a) * d2).min(most(step, p.alike));
    let s = |b: &Body, g: usize| b.spread.get(g).copied().unwrap_or(0.0);
    o.spread = (0..o.genes.len()).map(|g| pool(s(o, g), s(&c, g), (c.genes[g] - o.genes[g]).powi(2), range(&lay, g).0)).collect();
    let born = w.life.born(o);
    let gap = rows.map_or(0.0, |rows| born.iter().zip(rows).map(|(p, q)| (q - p) * (q - p)).sum::<f32>() / born.len().max(1) as f32);
    o.row_spread = pool(o.row_spread, c.row_spread, gap, row_step(p, 10f32.powf(o.genes[body::MUT])));
    let genes = mix(&o.genes, &c.genes);
    match rows {
        Some(rows) => w.life.reborn(o, genes, mix(born, rows)),
        None => w.life.regene(o, genes),
    }
    o.store.iter_mut().zip(&c.store).for_each(|(s, x)| *s += x);
    // The brood adds up like the stores it is part of.
    o.brood += c.brood;
    // A node has no wear: its members are of all ages (`body::live`).
    (o.e, o.count, o.wear) = (o.e + c.e, o.n() + c.n(), 0.0);
    o.vis = o.visible(&w.chem, &w.cfg.phys);
}

/// Share `f` of node `b`'s members as a new body (not placed): that share of each store and of the
/// heat; lineage, group, genome, memory as the node's; holding nothing, inside nothing.
fn part(b: &mut Body, f: f64) -> Body {
    // (The trace and the constants are not the part's: set aside, not cloned.)
    let (trace, k) = (std::mem::take(&mut b.trace), std::mem::take(&mut b.k));
    let mut c = Body { count: f * b.count, e: f * b.e, brood: f * b.brood, hold: 0, inside: 0, ..b.clone() };
    (b.trace, b.k) = (trace, k);
    for (a, x) in b.store.iter_mut().zip(c.store.iter_mut()) {
        *x = f * *a;
        *a -= *x;
    }
    (b.e, b.count, b.brood) = (b.e - c.e, b.count - c.count, b.brood - c.brood);
    c
}

/// At its update, a node whose members differ so much that its two halves would not be alike
/// splits in two (plan 7.1). The halves of a normal spread have means `d = sqrt(2/π)` standard
/// deviations from the mean, so it splits when the halves' gap passes `phys.alike` steps along
/// the gene where it is widest, in its steps (so not along a gene that is off in both halves; the
/// brain's sizes count here in their mutation steps, in likeness by whole units), or, with `cfg.split_rows`, of the born rows in row steps (the
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
/// that much: a monthly round of selection. Halves of a brain size are alike too (likeness counts
/// the sizes by whole units, `like`), so they go the same round, each paying for its own size (`Lineage::cost`
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
    if !w.cfg.nodes && !body::light_eater(&w.chem, &b.genes) {
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
    w.diag.splits[1] += 1.0;
    draw(w, b, r)
}

/// `n` members of the node in `slot` as individuals, each drawn as `one` draws a member (genes
/// from the node's spread, a share of its stores and heat, wear uniform), placed in the node's
/// cell: the close view of a node, where the player sees its members (plan 7.1, "split it back
/// into individuals when the player looks closely"), and what the check of nodes against
/// individuals compares. Each draw leaves the others' genes, so the mean of all is kept. The node
/// keeps at least one member (draws stop when it has fewer than two); members, matter and energy
/// are kept exactly. Returns the members' slots.
pub fn open(w: &mut World, slot: usize, n: usize) -> Vec<usize> {
    let mut b = std::mem::take(&mut w.life.bodies[slot]);
    let (mut r, keep) = (Rng::new(w.seed, w.day, !b.id(slot), Purpose::Split), w.keep_of(slot, b.gen));
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        // (A count a rounding away from a whole number is that number.)
        if (b.count - b.count.round()).abs() < 1e-9 {
            b.count = b.count.round();
        }
        if b.count < 2.0 {
            break;
        }
        let s = draw(w, &mut b, &mut r);
        w.set_keep(s, w.life.bodies[s].gen, keep);
        out.push(s);
    }
    w.life.bodies[slot] = b;
    out
}

/// One member drawn from node `b` (see `one`, which counts it as a split).
fn draw(w: &mut World, b: &mut Body, r: &mut Rng) -> usize {
    let (lay, n) = (Layout::of(&w.chem), b.count as f32);
    // (The member keeps no spread: it is left out of the clone.)
    let spread = std::mem::take(&mut b.spread);
    let (mut c, mut genes) = (part(b, 1.0 / b.count), b.genes.clone());
    b.spread = spread;
    for (g, x) in c.genes.iter_mut().enumerate() {
        let ((_, lo, hi), v) = (range(&lay, g), b.spread.get(g).map_or(0.0, |s| s.max(0.0)));
        *x += v.sqrt().min((*x - lo).min(hi - *x).max(0.0) / REACH) * noise(r);
        let d = *x - genes[g];
        genes[g] = (genes[g] - d / (n - 1.0)).clamp(lo, hi);
        if let Some(s) = b.spread.get_mut(g) {
            *s = ((n * v - d * d * n / (n - 1.0)) / (n - 1.0)).max(0.0);
        }
    }
    w.life.regene(b, genes);
    (c.count, c.spread, c.row_spread, c.wear) = (1.0, Vec::new(), 0.0, r.f64());
    own(w, b, &mut c);
    let delay = c.interval();
    w.life.add(c, w.day, delay)
}
