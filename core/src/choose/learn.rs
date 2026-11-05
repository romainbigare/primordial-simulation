//! Learning: a brain learns from the change of its body's well-being (`learn`, a policy gradient against the critic's
//! estimate), and a body copies part of another's brain by watching it (`watched`).

use crate::body::Body;
use crate::brain::{self, ln, Shape};
use crate::life::Mind;
use crate::math::Span;
use crate::event::{Event, EventKind};
use crate::World;

/// The floor of the root of the critic's moving mean square of its errors (`Body::var`), of a month's error.
const CRITIC_FLOOR: f32 = 1e-3;
/// A body copies less of one whose strength differs by this much (one standard deviation, as one of ln mass).
const STRENGTH_STEP: f32 = 0.2;

/// Learn from the last decision (actor-critic) over the time since, `span`: the change of well-being since (for a node,
/// of the member its trace stands for, `choose::spread`) plus the estimate from now, discounted over that time
/// (`discount` per month), less the estimate then. That error is of the time: the change and the discount over it. The
/// critic moves by the share `LEARN` of it, so it learns alike per second whatever the pace of decisions (continuous-time
/// TD learning, Doya 2000). The learned rows move along the decision's trace by it in units of its spread, which is of
/// the time too: their step is the learning rate per month over the time (`brain::learn`). The spread's moving average
/// weighs the newest error by `learn_beta` per month.
pub(super) fn learn(w: &World, b: &mut Body, m: &mut Mind, s: Shape, (wb, now): (f32, &[f32]), span: Span) {
    let (l, lr, cr, nh) = (s.learned(), b.learn_rate(), s.critic() * s.r(), s.h);
    if !b.traced || m.trace.len() != nh + l + 1 || lr <= 0.0 {
        return;
    }
    let p = &w.cfg.phys;
    // (The trace ends with the advantage of what was done, `choose::trace`.)
    let (h, c) = m.trace.split_at(nh);
    let v = |x: &[f32]| m.rows[cr + nh] + brain::dot(&m.rows[cr..cr + nh], x);
    let delta = wb - b.wb + c[l] + span.weight(p.discount) * v(now) - v(h);
    // The critic's errors are averaged as the rows' steps are (`learn_beta`). An error too big for a number teaches
    // nothing.
    let beta = span.share(p.learn_beta);
    let var = if b.var > 0.0 { b.var + beta * (delta * delta - b.var) } else { delta * delta };
    let (rate, floor) = (span.amount(lr as f64) as f32, span.amount(CRITIC_FLOOR as f64) as f32);
    let a = rate * delta / (var.sqrt() + floor);
    if !(a.is_finite() && var.is_finite()) {
        return;
    }
    b.var = var;
    brain::fit(&mut m.rows[cr..cr + s.r()], h, &[1.0], lr * delta, span.amount(brain::MAX_FIT as f64) as f32);
    brain::learn(&mut m.rows[..l * s.r()], h, &c[..l], a / rate, rate, m.ms, (beta, p.learn_most));
}

/// Body `by` watched body `of` in a batch (it cost the turn; `n` of its members watched, of `lineage` then), applied
/// in the serial pass (`life::land`) on both as they are then, if both live and it is still of that lineage: its rows move
/// part of the way toward the other's (`blend` per month, over the turn: the time its update integrated, `days`)
/// if the brains have the same sizes, less the less alike the bodies are, as a body can only copy what its own body can
/// do, and by the share of its members now who watched (a node's rows are the mean of its members').
pub(crate) fn watched(w: &mut World, by: u64, (n, lineage): (f64, u32), (of, days): (u64, f64)) {
    let (Some(s), Some(t)) = (w.life.find(by).filter(|&s| w.life[s].lineage == lineage), w.life.find(of)) else { return };
    let (b, o) = (std::mem::take(&mut w.life[s]), &w.life[t]);
    let (u, v) = (o.visible(&w.chem), b.visible(&w.chem));
    let d = |a: f32, b: f32| ln(a.max(1e-9) / b.max(1e-9)).powi(2);
    let alike = (-0.5 * (d(u[0], v[0]) + ((u[1] - v[1]) / STRENGTH_STEP).powi(2))).exp();
    let alike = if w.cfg.null { 0.0 } else { w.cfg.span(days).share(w.cfg.phys.blend) * alike * (n / b.n()).min(1.0) as f32 };
    let (same, of) = ((o.lineage == b.lineage) as u8 as f64, o.id);
    let o = o.clone();
    w.life.blend_rows(&b, &o, alike);
    w.events.push(Event { day: w.day(), kind: EventKind::Watched { by: b.id, of }, node: b.cell, value: same });
    w.life[s] = b;
}
