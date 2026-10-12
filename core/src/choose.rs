//! Law 5: Choose. A body perceives a little of itself, what it holds, its memory, its cell, its 8
//! neighbours, up to 3 other bodies in or next to its cell (their looks, signals and well-being,
//! and how past dealings with them went; walls hide them) and up to 3 parts in its cell. The
//! brain of its group, plus a few numbers of its own, scores every action; one is drawn by those
//! scores, and the body emits a signal and writes its memory. Actions are physical: move, take
//! from the node or a body, put to the node or a body, watch a body (copy part of its brain),
//! copy, rest, and the actions of Bond (`bond`): form a part, hold, let go, join, break, enter.
//! Each costs energy paid by the body's own
//! reactions; if it cannot pay, it fails. Copying is drawn first, by its own chance. At its next
//! decision the brain learns from the change of the body's own well-being (a policy gradient
//! whose baseline is the brain's own estimate of how well-being will go).
//!
//! Bodies that stay near each other share their brain's rows: a body that strays from its
//! group's centre takes a copy and starts a group; one that stays a year near a bigger group of
//! its lineage joins it.

use crate::body::{self, Body, Layout, Tmp, BROKEN, STRENGTH, TAKEN};
use crate::brain::{self, ln, score, squash, Shape, FIXED, H, OVERBS, R, SIG, VERBS};
use crate::chem::Phase;
use crate::event::{Event, EventKind};
use crate::grid::DIRS;
use crate::rng::{Purpose, Rng};
use crate::{bond, copy, World};

/// Features of a neighbour direction and of a perceived body; bodies and parts perceived; options.
const NF: usize = 10;
const NT: usize = 11;
const SEEN: usize = 3;
const OPTS: usize = FIXED + 8 + VERBS * SEEN + OVERBS * SEEN;
/// Energy to move one kg one metre on the flat, J.
pub(crate) const C_MOVE: f64 = 3.0;
/// Body mass per area that doubles the cost of entering a cell, kg/m².
const RHO_PASS: f64 = 5.0;
/// Body mass broken per J a mover dissipates against it, times (strength + 0.1), kg/J.
const WEAR: f64 = 3e-5;
/// Most a body can take per day of its interval, as a share of its own mass.
pub(crate) const TAKE: f64 = 0.05;
/// Energy to handle one kg taken or put, J.
pub(crate) const C_HANDLE: f64 = 1e3;
/// Spread of the shape of a part made around its mean.
const SHAPE_NOISE: f32 = 0.5;
/// Spread of the signal around its mean.
const NOISE: f32 = 0.3;
/// Weight of well-being one decision ahead against well-being now.
const DISCOUNT: f32 = 0.9;
/// Share of the way a watcher's rows move toward the watched one's (split over its group).
const BLEND: f32 = 0.3;
/// A body further than `LEAVE` cells from its group's centre leaves it; one within `JOIN` cells
/// of a bigger group's centre for `STAY` days joins it.
const LEAVE: f32 = 2.5;
const JOIN: usize = 2;
const STAY: u64 = 360;
const NONE: u32 = u32::MAX;
/// Share of what a group learned that fades each month.
const FORGET: f32 = 0.02;

pub fn shape(ns: usize) -> Shape {
    Shape { ni: 2 * ns + 15 + brain::MEM, nf: NF, nt: NT, no: bond::NO }
}

/// A new group of `lineage`, starting from the lineage's brain.
pub fn found(w: &mut World, lineage: u32) -> u32 {
    let base = shape(w.chem.ns()).base();
    let rows = w.life.brains[lineage as usize].as_ref().map_or(Vec::new(), |b| b[base..].to_vec());
    w.life.new_group(lineage, rows)
}

