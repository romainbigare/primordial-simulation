//! The monthly step of parts: heat, Change, loads and failure, floating and drifting.

use super::{at, fall_apart, holder, members, move_structure, restructure, root, sense::liquid, unlink, verbs::{bears, torn_from}, GROUND};
use crate::{change::{send_gas, GasMove, Reactor}, chem::Phase, ground::share, life::{self, Chunk}, rng::{Purpose, Rng}, space::COMPASS, store::{Class, Node}, AIR, World};

/// Parts react with the air every this many months.
const REACT_MONTHS: u64 = 6;
/// Liquid shallower than this, m, pushes no part.
const STILL_DEPTH: f64 = 0.01;

/// Monthly: each chunk on the pool (`life::run`), cell by cell, the turns of the parts in the order of their cell's
/// list: heat, and every `REACT_MONTHS` months the reactions with the air, on the chunk's view of the store; a part
/// whose walls are gone falls apart (`fall_apart`); then the load of each structure of the cell, in the order of its
/// root (`load`). Last the structures that drift move to their neighbour cells, in the order of the cells (`drift`).
pub fn step(w: &mut World) {
    if w.life.cells.iter().all(|c| c.parts.is_empty()) {
        return;
    }
    let react = w.month().is_multiple_of(REACT_MONTHS);
    let dt = (REACT_MONTHS * w.cfg.calendar().month) as f64;
    // A turn that only sheds and warms costs about a microsecond, less than a pool call: it counts no work, so the runner keeps
    // the chunks on the calling thread; the reactions are the work.
    let load = if react { w.life.cells.iter().map(|c| c.parts.len()).sum() } else { 0 };
    let init = |w: &World| react.then(|| (Reactor::new(&w.chem, &w.cfg), vec![0.0; w.chem.ns()]));
    let drifts = life::run(w, vec![Vec::new(); life::chunks(w)], load, init, |w, d, drifts, scratch| {
        let cells: Vec<usize> = d.cells().filter(|&i| !d.at(i).parts.is_empty()).collect();
        for i in cells {
            turns(w, d, i, scratch, dt);
            let roots: Vec<u64> = d.at(i).parts.iter().filter(|p| p.root == p.id).map(|p| p.id).collect();
            for r in roots {
                let Some(x) = at(d.at(i), r) else { continue };
                if react {
                    restructure(w, d.at_mut(i), x);
                }
                if let Some(j) = load_one(w, d, i, r) {
                    drifts.push((i, r, j));
                }
            }
        }
    });
    for (i, r, j) in drifts.into_iter().flatten() {
        move_structure(w, i, r, j);
    }
}

/// The turns of the parts of cell `i` of chunk `d`, in their order: heat, and the reactions with the air if `scratch`
/// holds a reactor; a part whose walls are gone falls apart.
fn turns(w: &World, d: &mut Chunk, i: usize, scratch: &mut Option<(Reactor, Vec<f64>)>, dt: f64) {
    let ids: Vec<u64> = d.at(i).parts.iter().map(|p| p.id).collect();
    for id in ids {
        let Some(x) = at(d.at(i), id) else { continue };
        d.moves.open(w, i, Node::new(Class::Part, x));
        let mut p = std::mem::take(&mut d.at_mut(i).parts[x]);
        if let Some((rc, pool)) = scratch {
            react_part(w, d, rc, (&mut p, i), pool, dt);
        }
        let gone = shed_and_warm(w, d, &mut p, i);
        d.at_mut(i).parts[x] = p;
        if gone {
            fall_apart(w, d, i, x);
        }
    }
}

