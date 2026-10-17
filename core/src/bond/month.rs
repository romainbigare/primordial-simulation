//! The monthly step of parts: heat, Change, loads and failure, floating and drifting.

use super::sense::liquid;
use super::verbs::{bears, torn_from};
use super::{cell, dissolve, holder, id, members, move_structure, ready, restructure, unlink, GROUND};
use crate::change::{send_gas, GasMove, Reactor};
use crate::flow::Sky;
use crate::ground::share;
use crate::chem::Phase;
use crate::space::square::DIRS;
use crate::rng::{Purpose, Rng};
use crate::World;

/// Parts react with the air every this many months.
const REACT_MONTHS: u64 = 6;
/// Liquid shallower than this, m, pushes no part.
const STILL_DEPTH: f64 = 0.01;

/// Monthly: heat, Change, load and failure, floating and drifting.
pub fn step(w: &mut World) {
    ready(w);
    if w.parts.parts.iter().all(|p| !p.alive) {
        return;
    }
    let (month, ns) = (w.cfg.calendar().month, w.chem.ns());
    let react = w.month().is_multiple_of(REACT_MONTHS);
    let mut rc = Reactor::new(&w.chem, &w.cfg);
    let (mut pool, mut pool0) = (vec![0.0; ns], vec![0.0; ns]);
    let dt = (REACT_MONTHS * month) as f64;
    for s in 0..w.parts.parts.len() {
        if !w.parts.parts[s].alive {
            continue;
        }
        if react {
            react_part(w, &mut rc, s, &mut pool, &mut pool0, dt);
        }
        settle(w, s);
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

/// Part `s` reacts with its share of the air of its cell for `dt` s (`pool0` that share before, `pool` after), and settles
/// the gas with the air as a ground node does (`change::GasMove`, `change::send_gas`): gas taken brings its heat at the
/// air's temperature, gas sent leaves with its share of the part's heat by heat capacity. The ground under it pays the
/// fusion and vaporisation enthalpy of the gas sent (gas taken gives it back); gas the ground cannot pay for goes to the
/// ground, with its heat.
fn react_part(w: &mut World, rc: &mut Reactor, s: usize, pool: &mut [f64], pool0: &mut [f64], dt: f64) {
    let (n, na) = (w.grid.n(), w.grid.na());
    let i = cell(w, s);
    let j = w.grid.air_of(i);
    let (air_share, lift, ta) = (w.grid.air_share(i), w.lift(i), w.t_air(j));
    let chem = &w.chem;
    let p = &mut w.parts.parts[s];
    for (k, p0) in pool0.iter_mut().enumerate() {
        *p0 = w.air[k * na + j] * air_share;
    }
    pool.copy_from_slice(pool0);
    let fixed: f64 = p.fill.iter().zip(&chem.subs).map(|(a, x)| a * x.cp).sum();
    rc.react(chem, &mut p.wall, Some(pool), &mut p.e, fixed, None, 0.0, dt);
    let gas = GasMove::of(chem, pool, pool0, ta);
    let c = p.heat_cap(chem);
    let q_out = p.e.max(0.0) * gas.c_out / (c + gas.c_out).max(1e-9);
    p.e += gas.q_in - q_out;
    (w.e_air[j], w.e_ground[i]) = (w.e_air[j] + q_out - gas.q_in, w.e_ground[i] + gas.back);
    let paid = share(w.take_heat(i, gas.pay), gas.pay);
    // The heat of the gas that goes to the ground goes with it.
    (w.e_ground[i], w.e_air[j]) = (w.e_ground[i] + q_out * (1.0 - paid), w.e_air[j] - q_out * (1.0 - paid));
    let ground = &mut w.ground;
    let mut sky = Sky { chem: &w.chem, air: &mut w.air, e_air: &mut w.e_air, potential: &mut w.potential };
    send_gas(pool, pool0, paid, &mut sky, (j, lift), |k, kept| ground[k * n + i] += kept);
}

/// Walls of part `s` that are no longer solid run off to the ground; the part takes the ground's temperature (if the
/// ground cannot pay the heat, it ends colder); walls lighter than `Physics::min_kg` fall apart.
fn settle(w: &mut World, s: usize) {
    let (ns, n) = (w.chem.ns(), w.grid.n());
    let i = cell(w, s);
    let tg = w.t_ground_last(i);
    let chem = &w.chem;
    let p = &mut w.parts.parts[s];
    let t = p.e / p.heat_cap(chem).max(1e-9);
    for k in 0..ns {
        if p.wall[k] > 0.0 && chem.phase_in(k, t, |j| p.wall[j]) != Phase::Solid {
            let q = p.wall[k] * chem.subs[k].cp * t;
            (w.ground[k * n + i], w.e_ground[i], p.e) = (w.ground[k * n + i] + p.wall[k], w.e_ground[i] + q, p.e - q);
            p.wall[k] = 0.0;
        }
    }
    let (e1, e0) = (p.heat_cap(chem) * tg, p.e);
    let got = w.take_heat(i, e1 - e0);
    let p = &mut w.parts.parts[s];
    p.e = if got == e1 - e0 { e1 } else { e0 + got };
    let chem = &w.chem;
    let gone = p.walls(chem).mass < w.cfg.phys.min_kg;
    p.resize(chem);
    if gone {
        dissolve(w, s);
    }
}

/// Load one structure: weights less buoyancy, pushes of moving liquid and bodies; break what is
/// overloaded; float and drift.
fn load(w: &mut World, r: usize) {
    let (list, i, g) = (members(w, r), cell(w, r), w.cfg.gravity);
    let (depth, rho, molv) = liquid(w, i);
    let flux = w.liquid_flux.get(i).copied().unwrap_or(0.0) as f64;
    let v = if depth > STILL_DEPTH { flux / (molv * w.grid.cell_m * depth) } else { 0.0 };
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
    // From the supports (anchored parts, else the held one) out along the bonds; each bond
    // carries what hangs beyond it; a ground bond carries the pushes on what it holds.
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
    // The holder of a structure that does not rest on the ground bears what hangs on the part it
    // holds (as of the start of the month) by its own force; past that, it lets go.
    let x = held.filter(|_| !s.anchored).and_then(|h| list.iter().position(|&p| p == h));
    if let Some((x, o)) = x.and_then(|x| holder(w, list[x]).map(|o| (x, o))) {
        if wt[x] + push[x] > bears(w, &w.life.bodies[o]) {
            torn_from(w, o);
        }
    }
    // Afloat and free: drift to the neighbour whose liquid stands lowest, as far as the
    // liquid moves in a month.
    if !afloat || held.is_some() || v <= 0.0 || w.parts.parts[r].root as usize != r {
        return;
    }
    let mut rng = Rng::of_node(w.seed, w.time, id(w, r), Purpose::Bond);
    let head = |j: usize| w.life.env[j][2] + w.life.env[j][1];
    let to = DIRS.iter().filter_map(|&d| w.grid.step(i, d)).filter(|&j| head(j) < head(i)).min_by(|&a, &b| head(a).total_cmp(&head(b)));
    if let Some(j) = to.filter(|_| rng.f64() < v * w.cfg.calendar().month as f64 / w.grid.cell_m) {
        move_structure(w, r, j);
    }
}
