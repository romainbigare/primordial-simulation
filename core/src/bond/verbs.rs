//! What bodies do to parts: hold, take, put, join, break and enter; form new parts, anchor them, dig the bonded ground,
//! let go, and carry what they hold when they move. A body's force decides what it can hold and break.

use super::sense::room;
use super::{dissolve, event, held, holder, link, make, restructure, strength, unlink, GROUND, HELD, LOAD, NOBODY};
use crate::body::{self, Body, Layout, STRENGTH};
use crate::chem::{Chemistry, Phase};
use crate::World;

/// Taking hold of a part costs this share of the work of handling its mass (`Physics::c_handle`).
const GRAB_COST: f64 = 0.1;
/// A held part of this share of the body's built mass gives half its full lever (`Physics::lever`).
const LEVER_REF: f64 = 0.05;
/// What a body keeps of each store it puts out: twice its safe minimum (as `give` does).
const KEEP_MINIMUMS: f64 = 2.0;
/// A body puts out at most this share of each store at once, and digs at most this share of each substance of the
/// bonded ground.
const MOST_SHARE: f64 = 0.5;

/// Body `b` takes hold of part `p` if it bears the weight of `p`'s structure (one bonded to the
/// ground rests on it).
fn grab(w: &mut World, slot: usize, b: &mut Body, p: usize) {
    let s = w.parts.parts[w.parts.parts[p].root as usize].s;
    if !s.anchored && s.mass * w.cfg.gravity > bears(w, b) {
        return;
    }
    w.parts.parts[p].held = b.id(slot);
    b.hold = w.parts.parts[p].id(p);
    event(w, true, b.cell, HELD);
}

/// Let go of the held part (also when the body dies).
pub(crate) fn let_go(w: &mut World, b: &mut Body, load: bool) {
    if let Some(p) = held(w, b) {
        w.parts.parts[p].held = NOBODY;
        event(w, false, b.cell, HELD + if load { LOAD } else { 0.0 });
    }
    b.hold = 0;
}

/// The body in slot `o` lets go of what it holds, broken off by a load.
pub(super) fn torn_from(w: &mut World, o: usize) {
    let mut ob = std::mem::take(&mut w.life.bodies[o]);
    let_go(w, &mut ob, true);
    w.life.bodies[o] = ob;
}

/// What a held part multiplies a body's force by: more for a hard part, up to the body's size.
pub(crate) fn lever(w: &World, b: &Body) -> f64 {
    held(w, b).map_or(1.0, |p| {
        let walls = w.parts.parts[p].walls(&w.chem);
        1.0 + w.cfg.phys.lever * walls.hardness * walls.mass / (walls.mass + LEVER_REF * b.built(&w.chem))
    })
}

/// Weight a body bears by its own force, N: what it holds adds nothing (a lever does not lift
/// itself). Force goes with the body that is built (`Body::built`: its matter and the liquid it
/// keeps, not the fuel it stores nor the matter held for offspring), not the whole mass: a hurt
/// body is weaker, and carrying fuel adds no strength.
pub(crate) fn bears(w: &World, b: &Body) -> f64 {
    b.gene(STRENGTH) * b.built(&w.chem) * w.cfg.gravity * w.cfg.phys.grip
}

/// Force a body pushes or pulls with, N (of its body, as `bears`).
pub fn force(w: &World, b: &Body) -> f64 {
    bears(w, b) * lever(w, b)
}

