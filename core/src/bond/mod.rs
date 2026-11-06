//! Law 3: Bond. Parts are nodes of the store (`Class::Part`) that bonds join into structures; a bond breaks when its
//! load passes its strength (laws.md 3). Here: parts, structures and their graph; `bonds`: the bonds and the enclosure;
//! `design`: the design key by which alike parts and structures find each other; `lots`: alike lone parts as one record;
//! `verbs`: what bodies do to parts; `edge`: how a held part's edge changes what breaking takes; `sense`: what parts
//! give a body; `month`: the monthly rot, heat and loads. A part is stored in the list of the cell it lies in
//! (`life::Cell::parts`), with the bonds of that cell's parts (`life::Cell::bonds`): a structure lies in one cell, so
//! all that changes parts runs on that cell, in its chunk (`life::Chunk`), but a structure carried to another cell
//! (`moved`).

mod bonds;
mod design;
mod edge;
mod lots;
mod month;
mod sense;
mod verbs;

pub use bonds::{Bonds, Enclosure};
pub use design::designs;
pub use edge::{blunt, edge, edge_leaves, Edge};
pub(crate) use edge::blunted;
pub use month::step;
pub use sense::{depth, features, room};
pub(crate) use sense::{leave_outgrown, possible, room_air, sense, SENSE};
pub use verbs::{act, bears, carry, form, moved, release, LET_GO, TORN};
pub(crate) use verbs::{dropped, let_go};

use crate::{body::Body, chem::{Chemistry, Phase}, event::{Event, EventKind}, life::{Cell, Chunk}, store::{Class, Node}, World};
use serde::{Deserialize, Serialize};

/// Held by no body.
pub const NOBODY: u64 = u64::MAX;
/// Bond kinds (the value of bond events): body–part, part–part; a break by load adds `LOAD`.
pub(crate) const HELD: f64 = 0.0;
pub(crate) const JOINED: f64 = 1.0;
pub const LOAD: f64 = 2.0;
/// Features of a perceived part.
pub(crate) const NO: usize = 8;
/// Most hollowness of a part (a share of its outer volume).
const MOST_HOLLOW: f32 = 0.95;
/// An elongation of 1 (or -1) is an aspect of 10 to this power (100).
const ASPECT_DECADES: f64 = 2.0;
/// Faces of a box: plates close one as wide as the longest part.
const BOX_FACES: f64 = 6.0;

/// A part, in the list of the cell it lies in: one part, or a lot of alike lone parts (`lots`), whose amounts are all
/// its parts' and whose shape is each one's.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Part {
    /// Its id, given when it is made (`add`), which never changes
    pub id: u64,
    /// The cell it lies in
    pub cell: u32,
    /// How many alike parts it is (real, as the members of a node are)
    pub count: f64,
    /// mol of each substance: the walls, and the liquid held in the hollow, of all its parts
    pub wall: Vec<f64>,
    pub fill: Vec<f64>,
    /// Thermal energy of all its parts, J
    pub e: f64,
    /// Outer size of each part, m, longest first; share of its outer volume that is hollow
    pub size: [f32; 3],
    pub hollow: f32,
    /// Body holding it (id) or `NOBODY`
    pub held: u64,
    /// The least id of its structure; that part holds the structure's numbers
    pub root: u64,
    pub s: Info,
    /// Largest push this month from bodies moving through its cell (on the root), N
    pub push: f32,
    /// How deep its parts have sunk into its cell's loose ground while they lay there alone, m: the mean, and half the
    /// width of the even spread of their depths around it (`month::bury`)
    pub buried: f32,
    pub spread: f32,
}

/// Numbers of a structure (of each of a lot's parts).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Info {
    pub parts: u32,
    /// kg (walls and what they hold); outer volume m³
    pub mass: f64,
    pub outer: f64,
    /// The space its walls close (m³), and the link between it and the cell
    pub space: f64,
    pub encl: Enclosure,
}

