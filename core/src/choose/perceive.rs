//! What a body perceives (laws.md 5.2): the features of the cells, read by every brain (`features`); the bodies and
//! parts of its own cell (`perceive`); its brain's inputs (`inputs`); each scaled to about one.

use super::{Choice, MAX_SEEN, MOVES, NT, OPTS, PARTS};
use crate::body::{Body, Layout};
use crate::brain::{ln, REST};
use crate::rng::Rng;
use crate::{bond, life::{Cell, Chunk}, World};

/// A brain perceives a share `x` of the body as `SHARE_SENSE` ln(x + floor): a store with the floor `STORE_FLOOR` (no
/// store: as that floor), less its safe minimum with the floor `MIN_FLOOR`.
const SHARE_SENSE: f32 = 0.25;
const STORE_FLOOR: f32 = 1e-4;
const MIN_FLOOR: f32 = 1e-2;
/// The body's temperature over its cell's ground, K.
const WARM_SCALE: f32 = 10.0;
/// The heat that left a body through its coat over the heat its upkeep released (`Lived::lost`) is perceived as its ln,
/// within plus or minus this (a factor of 20: a body that sweats away nearly all its heat, or a nearly dormant one that
/// burns far more than its upkeep to keep warm).
const LOST_SPAN: f32 = 3.0;
// ln of a mass: of a member, kg (less 1), of a perceived body's member over its own, of a cell's mean member.
pub(super) use crate::brain::LN_MASS;
/// A perceived body's well-being.
pub(super) const WB_SCALE: f32 = 4.0;

/// What a body perceives of another of its cell that it may act on: its id, its place in the cell's list
/// (`life::Handle::slot`, which holds through the pass), its visible traits (`Body::vis`) and its well-being at its last
/// decision.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Face {
    pub id: u64,
    pub slot: u32,
    pub vis: [f32; crate::body::VIS],
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
    let show = (-(crate::contract::hides(w, i) as f64)).exp();
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

/// A brain's inputs of its own state besides its stores, before its cell's features (`inputs`): its warmth, the heat it
/// loses, its size and the season's sine and cosine.
pub const OWN: usize = 5;

/// The brain's inputs (laws.md 5.2), of a body of cell `cell` at `temp` K, with `mem` numbers of memory, as it is when
/// it perceives: its stores against their minimums, its warmth, the heat it loses (the ln of the heat that left it
/// through its coat over the heat its upkeep released in its last sub-step, `Lived::lost`: above 0 where it burns more
/// to keep warm, the less so inside walls that hold its heat, below 0 where it sweats), its size and the season, its
/// cell's features, what Bond senses, its memory.
pub(super) fn inputs(w: &World, cell: &Cell, b: &Body, (temp, mem): (f64, usize), x: &mut Vec<f32>) {
    let (i, ns, chem, mass) = (b.cell as usize, w.chem.ns(), &w.chem, b.mass(&w.chem));
    let pm = mass / b.n();
    x.clear();
    let (none, min, bc) = (SHARE_SENSE * ln(STORE_FLOOR), Layout::of(chem).min, chem.body());
    let (held, least) = (|k: usize| b.amount(bc, k), |k: usize| bc.place(k).map_or(0.0, |s| b.genes[min + s].max(0.0)));
    x.extend((0..ns).map(|k| if held(k) > 0.0 { SHARE_SENSE * ln((held(k) * chem.subs[k].molar_mass / mass) as f32 + STORE_FLOOR) } else { none } - SHARE_SENSE * ln(least(k) + MIN_FLOOR)));
    let (dt, sun) = ((temp - crate::contract::place(w, i).t) as f32, w.season() as f32 * std::f32::consts::TAU);
    let lost = (ln(b.lived.lost.max(0.0) as f32) - ln(b.lived.released.max(0.0) as f32)).clamp(-LOST_SPAN, LOST_SPAN);
    x.extend([dt / WARM_SCALE, lost, ln(pm as f32) / LN_MASS - 1.0, sun.sin(), sun.cos()]);
    x.extend_from_slice(w.life.feat(i, 1));
    x.extend(bond::sense(cell, w, b, pm));
    x.extend_from_slice(&b.mem[..mem]);
}