pub fn act(w: &mut World, slot: usize, b: &mut Body, t: &mut Tmp) {
    let (i, ns, s) = (b.cell as usize, w.chem.ns(), shape(w.chem.ns()));
    let mut rng = Rng::new(w.seed, w.day, b.id(slot), Purpose::Choose);
    let (mass, env) = (b.mass(&w.chem), w.life.env[i]);
    let wb = b.wellbeing(&w.chem, mass);
    let Some(base) = w.life.brains[b.lineage as usize].as_ref() else { return };

    // Bodies perceived: in its own cell first, then next to it from a random side. Walls
    // between two bodies hide them from each other by how closed they are.
    let (mut seen, mut cnt, side) = ([(0usize, false); SEEN], 0, rng.below(8));
    let my_room = bond::room(w, b);
    for q in 0..9 {
        if cnt == SEEN {
            break;
        }
        let Some(j) = (if q == 0 { Some(i) } else { w.grid.step(i, DIRS[(side + q) % 8]) }) else { continue };
        for &o in w.life.at[j].iter().filter(|&&o| o as usize != slot) {
            let their = bond::room(w, &w.life.bodies[o as usize]);
            let walls = if my_room.map(|r| r.0) == their.map(|r| r.0) { 0.0 } else { my_room.map_or(0.0, |r| r.1).max(their.map_or(0.0, |r| r.1)) };
            if cnt < SEEN && (walls == 0.0 || rng.f64() as f32 >= walls) {
                (seen[cnt], cnt) = ((o as usize, q == 0), cnt + 1);
            }
        }
    }
    // Parts perceived in its cell (not the one it holds), from a random one on.
    let (held, list) = (bond::held(w, b), &w.parts.at[i]);
    let (mut parts, mut np, off) = ([0usize; SEEN], 0, if list.is_empty() { 0 } else { rng.below(list.len()) });
    for q in 0..list.len() {
        let p = list[(off + q) % list.len()] as usize;
        if np < SEEN && Some(p) != held {
            (parts[np], np) = (p, np + 1);
        }
    }
    let heard = seen[..cnt].iter().fold([0f32; SIG], |h, &(o, _)| [0, 1].map(|k| h[k] + w.life.bodies[o].sig[k] / cnt as f32));

    // Own stores against their safe minimums, warmth, size and light; the cell's loose
    // substances, liquid, temperature and crowding; the season; signals heard; memory.
    let chem = &w.chem;
    let x = &mut t.input;
    x.clear();
    let none = 0.25 * ln(1e-4);
    x.extend((0..ns).map(|k| if b.store[k] > 0.0 { 0.25 * ln((b.store[k] * chem.subs[k].molar_mass / mass) as f32 + 1e-4) } else { none } - b.k.minln[k]));
    x.extend_from_slice(&w.life.seen[i * ns..(i + 1) * ns]);
    let crowd = w.life.at[i].len() as f32 / w.cfg.max_bodies_per_cell as f32;
    let (dt, lit, sun) = ((b.temp(chem) - env[0]) as f32, t.lit as f32, w.season() as f32 * std::f32::consts::TAU);
    x.extend([dt / 10.0, ln(mass as f32) / 7.0 - 1.0, ln(1.0 + lit) / 15.0, ln(1.0 + env[1] as f32), (env[0] as f32 - 285.0) / 15.0, crowd, sun.sin(), sun.cos()]);
    x.extend(heard);
    x.extend(bond::sense(w, b, mass));
    x.extend(b.mem);
    brain::think(base, &w.life.groups[b.group as usize].rows, s, x, &mut t.thought);
    learn(w, b, s, wb, &t.thought.h);
    let out = &t.thought.out;

    // Options: the fixed actions, a move to each neighbour, each verb toward each perceived body.
    let mut logit = [f32::NEG_INFINITY; OPTS];
    for a in 0..FIXED {
        logit[a] = out[a] + b.adj[a];
    }
    // What the body can take (by its uptake selectivity) in a cell, as it perceives it.
    let smell = |j: usize| b.k.upt.iter().map(|&(k, u)| u * w.life.seen[j * ns + k]).sum::<f32>();
    let (here, f) = (smell(i), &mut t.feat);
    f.clear();
    f.resize((8 * NF).max(SEEN * NT), 0.0);
    let mut to = [usize::MAX; 8];
    for (d, &off) in DIRS.iter().enumerate() {
        let Some(j) = w.grid.step(i, off) else { continue };
        let (cj, e) = (&w.life.cellf[j], w.life.env[j]);
        let fd = &mut f[d * NF..(d + 1) * NF];
        fd[..8].copy_from_slice(cj);
        (fd[0], fd[6], fd[8]) = (((e[2] - env[2]) / 200.0) as f32, ((e[0] - env[0]) / 5.0) as f32, smell(j) - here);
        fd[9] = ln(1.0 + w.parts.at[j].len() as f32);
        logit[FIXED + d] = score(&out[s.dir()..s.dir() + NF + 1], fd) + b.adj[FIXED];
        to[d] = j;
    }
    let mut fb = [0f32; SEEN * NT];
    for (q, &(o, same)) in seen[..cnt].iter().enumerate() {
        let o = &w.life.bodies[o];
        let v = o.vis;
        let known = b.recall(o.id(seen[q].0));
        let fq = &mut fb[q * NT..(q + 1) * NT];
        fq.copy_from_slice(&[ln(v[0] / mass as f32) / 7.0, ln(v[1] / v[0]) / 5.0, v[2], v[3], v[4], v[5], o.sig[0], o.sig[1], known.unwrap_or(0.0), known.is_some() as u8 as f32, o.wb / 4.0]);
        for verb in 0..VERBS {
            if same || verb == 2 {
                logit[FIXED + 8 + q * VERBS + verb] = score(&out[s.verb(verb)..s.verb(verb) + NT + 1], fq) + b.adj[FIXED + 1 + verb];
            }
        }
    }
    let (o0, mut fo, vol) = (FIXED + 8 + VERBS * SEEN, [0f32; SEEN * bond::NO], bond::volume(&w.chem, b));
    for (q, &p) in parts[..np].iter().enumerate() {
        let fq = &mut fo[q * bond::NO..(q + 1) * bond::NO];
        fq.copy_from_slice(&bond::features(w, b, p, mass));
        for v in (0..OVERBS).filter(|&v| bond::possible(w, b, v, p, vol)) {
            let r = s.verb(VERBS + v);
            logit[o0 + q * OVERBS + v] = score(&out[r..r + bond::NO + 1], fq) + b.adj[FIXED + 1 + VERBS + v];
        }
    }
    // Letting go needs something held or walls around; joining to the ground, a held part.
    if held.is_none() {
        logit[6] = f32::NEG_INFINITY;
        if my_room.is_none() {
            logit[5] = f32::NEG_INFINITY;
        }
    }

    // Copy by its own chance (set by evolution and imitation, never learned); else draw one of
    // the other actions by its score.
    let copying = rng.f64() < 1.0 / (1.0 + (-logit[1] as f64).exp());
    logit[1] = f32::NEG_INFINITY;
    let top = logit.iter().cloned().fold(f32::MIN, f32::max);
    let mut p = logit.map(|l| (l - top).exp());
    let sum = p.iter().sum::<f32>();
    p.iter_mut().for_each(|v| *v /= sum);
    let mut u = rng.f64() as f32;
    let a = p.iter().position(|&v| {
        u -= v;
        u < 0.0
    });
    let a = if copying { 1 } else { a.unwrap_or(0) };
    if !copying {
        (w.diag.decisions, w.diag.greedy) = (w.diag.decisions + 1.0, w.diag.greedy + (logit[a] >= top) as u8 as f64);
    }

    // The trace of this decision for learning: the hidden layer, and per learned row how its
    // output moves the log-chance of what was done (nothing for a copy) and of the signal.
    let (l, learning) = (s.learned(), b.learn_rate() > 0.0);
    b.trace.resize(if learning { H + l } else { 0 }, 0.0);
    if learning {
        let (th, c) = b.trace.split_at_mut(H);
        th.copy_from_slice(&t.thought.h);
        c.fill(0.0);
        let g = |o: usize| if copying { 0.0 } else { (o == a) as u8 as f32 - p[o] };
        for q in 0..FIXED {
            c[q] = g(q);
        }
        let mut add = |row: usize, gq: f32, fq: &[f32]| {
            c[row] += gq;
            for (cr, x) in c[row + 1..].iter_mut().zip(fq) {
                *cr += gq * x;
            }
        };
        for d in (0..8).filter(|&d| to[d] != usize::MAX) {
            add(s.dir(), g(FIXED + d), &f[d * NF..(d + 1) * NF]);
        }
        for q in 0..cnt {
            for verb in 0..VERBS {
                add(s.verb(verb), g(FIXED + 8 + q * VERBS + verb), &fb[q * NT..(q + 1) * NT]);
            }
        }
        for q in 0..np {
            for v in 0..OVERBS {
                add(s.verb(VERBS + v), g(o0 + q * OVERBS + v), &fo[q * bond::NO..(q + 1) * bond::NO]);
            }
        }
    }
    for k in 0..SIG {
        let (mu, z) = (squash(out[s.sig() + k]), rng.normal() as f32);
        b.sig[k] = mu + NOISE * z;
        if learning {
            b.trace[H + s.sig() + k] = z / NOISE * (1.0 - mu * mu);
        }
    }
    // The shape of a part made: elongation and hollowness, drawn around the brain's means.
    let mut shape = [0f32; 2];
    for k in 0..2 {
        let (mu, z) = (squash(out[s.form() + k]), rng.normal() as f32);
        shape[k] = mu + SHAPE_NOISE * z;
        if learning {
            b.trace[H + s.form() + k] = if a == 4 { z / SHAPE_NOISE * (1.0 - mu * mu) } else { 0.0 };
        }
    }
    for k in 0..brain::MEM {
        b.mem[k] = squash(out[l + k]);
    }
    b.wb = wb;

    match a {
        0 => {}
        1 => {
            copy::copy(w, slot, b);
        }
        2 => put(w, b),
        3 => take_node(w, slot, b),
        4 => bond::form(w, slot, b, [shape[0], 0.5 * (shape[1] + 1.0)]),
        5 => bond::release(w, b),
        6 => bond::anchor(w, b),
        7 => bond::dig(w, b),
        a if a < FIXED + 8 => go(w, slot, b, a - FIXED, to[a - FIXED]),
        a if a >= o0 => bond::act(w, slot, b, (a - o0) % OVERBS, parts[(a - o0) / OVERBS]),
        a => {
            let (q, verb) = ((a - FIXED - 8) / VERBS, (a - FIXED - 8) % VERBS);
            match verb {
                0 => take_body(w, slot, b, seen[q].0),
                1 => give(w, slot, b, seen[q].0),
                _ => watch(w, b, seen[q].0),
            }
        }
    }
}

