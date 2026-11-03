//! What bodies do to parts: hold, take, put, join, break and enter; form new parts, let go, and carry what they hold
//! when they move. A body's force decides what it can hold and break. Each act runs on
//! the body's cell in its chunk (`life::Chunk`), the body taken out of its slot.

use super::{at, event, fall_apart, held, holder, link_in, make, members, move_structure, restructure, root, sense::room, strength, unlink, HELD, LOAD, NOBODY};
use crate::{body::{self, Body, STRENGTH}, brain::OVERBS, chem::{Chemistry, Phase, Stores}, life::{Cell, Chunk}, life::Entry, World};
use crate::units::BODY_STORES;

/// Body `b` takes hold of the part at `p` of its cell if it bears the weight of its structure.
fn grab(w: &World, d: &mut Chunk, b: &mut Body, p: usize) {
    let (i, bears) = (b.cell as usize, bears(w, b));
    let c = d.at_mut(i);
    let s = c.parts[root(c, p)].s;
    if s.mass * w.cfg.gravity > bears {
        return;
    }
    c.parts[p].held = b.id;
    b.hold = Some(c.parts[p].id);
    event(d, w, true, i, HELD);
}

/// The verbs of the part acts by which a body lets go of its part (`let_go`), or by a load (`TORN`), after the verbs of
/// `act`: parts change only in the acts' landing.
pub const LET_GO: usize = OVERBS;
pub const TORN: usize = OVERBS + 1;

/// Let go of the held part (also when the body dies or joins another), broken off by a load if `load`: a part act
/// queued for the landing of the acts (`dropped`, applied to that part if that body still holds it). The body holds
/// nothing from now.
pub(crate) fn let_go(d: &mut Chunk, b: &mut Body, load: bool) {
    let c = d.at(b.cell as usize);
    if let Some(p) = held(c, b) {
        let part = c.parts[p].id;
        d.out.push(Entry::Act { id: b.id, on: Ok((if load { TORN } else { LET_GO }, part)), node: None });
    }
    b.hold = None;
}

/// The part named `part` of cell `i` is let go of by body `by`, if that body holds it, broken off by a load if `load`.
pub(crate) fn dropped(w: &World, d: &mut Chunk, i: usize, (by, part): (u64, u64), load: bool) {
    let c = d.at_mut(i);
    if let Some(p) = at(c, part).filter(|&p| c.parts[p].held == by) {
        c.parts[p].held = NOBODY;
        event(d, w, false, i, HELD + if load { LOAD } else { 0.0 });
    }
}

/// Body `b` (taken out of its slot) lets go of the part it holds, if it still holds it; it holds nothing from now.
fn drop_held(w: &World, d: &mut Chunk, b: &mut Body, load: bool) {
    if let Some(part) = b.hold.take() {
        dropped(w, d, b.cell as usize, (b.id, part), load);
    }
}

/// The body at place `o` of cell `i` lets go of what it holds, broken off by a load.
pub(super) fn torn_from(w: &World, d: &mut Chunk, i: usize, o: usize) {
    let mut ob = std::mem::take(&mut d.at_mut(i).bodies[o]);
    drop_held(w, d, &mut ob, true);
    d.at_mut(i).bodies[o] = ob;
}

/// Weight a body bears by its own force, N, which is also the force it pushes or pulls with. Force goes with its living
/// body (`Body::built`), so a hurt body is weaker and fuel adds no strength.
pub fn bears(w: &World, b: &Body) -> f64 {
    b.gene(STRENGTH) * b.built(&w.chem) * w.cfg.gravity * w.cfg.phys.grip
}

