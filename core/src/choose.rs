//! Law 5: Choose. A body perceives a little of itself, what it holds, its memory, its cell, the
//! cells around it in 8 directions (as far as its sight), up to a few other bodies in or next to
//! its cell (their looks, signals and well-being, and how past dealings with them went; walls
//! and cover hide them) and up to 3 parts in its cell. How much it remembers, sees and thinks is set by
//! genes, and each costs energy per decision (`body::Lineage::cost`). The brain of its group,
//! plus a few numbers of its own, scores every action; one is drawn by those scores, and the
//! body emits a signal and writes its memory. Actions are physical: move, take from the node or
//! a body, put to the node or a body, watch a body (copy part of its brain), copy, rest, and the
//! actions of Bond (`bond`): form a part, hold, let go, join, break, enter (a population node
//! perceives parts as one of its members would, and does these through one member split off,
//! `copy::one`). Each costs energy paid by the body's own reactions; if it cannot pay, it fails.
//! Copying is drawn first, by its own chance. At its next decision the brain learns from the
//! change of the body's own well-being (a policy gradient whose baseline is the brain's own
//! estimate of how well-being will go).
//!
//! Bodies that stay near each other share their brain's rows (the mean of the rows they were
//! born with, plus what the group learned): a body that strays from its group's centre takes a
//! copy and starts a group; one that stays a year near a bigger group of its lineage joins it.
//! What a group learned fades at the mean forgetting rate of its members.

use crate::body::{self, Body, Layout, Lineage, Tmp, BROKEN, STRENGTH, TAKEN};
use crate::brain::{self, ln, score, squash, Shape, FIXED, OVERBS, SIG, VERBS};
use crate::chem::Phase;
use crate::event::{Event, EventKind};
use crate::grid::{DIRS, FORWARD};
use crate::rng::{Purpose, Rng};
use crate::{bond, copy, World};

/// Features of a neighbour direction and of a perceived body; most bodies perceived (the
/// `SEEN` gene sets how many), parts perceived; options.
const NF: usize = 10;
const NT: usize = 11;
const MAX_SEEN: usize = 6;
const PARTS: usize = 3;
const OPTS: usize = FIXED + 8 + VERBS * MAX_SEEN + OVERBS * PARTS;
const NONE: u32 = u32::MAX;

/// The brain's sizes for `ns` substances, `h` hidden units and `mem` memory numbers.
pub fn shape(ns: usize, h: usize, mem: usize) -> Shape {
    Shape { h, mem, ni: 2 * ns + 15 + mem, nf: NF, nt: NT, no: bond::NO }
}

/// The sizes the genes give: hidden units, memory numbers, bodies perceived, sight (cells).
pub fn sizes(genes: &[f32]) -> [usize; 4] {
    [body::HIDDEN, body::MEMORY, body::SEEN, body::SIGHT].map(|g| genes[g].round().max(0.0) as usize)
}

/// The sizes of a lineage.
pub fn sizes_of(l: &Lineage) -> [usize; 4] {
    [l.shape.h, l.shape.mem, l.seen, l.sight]
}

/// Per direction, the offsets of the cells seen that way and their weights.
pub type Sector = [Vec<(i32, i32, f32)>; 8];