/// What bodies did with parts since the shell last read it (not saved).
#[derive(Clone, Debug, Default)]
pub struct Tally {
    /// Parts formed; kg put into and taken from parts
    pub formed: f64,
    pub to_parts: f64,
    pub from_parts: f64,
    /// kg taken from bodies and the cover by bites an edge cut (`Edge`)
    pub edged: f64,
}

/// Of a part's walls (all a lot's): their mass, kg, and volume, m³, and by volume their hardness, bond strength (0..1), heat
/// conductivity, W/(m K), breaking stress, Pa, and colour (the solid reflectance of 3 bands, 0..1) (the rule of
/// mixtures).
#[derive(Clone, Copy, Debug)]
pub struct Walls {
    pub mass: f64,
    pub volume: f64,
    pub hardness: f64,
    pub strength: f64,
    pub conductivity: f64,
    pub rupture: f64,
    pub colour: [f64; 3],
}

impl Part {
    /// The mass of all its parts, kg.
    pub fn mass(&self, chem: &Chemistry) -> f64 {
        self.wall.iter().zip(&self.fill).zip(&chem.subs).map(|((a, f), s)| (a + f) * s.molar_mass).sum()
    }
    pub fn heat_cap(&self, chem: &Chemistry) -> f64 {
        self.wall.iter().zip(&self.fill).zip(&chem.subs).map(|((a, f), s)| (a + f) * s.cp).sum()
    }
    /// The outer volume of one of its parts, m³.
    pub fn outer(&self) -> f64 {
        self.size.iter().map(|&d| d as f64).product()
    }
    /// Its walls' numbers (`Walls`).
    pub fn walls(&self, chem: &Chemistry) -> Walls {
        let (mut r, mut c) = ([0.0; 6], [0.0; 3]);
        for (k, &a) in self.wall.iter().enumerate().filter(|p| *p.1 > 0.0) {
            let (s, v) = (&chem.subs[k], a * chem.molar_volume(k, Phase::Solid));
            r = [r[0] + a * s.molar_mass, r[1] + v, r[2] + v * s.hardness, r[3] + v * s.strength, r[4] + v * s.conductivity, r[5] + v * s.rupture];
            c.iter_mut().zip(s.refl[0]).for_each(|(x, b)| *x += v * b as f64);
        }
        let v = r[1].max(1e-12);
        Walls { mass: r[0], volume: r[1], hardness: r[2] / v, strength: r[3] / v, conductivity: r[4] / v, rupture: r[5] / v, colour: c.map(|x| x / v) }
    }
    /// The thickness of the walls of a part, m: their volume over half its outer surface (a plate's thickness, half a
    /// rod's width, twice a hollow box's wall).
    pub fn thickness(&self, chem: &Chemistry) -> f64 {
        let d = self.size.map(|x| x as f64);
        2.0 * self.walls(chem).volume / self.count / (2.0 * (d[0] * d[1] + d[1] * d[2] + d[2] * d[0])).max(1e-30)
    }
    /// Liquid volume the hollows of its parts can still take, m³.
    pub fn room(&self, chem: &Chemistry) -> f64 {
        let held: f64 = self.fill.iter().enumerate().map(|(k, a)| a * chem.molar_volume(k, Phase::Liquid)).sum();
        (self.hollow as f64 * self.outer() * self.count - held).max(0.0)
    }
    /// Area of the edge it is joined by, m².
    fn contact(&self) -> f64 {
        (self.size[0] * self.size[2]) as f64
    }
    /// Keep the shape when the walls change.
    fn resize(&mut self, chem: &Chemistry) {
        self.fit(self.walls(chem).volume);
    }
    /// Keep the shape around walls of volume `v` in all, m³ (`resize`, their volume known).
    fn fit(&mut self, v: f64) {
        let vo = self.outer();
        if vo > 0.0 {
            let f = (v / self.count / (1.0 - self.hollow as f64) / vo).cbrt() as f32;
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

/// The place in cell `c`'s list of the part named `id`, if it lies there.
pub fn at(c: &Cell, id: u64) -> Option<usize> {
    c.parts.iter().position(|p| p.id == id)
}

/// The places of the parts of the structure whose root is `r` (all in one cell).
fn members(c: &Cell, r: u64) -> Vec<usize> {
    (0..c.parts.len()).filter(|&x| c.parts[x].root == r).collect()
}

/// The place of the root of the structure of the part at `p` (`p` itself if it is the root).
fn root(c: &Cell, p: usize) -> usize {
    let r = c.parts[p].root;
    if r == c.parts[p].id {
        return p;
    }
    at(c, r).expect("a structure's root lies in its cell")
}

/// Place part `p` in cell `i` of chunk `d`, the last of its list, with the next id of the cell (`life::new_id`, as a
/// body's). Returns its place.
fn add(d: &mut Chunk, i: usize, mut p: Part) -> usize {
    let c = d.at_mut(i);
    c.parts_made += 1;
    p.id = crate::life::new_id(c.serial, c.parts_made);
    (p.root, p.cell) = (p.id, i as u32);
    c.parts.push(p);
    c.parts.len() - 1
}

/// Move the structure whose root is `r` from cell `from` to cell `to`, its parts and their bonds, in their order, with
/// the potential energy of the heads it leaves and lands at (`store::life_went`).
fn move_structure(w: &mut World, from: usize, r: u64, to: usize) {
    let c = &mut w.life.cells[from];
    let ids: Vec<u64> = members(c, r).into_iter().map(|x| c.parts[x].id).collect();
    let mut gone: Vec<Part> = Vec::with_capacity(ids.len());
    c.parts.retain(|p| if ids.contains(&p.id) { gone.push(p.clone()); false } else { true });
    let links = c.bonds.take(&ids);
    let kg = gone.iter().map(|p| p.mass(&w.chem)).sum();
    crate::store::life_went(w, &[(from, to, kg)]);
    let c = &mut w.life.cells[to];
    for mut p in gone {
        p.cell = to as u32;
        c.parts.push(p);
    }
    c.bonds.extend(links);
    ids.iter().for_each(|&id| w.life.note_part_at(id, to as u32));
}

fn event(d: &mut Chunk, w: &World, made: bool, i: usize, kind: f64) {
    let (day, what) = (w.day(), if made { EventKind::BondMade } else { EventKind::BondBroken });
    d.events.push(Event { day, kind: what, node: i as u32, value: kind });
}

/// Strength of a bond, N, between materials of bond strength `a` and `b` over `area` m²: the weaker one's.
fn strength(w: &World, a: f64, b: f64, area: f64) -> f64 {
    a.min(b) * w.cfg.phys.tensile * w.cfg.phys.joint * area
}

/// Find the structure of the part at `p` of cell `c` again along the bonds, and its numbers (`numbers`).
fn restructure(w: &World, c: &mut Cell, p: usize) {
    let mut list = vec![c.parts[p].id];
    let mut q = 0;
    while q < list.len() {
        for (o, _) in c.bonds.of(list[q]) {
            if !list.contains(&o) {
                list.push(o);
            }
        }
        q += 1;
    }
    let places: Vec<usize> = list.iter().map(|&x| at(c, x).expect("a bonded part lies in its cell")).collect();
    let r = *list.iter().min().expect("one part");
    places.iter().for_each(|&y| c.parts[y].root = r);
    let y = at(c, r).expect("its root");
    c.parts[y].s = numbers(w, places.iter().map(|&x| (&c.parts[x], c.parts[x].walls(&w.chem))));
}

/// The numbers of the structure of the parts `of`, each with its walls (`Info`; of a lot, each of its parts'). Plates
/// close a box as wide as the longest part; a hollow part closes its own hollow; the structure takes the larger space
/// and its enclosure.
fn numbers<'a>(w: &World, of: impl Iterator<Item = (&'a Part, Walls)>) -> Info {
    let chem = &w.chem;
    // `foot`: the area of the parts' largest faces, m².
    let (mut s, mut l, mut cond, mut best, mut foot) = (Info::default(), 0f64, 0f64, (0f64, 0f64), 0f64);
    for (pt, walls) in of {
        let d = pt.size.map(|v| v as f64);
        let (vo, face, n) = (pt.outer(), d[0] * d[1], pt.count);
        (s.parts, s.mass, s.outer, foot) = (s.parts + 1, s.mass + pt.mass(chem) / n, s.outer + vo, foot + face);
        l = l.max(d[0]);
        // Across its walls: the face over their resistance in series with the still air on each side of it (`r_air`,
        // ISO 6946's surface resistances): their thickness (their volume over the face) over their conductivity.
        cond += face / (2.0 * w.cfg.phys.r_air + walls.volume / n / (face * walls.conductivity).max(1e-12));
        if pt.hollow as f64 * vo > best.1 {
            best = (pt.hollow as f64, pt.hollow as f64 * vo);
        }
    }
    let need = BOX_FACES * l * l;
    let plates = if s.parts > 1 { (foot / need).min(1.0) } else { 0.0 };
    let (e, space) = if plates * l.powi(3) > best.1 { (plates, plates * l.powi(3)) } else { best };
    // The walls pass the share they close; the openings `leak` over the rest of the faces of a cube of that space.
    let g = e * cond + crate::math::cube_area(w.cfg.phys.leak * (1.0 - e), space);
    (s.space, s.encl) = (space, Enclosure { closed: e as f32, g });
    s
}

/// Join the part at `a` of cell `i` of chunk `d` to the part named `b` with a bond of `strength` N.
fn link_in(w: &World, d: &mut Chunk, i: usize, a: usize, b: u64, strength: f64) {
    let c = d.at_mut(i);
    let id = c.parts[a].id;
    c.bonds.add(id, b, strength as f32);
    restructure(w, c, a);
    event(d, w, true, i, JOINED);
}

/// Break the bond between the parts named `a` and `b` of cell `i`, by a load if `load`.
fn unlink(w: &World, d: &mut Chunk, i: usize, a: u64, b: u64, load: bool) {
    let c = d.at_mut(i);
    c.bonds.remove(a, b);
    if let Some(y) = at(c, b) {
        restructure(w, c, y);
    }
    if let Some(x) = at(c, a) {
        restructure(w, c, x);
    }
    event(d, w, false, i, JOINED + if load { LOAD } else { 0.0 });
}

/// Join the part named `a` to the part named `b` of its cell with a bond of `strength` N (an edit).
pub fn link(w: &mut World, a: u64, b: u64, strength: f64) {
    let i = w.life.part_cell(a).expect("the part");
    w.in_cell(i, |w, d| {
        let x = at(d.at(i), a).expect("the part");
        link_in(w, d, i, x, b, strength);
    });
}

/// Make a part of `wall` (mol) with `e` J in cell `i` of chunk `d`, of elongation `x` and hollowness `h`. Returns its
/// place in the cell's list.
pub fn make(w: &World, d: &mut Chunk, i: usize, wall: Vec<f64>, e: f64, x: f32, h: f32) -> usize {
    let (x, h) = (x.clamp(-1.0, 1.0), h.clamp(0.0, MOST_HOLLOW));
    let mut p = Part { count: 1.0, fill: vec![0.0; wall.len()], wall, e, hollow: h, held: NOBODY, ..Default::default() };
    p.size = dims(p.walls(&w.chem).volume, x, h);
    let s = add(d, i, p);
    restructure(w, d.at_mut(i), s);
    s
}

/// The part at `p` of cell `i` of chunk `d` falls apart: all it holds, its walls, the liquid in its hollow and its heat,
/// goes to its cell's ground (`fall`). Then it is gone (`dissolve`). Every way a part falls apart uses it: its walls run
/// off or it is buried in its turn (`month::turns`), a body takes them below `min_kg` (`verbs::take`), or its edge
/// wears them away (`edge::blunted`).
fn fall_apart(w: &World, d: &mut Chunk, i: usize, p: usize) {
    let (id, all) = (d.at(i).parts[p].id, d.at(i).parts[p].count);
    fall(w, d, i, (p, all));
    dissolve(w, d, i, id);
}

/// `k` parts of the lot at `p` of cell `i` of chunk `d` fall apart: their share of all it holds, its walls, the liquid
/// in its hollow and its heat (`Part::split`), goes to its cell's ground, moved by its own update on the chunk's view.
fn fall(w: &World, d: &mut Chunk, i: usize, (p, k): (usize, f64)) {
    d.moves.open(w, i, Node::new(Class::Part, p));
    let gone = d.at_mut(i).parts[p].split(k);
    d.moves.give_all(w, Node::ground(i), gone.wall.iter().zip(&gone.fill).map(|(a, f)| a + f).enumerate(), gone.e);
}

/// The share `f(k, t, walls)` of each substance `k` of the walls of part `p` leaves them with its heat at the part's
/// temperature `t` (the walls as they are by then): its mol and J are added to `off`, one entry per substance. Returns
/// the part's heat capacity after, J/K.
fn shed(chem: &Chemistry, p: &mut Part, f: impl Fn(usize, f64, &[f64]) -> f64, off: &mut [[f64; 2]]) -> f64 {
    let cap = p.heat_cap(chem);
    let (t, mut changed) = (p.e / cap.max(1e-9), false);
    for (k, o) in off.iter_mut().enumerate() {
        if p.wall[k] <= 0.0 {
            continue;
        }
        let f = f(k, t, &p.wall);
        if f > 0.0 {
            let (a, q) = (f * p.wall[k], f * p.wall[k] * chem.subs[k].cp * t);
            (p.e, p.wall[k], changed) = (p.e - q, p.wall[k] - a, true);
            *o = [o[0] + a, o[1] + q];
        }
    }
    if changed { p.heat_cap(chem) } else { cap }
}

/// What parts of cell `i` shed (`shed`: mol and J per substance) goes to its ground, one move per substance, on the
/// chunk's view opened for them.
fn to_ground(w: &World, d: &mut Chunk, i: usize, off: &[[f64; 2]]) {
    for (k, &[a, q]) in off.iter().enumerate().filter(|o| o.1[0] > 0.0) {
        d.moves.give(w, Node::ground(i), k, a, q);
    }
}

/// The loose solids of the ground of cell `i` as the chunk's view holds them, mol of each substance: those solid there
/// at its temperature `tg` K, in its mixture (`Chemistry::phase_in`).
fn loose_solids(w: &World, d: &Chunk, i: usize, tg: f64) -> Vec<f64> {
    let g = d.moves.amounts(w, Node::ground(i));
    (0..w.chem.ns()).map(|k| if w.chem.phase_in(k, tg, &g) == Phase::Solid { g(k).max(0.0) } else { 0.0 }).collect()
}

/// The part named `id` of cell `i` is gone: its bonds, its holder's hold and its place in the list.
fn dissolve(w: &World, d: &mut Chunk, i: usize, id: u64) {
    loop {
        let Some((o, _)) = d.at(i).bonds.of(id).next() else { break };
        unlink(w, d, i, id, o, true);
    }
    let c = d.at_mut(i);
    if let Some(h) = at(c, id).and_then(|p| holder(c, p)) {
        c.bodies[h].hold = None;
    }
    c.parts.retain(|p| p.id != id);
}

/// The part body `b` holds, in its cell `c`: its place there.
pub fn held(c: &Cell, b: &Body) -> Option<usize> {
    at(c, b.hold?)
}

/// The body of cell `c` holding the part at `p`: its place in the cell's list (not one taken out of the list for its
/// update).
fn holder(c: &Cell, p: usize) -> Option<usize> {
    let (by, me) = (c.parts[p].held, Some(c.parts[p].id));
    if by == NOBODY {
        return None;
    }
    c.bodies.iter().position(|b| b.alive && b.id == by && b.hold == me)
}
