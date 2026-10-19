//! Learning: a brain learns from the change of its body's well-being (`learn`, a policy gradient against the critic's
//! estimate), and a body copies part of another's brain by watching it (`watch`).

use crate::body::Body;
use crate::brain::{self, ln, Shape};
use crate::event::{Event, EventKind};
use crate::World;

/// Weight of each new error in the critic's moving average of its square (`Body::var`), and the floor of its root.
const CRITIC_RATE: f32 = 0.05;
const CRITIC_FLOOR: f32 = 1e-3;
/// A body copies less of one whose strength differs by this much (one standard deviation, as one of ln mass or ln
/// capture area).
const STRENGTH_STEP: f32 = 0.2;

/// Learn from the last decision (actor-critic): the change of well-being since plus the discounted estimate from now,
/// less the estimate then. The critic moves toward it; the learned rows move along the decision's trace by it, in units
/// of its spread, times the learning rate (`brain::learn`).
pub(super) fn learn(w: &World, b: &mut Body, s: Shape, wb: f32, now: &[f32]) {
    let (l, lr, cr, nh) = (s.learned(), b.learn_rate(), s.critic() * s.r(), s.h);
    if b.trace.len() != nh + l || lr <= 0.0 {
        return;
    }
    let (h, c) = b.trace.split_at(nh);
    let v = |x: &[f32]| b.rows[cr + nh] + brain::dot(&b.rows[cr..cr + nh], x);
    let delta = wb - b.wb + w.cfg.phys.discount * v(now) - v(h);
    // An error too big for a number teaches nothing.
    let var = if b.var > 0.0 { b.var + CRITIC_RATE * (delta * delta - b.var) } else { delta * delta };
    let a = lr * delta / (var.sqrt() + CRITIC_FLOOR);
    if !(a.is_finite() && var.is_finite()) {
        return;
    }
    b.var = var;
    brain::fit(&mut b.rows[cr..cr + s.r()], h, &[1.0], lr * delta);
    brain::learn(&mut b.rows[..l * s.r()], h, c, a / lr, lr, &mut b.ms, (w.cfg.phys.learn_beta, w.cfg.phys.learn_most));
}

/// Watch another body (it costs the turn): its rows move part of the way toward the other's if the brains have the same
/// sizes, less the less alike the bodies are, as a body can only copy what its own body can do.
pub(super) fn watch(w: &mut World, slot: usize, b: &mut Body, s: usize) {
    let o = &w.life.bodies[s];
    let (u, v) = (o.vis, b.visible(&w.chem, &w.cfg));
    let d = |a: f32, b: f32| ln(a.max(1e-9) / b.max(1e-9)).powi(2);
    let alike = (-0.5 * (d(u[0], v[0]) + d(u[1], v[1]) + ((u[2] - v[2]) / STRENGTH_STEP).powi(2))).exp();
    let alike = if w.cfg.null { 0.0 } else { w.cfg.phys.blend * alike };
    if w.life.shape_of(o.lineage) == w.life.shape_of(b.lineage) && o.rows.len() == b.rows.len() {
        brain::blend(&mut b.rows, &o.rows, alike);
    }
    let same = (o.lineage == b.lineage) as u8 as f64;
    w.events.push(Event { day: w.day(), kind: EventKind::Watched { by: b.id(slot), of: o.id(s) }, node: b.cell, value: same });
}
