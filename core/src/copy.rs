//! Law 4: Copy. A body whose members have enough mass, if it can pay the cost, makes a child
//! per member: a share of each of its stores and of its heat, and its genes plus small noise
//! (now and then a change of the brain rows it was born with, else it shares its parent's); it
//! starts in its parent's brain group (or, if brains are not shared, in a group of its own with
//! what its parent learned). A child whose brain-size genes are a whole unit away from its
//! lineage's sizes starts a new lineage (and group) with the new sizes, its brain carried over
//! where the sizes overlap; rarely the base of its lineage's brain is copied with small noise,
//! which starts a new lineage too. A member past `FULL` times its copy mass has as many
//! children as it grew past that.
//!
//! In a population node (count > 1) the children that stay are new members of it; a share of
//! them (the `DISPERSE` gene; none if less than one member) starts in a random neighbour cell
//! with the changed genes, where it joins the first node of its lineage (genes and born rows
//! become the means by count) or starts a node. An individual's children are a new body, in a
//! neighbour cell by the same chance; if more than one, they join a node of its lineage there as
//! a node's do. Only individuals' children start new lineages.

use crate::body::{self, Body, Layout, Lineage};
use crate::{brain, choose};
use crate::event::{Event, EventKind};
use crate::grid::DIRS;
use crate::rng::{Purpose, Rng};
use crate::World;

/// Energy to make one kg of child, J.
const C_COPY: f64 = 5e5;
/// Chance that a child starts a new lineage with a changed base (bodies of real size have many
/// more generations a year than the giant ones this was first set for).
const NEW_LINEAGE: f64 = 0.001;
/// Size of the base changes of a new lineage.
const BASE_NOISE: f32 = 0.03;
/// Chance that a child's born brain rows change, and the step of that change per generation
/// (times the body's mutation size; a change is bigger by 1 / sqrt(chance), so the rows drift
/// as if each child changed them a little). The other children share their parent's rows.
const ROW_CHANCE: f64 = 0.1;
const ROW_NOISE: f32 = 0.05;
/// Spread of the weights a bigger brain adds.
const NEW_NOISE: f32 = 0.1;
/// A member holding this many times its copy mass copies whether its brain draws it or not
/// (growth past the size its genes set goes into new members).
pub const FULL: f64 = 2.0;
/// Nodes in one cell whose genes differ by at most this many mutation steps in each merge
/// (`body::crowd`).
const ALIKE: f32 = 2.0;

/// Mutation step (times the body's mutation size) and range of a gene.
fn range(lay: &Layout, g: usize) -> (f32, f32, f32) {
    match g {
        body::GAS => (0.3, -1.0, 5.0),
        body::LIQ => (0.3, -4.0, 1.0),
        body::HEAT => (0.3, -3.0, 3.0),
        body::LEAF => (0.3, -4.0, 1.0),
        body::STRENGTH => (0.1, 0.0, 1.0),
        body::CHILD => (0.05, 0.05, 0.5),
        body::COPY_MASS => (0.2, -3.0, 4.0),
        body::INTERVAL => (0.2, 0.7, 1.78),
        body::MUT => (0.2, -3.0, 0.0),
        body::LEARN => (0.2, -4.0, 0.0),
        body::FORGET => (0.2, -4.0, -1.7),
        body::HIDDEN => (0.3, 4.0, 64.0),
        body::MEMORY => (0.15, 0.0, 12.0),
        body::SEEN => (0.1, 1.0, 6.0),
        body::SIGHT => (0.07, 1.0, 4.0),
        body::DISPERSE => (0.05, 0.0, 1.0),
        g if g < lay.min => (1.0, -2.0, 18.0),
        g if g < lay.upt => (0.05, -0.2, 0.9),
        _ => (0.2, -0.5, 2.0),
    }
}

/// Genes no more than `ALIKE` mutation steps apart in each gene (genes that are off count as 0;
/// the sizes of brain and senses are not compared, as `merge` carries a brain over to other sizes).
pub fn alike(lay: &Layout, a: &[f32], b: &[f32]) -> bool {
    let on = |g: usize, x: f32| if g < lay.cat { x } else { x.max(0.0) };
    let sizes = body::HIDDEN..=body::SIGHT;
    a.iter().zip(b).enumerate().all(|(g, (&x, &y))| sizes.contains(&g) || (on(g, x) - on(g, y)).abs() <= ALIKE * range(lay, g).0)
}

