//! What bodies do to parts: hold, take, put, join, break and enter; form new parts, anchor them, dig the bonded ground,
//! let go, and carry what they hold when they move. A body's force decides what it can hold and break.

use super::{dissolve, event, held, holder, id, link, make, move_structure, restructure, sense::room, strength, unlink, GROUND, HELD, LOAD, NOBODY};
use crate::{body::{self, Body, STRENGTH}, chem::{Chemistry, Phase}, life::Handle, store::{Class, Move}, World};

/// Body `b` takes hold of part `p` if it bears the weight of `p`'s structure (one bonded to the ground rests on it).
fn grab(w: &mut World, b: &mut Body, p: usize) {
    let s = w.parts.parts[w.parts.parts[p].root as usize].s;
    if !s.anchored && s.mass * w.cfg.gravity > bears(w, b) {
        return;
    }
    w.parts.parts[p].held = b.id;
    b.hold = Some(id(w, p));
    event(w, true, b.cell as usize, HELD);
}

/// Let go of the held part (also when the body dies).
pub(crate) fn let_go(w: &mut World, b: &mut Body, load: bool) {
    if let Some(p) = held(w, b) {
        w.parts.parts[p].held = NOBODY;
        event(w, false, b.cell as usize, HELD + if load { LOAD } else { 0.0 });
    }
    b.hold = None;
}

/// The body in slot `o` lets go of what it holds, broken off by a load.
pub(super) fn torn_from(w: &mut World, o: Handle) {
    let mut ob = std::mem::take(&mut w.life[o]);
    let_go(w, &mut ob, true);
    w.life[o] = ob;
}

/// What a held part multiplies a body's force by: more for a hard part, up to the body's size.
pub(crate) fn lever(w: &World, b: &Body) -> f64 {
    held(w, b).map_or(1.0, |p| {
        let walls = w.parts.parts[p].walls(&w.chem);
        1.0 + w.cfg.phys.lever * walls.hardness * walls.mass / (walls.mass + w.cfg.phys.lever_ref * b.built(&w.chem))
    })
}

/// Weight a body bears by its own force, N (a held lever does not lift itself). Force goes with its living body
/// (`Body::built`), so a hurt body is weaker and fuel adds no strength.
pub(crate) fn bears(w: &World, b: &Body) -> f64 {
    b.gene(STRENGTH) * b.built(&w.chem) * w.cfg.gravity * w.cfg.phys.grip
}

/// Force a body pushes or pulls with, N (of its body, as `bears`).
pub fn force(w: &World, b: &Body) -> f64 {
    bears(w, b) * lever(w, b)
}

/// Verb `v` toward part `p`: hold, take from, put to, join the held part to, break, enter.
pub fn act(w: &mut World, b: &mut Body, v: usize, p: usize) {
    let (mb, days) = (b.mass(&w.chem), b.interval(w) as f64);
    match v {
        0 => {
            if w.parts.parts[p].held == NOBODY && body::pay(w, b, w.cfg.phys.grab * w.cfg.phys.c_handle * w.parts.parts[p].mass(&w.chem), days) {
                let_go(w, b, false);
                grab(w, b, p);
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
                if f > force(w, &w.life[o]) {
                    torn_from(w, o);
                }
            } else if let Some((o, s)) = w.parts.bonds.of(p).min_by(|a, b| a.1.total_cmp(&b.1)) {
                if f > s as f64 {
                    unlink(w, p, o, false);
                }
            }
        }
        _ => {
            let (r, i) = (w.parts.parts[p].root as usize, b.cell as usize);
            let used: f64 = w.life.cell(i).map(|o| &w.life[o]).filter(|o| o.alive && room(w, o).is_some_and(|x| x.0 == r)).map(|o| o.volume(&w.chem, o.temp(&w.chem))).sum();
            let s = w.parts.parts[r].s;
            if s.encl.closed > 0.0 && s.space >= used + b.volume(&w.chem, b.temp(&w.chem)) && body::pay(w, b, w.cfg.phys.walk(mb / b.n()) * mb * w.parts.parts[p].size[0] as f64, days) {
                b.inside = Some(id(w, p));
            }
        }
    }
}