/// Verb `v` toward the part at `p` of body `b`'s cell: hold, take from, put to, join the held part to, break, enter. The
/// body pays as it acts (`body::pay`, on its cell's view), and does nothing if it cannot.
pub fn act(w: &World, d: &mut Chunk, b: &mut Body, v: usize, p: usize) {
    let (mb, days, i) = (b.mass(&w.chem), b.ahead(w), b.cell as usize);
    match v {
        0 => {
            let pt = &d.at(i).parts[p];
            if pt.held == NOBODY && body::pay(w, d, b, w.cfg.phys.grab * w.cfg.phys.c_handle * pt.mass(&w.chem)) {
                drop_held(w, d, b, false);
                grab(w, d, b, p);
            }
        }
        1 => take(w, d, b, p, mb, days),
        2 => put(w, d, b, p, mb, days),
        3 => {
            let c = d.at(i);
            let Some(h) = held(c, b) else { return };
            let (a, o) = (&c.parts[h], &c.parts[p]);
            let ac = a.contact().min(o.contact());
            let (s, apart, to) = (strength(w, a.walls(&w.chem).strength, o.walls(&w.chem).strength, ac), a.root != o.root, o.id);
            if apart && body::pay(w, d, b, w.cfg.phys.c_join * ac) {
                link_in(w, d, i, h, to, s);
            }
        }
        4 => {
            let f = bears(w, b);
            if !body::pay(w, d, b, f) {
                return;
            }
            let c = d.at(i);
            let id = c.parts[p].id;
            if let Some(o) = holder(c, p) {
                if f > bears(w, &c.bodies[o]) {
                    torn_from(w, d, i, o);
                }
            } else if let Some((o, s)) = c.bonds.of(id).min_by(|a, b| a.1.total_cmp(&b.1)) {
                if f > s as f64 {
                    unlink(w, d, i, id, o, false);
                }
            }
        }
        _ => {
            let c = d.at(i);
            let (r, id) = (c.parts[p].root, c.parts[p].id);
            let used: f64 = c.bodies.iter().filter(|o| o.alive && room(c, o).is_some_and(|x| x.0 == r)).map(|o| o.volume(&w.chem, o.temp(&w.chem))).sum();
            let (s, size) = (c.parts[root(c, p)].s, c.parts[p].size[0] as f64);
            if s.encl.closed > 0.0 && s.space >= used + b.volume(&w.chem, b.temp(&w.chem)) && body::pay(w, d, b, w.cfg.phys.walk(mb / b.n()) * mb * size) {
                b.inside = Some(id);
            }
        }
    }
}

/// Take from the part at `p` by the body's uptake selectivity, the held liquid first, as much as it takes by its mass in
/// the time and its stores have room for. Walls taken below `min_kg` fall apart (`fall_apart`): the rest goes to the
/// ground.
fn take(w: &World, d: &mut Chunk, b: &mut Body, p: usize, mb: f64, days: f64) {
    let (i, bc) = (b.cell as usize, w.chem.body());
    let pt = &d.at(i).parts[p];
    let m = b.wanted(&w.chem, |k| pt.wall[k] + pt.fill[k], w.cfg.phys.take * days * b.supply(&w.chem, &w.cfg.phys, b.temp(&w.chem)), mb);
    let got = bc.mass_of(&m);
    if got <= 0.0 || !body::pay(w, d, b, w.cfg.phys.c_handle * got) {
        return;
    }
    let (chem, pt) = (&w.chem, &mut d.at_mut(i).parts[p]);
    let me = Some(pt.id);
    // The heat that leaves with what it takes: its share by heat capacity.
    let q = pt.e * bc.heat_cap_of(&m) / pt.heat_cap(chem);
    for (s, (&a, &k)) in m.iter().zip(bc.subs()).enumerate() {
        let f = a.min(pt.fill[k]);
        pt.fill[k] -= f;
        pt.wall[k] = (pt.wall[k] - (a - f)).max(0.0);
        b.store[s] += a;
    }
    (pt.e, b.e) = (pt.e - q, b.e + q);
    let gone = pt.walls(chem).mass < w.cfg.phys.min_kg;
    b.digest(chem, Body::matter_at(chem).map_or(0.0, |s| m[s]), &mut d.counts.rounded[0]);
    d.counts.parts.from_parts += got;
    if gone {
        if b.hold == me {
            drop_held(w, d, b, false);
        }
        fall_apart(w, d, i, p);
    } else {
        let c = d.at_mut(i);
        c.parts[p].resize(chem);
        restructure(w, c, p);
    }
}

/// Put to the part at `p` what a body can spare (`Body::spare`): liquids into its hollow, solids onto its walls.
fn put(w: &World, d: &mut Chunk, b: &mut Body, p: usize, mb: f64, days: f64) {
    let (chem, t, bc, i) = (&w.chem, b.temp(&w.chem), w.chem.body(), b.cell as usize);
    let (size, n) = (b.size(chem), bc.n());
    let phase: [Phase; BODY_STORES] = std::array::from_fn(|s| if s < n { b.phase_of(chem, s, t) } else { Phase::Gas });
    let spare: Stores = std::array::from_fn(|s| if phase[s] == Phase::Gas { 0.0 } else { b.spare(chem, &w.cfg.phys, s, (size, mb)) });
    let all = bc.mass_of(&spare);
    if all <= 0.0 {
        return;
    }
    let wet = |s: usize| phase[s] == Phase::Liquid;
    let mut m: Stores = spare.map(|a| a * (w.cfg.phys.take * days * mb / all).min(1.0));
    let lv: f64 = (0..n).filter(|&s| wet(s)).map(|s| m[s] * chem.molar_volume(bc.subs()[s], Phase::Liquid)).sum();
    let fit = (d.at(i).parts[p].room(chem) / lv.max(1e-12)).min(1.0);
    (0..n).filter(|&s| wet(s)).for_each(|s| m[s] *= fit);
    let got = bc.mass_of(&m);
    if got <= 0.0 || !body::pay(w, d, b, w.cfg.phys.c_handle * got) {
        return;
    }
    let (m, q) = give_up(chem, b, &m);
    let pt = &mut d.at_mut(i).parts[p];
    bc.subs().iter().zip(&m).enumerate().for_each(|(s, (&k, &a))| *if wet(s) { &mut pt.fill[k] } else { &mut pt.wall[k] } += a);
    pt.e += q;
    pt.resize(chem);
    d.counts.parts.to_parts += got;
    restructure(w, d.at_mut(i), p);
}

