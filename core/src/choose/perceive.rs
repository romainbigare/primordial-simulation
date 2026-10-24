//! What a body perceives: the bodies and parts it sees (`perceive`), its brain's inputs (`inputs`), the cells seen in
//! each direction (`sector`) and what a cell offers as a place to move to (`cells`); each scaled to about one.

use super::{ActTmp, Choice, MAX_SEEN, MOVES, NT, OPTS, PARTS};
use crate::body::{Body, Layout, Lived};
use crate::brain::{ln, REST, SIG};
use crate::lanes::thickness;
use crate::rng::Rng;
use crate::{bond, World};

/// A brain perceives a share `x` of the body as `SHARE_SENSE` ln(x + floor): a store with the floor `STORE_FLOOR` (no
/// store: as that floor), less its safe minimum with the floor `MIN_FLOOR`.
const SHARE_SENSE: f32 = 0.25;
const STORE_FLOOR: f32 = 1e-4;
const MIN_FLOOR: f32 = 1e-2;
/// The body's temperature over its cell's ground, K.
const WARM_SCALE: f32 = 10.0;
// ln of a mass: of a member, kg (less 1), of a perceived body's member over its own, of a cell's mean member.
pub(super) use crate::brain::LN_MASS;
/// ln(1 + the light it absorbed, J per kg per day).
const LN_LIGHT: f32 = 15.0;
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

/// Bodies perceived, its own cell first, then a neighbour's from a random side, up to `nseen`: walls hide by how closed
/// they are, cover with chance 1 - exp(-a), a the others' capture area per m². Then the parts in its cell.
pub(super) fn perceive(w: &World, b: &Body, nseen: usize, rng: &mut Rng) -> Choice {
    let i = b.cell as usize;
    let (mut seen, mut cnt, side, cells) = ([(0usize, 0u64, false); MAX_SEEN], 0, rng.below(MOVES), w.mesh.cells());
    let room = bond::room(w, b);
    for q in 0..9 {
        let Some(j) = (if q == 0 { Some(i) } else { cells.way(i, (side + q) % MOVES).map(|e| cells.adj_cell()[e] as usize) }) else { continue };
        // (Itself, taken out of its slot, is no living body there.)
        for o in w.life.cell(j).filter(|&o| w.life[o].alive) {
            if cnt == nseen {
                break;
            }
            let ob = &w.life[o];
            let their = bond::room(w, ob);
            let walls = if room.map(|r| r.0) == their.map(|r| r.0) { 0.0 } else { room.map_or(0.0, |r| r.1.closed).max(their.map_or(0.0, |r| r.1.closed)) };
            let cover = (w.canopy[j] as f64 - ob.vis[1] as f64 * ob.n() / w.mesh.cells().area()[j]).max(0.0);
            if (walls == 0.0 || rng.f64() as f32 >= walls) && (cover <= 0.0 || rng.f64() < (-cover).exp()) {
                (seen[cnt], cnt) = ((o, ob.id, q == 0), cnt + 1);
            }
        }
    }
    let (held, list) = (bond::held(w, b), &w.parts.at[i][..]);
    let (mut parts, mut np, off) = ([0usize; PARTS], 0, if list.is_empty() { 0 } else { rng.below(list.len()) });
    for q in 0..list.len() {
        let p = list[(off + q) % list.len()] as usize;
        if np == PARTS {
            break;
        }
        if Some(p) != held {
            (parts[np], np) = (p, np + 1);
        }
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

/// The brain's inputs (laws.md 5.2).
pub(super) fn inputs(w: &World, b: &Body, lived: &Lived, temp: f64, c: &Choice, x: &mut Vec<f32>) {
    let (i, ns, chem, mass) = (b.cell as usize, w.chem.ns(), &w.chem, lived.mass);
    let (env, pm) = (w.life.env[i], mass / b.n());
    let heard = c.seen[..c.nseen].iter().fold([0f32; SIG], |h, &(o, ..)| [0, 1].map(|k| h[k] + w.life[o].sig[k] / c.nseen as f32));
    x.clear();
    let (none, min) = (SHARE_SENSE * ln(STORE_FLOOR), Layout::of(chem).min);
    x.extend((0..ns).map(|k| if b.store[k] > 0.0 { SHARE_SENSE * ln((b.store[k] * chem.subs[k].molar_mass / mass) as f32 + STORE_FLOOR) } else { none } - SHARE_SENSE * ln(b.genes[min + k].max(0.0) + MIN_FLOOR)));
    x.extend_from_slice(&w.life.seen[i * ns..(i + 1) * ns]);
    let crowd = ln(1.0 + (w.life.sum[i][1] / w.mesh.cells().area()[i]) as f32);
    let (dt, lit, sun) = ((temp - env[0]) as f32, lived.lit as f32, w.season() as f32 * std::f32::consts::TAU);
    x.extend([dt / WARM_SCALE, ln(pm as f32) / LN_MASS - 1.0, ln(1.0 + lit) / LN_LIGHT, ln(1.0 + env[1] as f32), (env[0] as f32 - T_MID) / T_SPAN, crowd, sun.sin(), sun.cos()]);
    x.extend(heard);
    x.extend(bond::sense(w, b, pm));
    x.extend_from_slice(&b.mem);
}

/// After each survey: what a brain perceives of each cell as a place to move to (the height and temperature differences
/// are added per move, in places 0 and 6, and what the body can take there in place 8; the depth of standing liquid is
/// in place `WET`, its parts in the last place).
pub fn cells(w: &mut World) {
    let (life, parts) = (&w.life, &w.parts.at);
    let cellf = life.sum.iter().zip(&life.env).zip(w.mesh.cells().area()).zip(parts).map(|(((c, e), &area), p)| {
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