/// Parent genes plus noise, kept in range.
pub fn mutate(lay: &Layout, genes: &[f32], r: &mut Rng) -> Vec<f32> {
    let size = 10f32.powf(genes[body::MUT]);
    (genes.iter().enumerate())
        .map(|(g, &v)| {
            let (step, lo, hi) = range(lay, g);
            // Noise near a standard normal: a sum of four uniforms, centred and scaled.
            let z = ((0..4).map(|_| r.f64()).sum::<f64>() - 2.0) * 3f64.sqrt();
            (v + step * size * z as f32).clamp(lo, hi)
        })
        .collect()
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
    // A member past `FULL` times its copy mass gives all it grew past that to children of the
    // size it would give at that mass: share `s` of its stores, `k` children.
    let (m1, full) = (mass / n, FULL * mc);
    let (s, k) = if m1 > full { (1.0 - (1.0 - frac) * full / m1, (m1 / full - 1.0 + frac) / frac) } else { (frac, 1.0) };
    if !body::pay(w, b, C_COPY * s * mass, b.interval() as f64) {
        return false;
    }
    // An individual's children are whole.
    let (i, births) = (b.cell as usize, if n > 1.0 { n * k } else { k.round() });
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
        if m == 0.0 {
            return true;
        }
    }
    let share = s * m / births;
    let mut c = Body {
        cell: cell as u32,
        count: m,
        lineage: b.lineage,
        parent: id,
        born: w.day,
        mark: w.light[cell],
        e: b.e * share,
        store: b.store.iter().map(|a| a * share).collect(),
        genes: mutate(&lay, &b.genes, &mut r),
        genome: b.genome,
        group: b.group,
        near: u32::MAX,
        ..Default::default()
    };
    b.e -= c.e;
    for (a, x) in b.store.iter_mut().zip(&c.store) {
        *a -= x;
    }
    let mut rows = (r.f64() < ROW_CHANCE).then(|| {
        let mut v = w.life.born(b).to_vec();
        shake(&mut v, &mut r, ROW_NOISE * 10f32.powf(b.genes[body::MUT]) / (ROW_CHANCE as f32).sqrt());
        v
    });
    // A node's children start no new lineage: a change of the brain's base or sizes in one child
    // is lost in the mean of many (the laws in average form, plan 7.1).
    if let Some(lin) = w.life.brains[b.lineage as usize].as_ref().filter(|_| !node) {
        let (old, want) = (choose::sizes_of(lin), choose::sizes(&c.genes));
        let gene = [body::HIDDEN, body::MEMORY, body::SEEN, body::SIGHT].map(|g| c.genes[g]);
        let new: [usize; 4] = std::array::from_fn(|k| if (gene[k] - old[k] as f32).abs() >= 1.0 { want[k] } else { old[k] });
        let fresh = if new != old {
            let to = choose::shape(w.chem.ns(), new[0], new[1]);
            rows = Some(brain::rerow(rows.as_deref().unwrap_or(w.life.born(b)), lin.shape, to, &mut r, NEW_NOISE));
            Some(Lineage { shape: to, seen: new[2], sight: new[3], base: brain::rebase(&lin.base, lin.shape, to, &mut r, NEW_NOISE) })
        } else if r.f64() < NEW_LINEAGE {
            let mut l = lin.clone();
            shake(&mut l.base, &mut r, BASE_NOISE);
            Some(l)
        } else {
            None
        };
        if let Some(l) = fresh {
            c.lineage = w.life.new_lineage(l);
            c.group = w.life.new_group(c.lineage, rows.clone().unwrap_or_else(|| w.life.born(b).to_vec()));
        }
    }
    // Children of a node, or more than one child of an individual, join a node of their lineage
    // there.
    let joins = |o: &Body| o.alive && o.lineage == c.lineage && o.n() > 1.0;
    if (node || m > 1.0) && c.lineage == b.lineage {
        if let Some(t) = w.life.at[cell].iter().map(|&s| s as usize).find(|&s| joins(&w.life.bodies[s])) {
            let rows = rows.unwrap_or_else(|| w.life.born(b).to_vec());
            join(w, t, c, &rows);
            return true;
        }
    }
    if let Some(v) = rows {
        c.genome = w.life.new_genome(v);
    }
    // Without shared brains the child starts a group of its own: its born rows plus what its
    // parent's group learned.
    if !w.cfg.share_brains && c.group == b.group {
        let gr = &w.life.groups[b.group as usize];
        let rows = w.life.born(&c).iter().zip(gr.learned()).map(|(r, l)| r + l).collect();
        let (var, ms) = (gr.var, gr.ms.clone());
        c.group = w.life.new_group(c.lineage, rows);
        (w.life.groups[c.group as usize].var, w.life.groups[c.group as usize].ms) = (var, ms);
    }
    (c.wet, c.vis) = (c.key_liquids(&w.chem), c.visible(&w.chem));
    let delay = c.interval();
    w.life.add(c, w.day, delay);
    true
}

/// The body in slot `s` joins the body in slot `t` (`join`), its born rows carried over to the
/// sizes of the other's brain where they differ.
pub fn merge(w: &mut World, s: usize, t: usize) {
    let (life, c) = (&w.life, w.life.bodies[s].clone());
    let shape = |b: &Body| life.brains[b.lineage as usize].as_ref().map(|l| l.shape);
    let rows = match (shape(&c), shape(&life.bodies[t])) {
        (Some(a), Some(b)) if a != b => brain::rerow(life.born(&c), a, b, &mut Rng::new(w.seed, w.day, c.id(s), Purpose::Mutate), NEW_NOISE),
        _ => life.born(&c).to_vec(),
    };
    w.life.remove(s);
    join(w, t, c, &rows);
}

/// Members `c` (not placed, born with `rows`) join body `t` of their lineage: stores and heat
/// add up; genes and born rows become the means by count.
pub fn join(w: &mut World, t: usize, c: Body, rows: &[f32]) {
    let mut o = std::mem::take(&mut w.life.bodies[t]);
    let a = (c.n() / (o.n() + c.n())) as f32;
    let mix = |x: &[f32], y: &[f32]| x.iter().zip(y).map(|(p, q)| p + a * (q - p)).collect::<Vec<f32>>();
    let (genes, born) = (mix(&o.genes, &c.genes), mix(w.life.born(&o), rows));
    w.life.reborn(&mut o, genes, born);
    o.store.iter_mut().zip(&c.store).for_each(|(s, x)| *s += x);
    (o.e, o.count) = (o.e + c.e, o.n() + c.n());
    o.vis = o.visible(&w.chem);
    w.life.bodies[t] = o;
}
