//! Learning: a brain learns from the change of its body's well-being (`learn`, a policy gradient against the critic's
//! estimate), groups share and forget what they learned (`regroup`), and a body copies part of another's brain by
//! watching it (`watch`).

use super::NONE;
use crate::body::Body;
use crate::brain::{self, ln, Shape, FIXED, OVERBS, VERBS};
use crate::event::{Event, EventKind};
use crate::World;

/// Weight of each new error in the critic's moving average of its square (`Group::var`), and the floor of its root.
const CRITIC_RATE: f32 = 0.05;
const CRITIC_FLOOR: f32 = 1e-3;
/// Rate of a body's own adjustment of its action scores (`Body::adj`) against that of the rows, and its bounds.
const ADJ_RATE: f32 = 0.1;
const ADJ_MOST: f32 = 3.0;
/// A body copies less of one whose strength differs by this much (one standard deviation, as one of ln mass or ln
/// capture area).
const STRENGTH_STEP: f32 = 0.2;

/// Learn from the last decision: the change of well-being since, plus the discounted estimate
/// of how it will go from the state now, less the estimate made then. The critic row moves
/// toward that; the group's learned rows and the body's own adjustment move along the
/// decision's trace by it, in units of its spread, times the body's learning rate (each row by
/// a step of its own size, `brain::learn`).
pub(super) fn learn(w: &mut World, b: &mut Body, s: Shape, wb: f32, now: &[f32]) {
    let (l, lr, cr, nh) = (s.learned(), b.learn_rate(), s.critic() * s.r(), s.h);
    if b.trace.len() != nh + l || lr <= 0.0 {
        return;
    }
    let gr = &mut w.life.groups[b.group as usize];
    let (h, c) = b.trace.split_at(nh);
    let v = |x: &[f32]| gr.rows[cr + nh] + brain::dot(&gr.rows[cr..cr + nh], x);
    let delta = wb - b.wb + w.cfg.phys.discount * v(now) - v(h);
    gr.var = if gr.var > 0.0 { gr.var + CRITIC_RATE * (delta * delta - gr.var) } else { delta * delta };
    let a = lr * delta / (gr.var.sqrt() + CRITIC_FLOOR);
    if !a.is_finite() {
        return;
    }
    // The group's rows step by the rate split over its members, so a shared brain moves as fast
    // as one body's would (on the mean of what its members met): a node's decision is that of
    // all its members, so it weighs by its count, and an individual split off a node of thousands
    // weighs as one of them (counts as of the month's start; a new group's first body, all).
    let gl = (lr as f64 * b.n() / gr.count.max(b.n())) as f32;
    brain::fit(&mut gr.rows[cr..cr + s.r()], h, &[1.0], gl * delta);
    brain::learn(&mut gr.rows[..l * s.r()], h, c, a / lr, gl, &mut gr.ms, (w.cfg.phys.learn_beta, w.cfg.phys.learn_most));
    let bias = (0..FIXED).map(|q| c[q]).chain([c[s.dir()]]).chain((0..VERBS + OVERBS).map(|v| c[s.verb(v)]));
    for (j, cb) in b.adj.iter_mut().zip(bias) {
        *j = (*j + ADJ_RATE * a * cb).clamp(-ADJ_MOST, ADJ_MOST);
    }
}