/// Verb `v` toward part `p`: hold, take from, put to, join the held part to, break, enter.
pub fn act(w: &mut World, slot: usize, b: &mut Body, v: usize, p: usize) {
    let (mb, days) = (b.mass(&w.chem), b.interval() as f64);
    match v {
        0 => {
            if w.parts.parts[p].held == NOBODY && body::pay(w, b, GRAB_COST * w.cfg.phys.c_handle * w.parts.parts[p].mass(&w.chem), days) {
                let_go(w, b, false);
                grab(w, slot, b, p);
            }
        }
        1 => take(w, b, p, mb, days),
        2 => put(w, b, p, mb, days),
        3 => {
            let Some(h) = held(w, b) else { return };
            let (a, c) = (&w.parts.parts[h], &w.parts.parts[p]);
            let ac = a.contact().min(c.contact());
            let s = strength(w, a.walls(&w.chem).strength, c.walls(&w.chem).strength, ac);
            if a.root != c.root && body::pay(w, b, w.cfg.phys.c_join * ac, days) {
                link(w, h, p as u32, s);
            }
        }
        4 => {
            let f = force(w, b);
            if !body::pay(w, b, f, days) {
                return;
            }
            if let Some(o) = holder(w, p) {
                if f > force(w, &w.life.bodies[o]) {
                    torn_from(w, o);
                }
            } else if let Some(&(o, s)) = w.parts.parts[p].links.iter().min_by(|a, b| a.1.total_cmp(&b.1)) {
                if f > s as f64 {
                    unlink(w, p, o, false);
                }
            }
        }
        _ => {
            let r = w.parts.parts[p].root as usize;
            let i = b.cell as usize;
            let used: f64 = w.life.at[i].iter().map(|&o| &w.life.bodies[o as usize]).filter(|o| o.alive && room(w, o).is_some_and(|x| x.0 == r)).map(|o| o.volume(&w.chem)).sum();
            let s = w.parts.parts[r].s;
            if s.encl > 0.0 && s.space >= used + b.volume(&w.chem) && body::pay(w, b, w.cfg.phys.walk(mb / b.n()) * mb * w.parts.parts[p].size[0] as f64, days) {
                b.inside = w.parts.parts[p].id(p);
            }
        }
    }
}

/// Take from a part by the body's uptake selectivity, the held liquid first, as much as it takes by its
/// mass (`Body::equiv`) in the time, as far as its stores have room (`Body::level`). Walls taken below
/// `w.cfg.phys.min_kg` fall apart.
fn take(w: &mut World, b: &mut Body, p: usize, mb: f64, days: f64) {
    let pt = &w.parts.parts[p];
    let m = b.wanted(&w.chem, |k| pt.wall[k] + pt.fill[k], w.cfg.phys.take * days * b.supply(&w.chem, &w.cfg.phys), mb);
    let got = w.chem.mass_of(&m);
    if got <= 0.0 || !body::pay(w, b, w.cfg.phys.c_handle * got, days) {
        return;
    }
    let (chem, pt) = (&w.chem, &mut w.parts.parts[p]);
    let q = chem.heat_share(pt.e, pt.heat_cap(chem), &m);
    for (k, &a) in m.iter().enumerate() {
        let f = a.min(pt.fill[k]);
        pt.fill[k] -= f;
        pt.wall[k] = (pt.wall[k] - (a - f)).max(0.0);
        b.store[k] += a;
    }
    b.digest(chem, chem.matter().map_or(0.0, |k| m[k]));
    (pt.e, b.e) = (pt.e - q, b.e + q);
    w.counts.parts.from_parts += got;
    if pt.walls(chem).mass < w.cfg.phys.min_kg {
        if b.hold == pt.id(p) {
            let_go(w, b, false);
        }
        dissolve(w, p);
    } else {
        pt.resize(chem);
        restructure(w, p);
    }
}

/// What body `b` (of size `size`) holds of `k` above `KEEP_MINIMUMS` times its safe minimum, mol.
fn spare(chem: &Chemistry, lay: &Layout, b: &Body, k: usize, size: f64) -> f64 {
    (b.store[k] - KEEP_MINIMUMS * b.gene(lay.min + k).max(0.0) * size / chem.subs[k].molar_mass).max(0.0)
}