/// After each survey: what a brain perceives of each cell as a place to move to (liquid depth,
/// bodies, their mean mass, light-capture area and strength, the resistance they give to moving
/// through; the height and temperature differences are added per move, in places 0 and 6).
pub fn cells(w: &mut World) {
    let (life, area) = (&mut w.life, w.grid.area() as f32);
    life.cellf.clear();
    life.cellf.extend(life.sum.iter().zip(&life.env).map(|(c, e)| {
        let c = c.map(|v| v as f32);
        let n = c[0].max(1.0);
        let leaf = if c[0] > 0.0 { ln(c[2] / c[1].max(1.0)) / 5.0 } else { 0.0 };
        let resist = 1.0 + c[1] / area / RHO_PASS as f32;
        [0.0, ln(1.0 + e[1] as f32), ln(1.0 + c[0]), ln((c[1] / n).max(1.0)) / 7.0 - 1.0, leaf, c[3] / n, 0.0, ln(resist)]
    }));
}

/// Learn from the last decision: the change of well-being since, plus the discounted estimate
/// of how it will go from the state now, less the estimate made then. The critic row moves
/// toward that; the group's learned rows and the body's own adjustment move along the
/// decision's trace by it, in units of its spread, times the body's learning rate.
fn learn(w: &mut World, b: &mut Body, s: Shape, wb: f32, now: &[f32; H]) {
    let (l, lr, cr) = (s.learned(), b.learn_rate(), s.critic() * R);
    if b.trace.len() != H + l || lr <= 0.0 {
        return;
    }
    let gr = &mut w.life.groups[b.group as usize];
    let (h, c): (&[f32; H], _) = (b.trace[..H].try_into().expect("trace"), &b.trace[H..]);
    let v = |x: &[f32; H]| gr.rows[cr + H] + brain::dot(&gr.rows[cr..cr + H], x);
    let delta = wb - b.wb + DISCOUNT * v(now) - v(h);
    gr.var += 0.05 * (delta * delta - gr.var);
    let a = lr * delta / (gr.var.sqrt() + 1e-3);
    if !a.is_finite() {
        return;
    }
    brain::learn(&mut gr.rows[cr..cr + R], h, &[1.0], lr * delta);
    brain::learn(&mut gr.rows[..l * R], h, c, a);
    let bias = (0..FIXED).map(|q| c[q]).chain([c[s.dir()]]).chain((0..VERBS + OVERBS).map(|v| c[s.verb(v)]));
    for (j, cb) in b.adj.iter_mut().zip(bias) {
        *j = (*j + 0.1 * a * cb).clamp(-3.0, 3.0);
    }
}

