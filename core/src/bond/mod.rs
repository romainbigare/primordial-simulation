//! Law 3: Bond. Parts are nodes of the store (`Class::Part`) that bonds join into structures; a bond breaks when its
//! load passes its strength (laws.md 3). Here: parts, structures and their graph; `bonds`: the bonds and the enclosure;
//! `verbs`: what bodies do to parts; `sense`: what parts give a body; `month`: the monthly heat, reactions, loads and
//! drift.

mod bonds;
mod month;
mod sense;
mod verbs;

pub use bonds::{Bonds, Enclosure};
pub use month::step;
pub use sense::{afloat, depth, features, room};
pub(crate) use sense::{enclosure, possible, sense, SENSE};
pub use verbs::{act, carry, dig, force, form, moved, release, ANCHORS, LET_GO, TORN};
pub(crate) use verbs::{bears, dropped, let_go, lever};

use crate::{chem::{Chemistry, Phase}, event::{Event, EventKind}, life::{free_place, Handle}, store::NodeId, World};
use serde::{Deserialize, Serialize};

/// Held by no body; the bonded ground (as the other end of a bond).
pub const NOBODY: u64 = u64::MAX;
pub const GROUND: u32 = u32::MAX;
/// Bond kinds (the value of bond events): body–part, part–part, part–ground; a break by load adds `LOAD`.
pub(crate) const HELD: f64 = 0.0;
pub(crate) const JOINED: f64 = 1.0;
pub(crate) const ANCHORED: f64 = 2.0;
pub const LOAD: f64 = 3.0;
/// Features of a perceived part.
pub(crate) const NO: usize = 8;
/// Most hollowness of a part (a share of its outer volume).
const MOST_HOLLOW: f32 = 0.95;
/// An elongation of 1 (or -1) is an aspect of 10 to this power (100).
const ASPECT_DECADES: f64 = 2.0;
/// Faces of a box: plates close one as wide as the longest part; the bonded ground is the last face of an anchored one.
const BOX_FACES: f64 = 6.0;

/// A part, at its slot among the nodes of `Class::Part`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Part {
    pub alive: bool,
    /// The generation of the part in its slot (its id is the slot and this): a freed slot's moves on
    pub gen: u32,
    /// The cell it lies in
    pub cell: u32,
    /// mol of each substance: the walls, and the liquid held in the hollow
    pub wall: Vec<f64>,
    pub fill: Vec<f64>,
    /// Thermal energy, J
    pub e: f64,
    /// Outer size, m, longest first; share of the outer volume that is hollow
    pub size: [f32; 3],
    pub hollow: f32,
    /// Body holding it (id) or `NOBODY`
    pub held: u64,
    /// Lowest index of its structure; that part holds the structure's numbers
    pub root: u32,
    pub s: Info,
    /// Largest push this month from bodies moving through its cell (on the root), N
    pub push: f32,
}

/// Numbers of a structure.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Info {
    pub parts: u32,
    /// kg (walls and what they hold); outer volume m³
    pub mass: f64,
    pub outer: f64,
    /// The space its walls close (m³), and the link between it and the cell
    pub space: f64,
    pub encl: Enclosure,
    pub anchored: bool,
    /// Held up by the liquid around it (as of the last month)
    pub afloat: bool,
}

