//! The monthly step of parts: rot and heat, burial, loads and failure.

use super::{fall_apart, holder, loose_solids, numbers, sense::liquid, shed, to_ground, unlink, verbs::{bears, torn_from}, Walls, NOBODY};
use crate::{change::{frac, rate, scarce}, chem::Phase, life::{self, Chunk}, store::{Class, Node}, World, AIR};

/// Liquid shallower than this, m, pushes no part.
const STILL_DEPTH: f64 = 0.01;

/// Monthly: each chunk on the pool (`life::run`), cell by cell, the turns of the cell's parts (`turns`), then its
/// structures' loads (`loads`).
pub fn step(w: &mut World) {
    if w.life.cells.iter().all(|c| c.parts.is_empty()) {
        return;
    }
    let dt = w.cfg.calendar().month as f64;
    let load = w.life.cells.iter().map(|c| c.parts.len()).sum();
    life::run(w, vec![(); life::chunks(w)], load, |_| (), |w, d, _, _| {
        let cells: Vec<usize> = d.cells().filter(|&i| !d.at(i).parts.is_empty()).collect();
        for i in cells {
            let walls = turns(w, d, i, dt);
            loads(w, d, i, &walls);
        }
    });
}

/// The turns of the parts of cell `i` of chunk `d` over `dt` s, in their order, on the chunk's view of the store: the
/// share of each substance of its walls that the open world's decomposers rot (`rot`, one share for the cell), and all
/// of a substance no longer solid, go to the ground with their heat, where Change rots them on, one move per substance;
/// then the parts take the ground's temperature, one move of heat (`warm`), and those lying alone sink (`bury`); a part
/// whose walls are under `min_kg`, or buried past its smallest size, falls apart into the ground (`fall_apart`).
/// Returns the walls of the parts left, in their order.
fn turns(w: &World, d: &mut Chunk, i: usize, dt: f64) -> Vec<Walls> {
    let (chem, tg) = (&w.chem, w.t_ground_last(i));
    // (Opened first: `rot` reads the cell's own share of its air.)
    d.moves.open(w, i, Node::new(Class::Part, 0));
    let share: Vec<f64> = rot(w, d, i, tg).into_iter().map(|r| frac(r * dt)).collect();
    // What goes to the ground of each substance: mol, then J; each part's heat capacity after.
    let mut off = vec![[0.0; 2]; chem.ns()];
    let caps: Vec<f64> = d.at_mut(i).parts.iter_mut().map(|p| shed(chem, p, |k, t, wall| if chem.phase_in(k, t, |j| wall[j]) != Phase::Solid { 1.0 } else { share[k] }, &mut off)).collect();
    to_ground(w, d, i, &off);
    warm(w, d, i, tg, &caps);
    bury(w, d, i, (tg, dt));
    let (mut x, mut kept) = (0, Vec::with_capacity(caps.len()));
    while x < d.at(i).parts.len() {
        let p = &mut d.at_mut(i).parts[x];
        let walls = p.walls(chem);
        let gone = walls.mass < w.cfg.phys.min_kg || p.buried >= p.size[2];
        p.fit(walls.volume);
        if gone {
            fall_apart(w, d, i, x);
        } else {
            kept.push(walls);
            x += 1;
        }
    }
    kept
}

/// Flow's mixing of the loose ground (laws.md 1.3; bioturbation, Darwin 1881): each part of cell `i` of chunk `d` that
/// no body holds and no bond joins sinks `bury` × `dt` m into the loose ground at `tg` K, no deeper than that ground is
/// (its loose solids' volume over the cell's area). A structure stays on top while it is bonded.
fn bury(w: &World, d: &mut Chunk, i: usize, (tg, dt): (f64, f64)) {
    let loose = loose_solids(w, d, i, tg);
    let volume: f64 = loose.iter().enumerate().map(|(k, a)| a * w.chem.molar_volume(k, Phase::Solid)).sum();
    let depth = (volume / w.mesh.cells().area()[i]) as f32;
    let (c, sink) = (d.at_mut(i), (w.cfg.bury * dt) as f32);
    // The parts a bond joins, by id.
    let mut bonded: Vec<u64> = c.bonds.a.iter().chain(&c.bonds.b).copied().collect();
    bonded.sort_unstable();
    for p in c.parts.iter_mut().filter(|p| p.held == NOBODY && bonded.binary_search(&p.id).is_err()) {
        p.buried = (p.buried + sink).min(depth);
    }
}