/// Monthly: a body that has stayed for a year within `JOIN` cells of the centre of a bigger
/// group of its lineage joins it; if it was alone, its rows are blended in by its share.
pub fn regroup(w: &mut World) {
    let (gw, gh, day, ns, life) = (w.grid.w, w.grid.h, w.day, w.chem.ns(), &mut w.life);
    let at = |g: &body::Group| {
        let (x, y) = g.centre();
        (y.round() as usize).min(gh - 1) * gw + (x.round() as usize).min(gw - 1)
    };
    // Groups by the cell of their centre.
    let mut head = vec![0u32; gw * gh + 1];
    for g in life.groups.iter().filter(|g| g.members > 0) {
        head[at(g) + 1] += 1;
    }
    for c in 0..gw * gh {
        head[c + 1] += head[c];
    }
    let (mut list, mut fill) = (vec![0u32; head[gw * gh] as usize], head.clone());
    for (k, g) in life.groups.iter().enumerate().filter(|(_, g)| g.members > 0) {
        list[fill[at(g)] as usize] = k as u32;
        fill[at(g)] += 1;
    }
    // What a group learned fades back toward its lineage's starting rows.
    let base = shape(ns).base();
    for g in life.groups.iter_mut().filter(|g| g.members > 0) {
        if let Some(start) = &life.brains[g.lineage as usize] {
            brain::blend(&mut g.rows, &start[base..], FORGET);
        }
    }
    for slot in 0..life.bodies.len() {
        let b = &life.bodies[slot];
        if !b.alive {
            continue;
        }
        let ((x, y), groups) = (((b.cell as usize) % gw, (b.cell as usize) / gw), &life.groups);
        let key = |g: u32| (groups[g as usize].members, u32::MAX - g);
        let mut best = b.group;
        for yy in y.saturating_sub(JOIN)..=(y + JOIN).min(gh - 1) {
            for xx in x.saturating_sub(JOIN)..=(x + JOIN).min(gw - 1) {
                let c = yy * gw + xx;
                for &g in &list[head[c] as usize..head[c + 1] as usize] {
                    if groups[g as usize].members > 0 && groups[g as usize].lineage == b.lineage && key(g) > key(best) {
                        best = g;
                    }
                }
            }
        }
        let b = &mut life.bodies[slot];
        if best == b.group {
            b.near = NONE;
        } else if best != b.near {
            (b.near, b.since) = (best, day);
        } else if day - b.since >= STAY {
            let (old, cell) = (b.group, b.cell);
            (b.group, b.near) = (best, NONE);
            if life.groups[old as usize].members == 1 {
                let o = std::mem::take(&mut life.groups[old as usize].rows);
                let gr = &mut life.groups[best as usize];
                brain::blend(&mut gr.rows, &o, 1.0 / (gr.members + 1) as f32);
                life.groups[old as usize].rows = o;
            }
            life.enrol(best, cell, 1);
            life.enrol(old, cell, -1);
        }
    }
}

