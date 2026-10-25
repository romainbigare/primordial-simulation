//! A node's decision as a spread of actions (handover 7.1): its members spread over the options by the chances its
//! brain gives them, the chances an individual draws from, and each part acts as so many members would.

use super::go::can_go;
use super::*;
use crate::body::{die, LACK};
use crate::life::Handle;
use crate::store::{Class, Node, NodeId};
use crate::nodes;

/// Node `node` (taken out of its slot) spreads its members over the options (`members`), and each part acts as so many
/// members together (`part_acts`; the moves last, so the others act where the node is). Its brain learns as from one
/// member drawn among those who stayed (`trace`): from what that member did, and how much more its part's action raised
/// their well-being than the mean of the members.
pub(super) fn spread(w: &World, d: &mut Domain, node: &mut Body, c: &Choice, k: &Kit, rng: &mut Rng) {
    let x = members(node, c, rng);
    // The members of each option in the node after acting, and the change of their well-being.
    let (mut kept, mut gain) = ([0.0; OPTS], [0f32; OPTS]);
    for o in (0..FIXED).chain(TO_BODY..OPTS).chain(FIXED..TO_BODY).filter(|&o| x[o] > 0.0) {
        let (g, left) = if o == REST { (0.0, Some(false)) } else { part_acts(w, d, node, c, k, (o, x[o]), rng) };
        (gain[o], kept[o]) = (g, if left == Some(true) { 0.0 } else { x[o] });
        let Some(_) = left else { break };
    }
    // (With no member drawn for any option, all rest.)
    let mean = kept.iter().zip(&gain).map(|(n, &g)| n * g as f64).sum::<f64>() / kept.iter().sum::<f64>().max(f64::MIN_POSITIVE);
    let mut u = rng.f64() * kept.iter().sum::<f64>();
    let a = (0..OPTS).find(|&o| { u -= kept[o]; u < 0.0 }).unwrap_or(REST);
    node.trace = trace(w, node, c, k, a, gain[a] - mean as f32);
}

/// The members of node `node` that do each option: count × chance, a part of less than one member being one member with
/// that chance. The members that move are one group, count × the sum of the chances to go in every direction, and the
/// group takes the one direction drawn by its chance over that sum, so its option holds the group and the other
/// directions none.
fn members(node: &Body, c: &Choice, rng: &mut Rng) -> [f64; OPTS] {
    let mut part = |chance: f64| match node.count * chance { x if x > 0.0 && x < 1.0 => (rng.f64() < x) as u8 as f64, x => x };
    let moves = FIXED..TO_BODY;
    let mut x: [f64; OPTS] = std::array::from_fn(|o| if moves.contains(&o) { 0.0 } else { part(c.p[o] as f64) });
    let chance: f64 = c.p[moves.clone()].iter().map(|&p| p as f64).sum();
    let group = part(chance);
    if group > 0.0 {
        // The direction: the chance of each over their sum.
        let mut u = rng.f64() * chance;
        let way = moves.clone().filter(|&o| c.p[o] > 0.0).find(|&o| { u -= c.p[o] as f64; u < 0.0 });
        x[way.unwrap_or_else(|| moves.rev().find(|&o| c.p[o] > 0.0).expect("a group of movers has a direction"))] = group;
    }
    x
}

