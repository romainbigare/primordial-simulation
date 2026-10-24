//! A node's decision as a spread of actions (handover 7.1): its members spread over the options by the chances its
//! brain gives them, the chances an individual draws from, and each part acts as so many members would.

use super::*;
use crate::nodes;

/// Node `node` (in slot `slot`) spreads its members over the options: count × chance of them do each, a part of less
/// than one member being one member with that chance. A part acts as so many members together (`part_acts`; the moves
/// last, so the others act where the node is). Its brain learns as from one member drawn among those who stayed (`trace`): from what that member did,
/// and how much more its part's action raised their well-being than the mean of the members.
pub(super) fn spread(w: &mut World, slot: usize, node: &mut Body, c: &Choice, k: &Kit, rng: &mut Rng) {
    let x: [f64; OPTS] = std::array::from_fn(|o| match node.count * c.p[o] as f64 { x if x > 0.0 && x < 1.0 => (rng.f64() < x) as u8 as f64, x => x });
    // The members of each option in the node after acting, and the change of their well-being.
    let (mut kept, mut gain) = ([0.0; OPTS], [0f32; OPTS]);
    for o in (0..FIXED).chain(TO_BODY..OPTS).chain(FIXED..TO_BODY).filter(|&o| x[o] > 0.0) {
        let (g, left) = if o == REST { (0.0, Some(false)) } else { part_acts(w, slot, node, c, k, (o, x[o]), rng) };
        (gain[o], kept[o]) = (g, if left == Some(true) { 0.0 } else { x[o] });
        let Some(_) = left else { break };
    }
    // (With no member drawn for any option, all rest.)
    let mean = kept.iter().zip(&gain).map(|(n, &g)| n * g as f64).sum::<f64>() / kept.iter().sum::<f64>().max(f64::MIN_POSITIVE);
    let mut u = rng.f64() * kept.iter().sum::<f64>();
    let a = (0..OPTS).find(|&o| { u -= kept[o]; u < 0.0 }).unwrap_or(REST);
    node.trace = trace(w, node, c, k, a, gain[a] - mean as f32);
}

/// `x` members of node `node` (in slot `slot`) do option `o`. Members that stay in its cell (any option but a move or a
/// bond) act as the node itself, holding and inside nothing, while the others wait aside with their share of its amounts
/// (`nodes::Lot::split`); then those come back (`nodes::rejoin`: amounts, sizes and, after watching, rows by members).
/// Members that move or bond are placed as a body of their own (`nodes::part`); those still in its cell, holding and
/// inside nothing, then come back as well; the others live on as a body with the trace of what they did, and those who
/// moved join the first node of their kind and stage where they are (`copy::kin`). The node acts itself if less than
/// one member would be left. Returns the change of their well-being, and whether they left the node (none: it moved
/// whole).
fn part_acts(w: &mut World, slot: usize, node: &mut Body, c: &Choice, k: &Kit, (o, x): (usize, f64), rng: &mut Rng) -> (f32, Option<bool>) {
    let (cell, moves, bond) = (node.cell, (FIXED..TO_BODY).contains(&o), (FORM..FIXED).contains(&o) || o >= TO_PART);
    let mut acts = |w: &mut World, s: usize, b: &mut Body| {
        let before = (k.wb)(w, b);
        act_as(w, s, b, c, k, o, rng);
        (k.wb)(w, b) - before
    };
    if node.count - x < 1.0 {
        return (acts(w, slot, node), (node.cell == cell).then_some(false));
    }
    let f = x / node.count;
    if !(moves || bond) {
        // The others wait aside with their share; as members split by shares, their sizes are the node's.
        let size = node.size_moments(&w.chem);
        let rest = nodes::Lot::split(&mut w.life, node, 1.0 - f, 1.0 - f);
        let held = (node.hold.take(), node.inside.take());
        // A watcher changes its rows: the others keep the node's.
        let kept_rows = ((TO_BODY..TO_PART).contains(&o) && (o - TO_BODY) % VERBS == WATCH).then(|| node.rows.clone());
        let gain = acts(w, slot, node);
        debug_assert!(node.hold.is_none() && node.inside.is_none(), "an action that stays in its cell takes no hold and enters nothing");
        (node.hold, node.inside) = held;
        nodes::rejoin(w, node, rest, size, kept_rows.as_deref());
        return (gain, Some(false));
    }
    let part = nodes::part(&mut w.life, node, f, f);
    let s = w.place(part);
    let mut m = std::mem::take(&mut w.life.bodies[s]);
    let gain = acts(w, s, &mut m);
    if m.cell == cell && m.hold.is_none() && m.inside.is_none() {
        w.life.remove(s, &m);
        let size = m.size_moments(&w.chem);
        nodes::rejoin(w, node, nodes::Lot::of(&mut m), size, None);
        return (gain, Some(false));
    }
    m.trace = trace(w, &m, c, k, o, 0.0);
    w.see(&mut m);
    // (Out of its slot, it is no node of its cell for `kin`.)
    let kin = crate::copy::kin(w, m.cell as usize, m.lineage, (&m.genes, &m.spread), m.adult(&w.chem)).filter(|_| moves);
    w.life.bodies[s] = m;
    if let Some(t) = kin {
        nodes::merge(w, s, t);
    }
    (gain, Some(true))
}