/// Move to a neighbour that has room: cost by mass (with the structure it carries), distance,
/// climb, and the bodies already there; less, and no climb, into deep liquid while held up by
/// what it carries. The part spent pushing through bodies breaks their bonds (`wear`). A body
/// that strays from its group's centre leaves with a copy of its rows.
fn go(w: &mut World, slot: usize, b: &mut Body, d: usize, j: usize) {
    let i = b.cell as usize;
    if w.life.at[j].len() >= w.cfg.max_bodies_per_cell {
        return;
    }
    let mb = b.mass(&w.chem);
    let carried = bond::carry(w, b, mb);
    let afloat = carried.is_some() && bond::afloat(w, b, j, mb);
    let climb = if afloat { 0.0 } else { (w.life.env[j][2] - w.life.env[i][2]).max(0.0) };
    let resist = 1.0 + w.life.sum[j][1] / w.grid.area() / RHO_PASS;
    let mass = mb + carried.map_or(0.0, |c| c.1);
    let cost = mass * (C_MOVE * w.grid.dist(DIRS[d]) + w.cfg.gravity * climb) * resist * if afloat { bond::AFLOAT } else { 1.0 };
    if !body::pay(w, b, cost, b.interval() as f64) {
        return;
    }
    bond::moved(w, b, carried, j, mb);
    w.parts.tally.afloat += afloat as u8 as f64;
    w.life.relocate(slot, b.group, i, j);
    b.cell = j as u32;
    b.mark = w.light[j];
    let n = w.grid.n();
    w.diag.crossings.resize(n, 0.0);
    w.diag.crossings[j] += 1.0;
    wear(w, j, slot, cost * (1.0 - 1.0 / resist));
    let gr = &w.life.groups[b.group as usize];
    let ((cx, cy), (x, y)) = (gr.centre(), w.grid.xy(j));
    if gr.members > 1 && (x as f32 - cx).abs().max((y as f32 - cy).abs()) > LEAVE {
        let (rows, var) = (gr.rows.clone(), gr.var);
        let g = w.life.new_group(b.lineage, rows);
        w.life.groups[g as usize].var = var;
        w.life.enrol(g, b.cell, 1);
        w.life.enrol(b.group, b.cell, -1);
        (b.group, b.near) = (g, NONE);
    }
}

