//! A node's decision as a spread of actions (handover 7.1): its members spread over the options by the chances its
//! brain gives them, the chances an individual draws from, and each part acts as so many members would.

use super::go::can_go;
use super::*;
use crate::body::{die, LACK};
use crate::life::Handle;

use crate::nodes;

/// Node `node` (taken out of its slot) spreads its members over the options (`members`), and each part acts as so many members together (`part_acts`; the
/// moves last, so the others act where the node is). Its brain learns as from one member drawn among those who stayed
/// (`trace`): from what that member did, and how much more its part's action raised their well-being than the mean of
/// the members.
pub(super) fn spread(w: &World, d: &mut Chunk, node: &mut Body, c: &Choice, k: &Kit, rng: &mut Rng) {
    let q = top_chances(c);
    let x = members(node, &q, rng);
    // The members of each option in the node after acting, and the change of their well-being.
    let (mut kept, mut gain, mut acted) = ([0.0; OPTS], [0f32; OPTS], 0.0);
    // (Until a part's acting changes the node, each member's well-being is the node's at the decision, `Body::wb`.)
    let mut fresh = true;
    for o in (0..FIXED).chain(TO_BODY..OPTS).chain(FIXED..TO_BODY).filter(|&o| x[o] > 0.0) {
        let (g, left) = if o == REST { (0.0, Some(false)) } else { part_acts(w, d, node, (c, &q), k, (o, x[o]), (&mut fresh, rng)) };
        (gain[o], kept[o], acted) = (g, if left == Some(true) { 0.0 } else { x[o] }, acted + x[o]);
        let Some(_) = left else { break };
    }
    // The members who did nothing (none drawn for them, or their option not reached) rest, at no gain, and count in
    // the mean as those who rested.
    kept[REST] += (node.count - acted).max(0.0);
    let mean = kept.iter().zip(&gain).map(|(n, &g)| n * g as f64).sum::<f64>() / kept.iter().sum::<f64>().max(f64::MIN_POSITIVE);
    let mut u = rng.f64() * kept.iter().sum::<f64>();
    let a = (0..OPTS).find(|&o| { u -= kept[o]; u < 0.0 }).unwrap_or(REST);
    node.traced = trace(w, node, c, (&q, a, gain[a] - mean as f32), k, d.brains(node.cell as usize).mind(node.brain, k.s).trace);
}

/// `count` × `chance` members, a part of less than one member being one member with that chance.
fn share(count: f64, chance: f64, rng: &mut Rng) -> f64 {
    match count * chance {
        x if x > 0.0 && x < 1.0 => (rng.f64() < x) as u8 as f64,
        x => x,
    }
}

/// The most options a node spreads its members over at a decision (a speed setting: the likeliest, the moves counted as
/// one); its members of the others rest.
const MOST_ACTS: usize = 4;

/// The chances of the decision `c` over its `MOST_ACTS` likeliest options (the moves together one), the others none,
/// brought back to a sum of 1: the chances a node's members spread by, and its brain learns from (`trace`).
fn top_chances(c: &Choice) -> [f32; OPTS] {
    let moves = FIXED..TO_BODY;
    let sum: f32 = c.p[moves.clone()].iter().sum();
    let chance = |o: usize| if moves.contains(&o) { sum } else { c.p[o] };
    // The likeliest, kept in order (the larger chance first, the lower option of equals) as each option is seen.
    let mut top = [(0.0f32, usize::MAX); MOST_ACTS];
    for o in (0..OPTS).filter(|&o| !moves.contains(&o) || o == FIXED) {
        let x = chance(o);
        if x > 0.0 && x > top[MOST_ACTS - 1].0 {
            let at = top.iter().position(|e| x > e.0).unwrap_or(MOST_ACTS - 1);
            top.copy_within(at..MOST_ACTS - 1, at + 1);
            top[at] = (x, o);
        }
    }
    let kept: f32 = top.iter().map(|e| e.0).sum();
    let mut q = [0.0; OPTS];
    for &(_, o) in top.iter().filter(|e| e.1 != usize::MAX) {
        let span = if o == FIXED { moves.clone() } else { o..o + 1 };
        span.for_each(|x| q[x] = c.p[x] / kept);
    }
    q
}