/// The parts of cell `i` of chunk `d`, of heat capacities `caps` (J/K), take its ground's temperature `tg`, K, as one
/// move of heat with the ground: all each wants, unless the ground cannot give that much (`Moves::take_from_ground`);
/// then what it gives goes to the parts that warm, by what each wants.
fn warm(w: &World, d: &mut Chunk, i: usize, tg: f64, caps: &[f64]) {
    let (mut up, mut down) = (0.0, 0.0);
    for (p, cap) in d.at(i).parts.iter().zip(caps) {
        let want = cap * tg - p.e;
        if want > 0.0 {
            up += want;
        } else {
            down -= want;
        }
    }
    let ground = d.moves.heat_cap_ground(w, i);
    let got = d.moves.take_from_ground(w, i, (0, 0.0, 0.0), up - down, ground).1;
    let short = got != up - down;
    for (p, cap) in d.at_mut(i).parts.iter_mut().zip(caps) {
        let e1 = cap * tg;
        p.e = if short && e1 > p.e { p.e + (e1 - p.e) * (got + down) / up } else { e1 };
    }
}

/// The share of each substance of the walls of a part in cell `i` at `t` K that the open world's decomposers rot, per
/// second: each reaction they speed (`Reaction::open` above 1) at Change's rate (`change::rate`) at its cell ground's
/// moisture, the substance of the walls a phase of its own, as a solid is in Change, and the other inputs taken from
/// its cell's air by their share there. The walls are the scarce input (a part holds far less than its cell's air), so
/// the rot is first order in them and one share fits every part of the cell (laws.md 3.1): it is found for one mol, of
/// the substances the cell's walls hold. The moves of cell `i` are open (`Moves::open`), so the air read is its share.
fn rot(w: &World, d: &Chunk, i: usize, t: f64) -> Vec<f64> {
    let (ns, j) = (w.chem.ns(), w.mesh.up(i, 0, AIR));
    let (mut rot, parts) = (vec![0.0; ns], &d.at(i).parts);
    // Of each substance an open reaction takes, whether a part's walls hold it (the others are not read).
    let takes = |k: usize| w.chem.reactions.iter().any(|rx| rx.open > 1.0 && rx.inputs.iter().any(|x| x.0 as usize == k));
    let held: Vec<bool> = (0..ns).map(|k| takes(k) && parts.iter().any(|p| p.wall[k] > 0.0)).collect();
    let mut rots = w.chem.reactions.iter().filter(|rx| rx.open > 1.0 && rx.inputs.iter().any(|x| held[x.0 as usize])).peekable();
    if rots.peek().is_none() {
        return rot;
    }
    let air = d.moves.amounts(w, Node::air(j));
    let (all, moist) = ((0..ns).map(&air).sum::<f64>(), w.scratch.view().moist(i, &w.cfg.phys));
    for rx in rots {
        for &(k, nu) in rx.inputs.iter().filter(|x| held[x.0 as usize]) {
            let k = k as usize;
            let a = |q: usize| if q == k { 1.0 } else { air(q) };
            let whole = |q: usize| if q == k { 1.0 } else { all };
            let lim = scarce(&rx.inputs, a).0;
            if lim > 0.0 {
                rot[k] += nu as f64 * lim * rate(rx, lim, (t, moist), a, whole);
            }
        }
    }
    rot
}