/// Monthly: what each group learned fades by its forgetting rate. A body that has stayed for a
/// year within `phys.join` cells of the centre of a bigger group of its lineage joins it; if it was
/// alone, what it learned is blended in by its share. (No joining unless brains are shared.)
pub(crate) fn regroup(w: &mut World) {
    let (gw, gh, day, share, life) = (w.grid.w, w.grid.h, w.day, w.cfg.share_brains, &mut w.life);
    let at = |g: &crate::life::Group| {
        let (x, y) = g.centre();
        (y.round() as usize).min(gh - 1) * gw + (x.round() as usize).min(gw - 1)
    };
    // Groups by the cell of their centre.
    let mut head = vec![0u32; gw * gh + 1];
    for g in life.groups.iter().filter(|g| g.members > 0) {
        head[at(g) + 1] += 1;
    }
    for c in 0..gw * gh {
        head[c + 1] += head[c];
    }
    let (mut list, mut fill) = (vec![0u32; head[gw * gh] as usize], head.clone());
    for (k, g) in life.groups.iter().enumerate().filter(|(_, g)| g.members > 0) {
        list[fill[at(g)] as usize] = k as u32;
        fill[at(g)] += 1;
    }
    for g in life.groups.iter_mut().filter(|g| g.members > 0) {
        let f = (g.forget / g.members as f32).max(0.0);
        if f > 0.0 {
            let fade: Vec<f32> = g.learned().map(|l| f * l).collect();
            g.rows.iter_mut().zip(fade).for_each(|(r, x)| *r -= x);
        }
    }
    if !share {
        return;
    }
    for slot in 0..life.bodies.len() {
        let b = &life.bodies[slot];
        if !b.alive {
            continue;
        }
        let ((x, y), groups) = (((b.cell as usize) % gw, (b.cell as usize) / gw), &life.groups);
        let key = |g: u32| (groups[g as usize].members, u32::MAX - g);
        let mut best = b.group;
        for yy in y.saturating_sub(w.cfg.phys.join)..=(y + w.cfg.phys.join).min(gh - 1) {
            for xx in x.saturating_sub(w.cfg.phys.join)..=(x + w.cfg.phys.join).min(gw - 1) {
                let c = yy * gw + xx;
                for &g in &list[head[c] as usize..head[c + 1] as usize] {
                    if groups[g as usize].members > 0 && groups[g as usize].lineage == b.lineage && key(g) > key(best) {
                        best = g;
                    }
                }
            }
        }
        let b = &mut life.bodies[slot];
        if best == b.group {
            b.near = NONE;
        } else if best != b.near {
            (b.near, b.since) = (best, day);
        } else if day - b.since >= w.cfg.phys.stay {
            let old = b.group;
            (b.group, b.near) = (best, NONE);
            let o = &life.groups[old as usize];
            let lone: Option<Vec<f32>> = (o.members == 1).then(|| o.learned().collect());
            let b = std::mem::take(&mut life.bodies[slot]);
            life.enrol(best, &b, 1);
            life.enrol(old, &b, -1);
            life.bodies[slot] = b;
            if let Some(o) = lone {
                let gr = &mut life.groups[best as usize];
                let n = gr.members as f32;
                let add: Vec<f32> = gr.learned().zip(o).map(|(l, x)| (x - l) / n).collect();
                gr.rows.iter_mut().zip(add).for_each(|(r, a)| *r += a);
            }
        }
    }
}

/// Watch another body (it costs the turn): the group's rows move part of the way toward the
/// rows of the other's group if their brains have the same sizes (the share split over the
/// group's members), and the body's own adjustment toward the other's; less the less alike the two bodies are (size,
/// light-capture area, strength), as a body can only copy what its own body can do.
pub(super) fn watch(w: &mut World, b: &mut Body, s: usize) {
    let o = &w.life.bodies[s];
    let (og, oadj, ol, (u, v)) = (o.group as usize, o.adj, o.lineage, (o.vis, b.visible(&w.chem, &w.cfg.phys)));
    let d = |a: f32, b: f32| ln(a.max(1e-9) / b.max(1e-9)).powi(2);
    let alike = (-0.5 * (d(u[0], v[0]) + d(u[1], v[1]) + ((u[2] - v[2]) / STRENGTH_STEP).powi(2))).exp();
    let alike = if w.cfg.null { 0.0 } else { w.cfg.phys.blend * alike };
    brain::blend(&mut b.adj, &oadj, alike);
    let g = b.group as usize;
    if og != g && w.life.shape_of(ol) == w.life.shape_of(b.lineage) {
        let (lo, hi) = w.life.groups.split_at_mut(g.max(og));
        let (mine, theirs) = if g < og { (&mut lo[g], &hi[0]) } else { (&mut hi[0], &lo[og]) };
        brain::blend(&mut mine.rows, &theirs.rows, alike / mine.members.max(1) as f32);
    }
    let same = (ol == b.lineage) as u8 as f64;
    w.events.push(Event { day: w.day, kind: EventKind::Watched { by: g as u32, of: og as u32 }, node: b.cell, value: same });
}