/// Energy dissipated pushing through the bodies of cell `j` breaks their bonds: each loses mass
/// by its share of their mass, less when stronger. What breaks off goes to the ground as loose
/// amounts, with its heat.
fn wear(w: &mut World, j: usize, mover: usize, energy: f64) {
    let chem = &w.chem;
    let others: Vec<u32> = w.life.at[j].iter().copied().filter(|&s| s as usize != mover).collect();
    let total: f64 = others.iter().map(|&s| w.life.bodies[s as usize].mass(chem)).sum();
    if energy <= 0.0 || total <= 0.0 {
        return;
    }
    let n = w.grid.n();
    for s in others {
        let s = s as usize;
        let mut o = std::mem::take(&mut w.life.bodies[s]);
        let (m, t) = (o.mass(&w.chem), o.temp(&w.chem));
        let f = (WEAR * energy / total / (o.gene(STRENGTH) + 0.1)).min(0.5);
        for k in (0..w.chem.ns()).filter(|&k| w.chem.phase(k, t) != Phase::Gas) {
            let a = o.store[k] * f;
            let q = a * w.chem.subs[k].cp * t;
            o.store[k] -= a;
            w.ground[k * n + j] += a;
            o.e -= q;
            w.e_ground[j] += q;
        }
        if o.mass(&w.chem) < 0.5 * m || o.lacking(&w.chem) {
            body::die(w, s, o, BROKEN);
        } else {
            o.vis = o.visible(&w.chem);
            w.life.bodies[s] = o;
        }
    }
}

/// Amounts (mol) to take from `from` by the body's uptake selectivity, up to `kg` in all.
pub(crate) fn wanted(w: &World, b: &Body, from: impl Fn(usize) -> f64, kg: f64) -> Vec<f64> {
    let (chem, lay) = (&w.chem, Layout::of(&w.chem));
    let from = |k: usize| from(k).max(0.0);
    let up = |k: usize| if from(k) > 0.0 { b.gene(lay.upt + k).max(0.0) } else { 0.0 };
    let sum: f64 = (0..chem.ns()).map(up).sum();
    let mass = b.mass(chem);
    let room = |k: usize| (b.level(chem, k, mass) - b.store[k]).max(0.0);
    (0..chem.ns()).map(|k| if up(k) > 0.0 { (kg * up(k) / sum / chem.subs[k].molar_mass).min(0.5 * from(k)).min(room(k)) } else { 0.0 }).collect()
}

