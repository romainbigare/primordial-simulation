//! The monthly step of parts: heat, Change, loads and failure, floating and drifting.

use super::{cell, dissolve, fall_apart, holder, id, members, move_structure, restructure, sense::liquid, unlink, verbs::{bears, torn_from}, Part, GROUND};
use crate::{change::{send_gas, GasMove, Reactor}, chem::Phase, ground::share, life::{self, Chunk}, rng::{Purpose, Rng}, space::COMPASS, store::{Class, Node}, AIR, World};

/// Parts react with the air every this many months.
const REACT_MONTHS: u64 = 6;
/// Liquid shallower than this, m, pushes no part.
const STILL_DEPTH: f64 = 0.01;

/// Monthly: heat, Change, load and failure, floating and drifting.
pub fn step(w: &mut World) {
    if w.parts.parts.iter().all(|p| !p.alive) {
        return;
    }
    let react = w.month().is_multiple_of(REACT_MONTHS);
    for s in turns(w, react) {
        dissolve(w, s);
    }
    for r in 0..w.parts.parts.len() {
        if w.parts.parts[r].alive && w.parts.parts[r].root as usize == r {
            if react {
                restructure(w, r);
            }
            load(w, r);
        }
    }
}

/// A part's turn: the part (taken out of its slot while its chunk runs), its slot and cell, and whether its walls are
/// gone.
struct Turn {
    slot: usize,
    cell: usize,
    part: Part,
    gone: bool,
}

/// The turns of all parts: heat, and every `REACT_MONTHS` months the reactions with the air. Each chunk takes the turns
/// of the parts of its cells, cell by cell and in a cell in slot order, on its view of the store (`store::Moves`), on
/// the pool (`life::run`), whose moves are booked when the chunks end. A part whose walls are gone falls apart in its
/// turn (`fall_apart`). Returns the slots of the parts that fall apart, in order: they are freed one by one
/// (`dissolve`), as that unbonds other parts.
fn turns(w: &mut World, react: bool) -> Vec<usize> {
    let dt = (REACT_MONTHS * w.cfg.calendar().month) as f64;
    // The parts by their cell, then by slot.
    let mut order: Vec<(usize, usize)> = (0..w.parts.parts.len()).filter(|&s| w.parts.parts[s].alive).map(|s| (cell(w, s), s)).collect();
    order.sort_unstable();
    let bounds = life::bounds(w);
    let mut work: Vec<Vec<Turn>> = (1..bounds.len()).map(|_| Vec::new()).collect();
    for (cell, slot) in order {
        work[bounds.partition_point(|&b| b <= cell) - 1].push(Turn { slot, cell, part: std::mem::take(&mut w.parts.parts[slot]), gone: false });
    }
    // A turn that only sheds and warms costs about a microsecond, less than a pool call: it counts no work, so the runner keeps
    // the chunks on the calling thread; the reactions are the work.
    let load = if react { work.iter().map(Vec::len).sum() } else { 0 };
    let init = |w: &World| react.then(|| (Reactor::new(&w.chem, &w.cfg), vec![0.0; w.chem.ns()]));
    let work = life::run(w, work, load, init, |w, d, mine, scratch| {
        for t in mine.iter_mut() {
            d.moves.open(w, t.cell, Node::new(Class::Part, t.slot));
            if let Some((rc, pool)) = scratch {
                react_part(w, d, rc, (&mut t.part, t.cell), pool, dt);
            }
            t.gone = shed_and_warm(w, d, &mut t.part, t.cell);
            if t.gone {
                fall_apart(w, d, t.slot, &mut t.part);
            }
        }
    });
    let mut gone = Vec::new();
    for t in work.into_iter().flatten() {
        w.parts.parts[t.slot] = t.part;
        if t.gone {
            gone.push(t.slot);
        }
    }
    gone.sort_unstable();
    gone
}