/// Put to a part what a body holds above twice its safe minimums (`spare`): liquids into its hollow as far as there is
/// room, solids onto its walls; at most half of each store.
fn put(w: &mut World, b: &mut Body, p: usize, mb: f64, days: f64) {
    let (chem, lay, t) = (&w.chem, Layout::of(&w.chem), b.temp(&w.chem));
    let size = b.size(chem);
    let phase: Vec<Phase> = (0..chem.ns()).map(|k| chem.phase_in(k, t, |j| b.store[j])).collect();
    let spare: Vec<f64> = (0..chem.ns()).map(|k| if phase[k] == Phase::Gas { 0.0 } else { spare(chem, &lay, b, k, size) }).collect();
    let all = w.chem.mass_of(&spare);
    if all <= 0.0 {
        return;
    }
    let wet: Vec<bool> = phase.iter().map(|&p| p == Phase::Liquid).collect();
    let mut m: Vec<f64> = spare.iter().zip(&b.store).map(|(a, s)| (a * (w.cfg.phys.take * days * mb / all).min(1.0)).min(MOST_SHARE * s)).collect();
    let lv: f64 = (0..chem.ns()).filter(|&k| wet[k]).map(|k| m[k] * chem.molar_volume(k, Phase::Liquid)).sum();
    let fit = (w.parts.parts[p].room(chem) / lv.max(1e-12)).min(1.0);
    (0..chem.ns()).filter(|&k| wet[k]).for_each(|k| m[k] *= fit);
    let got = w.chem.mass_of(&m);
    if got <= 0.0 || !body::pay(w, b, w.cfg.phys.c_handle * got, days) {
        return;
    }
    let chem = &w.chem;
    let m: Vec<f64> = m.iter().zip(&b.store).map(|(a, s)| a.min(*s)).collect();
    let q = chem.heat_share(b.e, b.heat_cap(chem), &m);
    let pt = &mut w.parts.parts[p];
    for (k, &a) in m.iter().enumerate() {
        *if wet[k] { &mut pt.fill[k] } else { &mut pt.wall[k] } += a;
        b.store[k] -= a;
    }
    (pt.e, b.e) = (pt.e + q, b.e - q);
    w.counts.parts.to_parts += got;
    pt.resize(chem);
    restructure(w, p);
}

/// Put solid stores above twice their safe minimums (`spare`) out as a new part (up to `phys.form` of the
/// body's mass, and at least `phys.min_kg`: lighter walls fall apart), shaped by the brain:
/// elongation and hollowness. The body holds it if its hands are free and its force bears it
/// (`grab`). The number of parts in a cell is not limited: each one costs matter and energy.
pub fn form(w: &mut World, slot: usize, b: &mut Body, shape: [f32; 2]) {
    let i = b.cell as usize;
    let (chem, lay, mb, t) = (&w.chem, Layout::of(&w.chem), b.mass(&w.chem), b.temp(&w.chem));
    let size = b.size(chem);
    let spare: Vec<f64> = (0..chem.ns()).map(|k| if chem.phase_in(k, t, |j| b.store[j]) == Phase::Solid { spare(chem, &lay, b, k, size) } else { 0.0 }).collect();
    let all = w.chem.mass_of(&spare);
    let f = (w.cfg.phys.form * mb / all.max(1e-9)).min(1.0);
    let wall: Vec<f64> = spare.iter().zip(&b.store).map(|(a, s)| (a * f).min(MOST_SHARE * s)).collect();
    let got = w.chem.mass_of(&wall);
    if got < w.cfg.phys.min_kg || !body::pay(w, b, w.cfg.phys.c_form * got, b.interval() as f64) {
        return;
    }
    // Paying may have used some of what was to be put out.
    let wall: Vec<f64> = wall.iter().zip(&b.store).map(|(a, s)| a.min(*s)).collect();
    let e = w.chem.heat_share(b.e, b.heat_cap(&w.chem), &wall);
    for (s, a) in b.store.iter_mut().zip(&wall) {
        *s -= a;
    }
    b.e -= e;
    let p = make(w, i, wall, e, shape[0], shape[1]);
    w.counts.parts.formed += 1.0;
    if held(w, b).is_none() {
        grab(w, slot, b, p);
    }
}

