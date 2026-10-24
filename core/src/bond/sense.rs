//! What bodies perceive of parts, and what parts give a body: the inside of a structure, and being held up afloat.

use super::{cell, held, part, Enclosure, NO, NOBODY};
use crate::{body::Body, brain::LN_MASS, chem::Phase, World};

/// A body perceives the ln of a part's length (m) over this (its mass as every mass, `brain::LN_MASS`).
const LN_SIZE_SCALE: f32 = 3.0;

/// The structure body `b` is inside: its root, and its enclosure.
pub fn room(w: &World, b: &Body) -> Option<(usize, Enclosure)> {
    let p = part(w, b.inside?)?;
    let r = w.parts.parts[p].root as usize;
    let e = w.parts.parts[r].s.encl;
    (cell(w, p) == b.cell as usize && e.closed > 0.0).then_some((r, e))
}

/// For a body's update: the link between it and the outside through the walls around it, its share of their
/// enclosure among the bodies inside.
pub(crate) fn enclosure(w: &World, b: &Body) -> Option<Enclosure> {
    let (r, e) = room(w, b)?;
    let others = w.life.cell(b.cell as usize).filter(|&o| w.life[o].alive && room(w, &w.life[o]).is_some_and(|x| x.0 == r)).count();
    Some(Enclosure { g: e.g / (others + 1) as f64, ..e })
}

/// Standing liquid on cell `i`: depth m, density kg/m³, mol per m³.
pub(super) fn liquid(w: &World, i: usize) -> (f64, f64, f64) {
    let env = w.life.env[i];
    let (mut m, mut v, mut mol) = (0.0, 0.0, 0.0);
    for k in (0..w.chem.ns()).filter(|&k| w.chem.phase_in(k, env[0], |j| w.g(j, i)) == Phase::Liquid) {
        let a = w.g(k, i);
        (m, v, mol) = (m + a * w.chem.subs[k].molar_mass, v + a * w.chem.molar_volume(k, Phase::Liquid), mol + a);
    }
    if v > 0.0 { (env[1], m / v, mol / v) } else { (0.0, 0.0, 0.0) }
}

/// Is body `b` (mass `mb`) held up in cell `j`'s liquid: does the liquid that its volume and the structure it holds
/// displace weigh as much as they do (Archimedes, as for parts)? Not where the liquid is shallower than `deep`, nor
/// holding a structure bonded to the ground.
pub fn afloat(w: &World, b: &Body, j: usize, mb: f64) -> bool {
    let s = held(w, b).map(|p| w.parts.parts[w.parts.parts[p].root as usize].s);
    let (depth, rho, _) = liquid(w, j);
    if depth < w.cfg.phys.deep || s.is_some_and(|s| s.anchored) {
        return false;
    }
    let (outer, mass) = s.map_or((0.0, 0.0), |s| (s.outer, s.mass));
    rho * (b.volume(&w.chem, b.temp(&w.chem)) + outer) >= mb + mass
}

/// Depth of liquid over a body: none while afloat.
pub fn depth(w: &World, b: &Body, d: f64) -> f64 {
    if d > 0.0 && afloat(w, b, b.cell as usize, b.mass(&w.chem)) { 0.0 } else { d }
}

/// Numbers a body perceives of what it holds and of the walls around it (`sense`).
pub(crate) const SENSE: usize = 5;

/// What a body perceives of what it holds (whether, ln mass ratio, hardness, hollowness) and of the walls around it.
/// The mass ratio reads one unit up, so holding nothing (0) fades into holding a part 1,100 times lighter.
pub(crate) fn sense(w: &World, b: &Body, mb: f64) -> [f32; SENSE] {
    let mut x = [0f32; SENSE];
    if let Some(p) = held(w, b) {
        let pt = &w.parts.parts[p];
        let walls = pt.walls(&w.chem);
        x[..4].copy_from_slice(&[1.0, crate::brain::ln((walls.mass / mb) as f32) / LN_MASS + 1.0, walls.hardness as f32, pt.hollow]);
    }
    x[4] = room(w, b).map_or(0.0, |r| r.1.closed);
    x
}

/// What a body perceives of part `p`: ln mass ratio, hardness, hollowness, ln length, how closed its structure is,
/// anchored, held, the share the body would take.
pub fn features(w: &World, b: &Body, p: usize, mb: f64) -> [f32; NO] {
    let (pt, chem) = (&w.parts.parts[p], &w.chem);
    let s = &w.parts.parts[pt.root as usize].s;
    let m = pt.mass(chem);
    let want: f64 = b.k.upt.iter().map(|&(k, u)| u as f64 * (pt.wall[k] + pt.fill[k]) * chem.subs[k].molar_mass).sum();
    let ln = crate::brain::ln;
    [ln((m / mb) as f32) / LN_MASS, pt.walls(chem).hardness as f32, pt.hollow, ln(pt.size[0]) / LN_SIZE_SCALE, s.encl.closed, s.anchored as u8 as f32, (pt.held != NOBODY) as u8 as f32, (want / m.max(1e-9)) as f32]
}

/// Can verb `v` be done to part `p` at all (as the body sees it)?
pub(crate) fn possible(w: &World, b: &Body, v: usize, p: usize, vol: f64) -> bool {
    let pt = &w.parts.parts[p];
    let s = &w.parts.parts[pt.root as usize].s;
    match v {
        0 => pt.held == NOBODY,
        3 => held(w, b).is_some_and(|h| w.parts.parts[h].root != pt.root),
        5 => s.encl.closed > 0.0 && s.space >= vol && room(w, b).is_none_or(|r| r.0 != pt.root as usize),
        _ => true,
    }
}