/// The cells within `sight` of a cell, by the nearest of the 8 directions they lie in: offsets
/// and weights (nearer cells more, 1 / distance², distance counted in rings).
fn sector(sight: usize) -> Sector {
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

pub fn act(w: &mut World, slot: usize, b: &mut Body, t: &mut Tmp) {
    let (i, ns) = (b.cell as usize, w.chem.ns());
    let mut rng = Rng::new(w.seed, w.day, b.id(slot), Purpose::Choose);
    // Mass and heat capacity are as `body::live` just left them.
    let (mass, env, temp) = (t.mass, w.life.env[i], b.e / t.cap.max(1e-9));
    // A brain perceives sizes per member.
    let pm = mass / b.n();
    let wb = b.wellbeing_at(&w.chem, w.cfg.phys.think(w.cfg.phys.c_brain, b.dec(body::COPY_MASS)), mass, temp, t.spare);
    let Some(lin) = w.life.brains[b.lineage as usize].as_ref() else { return };
    let (s, base, nseen, sight) = (lin.shape, &lin.base, lin.seen.min(MAX_SEEN), lin.sight.max(1));
    b.mem.resize(s.mem, 0.0);
    while t.sector.len() <= sight {
        t.sector.push(sector(t.sector.len()));
    }

    // Bodies perceived: in its own cell first, then next to it from a random side. Walls
    // between two bodies hide them from each other by how closed they are; cover hides a body
    // as it shades the ground: it is seen through the others' capture area per m² of its cell,
    // a, with chance exp(-a).
    let (mut seen, mut cnt, side) = ([(0usize, false); MAX_SEEN], 0, rng.below(8));
    let my_room = bond::room(w, b);
    for q in 0..9 {
        if cnt == nseen {
            break;
        }
        let Some(j) = (if q == 0 { Some(i) } else { w.grid.step(i, DIRS[(side + q) % 8]) }) else { continue };
        for &o in w.life.at[j].iter().filter(|&&o| o as usize != slot) {
            let ob = &w.life.bodies[o as usize];
            let their = bond::room(w, ob);
            let walls = if my_room.map(|r| r.0) == their.map(|r| r.0) { 0.0 } else { my_room.map_or(0.0, |r| r.1).max(their.map_or(0.0, |r| r.1)) };
            let cover = (w.canopy[j] as f64 - ob.vis[1] as f64 * ob.n() / w.grid.area()).max(0.0);
            if cnt < nseen && (walls == 0.0 || rng.f64() as f32 >= walls) && (cover <= 0.0 || rng.f64() < (-cover).exp()) {
                (seen[cnt], cnt) = ((o as usize, q == 0), cnt + 1);
            }
        }
    }
    // Parts perceived in its cell (not the one it holds), from a random one on. A node perceives
    // them as one of its members would.
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
    let heard = seen[..cnt].iter().fold([0f32; SIG], |h, &(o, _)| [0, 1].map(|k| h[k] + w.life.bodies[o].sig[k] / cnt as f32));

    // Own stores against their safe minimums, warmth, size and light; the cell's loose
    // substances, liquid, temperature and crowding; the season; signals heard; memory.
    let chem = &w.chem;
    let x = &mut t.input;
    x.clear();
    let none = 0.25 * ln(1e-4);
    x.extend((0..ns).map(|k| if b.store[k] > 0.0 { 0.25 * ln((b.store[k] * chem.subs[k].molar_mass / mass) as f32 + 1e-4) } else { none } - b.k.minln[k]));
    x.extend_from_slice(&w.life.seen[i * ns..(i + 1) * ns]);
    let crowd = ln(1.0 + (w.life.sum[i][1] / w.grid.area()) as f32);
    let (dt, lit, sun) = ((temp - env[0]) as f32, t.lit as f32, w.season() as f32 * std::f32::consts::TAU);
    x.extend([dt / 10.0, ln(pm as f32) / 7.0 - 1.0, ln(1.0 + lit) / 15.0, ln(1.0 + env[1] as f32), (env[0] as f32 - 285.0) / 15.0, crowd, sun.sin(), sun.cos()]);
    x.extend(heard);
    x.extend(bond::sense(w, b, pm));
    x.extend_from_slice(&b.mem);
    brain::think(base, &w.life.groups[b.group as usize].rows, s, x, &mut t.thought);
    learn(w, b, s, wb, &t.thought.h);
    let out = &t.thought.out;

    // Options: the fixed actions, a move to each neighbour, each verb toward each perceived body.
    let mut logit = [f32::NEG_INFINITY; OPTS];
    for a in 0..FIXED {
        logit[a] = out[a] + b.adj[a];
    }
    // What the body can take (by its uptake selectivity) in a cell, as it perceives it. Each
    // direction's features are the weighted mean over the cells it sees that way.
    let smell = |j: usize| b.k.upt.iter().map(|&(k, u)| u * w.life.seen[j * ns + k]).sum::<f32>();
    let (here, f, sec) = (smell(i), &mut t.feat, &t.sector[sight]);
    f.clear();
    f.resize(8 * NF, 0.0);
    let mut to = [usize::MAX; 8];
    for (d, &off) in DIRS.iter().enumerate() {
        let Some(j) = w.grid.step(i, off) else { continue };
        let (fd, mut sum) = (&mut f[d * NF..(d + 1) * NF], 0.0);
        for &(dx, dy, wt) in &sec[d] {
            let Some(k) = w.grid.step(i, (dx, dy)) else { continue };
            let (ck, e) = (&w.life.cellf[k], w.life.env[k]);
            fd[..8].iter_mut().zip(ck).for_each(|(x, c)| *x += wt * c);
            fd[0] += wt * ((e[2] - env[2]) / 200.0) as f32;
            fd[6] += wt * ((e[0] - env[0]) / 5.0) as f32;
            fd[8] += wt * smell(k);
            fd[9] += wt * ln(1.0 + w.parts.at[k].len() as f32);
            // The step into the next cell is felt along its edge, worn or not.
            if (dx, dy) == off {
                fd[7] += wt * (ln(resist(w, i, d, j) as f32) - ck[7]);
            }
            sum += wt;
        }
        fd.iter_mut().for_each(|x| *x /= sum);
        fd[8] -= here;
        logit[FIXED + d] = score(&out[s.dir()..s.dir() + NF + 1], fd) + b.adj[FIXED];
        to[d] = j;
    }
    let mut fb = [0f32; MAX_SEEN * NT];
    for (q, &(o, same)) in seen[..cnt].iter().enumerate() {
        let o = &w.life.bodies[o];
        let v = o.vis;
        let known = b.recall(o.id(seen[q].0));
        let fq = &mut fb[q * NT..(q + 1) * NT];
        fq.copy_from_slice(&[ln(v[0] / pm as f32) / 7.0, ln(v[1] / v[0]) / 5.0, v[2], v[3], v[4], v[5], o.sig[0], o.sig[1], known.unwrap_or(0.0), known.is_some() as u8 as f32, o.wb / 4.0]);
        for verb in 0..VERBS {
            if same || verb == 2 {
                logit[FIXED + 8 + q * VERBS + verb] = score(&out[s.verb(verb)..s.verb(verb) + NT + 1], fq) + b.adj[FIXED + 1 + verb];
            }
        }
    }
    let (o0, mut fo, vol) = (FIXED + 8 + VERBS * MAX_SEEN, [0f32; PARTS * bond::NO], bond::volume(&w.chem, b) / b.n());
    for (q, &p) in parts[..np].iter().enumerate() {
        let fq = &mut fo[q * bond::NO..(q + 1) * bond::NO];
        fq.copy_from_slice(&bond::features(w, b, p, pm));
        for v in (0..OVERBS).filter(|&v| bond::possible(w, b, v, p, vol)) {
            let r = s.verb(VERBS + v);
            logit[o0 + q * OVERBS + v] = score(&out[r..r + bond::NO + 1], fq) + b.adj[FIXED + 1 + VERBS + v];
        }
    }
    // Letting go needs something held or walls around; joining to the ground, a held part. A node
    // does the actions of Bond through one member split off (below): with less than two members
    // it has none to spare.
    if b.n() > 1.0 && b.count < 2.0 {
        logit[4..FIXED].fill(f32::NEG_INFINITY);
        logit[o0..].fill(f32::NEG_INFINITY);
    }
    if held.is_none() {
        logit[6] = f32::NEG_INFINITY;
        if my_room.is_none() {
            logit[5] = f32::NEG_INFINITY;
        }
    }

    // Copy by its own chance (set by evolution and imitation, never learned) while a member holds
    // less than `phys.full` times its copy mass (past it, it copied at its update, `body::live`);
    // else draw one of the other actions by its score.
    let copying = rng.f64() < 1.0 / (1.0 + (-logit[1] as f64).exp()) && pm < w.cfg.phys.full * b.dec(body::COPY_MASS);
    logit[1] = f32::NEG_INFINITY;
    let top = logit.iter().cloned().fold(f32::MIN, f32::max);
    let mut p = logit.map(|l| if l == f32::NEG_INFINITY { 0.0 } else { (l - top).exp() });
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
    // output moves the log-chance of what was done (nothing for a copy) and of the signal. For
    // the values drawn around a mean (signal, shape) that is the natural gradient (times the
    // spread squared), so their noise does not use up the step allowed to the actions.
    let (l, nh, learning) = (s.learned(), s.h, b.learn_rate() > 0.0 && !w.cfg.null);
    b.trace.resize(if learning { nh + l } else { 0 }, 0.0);
    if learning {
        let (th, c) = b.trace.split_at_mut(nh);
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
        b.sig[k] = mu + w.cfg.phys.signal_noise * z;
        if learning {
            b.trace[nh + s.sig() + k] = z * w.cfg.phys.signal_noise * (1.0 - mu * mu);
        }
    }
    // The shape of a part made: elongation and hollowness, drawn around the brain's means.
    let mut shape = [0f32; 2];
    for k in 0..2 {
        let (mu, z) = (squash(out[s.form() + k]), rng.normal() as f32);
        shape[k] = mu + w.cfg.phys.shape_noise * z;
        if learning {
            b.trace[nh + s.form() + k] = if a == 4 { z * w.cfg.phys.shape_noise * (1.0 - mu * mu) } else { 0.0 };
        }
    }
    for (k, m) in b.mem.iter_mut().enumerate() {
        *m = squash(out[l + k]);
    }
    b.wb = wb;

    // A node does an action of Bond through one member, split off as an individual that acts
    // (plan 7.1). If it then holds or is inside something it lives on as one (`body::crowd` never
    // merges it); else it has no reason to stay apart and goes back into the node at once, with
    // what it got, as the next month's merge would take it back.
    let (me, mut one) = if b.n() > 1.0 && ((4..FIXED).contains(&a) || a >= o0) {
        let s = copy::one(w, b, &mut rng);
        (s, Some(std::mem::take(&mut w.life.bodies[s])))
    } else {
        (slot, None)
    };
    let node = b;
    let (b, slot) = (one.as_mut().unwrap_or(&mut *node), me);
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
                0 => take_body(w, slot, b, seen[q].0, &mut rng),
                1 => give(w, slot, b, seen[q].0),
                _ => watch(w, b, seen[q].0),
            }
        }
    }
    if let Some(mut o) = one {
        o.vis = o.visible(&w.chem, w.cfg.phys.full);
        w.life.bodies[me] = o;
        if w.life.bodies[me].hold == 0 && w.life.bodies[me].inside == 0 {
            let (o, rows) = (w.life.bodies[me].clone(), w.life.born(node).to_vec());
            w.life.remove(me);
            copy::take_in(w, node, o, &rows);
        }
    }
}

