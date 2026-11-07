//! A month of the cover of each cell of a chunk (laws.md 4.5): it catches light and runs its light reaction, pays its
//! upkeep by burning its fuel, loses what is unpaid, worn out or frozen, builds from its fuel and the minerals of the
//! ground (fixing what it lacks if it can), and sends seeds out. Everything it takes and gives is booked through the
//! chunk's moves, with its heat; its own side it books itself.

use super::{dec, distance, Cover, Parts, APART, EMPTY_KG, FIX, FIX_OFF, FROST, FUEL, LIGHT, MATTER, NG, NO_ROOT, NP, REPAIR, SEED, SLOTS, STORE, WALL, WALLS, MOIST};
use crate::body::{AGE, PHASE, SPENT};
use crate::chem::{Chemistry, Phase, Reaction};
use crate::rng::{Purpose, Rng};
use crate::store::{Class, Moves, Node};
use crate::units::{DAY, MAX_SUBSTANCES};
use crate::{life::Chunk, World, AIR};

/// The month of the cover of every cell of chunk `d`, in the order of its cells; its seeds wait in `Cover::seeds` until
/// all chunks ran (`land`).
pub fn step(w: &World, d: &mut Chunk) {
    let Some(p) = Parts::of(&w.chem) else { return };
    let dt = w.cfg.calendar().month as f64;
    for i in d.cells() {
        if d.cover(i).any() {
            cell(w, d, &p, i, dt);
        } else {
            let c = d.cover(i);
            (c.mark, c.keep) = (w.light[i], 0.0);
        }
    }
}

/// What a slot is in a month: its built cover and its living cover (`Cover::living`), kg; its capture area and what of
/// it absorbs, m²; its light per unit of absorbing area over the cover's mean, and at the bottom of its layer
/// (`math::shade`); its activity; what one more kg of it would store in the month at the bottom of its layer, J (`light`).
#[derive(Clone, Copy, Default)]
struct Slot {
    built: f64,
    living: f64,
    leaf: f64,
    abs: f64,
    shade: f64,
    bottom: f64,
    act: f64,
    gain: f64,
}

/// What a cell's cover changes outside itself in a month: per substance the mol it takes in (+) or gives out (-), the
/// heat its reactions release, J, and per substance its remains, mol; and per gas what the cell's share of its air node
/// offers, mol (infinite for what is not a gas: the ground's inputs are limited where they are read).
struct Books {
    ext: [f64; MAX_SUBSTANCES],
    heat: f64,
    dead: [f64; MAX_SUBSTANCES],
    air: [f64; MAX_SUBSTANCES],
}

impl Books {
    /// The most units of `rx` the gas the cell's share of its air node still offers allows (what the cover took of it
    /// this month counted), as the light reaction is cut (`light`).
    fn room(&self, p: &Parts, rx: &Reaction) -> f64 {
        let outside = rx.inputs.iter().filter(|q| !p.sub.contains(&(q.0 as usize)));
        outside.map(|&(k, n)| (self.air[k as usize] - self.ext[k as usize]).max(0.0) / n as f64).fold(f64::INFINITY, f64::min)
    }
    /// `x` units of `rx` run in slot `m` (its parts by `p`); what is not a part is the cell's to take or give.
    fn run(&mut self, chem: &Chemistry, p: &Parts, m: &mut [f64; NP], rx: &Reaction, x: f64) {
        if x <= 0.0 {
            return;
        }
        let mut one = |k: u16, d: f64| match p.sub.iter().position(|&s| s == k as usize) {
            Some(q) => m[q] += d,
            None => self.ext[k as usize] -= d,
        };
        rx.inputs.iter().for_each(|&(k, n)| one(k, -(n as f64) * x));
        rx.outputs.iter().for_each(|&(k, n)| one(k, n as f64 * x));
        self.heat -= chem.dh_of(rx) * x;
    }
}

/// The units of `rx` per unit of substance `k` among its inputs (none if it takes none).
fn per_input(rx: &Reaction, k: usize) -> Option<f64> {
    rx.inputs.iter().find(|q| q.0 as usize == k).map(|q| q.1 as f64)
}