/// Part `p` of cell `i` reacts with its share of its cell's air for `dt` s and settles the gas as a ground node does
/// (`change::send_gas`); the ground under it pays the gas's latent heat, and gas it cannot pay for goes to the ground.
fn react_part(w: &World, d: &mut Chunk, rc: &mut Reactor, (p, i): (&mut super::Part, usize), pool: &mut [f64], dt: f64) {
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
/// walls are under `min_kg`: it falls apart.
fn shed_and_warm(w: &World, d: &mut Chunk, p: &mut super::Part, i: usize) -> bool {
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

/// Load the structure whose root is `r` in cell `i` of chunk `d`: weights less buoyancy, pushes of liquid and bodies;
/// break what is overloaded. Returns the neighbour cell it drifts to, afloat and free (moved when the chunks end).
fn load_one(w: &World, d: &mut Chunk, i: usize, r: u64) -> Option<usize> {
    let (c, g) = (d.at(i), w.cfg.gravity);
    let list = members(c, r);
    let (depth, rho, molv) = liquid(w, i, d.moves.amounts(w, Node::ground(i)));
    let flux = w.liquid_flux.get(i).copied().unwrap_or(0.0) as f64;
    // The liquid's speed through the cell, m/s: its flux over the mean width of the borders it crosses times its depth.
    let cells = w.mesh.cells();
    let (sum, borders) = cells.around(i).0.iter().map(|&l| cells.width()[l as usize]).filter(|&wd| wd > 0.0).fold((0.0, 0.0), |(s, k), wd| (s + wd, k + 1.0));
    let width = sum / borders;
    let v = if depth > STILL_DEPTH { flux / (molv * width * depth) } else { 0.0 };
    let rp = at(c, r).expect("the root");
    let bump = c.parts[rp].push as f64 / list.len() as f64;
    let (mut wt, mut push, mut lift) = (vec![0.0; list.len()], vec![0.0; list.len()], 0.0);
    for (x, &p) in list.iter().enumerate() {
        let pt = &c.parts[p];
        let face = (pt.size[0] * pt.size[1]) as f64;
        let buoy = rho * g * pt.outer().min(face * depth);
        lift += buoy;
        wt[x] = (pt.mass(&w.chem) * g - buoy).max(0.0);
        push[x] = 0.5 * rho * v * v * face + bump;
    }
    let s = c.parts[rp].s;
    let afloat = !s.anchored && depth > 0.0 && lift >= s.mass * g;
    let held = list.iter().copied().find(|&p| holder(c, p).is_some());
    // From the supports out along the bonds, each carrying what hangs beyond it; a ground bond carries the pushes too.
    let ids: Vec<u64> = list.iter().map(|&p| c.parts[p].id).collect();
    let mut order: Vec<usize> = (0..list.len()).filter(|&x| c.bonds.of(ids[x]).any(|l| l.0 == GROUND)).collect();
    if order.is_empty() {
        order.extend(held.and_then(|h| list.iter().position(|&p| p == h)));
    }
    let mut up = vec![usize::MAX; list.len()];
    let mut q = 0;
    while q < order.len() {
        for (o, _) in c.bonds.of(ids[order[q]]) {
            if let Some(x) = ids.iter().position(|&p| p == o).filter(|&x| !order.contains(&x)) {
                up[x] = order[q];
                order.push(x);
            }
        }
        q += 1;
    }
    for &x in order.iter().rev() {
        if up[x] != usize::MAX {
            (wt[up[x]], push[up[x]]) = (wt[up[x]] + wt[x], push[up[x]] + push[x]);
        }
    }
    let mut breaks = Vec::new();
    for &x in &order {
        for (o, st) in c.bonds.of(ids[x]) {
            let hang = if o == GROUND { push[x] } else if up[x] != usize::MAX && ids[up[x]] == o { wt[x] + push[x] } else { 0.0 };
            if hang > st as f64 {
                breaks.push((ids[x], o));
            }
        }
    }
    // The holder of a structure not resting on the ground bears what hangs on its part by its own force, or lets go.
    let x = held.filter(|_| !s.anchored).and_then(|h| list.iter().position(|&p| p == h));
    let torn = x.and_then(|x| holder(c, list[x]).filter(|&o| wt[x] + push[x] > bears(w, &c.bodies[o])));
    let c = d.at_mut(i);
    (c.parts[rp].push, c.parts[rp].s.afloat) = (0.0, afloat);
    for (a, o) in breaks {
        if d.at(i).bonds.of(a).any(|l| l.0 == o) {
            unlink(w, d, i, a, o, true);
        }
    }
    if let Some(o) = torn {
        torn_from(w, d, i, o);
    }
    // Afloat and free: drift to the neighbour whose liquid stands lowest.
    let c = d.at(i);
    if !afloat || held.is_some() || v <= 0.0 || at(c, r).is_none_or(|p| root(c, p) != p) {
        return None;
    }
    let mut rng = Rng::of_part(w.seed, w.time, r, Purpose::Bond);
    let head = |j: usize| w.life.env[j][2] + w.life.env[j][1];
    let near = (0..COMPASS).filter_map(|e| cells.way(i, e).map(|e| cells.adj_cell()[e] as usize));
    let to = near.filter(|&j| head(j) < head(i)).min_by(|&a, &b| head(a).total_cmp(&head(b)));
    // It crosses the cell's width at the liquid's speed.
    to.filter(|_| rng.f64() < v * w.cfg.calendar().month as f64 / width)
}