/// After each survey: what a brain perceives of each cell as a place to move to (liquid depth,
/// members, their mean mass, light-capture area and strength, the resistance they give to moving
/// through; the height and temperature differences are added per move, in places 0 and 6).
pub fn cells(w: &mut World) {
    let (life, area) = (&mut w.life, w.grid.area() as f32);
    life.cellf.clear();
    life.cellf.extend(life.sum.iter().zip(&life.env).map(|(c, e)| {
        let c = c.map(|v| v as f32);
        let n = c[0].max(1.0);
        let leaf = if c[0] > 0.0 { ln(c[2] / c[1].max(1.0)) / 5.0 } else { 0.0 };
        let resist = 1.0 + c[1] / area / w.cfg.phys.rho_pass as f32;
        [0.0, ln(1.0 + e[1] as f32), ln(1.0 + c[0]) / 10.0, ln((c[1] / n).max(1.0)) / 7.0 - 1.0, leaf, c[3] / n, 0.0, ln(resist)]
    }));
}

/// Learn from the last decision: the change of well-being since, plus the discounted estimate
/// of how it will go from the state now, less the estimate made then. The critic row moves
/// toward that; the group's learned rows and the body's own adjustment move along the
/// decision's trace by it, in units of its spread, times the body's learning rate (each row by
/// a step of its own size, `brain::learn`).
fn learn(w: &mut World, b: &mut Body, s: Shape, wb: f32, now: &[f32]) {
    let (l, lr, cr, nh) = (s.learned(), b.learn_rate(), s.critic() * s.r(), s.h);
    if b.trace.len() != nh + l || lr <= 0.0 {
        return;
    }
    let gr = &mut w.life.groups[b.group as usize];
    let (h, c) = b.trace.split_at(nh);
    let v = |x: &[f32]| gr.rows[cr + nh] + brain::dot(&gr.rows[cr..cr + nh], x);
    let delta = wb - b.wb + w.cfg.phys.discount * v(now) - v(h);
    gr.var = if gr.var > 0.0 { gr.var + 0.05 * (delta * delta - gr.var) } else { delta * delta };
    let a = lr * delta / (gr.var.sqrt() + 1e-3);
    if !a.is_finite() {
        return;
    }
    // The group's rows step by the rate split over its members, so a shared brain moves as fast
    // as one body's would (on the mean of what its members met): a node's decision is that of
    // all its members, so it weighs by its count, and an individual split off a node of thousands
    // weighs as one of them (counts as of the month's start; a new group's first body, all).
    let gl = (lr as f64 * b.n() / gr.count.max(b.n())) as f32;
    brain::fit(&mut gr.rows[cr..cr + s.r()], h, &[1.0], gl * delta);
    brain::learn(&mut gr.rows[..l * s.r()], h, c, a / lr, gl, &mut gr.ms, (w.cfg.phys.learn_beta, w.cfg.phys.learn_most));
    let bias = (0..FIXED).map(|q| c[q]).chain([c[s.dir()]]).chain((0..VERBS + OVERBS).map(|v| c[s.verb(v)]));
    for (j, cb) in b.adj.iter_mut().zip(bias) {
        *j = (*j + 0.1 * a * cb).clamp(-3.0, 3.0);
    }
}