/// The month of the cover of cell `i` of `d`.
fn cell(w: &World, d: &mut Chunk, p: &Parts, i: usize, dt: f64) {
    let (chem, ph) = (&w.chem, &w.cfg.phys);
    let (c, moves, counts) = d.cover_with(i);
    let (j, area, mm, (t, plateau)) = (w.mesh.up(i, 0, AIR), w.mesh.cells().area()[i], p.mm(chem), w.thaw_last(i));
    moves.open(w, i, Node::new(Class::Cover, i));
    // How wet its ground is (`Physics::wetness`, as bodies feel it): the depth of its key liquid that is liquid, in its
    // pores and standing.
    let liquid = moves.amount(w, Node::ground(i), p.liquid) * w.molten(p.liquid, i, t, plateau);
    let depth = liquid * chem.molar_volume(p.liquid, Phase::Liquid) / area;
    let moist = ph.wetness(depth);
    let reach = (-w.life.env.get(i).map_or(0.0, |e| e[1]) / ph.light_depth).exp();
    // The light since the last step: its capture areas share it by height (their walls per m²), tallest first.
    let dl = (w.light[i] - c.mark).max(0.0);
    c.mark = w.light[i];
    let mut s = [Slot::default(); SLOTS];
    let mut layers = Vec::with_capacity(SLOTS);
    for (q, sq) in s.iter_mut().enumerate() {
        let (built, leaf) = (c.built(&mm, q), c.leaf(&mm, q));
        if c.root[q] == NO_ROOT || built <= EMPTY_KG {
            continue;
        }
        let act = ph.activity(moist / dec(c.genes[q][MOIST]));
        *sq = Slot { built, living: c.living(&mm, q), leaf, abs: leaf * (1.0 - c.reflectance(chem, p, q)), act, ..Default::default() };
        layers.push(((c.walls(&mm, q) / area) as f32, leaf / area, q));
    }
    crate::math::shade(&mut layers, |q, f, bottom| (s[q].shade, s[q].bottom) = (f, bottom));
    let (a_abs, lit): (f64, f64) = (s.iter().map(|x| x.abs).sum(), s.iter().map(|x| x.abs * x.shade).sum());
    let caught = dl * a_abs;
    // The light its bodies' node set aside for it (`Cover::keep`), as far as the node holds it.
    let set_aside = (c.keep as f64 * caught).min(moves.heat(w, Node::bodies(i))).max(0.0);
    let air = {
        let seen = moves.seen(w, j);
        std::array::from_fn(|kk| if kk < chem.ns() && chem.phase(kk, t) == Phase::Gas { seen(kk) } else { f64::INFINITY })
    };
    let mut b = Books { ext: [0.0; MAX_SUBSTANCES], heat: 0.0, dead: [0.0; MAX_SUBSTANCES], air };
    let stored = light(w, (moves, c), p, (i, j), &mut s, caught * reach / lit.max(1e-300), (t, dt, liquid), &mut b);
    c.keep = if caught > 0.0 { (stored / caught) as f32 } else { 0.0 };
    // Its minerals in the ground, shared out slot by slot (the tallest first) as each builds.
    let mut ground: [f64; MAX_SUBSTANCES] = std::array::from_fn(|x| if x < chem.ns() { moves.amount(w, Node::ground(i), x) } else { 0.0 });
    let mut order: Vec<usize> = (0..SLOTS).filter(|&q| s[q].built > 0.0).collect();
    order.sort_by(|&a, &b| s[b].shade.total_cmp(&s[a].shade));
    for x in order {
        let (lost, cost) = upkeep(w, c, p, x, &s[x], (t, dt), &mut b);
        counts.cover_lost.iter_mut().zip(lost).for_each(|(a, l)| *a += l);
        let pays = s[x].gain * s[x].built > cost;
        c.full[x] = !pays;
        build(w, c, p, x, (&s[x], pays), (i, t, dt), &mut ground, &mut b);
        // The fuel it holds past its reserve when it has built what it could goes to its ground, as roots exude the
        // fuel a plant cannot use (Jones, Hodge and Kuzyakov 2004), where the open world's rot takes it.
        let over = (c.mol[x][FUEL] - dec(c.genes[x][STORE]) * c.built(&mm, x) / mm[FUEL]).max(0.0);
        (c.mol[x][FUEL], b.dead[p.sub[FUEL]]) = (c.mol[x][FUEL] - over, b.dead[p.sub[FUEL]] + over);
        // What rounding left below zero is set to zero and counted (a larger overdraw is this law's fault).
        let (size, all) = (c.mol[x].iter().map(|m| m.abs()).sum::<f64>(), c.mol[x]);
        for m in c.mol[x].iter_mut().filter(|m| **m < 0.0) {
            debug_assert!(-*m <= crate::store::ROUNDING * size, "the cover overdrew a part by {m}: {all:?}");
            counts.rounded[0] -= std::mem::take(m);
        }
    }
    merge_alike(c, &mm, &w.cfg.phys);
    book(w, moves, c, p, (i, j, t), set_aside, &b);
}

