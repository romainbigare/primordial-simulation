//! What a body perceives (laws.md 5.2): the features of each cell, worked out once a month for every cell and read by
//! every brain (`cells`); the bodies and parts of its own cell (`perceive`); its brain's inputs (`inputs`); each scaled
//! to about one.

use super::{Choice, MAX_SEEN, MOVES, NT, OPTS, PARTS};
use crate::body::{Body, Layout};
use crate::brain::{ln, REST};
use crate::lanes::thickness;
use crate::rng::Rng;
use crate::store::Class;
use crate::{bond, life::{Cell, Chunk}, World};

/// A brain perceives a share `x` of the body as `SHARE_SENSE` ln(x + floor): a store with the floor `STORE_FLOOR` (no
/// store: as that floor), less its safe minimum with the floor `MIN_FLOOR`.
const SHARE_SENSE: f32 = 0.25;
const STORE_FLOOR: f32 = 1e-4;
const MIN_FLOOR: f32 = 1e-2;
/// The body's temperature over its cell's ground, K.
const WARM_SCALE: f32 = 10.0;
// ln of a mass: of a member, kg (less 1), of a perceived body's member over its own, of a cell's mean member.
pub(super) use crate::brain::LN_MASS;
/// ln(1 + the cover's capture area per m² of its cell, `World::canopy`) under a dense cover (10 m² per m²).
const LN_CANOPY: f32 = 2.4;
/// The ground temperature, from `T_MID` over `T_SPAN`, K.
const T_MID: f32 = 285.0;
const T_SPAN: f32 = 15.0;
/// The surface height, m.
const DZ_SCALE: f32 = 200.0;
/// ln of the light-capture area per mass of a perceived body.
pub(super) const LN_LEAF: f32 = 5.0;
/// ln(1 + the members of a cell per `COUNT_AREA`).
const LN_MEMBERS: f32 = 10.0;
/// The area a cell's counts (members, parts, members that moved in) are taken over, m²: a square of the island (2 km).
const COUNT_AREA: f64 = 4e6;
/// A perceived body's well-being.
pub(super) const WB_SCALE: f32 = 4.0;
/// The members of the three classes of animals of a cell's features weigh below `SMALL_KG`, up to `BIG_KG`, and more
/// (`docs/numbers.md`).
const SMALL_KG: f32 = 1.0;
const BIG_KG: f32 = 100.0;

/// The features of a cell (`cells`): its ground temperature, the depth of its standing liquid (ln(1 + depth / m)), its
/// surface height, the shade of its cover, the mass of each slot of its cover and of the cover's fuel and body matter
/// (what most diets eat), the fuel and body matter loose on its ground, the mass of its animals in three classes of
/// member mass, their members and mean strength, how hard it is to push through, its parts, and the members that moved
/// in last month; masses per m² as ln(1 + kg/m²), counts per `COUNT_AREA`.
pub const T: usize = 0;
pub const WET: usize = 1;
pub const HEIGHT: usize = 2;
const SHADE: usize = 3;
const COVER: usize = 4;
pub const EDIBLE: usize = COVER + crate::cover::SLOTS;
pub const DEAD: usize = EDIBLE + 1;
const ANIMALS: usize = DEAD + 1;
const MEMBERS: usize = ANIMALS + 3;
const STRENGTH: usize = MEMBERS + 1;
const RESIST: usize = STRENGTH + 1;
const PARTS_HERE: usize = RESIST + 1;
const CROSSED: usize = PARTS_HERE + 1;
pub const NCELL: usize = CROSSED + 1;
/// The levels of the cells' features (`Life::feat`): the cells' own, then smoothed once to `SIGHTS - 1` times over
/// their neighbours, what a body of sight 1 to `SIGHTS` sees of the cell it would step into.
pub const SIGHTS: usize = 4;

