//! What bodies perceive of parts, and what parts give a body: the inside of a structure, and being held up afloat.

use super::{held, NO, NOBODY};
use crate::body::Body;
use crate::chem::Phase;
use crate::World;

/// A body perceives the ln of a part's mass over its own, and the ln of its length (m), over these.
const LN_MASS_SCALE: f32 = 7.0;
const LN_SIZE_SCALE: f32 = 3.0;

/// The structure body `b` is inside: its root, how closed it is, its conductance (W/K).
pub fn room(w: &World, b: &Body) -> Option<(usize, f32, f64)> {
    let p = if b.inside == 0 { None } else { w.parts.get(b.inside) }?;
    let r = w.parts.parts[p].root as usize;
    let s = &w.parts.parts[r].s;
    (w.parts.parts[p].cell == b.cell && s.encl > 0.0).then_some((r, s.encl, s.g))
}

/// For a body's update: how closed the walls around it are, and its share of their conductance.
pub(crate) fn walls_around(w: &World, b: &Body) -> Option<(f64, f64)> {
    let (r, e, g) = room(w, b)?;
    let others = w.life.at[b.cell as usize].iter().filter(|&&o| w.life.bodies[o as usize].alive && room(w, &w.life.bodies[o as usize]).is_some_and(|x| x.0 == r)).count();
    Some((e as f64, g / (others + 1) as f64))
}

/// Standing liquid on cell `i`: depth m, density kg/m³, mol per m³.
pub(super) fn liquid(w: &World, i: usize) -> (f64, f64, f64) {
    let (env, n) = (w.life.env[i], w.grid.n());
    let (mut m, mut v, mut mol) = (0.0, 0.0, 0.0);
    for k in (0..w.chem.ns()).filter(|&k| w.chem.phase_in(k, env[0], |j| w.g(j, i)) == Phase::Liquid) {
        let a = w.ground[k * n + i];
        (m, v, mol) = (m + a * w.chem.subs[k].molar_mass, v + a * w.chem.molar_volume(k, Phase::Liquid), mol + a);
    }
    if v > 0.0 { (env[1], m / v, mol / v) } else { (0.0, 0.0, 0.0) }
}

/// Is body `b` (mass `mb`) held up in cell `j`'s liquid by the structure it holds?
pub fn afloat(w: &World, b: &Body, j: usize, mb: f64) -> bool {
    let Some(p) = held(w, b) else { return false };
    let s = &w.parts.parts[w.parts.parts[p].root as usize].s;
    let (depth, rho, _) = liquid(w, j);
    depth >= w.cfg.phys.deep && !s.anchored && rho * s.outer >= s.mass + mb
}

/// Depth of liquid over a body: none while afloat.
pub fn depth(w: &World, b: &Body, d: f64) -> f64 {
    if d > 0.0 && b.hold != 0 && afloat(w, b, b.cell as usize, b.mass(&w.chem)) { 0.0 } else { d }
}

/// What a body perceives of what it holds (whether, ln mass ratio, hardness, hollowness) and
/// of the walls around it.
pub(crate) fn sense(w: &World, b: &Body, mb: f64) -> [f32; 5] {
    let mut x = [0f32; 5];
    if let Some(p) = held(w, b) {
        let pt = &w.parts.parts[p];
        let walls = pt.walls(&w.chem);
        x[..4].copy_from_slice(&[1.0, crate::brain::ln((walls.mass / mb) as f32) / LN_MASS_SCALE + 1.0, walls.hardness as f32, pt.hollow]);
    }
    x[4] = room(w, b).map_or(0.0, |r| r.1);
    x
}

/// What a body perceives of part `p`: ln mass ratio, hardness, hollowness, ln length, how closed
/// its structure is, anchored, held, share of it the body would take.
pub fn features(w: &World, b: &Body, p: usize, mb: f64) -> [f32; NO] {
    let (pt, chem) = (&w.parts.parts[p], &w.chem);
    let s = &w.parts.parts[pt.root as usize].s;
    let m = pt.mass(chem);
    let want: f64 = b.k.upt.iter().map(|&(k, u)| u as f64 * (pt.wall[k] + pt.fill[k]) * chem.subs[k].molar_mass).sum();
    let ln = crate::brain::ln;
    [ln((m / mb) as f32) / LN_MASS_SCALE, pt.walls(chem).hardness as f32, pt.hollow, ln(pt.size[0]) / LN_SIZE_SCALE, s.encl, s.anchored as u8 as f32, (pt.held != NOBODY) as u8 as f32, (want / m.max(1e-9)) as f32]
}

/// Can verb `v` be done to part `p` at all (as the body sees it)?
pub(crate) fn possible(w: &World, b: &Body, v: usize, p: usize, vol: f64) -> bool {
    let pt = &w.parts.parts[p];
    let s = &w.parts.parts[pt.root as usize].s;
    match v {
        0 => pt.held == NOBODY,
        3 => held(w, b).is_some_and(|h| w.parts.parts[h].root != pt.root),
        5 => s.encl > 0.0 && s.space >= vol && room(w, b).is_none_or(|r| r.0 != pt.root as usize),
        _ => true,
    }
}