/// The light reaction of each slot over `dt` at `t` K: what its share of the light caught (`per` of it per unit of
/// absorbing area times shade) stores at `light_yield`, at most its capacity times its activity and the reaction's
/// pace, both times what the air's share of the reaction's gas allows (`Physics::gas_factor`, the least over its gas
/// inputs); all slots cut together to the key liquid in the ground that is liquid (`liquid`, mol) and the gas the cell's
/// share of its air node holds.
/// Each slot's `Slot::gain` is set: what one more kg would store, its light that of the bottom of its layer. Returns the
/// energy stored, J.
#[allow(clippy::too_many_arguments)]
fn light(w: &World, (moves, c): (&Moves, &mut Cover), p: &Parts, (i, j): (usize, usize), s: &mut [Slot; SLOTS], per: f64, (t, dt, liquid): (f64, f64, f64), b: &mut Books) -> f64 {
    let (chem, ph) = (&w.chem, &w.cfg.phys);
    let rx = &chem.reactions[p.light];
    let (dh, pace) = (chem.dh(p.light), rx.arrhenius(t.max(1.0)) / rx.arrhenius(ph.t_cat));
    let seen = moves.seen(w, j);
    let air: f64 = (0..chem.ns()).map(&seen).sum();
    let gas = rx.inputs.iter().filter(|q| chem.phase(q.0 as usize, t) == Phase::Gas).map(|q| ph.gas_factor(seen(q.0 as usize) / air.max(1e-300))).fold(1.0, f64::min);
    let mut x = [0.0; SLOTS];
    for q in 0..SLOTS {
        let need = dec(c.genes[q][MOIST]);
        let cap = dec(c.genes[q][LIGHT]) * need * pace * s[q].act * dt;
        let lit = |f: f64| per * s[q].abs * f * ph.light_yield / dh;
        x[q] = lit(s[q].shade).min(cap * s[q].living).max(0.0) * gas;
        s[q].gain = if s[q].built > 0.0 { (lit(s[q].bottom).min(cap * s[q].living) / s[q].built).max(0.0) * gas * dh } else { 0.0 };
    }
    let all: f64 = x.iter().sum();
    if all <= 0.0 {
        return 0.0;
    }
    // What the cell offers of each input that is not a part: the gas of its share of its air node, the rest from its
    // ground (of the key liquid, what is liquid).
    let offer = |kk: usize| match kk {
        _ if chem.phase(kk, t) == Phase::Gas => b.air[kk],
        _ if kk == p.liquid => liquid,
        _ => moves.amount(w, Node::ground(i), kk),
    };
    let f = rx.inputs.iter().filter(|q| !p.sub.contains(&(q.0 as usize))).map(|&(kk, n)| offer(kk as usize) / (n as f64 * all)).fold(1.0, f64::min);
    for (m, xq) in c.mol.iter_mut().zip(x) {
        b.run(chem, p, m, rx, xq * f);
    }
    all * f * dh
}