/// What a body perceives of another of its cell that it may act on: its id, its place in the cell's list
/// (`life::Handle::slot`, which holds through the pass), its visible traits (`Body::vis`) and its well-being at its last
/// decision.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Face {
    pub id: u64,
    pub slot: u32,
    pub vis: [f32; 7],
    pub wb: f32,
}

/// Bodies perceived in its own cell, from a random place in the cell's list on, up to `nseen`, as they are now: walls
/// hide by how closed they are, cover with chance 1 - exp(-a), a the cell's capture area per m² (`Face::show`). Then
/// the parts in its cell.
pub(super) fn perceive(w: &World, d: &Chunk, b: &Body, nseen: usize, rng: &mut Rng) -> Choice {
    let (i, mut seen, mut cnt) = (b.cell as usize, [Face::default(); MAX_SEEN], 0);
    let cell = d.at(i);
    let room = bond::room(cell, b);
    let mine = room.map(|(r, e)| (r, e.closed));
    let show = (-(w.canopy[i] as f64)).exp();
    let n = cell.bodies.len();
    let off = if n == 0 { 0 } else { rng.below(n) };
    for slot in (0..n).map(|q| (off + q) % n) {
        let o = &cell.bodies[slot];
        if cnt == nseen {
            break;
        }
        if !o.alive || o.id == b.id || o.leaving {
            continue;
        }
        let theirs = bond::room(cell, o).map(|(r, e)| (r, e.closed));
        let walls = if mine.map(|r| r.0) == theirs.map(|r| r.0) { 0.0 } else { mine.map_or(0.0, |r| r.1).max(theirs.map_or(0.0, |r| r.1)) };
        if (walls == 0.0 || rng.f64() as f32 >= walls) && (show >= 1.0 || rng.f64() < show) {
            (seen[cnt], cnt) = (Face { id: o.id, slot: slot as u32, vis: o.vis, wb: o.wb }, cnt + 1);
        }
    }
    let (held, np_all) = (bond::held(cell, b), cell.parts.len());
    let (mut parts, mut np, off) = ([0usize; PARTS], 0, if np_all == 0 { 0 } else { rng.below(np_all) });
    for p in (0..np_all).map(|q| (off + q) % np_all).filter(|&p| Some(p) != held).take(PARTS) {
        (parts[np], np) = (p, np + 1);
    }
    Choice {
        seen,
        nseen: cnt,
        parts,
        nparts: np,
        room,
        held,
        logit: [f32::NEG_INFINITY; OPTS],
        p: [0.0; OPTS],
        to: [usize::MAX; MOVES],
        fb: [0.0; MAX_SEEN * NT],
        fo: [0.0; PARTS * bond::NO],
        a: REST,
    }
}

/// A brain's inputs of its own state besides its stores, before its cell's features (`inputs`): its warmth, its size
/// and the season's sine and cosine.
pub const OWN: usize = 4;

/// The brain's inputs (laws.md 5.2), of a body of cell `cell` at `temp` K, with `mem` numbers of memory, as it is when
/// it perceives: its stores against their minimums, its warmth, its size and the season, its cell's features, what
/// Bond senses, its memory.
pub(super) fn inputs(w: &World, cell: &Cell, b: &Body, (temp, mem): (f64, usize), x: &mut Vec<f32>) {
    let (i, ns, chem, mass) = (b.cell as usize, w.chem.ns(), &w.chem, b.mass(&w.chem));
    let pm = mass / b.n();
    x.clear();
    let (none, min, bc) = (SHARE_SENSE * ln(STORE_FLOOR), Layout::of(chem).min, chem.body());
    let (held, least) = (|k: usize| b.amount(bc, k), |k: usize| bc.place(k).map_or(0.0, |s| b.genes[min + s].max(0.0)));
    x.extend((0..ns).map(|k| if held(k) > 0.0 { SHARE_SENSE * ln((held(k) * chem.subs[k].molar_mass / mass) as f32 + STORE_FLOOR) } else { none } - SHARE_SENSE * ln(least(k) + MIN_FLOOR)));
    let (dt, sun) = ((temp - w.life.env[i][0]) as f32, w.season() as f32 * std::f32::consts::TAU);
    x.extend([dt / WARM_SCALE, ln(pm as f32) / LN_MASS - 1.0, sun.sin(), sun.cos()]);
    x.extend_from_slice(w.life.feat(i, 1));
    x.extend(bond::sense(cell, w, b, pm));
    x.extend_from_slice(&b.mem[..mem]);
}

