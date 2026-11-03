//! What bodies perceive of parts, and what parts give a body: the inside of a structure, and being held up afloat. A
//! body reads the parts of its own cell (`life::Cell`).

use super::{at, held, root, Enclosure, NO, NOBODY};
use crate::{body::Body, brain::LN_MASS, chem::Phase, life::{Cell, Chunk}, store::Node, World};

/// A body perceives the ln of a part's length (m) over this (its mass as every mass, `brain::LN_MASS`).
const LN_SIZE_SCALE: f32 = 3.0;

/// The structure body `b` is inside, in its cell `c`: its root's id, and its enclosure.
pub fn room(c: &Cell, b: &Body) -> Option<(u64, Enclosure)> {
    let p = at(c, b.inside?)?;
    let r = &c.parts[root(c, p)];
    (r.s.encl.closed > 0.0).then_some((r.id, r.s.encl))
}

/// For a body's update: the link between it and the outside through the walls around it, its share of their
/// enclosure among the bodies inside.
pub(crate) fn enclosure(d: &Chunk, b: &Body) -> Option<Enclosure> {
    let (i, c) = (b.cell as usize, d.at(b.cell as usize));
    let (r, e) = room(c, b)?;
    let others = d.cell(i).filter(|&o| d[o].alive && room(c, &d[o]).is_some_and(|x| x.0 == r)).count();
    Some(Enclosure { g: e.g / (others + 1) as f64, ..e })
}

/// Standing liquid on cell `i` whose ground holds `g` mol of each substance: depth m, density kg/m³, mol per m³.
pub(super) fn liquid(w: &World, i: usize, g: impl Fn(usize) -> f64) -> (f64, f64, f64) {
    let env = w.life.env[i];
    let (mut m, mut v, mut mol) = (0.0, 0.0, 0.0);
    for k in (0..w.chem.ns()).filter(|&k| w.chem.phase_in(k, env[0], &g) == Phase::Liquid) {
        let a = g(k);
        (m, v, mol) = (m + a * w.chem.subs[k].molar_mass, v + a * w.chem.molar_volume(k, Phase::Liquid), mol + a);
    }
    if v > 0.0 { (env[1], m / v, mol / v) } else { (0.0, 0.0, 0.0) }
}

/// Is body `b` (mass `mb`) held up in cell `j`'s liquid: does the liquid that its volume and the structure it holds
/// displace weigh as much as they do (Archimedes, as for parts)? Not where the liquid is shallower than `deep`.
pub fn afloat(w: &World, d: &Chunk, b: &Body, j: usize, mb: f64) -> bool {
    let c = d.at(b.cell as usize);
    let s = held(c, b).map(|p| c.parts[root(c, p)].s);
    let (depth, rho, _) = liquid(w, j, d.moves.amounts(w, Node::ground(j)));
    if depth < w.cfg.phys.deep {
        return false;
    }
    let (outer, mass) = s.map_or((0.0, 0.0), |s| (s.outer, s.mass));
    rho * (b.volume(&w.chem, b.temp(&w.chem)) + outer) >= mb + mass
}

/// Depth of liquid over a body: none while afloat.
pub fn depth(w: &World, d: &Chunk, b: &Body, depth: f64) -> f64 {
    if depth > 0.0 && afloat(w, d, b, b.cell as usize, b.mass(&w.chem)) { 0.0 } else { depth }
}

/// Numbers a body perceives of what it holds and of the walls around it (`sense`).
pub(crate) const SENSE: usize = 5;

/// What a body perceives of what it holds (whether, ln mass ratio, hardness, hollowness) and of the walls around it, in
/// its cell `c`. The mass ratio reads one unit up, so holding nothing (0) fades into holding a part 1,100 times lighter.
pub(crate) fn sense(c: &Cell, w: &World, b: &Body, mb: f64) -> [f32; SENSE] {
    let mut x = [0f32; SENSE];
    if let Some(p) = held(c, b) {
        let pt = &c.parts[p];
        let walls = pt.walls(&w.chem);
        x[..4].copy_from_slice(&[1.0, crate::brain::ln((walls.mass / mb) as f32) / LN_MASS + 1.0, walls.hardness as f32, pt.hollow]);
    }
    x[4] = room(c, b).map_or(0.0, |r| r.1.closed);
    x
}

/// What a body perceives of the part at `p` of its cell `c`: ln mass ratio, hardness, hollowness, ln length, how closed
/// its structure is, held, the share the body would take.
pub fn features(c: &Cell, w: &World, b: &Body, p: usize, mb: f64) -> [f32; NO] {
    let (pt, chem) = (&c.parts[p], &w.chem);
    let s = &c.parts[root(c, p)].s;
    let m = pt.mass(chem);
    let want: f64 = b.k.upt.iter().map(|&(k, u)| u as f64 * (pt.wall[k] + pt.fill[k]) * chem.subs[k].molar_mass).sum();
    let ln = crate::brain::ln;
    [ln((m / mb) as f32) / LN_MASS, pt.walls(chem).hardness as f32, pt.hollow, ln(pt.size[0]) / LN_SIZE_SCALE, s.encl.closed, (pt.held != NOBODY) as u8 as f32, (want / m.max(1e-9)) as f32]
}

/// Can verb `v` be done to the part at `p` of body `b`'s cell `c` at all (as the body sees it)?
pub(crate) fn possible(c: &Cell, b: &Body, v: usize, p: usize, vol: f64) -> bool {
    let pt = &c.parts[p];
    let s = &c.parts[root(c, p)].s;
    match v {
        0 => pt.held == NOBODY,
        3 => held(c, b).is_some_and(|h| c.parts[h].root != pt.root),
        5 => s.encl.closed > 0.0 && s.space >= vol && room(c, b).is_none_or(|r| r.0 != pt.root),
        _ => true,
    }
}
