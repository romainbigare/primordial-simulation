//! Law 4: Copy. A body with enough mass that can pay the cost makes a child: a share of each
//! of its stores and of its heat, and its genes plus small noise; it starts in its parent's
//! brain group. Now and then the brain of its lineage is copied with noise too, which starts a
//! new lineage (and a new group).

use crate::body::{self, Body, Layout};
use crate::brain;
use crate::event::{Event, EventKind};
use crate::grid::DIRS;
use crate::rng::{Purpose, Rng};
use crate::World;

/// Energy to make one kg of child, J.
const C_COPY: f64 = 5e5;
/// Chance that a child starts a new lineage with a changed brain.
const NEW_LINEAGE: f64 = 0.02;
/// Size of the brain changes of a new lineage.
const BRAIN_NOISE: f32 = 0.1;

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

pub fn copy(w: &mut World, slot: usize, b: &mut Body) -> bool {
    let lay = Layout::of(&w.chem);
    let mass = b.mass(&w.chem);
    if mass < b.dec(body::COPY_MASS) {
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
        group: b.group,
        near: u32::MAX,
        ..Default::default()
    };
    b.e -= c.e;
    for (a, x) in b.store.iter_mut().zip(&c.store) {
        *a -= x;
    }
    if r.f64() < NEW_LINEAGE {
        if let Some(mut brain) = w.life.brains[b.lineage as usize].clone() {
            brain::mutate(&mut brain, &mut r, BRAIN_NOISE);
            c.lineage = w.life.new_lineage(brain);
            c.group = crate::choose::found(w, c.lineage);
        }
    }
    (c.wet, c.vis) = (c.key_liquids(&w.chem), c.visible(&w.chem));
    let delay = c.interval();
    let s = w.life.add(c, w.day, delay);
    let child = w.life.bodies[s].id(s);
    w.events.push(Event { day: w.day, kind: EventKind::Born, node: cell as u32, value: child as f64 });
    true
}
