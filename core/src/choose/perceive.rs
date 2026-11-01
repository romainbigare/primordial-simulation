//! What a body perceives: the bodies and parts it sees (`perceive`), its brain's inputs (`inputs`), the cells seen in
//! each direction (`sector`) and what a cell offers as a place to move to (`cells`); each scaled to about one.

use super::{ActTmp, Choice, MAX_SEEN, MOVES, NT, OPTS, PARTS};
use crate::body::{Body, Layout, Lived};
use crate::brain::{ln, REST, SIG};
use crate::lanes::thickness;
use crate::rng::Rng;
use crate::survey::{Face, UNCOVERED};
use crate::{bond, life::{Cell, Chunk, Handle}, World};

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
/// From a cell to the cells it sees: the height difference, m, and the temperature difference, K.
pub(super) const DZ_SCALE: f64 = 200.0;
pub(super) const DT_NEAR: f64 = 5.0;
/// ln of the light-capture area per mass (of a perceived body, of a cell's members).
pub(super) const LN_LEAF: f32 = 5.0;
/// ln(1 + the members of a cell).
const LN_MEMBERS: f32 = 10.0;
/// A perceived body's well-being.
pub(super) const WB_SCALE: f32 = 4.0;

/// The cells within `sight` links of cell `i` (not `i`), each with its weight (nearer cells more, 1 / rings²), in the
/// order of the cells: a walk over the mesh's links, ring by ring, that marks what it reached with `t.stamp`.
pub(super) fn walk(w: &World, i: usize, sight: usize, t: &mut ActTmp) {
    let cells = w.mesh.cells();
    t.mark.resize(cells.n(), 0);
    t.stamp = t.stamp.wrapping_add(1);
    if t.stamp == 0 {
        t.mark.fill(0);
        t.stamp = 1;
    }
    let (seen, mark, stamp) = (&mut t.seen, &mut t.mark, t.stamp);
    seen.clear();
    seen.push((i as u32, 0.0));
    mark[i] = stamp;
    let mut from = 0;
    for ring in 1..=sight {
        let to = seen.len();
        for q in from..to {
            for &j in cells.around(seen[q].0 as usize).1 {
                if mark[j as usize] != stamp {
                    mark[j as usize] = stamp;
                    seen.push((j, 1.0 / (ring * ring) as f32));
                }
            }
        }
        from = to;
    }
    seen.swap_remove(0);
    seen.sort_unstable_by_key(|s| s.0);
}

