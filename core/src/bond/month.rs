//! The monthly step of parts: rot and heat, loads and failure, floating and drifting.

use super::{at, fall_apart, holder, members, move_structure, restructure, root, sense::liquid, unlink, verbs::{bears, torn_from}, GROUND};
use crate::{change::{frac, open_rot}, chem::Phase, life::{self, Chunk}, rng::{Purpose, Rng}, space::COMPASS, store::{Class, Node}, World};

/// Liquid shallower than this, m, pushes no part.
const STILL_DEPTH: f64 = 0.01;

/// Monthly: each chunk on the pool (`life::run`), cell by cell, the turns of the parts in the order of their cell's
/// list: what of its walls rots or is no longer solid goes to the ground, and it takes the ground's temperature, on the
/// chunk's view of the store; a part whose walls are gone falls apart (`fall_apart`); then each structure of the cell,
/// in the order of its root, finds its numbers again and bears its load (`load_one`). Last the structures that drift
/// move to their neighbour cells, in the order of the cells.
pub fn step(w: &mut World) {
    if w.life.cells.iter().all(|c| c.parts.is_empty()) {
        return;
    }
    let dt = w.cfg.calendar().month as f64;
    let load = w.life.cells.iter().map(|c| c.parts.len()).sum();
    let drifts = life::run(w, vec![Vec::new(); life::chunks(w)], load, |_| (), |w, d, drifts, _| {
        let cells: Vec<usize> = d.cells().filter(|&i| !d.at(i).parts.is_empty()).collect();
        for i in cells {
            turns(w, d, i, dt);
            let roots: Vec<u64> = d.at(i).parts.iter().filter(|p| p.root == p.id).map(|p| p.id).collect();
            for r in roots {
                let Some(x) = at(d.at(i), r) else { continue };
                restructure(w, d.at_mut(i), x);
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

/// The turns of the parts of cell `i` of chunk `d` over `dt` s, in their order (`rot_and_warm`); a part whose walls are
/// gone falls apart.
fn turns(w: &World, d: &mut Chunk, i: usize, dt: f64) {
    let ids: Vec<u64> = d.at(i).parts.iter().map(|p| p.id).collect();
    for id in ids {
        let Some(x) = at(d.at(i), id) else { continue };
        d.moves.open(w, i, Node::new(Class::Part, x));
        let mut p = std::mem::take(&mut d.at_mut(i).parts[x]);
        let gone = rot_and_warm(w, d, &mut p, i, dt);
        d.at_mut(i).parts[x] = p;
        if gone {
            fall_apart(w, d, i, x);
        }
    }
}

/// Over `dt` s, the share of the walls of part `p` of cell `i` that the open world's decomposers rot at the ground's
/// temperature and moisture (`change::open_rot`) goes to its ground with its heat, where Change rots it on, as do the
/// walls no longer solid; the part takes the ground's temperature. Returns whether its walls are under `min_kg`: it
/// falls apart.
fn rot_and_warm(w: &World, d: &mut Chunk, p: &mut super::Part, i: usize, dt: f64) -> bool {
    let (ns, chem, tg) = (w.chem.ns(), &w.chem, w.t_ground_last(i));
    let (t, moist) = (p.e / p.heat_cap(chem).max(1e-9), || w.scratch.view().moist(i, &w.cfg.phys));
    for k in 0..ns {
        if p.wall[k] <= 0.0 {
            continue;
        }
        let off = if chem.phase_in(k, t, |j| p.wall[j]) != Phase::Solid { 1.0 } else { frac(open_rot(chem, k, tg, moist) * dt) };
        if off > 0.0 {
            let (a, q) = (off * p.wall[k], off * p.wall[k] * chem.subs[k].cp * t);
            (p.e, p.wall[k]) = (p.e - q, p.wall[k] - a);
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