/// Each structure of cell `i` of chunk `d`, in the order of its root in the cell's list: its numbers found again from
/// its parts, their walls `walls` in the order of the list (`numbers`: their walls changed in their turns, their bonds
/// did not), and its load (`load_one`).
fn loads(w: &World, d: &mut Chunk, i: usize, walls: &[Walls]) {
    let c = d.at(i);
    // The parts that are not the root of their structure, (root, place), by root and place.
    let mut joined: Vec<(u64, usize)> = c.parts.iter().enumerate().filter(|x| x.1.root != x.1.id).map(|(x, p)| (p.root, x)).collect();
    joined.sort_unstable();
    let roots: Vec<usize> = (0..c.parts.len()).filter(|&x| c.parts[x].root == c.parts[x].id).collect();
    let mut of = Vec::new();
    for x in roots {
        let r = d.at(i).parts[x].id;
        // Its parts in the order of the list: the root and those joined to it.
        of.clear();
        of.push(x);
        of.extend(joined[joined.partition_point(|e| e.0 < r)..joined.partition_point(|e| e.0 <= r)].iter().map(|e| e.1));
        of.sort_unstable();
        let c = d.at_mut(i);
        c.parts[x].s = numbers(w, of.iter().map(|&y| (&c.parts[y], walls[y])));
        load_one(w, d, i, (x, &of));
    }
}

/// Load the structure whose root is at `rp` in cell `i` of chunk `d`, its parts at `of` (`loads`): weights less
/// buoyancy, pushes of liquid and bodies, from the part a body holds out along the bonds; break what is overloaded. A
/// structure no body holds rests on the ground and bears no load.
fn load_one(w: &World, d: &mut Chunk, i: usize, (rp, of): (usize, &[usize])) {
    let (c, g) = (d.at(i), w.cfg.gravity);
    let Some(h) = of.iter().position(|&p| holder(c, p).is_some()) else {
        d.at_mut(i).parts[rp].push = 0.0;
        return;
    };
    let (depth, rho, molv) = liquid(w, i, d.moves.amounts(w, Node::ground(i)));
    let flux = w.liquid_flux.get(i).copied().unwrap_or(0.0) as f64;
    // The liquid's speed through the cell, m/s: its flux over the mean width of the borders it crosses times its depth.
    let cells = w.mesh.cells();
    let (sum, borders) = cells.around(i).0.iter().map(|&l| cells.width()[l as usize]).filter(|&wd| wd > 0.0).fold((0.0, 0.0), |(s, k), wd| (s + wd, k + 1.0));
    let width = sum / borders;
    let v = if depth > STILL_DEPTH { flux / (molv * width * depth) } else { 0.0 };
    let bump = c.parts[rp].push as f64 / of.len() as f64;
    let (mut wt, mut push) = (vec![0.0; of.len()], vec![0.0; of.len()]);
    for (x, &p) in of.iter().enumerate() {
        let pt = &c.parts[p];
        let face = (pt.size[0] * pt.size[1]) as f64;
        let buoy = rho * g * pt.outer().min(face * depth);
        wt[x] = (pt.mass(&w.chem) * g - buoy).max(0.0);
        push[x] = 0.5 * rho * v * v * face + bump;
    }
    // From the held part out along the bonds, each carrying what hangs beyond it.
    let ids: Vec<u64> = of.iter().map(|&p| c.parts[p].id).collect();
    let mut order = vec![h];
    let mut up = vec![usize::MAX; of.len()];
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
            let hang = if up[x] != usize::MAX && ids[up[x]] == o { wt[x] + push[x] } else { 0.0 };
            if hang > st as f64 {
                breaks.push((ids[x], o));
            }
        }
    }
    // The holder bears what hangs on its part by its own force, or lets go.
    let torn = holder(c, of[h]).filter(|&o| wt[h] + push[h] > bears(w, &c.bodies[o]));
    let c = d.at_mut(i);
    c.parts[rp].push = 0.0;
    for (a, o) in breaks {
        if d.at(i).bonds.of(a).any(|l| l.0 == o) {
            unlink(w, d, i, a, o, true);
        }
    }
    if let Some(o) = torn {
        torn_from(w, d, i, o);
    }
}