/// Monthly: what each group learned fades by its forgetting rate. A body that has stayed for a
/// year within `phys.join` cells of the centre of a bigger group of its lineage joins it; if it was
/// alone, what it learned is blended in by its share. (No joining unless brains are shared.)
pub fn regroup(w: &mut World) {
    let (gw, gh, day, share, life) = (w.grid.w, w.grid.h, w.day, w.cfg.share_brains, &mut w.life);
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
    for g in life.groups.iter_mut().filter(|g| g.members > 0) {
        let f = (g.forget / g.members as f32).max(0.0);
        if f > 0.0 {
            let fade: Vec<f32> = g.learned().map(|l| f * l).collect();
            g.rows.iter_mut().zip(fade).for_each(|(r, x)| *r -= x);
        }
    }
    if !share {
        return;
    }
    for slot in 0..life.bodies.len() {
        let b = &life.bodies[slot];
        if !b.alive {
            continue;
        }
        let ((x, y), groups) = (((b.cell as usize) % gw, (b.cell as usize) / gw), &life.groups);
        let key = |g: u32| (groups[g as usize].members, u32::MAX - g);
        let mut best = b.group;
        for yy in y.saturating_sub(w.cfg.phys.join)..=(y + w.cfg.phys.join).min(gh - 1) {
            for xx in x.saturating_sub(w.cfg.phys.join)..=(x + w.cfg.phys.join).min(gw - 1) {
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
        } else if day - b.since >= w.cfg.phys.stay {
            let old = b.group;
            (b.group, b.near) = (best, NONE);
            let o = &life.groups[old as usize];
            let lone: Option<Vec<f32>> = (o.members == 1).then(|| o.learned().collect());
            let b = std::mem::take(&mut life.bodies[slot]);
            life.enrol(best, &b, 1);
            life.enrol(old, &b, -1);
            life.bodies[slot] = b;
            if let Some(o) = lone {
                let gr = &mut life.groups[best as usize];
                let n = gr.members as f32;
                let add: Vec<f32> = gr.learned().zip(o).map(|(l, x)| (x - l) / n).collect();
                gr.rows.iter_mut().zip(add).for_each(|(r, a)| *r += a);
            }
        }
    }
}

/// Move to a neighbour: cost by mass (with the structure it carries), distance, climb, and the
/// body mass per area already there; less, and no climb, into deep liquid while held up by what
/// it carries. The movers must fit in the free space over the cell (`phys.layer`). The part spent
/// pushing through bodies breaks their bonds (`wear`). A body that strays from its group's
/// centre leaves with a copy of its rows. A node moves as one: all its members go (they drew
/// the same action), and it stays one node.
pub fn go(w: &mut World, slot: usize, b: &mut Body, d: usize, j: usize) {
    let (i, mb) = (b.cell as usize, b.mass(&w.chem));
    if (w.life.sum[j][1] + mb) / w.cfg.phys.rho_body > w.cfg.phys.layer * w.grid.area() {
        return;
    }
    let carried = bond::carry(w, b, mb);
    let afloat = carried.is_some() && bond::afloat(w, b, j, mb);
    let climb = if afloat { 0.0 } else { (w.life.env[j][2] - w.life.env[i][2]).max(0.0) };
    let resist = resist(w, i, d, j);
    let mass = mb + carried.map_or(0.0, |c| c.1);
    let cost = mass * (w.cfg.phys.c_move * w.grid.dist(DIRS[d]) + w.cfg.gravity * climb) * resist * if afloat { w.cfg.phys.afloat } else { 1.0 };
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
    w.diag.crossings[j] += b.n() as f32;
    let cover = w.life.sum[j][1] / w.grid.area();
    let broken = wear(w, j, slot, cost * (1.0 - 1.0 / resist));
    if broken > 0.0 && cover > 0.0 {
        // The cover broken along the way was the lane's: that much ground is now clear.
        w.pass.resize(4 * n, 0.0);
        let e = &mut w.pass[edge(i, d, j)];
        *e = (*e + (broken / cover) as f32).min((w.cfg.phys.lane * w.grid.dist(DIRS[d])) as f32);
    }
    let gr = &w.life.groups[b.group as usize];
    let ((cx, cy), (x, y)) = (gr.centre(), w.grid.xy(j));
    if gr.members > 1 && (x as f32 - cx).abs().max((y as f32 - cy).abs()) > w.cfg.phys.leave {
        let (rows, var, ms) = (gr.rows.clone(), gr.var, gr.ms.clone());
        let g = w.life.new_group(b.lineage, rows);
        (w.life.groups[g as usize].var, w.life.groups[g as usize].ms) = (var, ms);
        w.life.enrol(g, b, 1);
        w.life.enrol(b.group, b, -1);
        (b.group, b.near) = (g, NONE);
    }
}

/// Energy dissipated pushing through the bodies of cell `j` breaks their bonds: each loses mass
/// by its share of their mass, less when stronger (a node loses members). What breaks off goes
/// to the ground as loose amounts, with its heat. Returns the mass broken, kg.
fn wear(w: &mut World, j: usize, mover: usize, energy: f64) -> f64 {
    let chem = &w.chem;
    let others: Vec<u32> = w.life.at[j].iter().copied().filter(|&s| s as usize != mover).collect();
    let total: f64 = others.iter().map(|&s| w.life.bodies[s as usize].mass(chem)).sum();
    let mut broken = 0.0;
    if energy <= 0.0 || total <= 0.0 {
        return 0.0;
    }
    for s in others {
        let s = s as usize;
        let mut o = std::mem::take(&mut w.life.bodies[s]);
        let (m, t) = (o.mass(&w.chem), o.temp(&w.chem));
        let f = (w.cfg.phys.walk_wear * energy / total / (o.gene(STRENGTH) + 0.1)).min(0.5);
        broken += m * f;
        let dead: Vec<f64> = (0..w.chem.ns()).map(|k| if w.chem.phase(k, t) != Phase::Gas { o.store[k] * f } else { 0.0 }).collect();
        // In a node, members are broken off; an individual loses part of itself.
        let left = body::thin(w, &mut o, f, &dead, BROKEN);
        if !left || (o.n() <= 1.0 && (o.mass(&w.chem) < 0.5 * m || o.lacking(&w.chem))) {
            body::die(w, s, o, BROKEN);
        } else {
            o.vis = o.visible(&w.chem, w.cfg.phys.full);
            w.life.bodies[s] = o;
        }
    }
    broken
}

/// The edge from cell `i` along `DIRS[d]` to `j` in `World::pass`: each edge once, as the forward
/// edge of its lower end.
fn edge(i: usize, d: usize, j: usize) -> usize {
    const F: [usize; 4] = [0, 2, 1, 3];
    if d < 4 { 4 * i + F[d] } else { 4 * j + F[d - 4] }
}

/// What it costs, against bare ground, to step from `i` along `DIRS[d]` into `j`: one, plus the
/// body mass per area in `j` over `phys.rho_pass` for the share of the passage not yet cleared.
pub(crate) fn resist(w: &World, i: usize, d: usize, j: usize) -> f64 {
    let open = w.pass.get(edge(i, d, j)).map_or(0.0, |&a| (a as f64 / (w.cfg.phys.lane * w.grid.dist(DIRS[d]))).min(1.0));
    1.0 + (1.0 - open) * w.life.sum[j][1] / w.grid.area() / w.cfg.phys.rho_pass
}

/// How much cheaper the edge from `i` along `FORWARD[f]` is than uncleared ground, for the mean
/// cover of its two cells (1 when bare or not cleared).
pub fn eased(w: &World, i: usize, f: usize) -> f64 {
    let Some(j) = w.grid.step(i, FORWARD[f]).filter(|_| !w.pass.is_empty()) else { return 1.0 };
    let (a, c) = (w.pass.get(4 * i + f).copied().unwrap_or(0.0) as f64, (w.life.sum[i][1] + w.life.sum[j][1]) / 2.0 / w.grid.area() / w.cfg.phys.rho_pass);
    (1.0 + c) / (1.0 + (1.0 - (a / (w.cfg.phys.lane * w.grid.dist(FORWARD[f]))).min(1.0)) * c)
}

/// Slow (yearly, from Flow): bodies spread back into the cleared ground of every edge from both
/// sides, at `cfg.regrow` times the cover there over `phys.rho_pass` (all of it when dense).
pub fn regrow(w: &mut World, dt: f64) {
    for i in 0..w.pass.len() / 4 {
        for (f, &off) in FORWARD.iter().enumerate() {
            if w.pass[4 * i + f] <= 0.0 {
                continue;
            }
            let Some(j) = w.grid.step(i, off) else { continue };
            let fill = ((w.life.sum[i][1] + w.life.sum[j][1]) / 2.0 / w.grid.area() / w.cfg.phys.rho_pass).min(1.0);
            let a = &mut w.pass[4 * i + f];
            *a = (*a - (2.0 * w.cfg.regrow * fill * w.grid.dist(off) * dt) as f32).max(0.0);
        }
    }
}

/// Amounts (mol) to take from `from` by the body's uptake selectivity, up to `kg` in all, for a
/// body of `mass` kg.
pub(crate) fn wanted(w: &World, b: &Body, from: impl Fn(usize) -> f64, kg: f64, mass: f64) -> Vec<f64> {
    let (chem, lay) = (&w.chem, Layout::of(&w.chem));
    let from = |k: usize| from(k).max(0.0);
    let up = |k: usize| if from(k) > 0.0 { b.gene(lay.upt + k).max(0.0) } else { 0.0 };
    let sum: f64 = (0..chem.ns()).map(up).sum();
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

/// Most a body takes per day per kg of its mass from amounts `from` (mol) spread over a cell:
/// `phys.take`, and no more than the area it works per day per kg (`phys.sweep`) holds of what
/// it takes by its uptake: the mass per m² of the cell of those substances. So a thin cover is
/// as hard to find as it is thin, whatever the body could carry.
fn reach(w: &World, b: &Body, from: impl Fn(usize) -> f64) -> f64 {
    let dens: f64 = b.k.upt.iter().map(|&(k, _)| from(k).max(0.0) * w.chem.subs[k].molar_mass).sum::<f64>() / w.grid.area();
    w.cfg.phys.take.min(w.cfg.phys.sweep * dens)
}

/// Take loose solids and liquids from the cell's ground, as `reach` allows for the mass that
/// works (`Body::work`; a node's members each work their own area: it is the node's total).
fn take_node(w: &mut World, slot: usize, b: &mut Body) {
    let (i, n) = (b.cell as usize, w.grid.n());
    let tg = w.life.env[i][0];
    let free = |k: usize| if w.chem.phase(k, tg) != Phase::Gas { w.g(k, i) } else { 0.0 };
    let mass = b.mass(&w.chem);
    let m = wanted(w, b, free, reach(w, b, free) * b.interval() as f64 * b.intake(&w.cfg.phys, mass), mass);
    if !body::pay(w, b, w.cfg.phys.c_handle * kg(w, &m), b.interval() as f64) {
        return;
    }
    let ev = EventKind::Took { by: b.id(slot), from: u64::MAX, burn: burnable(w, b, &m) as f32, cat: [body::light_cat(&w.chem, &b.genes), 0.0] };
    w.events.push(Event { day: w.day, kind: ev, node: b.cell, value: kg(w, &m) });
    for (k, &a) in m.iter().enumerate() {
        // The ground pays the heat that goes with what is taken; if it cannot, less is taken.
        let q = a * (w.chem.subs[k].cp * tg + body::fused(&w.chem, k, tg));
        let took = w.take_heat_with(i, q, a * w.chem.subs[k].cp);
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
    if !body::pay(w, b, w.cfg.phys.c_handle * kg(w, &m), b.interval() as f64) {
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
    let q = from.e * m.iter().zip(&chem.subs).map(|(a, x)| a * x.cp).sum::<f64>() / from.heat_cap(chem).max(1e-9);
    for (k, &a) in m.iter().enumerate() {
        from.store[k] -= a;
        to.store[k] += a;
    }
    from.e -= q;
    to.e += q;
}

/// Take from another body in the cell, by uptake selectivity, as much as the mass that works and
/// its force against the other's allow (a member's force: strength x what a held part adds x
/// its mass, as `bond::force`), and its room. The quicker body acts first: one that decides
/// more often than the taker has acted again within part of the taker's interval and gets away
/// with chance 1 - its interval / the taker's (the taker has paid). The other dies if it loses
/// half its mass or falls below a safe minimum (a node loses the members eaten); else it
/// remembers this against the taker. From a node (its members lie spread over the cell) the
/// taker also reaches only the area it works (`reach`); an individual it perceives is already
/// found, so only the other caps hold for it.
fn take_body(w: &mut World, slot: usize, b: &mut Body, s: usize, rng: &mut Rng) {
    let o = &w.life.bodies[s];
    let (mb, mo) = (b.mass(&w.chem), o.mass(&w.chem));
    let lever = bond::lever(w, b, mb);
    let (fb, fo) = (b.gene(STRENGTH) * lever * mb / b.n(), o.gene(STRENGTH) * bond::lever(w, o, mo) * mo / o.n());
    let rate = if o.n() > 1.0 { reach(w, b, |k| o.store[k]).min(w.cfg.phys.take * fb / (fb + fo + 1e-30)) } else { w.cfg.phys.take * fb / (fb + fo + 1e-30) };
    let m = wanted(w, b, |k| o.store[k], rate * b.interval() as f64 * b.intake(&w.cfg.phys, mb), mb);
    let resist = 1.0 + w.life.sum[b.cell as usize][1] / w.grid.area() / w.cfg.phys.rho_pass;
    let away = rng.f64() < 1.0 - o.interval() as f64 / b.interval() as f64;
    if !body::pay(w, b, w.cfg.phys.c_handle * kg(w, &m) * resist, b.interval() as f64) || away {
        return;
    }
    let mut o = std::mem::take(&mut w.life.bodies[s]);
    // From a node, whole members are eaten: the largest share taken of any store; what the taker
    // does not take of them goes to the ground.
    let node = o.n() > 1.0;
    let f = if node { m.iter().zip(&o.store).map(|(a, s)| if *s > 0.0 { a / s } else { 0.0 }).fold(0.0, f64::max) } else { 0.0 };
    let rest: Vec<f64> = o.store.iter().zip(&m).map(|(s, a)| (f * s - a).max(0.0)).collect();
    transfer(w, &mut o, b, &m);
    let got = kg(w, &m);
    let t = &mut w.parts.tally.take[(lever > 1.0) as usize];
    (t[0], t[1]) = (t[0] + 1.0, t[1] + got / mb);
    let ev = EventKind::Took { by: b.id(slot), from: o.id(s), burn: burnable(w, b, &m) as f32, cat: [body::light_cat(&w.chem, &b.genes), body::light_cat(&w.chem, &o.genes)] };
    w.events.push(Event { day: w.day, kind: ev, node: b.cell, value: got });
    if if node { !body::thin(w, &mut o, f, &rest, TAKEN) } else { o.mass(&w.chem) < 0.5 * mo || o.lacking(&w.chem) } {
        body::die(w, s, o, TAKEN);
    } else {
        o.note(b.id(slot), -(10.0 * got / mo) as f32);
        o.vis = o.visible(&w.chem, w.cfg.phys.full);
        w.life.bodies[s] = o;
    }
}

/// Put to another body in the cell: from the stores held above twice their safe minimum, by their
/// shares, up to what the other could take by its own size in the time (`phys.take` x interval x
/// its mass that works, `Body::work`), as far as it has room. A node gives as one member: a share 1/count of its spare (its
/// members cannot all reach one body). Before (review 3, M2) the cap was the giver's mass, so a
/// node of 10^7 members could give its whole spare to a 1 kg member, which rain then filled to
/// its keep level, and members of thousands of tonnes grew. It remembers this for the giver.
fn give(w: &mut World, slot: usize, b: &mut Body, s: usize) {
    let (chem, lay, mb) = (&w.chem, Layout::of(&w.chem), b.mass(&w.chem));
    let spare = |k: usize| (b.store[k] - 2.0 * b.gene(lay.min + k).max(0.0) * mb / chem.subs[k].molar_mass).max(0.0);
    let all: f64 = (0..chem.ns()).map(|k| spare(k) * chem.subs[k].molar_mass).sum();
    let o = &w.life.bodies[s];
    let mo = o.mass(chem);
    let share = (w.cfg.phys.take * b.interval() as f64 * o.intake(&w.cfg.phys, mo) / all.max(1e-9)).min(1.0 / b.n());
    let m: Vec<f64> = (0..chem.ns()).map(|k| (spare(k) * share).min((o.level(chem, k, mo) - o.store[k]).max(0.0))).collect();
    let got = kg(w, &m);
    if got <= 0.0 || !body::pay(w, b, w.cfg.phys.c_handle * got, b.interval() as f64) {
        return;
    }
    // Paying may have used some of what was to be put.
    let m: Vec<f64> = m.iter().zip(&b.store).map(|(a, s)| a.min(*s)).collect();
    let mut o = std::mem::take(&mut w.life.bodies[s]);
    transfer(w, b, &mut o, &m);
    o.note(b.id(slot), (10.0 * got / mo) as f32);
    o.vis = o.visible(&w.chem, w.cfg.phys.full);
    w.events.push(Event { day: w.day, kind: EventKind::Gave { from: b.id(slot), to: o.id(s) }, node: b.cell, value: got });
    w.life.bodies[s] = o;
}

/// Watch another body (it costs the turn): the group's rows move part of the way toward the
/// rows of the other's group if their brains have the same sizes (the share split over the
/// group's members), and the
/// body's own adjustment toward the other's; less the less alike the two bodies are (size,
/// light-capture area, strength), as a body can only copy what its own body can do.
fn watch(w: &mut World, b: &mut Body, s: usize) {
    let o = &w.life.bodies[s];
    let (og, oadj, ol, (u, v)) = (o.group as usize, o.adj, o.lineage, (o.vis, b.visible(&w.chem, w.cfg.phys.full)));
    let d = |a: f32, b: f32| ln(a.max(1e-9) / b.max(1e-9)).powi(2);
    let alike = (-0.5 * (d(u[0], v[0]) + d(u[1], v[1]) + ((u[2] - v[2]) / 0.2).powi(2))).exp();
    let alike = if w.cfg.null { 0.0 } else { w.cfg.phys.blend * alike };
    brain::blend(&mut b.adj, &oadj, alike);
    let (g, shape) = (b.group as usize, |l: u32| w.life.brains[l as usize].as_ref().map(|x| x.shape));
    if og != g && shape(ol) == shape(b.lineage) {
        let (lo, hi) = w.life.groups.split_at_mut(g.max(og));
        let (mine, theirs) = if g < og { (&mut lo[g], &hi[0]) } else { (&mut hi[0], &lo[og]) };
        brain::blend(&mut mine.rows, &theirs.rows, alike / mine.members.max(1) as f32);
    }
    let same = (ol == b.lineage) as u8 as f64;
    w.events.push(Event { day: w.day, kind: EventKind::Watched { by: g as u32, of: og as u32 }, node: b.cell, value: same });
}