/// Slot `x` pays its upkeep over `dt` at `t` K by burning its fuel, then loses the share unpaid (`SPENT`), the share
/// its wear gives (`AGE`) and the share frozen (`PHASE`) to its cell's remains. Returns the kg lost by cause, and the
/// upkeep, J.
fn upkeep(w: &World, c: &mut Cover, p: &Parts, x: usize, s: &Slot, (t, dt): (f64, f64), b: &mut Books) -> ([f64; 6], f64) {
    let (chem, ph, g) = (&w.chem, &w.cfg.phys, c.genes[x]);
    let (burn, days) = (&chem.reactions[p.burn], dt / DAY);
    let pace = burn.arrhenius(t.max(1.0)) / burn.arrhenius(ph.t_cat);
    // A capacity that could store `cap` W per kg costs the share `c_light` of that, the dearer the colder it stands (as
    // a firmer catalyst costs more).
    let cap = dec(g[LIGHT]) * dec(g[MOIST]) * chem.dh(p.light) * DAY;
    let firm = 1.0 + (chem.subs[p.liquid].melt - g[FROST] as f64).max(0.0) / ph.c_firm;
    let active = (s.living * days * (ph.c_basal + ph.c_light * cap * firm) + ph.c_leaf * s.leaf * days) * ph.repair(g[REPAIR] as f64) * pace;
    let cost = active * s.act;
    let (units, fuel) = (cost / -chem.dh(p.burn), per_input(burn, p.sub[FUEL]).unwrap_or(1.0));
    // (What it burns is worked out first: one less a share near one would lose a small fuel to rounding.) It burns no
    // more than its fuel and the gas the cell's share of its air node still offers.
    let burnt = (units * fuel).min(c.mol[x][FUEL].max(0.0)).min(b.room(p, burn) * fuel);
    let short = if units > 0.0 { 1.0 - burnt / (units * fuel) } else { 0.0 };
    b.run(chem, p, &mut c.mol[x], burn, burnt / fuel);
    // Members of a field wear out at the rate of living as a body's do, a dormant one at least at `Physics::dormant` of
    // the active rate (laws.md 6.7); below its frost gene its frozen share dies as far as it is active, within
    // `frost_k` K (laws.md 6.5).
    let worn = active * s.act.max(ph.dormant) / (s.living * ph.e_life * dec(g[REPAIR])).max(1e-300);
    let frozen = if t < g[FROST] as f64 { ((g[FROST] as f64 - t) / ph.frost_k).min(1.0) * s.act } else { 0.0 };
    let mut lost = [0.0; 6];
    let mut left = 1.0;
    for (f, cause) in [(short, SPENT), (crate::math::one_minus_exp(worn), AGE), (frozen, PHASE)] {
        lost[cause as usize] = f * left;
        left *= 1.0 - f;
    }
    let dies = 1.0 - left;
    if dies > 0.0 {
        let mm = p.mm(chem);
        let all = c.kg(&mm, x);
        lost.iter_mut().for_each(|l| *l *= all);
        for q in 0..NP {
            let m = c.mol[x][q].max(0.0) * dies;
            (c.mol[x][q], b.dead[p.sub[q]]) = (c.mol[x][q] - m, b.dead[p.sub[q]] + m);
        }
    }
    (lost, cost)
}

