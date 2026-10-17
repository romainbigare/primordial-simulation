//! What a body perceives: the bodies and parts it sees (`perceive`), its brain's inputs (`inputs`), the cells seen in
//! each direction (`sector`) and what a cell offers as a place to move to (`cells`); each scaled to about one.

use super::{Choice, MAX_SEEN, MOVES, NT, OPTS, PARTS};
use crate::body::{Body, Lived, SHARE_SENSE};
use crate::brain::{ln, REST, SIG};
use crate::space::square::DIRS;
use crate::lanes::thickness;
use crate::rng::Rng;
use crate::{bond, World};

/// A store's share of the body's mass is perceived as `SHARE_SENSE` ln(share + this) (no store: as this).
const STORE_FLOOR: f32 = 1e-4;
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

/// Per direction, the offsets of the cells seen that way and their weights.
pub type Sector = [Vec<(i32, i32, f32)>; 8];

/// The cells within `sight` of a cell, by the nearest of the 8 directions they lie in: offsets and weights (nearer
/// cells more, 1 / distance², distance counted in rings).
pub(super) fn sector(sight: usize) -> Sector {
    let (mut s, r): (Sector, i32) = (Default::default(), sight as i32);
    for dy in -r..=r {
        for dx in -r..=r {
            let k = dx.abs().max(dy.abs());
            if k > 0 {
                let d = ((dy as f64).atan2(dx as f64) / std::f64::consts::FRAC_PI_4).round() as i32;
                s[d.rem_euclid(8) as usize].push((dx, dy, 1.0 / (k * k) as f32));
            }
        }
    }
    s
}

/// Bodies perceived: in its own cell first, then next to it from a random side, up to `nseen`. Walls between two bodies
/// hide them from each other by how closed they are; cover hides a body as it shades the ground: it is seen through the
/// others' capture area per m² of its cell, a, with chance exp(-a). Parts perceived in its cell (not the one it holds),
/// from a random one on; a node perceives them as one of its members would.
pub(super) fn perceive(w: &World, slot: usize, b: &Body, nseen: usize, rng: &mut Rng) -> Choice {
    let i = b.cell as usize;
    let (mut seen, mut cnt, side) = ([(0usize, false); MAX_SEEN], 0, rng.below(8));
    let room = bond::room(w, b);
    for q in 0..9 {
        if cnt == nseen {
            break;
        }
        let Some(j) = (if q == 0 { Some(i) } else { w.grid.step(i, DIRS[(side + q) % 8]) }) else { continue };
        for &o in w.life.at[j].iter().filter(|&&o| o as usize != slot) {
            let ob = &w.life.bodies[o as usize];
            let their = bond::room(w, ob);
            let walls = if room.map(|r| r.0) == their.map(|r| r.0) { 0.0 } else { room.map_or(0.0, |r| r.1.closed).max(their.map_or(0.0, |r| r.1.closed)) };
            let cover = (w.canopy[j] as f64 - ob.vis[1] as f64 * ob.n() / w.grid.area()).max(0.0);
            if cnt < nseen && (walls == 0.0 || rng.f64() as f32 >= walls) && (cover <= 0.0 || rng.f64() < (-cover).exp()) {
                (seen[cnt], cnt) = ((o as usize, q == 0), cnt + 1);
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

/// The brain's inputs: own stores against their safe minimums, the cell's loose substances, warmth, size and light,
/// the cell's liquid, temperature and crowding, the season, signals heard, what Bond senses, memory.
pub(super) fn inputs(w: &World, b: &Body, lived: &Lived, temp: f64, c: &Choice, x: &mut Vec<f32>) {
    let (i, ns, chem, mass) = (b.cell as usize, w.chem.ns(), &w.chem, lived.mass);
    let (env, pm) = (w.life.env[i], mass / b.n());
    let heard = c.seen[..c.nseen].iter().fold([0f32; SIG], |h, &(o, _)| [0, 1].map(|k| h[k] + w.life.bodies[o].sig[k] / c.nseen as f32));
    x.clear();
    let none = SHARE_SENSE * ln(STORE_FLOOR);
    x.extend((0..ns).map(|k| if b.store[k] > 0.0 { SHARE_SENSE * ln((b.store[k] * chem.subs[k].molar_mass / mass) as f32 + STORE_FLOOR) } else { none } - b.k.minln[k]));
    x.extend_from_slice(&w.life.seen[i * ns..(i + 1) * ns]);
    let crowd = ln(1.0 + (w.life.sum[i][1] / w.grid.area()) as f32);
    let (dt, lit, sun) = ((temp - env[0]) as f32, lived.lit as f32, w.season() as f32 * std::f32::consts::TAU);
    x.extend([dt / WARM_SCALE, ln(pm as f32) / LN_MASS - 1.0, ln(1.0 + lit) / LN_LIGHT, ln(1.0 + env[1] as f32), (env[0] as f32 - T_MID) / T_SPAN, crowd, sun.sin(), sun.cos()]);
    x.extend(heard);
    x.extend(bond::sense(w, b, pm));
    x.extend_from_slice(&b.mem);
}

/// After each survey: what a brain perceives of each cell as a place to move to (liquid depth, members, their mean mass,
/// light-capture area and strength, the resistance they give to moving through; the height and temperature differences
/// are added per move, in places 0 and 6).
pub fn cells(w: &mut World) {
    let life = &w.life;
    let cellf = life.sum.iter().zip(&life.env).map(|(c, e)| {
        let resist = 1.0 + thickness(w, c[1]) as f32;
        let c = c.map(|v| v as f32);
        let n = c[0].max(1.0);
        let leaf = if c[0] > 0.0 { ln(c[2] / c[1].max(1.0)) / LN_LEAF } else { 0.0 };
        [0.0, ln(1.0 + e[1] as f32), ln(1.0 + c[0]) / LN_MEMBERS, ln((c[1] / n).max(1.0)) / LN_MASS - 1.0, leaf, c[3] / n, 0.0, ln(resist)]
    });
    w.life.cellf = cellf.collect();
}