pub(crate) fn kg(w: &World, m: &[f64]) -> f64 {
    m.iter().zip(&w.chem.subs).map(|(a, s)| a * s.molar_mass).sum()
}

/// Mass of the amounts `m` that the body's reactions can burn, kg.
fn burnable(w: &World, b: &Body, m: &[f64]) -> f64 {
    let t = b.temp(&w.chem);
    let fuel = |k: usize| w.chem.phase(k, t) != Phase::Gas && b.k.burn.iter().any(|&(r, _)| w.chem.reactions[r].inputs.iter().any(|p| p.0 as usize == k));
    m.iter().enumerate().filter(|&(k, _)| fuel(k)).map(|(k, a)| a * w.chem.subs[k].molar_mass).sum()
}

/// Take loose solids and liquids from the cell's ground.
fn take_node(w: &mut World, slot: usize, b: &mut Body) {
    let (i, n) = (b.cell as usize, w.grid.n());
    let tg = w.life.env[i][0];
    let free = |k: usize| if w.chem.phase(k, tg) != Phase::Gas { w.g(k, i) } else { 0.0 };
    let m = wanted(w, b, free, TAKE * b.interval() as f64 * b.mass(&w.chem));
    if !body::pay(w, b, C_HANDLE * kg(w, &m), b.interval() as f64) {
        return;
    }
    let ev = EventKind::Took { by: b.id(slot), from: u64::MAX, burn: burnable(w, b, &m) as f32, cat: [body::light_cat(&w.chem, &b.genes), 0.0] };
    w.events.push(Event { day: w.day, kind: ev, node: b.cell, value: kg(w, &m) });
    for (k, &a) in m.iter().enumerate() {
        // The ground pays the heat that goes with what is taken; if it cannot, less is taken.
        let q = a * (w.chem.subs[k].cp * tg + body::fused(&w.chem, k, tg));
        let took = w.take_heat(i, q);
        let a = if q > 0.0 { a * crate::share(took, q) } else { a };
        w.ground[k * n + i] -= a;
        b.store[k] += a;
        b.e += took;
    }
}