/// Bodies perceived, its own cell first, then a neighbour's from a random side, up to `nseen`, as they were at the
/// batch's start (their faces, `survey::Faces`): walls hide by how closed they are, cover with chance 1 - exp(-a), a the
/// others' capture area per m² (`Face::show`). Then the parts in its cell.
pub(super) fn perceive(w: &World, d: &Chunk, b: &Body, nseen: usize, rng: &mut Rng) -> Choice {
    let i = b.cell as usize;
    let (mut seen, mut cnt, side, cells) = ([(Face::default(), false); MAX_SEEN], 0, rng.below(MOVES), w.mesh.cells());
    let room = bond::room(d.at(i), b);
    // Its own room as the others' are seen, from its face (outside a batch, where there are none, as it is now).
    let face = w.life.faces_of(i).iter().find(|o| o.id == b.id);
    let mine = face.map_or_else(|| room.map(|(r, e)| (r, e.closed)), |o| o.room);
    for q in 0..9 {
        let Some(j) = (if q == 0 { Some(i) } else { cells.way(i, (side + q) % MOVES).map(|e| cells.adj_cell()[e] as usize) }) else { continue };
        // (Of its own cell, not those leaving it: they are there only until the serial pass, `Body::leaving`.)
        let here = |o: &&Face| q > 0 || !d[Handle { cell: j as u32, slot: o.slot }].leaving;
        for o in w.life.faces_of(j).iter().filter(|o| o.id != b.id).filter(here) {
            if cnt == nseen {
                break;
            }
            let walls = if mine.map(|r| r.0) == o.room.map(|r| r.0) { 0.0 } else { mine.map_or(0.0, |r| r.1).max(o.room.map_or(0.0, |r| r.1)) };
            if (walls == 0.0 || rng.f64() as f32 >= walls) && (o.show == UNCOVERED || rng.f64() < o.show) {
                (seen[cnt], cnt) = ((*o, q == 0), cnt + 1);
            }
        }
    }
    let (held, n) = (bond::held(d.at(i), b), d.at(i).parts.len());
    let (mut parts, mut np, off) = ([0usize; PARTS], 0, if n == 0 { 0 } else { rng.below(n) });
    for p in (0..n).map(|q| (off + q) % n).filter(|&p| Some(p) != held).take(PARTS) {
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

/// The brain's inputs (laws.md 5.2), of a body of cell `cell` that has `lived` its update at `temp` K, with `mem`
/// numbers of memory.
pub(super) fn inputs(w: &World, cell: &Cell, b: &Body, (lived, temp, mem): (&Lived, f64, usize), c: &Choice, x: &mut Vec<f32>) {
    let (i, ns, chem, mass) = (b.cell as usize, w.chem.ns(), &w.chem, lived.mass);
    let (env, pm) = (w.life.env[i], mass / b.n());
    let heard = c.seen[..c.nseen].iter().fold([0f32; SIG], |h, (o, _)| [0, 1].map(|k| h[k] + o.sig[k] / c.nseen as f32));
    x.clear();
    let (none, min, bc) = (SHARE_SENSE * ln(STORE_FLOOR), Layout::of(chem).min, chem.body());
    let (held, least) = (|k: usize| b.amount(bc, k), |k: usize| bc.place(k).map_or(0.0, |s| b.genes[min + s].max(0.0)));
    x.extend((0..ns).map(|k| if held(k) > 0.0 { SHARE_SENSE * ln((held(k) * chem.subs[k].molar_mass / mass) as f32 + STORE_FLOOR) } else { none } - SHARE_SENSE * ln(least(k) + MIN_FLOOR)));
    x.extend_from_slice(&w.life.seen[i * ns..(i + 1) * ns]);
    let crowd = ln(1.0 + (w.life.sum(i)[1] / w.mesh.cells().area()[i]) as f32);
    let (dt, canopy, sun) = ((temp - env[0]) as f32, w.canopy[i], w.season() as f32 * std::f32::consts::TAU);
    x.extend([dt / WARM_SCALE, ln(pm as f32) / LN_MASS - 1.0, ln(1.0 + canopy) / LN_CANOPY, ln(1.0 + env[1] as f32), (env[0] as f32 - T_MID) / T_SPAN, crowd, sun.sin(), sun.cos()]);
    x.extend(heard);
    x.extend(bond::sense(cell, w, b, pm));
    x.extend_from_slice(&b.mem[..mem]);
}

/// After each survey: what a brain perceives of each cell as a place to move to (the height and temperature differences
/// are added per move, in places 0 and 6, and what the body can take there in place 8; the depth of standing liquid is
/// in place `WET`, its parts in the last place).
pub fn cells(w: &mut World) {
    let life = &w.life;
    let cellf = (0..w.mesh.cells().n()).map(|i| {
        let (c, e, area, p) = (life.sum(i), &life.env[i], w.mesh.cells().area()[i], &life.cells[i].parts);
        let resist = 1.0 + thickness(w, c[1], area) as f32;
        let c = c.map(|v| v as f32);
        let n = c[0].max(1.0);
        let leaf = if c[0] > 0.0 { ln(c[2] / c[1].max(1.0)) / LN_LEAF } else { 0.0 };
        let (members, mass) = (ln(1.0 + c[0]) / LN_MEMBERS, ln((c[1] / n).max(1.0)) / LN_MASS - 1.0);
        let parts = ln(1.0 + p.len() as f32);
        [0.0, ln(1.0 + e[1] as f32), members, mass, leaf, c[3] / n, 0.0, ln(resist), 0.0, parts]
    });
    w.life.cellf = cellf.collect();
}