/// Part `p` of cell `i` reacts with its share of its cell's air for `dt` s and settles the gas as a ground node does
/// (`change::send_gas`); the ground under it pays the gas's latent heat, and gas it cannot pay for goes to the ground.
fn react_part(w: &World, d: &mut Chunk, rc: &mut Reactor, (p, i): (&mut Part, usize), pool: &mut [f64], dt: f64) {
    let j = w.mesh.up(i, 0, AIR);
    let (lift, ta) = (w.lift(i), d.moves.t_air(w, j));
    let chem = &w.chem;
    pool.iter_mut().enumerate().for_each(|(k, p)| *p = d.moves.amount(w, Node::air(j), k));
    let fixed: f64 = p.fill.iter().zip(&chem.subs).map(|(a, x)| a * x.cp).sum();
    // It lies on its cell's ground, as moist as that.
    rc.moist = w.scratch.view().moist(i, &w.cfg.phys);
    rc.react(chem, &mut p.wall, Some(pool), &mut p.e, fixed, dt);
    let gas = GasMove::of(chem, &rc.dpool, ta);
    let c = p.heat_cap(chem);
    let q_out = p.e.max(0.0) * gas.c_out / (c + gas.c_out).max(1e-9);
    p.e += gas.q_in - q_out;
    let m = &mut d.moves;
    m.give(w, Node::air(j), 0, 0.0, q_out);
    m.take(w, Node::air(j), 0, 0.0, gas.q_in);
    m.give(w, Node::ground(i), 0, 0.0, gas.back);
    let cap = m.heat_cap_ground(w, i);
    let paid = share(m.take_from_ground(w, i, (0, 0.0, 0.0), gas.pay, cap).1, gas.pay);
    // The heat of the gas that goes to the ground goes with it.
    m.take(w, Node::air(j), 0, 0.0, q_out * (1.0 - paid));
    m.give(w, Node::ground(i), 0, 0.0, q_out * (1.0 - paid));
    send_gas(&rc.dpool, paid, |k, sent, kept| {
        m.give(w, Node::ground(i), k, kept, 0.0);
        if sent != 0.0 {
            m.gas(w, k, sent, lift);
        }
    });
}

/// Walls of part `p` of cell `i` no longer solid run off; the part takes the ground's temperature. Returns whether its
/// walls are under `min_kg`: it falls apart (`dissolve`).
fn shed_and_warm(w: &World, d: &mut Chunk, p: &mut Part, i: usize) -> bool {
    let (ns, chem, tg) = (w.chem.ns(), &w.chem, w.t_ground_last(i));
    let t = p.e / p.heat_cap(chem).max(1e-9);
    for k in 0..ns {
        if p.wall[k] > 0.0 && chem.phase_in(k, t, |j| p.wall[j]) != Phase::Solid {
            let (a, q) = (p.wall[k], p.wall[k] * chem.subs[k].cp * t);
            (p.e, p.wall[k]) = (p.e - q, 0.0);
            d.moves.give(w, Node::ground(i), k, a, q);
        }
    }
    let (e1, e0) = (p.heat_cap(chem) * tg, p.e);
    let cap = d.moves.heat_cap_ground(w, i);
    let got = d.moves.take_from_ground(w, i, (0, 0.0, 0.0), e1 - e0, cap).1;
    p.e = if got == e1 - e0 { e1 } else { e0 + got };
    let gone = p.walls(chem).mass < w.cfg.phys.min_kg;
    p.resize(chem);
    gone
}