/// `x` members of node `node` (taken out of its slot) do option `o`. Members that stay in its cell (any option but a move or a
/// bond) act as the node itself, holding and inside nothing, while the others wait aside with their share of its amounts
/// (`nodes::Lot::split`); then those come back (`nodes::rejoin`: amounts and sizes by members).
/// Members that move or bond are placed as a body of their own (`nodes::part`); unless they queued a move or a part
/// act (`outbox`), they then come back as well; else they live on as a body, leaving the node (`Body::leaving`), with
/// the trace of what they did: those who move join the first node of their kind and stage where they land
/// (`choose::land_move`), those who act on a part come back after it if they can (`land_act`). The node acts itself if less
/// than one member would be left. Returns the change of their well-being, and whether they left the node (none: it
/// moved whole).
fn part_acts(w: &World, d: &mut Domain, node: &mut Body, c: &Choice, k: &Kit, (o, x): (usize, f64), rng: &mut Rng) -> (f32, Option<bool>) {
    let (moves, bond) = ((FIXED..TO_BODY).contains(&o), (FORM..FIXED).contains(&o) || o >= TO_PART);
    let mut acts = |d: &mut Domain, b: &mut Body, part_of: Option<u64>| {
        let before = (k.wb)(w, b);
        act_as(w, d, b, c, k, (o, part_of), rng);
        (k.wb)(w, b) - before
    };
    if node.count - x < 1.0 {
        return (acts(d, node, None), (!node.leaving).then_some(false));
    }
    let f = x / node.count;
    if !(moves || bond) {
        // The others wait aside with their share; as members split by shares, their sizes are the node's.
        let size = node.size_moments(&w.chem);
        let rest = nodes::Lot::split(d, node, 1.0 - f, 1.0 - f);
        let held = (node.hold.take(), node.inside.take());
        let gain = acts(d, node, None);
        debug_assert!(node.hold.is_none() && node.inside.is_none(), "an action that stays in its cell takes no hold and enters nothing");
        (node.hold, node.inside) = held;
        nodes::rejoin(w, d, node, rest, size, None);
        return (gain, Some(false));
    }
    // (A move that cannot happen splits nothing.)
    if moves && !can_go(w, node, c.to[o - FIXED], f, 0.0) {
        return (0.0, Some(false));
    }
    let part = nodes::part(d, node, f, f);
    let s = d.place(w, part);
    let mut m = std::mem::take(&mut d[s]);
    let gain = acts(d, &mut m, Some(node.id));
    if !m.leaving {
        back(w, d, node, s, m);
        return (gain, Some(false));
    }
    m.trace = trace(w, &m, c, k, o, 0.0);
    d.see(w, &mut m);
    d[s] = m;
    (gain, Some(true))
}

/// Part `m` of node `node` (both taken out of their slots, `m`'s at `s`) comes back to it (`nodes::rejoin`).
fn back(w: &World, d: &mut Domain, node: &mut Body, s: Handle, mut m: Body) {
    d.remove(s, &m);
    let size = m.size_moments(&w.chem);
    nodes::rejoin(w, d, node, nodes::Lot::of(&mut m), size, None);
}

/// The part act body `id` queued in a batch (`outbox`), run whole on it and on the part as they are now: verb `on.0`
/// to part `on.1` if that part still exists, lies in the body's cell and the act is still possible (`bond::possible`),
/// or a part of the shape given formed; the body pays as it acts, and does nothing if it cannot. A part of node `node` then comes back to it if it holds
/// nothing, is inside nothing, and the node is in its cell and not leaving it; else it lives on as a body.
pub(crate) fn land_act(w: &mut World, id: u64, on: Result<(usize, NodeId), [f32; 2]>, node: Option<u64>) {
    let Some(s) = w.life.find(id) else { return };
    let mut b = std::mem::take(&mut w.life[s]);
    let i = b.cell as usize;
    w.in_domain(i, |w, d| d.moves.open(w, i, Node::new(Class::Body, s.slot as usize)));
    let here = |v: usize, p: usize| bond::cell(w, p) == b.cell as usize && bond::possible(w, &b, v, p, b.volume(&w.chem, b.temp(&w.chem)) / b.n());
    match on.map(|(v, p)| (v, bond::part(w, p).filter(|&p| here(v, p)))) {
        Err(shape) => bond::form(w, &mut b, shape),
        Ok((v, Some(p))) => bond::act(w, &mut b, v, p),
        _ => {}
    }
    b.leaving = false;
    let its = node.and_then(|n| w.life.find(n)).filter(|&t| w.life[t].cell == b.cell && !w.life[t].leaving && b.hold.is_none() && b.inside.is_none());
    w.in_domain(i, move |w, d| {
        // Paying can leave nothing of a member: it dies.
        if b.empty_at(b.mass(&w.chem)) {
            return die(w, d, s, b, LACK);
        }
        let Some(t) = its else {
            d.see(w, &mut b);
            d[s] = b;
            return;
        };
        let mut o = std::mem::take(&mut d[t]);
        back(w, d, &mut o, s, b);
        d.see(w, &mut o);
        d[t] = o;
    });
}
