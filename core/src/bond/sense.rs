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

/// A body that has outgrown the structure it is inside (by growth or by members joining it), its volume past the space
/// the walls close, is no longer inside it: it enters only where it fits (`verbs::act`), and it stays only while it does.
pub(crate) fn leave_outgrown(w: &World, d: &Chunk, b: &mut Body) {
    let c = d.at(b.cell as usize);
    let Some(p) = b.inside.and_then(|id| at(c, id)) else { return };
    if c.parts[root(c, p)].s.space < b.volume(&w.chem, b.temp(&w.chem)) {
        b.inside = None;
    }
}

/// The air of the room a body is inside, as its update finds it (laws.md 3.4): the conductance of the walls and openings
/// between that air and the outside, W/K, and how far the heat of the other bodies inside holds it above the outside, K.
pub(crate) struct RoomAir {
    pub g: f64,
    pub rise: f64,
}

/// For a body's update: the air of the room it is inside (`RoomAir`). The heat that leaves the bodies inside through
/// their coats warms that air, and it passes the walls and openings to the outside: in its steady state the air stands
/// above the outside by the heat of all of them over the walls' conductance. Its own time, its heat capacity (about
/// 1.2 kJ per m³ and K) over that conductance, is seconds to minutes, far below a sub-step, so it stands there at every
/// sub-step and holds no heat of its own: what the bodies lose reaches their ground, as outside. A body reads the
/// others as their last sub-step left them (`Lived::lost`), and its own heat passes its coat and the walls in series.
pub(crate) fn room_air(d: &Chunk, b: &Body) -> Option<RoomAir> {
    let (i, c) = (b.cell as usize, d.at(b.cell as usize));
    let (r, e) = room(c, b)?;
    let others: f64 = d.cell(i).map(|o| &d[o]).filter(|o| o.alive && room(c, o).is_some_and(|x| x.0 == r)).map(|o| o.lived.lost).sum();
    Some(RoomAir { g: e.g, rise: others / e.g.max(1e-300) })
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

/// Is body `b` held up in its cell's liquid: does the liquid that its volume and the structure it holds displace weigh
/// as much as they do (Archimedes, as for parts)? Not where the liquid is shallower than `deep`.
fn afloat(w: &World, d: &Chunk, b: &Body) -> bool {
    let (i, mb) = (b.cell as usize, b.mass(&w.chem));
    let c = d.at(i);
    let s = held(c, b).map(|p| c.parts[root(c, p)].s);
    let (depth, rho, _) = liquid(w, i, d.moves.amounts(w, Node::ground(i)));
    if depth < w.cfg.phys.deep {
        return false;
    }
    let (outer, mass) = s.map_or((0.0, 0.0), |s| (s.outer, s.mass));
    rho * (b.volume(&w.chem, b.temp(&w.chem)) + outer) >= mb + mass
}

/// Depth of liquid over a body: none while afloat.
pub fn depth(w: &World, d: &Chunk, b: &Body, depth: f64) -> f64 {
    if depth > 0.0 && afloat(w, d, b) { 0.0 } else { depth }
}

/// Numbers a body perceives of what it holds and of the walls around it (`sense`).
pub(crate) const SENSE: usize = 6;

/// How a body perceives the edge of the part at `p` of its cell `c`: the ln of its walls' thickness over a member's
/// width (`Body::width`), the share an edge leaves of what breaking a body like itself takes (`Edge::cut`); 0 for a
/// part as thick as the body is wide.
fn edge_seen(c: &Cell, w: &World, b: &Body, p: usize) -> f32 {
    crate::brain::ln((c.parts[p].thickness(&w.chem) / b.width(&w.chem, &w.cfg).max(1e-30)) as f32) / LN_SIZE_SCALE
}

/// What a body perceives of what it holds (whether, ln mass ratio, hardness, hollowness, its edge) and of the walls
/// around it, in its cell `c`. The mass ratio reads one unit up, so holding nothing (0) fades into holding a part 1,100
/// times lighter.
pub(crate) fn sense(c: &Cell, w: &World, b: &Body, mb: f64) -> [f32; SENSE] {
    let mut x = [0f32; SENSE];
    if let Some(p) = held(c, b) {
        let pt = &c.parts[p];
        let walls = pt.walls(&w.chem);
        x[..5].copy_from_slice(&[1.0, crate::brain::ln((walls.mass / mb) as f32) / LN_MASS + 1.0, walls.hardness as f32, pt.hollow, edge_seen(c, w, b, p)]);
    }
    x[5] = room(c, b).map_or(0.0, |r| r.1.closed);
    x
}

/// What a body perceives of the part at `p` of its cell `c` (of a lot, of one of its parts): ln mass ratio, hardness,
/// hollowness, ln length, its edge (`edge_seen`), how closed its structure is, held, the share the body would take.
pub fn features(c: &Cell, w: &World, b: &Body, p: usize, mb: f64) -> [f32; NO] {
    let (pt, chem) = (&c.parts[p], &w.chem);
    let s = &c.parts[root(c, p)].s;
    let m = pt.mass(chem) / pt.count;
    let want: f64 = b.k.upt.iter().map(|&(k, u)| u as f64 * (pt.wall[k] + pt.fill[k]) / pt.count * chem.subs[k].molar_mass).sum();
    let ln = crate::brain::ln;
    [ln((m / mb) as f32) / LN_MASS, pt.walls(chem).hardness as f32, pt.hollow, ln(pt.size[0]) / LN_SIZE_SCALE, edge_seen(c, w, b, p), s.encl.closed, (pt.held != NOBODY) as u8 as f32, (want / m.max(1e-9)) as f32]
}

/// Can verb `v` be done to the part at `p` of body `b`'s cell `c` at all (as the body sees it)?
pub(crate) fn possible(c: &Cell, b: &Body, v: usize, p: usize, vol: f64) -> bool {
    let pt = &c.parts[p];
    match v {
        0 => pt.held == NOBODY,
        3 => held(c, b).is_some_and(|h| c.parts[h].root != pt.root),
        5 => {
            let s = &c.parts[root(c, p)].s;
            s.encl.closed > 0.0 && s.space >= vol && room(c, b).is_none_or(|r| r.0 != pt.root)
        }
        _ => true,
    }
}