/// Once a month, after the survey: the features of every cell (`NCELL`, the same for every brain), then each level of
/// `SIGHTS` the last smoothed over each cell and its neighbours, by area (`Life::feat`).
pub fn cells(w: &mut World) {
    let (n, life, chem) = (w.mesh.cells().n(), &w.life, &w.chem);
    let parts = crate::cover::Parts::of(chem);
    let mm = parts.as_ref().map(|p| p.mm(chem));
    let ln_kg = |kg: f64, area: f64| ln(1.0 + (kg / area) as f32);
    let ground = w.store.amounts(Class::Ground);
    let mut feat: Vec<[f32; NCELL]> = (0..n).map(|i| {
        let (c, e, area, cell) = (life.sum(i), &life.env[i], w.mesh.cells().area()[i], &life.cells[i]);
        let mut f = [0f32; NCELL];
        (f[T], f[WET], f[HEIGHT]) = ((e[0] as f32 - T_MID) / T_SPAN, ln(1.0 + e[1] as f32), e[2] as f32 / DZ_SCALE);
        f[SHADE] = ln(1.0 + w.canopy[i]) / LN_CANOPY;
        if let (Some(p), Some(mm)) = (&parts, &mm) {
            for x in 0..crate::cover::SLOTS {
                f[COVER + x] = ln_kg(cell.cover.kg(mm, x), area);
            }
            let soft: f64 = (0..crate::cover::SLOTS).map(|x| cell.cover.mol[x][0] * mm[0] + cell.cover.mol[x][1] * mm[1]).sum();
            f[EDIBLE] = ln_kg(soft, area);
            f[DEAD] = ln_kg(p.sub[..2].iter().zip(mm).map(|(&k, m)| ground[k * n + i] * m).sum(), area);
        }
        for o in cell.bodies.iter().filter(|o| o.alive) {
            let class = if o.vis[0] < SMALL_KG { 0 } else if o.vis[0] < BIG_KG { 1 } else { 2 };
            f[ANIMALS + class] += o.vis[0] * o.n() as f32;
        }
        for class in 0..3 {
            f[ANIMALS + class] = ln(1.0 + f[ANIMALS + class] / area as f32);
        }
        let per = COUNT_AREA / area;
        f[MEMBERS] = ln(1.0 + (c[0] * per) as f32) / LN_MEMBERS;
        f[STRENGTH] = (c[3] / c[0].max(1.0)) as f32;
        f[RESIST] = ln(1.0 + thickness(w, c[1], area) as f32);
        (f[PARTS_HERE], f[CROSSED]) = (ln(1.0 + (cell.parts.len() as f64 * per) as f32), ln(1.0 + (cell.crossings as f64 * per) as f32));
        f
    }).collect();
    // Each level the last smoothed over each cell and its neighbours, each by its area (over the cell's own).
    let cells = w.mesh.cells();
    for level in 1..SIGHTS {
        let last = &feat[(level - 1) * n..level * n];
        let next: Vec<[f32; NCELL]> = (0..n).map(|i| {
            let (mut f, mut sum) = (last[i], 1.0);
            for &j in cells.around(i).1 {
                let by = (cells.area()[j as usize] / cells.area()[i]) as f32;
                f.iter_mut().zip(&last[j as usize]).for_each(|(a, b)| *a += by * b);
                sum += by;
            }
            f.map(|v| v / sum)
        }).collect();
        feat.extend(next);
    }
    w.life.feat = feat;
}