/// The members of node `node` that do each option by the chances `p` (`top_chances`): count × chance (`share`). The
/// members that move are one group, count × the sum of the chances to go in every direction, and the group takes the
/// one direction drawn by its chance over that sum, so its option holds the group and the other directions none. A
/// group that would leave less than one member behind is the whole node, with the chance that keeps the mean of the
/// movers at count × chance.
fn members(node: &Body, p: &[f32; OPTS], rng: &mut Rng) -> [f64; OPTS] {
    let moves = FIXED..TO_BODY;
    let sum: f64 = p[moves.clone()].iter().map(|&p| p as f64).sum();
    let mut x: [f64; OPTS] = std::array::from_fn(|o| if moves.contains(&o) { 0.0 } else { share(node.count, p[o] as f64, rng) });
    if sum <= 0.0 {
        return x;
    }
    let chance = sum.min(1.0);
    // (If fewer than one member would stay, the node goes whole with the chance that makes the movers count × chance.)
    let group = match node.count * chance {
        g if g > 0.0 && node.count - g < 1.0 => if rng.f64() < chance { node.count } else { 0.0 },
        _ => share(node.count, chance, rng),
    };
    if group > 0.0 {
        // The direction: the chance of each over their sum.
        let mut u = rng.f64() * sum;
        let way = moves.clone().filter(|&o| p[o] > 0.0).find(|&o| { u -= p[o] as f64; u < 0.0 });
        x[way.unwrap_or_else(|| moves.rev().find(|&o| p[o] > 0.0).expect("a group of movers has a direction"))] = group;
    }
    x
}

/// `x` members of node `node` (taken out of its slot) that stay in its cell act by `act` as the node itself: the whole
/// node if less than one member would be left, else holding and inside nothing while the others wait aside with their
/// share of its amounts (`nodes::Lot::split`), and then come back (`nodes::rejoin`: amounts and sizes by members).
fn stays<R>(d: &mut Chunk, node: &mut Body, x: f64, act: impl FnOnce(&mut Chunk, &mut Body) -> R) -> R {
    if node.count - x < 1.0 {
        return act(d, node);
    }
    let f = x / node.count;
    let rest = nodes::Lot::split(node, 1.0 - f, 1.0 - f);
    let held = (node.hold.take(), node.inside.take());
    let out = act(d, node);
    debug_assert!(node.hold.is_none() && node.inside.is_none(), "an action that stays in its cell takes no hold and enters nothing");
    (node.hold, node.inside) = held;
    nodes::rejoin(node, rest);
    out
}

