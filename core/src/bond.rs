//! Law 3: Bond. Nodes can be joined; joins break when pushed too hard.
//!
//! Parts (made objects) are nodes of the sparse graph like bodies (slot and generation): solid
//! walls, the liquid held in their hollow, heat, a size (3 numbers, longest first) and a
//! hollowness. A part lies in a cell, or is held by a body (a body–part bond). Bonds join two
//! parts, or a part and the bonded ground of its cell; their strength (N) comes from the weaker
//! material at the joint and the contact area. A structure is the parts joined by bonds; it
//! moves as one.
//!
//! Once a month each part takes its cell's temperature and reacts with the air (Change), and each
//! structure is loaded: a bond carries the weight hanging on it from the anchored part (or the
//! held one), less what the liquid around holds up, plus the push of moving liquid and of bodies
//! moving through the cell. A bond loaded beyond its strength breaks; what is left without
//! support lies on the ground. A body–part bond is as strong as the weight the holder bears by its
//! own force (strength × its mass × gravity × grip; a lever does not lift itself): a body takes
//! hold of a part only if that bears the weight of the part's structure, and lets go when it no
//! longer does. A structure the liquid holds up floats and drifts down the liquid's slope. A
//! structure whose walls close around a space has an inside: a body in it loses heat to the
//! outside only through the walls and the openings (a conductance in series with its own), and
//! gets the light and rain that come through the openings. The inside holds almost no heat
//! itself, so it is solved in closed form.

use crate::body::{self, Body, Layout, STRENGTH};
use crate::change::Reactor;
use crate::chem::{Chemistry, Phase};
use crate::choose::{kg, wanted};
use crate::event::{Event, EventKind};
use crate::grid::DIRS;
use crate::rng::{Purpose, Rng};
use crate::World;
use serde::{Deserialize, Serialize};

/// Held by no body; the bonded ground (as the other end of a bond).
pub const NOBODY: u64 = u64::MAX;
pub const GROUND: u32 = u32::MAX;
/// Bond kinds (the value of bond events): body–part, part–part, part–ground; a break by load
/// adds `LOAD`.
pub const HELD: f64 = 0.0;
pub const JOINED: f64 = 1.0;
pub const ANCHORED: f64 = 2.0;
pub const LOAD: f64 = 3.0;
/// Features of a perceived part.
pub const NO: usize = 8;
/// Parts react with the air every this many months.
const REACT_MONTHS: u64 = 6;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Part {
    pub gen: u32,
    pub alive: bool,
    pub cell: u32,
    pub born: u64,
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
    /// Bonds: the other part (slot) or `GROUND`, and the bond's strength, N
    pub links: Vec<(u32, f32)>,
    /// Lowest slot of its structure; that part holds the structure's numbers
    pub root: u32,
    pub s: Info,
    /// Largest push this month from bodies moving through its cell (on the root), N
    pub push: f32,
}

/// Numbers of a structure.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Info {
    pub parts: u32,
    /// kg (walls and what they hold); outer volume m³; area of the largest faces m²
    pub mass: f64,
    pub outer: f64,
    pub foot: f64,
    /// How closed its walls are around the space inside (0..1), that space m³, and the heat
    /// conductance between inside and outside, W/K
    pub encl: f32,
    pub space: f64,
    pub g: f64,
    pub anchored: bool,
    /// Held up by the liquid around it (as of the last month)
    pub afloat: bool,
}