/// Slot `x` builds what its fuel over its reserve (`STORE`) and the minerals of its ground (`ground`, mol, shared out
/// as slots build) allow, by the conversions of growth (`Chemistry::build`, `builds`) at its share of walls (`WALLS`),
/// paying `c_copy` per kg by burning fuel; a fixer (`FIX`) makes the mineral it lacks, at its
/// capacity, paid by the fixing reaction's fuel. The share `SEED` of what it builds leaves as seeds, `spread_m` away in a
/// compass direction drawn, over the cells the area of its cell shifted so far overlaps (`copy::dispersed`, `seed::send`),
/// with a generation's mutation; all of it where one more kg would not pay its upkeep (`pays`
/// false: its light at the bottom of its layer is under its compensation point), so a cover stops thickening where its
/// lowest leaves would not pay for themselves (Monsi and Saeki 1953; Hirose 2005).
#[allow(clippy::too_many_arguments)]
fn build(w: &World, c: &mut Cover, p: &Parts, x: usize, (s, pays): (&Slot, bool), (i, t, dt): (usize, f64, f64), ground: &mut [f64; MAX_SUBSTANCES], b: &mut Books) {
    let (chem, ph, g) = (&w.chem, &w.cfg.phys, c.genes[x]);
    let (Some(bl), mm) = (chem.build.as_ref(), p.mm(chem)) else { return };
    let Some(wall) = chem.builds.iter().find(|r| r.outputs[0].0 as usize == p.sub[WALL]) else { return };
    // Per kg built: the units of each conversion, and what they take.
    let fm = 1.0 / (1.0 + dec(g[WALLS]));
    let units = [(bl, fm / mm[MATTER] / bl.outputs[0].1 as f64), (wall, dec(g[WALLS]) * fm / mm[WALL] / wall.outputs[0].1 as f64)];
    let mut need = [0.0; MAX_SUBSTANCES];
    units.iter().for_each(|(rx, u)| rx.inputs.iter().for_each(|&(k, n)| need[k as usize] += n as f64 * u));
    let burn = &chem.reactions[p.burn];
    let (fuel_burn, burn_e) = (per_input(burn, p.sub[FUEL]).unwrap_or(1.0), -chem.dh(p.burn));
    let fpk = need[p.sub[FUEL]] + ph.c_copy / burn_e * fuel_burn;
    let surplus = (c.mol[x][FUEL] - dec(g[STORE]) * s.built / mm[FUEL]).max(0.0);
    // The most each limit allows, kg: the fuel, and each mineral from the ground; a fixer then makes what the mineral it
    // fixes lacks, its fuel counted.
    let minerals = || (0..chem.ns()).filter(|&k| need[k] > 0.0 && k != p.sub[FUEL]);
    let by = |k: usize| ground[k] / need[k];
    // (Fixing takes its gas from the cell's share of the air node, as far as that still offers it.)
    let fixed = p.fix.filter(|_| g[FIX] > FIX_OFF).and_then(|r| {
        let rx = &chem.reactions[r];
        let &(m, out) = rx.outputs.iter().find(|o| need[o.0 as usize] > 0.0)?;
        let most = (dec(g[FIX]) * s.living * rx.arrhenius(t.max(1.0)) / rx.arrhenius(ph.t_cat) * s.act * dt).min(b.room(p, rx));
        Some((rx, m as usize, out as f64, per_input(rx, p.sub[FUEL])?, most))
    });
    let others = minerals().filter(|&k| fixed.is_none_or(|f| f.1 != k)).map(by).fold(surplus / fpk, f64::min);
    let mut kg = minerals().map(by).fold(others, f64::min);
    let mut y = 0.0;
    if let Some((rx, m, out, fuel, most)) = fixed {
        let r = fuel / out;
        let with = others.min((surplus + r * ground[m]) / (fpk + r * need[m])).min((ground[m] + out * most) / need[m]);
        if with > kg {
            (kg, y) = (with, (with * need[m] - ground[m]).max(0.0) / out);
        }
        b.run(chem, p, &mut c.mol[x], rx, y);
    }
    // Building's work burns fuel with the gas the cell's share of its air node still offers.
    kg = kg.min(b.room(p, burn) * burn_e / ph.c_copy.max(1e-300));
    if kg <= 0.0 {
        return;
    }
    minerals().for_each(|k| ground[k] = (ground[k] - kg * need[k]).max(0.0));
    if let Some((_, m, out, ..)) = fixed {
        ground[m] += y * out - (y * out).min(kg * need[m]);
    }
    b.run(chem, p, &mut c.mol[x], burn, kg * ph.c_copy / burn_e);
    // What it builds goes into the slot; the share `SEED` of it leaves as seeds.
    let before = c.mol[x];
    units.iter().for_each(|(rx, u)| b.run(chem, p, &mut c.mol[x], rx, kg * u));
    let seed = if pays { g[SEED].clamp(0.0, 1.0) as f64 } else { 1.0 };
    if seed > 0.0 {
        let mut mol = [0.0; NP];
        for q in [MATTER, WALL] {
            mol[q] = (c.mol[x][q] - before[q]).max(0.0) * seed;
            c.mol[x][q] -= mol[q];
        }
        let heat = (0..NP).map(|q| mol[q] * chem.subs[p.sub[q]].cp * t).sum::<f64>();
        b.heat -= heat;
        let mut r = Rng::new(w.seed, w.month(), (i * SLOTS + x) as u64, Purpose::Cover);
        let mut lands: Vec<(usize, f64)> = Vec::new();
        crate::copy::dispersed(w, i, &mut r, |j, share| lands.push((j, share)));
        let genes = super::mutate(&g, ph.cover_mut, &mut r);
        super::seed::send(c, x, (mol, heat), genes, (i, lands));
    }
}