/// Load one structure: weights less buoyancy, pushes of liquid and bodies; break what is overloaded; float and drift.
fn load(w: &mut World, r: usize) {
    let (list, i, g) = (members(w, r), cell(w, r), w.cfg.gravity);
    let (depth, rho, molv) = liquid(w, i, w.stock(Node::ground(i)));
    let flux = w.liquid_flux.get(i).copied().unwrap_or(0.0) as f64;
    // The liquid's speed through the cell, m/s: its flux over the mean width of the borders it crosses times its depth.
    let cells = w.mesh.cells();
    let (sum, borders) = cells.around(i).0.iter().map(|&l| cells.width()[l as usize]).filter(|&wd| wd > 0.0).fold((0.0, 0.0), |(s, k), wd| (s + wd, k + 1.0));
    let width = sum / borders;
    let v = if depth > STILL_DEPTH { flux / (molv * width * depth) } else { 0.0 };
    let bump = std::mem::take(&mut w.parts.parts[r].push) as f64 / list.len() as f64;
    let (mut wt, mut push, mut lift) = (vec![0.0; list.len()], vec![0.0; list.len()], 0.0);
    for (x, &p) in list.iter().enumerate() {
        let pt = &w.parts.parts[p];
        let face = (pt.size[0] * pt.size[1]) as f64;
        let buoy = rho * g * pt.outer().min(face * depth);
        lift += buoy;
        wt[x] = (pt.mass(&w.chem) * g - buoy).max(0.0);
        push[x] = 0.5 * rho * v * v * face + bump;
    }
    let s = w.parts.parts[r].s;
    let afloat = !s.anchored && depth > 0.0 && lift >= s.mass * g;
    w.parts.parts[r].s.afloat = afloat;
    let held = list.iter().copied().find(|&p| holder(w, p).is_some());
    // From the supports out along the bonds, each carrying what hangs beyond it; a ground bond carries the pushes too.
    let mut order: Vec<usize> = (0..list.len()).filter(|&x| w.parts.bonds.of(list[x]).any(|l| l.0 == GROUND)).collect();
    if order.is_empty() {
        order.extend(held.and_then(|h| list.iter().position(|&p| p == h)));
    }
    let mut up = vec![usize::MAX; list.len()];
    let mut q = 0;
    while q < order.len() {
        for (o, _) in w.parts.bonds.of(list[order[q]]) {
            if let Some(c) = list.iter().position(|&p| p as u32 == o).filter(|&c| !order.contains(&c)) {
                up[c] = order[q];
                order.push(c);
            }
        }
        q += 1;
    }
    for &c in order.iter().rev() {
        if up[c] != usize::MAX {
            (wt[up[c]], push[up[c]]) = (wt[up[c]] + wt[c], push[up[c]] + push[c]);
        }
    }
    let mut breaks = Vec::new();
    for &c in &order {
        for (o, st) in w.parts.bonds.of(list[c]) {
            let hang = if o == GROUND { push[c] } else if up[c] != usize::MAX && list[up[c]] as u32 == o { wt[c] + push[c] } else { 0.0 };
            if hang > st as f64 {
                breaks.push((list[c], o));
            }
        }
    }
    for (a, o) in breaks {
        if w.parts.bonds.of(a).any(|l| l.0 == o) {
            unlink(w, a, o, true);
        }
    }
    // The holder of a structure not resting on the ground bears what hangs on its part by its own force, or lets go.
    let x = held.filter(|_| !s.anchored).and_then(|h| list.iter().position(|&p| p == h));
    if let Some((x, o)) = x.and_then(|x| holder(w, list[x]).map(|o| (x, o))) {
        if wt[x] + push[x] > bears(w, &w.life[o]) {
            torn_from(w, o);
        }
    }
    // Afloat and free: drift to the neighbour whose liquid stands lowest.
    if !afloat || held.is_some() || v <= 0.0 || w.parts.parts[r].root as usize != r {
        return;
    }
    let mut rng = Rng::of_node(w.seed, w.time, id(w, r), Purpose::Bond);
    let head = |j: usize| w.life.env[j][2] + w.life.env[j][1];
    let cells = w.mesh.cells();
    let near = (0..COMPASS).filter_map(|d| cells.way(i, d).map(|e| cells.adj_cell()[e] as usize));
    let to = near.filter(|&j| head(j) < head(i)).min_by(|&a, &b| head(a).total_cmp(&head(b)));
    // It crosses the cell's width at the liquid's speed.
    if let Some(j) = to.filter(|_| rng.f64() < v * w.cfg.calendar().month as f64 / width) {
        move_structure(w, r, j);
    }
}
