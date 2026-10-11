//! Law 5: Choose. A body perceives a little of itself, its cell, its 8 neighbours and up to 3
//! other bodies in its cell; its lineage's brain scores every action it can take, and one is
//! drawn by those scores. Actions are physical: move, take from the node or from a body, put,
//! copy, rest. Each costs energy paid by the body's own reactions; if it cannot pay, it fails.

use crate::body::{self, Body, Layout, Tmp, STRENGTH, TAKEN};
use crate::brain::{self, ln, Shape, FIXED};
use crate::chem::Phase;
use crate::grid::DIRS;
use crate::rng::{Purpose, Rng};
use crate::{copy, World};

/// Features of a neighbour direction and of a perceived body.
const NF: usize = 7;
const NT: usize = 6;
/// Energy to move one kg one metre on the flat, J.
const C_MOVE: f64 = 3.0;
/// Body mass per area that doubles the cost of entering a cell, kg/m².
const RHO_PASS: f64 = 5.0;
/// Most a body can take per day of its interval, as a share of its own mass.
const TAKE: f64 = 0.05;
/// Energy to handle one kg taken or put, J.
const C_HANDLE: f64 = 1e3;

pub fn shape(ns: usize) -> Shape {
    Shape { ni: 2 * ns + 6, nf: NF, nt: NT }
}

pub fn act(w: &mut World, slot: usize, b: &mut Body, t: &mut Tmp) {
    let (chem, lay, i) = (&w.chem, Layout::of(&w.chem), b.cell as usize);
    let ns = chem.ns();
    let Some(weights) = w.life.brains[b.lineage as usize].as_ref() else { return };
    let mut rng = Rng::new(w.seed, w.day, b.id(slot), Purpose::Choose);
    let (mass, env) = (b.mass(chem), w.life.env[i]);

    // Own stores against their safe minimums, warmth, size and light; the cell's loose
    // substances, liquid, temperature and crowding.
    let x = &mut t.input;
    x.clear();
    x.extend((0..ns).map(|k| 0.25 * ln((b.store[k] * chem.subs[k].molar_mass / mass) as f32 + 1e-4) - 0.25 * ln(b.genes[lay.min + k].max(0.0) + 1e-2)));
    x.extend_from_slice(&w.life.seen[i * ns..(i + 1) * ns]);
    let crowd = w.life.at[i].len() as f32 / w.cfg.max_bodies_per_cell as f32;
    let (dt, lit) = ((b.temp(chem) - env[0]) as f32, t.lit as f32);
    x.extend([dt / 10.0, ln(mass as f32) / 7.0 - 1.0, ln(1.0 + lit) / 15.0, ln(1.0 + env[1] as f32), (env[0] as f32 - 285.0) / 15.0, crowd]);
    brain::think(weights, shape(ns), x, &mut t.thought);

    // Options: the fixed actions, a move to each neighbour, taking from each perceived body.
    let mut logit = [f32::NEG_INFINITY; FIXED + 8 + 3];
    logit[..FIXED].copy_from_slice(&t.thought.fixed);
    let mut to = [usize::MAX; 8];
    for (d, &off) in DIRS.iter().enumerate() {
        let Some(j) = w.grid.step(i, off) else { continue };
        let (s, e) = (w.life.sum[j].map(|v| v as f32), w.life.env[j]);
        let c = s[0].max(1.0);
        let f = [((e[2] - env[2]) / 200.0) as f32, ln(1.0 + e[1] as f32), ln(1.0 + s[0]), ln((s[1] / c).max(1.0)) / 7.0 - 1.0, ln(s[2] / s[1].max(1.0)) / 5.0, s[3] / c, ((e[0] - env[0]) / 5.0) as f32];
        logit[FIXED + d] = brain::score(&t.thought.dir, &f);
        to[d] = j;
    }
    let here = &w.life.at[i];
    let mut seen = [usize::MAX; 3];
    let (mut cnt, start) = (0, if here.is_empty() { 0 } else { rng.below(here.len()) });
    for q in 0..here.len() {
        let s = here[(start + q) % here.len()] as usize;
        if cnt == 3 {
            break;
        }
        if s != slot {
            let o = w.life.bodies[s].vis;
            let f = [ln(o[0] / mass as f32) / 7.0, ln(o[1] / o[0]) / 5.0, o[2], o[3], o[4], o[5]];
            logit[FIXED + 8 + cnt] = brain::score(&t.thought.body, &f);
            seen[cnt] = s;
            cnt += 1;
        }
    }

    // Draw one action by its score.
    let top = logit.iter().cloned().fold(f32::MIN, f32::max);
    let p = logit.map(|l| (l - top).exp());
    let mut u = rng.f64() as f32 * p.iter().sum::<f32>();
    let a = p.iter().position(|&v| {
        u -= v;
        u < 0.0
    });
    match a.unwrap_or(0) {
        0 => {}
        1 => {
            copy::copy(w, slot, b);
        }
        2 => put(w, b),
        3 => take_node(w, b),
        a if a < FIXED + 8 => go(w, slot, b, a - FIXED, to[a - FIXED]),
        a => take_body(w, b, seen[a - FIXED - 8]),
    }
}