/// Alike kinds of a cell (`APART` × `Physics::alike` steps in every gene, `distance`) share a slot: the lighter joins
/// the heavier, the genes the mean by built cover, its starting kind the heavier's.
pub(super) fn merge_alike(c: &mut Cover, mm: &[f64; NP], ph: &crate::Physics) {
    for a in 0..SLOTS {
        for z in a + 1..SLOTS {
            if c.root[a] == NO_ROOT || c.root[z] == NO_ROOT || distance(&c.genes[a], &c.genes[z]) > APART * ph.alike {
                continue;
            }
            let (big, small) = if c.built(mm, z) > c.built(mm, a) { (z, a) } else { (a, z) };
            join(c, mm, big, small);
        }
    }
}

/// Slot `small` joins slot `big` of the same cell: amounts added, genes the mean by built cover; `small` is empty after.
pub(super) fn join(c: &mut Cover, mm: &[f64; NP], big: usize, small: usize) {
    let (wb, ws) = (c.built(mm, big), c.built(mm, small));
    let f = (ws / (wb + ws).max(1e-300)) as f32;
    let (gs, ms) = (c.genes[small], c.mol[small]);
    (0..NG).for_each(|g| c.genes[big][g] += f * (gs[g] - c.genes[big][g]));
    (0..NP).for_each(|q| c.mol[big][q] += ms[q]);
    (c.mol[small], c.root[small], c.full[small]) = ([0.0; NP], NO_ROOT, false);
}

/// Book what the cover of cell `i` (air node `j`, ground at `t` K) took and gave in its month (`Books`): the
/// light set aside for it from its bodies' node; gases from and to the air (taken from the air node with their heat at
/// its temperature and their latent heat, given into the bodies' node with theirs, as bodies breathe); liquids and
/// minerals from and to its ground, with their heat; its remains to its ground. Its slots left with no built cover go
/// to the ground too. Last its heat comes to the ground's temperature with its ground.
fn book(w: &World, moves: &mut Moves, c: &mut Cover, p: &Parts, (i, j, t): (usize, usize, f64), set_aside: f64, b: &Books) {
    let chem = &w.chem;
    let mm = p.mm(chem);
    let (mut e, lift, ta, ground) = (c.e + set_aside + b.heat, w.lift(i), moves.t_air(w, j), Node::ground(i));
    moves.take(w, Node::bodies(i), 0, 0.0, set_aside);
    let mut dead = b.dead;
    for x in 0..SLOTS {
        if c.root[x] != NO_ROOT && c.built(&mm, x) <= EMPTY_KG {
            (0..NP).for_each(|q| dead[p.sub[q]] += c.mol[x][q]);
            (c.mol[x], c.root[x], c.full[x]) = ([0.0; NP], NO_ROOT, false);
        }
    }
    let cap = moves.heat_cap_ground(w, i);
    let held = |kk: usize, m: f64| m * (chem.subs[kk].cp * t + if chem.phase(kk, t) == Phase::Solid { 0.0 } else { chem.subs[kk].lf });
    for (kk, (&ext, &died)) in b.ext.iter().zip(&dead).enumerate().take(chem.ns()) {
        let (m, s) = (ext - died, &chem.subs[kk]);
        if m == 0.0 {
            continue;
        }
        if chem.phase(kk, t) == Phase::Gas {
            moves.gas(w, kk, -m, lift);
            if m > 0.0 {
                moves.take(w, Node::air(j), 0, 0.0, m * s.cp * ta);
                e += m * (s.cp * ta + s.latent_gas());
            } else {
                moves.give(w, Node::bodies(i), 0, 0.0, -m * s.cp * t);
                e += m * (s.cp * t + s.latent_gas());
            }
        } else if m > 0.0 {
            moves.take(w, ground, kk, m, 0.0);
            e += moves.take_from_ground(w, i, (0, 0.0, 0.0), held(kk, m), cap).1;
        } else {
            moves.give(w, ground, kk, -m, held(kk, -m));
            e -= held(kk, -m);
        }
    }
    let target = c.cap(chem, p) * t;
    if target > e {
        e += moves.take_from_ground(w, i, (0, 0.0, 0.0), target - e, cap).1;
    } else {
        moves.give(w, ground, 0, 0.0, e - target);
        e = target;
    }
    c.e = e;
}
