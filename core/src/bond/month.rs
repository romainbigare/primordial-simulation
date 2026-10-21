//! The monthly step of parts: heat, Change, loads and failure, floating and drifting.

use super::{cell, dissolve, holder, id, members, move_structure, restructure, sense::liquid, unlink, verbs::{bears, torn_from}, GROUND};
use crate::{change::{send_gas, GasMove, Reactor}, chem::Phase, ground::share, rng::{Purpose, Rng}, space::COMPASS, store::{Class, Node}, AIR, World};

/// Parts react with the air every this many months.
const REACT_MONTHS: u64 = 6;
/// Liquid shallower than this, m, pushes no part.
const STILL_DEPTH: f64 = 0.01;

/// Monthly: heat, Change, load and failure, floating and drifting.
pub fn step(w: &mut World) {
    if w.parts.parts.iter().all(|p| !p.alive) {
        return;
    }
    let (month, ns) = (w.cfg.calendar().month, w.chem.ns());
    let react = w.month().is_multiple_of(REACT_MONTHS);
    let mut rc = Reactor::new(&w.chem, &w.cfg);
    let mut pool = vec![0.0; ns];
    let dt = (REACT_MONTHS * month) as f64;
    for s in 0..w.parts.parts.len() {
        if !w.parts.parts[s].alive {
            continue;
        }
        // What it moves to and from the store is gathered and booked once, at the end of its turn.
        w.open_moves(cell(w, s), Node::new(Class::Part, s));
        if react {
            react_part(w, &mut rc, s, &mut pool, dt);
        }
        settle(w, s);
        w.book_moves();
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

/// Part `s` reacts with its share of its cell's air for `dt` s and settles the gas as a ground node does
/// (`change::send_gas`); the ground under it pays the gas's latent heat, and gas it cannot pay for goes to the ground.
fn react_part(w: &mut World, rc: &mut Reactor, s: usize, pool: &mut [f64], dt: f64) {
    let (na, i) = (w.mesh.level(AIR).n(), cell(w, s));
    let j = w.mesh.up(i, 0, AIR);
    let (air_share, lift, ta) = (w.mesh.share(i, AIR), w.lift(i), w.t_air(j));
    let chem = &w.chem;
    let p = &mut w.parts.parts[s];
    pool.iter_mut().enumerate().for_each(|(k, p)| *p = w.store.amounts(Class::Air)[k * na + j] * air_share);
    let fixed: f64 = p.fill.iter().zip(&chem.subs).map(|(a, x)| a * x.cp).sum();
    // It lies on its cell's ground, as moist as that.
    rc.moist = w.scratch.view().moist(i, w.cfg.phys.field);
    rc.react(chem, &mut p.wall, Some(pool), &mut p.e, fixed, None, 0.0, dt);
    let gas = GasMove::of(chem, &rc.dpool, ta);
    let c = p.heat_cap(chem);
    let q_out = p.e.max(0.0) * gas.c_out / (c + gas.c_out).max(1e-9);
    p.e += gas.q_in - q_out;
    w.moves.give(&w.chem, Node::air(j), 0, 0.0, q_out);
    w.moves.take(&w.chem, Node::air(j), 0, 0.0, gas.q_in);
    w.moves.give(&w.chem, Node::ground(i), 0, 0.0, gas.back);
    let cap = w.heat_cap_ground(i);
    let paid = share(w.take_from_ground(i, (0, 0.0, 0.0), gas.pay, cap).1, gas.pay);
    // The heat of the gas that goes to the ground goes with it.
    w.moves.take(&w.chem, Node::air(j), 0, 0.0, q_out * (1.0 - paid));
    w.moves.give(&w.chem, Node::ground(i), 0, 0.0, q_out * (1.0 - paid));
    send_gas(&rc.dpool, paid, |k, sent, kept| {
        w.moves.give(&w.chem, Node::ground(i), k, kept, 0.0);
        if sent != 0.0 {
            w.moves.gas(&w.chem, k, sent, lift);
        }
    });
}

/// Walls of part `s` no longer solid run off; the part takes the ground's temperature; walls under `min_kg` fall apart.
fn settle(w: &mut World, s: usize) {
    let (ns, i) = (w.chem.ns(), cell(w, s));
    let (tg, chem) = (w.t_ground_last(i), &w.chem);
    let p = &mut w.parts.parts[s];
    let t = p.e / p.heat_cap(chem).max(1e-9);
    for k in 0..ns {
        if p.wall[k] > 0.0 && chem.phase_in(k, t, |j| p.wall[j]) != Phase::Solid {
            let q = p.wall[k] * chem.subs[k].cp * t;
            w.moves.give(chem, Node::ground(i), k, p.wall[k], q);
            (p.e, p.wall[k]) = (p.e - q, 0.0);
        }
    }
    let (e1, e0) = (p.heat_cap(chem) * tg, p.e);
    let cap = w.heat_cap_ground(i);
    let got = w.take_from_ground(i, (0, 0.0, 0.0), e1 - e0, cap).1;
    let p = &mut w.parts.parts[s];
    p.e = if got == e1 - e0 { e1 } else { e0 + got };
    let chem = &w.chem;
    let gone = p.walls(chem).mass < w.cfg.phys.min_kg;
    p.resize(chem);
    if gone {
        dissolve(w, s);
    }
}

/// Load one structure: weights less buoyancy, pushes of liquid and bodies; break what is overloaded; float and drift.
fn load(w: &mut World, r: usize) {
    let (list, i, g) = (members(w, r), cell(w, r), w.cfg.gravity);
    let (depth, rho, molv) = liquid(w, i);
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
        if wt[x] + push[x] > bears(w, &w.life.bodies[o]) {
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