/// Move to a neighbour that has room: cost by mass, distance, climb, and the bodies already there.
fn go(w: &mut World, slot: usize, b: &mut Body, d: usize, j: usize) {
    let i = b.cell as usize;
    if w.life.at[j].len() >= w.cfg.max_bodies_per_cell {
        return;
    }
    let climb = (w.life.env[j][2] - w.life.env[i][2]).max(0.0);
    let resist = 1.0 + w.life.sum[j][1] / w.grid.area() / RHO_PASS;
    let cost = b.mass(&w.chem) * (C_MOVE * w.grid.dist(DIRS[d]) + w.cfg.gravity * climb) * resist;
    if body::pay(w, b, cost, b.interval() as f64) {
        w.life.relocate(slot, i, j);
        b.cell = j as u32;
        b.mark = w.light[j];
    }
}

/// Amounts (mol) to take from `from` by the body's uptake selectivity, up to `kg` in all.
fn wanted(w: &World, b: &Body, from: impl Fn(usize) -> f64, kg: f64) -> Vec<f64> {
    let (chem, lay) = (&w.chem, Layout::of(&w.chem));
    let from = |k: usize| from(k).max(0.0);
    let up = |k: usize| if from(k) > 0.0 { b.gene(lay.upt + k).max(0.0) } else { 0.0 };
    let sum: f64 = (0..chem.ns()).map(up).sum();
    let room = |k: usize| (b.level(chem, k) - b.store[k]).max(0.0);
    (0..chem.ns()).map(|k| if up(k) > 0.0 { (kg * up(k) / sum / chem.subs[k].molar_mass).min(0.5 * from(k)).min(room(k)) } else { 0.0 }).collect()
}

fn kg(w: &World, m: &[f64]) -> f64 {
    m.iter().zip(&w.chem.subs).map(|(a, s)| a * s.molar_mass).sum()
}

/// Take loose solids and liquids from the cell's ground.
fn take_node(w: &mut World, b: &mut Body) {
    let (i, n) = (b.cell as usize, w.grid.n());
    let tg = w.life.env[i][0];
    let free = |k: usize| if w.chem.phase(k, tg) != Phase::Gas { w.g(k, i) } else { 0.0 };
    let m = wanted(w, b, free, TAKE * b.interval() as f64 * b.mass(&w.chem));
    if !body::pay(w, b, C_HANDLE * kg(w, &m), b.interval() as f64) {
        return;
    }
    for (k, &a) in m.iter().enumerate() {
        let q = a * (w.chem.subs[k].cp * tg + body::fused(&w.chem, k, tg));
        w.ground[k * n + i] -= a;
        b.store[k] += a;
        w.e_ground[i] -= q;
        b.e += q;
    }
}

/// Release stores above the level the body keeps (`Body::level`), to the cell's ground.
fn put(w: &mut World, b: &mut Body) {
    let (i, n) = (b.cell as usize, w.grid.n());
    let m: Vec<f64> = (0..w.chem.ns()).map(|k| (b.store[k] - b.level(&w.chem, k)).clamp(0.0, b.store[k])).collect();
    if !body::pay(w, b, C_HANDLE * kg(w, &m), b.interval() as f64) {
        return;
    }
    let t = b.temp(&w.chem);
    for (k, &a) in m.iter().enumerate() {
        let a = a.min(b.store[k]);
        let q = a * w.chem.subs[k].cp * t;
        w.ground[k * n + i] += a;
        b.store[k] -= a;
        w.e_ground[i] += q;
        b.e -= q;
    }
}

/// Take from another body in the cell, by uptake selectivity, as much as the body's mass and
/// its strength against the other's allow. The other dies if it loses half its mass or falls
/// below a safe minimum.
fn take_body(w: &mut World, b: &mut Body, s: usize) {
    let o = &w.life.bodies[s];
    let (mb, mo) = (b.mass(&w.chem), o.mass(&w.chem));
    let (sb, so) = (b.gene(STRENGTH), o.gene(STRENGTH));
    let m = wanted(w, b, |k| o.store[k], TAKE * b.interval() as f64 * mb * sb / (sb + so + 1e-9));
    if !body::pay(w, b, C_HANDLE * kg(w, &m), b.interval() as f64) {
        return;
    }
    let chem = &w.chem;
    let mut o = std::mem::take(&mut w.life.bodies[s]);
    let q = o.e * m.iter().zip(&chem.subs).map(|(a, x)| a * x.cp).sum::<f64>() / o.heat_cap(chem);
    for (k, &a) in m.iter().enumerate() {
        o.store[k] -= a;
        b.store[k] += a;
    }
    o.e -= q;
    b.e += q;
    b.took[!body::light_eater(chem, &o.genes) as usize] += kg(w, &m) as f32;
    if o.mass(chem) < 0.5 * mo || o.lacking(chem) {
        body::die(w, s, o, TAKEN);
    } else {
        o.vis = o.visible(chem);
        w.life.bodies[s] = o;
    }
}