/// Take from a part by the body's uptake selectivity, the held liquid first, as much as it takes by its mass in the
/// time and its stores have room for. Walls taken below `min_kg` fall apart.
fn take(w: &mut World, b: &mut Body, p: usize, mb: f64, days: f64) {
    let pt = &w.parts.parts[p];
    let m = b.wanted(&w.chem, b.paces(w), |k| pt.wall[k] + pt.fill[k], w.cfg.phys.take * days * b.supply(&w.chem, &w.cfg.phys, b.temp(&w.chem)), mb);
    let got = w.chem.mass_of(&m);
    if got <= 0.0 || !body::pay(w, b, w.cfg.phys.c_handle * got, days) {
        return;
    }
    let (me, chem, pt) = (Some(id(w, p)), &w.chem, &mut w.parts.parts[p]);
    // The heat that leaves with what it takes: its share by heat capacity.
    let q = pt.e * chem.heat_cap_of(&m) / pt.heat_cap(chem);
    for (k, &a) in m.iter().enumerate() {
        let f = a.min(pt.fill[k]);
        pt.fill[k] -= f;
        pt.wall[k] = (pt.wall[k] - (a - f)).max(0.0);
        b.store[k] += a;
    }
    b.digest(chem, chem.matter().map_or(0.0, |k| m[k]), &mut w.counts.rounded[0]);
    (pt.e, b.e) = (pt.e - q, b.e + q);
    w.counts.parts.from_parts += got;
    if pt.walls(chem).mass < w.cfg.phys.min_kg {
        if b.hold == me {
            let_go(w, b, false);
        }
        dissolve(w, p);
    } else {
        pt.resize(chem);
        restructure(w, p);
    }
}

/// Put to a part what a body can spare (`Body::spare`): liquids into its hollow, solids onto its walls.
fn put(w: &mut World, b: &mut Body, p: usize, mb: f64, days: f64) {
    let (chem, t) = (&w.chem, b.temp(&w.chem));
    let size = b.size(chem);
    let phase: Vec<Phase> = (0..chem.ns()).map(|k| chem.phase_in(k, t, |j| b.store[j])).collect();
    let spare: Vec<f64> = (0..chem.ns()).map(|k| if phase[k] == Phase::Gas { 0.0 } else { b.spare(chem, &w.cfg.phys, k, (size, mb)) }).collect();
    let all = w.chem.mass_of(&spare);
    if all <= 0.0 {
        return;
    }
    let wet: Vec<bool> = phase.iter().map(|&p| p == Phase::Liquid).collect();
    let mut m: Vec<f64> = spare.iter().map(|a| a * (w.cfg.phys.take * days * mb / all).min(1.0)).collect();
    let lv: f64 = (0..chem.ns()).filter(|&k| wet[k]).map(|k| m[k] * chem.molar_volume(k, Phase::Liquid)).sum();
    let fit = (w.parts.parts[p].room(chem) / lv.max(1e-12)).min(1.0);
    (0..chem.ns()).filter(|&k| wet[k]).for_each(|k| m[k] *= fit);
    let got = w.chem.mass_of(&m);
    if got <= 0.0 || !body::pay(w, b, w.cfg.phys.c_handle * got, days) {
        return;
    }
    let (m, q) = give_up(&w.chem, b, &m);
    let pt = &mut w.parts.parts[p];
    m.iter().enumerate().for_each(|(k, &a)| *if wet[k] { &mut pt.fill[k] } else { &mut pt.wall[k] } += a);
    pt.e += q;
    w.counts.parts.to_parts += got;
    pt.resize(&w.chem);
    restructure(w, p);
}

/// Body `b` gives up `m` mol of its stores, no more than it holds (paying may have used some of them), with their share
/// of its heat. Returns what it gave and that heat, J.
fn give_up(chem: &Chemistry, b: &mut Body, m: &[f64]) -> (Vec<f64>, f64) {
    let m: Vec<f64> = m.iter().zip(&b.store).map(|(a, s)| a.min(*s)).collect();
    let q = b.heat_with(chem, &b.thermal(chem), &m);
    b.store.iter_mut().zip(&m).for_each(|(s, a)| *s -= a);
    b.e -= q;
    (m, q)
}