/// Release stores above the level the body keeps (`Body::level`), to the cell's ground.
fn put(w: &mut World, b: &mut Body) {
    let (i, n) = (b.cell as usize, w.grid.n());
    let mass = b.mass(&w.chem);
    let m: Vec<f64> = (0..w.chem.ns()).map(|k| (b.store[k] - b.level(&w.chem, k, mass)).clamp(0.0, b.store[k])).collect();
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

/// Move amounts `m` (mol) from `from` to `to`, with their share of `from`'s heat.
fn transfer(w: &World, from: &mut Body, to: &mut Body, m: &[f64]) {
    let chem = &w.chem;
    let q = from.e * m.iter().zip(&chem.subs).map(|(a, x)| a * x.cp).sum::<f64>() / from.heat_cap(chem);
    for (k, &a) in m.iter().enumerate() {
        from.store[k] -= a;
        to.store[k] += a;
    }
    from.e -= q;
    to.e += q;
}

/// Take from another body in the cell, by uptake selectivity, as much as the body's mass and
/// its strength against the other's allow (each times what a held part adds), and its room. The
/// other dies if it loses half its mass or falls below a safe minimum; else it remembers this
/// against the taker.
fn take_body(w: &mut World, slot: usize, b: &mut Body, s: usize) {
    let o = &w.life.bodies[s];
    let (mb, mo) = (b.mass(&w.chem), o.mass(&w.chem));
    let lever = bond::lever(w, b, mb);
    let (sb, so) = (b.gene(STRENGTH) * lever, o.gene(STRENGTH) * bond::lever(w, o, mo));
    let m = wanted(w, b, |k| o.store[k], TAKE * b.interval() as f64 * mb * sb / (sb + so + 1e-9));
    if !body::pay(w, b, C_HANDLE * kg(w, &m), b.interval() as f64) {
        return;
    }
    let mut o = std::mem::take(&mut w.life.bodies[s]);
    transfer(w, &mut o, b, &m);
    let got = kg(w, &m);
    let t = &mut w.parts.tally.take[(lever > 1.0) as usize];
    (t[0], t[1]) = (t[0] + 1.0, t[1] + got / mb);
    let ev = EventKind::Took { by: b.id(slot), from: o.id(s), burn: burnable(w, b, &m) as f32, cat: [body::light_cat(&w.chem, &b.genes), body::light_cat(&w.chem, &o.genes)] };
    w.events.push(Event { day: w.day, kind: ev, node: b.cell, value: got });
    if o.mass(&w.chem) < 0.5 * mo || o.lacking(&w.chem) {
        body::die(w, s, o, TAKEN);
    } else {
        o.note(b.id(slot), -(10.0 * got / mo) as f32);
        o.vis = o.visible(&w.chem);
        w.life.bodies[s] = o;
    }
}

/// Put to another body in the cell: up to what a take would move, from the stores held above
/// twice their safe minimum, by their shares, as far as the other has room. It remembers this
/// for the giver.
fn give(w: &mut World, slot: usize, b: &mut Body, s: usize) {
    let (chem, lay, mb) = (&w.chem, Layout::of(&w.chem), b.mass(&w.chem));
    let spare = |k: usize| (b.store[k] - 2.0 * b.gene(lay.min + k).max(0.0) * mb / chem.subs[k].molar_mass).max(0.0);
    let all: f64 = (0..chem.ns()).map(|k| spare(k) * chem.subs[k].molar_mass).sum();
    let o = &w.life.bodies[s];
    let share = (TAKE * b.interval() as f64 * mb / all.max(1e-9)).min(1.0);
    let mo = o.mass(chem);
    let m: Vec<f64> = (0..chem.ns()).map(|k| (spare(k) * share).min((o.level(chem, k, mo) - o.store[k]).max(0.0))).collect();
    let got = kg(w, &m);
    if got <= 0.0 || !body::pay(w, b, C_HANDLE * got, b.interval() as f64) {
        return;
    }
    // Paying may have used some of what was to be put.
    let m: Vec<f64> = m.iter().zip(&b.store).map(|(a, s)| a.min(*s)).collect();
    let mut o = std::mem::take(&mut w.life.bodies[s]);
    transfer(w, b, &mut o, &m);
    o.note(b.id(slot), (10.0 * got / mo) as f32);
    o.vis = o.visible(&w.chem);
    w.events.push(Event { day: w.day, kind: EventKind::Gave { from: b.id(slot), to: o.id(s) }, node: b.cell, value: got });
    w.life.bodies[s] = o;
}

/// Watch another body (it costs the turn): the group's rows move part of the way
/// toward the rows of the other's group (the share split over the group's members), and the
/// body's own adjustment toward the other's; less the less alike the two bodies are (size,
/// light-capture area, strength), as a body can only copy what its own body can do.
fn watch(w: &mut World, b: &mut Body, s: usize) {
    let o = &w.life.bodies[s];
    let (og, oadj, ol, (u, v)) = (o.group as usize, o.adj, o.lineage, (o.vis, b.visible(&w.chem)));
    let d = |a: f32, b: f32| ln(a.max(1e-9) / b.max(1e-9)).powi(2);
    let alike = (-0.5 * (d(u[0], v[0]) + d(u[1], v[1]) + ((u[2] - v[2]) / 0.2).powi(2))).exp();
    brain::blend(&mut b.adj, &oadj, BLEND * alike);
    let g = b.group as usize;
    if og != g {
        let (lo, hi) = w.life.groups.split_at_mut(g.max(og));
        let (mine, theirs) = if g < og { (&mut lo[g], &hi[0]) } else { (&mut hi[0], &lo[og]) };
        brain::blend(&mut mine.rows, &theirs.rows, BLEND * alike / mine.members.max(1) as f32);
    }
    let same = (ol == b.lineage) as u8 as f64;
    w.events.push(Event { day: w.day, kind: EventKind::Watched { by: g as u32, of: og as u32 }, node: b.cell, value: same });
}