/// `x` members of node `node` (taken out of its slot) do option `o`. Members that stay in its cell (any option but a
/// move or a bond) act as the node itself (`stays`). Members that move or bond are placed as a body of their own
/// (`nodes::part`); unless they queued a move or a part act (`life::Entry`), they then come back (`nodes::rejoin`); else
/// they live on as a body, leaving the node (`Body::leaving`), with the trace of what they did: those who move join the
/// first node of their kind and stage where they land (`choose::land_move`), those who act on a part come back after it
/// if they can (`land_act`). The node acts itself if less than one member would be left. `q`: the node's chances
/// (`top_chances`), which a part that leaves learns from. While `fresh`, no part's acting has changed the node yet, so
/// the members' well-being before they act is the node's at its decision (`Body::wb`: a split leaves each member's
/// share alike); it is false once one has. Returns the change of their well-being (none worked out for a part that
/// leaves: it counts in no mean and is never the member drawn, `spread`), and whether they left the node (none: it moved
/// whole).
fn part_acts(w: &World, d: &mut Chunk, node: &mut Body, (c, q): (&Choice, &[f32; OPTS]), k: &Kit, (o, x): (usize, f64), (fresh, rng): (&mut bool, &mut Rng)) -> (f32, Option<bool>) {
    let (moves, bond) = ((FIXED..TO_BODY).contains(&o), (FORM..FIXED).contains(&o) || o >= TO_PART);
    let first = *fresh;
    let mut acts = |d: &mut Chunk, b: &mut Body, part_of: Option<u64>| {
        // (A part that forms a part or acts on one always leaves, `act_as`.)
        let before = if first || (part_of.is_some() && (o == FORM || o >= TO_PART)) { b.wb } else { (k.wb)(w, b) };
        act_as(w, d, b, c, k, (o, part_of), rng);
        if part_of.is_some() && b.leaving { 0.0 } else { (k.wb)(w, b) - before }
    };
    if !(moves || bond) {
        *fresh = false;
        return (stays(d, node, x, |d, b| acts(d, b, None)), Some(false));
    }
    if node.count - x < 1.0 {
        *fresh = false;
        return (acts(d, node, None), (!node.leaving).then_some(false));
    }
    let f = x / node.count;
    // (A move that cannot happen splits nothing.)
    if moves && !can_go(w, node, c.to[o - FIXED], f, 0.0) {
        return (0.0, Some(false));
    }
    let part = nodes::part(node, f, f);
    let s = d.place_part(w, part);
    let mut m = std::mem::take(&mut d[s]);
    let gain = acts(d, &mut m, Some(node.id));
    if !m.leaving {
        back(d, node, s, m);
        *fresh = false;
        return (gain, Some(false));
    }
    // (It lives on as a body: a brain of its own, the node's, with the trace of what it did. Movers carry its numbers in
    // their move, `Entry::Go`, which lands them where they go.)
    let brains = d.brains(m.cell as usize);
    if moves {
        let mut v = brains.all(node.brain).to_vec();
        m.traced = trace(w, &m, c, (q, o, 0.0), k, crate::life::Brains::split(&mut v, k.s).trace);
        let Some(Entry::Go { brain, .. }) = d.out.last_mut() else { unreachable!("a mover that leaves queued its move last") };
        *brain = v;
    } else {
        m.brain = brains.copy(node.brain);
        m.traced = trace(w, &m, c, (q, o, 0.0), k, brains.mind(m.brain, k.s).trace);
    }
    m.stale = true;
    d[s] = m;
    (gain, Some(true))
}

/// Part `m` of node `node` (both taken out of their slots, `m`'s at `s`) comes back to it (`nodes::rejoin`).
fn back(d: &mut Chunk, node: &mut Body, s: Handle, mut m: Body) {
    d.remove(s, &m);
    nodes::rejoin(node, nodes::Lot::of(&mut m));
}

/// The part act of the body at `s` of chunk `d` queued in a batch (`life::Entry`), run whole on it and on the part as
/// they are now, in the acts' landing (`life::land`): verb `on.0` to part `on.1` if that part still lies in the body's
/// cell and the act is still possible (`bond::possible`), or a part of the shape given formed; the body pays as it
/// acts, and does nothing if it cannot. A part of node `node` then comes back to it if it holds nothing, is inside
/// nothing, and the node is in its cell and not leaving it; else it lives on as a body.
pub(crate) fn land_act(w: &World, d: &mut Chunk, s: Handle, on: Result<(usize, u64), [f32; 3]>, node: Option<u64>) {
    let mut b = std::mem::take(&mut d[s]);
    let i = b.cell as usize;
    d.moves.open(w, i, crate::store::Node::new(crate::store::Class::Body, s.slot as usize));
    let vol = b.volume(&w.chem, b.temp(&w.chem)) / b.n();
    match on.map(|(v, p)| (v, bond::at(d.at(i), p).filter(|&p| bond::possible(d.at(i), &b, v, p, vol)))) {
        Err(shape) => bond::form(w, d, &mut b, shape),
        Ok((v, Some(p))) => bond::act(w, d, &mut b, v, p),
        _ => {}
    }
    b.leaving = false;
    // Paying can leave nothing of a member: it dies, its remains to the ground.
    if b.empty_at(b.mass(&w.chem)) {
        return die(w, d, s, b, LACK);
    }
    let its = node.and_then(|n| d.cell(i).find(|&t| d[t].id == n)).filter(|&t| !d[t].leaving && b.hold.is_none() && b.inside.is_none());
    let Some(t) = its else {
        b.stale = true;
        d[s] = b;
        return;
    };
    let mut o = std::mem::take(&mut d[t]);
    back(d, &mut o, s, b);
    o.stale = true;
    d[t] = o;
}