/// What bodies did with parts since the shell last read it (not saved).
#[derive(Clone, Debug, Default)]
pub struct Tally {
    /// Takes from bodies and breaks of the bonded ground, [without, with] a held part: count, and
    /// what they yielded as a share of the body's own mass
    pub take: [[f64; 2]; 2],
    pub dig: [[f64; 2]; 2],
    /// Moves into deep liquid afloat; parts formed; kg put into and taken from parts
    pub afloat: f64,
    pub formed: f64,
    pub to_parts: f64,
    pub from_parts: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Parts {
    pub parts: Vec<Part>,
    free: Vec<u32>,
    /// Parts in each cell, in a fixed order
    pub at: Vec<Vec<u32>>,
    #[serde(skip)]
    pub tally: Tally,
}

impl Part {
    pub fn id(&self, slot: usize) -> u64 {
        (self.gen as u64) << 32 | slot as u64
    }
    pub fn mass(&self, chem: &Chemistry) -> f64 {
        self.wall.iter().zip(&self.fill).zip(&chem.subs).map(|((a, f), s)| (a + f) * s.molar_mass).sum()
    }
    pub fn heat_cap(&self, chem: &Chemistry) -> f64 {
        self.wall.iter().zip(&self.fill).zip(&chem.subs).map(|((a, f), s)| (a + f) * s.cp).sum()
    }
    pub fn outer(&self) -> f64 {
        self.size.iter().map(|&d| d as f64).product()
    }
    /// Of its walls: mass kg, volume m³, and by volume hardness, bond strength, heat conductivity.
    pub fn walls(&self, chem: &Chemistry) -> [f64; 5] {
        let mut r = [0.0; 5];
        for (k, &a) in self.wall.iter().enumerate().filter(|p| *p.1 > 0.0) {
            let (s, v) = (&chem.subs[k], a * chem.molar_volume(k, Phase::Solid));
            r = [r[0] + a * s.molar_mass, r[1] + v, r[2] + v * s.hardness, r[3] + v * s.strength, r[4] + v * s.conductivity];
        }
        let v = r[1].max(1e-12);
        [r[0], r[1], r[2] / v, r[3] / v, r[4] / v]
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
        let (v, vo) = (self.walls(chem)[1], self.outer());
        if vo > 0.0 {
            let f = (v / (1.0 - self.hollow as f64) / vo).cbrt() as f32;
            self.size = self.size.map(|d| d * f);
        }
    }
}

/// Size of a part with wall volume `v`, longest first: elongation `x` (-1 flat .. 1 long, an
/// aspect of up to 100), hollowness `h`.
fn dims(v: f64, x: f32, h: f32) -> [f32; 3] {
    let (r, vo) = (10f64.powf(2.0 * x.abs() as f64), v / (1.0 - h as f64));
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
    /// The slot of part `id`, if it still exists.
    pub fn get(&self, id: u64) -> Option<usize> {
        let s = id as u32 as usize;
        (id != 0 && self.parts.get(s).is_some_and(|p| p.alive && p.gen == (id >> 32) as u32)).then_some(s)
    }
    /// The parts of the structure whose root is `r` (all in one cell).
    pub fn members(&self, r: usize) -> Vec<usize> {
        self.at[self.parts[r].cell as usize].iter().map(|&x| x as usize).filter(|&x| self.parts[x].root as usize == r).collect()
    }
    fn add(&mut self, mut p: Part) -> usize {
        let slot = match self.free.pop() {
            Some(s) => s as usize,
            None => {
                self.parts.push(Part::default());
                self.parts.len() - 1
            }
        };
        (p.gen, p.alive, p.root) = (self.parts[slot].gen + 1, true, slot as u32);
        self.at[p.cell as usize].push(slot as u32);
        self.parts[slot] = p;
        slot
    }
    fn relocate(&mut self, s: usize, to: usize) {
        let from = self.parts[s].cell as usize;
        self.at[from].retain(|&x| x as usize != s);
        self.at[to].push(s as u32);
        self.parts[s].cell = to as u32;
    }
}

/// Size the per-cell lists.
pub fn ready(w: &mut World) {
    if w.parts.at.len() != w.grid.n() {
        w.parts.at = vec![Vec::new(); w.grid.n()];
    }
}

fn event(w: &mut World, made: bool, cell: u32, kind: f64) {
    let kind_ = if made { EventKind::BondMade } else { EventKind::BondBroken };
    w.events.push(Event { day: w.day, kind: kind_, node: cell, value: kind });
}

/// Find the structure of part `p` again, and its numbers. Plates close a box as wide as the
/// longest part (five faces when the ground is the sixth); a hollow part closes its own hollow.
/// The structure takes the larger of the two spaces.
fn restructure(w: &mut World, p: usize) {
    let mut list = vec![p];
    let mut q = 0;
    while q < list.len() {
        for &(o, _) in &w.parts.parts[list[q]].links {
            if o != GROUND && !list.contains(&(o as usize)) {
                list.push(o as usize);
            }
        }
        q += 1;
    }
    let chem = &w.chem;
    let (mut s, mut l, mut cond, mut best) = (Info { parts: list.len() as u32, ..Default::default() }, 0f64, 0f64, (0f64, 0f64));
    for &x in &list {
        let pt = &w.parts.parts[x];
        let ([_, v, _, _, k], d) = (pt.walls(chem), pt.size.map(|v| v as f64));
        let (vo, face) = (pt.outer(), d[0] * d[1]);
        (s.mass, s.outer, s.foot) = (s.mass + pt.mass(chem), s.outer + vo, s.foot + face);
        s.anchored |= pt.links.iter().any(|b| b.0 == GROUND);
        l = l.max(d[0]);
        cond += k * face * face / v.max(1e-9);
        if pt.hollow as f64 * vo > best.1 {
            best = (pt.hollow as f64, pt.hollow as f64 * vo);
        }
    }
    let need = if s.anchored { 5.0 } else { 6.0 } * l * l;
    let plates = if list.len() > 1 { (s.foot / need).min(1.0) } else { 0.0 };
    let (e, space) = if plates * l.powi(3) > best.1 { (plates, plates * l.powi(3)) } else { best };
    (s.encl, s.space) = (e as f32, space);
    s.g = e * cond + w.cfg.phys.leak * (1.0 - e) * 6.0 * space.powf(2.0 / 3.0);
    let root = *list.iter().min().expect("one part");
    for &x in &list {
        w.parts.parts[x].root = root as u32;
    }
    w.parts.parts[root].s = s;
}

/// Join part `a` to part `b` (or `GROUND`) with a bond of `strength` N.
pub fn link(w: &mut World, a: usize, b: u32, strength: f64) {
    w.parts.parts[a].links.push((b, strength as f32));
    if b != GROUND {
        w.parts.parts[b as usize].links.push((a as u32, strength as f32));
    }
    event(w, true, w.parts.parts[a].cell, if b == GROUND { ANCHORED } else { JOINED });
    restructure(w, a);
}

fn unlink(w: &mut World, a: usize, b: u32, load: bool) {
    let drop = |l: &mut Vec<(u32, f32)>, o: u32| {
        if let Some(x) = l.iter().position(|p| p.0 == o) {
            l.remove(x);
        }
    };
    drop(&mut w.parts.parts[a].links, b);
    if b != GROUND {
        drop(&mut w.parts.parts[b as usize].links, a as u32);
        restructure(w, b as usize);
    }
    let kind = if b == GROUND { ANCHORED } else { JOINED } + if load { LOAD } else { 0.0 };
    event(w, false, w.parts.parts[a].cell, kind);
    restructure(w, a);
}

/// Make a part of `wall` (mol) with `e` J in cell `i`, of elongation `x` and hollowness `h`.
pub fn make(w: &mut World, i: usize, wall: Vec<f64>, e: f64, x: f32, h: f32) -> usize {
    let (x, h) = (x.clamp(-1.0, 1.0), h.clamp(0.0, 0.95));
    let mut p = Part { cell: i as u32, born: w.day, fill: vec![0.0; wall.len()], wall, e, hollow: h, held: NOBODY, ..Default::default() };
    p.size = dims(p.walls(&w.chem)[1], x, h);
    let s = w.parts.add(p);
    restructure(w, s);
    s
}

/// A part falls apart: its amounts and heat go to its cell's ground.
fn dissolve(w: &mut World, s: usize) {
    while let Some(&(o, _)) = w.parts.parts[s].links.first() {
        unlink(w, s, o, true);
    }
    if let Some(h) = holder(w, s) {
        w.life.bodies[h].hold = 0;
    }
    let (n, p) = (w.grid.n(), std::mem::take(&mut w.parts.parts[s]));
    let i = p.cell as usize;
    for (k, (a, f)) in p.wall.iter().zip(&p.fill).enumerate() {
        w.ground[k * n + i] += a + f;
    }
    w.e_ground[i] += p.e;
    w.parts.at[i].retain(|&x| x as usize != s);
    w.parts.parts[s].gen = p.gen;
    w.parts.free.push(s as u32);
}

/// The part body `b` holds.
pub fn held(w: &World, b: &Body) -> Option<usize> {
    if b.hold == 0 { None } else { w.parts.get(b.hold) }
}

/// The body holding part `p` (not one taken out of the list for its update).
fn holder(w: &World, p: usize) -> Option<usize> {
    let (h, id) = (w.parts.parts[p].held, w.parts.parts[p].id(p));
    let s = h as u32 as usize;
    (h != NOBODY && w.life.bodies.get(s).is_some_and(|o| o.alive && o.gen == (h >> 32) as u32 && o.hold == id)).then_some(s)
}

/// Body `b` (mass `mb`) takes hold of part `p` if it bears the weight of `p`'s structure (one
/// bonded to the ground rests on it).
fn grab(w: &mut World, slot: usize, b: &mut Body, p: usize, mb: f64) {
    let s = w.parts.parts[w.parts.parts[p].root as usize].s;
    if !s.anchored && s.mass * w.cfg.gravity > bears(w, b, mb) {
        return;
    }
    w.parts.parts[p].held = b.id(slot);
    b.hold = w.parts.parts[p].id(p);
    event(w, true, b.cell, HELD);
}

/// Let go of the held part (also when the body dies).
pub fn let_go(w: &mut World, b: &mut Body, load: bool) {
    if let Some(p) = held(w, b) {
        w.parts.parts[p].held = NOBODY;
        event(w, false, b.cell, HELD + if load { LOAD } else { 0.0 });
    }
    b.hold = 0;
}

/// What a held part multiplies a body's force by: more for a hard part, up to its mass.
pub fn lever(w: &World, b: &Body, mb: f64) -> f64 {
    held(w, b).map_or(1.0, |p| {
        let [m, _, hard, _, _] = w.parts.parts[p].walls(&w.chem);
        1.0 + w.cfg.phys.lever * hard * m / (m + 0.05 * mb)
    })
}

/// Weight a body bears by its own force, N: what it holds adds nothing (a lever does not lift
/// itself).
fn bears(w: &World, b: &Body, mb: f64) -> f64 {
    b.gene(STRENGTH) * mb * w.cfg.gravity * w.cfg.phys.grip
}

/// Force a body pushes or pulls with, N.
fn force(w: &World, b: &Body, mb: f64) -> f64 {
    b.gene(STRENGTH) * lever(w, b, mb) * mb * w.cfg.gravity * w.cfg.phys.grip
}

/// The structure body `b` is inside: its root, how closed it is, its conductance (W/K).
pub fn room(w: &World, b: &Body) -> Option<(usize, f32, f64)> {
    let p = if b.inside == 0 { None } else { w.parts.get(b.inside) }?;
    let r = w.parts.parts[p].root as usize;
    let s = &w.parts.parts[r].s;
    (w.parts.parts[p].cell == b.cell && s.encl > 0.0).then_some((r, s.encl, s.g))
}

/// For a body's update: how closed the walls around it are, and its share of their conductance.
pub fn walls_around(w: &World, b: &Body) -> Option<(f64, f64)> {
    let (r, e, g) = room(w, b)?;
    let others = w.life.at[b.cell as usize].iter().filter(|&&o| w.life.bodies[o as usize].alive && room(w, &w.life.bodies[o as usize]).is_some_and(|x| x.0 == r)).count();
    Some((e as f64, g / (others + 1) as f64))
}

/// Volume of a body's solids and liquids, m³.
pub fn volume(chem: &Chemistry, b: &Body) -> f64 {
    let t = b.temp(chem);
    b.store.iter().enumerate().map(|(k, a)| a * chem.molar_volume(k, chem.phase(k, t))).sum()
}

/// Standing liquid on cell `i`: depth m, density kg/m³, mol per m³.
fn liquid(w: &World, i: usize) -> (f64, f64, f64) {
    let (env, n) = (w.life.env[i], w.grid.n());
    let (mut m, mut v, mut mol) = (0.0, 0.0, 0.0);
    for k in (0..w.chem.ns()).filter(|&k| w.chem.phase(k, env[0]) == Phase::Liquid) {
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
pub fn sense(w: &World, b: &Body, mb: f64) -> [f32; 5] {
    let mut x = [0f32; 5];
    if let Some(p) = held(w, b) {
        let pt = &w.parts.parts[p];
        let [m, _, hard, _, _] = pt.walls(&w.chem);
        x[..4].copy_from_slice(&[1.0, crate::brain::ln((m / mb) as f32) / 7.0 + 1.0, hard as f32, pt.hollow]);
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
    [ln((m / mb) as f32) / 7.0, pt.walls(chem)[2] as f32, pt.hollow, ln(pt.size[0]) / 3.0, s.encl, s.anchored as u8 as f32, (pt.held != NOBODY) as u8 as f32, (want / m.max(1e-9)) as f32]
}

/// Can verb `v` be done to part `p` at all (as the body sees it)?
pub fn possible(w: &World, b: &Body, v: usize, p: usize, vol: f64) -> bool {
    let pt = &w.parts.parts[p];
    let s = &w.parts.parts[pt.root as usize].s;
    match v {
        0 => pt.held == NOBODY,
        3 => held(w, b).is_some_and(|h| w.parts.parts[h].root != pt.root),
        5 => s.encl > 0.0 && s.space >= vol && room(w, b).is_none_or(|r| r.0 != pt.root as usize),
        _ => true,
    }
}

/// Verb `v` toward part `p`: hold, take from, put to, join the held part to, break, enter.
pub fn act(w: &mut World, slot: usize, b: &mut Body, v: usize, p: usize) {
    let (mb, days) = (b.mass(&w.chem), b.interval() as f64);
    match v {
        0 => {
            if w.parts.parts[p].held == NOBODY && body::pay(w, b, 0.1 * w.cfg.phys.c_handle * w.parts.parts[p].mass(&w.chem), days) {
                let_go(w, b, false);
                grab(w, slot, b, p, mb);
            }
        }
        1 => take(w, b, p, mb, days),
        2 => put(w, b, p, mb, days),
        3 => {
            let Some(h) = held(w, b) else { return };
            let (a, c) = (&w.parts.parts[h], &w.parts.parts[p]);
            let ac = a.contact().min(c.contact());
            let s = a.walls(&w.chem)[3].min(c.walls(&w.chem)[3]) * w.cfg.phys.tensile * w.cfg.phys.joint * ac;
            if a.root != c.root && body::pay(w, b, w.cfg.phys.c_join * ac, days) {
                link(w, h, p as u32, s);
            }
        }
        4 => {
            let f = force(w, b, mb);
            if !body::pay(w, b, f, days) {
                return;
            }
            if let Some(o) = holder(w, p) {
                let ob = &w.life.bodies[o];
                if f > force(w, ob, ob.mass(&w.chem)) {
                    let mut ob = std::mem::take(&mut w.life.bodies[o]);
                    let_go(w, &mut ob, true);
                    w.life.bodies[o] = ob;
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
            let used: f64 = w.life.at[i].iter().map(|&o| &w.life.bodies[o as usize]).filter(|o| o.alive && room(w, o).is_some_and(|x| x.0 == r)).map(|o| volume(&w.chem, o)).sum();
            let s = w.parts.parts[r].s;
            if s.encl > 0.0 && s.space >= used + volume(&w.chem, b) && body::pay(w, b, w.cfg.phys.walk(mb / b.n()) * mb * w.parts.parts[p].size[0] as f64, days) {
                b.inside = w.parts.parts[p].id(p);
            }
        }
    }
}

/// Take from a part by the body's uptake selectivity, the held liquid first, as much as it takes by its
/// mass (`Body::equiv`) in the time, as far as it can use it (`choose::sate`). Walls taken below
/// `w.cfg.phys.min_kg` fall apart.
fn take(w: &mut World, b: &mut Body, p: usize, mb: f64, days: f64) {
    let pt = &w.parts.parts[p];
    let m = wanted(w, b, |k| pt.wall[k] + pt.fill[k], w.cfg.phys.take * days * b.equiv(&w.cfg.phys, mb), mb, days);
    let got = kg(w, &m);
    if got <= 0.0 || !body::pay(w, b, w.cfg.phys.c_handle * got, days) {
        return;
    }
    let (chem, pt) = (&w.chem, &mut w.parts.parts[p]);
    let q = pt.e * m.iter().zip(&chem.subs).map(|(a, s)| a * s.cp).sum::<f64>() / pt.heat_cap(chem);
    for (k, &a) in m.iter().enumerate() {
        let f = a.min(pt.fill[k]);
        pt.fill[k] -= f;
        pt.wall[k] = (pt.wall[k] - (a - f)).max(0.0);
        b.store[k] += a;
    }
    b.digest(chem, chem.matter().map_or(0.0, |k| m[k]));
    (pt.e, b.e) = (pt.e - q, b.e + q);
    w.parts.tally.from_parts += got;
    if pt.walls(chem)[0] < w.cfg.phys.min_kg {
        if b.hold == pt.id(p) {
            let_go(w, b, false);
        }
        dissolve(w, p);
    } else {
        pt.resize(chem);
        restructure(w, p);
    }
}

/// Put to a part what a body holds above twice its safe minimums (as `give` does): liquids into
/// its hollow as far as there is room, solids onto its walls; at most half of each store.
fn put(w: &mut World, b: &mut Body, p: usize, mb: f64, days: f64) {
    let (chem, lay, t) = (&w.chem, Layout::of(&w.chem), b.temp(&w.chem));
    let spare: Vec<f64> = (0..chem.ns()).map(|k| if chem.phase(k, t) == Phase::Gas { 0.0 } else { (b.store[k] - 2.0 * b.gene(lay.min + k).max(0.0) * mb / chem.subs[k].molar_mass).max(0.0) }).collect();
    let all = kg(w, &spare);
    if all <= 0.0 {
        return;
    }
    let wet: Vec<bool> = (0..chem.ns()).map(|k| chem.phase(k, t) == Phase::Liquid).collect();
    let mut m: Vec<f64> = spare.iter().zip(&b.store).map(|(a, s)| (a * (w.cfg.phys.take * days * mb / all).min(1.0)).min(0.5 * s)).collect();
    let lv: f64 = (0..chem.ns()).filter(|&k| wet[k]).map(|k| m[k] * chem.molar_volume(k, Phase::Liquid)).sum();
    let fit = (w.parts.parts[p].room(chem) / lv.max(1e-12)).min(1.0);
    (0..chem.ns()).filter(|&k| wet[k]).for_each(|k| m[k] *= fit);
    let got = kg(w, &m);
    if got <= 0.0 || !body::pay(w, b, w.cfg.phys.c_handle * got, days) {
        return;
    }
    let chem = &w.chem;
    let m: Vec<f64> = m.iter().zip(&b.store).map(|(a, s)| a.min(*s)).collect();
    let q = b.e * m.iter().zip(&chem.subs).map(|(a, s)| a * s.cp).sum::<f64>() / b.heat_cap(chem);
    let pt = &mut w.parts.parts[p];
    for (k, &a) in m.iter().enumerate() {
        *if wet[k] { &mut pt.fill[k] } else { &mut pt.wall[k] } += a;
        b.store[k] -= a;
    }
    (pt.e, b.e) = (pt.e + q, b.e - q);
    w.parts.tally.to_parts += got;
    pt.resize(chem);
    restructure(w, p);
}

/// Put solid stores above twice their safe minimums out as a new part (up to `phys.form` of the
/// body's mass, and at least `phys.min_kg`: lighter walls fall apart), shaped by the brain:
/// elongation and hollowness. The body holds it if its hands are free and its force bears it
/// (`grab`). The number of parts in a cell is not limited: each one costs matter and energy.
pub fn form(w: &mut World, slot: usize, b: &mut Body, shape: [f32; 2]) {
    let i = b.cell as usize;
    let (chem, lay, mb, t) = (&w.chem, Layout::of(&w.chem), b.mass(&w.chem), b.temp(&w.chem));
    let spare: Vec<f64> = (0..chem.ns()).map(|k| if chem.phase(k, t) == Phase::Solid { (b.store[k] - 2.0 * b.gene(lay.min + k).max(0.0) * mb / chem.subs[k].molar_mass).max(0.0) } else { 0.0 }).collect();
    let all = kg(w, &spare);
    let f = (w.cfg.phys.form * mb / all.max(1e-9)).min(1.0);
    let wall: Vec<f64> = spare.iter().zip(&b.store).map(|(a, s)| (a * f).min(0.5 * s)).collect();
    let got = kg(w, &wall);
    if got < w.cfg.phys.min_kg || !body::pay(w, b, w.cfg.phys.c_form * got, b.interval() as f64) {
        return;
    }
    // Paying may have used some of what was to be put out.
    let wall: Vec<f64> = wall.iter().zip(&b.store).map(|(a, s)| a.min(*s)).collect();
    let e = b.e * wall.iter().zip(&w.chem.subs).map(|(a, s)| a * s.cp).sum::<f64>() / b.heat_cap(&w.chem);
    for (s, a) in b.store.iter_mut().zip(&wall) {
        *s -= a;
    }
    b.e -= e;
    let p = make(w, i, wall, e, shape[0], shape[1]);
    w.parts.tally.formed += 1.0;
    if held(w, b).is_none() {
        let mb = b.mass(&w.chem);
        grab(w, slot, b, p, mb);
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
    let s = w.parts.parts[p].walls(&w.chem)[3].min(sv / v.max(1e-12)) * w.cfg.phys.tensile * w.cfg.phys.joint * ac;
    if v > 0.0 && body::pay(w, b, w.cfg.phys.c_join * ac, b.interval() as f64) {
        link(w, p, GROUND, s);
        let_go(w, b, false);
    }
}

/// Break bonded ground with force: each substance whose bond strength is below the body's
/// strength (times what a held part adds) comes loose, the more the weaker it is.
pub fn dig(w: &mut World, b: &mut Body) {
    let (i, n, mb) = (b.cell as usize, w.grid.n(), b.mass(&w.chem));
    let (lev, chem) = (lever(w, b, mb), &w.chem);
    let f = b.gene(STRENGTH) * lev;
    let vol: f64 = (0..chem.ns()).map(|k| w.bed[k * n + i] * chem.molar_volume(k, Phase::Solid)).sum();
    if vol <= 0.0 {
        return;
    }
    let most = w.cfg.phys.dig * b.interval() as f64 * mb;
    let m: Vec<f64> = (0..chem.ns())
        .map(|k| {
            let (a, s) = (w.bed[k * n + i], &chem.subs[k]);
            if f <= s.strength || a <= 0.0 { 0.0 } else { (most * a * chem.molar_volume(k, Phase::Solid) / vol * (1.0 - s.strength / f) / s.molar_mass).min(0.5 * a) }
        })
        .collect();
    let got = kg(w, &m);
    if !body::pay(w, b, w.cfg.phys.c_handle * got, b.interval() as f64) {
        return;
    }
    for (k, a) in m.iter().enumerate() {
        w.bed[k * n + i] -= a;
        w.ground[k * n + i] += a;
    }
    let t = &mut w.parts.tally.dig[(lev > 1.0) as usize];
    (t[0], t[1]) = (t[0] + 1.0, t[1] + got / mb);
}

/// Before a body moves: the structure it holds and its mass, if it can carry it; else it lets
/// go (an anchored structure, or one heavier than the body bears).
pub fn carry(w: &mut World, b: &mut Body, mb: f64) -> Option<(usize, f64)> {
    let r = w.parts.parts[held(w, b)?].root as usize;
    let s = w.parts.parts[r].s;
    if s.anchored || s.mass * w.cfg.gravity > bears(w, b, mb) {
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
        for x in w.parts.members(r) {
            w.parts.relocate(x, j);
        }
    }
    let push = (w.cfg.phys.bump * mb * w.cfg.gravity) as f32;
    for x in 0..w.parts.at[j].len() {
        let r = w.parts.parts[w.parts.at[j][x] as usize].root as usize;
        w.parts.parts[r].push = w.parts.parts[r].push.max(push);
    }
}

/// Monthly: heat, Change, load and failure, floating and drifting.
pub fn step(w: &mut World) {
    ready(w);
    if w.parts.parts.iter().all(|p| !p.alive) {
        return;
    }
    let (dpm, ns, na) = (w.cfg.days_per_month as u64, w.chem.ns(), w.grid.na());
    let react = (w.day / dpm) % REACT_MONTHS == 0;
    let (mut rc, share) = (Reactor::new(&w.chem), 1.0 / (w.grid.air_scale * w.grid.air_scale) as f64);
    let (mut pool, mut pool0) = (vec![0.0; ns], vec![0.0; ns]);
    let dt = (REACT_MONTHS * dpm) as f64 * 86_400.0;
    for s in 0..w.parts.parts.len() {
        if !w.parts.parts[s].alive {
            continue;
        }
        let (i, n) = (w.parts.parts[s].cell as usize, w.grid.n());
        let (j, tg) = (w.grid.air_of(i), w.life.env[i][0]);
        let chem = &w.chem;
        let p = &mut w.parts.parts[s];
        if react {
            for k in 0..ns {
                pool0[k] = w.air[k * na + j] * share;
            }
            pool.copy_from_slice(&pool0);
            let fixed: f64 = p.fill.iter().zip(&chem.subs).map(|(a, x)| a * x.cp).sum();
            rc.react(chem, &mut p.wall, Some(&mut pool), &mut p.e, fixed, None, 0.0, dt);
            // Gas sent to the air takes its fusion and vaporisation enthalpy from the ground (gas
            // taken back returns it); gas the ground cannot pay for goes to the ground instead.
            let held = |k: usize| chem.subs[k].lf + chem.subs[k].lv;
            let (mut pay, mut back) = (0.0, 0.0);
            for k in 0..ns {
                let d = pool[k] - pool0[k];
                (pay, back) = (pay + d.max(0.0) * held(k), back - d.min(0.0) * held(k));
            }
            w.e_ground[i] += back;
            let paid = if pay > 0.0 { crate::share(w.take_heat(i, pay), pay) } else { 1.0 };
            for k in 0..ns {
                let d = pool[k] - pool0[k];
                let sent = if d > 0.0 { d * paid } else { d };
                w.air[k * na + j] += sent;
                w.ground[k * n + i] += d - sent;
            }
        }
        // Walls that are no longer solid run off to the ground; the part takes the ground's
        // temperature (if the ground cannot pay the heat, it ends colder).
        let chem = &w.chem;
        let p = &mut w.parts.parts[s];
        let t = p.e / p.heat_cap(chem).max(1e-9);
        for k in 0..ns {
            if p.wall[k] > 0.0 && chem.phase(k, t) != Phase::Solid {
                let q = p.wall[k] * chem.subs[k].cp * t;
                (w.ground[k * n + i], w.e_ground[i], p.e) = (w.ground[k * n + i] + p.wall[k], w.e_ground[i] + q, p.e - q);
                p.wall[k] = 0.0;
            }
        }
        let (e1, e0) = (p.heat_cap(chem) * tg, p.e);
        let got = w.take_heat(i, e1 - e0);
        let p = &mut w.parts.parts[s];
        p.e = if got == e1 - e0 { e1 } else { e0 + got };
        let chem = &w.chem;
        let gone = p.walls(chem)[0] < w.cfg.phys.min_kg;
        p.resize(chem);
        if gone {
            dissolve(w, s);
        }
    }
    for r in 0..w.parts.parts.len() {
        if w.parts.parts[r].alive && w.parts.parts[r].root as usize == r {
            if react {
                restructure(w, r);
            }
            load(w, r);
        }
    }
}

/// Load one structure: weights less buoyancy, pushes of moving liquid and bodies; break what is
/// overloaded; float and drift.
fn load(w: &mut World, r: usize) {
    let (list, i, g) = (w.parts.members(r), w.parts.parts[r].cell as usize, w.cfg.gravity);
    let (depth, rho, molv) = liquid(w, i);
    let flux = w.diag.liquid_flux.get(i).copied().unwrap_or(0.0) as f64;
    let v = if depth > 0.01 { flux / (molv * w.grid.cell_m * depth) } else { 0.0 };
    let bump = std::mem::take(&mut w.parts.parts[r].push) as f64 / list.len() as f64;
    let (mut wt, mut push, mut lift) = (vec![0.0; list.len()], vec![0.0; list.len()], 0.0);
    for (x, &p) in list.iter().enumerate() {
        let pt = &w.parts.parts[p];
        let face = (pt.size[0] * pt.size[1]) as f64;
        let buoy = rho * g * pt.outer().min(face * depth);
        lift += buoy;
        wt[x] = (pt.mass(&w.chem) * g - buoy).max(0.0);
        push[x] = 0.5 * rho * v * v * face + bump;
    }
    let s = w.parts.parts[r].s;
    let afloat = !s.anchored && depth > 0.0 && lift >= s.mass * g;
    w.parts.parts[r].s.afloat = afloat;
    let held = list.iter().copied().find(|&p| holder(w, p).is_some());
    // From the supports (anchored parts, else the held one) out along the bonds; each bond
    // carries what hangs beyond it; a ground bond carries the pushes on what it holds.
    let mut order: Vec<usize> = (0..list.len()).filter(|&x| w.parts.parts[list[x]].links.iter().any(|l| l.0 == GROUND)).collect();
    if order.is_empty() {
        order.extend(held.and_then(|h| list.iter().position(|&p| p == h)));
    }
    let mut up = vec![usize::MAX; list.len()];
    let mut q = 0;
    while q < order.len() {
        for &(o, _) in &w.parts.parts[list[order[q]]].links {
            if let Some(c) = list.iter().position(|&p| p as u32 == o).filter(|&c| !order.contains(&c)) {
                up[c] = order[q];
                order.push(c);
            }
        }
        q += 1;
    }
    for &c in order.iter().rev() {
        if up[c] != usize::MAX {
            (wt[up[c]], push[up[c]]) = (wt[up[c]] + wt[c], push[up[c]] + push[c]);
        }
    }
    let mut breaks = Vec::new();
    for &c in &order {
        for &(o, st) in &w.parts.parts[list[c]].links {
            let hang = if o == GROUND { push[c] } else if up[c] != usize::MAX && list[up[c]] as u32 == o { wt[c] + push[c] } else { 0.0 };
            if hang > st as f64 {
                breaks.push((list[c], o));
            }
        }
    }
    for (a, o) in breaks {
        if w.parts.parts[a].links.iter().any(|l| l.0 == o) {
            unlink(w, a, o, true);
        }
    }
    // The holder of a structure that does not rest on the ground bears what hangs on the part it
    // holds (as of the start of the month) by its own force; past that, it lets go.
    let x = held.filter(|_| !s.anchored).and_then(|h| list.iter().position(|&p| p == h));
    if let Some((x, o)) = x.and_then(|x| holder(w, list[x]).map(|o| (x, o))) {
        let ob = &w.life.bodies[o];
        if wt[x] + push[x] > bears(w, ob, ob.mass(&w.chem)) {
            let mut ob = std::mem::take(&mut w.life.bodies[o]);
            let_go(w, &mut ob, true);
            w.life.bodies[o] = ob;
        }
    }
    // Afloat and free: drift to the neighbour whose liquid stands lowest, as far as the
    // liquid moves in a month.
    if !afloat || held.is_some() || v <= 0.0 || w.parts.parts[r].root as usize != r {
        return;
    }
    let mut rng = Rng::new(w.seed, w.day, w.parts.parts[r].id(r), Purpose::Bond);
    let head = |j: usize| w.life.env[j][2] + w.life.env[j][1];
    let to = DIRS.iter().filter_map(|&d| w.grid.step(i, d)).filter(|&j| head(j) < head(i)).min_by(|&a, &b| head(a).total_cmp(&head(b)));
    if let Some(j) = to.filter(|_| rng.f64() < v * (w.cfg.days_per_month as f64 * 86_400.0) / w.grid.cell_m) {
        for p in w.parts.members(r) {
            w.parts.relocate(p, j);
        }
    }
}
