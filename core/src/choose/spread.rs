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

/// `x` members of node `node` (in slot `slot`) do option `o`, split off by shares (`nodes::part`). Those still in its
/// cell, holding and inside nothing, then go back in (watchers with the rows they copied); the others live on as a body
/// with the trace of what they did, and those who moved join the first node of their kind and stage where they are
/// (`copy::kin`). The node acts itself if less than one member would be left. Returns the change of their well-being, and whether they left the node (none: it moved whole).
fn part_acts(w: &mut World, slot: usize, node: &mut Body, c: &Choice, k: &Kit, (o, x): (usize, f64), rng: &mut Rng) -> (f32, Option<bool>) {
    let (cell, moves, bond) = (node.cell, (FIXED..TO_BODY).contains(&o), (FORM..FIXED).contains(&o) || o >= TO_PART);
    if node.count - x < 1.0 {
        let before = (k.wb)(w, node);
        act_as(w, slot, node, c, k, o, rng);
        return ((k.wb)(w, node) - before, (node.cell == cell).then_some(false));
    }
    let q = nodes::part(&mut w.life, node, x / node.count, x / node.count);
    let s = w.place(q);
    let mut m = std::mem::take(&mut w.life.bodies[s]);
    let before = (k.wb)(w, &m);
    // (A part that stays acts under the node's name.)
    act_as(w, if moves || bond { s } else { slot }, &mut m, c, k, o, rng);
    let gain = (k.wb)(w, &m) - before;
    if m.cell == cell && m.hold.is_none() && m.inside.is_none() {
        let born = ((TO_BODY..TO_PART).contains(&o) && (o - TO_BODY) % VERBS == WATCH).then(|| w.life.born(node).to_vec());
        w.life.bodies[s] = m.clone();
        w.life.remove(s);
        nodes::take_in(w, node, m, born.as_deref());
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
