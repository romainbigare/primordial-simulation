//! What lies in the cells of a window of the map, thing by thing, for a viewer that draws each one from its numbers
//! (laws.md 3.1, 4, 6.1): the bodies of each cell, the slots of its cover and its parts with their bonds. Read from the
//! world, nothing written. A shell hands the list out where the camera looks closely.
//!
//! The list is little-endian: `u32` cells, then for each cell `u32` its index, `f32` its west, north, east and south
//! edges in squares and its area in m², `u16` its bodies, slots, parts and bonds, then those, each a fixed record:
//! - a body: `id`, `f32` members, mass of a member kg, living body of a member kg, strength, colour (3 bands), copy mass
//!   kg, coat (`HEAT`), `KEEP_T` K, `COOL_T` K, `u32` starting kind (`u32::MAX`: none), lineage, `u8` diet (`DIET_*`),
//!   adult (1) or young (0), hidden units of its brain (at most 255), 0, `id` of the part it holds, `id` of the part of
//!   the structure it is inside;
//! - a slot of cover: `u32` starting kind, `f32` built cover kg/m², capture area m²/m², its 9 genes (`cover::LIGHT` to `cover::FIX`),
//!   `u8` full (1) or not, then 3 zeros;
//! - a part: `id`, `f32` its 3 outer sizes m (longest first), hollowness, mass kg, walls' colour (3 bands), walls'
//!   hardness, depth buried m, `id` of the body holding it, `id` of its structure's root, the space its structure's
//!   walls close m³, how closed they are (0..1);
//! - a bond: `id` of each end, `f32` strength N.
//!
//! An `id` is two `u32`, its low then its high half; 0 is none.

use pcore::body::{Body, Layout};
use pcore::bond::{Part, NOBODY};
use pcore::cover::{Parts, NG};
use pcore::World;

/// Diet bits: it takes what it uses of the cover (by grazing), of other bodies (by a bite: a choice, so it needs a
/// brain), and of the remains on the ground (body matter or a built solid it digests).
pub const DIET_COVER: u8 = 1;
pub const DIET_BODIES: u8 = 2;
pub const DIET_REMAINS: u8 = 4;

/// The things of each cell that overlaps the window from column `x` and row `y` of the map, `w` squares wide and `h`
/// high, cell by cell in their order (the module's list).
pub fn things(world: &World, [x, y, wide, high]: [f64; 4]) -> Vec<u8> {
    let cells: Vec<usize> = (0..world.mesh.cells().n())
        .filter(|&i| {
            let [w0, n0, e0, s0] = world.map.bounds(i);
            w0 < x + wide && e0 > x && n0 < y + high && s0 > y
        })
        .collect();
    let mut out = Out(Vec::new());
    out.u32(cells.len() as u32);
    for i in cells {
        cell(world, i, &mut out);
    }
    out.0
}

/// The record of cell `i` and of its things.
fn cell(w: &World, i: usize, out: &mut Out) {
    let (life, area, parts) = (w.life.cell_at(i), w.mesh.cells().area()[i], Parts::of(&w.chem));
    let bodies: Vec<&Body> = w.life.cell(i).map(|h| &w.life[h]).collect();
    let cover = &life.cover;
    let slots: Vec<usize> = if parts.is_some() { cover.held().collect() } else { vec![] };
    out.u32(i as u32);
    w.map.bounds(i).into_iter().chain([area]).for_each(|v| out.f32(v));
    [bodies.len(), slots.len(), life.parts.len(), life.bonds.count()].into_iter().for_each(|n| out.u16(n.min(u16::MAX as usize) as u16));
    bodies.iter().for_each(|b| body(w, b, out));
    if let Some(p) = parts {
        let mm = p.mm(&w.chem);
        for x in slots {
            out.u32(cover.root[x]);
            out.f32(cover.built(&mm, x) / area);
            out.f32(cover.leaf(&mm, x) / area);
            cover.genes[x][..NG].iter().for_each(|&g| out.f32(g as f64));
            out.0.extend([cover.full[x] as u8, 0, 0, 0]);
        }
    }
    life.parts.iter().for_each(|p| part(w, p, out));
    for l in 0..life.bonds.count() {
        out.id(life.bonds.a[l]);
        out.id(life.bonds.b[l]);
        out.f32(life.bonds.strength[l] as f64);
    }
}

/// The record of body `b`.
fn body(w: &World, b: &Body, out: &mut Out) {
    let (chem, lin) = (&w.chem, w.life.brain(b.lineage));
    let vis = b.visible(chem);
    out.id(b.id);
    out.f32(b.n());
    out.f32(vis[0] as f64);
    out.f32(b.member_body(chem));
    vis[1..].iter().for_each(|&v| out.f32(v as f64));
    out.f32(b.k.copy_mass);
    [pcore::body::HEAT, pcore::body::KEEP_T, pcore::body::COOL_T].into_iter().for_each(|g| out.f32(b.gene(g)));
    out.u32(lin.map_or(u32::MAX, |l| l.root));
    out.u32(b.lineage);
    let hidden = lin.map_or(0, |l| l.shape.h);
    out.0.extend([diet(w, b, lin.is_some_and(|l| !l.none())), b.adult(chem) as u8, hidden.min(u8::MAX as usize) as u8, 0]);
    out.id(b.hold.unwrap_or(0));
    out.id(b.inside.unwrap_or(0));
}

/// What body `b` takes from where (`DIET_*`): each source holds a substance it uses (`Body::uses`) with its uptake gene
/// on (`Body::wanted`); a bite is an act of a `brain`.
fn diet(w: &World, b: &Body, brain: bool) -> u8 {
    let (chem, lay) = (&w.chem, Layout::of(&w.chem));
    let t = b.temp(chem);
    let takes = |k: usize| chem.body().place(k).is_some_and(|s| b.gene(lay.upt + s) > 0.0 && b.uses(chem, k, t));
    let cover = Parts::of(chem).is_some_and(|p| p.sub.iter().any(|&k| takes(k)));
    let bodies = brain && chem.body().subs().iter().any(|&k| takes(k));
    let remains = (0..chem.ns()).any(|k| (chem.matter() == Some(k) || chem.is_material(k)) && takes(k));
    (DIET_COVER * cover as u8) | (DIET_BODIES * bodies as u8) | (DIET_REMAINS * remains as u8)
}

/// The record of part `p`.
fn part(w: &World, p: &Part, out: &mut Out) {
    let walls = p.walls(&w.chem);
    out.id(p.id);
    p.size.iter().for_each(|&d| out.f32(d as f64));
    out.f32(p.hollow as f64);
    out.f32(p.mass(&w.chem));
    walls.colour.into_iter().chain([walls.hardness, p.buried as f64]).for_each(|v| out.f32(v));
    out.id(if p.held == NOBODY { 0 } else { p.held });
    out.id(p.root);
    // The structure's numbers are on its root, which lies in the same cell.
    let s = w.life.cell_at(p.cell as usize).parts.iter().find(|q| q.id == p.root).map_or(p.s, |q| q.s);
    out.f32(s.space);
    out.f32(s.encl.closed as f64);
}

/// The list being written.
struct Out(Vec<u8>);

impl Out {
    fn u16(&mut self, v: u16) {
        self.0.extend(v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.0.extend(v.to_le_bytes());
    }
    fn f32(&mut self, v: f64) {
        self.0.extend((v as f32).to_le_bytes());
    }
    fn id(&mut self, v: u64) {
        self.u32(v as u32);
        self.u32((v >> 32) as u32);
    }
}
