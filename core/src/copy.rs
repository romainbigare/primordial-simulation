//! Law 4: Copy. A body with enough mass that can pay the cost makes a child: a share of each
//! of its stores and of its heat, and its genes plus small noise (now and then a change of the
//! brain rows it was born with, else it shares its parent's); it starts in its parent's brain
//! group (or, if brains are not shared, in a group of its own with what its parent learned). A
//! child whose brain-size genes are a whole unit away from its lineage's sizes starts a new
//! lineage (and group) with the new sizes, its brain carried over where the sizes overlap;
//! rarely the base of its lineage's brain is copied with small noise, which starts a new
//! lineage too.

use crate::body::{self, Body, Layout, Lineage};
use crate::{brain, choose};
use crate::event::{Event, EventKind};
use crate::grid::DIRS;
use crate::rng::{Purpose, Rng};
use crate::World;

/// Energy to make one kg of child, J.
const C_COPY: f64 = 5e5;
/// Chance that a child starts a new lineage with a changed base.
const NEW_LINEAGE: f64 = 0.005;
/// Size of the base changes of a new lineage.
const BASE_NOISE: f32 = 0.03;
/// Chance that a child's born brain rows change, and the step of that change per generation
/// (times the body's mutation size; a change is bigger by 1 / sqrt(chance), so the rows drift
/// as if each child changed them a little). The other children share their parent's rows.
const ROW_CHANCE: f64 = 0.1;
const ROW_NOISE: f32 = 0.05;
/// Spread of the weights a bigger brain adds.
const NEW_NOISE: f32 = 0.1;

/// Mutation step (times the body's mutation size) and range of a gene.
fn range(lay: &Layout, g: usize) -> (f32, f32, f32) {
    match g {
        body::GAS => (0.3, -1.0, 5.0),
        body::LIQ => (0.3, -4.0, 1.0),
        body::HEAT => (0.3, -3.0, 3.0),
        body::LEAF => (0.3, -4.0, 1.0),
        body::STRENGTH => (0.1, 0.0, 1.0),
        body::CHILD => (0.05, 0.05, 0.5),
        body::COPY_MASS => (0.2, 0.0, 8.0),
        body::INTERVAL => (0.2, 0.7, 1.78),
        body::MUT => (0.2, -3.0, 0.0),
        body::LEARN => (0.2, -4.0, 0.0),
        body::FORGET => (0.2, -4.0, -1.7),
        body::HIDDEN => (1.0, 4.0, 64.0),
        body::MEMORY => (0.5, 0.0, 12.0),
        body::SEEN => (0.3, 1.0, 6.0),
        body::SIGHT => (0.2, 1.0, 4.0),
        g if g < lay.min => (1.0, -2.0, 18.0),
        g if g < lay.upt => (0.05, -0.2, 0.9),
        _ => (0.2, -0.5, 2.0),
    }
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
    let mass = b.mass(&w.chem);
    if mass < b.dec(body::COPY_MASS) * b.n() {
        return false;
    }
    let id = b.id(slot);
    let mut r = Rng::new(w.seed, w.day, id, Purpose::Mutate);
    let (i, max) = (b.cell as usize, w.cfg.max_bodies_per_cell);
    let room = |j: usize| w.life.at[j].len() < max;
    let cell = if room(i) { Some(i) } else { w.grid.step(i, DIRS[r.below(8)]).filter(|&j| room(j)) };
    let frac = b.gene(body::CHILD).clamp(0.05, 0.5);
    let Some(cell) = cell else { return false };
    if !body::pay(w, b, C_COPY * frac * mass, b.interval() as f64) {
        return false;
    }
    let mut c = Body {
        cell: cell as u32,
        lineage: b.lineage,
        parent: id,
        born: w.day,
        mark: w.light[cell],
        e: b.e * frac,
        store: b.store.iter().map(|a| a * frac).collect(),
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
    if let Some(lin) = &w.life.brains[b.lineage as usize] {
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
    let s = w.life.add(c, w.day, delay);
    let child = w.life.bodies[s].id(s);
    w.events.push(Event { day: w.day, kind: EventKind::Born, node: cell as u32, value: child as f64 });
    true
}