/// Put the solids it holds above what it keeps (`Body::kept`) out as a new part (up to `form` of the body's mass, at least
/// `min_kg`), shaped by the brain; the body holds it if its hands are free and its force bears it.
pub fn form(w: &mut World, b: &mut Body, shape: [f32; 2]) {
    let i = b.cell as usize;
    let (chem, mb, t) = (&w.chem, b.mass(&w.chem), b.temp(&w.chem));
    let size = b.size(chem);
    let spare: Vec<f64> = (0..chem.ns()).map(|k| if chem.phase_in(k, t, |j| b.store[j]) == Phase::Solid { (b.store[k] - b.kept(chem, &w.cfg.phys, k, size)).max(0.0) } else { 0.0 }).collect();
    let all = w.chem.mass_of(&spare);
    let f = (w.cfg.phys.form * mb / all.max(1e-9)).min(1.0);
    let wall: Vec<f64> = spare.iter().map(|a| a * f).collect();
    let got = w.chem.mass_of(&wall);
    if got < w.cfg.phys.min_kg || !body::pay(w, b, w.cfg.phys.c_form * got, b.interval(w) as f64) {
        return;
    }
    let (wall, e) = give_up(&w.chem, b, &wall);
    let p = make(w, i, wall, e, shape[0], shape[1]);
    w.counts.parts.formed += 1.0;
    if held(w, b).is_none() {
        grab(w, b, p);
    }
}

/// Let go of the held part; with nothing held, leave the inside.
pub fn release(w: &mut World, b: &mut Body) {
    if held(w, b).is_some() { let_go(w, b, false) } else { b.inside = None }
}

/// Bond the held part to the cell's bonded ground and let go of it.
pub fn anchor(w: &mut World, b: &mut Body) {
    let (Some(p), i, n) = (held(w, b), b.cell as usize, w.mesh.cells().n()) else { return };
    let (mut v, mut sv) = (0.0, 0.0);
    for k in 0..w.chem.ns() {
        let vol = w.store.amounts(Class::Bed)[k * n + i] * w.chem.molar_volume(k, Phase::Solid);
        (v, sv) = (v + vol, sv + vol * w.chem.subs[k].strength);
    }
    let ac = w.parts.parts[p].contact();
    let s = strength(w, w.parts.parts[p].walls(&w.chem).strength, sv / v.max(1e-12), ac);
    if v > 0.0 && body::pay(w, b, w.cfg.phys.c_join * ac, b.interval(w) as f64) {
        link(w, p, GROUND, s);
        let_go(w, b, false);
    }
}

/// Break bonded ground with force: each substance weaker than the body's strength (times a held part's lever) comes
/// loose, the more the weaker, as far as `dig` × its living body allows in the time.
pub fn dig(w: &mut World, b: &mut Body) {
    let (i, n, mb) = (b.cell as usize, w.mesh.cells().n(), b.built(&w.chem));
    let (lev, chem) = (lever(w, b), &w.chem);
    let f = b.gene(STRENGTH) * lev;
    let vol: f64 = (0..chem.ns()).map(|k| w.store.amounts(Class::Bed)[k * n + i] * chem.molar_volume(k, Phase::Solid)).sum();
    if vol <= 0.0 {
        return;
    }
    let most = w.cfg.phys.dig * b.interval(w) as f64 * mb;
    let m: Vec<f64> = (0..chem.ns())
        .map(|k| {
            let (a, s) = (w.store.amounts(Class::Bed)[k * n + i], &chem.subs[k]);
            if f <= s.strength || a <= 0.0 { 0.0 } else { (most * a * chem.molar_volume(k, Phase::Solid) / vol * (1.0 - s.strength / f) / s.molar_mass).min(a) }
        })
        .collect();
    let got = w.chem.mass_of(&m);
    if !body::pay(w, b, w.cfg.phys.c_handle * got, b.interval(w) as f64) {
        return;
    }
    let bed = |k: usize| w.store.amounts(Class::Bed)[k * n + i];
    m.iter().enumerate().for_each(|(k, &a)| w.moves.push(&w.chem, Move::loosen(i, k, a, bed(k))));
}

/// Before a body moves: the structure it holds and its mass, if it can carry it; else it lets go.
pub fn carry(w: &mut World, b: &mut Body) -> Option<(usize, f64)> {
    let r = w.parts.parts[held(w, b)?].root as usize;
    let s = w.parts.parts[r].s;
    if s.anchored || s.mass * w.cfg.gravity > bears(w, b) {
        let_go(w, b, true);
        return None;
    }
    Some((r, s.mass))
}

/// After a body moved from `i` into `j`: the structure it carried comes along; it pushes the structures there.
pub fn moved(w: &mut World, b: &mut Body, carried: Option<(usize, f64)>, j: usize, mb: f64) {
    b.inside = None;
    if let Some((r, _)) = carried {
        move_structure(w, r, j);
    }
    let push = (w.cfg.phys.bump * mb * w.cfg.gravity) as f32;
    for x in 0..w.parts.at[j].len() {
        let r = w.parts.parts[w.parts.at[j][x] as usize].root as usize;
        w.parts.parts[r].push = w.parts.parts[r].push.max(push);
    }
}