/// Body `b` gives up `m` mol of each of its stores, no more than it holds (paying may have used some of them), with their
/// share of its heat. Returns what it gave and that heat, J.
fn give_up(chem: &Chemistry, b: &mut Body, m: &Stores) -> (Stores, f64) {
    let m: Stores = std::array::from_fn(|s| m[s].min(b.store[s]));
    let q = b.heat_with(chem, &b.thermal(chem), &m);
    b.store.iter_mut().zip(&m).for_each(|(s, a)| *s -= a);
    b.e -= q;
    (m, q)
}

/// Put the solids it holds above what it keeps (`Body::kept`) out as a new part (up to `form` of the body's mass, at least
/// `min_kg`), shaped by the brain; the body holds it if its hands are free and its force bears it.
pub fn form(w: &World, d: &mut Chunk, b: &mut Body, shape: [f32; 2]) {
    let i = b.cell as usize;
    let (chem, mb, t, bc) = (&w.chem, b.mass(&w.chem), b.temp(&w.chem), w.chem.body());
    let size = b.size(chem);
    let spare: Stores = std::array::from_fn(|s| if s < bc.n() && b.phase_of(chem, s, t) == Phase::Solid { (b.store[s] - b.kept(chem, &w.cfg.phys, s, size)).max(0.0) } else { 0.0 });
    let all = bc.mass_of(&spare);
    let f = (w.cfg.phys.form * mb / all.max(1e-9)).min(1.0);
    let wall = spare.map(|a| a * f);
    let got = bc.mass_of(&wall);
    if got < w.cfg.phys.min_kg || !body::pay(w, d, b, w.cfg.phys.c_form * got) {
        return;
    }
    let (wall, e) = give_up(chem, b, &wall);
    let wall = bc.spread_out(&wall)[..chem.ns()].to_vec();
    let p = make(w, d, i, wall, e, shape[0], shape[1]);
    d.counts.parts.formed += 1.0;
    if held(d.at(i), b).is_none() {
        grab(w, d, b, p);
    }
}

/// Let go of the held part; with nothing held, leave the inside.
pub fn release(d: &mut Chunk, b: &mut Body) {
    if held(d.at(b.cell as usize), b).is_some() { let_go(d, b, false) } else { b.inside = None }
}

/// Before a body moves: the structure it holds (its root's id) and its mass, if it can carry it (if not, it lets go by a
/// load). `c`: the body's cell.
pub fn carry(c: &Cell, w: &World, b: &Body) -> Option<(u64, f64)> {
    let r = root(c, held(c, b)?);
    let s = c.parts[r].s;
    (s.mass * w.cfg.gravity <= bears(w, b)).then_some((c.parts[r].id, s.mass))
}

/// After body `by` of mass `mb` kg moved from cell `i` into cell `j` (`choose::land_move`): the structure it carried
/// (`carried`, its root) comes along if it is still in `i`: the other bodies that held a part of it, left behind, let go
/// of it, and those inside it are left outside; it pushes the structures there. A step on two cells: serial.
pub fn moved(w: &mut World, (by, carried): (u64, Option<u64>), (i, j): (usize, usize), mb: f64) {
    if let Some(r) = carried.filter(|&r| at(&w.life.cells[i], r).is_some()) {
        w.in_cell(i, |w, d| {
            let parts = members(d.at(i), r);
            for &p in &parts {
                if let Some(o) = holder(d.at(i), p).filter(|&o| d.at(i).bodies[o].id != by) {
                    let mut ob = std::mem::take(&mut d.at_mut(i).bodies[o]);
                    drop_held(w, d, &mut ob, false);
                    d.at_mut(i).bodies[o] = ob;
                }
            }
            let ids: Vec<u64> = parts.iter().map(|&p| d.at(i).parts[p].id).collect();
            for o in d.at_mut(i).bodies.iter_mut().filter(|o| o.alive && o.inside.is_some_and(|x| ids.contains(&x))) {
                o.inside = None;
            }
        });
        move_structure(w, i, r, j);
    }
    let push = (w.cfg.phys.bump * mb * w.cfg.gravity) as f32;
    let c = &mut w.life.cells[j];
    for x in 0..c.parts.len() {
        let r = root(c, x);
        c.parts[r].push = c.parts[r].push.max(push);
    }
}