/// Let go of the held part; with nothing held, leave the inside.
pub fn release(w: &mut World, b: &mut Body) {
    if held(w, b).is_some() { let_go(w, b, false) } else { b.inside = 0 }
}

/// Bond the held part to the cell's bonded ground and let go of it.
pub fn anchor(w: &mut World, b: &mut Body) {
    let (Some(p), i, n) = (held(w, b), b.cell as usize, w.grid.n()) else { return };
    let (mut v, mut sv) = (0.0, 0.0);
    for k in 0..w.chem.ns() {
        let vol = w.bed[k * n + i] * w.chem.molar_volume(k, Phase::Solid);
        (v, sv) = (v + vol, sv + vol * w.chem.subs[k].strength);
    }
    let ac = w.parts.parts[p].contact();
    let s = strength(w, w.parts.parts[p].walls(&w.chem).strength, sv / v.max(1e-12), ac);
    if v > 0.0 && body::pay(w, b, w.cfg.phys.c_join * ac, b.interval() as f64) {
        link(w, p, GROUND, s);
        let_go(w, b, false);
    }
}

/// Break bonded ground with force: each substance whose bond strength is below the body's
/// strength (times what a held part adds) comes loose, the more the weaker it is, as far as the body
/// can work in the time (`Physics::dig` x its body, `Body::built`, as force goes by the body).
pub fn dig(w: &mut World, b: &mut Body) {
    let (i, n, mb) = (b.cell as usize, w.grid.n(), b.built(&w.chem));
    let (lev, chem) = (lever(w, b), &w.chem);
    let f = b.gene(STRENGTH) * lev;
    let vol: f64 = (0..chem.ns()).map(|k| w.bed[k * n + i] * chem.molar_volume(k, Phase::Solid)).sum();
    if vol <= 0.0 {
        return;
    }
    let most = w.cfg.phys.dig * b.interval() as f64 * mb;
    let m: Vec<f64> = (0..chem.ns())
        .map(|k| {
            let (a, s) = (w.bed[k * n + i], &chem.subs[k]);
            if f <= s.strength || a <= 0.0 { 0.0 } else { (most * a * chem.molar_volume(k, Phase::Solid) / vol * (1.0 - s.strength / f) / s.molar_mass).min(MOST_SHARE * a) }
        })
        .collect();
    let got = w.chem.mass_of(&m);
    if !body::pay(w, b, w.cfg.phys.c_handle * got, b.interval() as f64) {
        return;
    }
    for (k, a) in m.iter().enumerate() {
        w.bed[k * n + i] -= a;
        w.ground[k * n + i] += a;
    }
}

/// Before a body moves: the structure it holds and its mass, if it can carry it; else it lets
/// go (an anchored structure, or one heavier than the body bears, `bears`).
pub fn carry(w: &mut World, b: &mut Body) -> Option<(usize, f64)> {
    let r = w.parts.parts[held(w, b)?].root as usize;
    let s = w.parts.parts[r].s;
    if s.anchored || s.mass * w.cfg.gravity > bears(w, b) {
        let_go(w, b, true);
        return None;
    }
    Some((r, s.mass))
}

/// After a body moved from `i` into `j`: the structure it carried comes along; it pushes the
/// structures there.
pub fn moved(w: &mut World, b: &mut Body, carried: Option<(usize, f64)>, j: usize, mb: f64) {
    b.inside = 0;
    if let Some((r, _)) = carried {
        w.parts.move_structure(r, j);
    }
    let push = (w.cfg.phys.bump * mb * w.cfg.gravity) as f32;
    for x in 0..w.parts.at[j].len() {
        let r = w.parts.parts[w.parts.at[j][x] as usize].root as usize;
        w.parts.parts[r].push = w.parts.parts[r].push.max(push);
    }
}