/// What bodies did with parts since the shell last read it (not saved).
#[derive(Clone, Debug, Default)]
pub struct Tally {
    /// Moves into deep liquid afloat; parts formed; kg put into and taken from parts
    pub afloat: f64,
    pub formed: f64,
    pub to_parts: f64,
    pub from_parts: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Parts {
    /// By slot (`Class::Part`); a free slot holds a part that is not alive
    pub parts: Vec<Part>,
    /// The free slots, the last freed first
    free: Vec<u32>,
    pub bonds: Bonds,
    /// Parts in each cell, in a fixed order (sized when the world is made)
    pub at: Vec<Vec<u32>>,
}

/// Of a part's walls: their mass, kg, and volume, m³, and by volume their hardness, bond strength (0..1) and heat
/// conductivity, W/(m K).
#[derive(Clone, Copy, Debug)]
pub struct Walls {
    pub mass: f64,
    pub volume: f64,
    pub hardness: f64,
    pub strength: f64,
    pub conductivity: f64,
}

impl Part {
    pub fn mass(&self, chem: &Chemistry) -> f64 {
        self.wall.iter().zip(&self.fill).zip(&chem.subs).map(|((a, f), s)| (a + f) * s.molar_mass).sum()
    }
    pub fn heat_cap(&self, chem: &Chemistry) -> f64 {
        self.wall.iter().zip(&self.fill).zip(&chem.subs).map(|((a, f), s)| (a + f) * s.cp).sum()
    }
    pub fn outer(&self) -> f64 {
        self.size.iter().map(|&d| d as f64).product()
    }
    /// Its walls' numbers (`Walls`).
    pub fn walls(&self, chem: &Chemistry) -> Walls {
        let mut r = [0.0; 5];
        for (k, &a) in self.wall.iter().enumerate().filter(|p| *p.1 > 0.0) {
            let (s, v) = (&chem.subs[k], a * chem.molar_volume(k, Phase::Solid));
            r = [r[0] + a * s.molar_mass, r[1] + v, r[2] + v * s.hardness, r[3] + v * s.strength, r[4] + v * s.conductivity];
        }
        let v = r[1].max(1e-12);
        Walls { mass: r[0], volume: r[1], hardness: r[2] / v, strength: r[3] / v, conductivity: r[4] / v }
    }
    /// Liquid volume its hollow can still take, m³.
    pub fn room(&self, chem: &Chemistry) -> f64 {
        let held: f64 = self.fill.iter().enumerate().map(|(k, a)| a * chem.molar_volume(k, Phase::Liquid)).sum();
        (self.hollow as f64 * self.outer() - held).max(0.0)
    }
    /// Area of the edge it is joined by, m².
    fn contact(&self) -> f64 {
        (self.size[0] * self.size[2]) as f64
    }
    /// Keep the shape when the walls change.
    fn resize(&mut self, chem: &Chemistry) {
        let (v, vo) = (self.walls(chem).volume, self.outer());
        if vo > 0.0 {
            let f = (v / (1.0 - self.hollow as f64) / vo).cbrt() as f32;
            self.size = self.size.map(|d| d * f);
        }
    }
}

/// Size of a part with wall volume `v`, longest first: elongation `x` (-1 flat .. 1 long), hollowness `h`.
fn dims(v: f64, x: f32, h: f32) -> [f32; 3] {
    let (r, vo) = (10f64.powf(ASPECT_DECADES * x.abs() as f64), v / (1.0 - h as f64));
    if x >= 0.0 {
        let a = (vo / r).cbrt();
        [(r * a) as f32, a as f32, a as f32]
    } else {
        let a = (vo / (r * r)).cbrt();
        [(r * a) as f32, (r * a) as f32, a as f32]
    }
}

impl Parts {
    pub fn alive(&self) -> impl Iterator<Item = &Part> {
        self.parts.iter().filter(|p| p.alive)
    }
}

/// The id of part `p`.
pub fn id(w: &World, p: usize) -> NodeId {
    NodeId { slot: p as u32, gen: w.parts.parts[p].gen }
}

/// The cell part `p` lies in.
pub fn cell(w: &World, p: usize) -> usize {
    w.parts.parts[p].cell as usize
}

/// The part named `id`, if it still exists.
pub fn part(w: &World, id: NodeId) -> Option<usize> {
    w.parts.parts.get(id.slot as usize).is_some_and(|p| p.alive && p.gen == id.gen).then_some(id.slot as usize)
}

/// The parts of the structure whose root is `r` (all in one cell).
fn members(w: &World, r: usize) -> Vec<usize> {
    w.parts.at[cell(w, r)].iter().map(|&x| x as usize).filter(|&x| w.parts.parts[x].root as usize == r).collect()
}

/// Place part `p` in cell `i`, in the slot freed last (whose generation moved on, so an old id names nothing) or a new one.
fn add(w: &mut World, mut p: Part, i: usize) -> usize {
    let x = free_place(&mut w.parts.parts, &mut w.parts.free);
    (p.alive, p.root, p.gen, p.cell) = (true, x as u32, w.parts.parts[x].gen, i as u32);
    w.parts.at[i].push(x as u32);
    w.parts.parts[x] = p;
    x
}

/// Move the structure whose root is `r` to cell `to`.
fn move_structure(w: &mut World, r: usize, to: usize) {
    let from = cell(w, r);
    for x in members(w, r) {
        w.parts.at[from].retain(|&y| y as usize != x);
        w.parts.at[to].push(x as u32);
        w.parts.parts[x].cell = to as u32;
    }
}

fn event(w: &mut World, made: bool, i: usize, kind: f64) {
    let (day, what) = (w.day(), if made { EventKind::BondMade } else { EventKind::BondBroken });
    w.events.push(Event { day, kind: what, node: i as u32, value: kind });
}

/// Strength of a bond, N, between materials of bond strength `a` and `b` over `area` m²: the weaker one's.
fn strength(w: &World, a: f64, b: f64, area: f64) -> f64 {
    a.min(b) * w.cfg.phys.tensile * w.cfg.phys.joint * area
}

/// Find the structure of part `p` again, and its numbers. Plates close a box as wide as the longest part (the ground
/// may be its sixth face); a hollow part closes its own hollow; the structure takes the larger space and its enclosure.
fn restructure(w: &mut World, p: usize) {
    let mut list = vec![p];
    let mut q = 0;
    while q < list.len() {
        for (o, _) in w.parts.bonds.of(list[q]) {
            if o != GROUND && !list.contains(&(o as usize)) {
                list.push(o as usize);
            }
        }
        q += 1;
    }
    let chem = &w.chem;
    // `foot`: the area of the parts' largest faces, m².
    let (mut s, mut l, mut cond, mut best, mut foot) = (Info { parts: list.len() as u32, ..Default::default() }, 0f64, 0f64, (0f64, 0f64), 0f64);
    for &x in &list {
        let pt = &w.parts.parts[x];
        let (walls, d) = (pt.walls(chem), pt.size.map(|v| v as f64));
        let (vo, face) = (pt.outer(), d[0] * d[1]);
        (s.mass, s.outer, foot) = (s.mass + pt.mass(chem), s.outer + vo, foot + face);
        s.anchored |= w.parts.bonds.of(x).any(|b| b.0 == GROUND);
        l = l.max(d[0]);
        // Across its walls: their conductivity times the face over their thickness (their volume over the face).
        cond += walls.conductivity * face * face / walls.volume.max(1e-9);
        if pt.hollow as f64 * vo > best.1 {
            best = (pt.hollow as f64, pt.hollow as f64 * vo);
        }
    }
    let need = if s.anchored { BOX_FACES - 1.0 } else { BOX_FACES } * l * l;
    let plates = if list.len() > 1 { (foot / need).min(1.0) } else { 0.0 };
    let (e, space) = if plates * l.powi(3) > best.1 { (plates, plates * l.powi(3)) } else { best };
    // The walls pass the share they close; the openings `leak` over the rest of the faces of a cube of that space.
    let g = e * cond + crate::math::cube_area(w.cfg.phys.leak * (1.0 - e), space);
    (s.space, s.encl) = (space, Enclosure { closed: e as f32, g });
    let root = *list.iter().min().expect("one part");
    list.iter().for_each(|&x| w.parts.parts[x].root = root as u32);
    w.parts.parts[root].s = s;
}

/// Join part `a` to part `b` (or `GROUND`) with a bond of `strength` N.
pub fn link(w: &mut World, a: usize, b: u32, strength: f64) {
    w.parts.bonds.add(a as u32, b, strength as f32);
    event(w, true, cell(w, a), if b == GROUND { ANCHORED } else { JOINED });
    restructure(w, a);
}

fn unlink(w: &mut World, a: usize, b: u32, load: bool) {
    w.parts.bonds.remove(a as u32, b);
    if b != GROUND {
        restructure(w, b as usize);
    }
    let kind = if b == GROUND { ANCHORED } else { JOINED } + if load { LOAD } else { 0.0 };
    event(w, false, cell(w, a), kind);
    restructure(w, a);
}

/// Make a part of `wall` (mol) with `e` J in cell `i`, of elongation `x` and hollowness `h`.
pub fn make(w: &mut World, i: usize, wall: Vec<f64>, e: f64, x: f32, h: f32) -> usize {
    let (x, h) = (x.clamp(-1.0, 1.0), h.clamp(0.0, MOST_HOLLOW));
    let mut p = Part { fill: vec![0.0; wall.len()], wall, e, hollow: h, held: NOBODY, ..Default::default() };
    p.size = dims(p.walls(&w.chem).volume, x, h);
    let s = add(w, p, i);
    restructure(w, s);
    s
}

/// A part falls apart (its amounts and heat went to its cell's ground, `month::turns`): its bonds are gone, and its node.
fn dissolve(w: &mut World, s: usize) {
    loop {
        let Some((o, _)) = w.parts.bonds.of(s).next() else { break };
        unlink(w, s, o, true);
    }
    if let Some(h) = holder(w, s) {
        w.life[h].hold = None;
    }
    let (i, p) = (cell(w, s), std::mem::take(&mut w.parts.parts[s]));
    w.parts.at[i].retain(|&x| x as usize != s);
    // Its slot is free for a new part, of the next generation.
    w.parts.parts[s].gen = p.gen + 1;
    w.parts.free.push(s as u32);
}

/// The part body `b` holds.
pub fn held(w: &World, b: &crate::body::Body) -> Option<usize> {
    part(w, b.hold?)
}

/// The body holding part `p` (not one taken out of the list for its update).
fn holder(w: &World, p: usize) -> Option<Handle> {
    let (s, me) = (w.life.find(w.parts.parts[p].held)?, Some(id(w, p)));
    (w.life[s].alive && w.life[s].hold == me).then_some(s)
}
