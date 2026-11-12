//! A month of the cover of each cell of a chunk (laws.md 4.5): it catches light and runs its light reaction, pays its
//! upkeep by burning its fuel, loses what is unpaid, worn out or frozen, builds from its fuel and the minerals of the
//! ground (fixing what it lacks if it can), and sends seeds out. Everything it takes and gives is booked through the
//! chunk's moves, with its heat; its own side it books itself.

use super::build::per_input;
use super::{dec, Cover, Parts, EMPTY_KG, FROST, FUEL, LIGHT, MOIST, NO_ROOT, NP, REPAIR, SLOTS, STORE, WALL};
use crate::body::{AGE, PHASE, SPENT};
use crate::chem::{Chemistry, Phase, Reaction};
use crate::store::{Class, Moves, Node};
use crate::units::{DAY, MAX_SUBSTANCES};
use crate::{life::Chunk, Counters, World, AIR};

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
pub(super) struct Slot {
    pub(super) built: f64,
    pub(super) living: f64,
    leaf: f64,
    abs: f64,
    shade: f64,
    bottom: f64,
    pub(super) act: f64,
    gain: f64,
}

/// What a cell's cover changes outside itself in a month: per substance the mol it takes in (+) or gives out (-), the
/// heat its reactions release, J, and per substance its remains, mol; and per gas what the cell's share of its air node
/// offers, mol (infinite for what is not a gas: the ground's inputs are limited where they are read); and the key liquid
/// its leaves lost as vapour, mol (taken from its ground with the rest of `ext`, given to the air).
pub struct Books {
    pub ext: [f64; MAX_SUBSTANCES],
    pub heat: f64,
    pub dead: [f64; MAX_SUBSTANCES],
    air: [f64; MAX_SUBSTANCES],
    pub vapour: f64,
}

/// What the cover of a cell reads of its world in a month: the full world's (`around`) or a year a nursery keeps
/// (`worldgen::life::nursery`). Its ground's temperature, K; of the key liquid in its ground, what is liquid, mol, and
/// how wet that leaves its ground (`wet`); the share of the light that reaches it under standing liquid; the cell's light
/// (`World::light`), J/m²; the air's factor for the light reaction (`Physics::gas_factor`, the least over its gas
/// inputs); the key liquid its leaves lose as vapour per mol of gas they take in (`thirst`); per substance what the
/// cell's share of its air node offers as a gas, mol (infinite for what is not a gas), and what its ground holds, mol;
/// and the tick that keys its seeds' draws (`World::month`).
#[derive(Clone, Debug)]
pub struct Around {
    pub t: f64,
    pub liquid: f64,
    pub moist: f64,
    pub reach: f64,
    pub light: f64,
    pub gas: f64,
    pub thirst: f64,
    pub air: [f64; MAX_SUBSTANCES],
    pub ground: [f64; MAX_SUBSTANCES],
    pub tick: u64,
}

/// What the cover of cell `i` reads of the world now, through the moves `moves` opened on it.
pub(crate) fn around(w: &World, moves: &Moves, p: &Parts, i: usize) -> Around {
    let (chem, ph) = (&w.chem, &w.cfg.phys);
    let (j, t) = (w.mesh.up(i, 0, AIR), w.thaw_last(i).0);
    let (liquid, moist) = super::wet(w, p, i, moves.amount(w, Node::ground(i), p.liquid));
    let reach = (-w.life.env.get(i).map_or(0.0, |e| e[1]) / ph.light_depth).exp();
    let seen = moves.seen(w, j);
    let all: f64 = (0..chem.ns()).map(&seen).sum();
    let rx = &chem.reactions[p.light];
    let gas = rx.inputs.iter().filter(|q| chem.phase(q.0 as usize, t) == Phase::Gas).map(|q| ph.gas_factor(seen(q.0 as usize) / all.max(1e-300))).fold(1.0, f64::min);
    // The surface pressure: the weight of the cell's share of its air column per m² of the cell.
    let column: f64 = (0..chem.ns()).map(|k| moves.amount(w, Node::air(j), k) * chem.subs[k].molar_mass).sum();
    let pressure = column * w.cfg.gravity / w.mesh.cells().area()[i];
    let thirst = rx.inputs.iter().find(|q| chem.phase(q.0 as usize, t) == Phase::Gas).map_or(0.0, |q| {
        thirst(chem, ph, (p.liquid, q.0 as usize), (t, pressure), [seen(p.liquid), seen(q.0 as usize)].map(|x| x / all.max(1e-300)))
    });
    let air = std::array::from_fn(|kk| if kk < chem.ns() && chem.phase(kk, t) == Phase::Gas { seen(kk) } else { f64::INFINITY });
    let ground = std::array::from_fn(|x| if x < chem.ns() { moves.amount(w, Node::ground(i), x) } else { 0.0 });
    Around { t, liquid, moist, reach, light: w.light[i], gas, thirst, air, ground, tick: w.month() }
}

/// The key liquid `liquid` a leaf at `t` K under air at `pressure` Pa loses as vapour per mol of the gas `gas` it takes
/// in, mol, where the air holds the mole fractions `[x_liquid, x_gas]` of them: its pores pass both, the vapour out
/// down its deficit (saturation at the leaf, `Chemistry::p_sat`, less the air's), the gas in down its drawdown inside
/// the leaf (`Physics::drawdown` of the air's), each at its diffusivity, whose ratio is the square root of the ratio of
/// their molar masses (Graham's law): the transpiration ratio of Cowan and Farquhar 1977 and Farquhar and Sharkey 1982.
pub fn thirst(chem: &Chemistry, ph: &crate::Physics, (liquid, gas): (usize, usize), (t, pressure): (f64, f64), [x_liquid, x_gas]: [f64; 2]) -> f64 {
    let deficit = (chem.p_sat(liquid, t) / pressure.max(1e-300) - x_liquid).max(0.0);
    let diffusivity = (chem.subs[gas].molar_mass / chem.subs[liquid].molar_mass).sqrt();
    diffusivity * deficit / (ph.drawdown * x_gas).max(1e-300)
}

impl Books {
    /// The most units of `rx` the gas the cell's share of its air node still offers allows (what the cover took of it
    /// this month counted), as the light reaction is cut (`light`).
    pub(super) fn room(&self, p: &Parts, rx: &Reaction) -> f64 {
        let outside = rx.inputs.iter().filter(|q| !p.sub.contains(&(q.0 as usize)));
        outside.map(|&(k, n)| (self.air[k as usize] - self.ext[k as usize]).max(0.0) / n as f64).fold(f64::INFINITY, f64::min)
    }
    /// `x` units of `rx` run in slot `m` (its parts by `p`); what is not a part is the cell's to take or give.
    pub(super) fn run(&mut self, chem: &Chemistry, p: &Parts, m: &mut [f64; NP], rx: &Reaction, x: f64) {
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

/// The month of the cover of cell `i` of `d`: what it reads of the world (`around`), its month (`live`), and its
/// books (`book`).
fn cell(w: &World, d: &mut Chunk, p: &Parts, i: usize, dt: f64) {
    let (c, moves, counts) = d.cover_with(i);
    moves.open(w, i, Node::new(Class::Cover, i));
    let a = around(w, moves, p, i);
    let keep = c.keep;
    let (b, caught) = live(w, c, p, &a, (i, dt), counts);
    // The light its bodies' node set aside for it (`Cover::keep`), as far as the node holds it.
    let set_aside = (keep as f64 * caught).min(moves.heat(w, Node::bodies(i))).max(0.0);
    book(w, moves, c, p, (i, w.mesh.up(i, 0, AIR), a.t), set_aside, &b);
}

/// The month over `dt` s of the cover `c` of cell `i` (its place on the map, where its seeds go from) that reads `a`:
/// it catches light, runs its light reaction, pays its upkeep, builds and sends seeds (into `Cover::seeds`); alike
/// slots merge, and slots left with no built cover empty into its remains. Returns what it changed outside itself
/// (`Books`) and the light it caught, J; what it lost and rounded is counted in `counts`.
pub fn live(w: &World, c: &mut Cover, p: &Parts, a: &Around, (i, dt): (usize, f64), counts: &mut Counters) -> (Books, f64) {
    let (chem, ph) = (&w.chem, &w.cfg.phys);
    let (area, mm, t, rho) = (w.mesh.cells().area()[i], p.mm(chem), a.t, chem.subs[p.sub[WALL]].density[0]);
    // The light since the last step: its capture areas share it by the height of their plants, tallest first, each on
    // the share of the cell its roots hold.
    let dl = (a.light - c.mark).max(0.0);
    c.mark = a.light;
    let mut s = [Slot::default(); SLOTS];
    let mut layers = Vec::with_capacity(SLOTS);
    for (q, sq) in s.iter_mut().enumerate() {
        let (built, leaf) = (c.built(&mm, q), c.leaf(&mm, q));
        if c.root[q] == NO_ROOT || built <= EMPTY_KG {
            continue;
        }
        let act = super::activity(ph, &c.genes[q], a.moist);
        *sq = Slot { built, living: c.living(&mm, q), leaf, abs: leaf * (1.0 - c.reflectance(chem, p, q)), act, ..Default::default() };
        // The share of the cell its plants hold: where its loose ground is as deep as their roots need (found again
        // only when their need has moved by more than a bin of depth).
        let h = c.height(ph, &mm, rho, q);
        let need = super::stand::need(ph, h);
        if super::stand::moved(c.need[q], need) {
            c.rooted[q] = w.loose_bins.get(i).map_or(1.0, |b| super::stand::rooted_share(ph, b, need)) as f32;
            c.need[q] = need as f32;
        }
        layers.push((h as f32, leaf / area, c.rooted[q] as f64, q));
    }
    crate::math::shade(&mut layers, |q, f, bottom| (s[q].shade, s[q].bottom) = (f, bottom));
    let (a_abs, lit): (f64, f64) = (s.iter().map(|x| x.abs).sum(), s.iter().map(|x| x.abs * x.shade).sum());
    let caught = dl * a_abs;
    let mut b = Books { ext: [0.0; MAX_SUBSTANCES], heat: 0.0, dead: [0.0; MAX_SUBSTANCES], air: a.air, vapour: 0.0 };
    let stored = light(chem, ph, c, p, &mut s, (caught * a.reach / lit.max(1e-300), caught * a.reach, a, dt), &mut b);
    let all: f64 = stored.iter().sum();
    c.keep = if caught > 0.0 { (all / caught) as f32 } else { 0.0 };
    let year = w.cfg.months_per_year as f32;
    // Its minerals in the ground, shared out slot by slot (the tallest first) as each builds.
    let mut ground = a.ground;
    let mut order: Vec<usize> = (0..SLOTS).filter(|&q| s[q].built > 0.0).collect();
    order.sort_by(|&a, &b| s[b].shade.total_cmp(&s[a].shade));
    for x in order {
        let (lost, cost) = upkeep(w, c, p, x, &s[x], (t, dt), &mut b);
        counts.cover_lost.iter_mut().zip(lost).for_each(|(a, l)| *a += l);
        let pays = s[x].gain * s[x].built > cost;
        c.full[x] = !pays;
        // Its balance over about a year: what it stored less its upkeep, per kg.
        c.net[x] += (((stored[x] - cost) / s[x].built) as f32 - c.net[x]) / year;
        // Where one more kg would not pay it builds living cover no more, but walls alone while its plants lack them.
        if pays || super::build::frame_short(w, c, p, x) <= 0.0 {
            super::build::build(w, c, p, x, (&s[x], pays), (i, a.tick, t, dt), &mut ground, &mut b);
        }
        super::build::frame(w, c, p, x, &mut b);
        super::stand::thin(c, &mm, ph, area, x);
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
    super::stand::merge_alike(c, &mm, ph, area);
    for x in 0..SLOTS {
        if c.root[x] != NO_ROOT && c.built(&mm, x) <= EMPTY_KG {
            (0..NP).for_each(|q| b.dead[p.sub[q]] += c.mol[x][q]);
            (c.mol[x], c.root[x], c.full[x], c.net[x], c.plants[x], c.need[x]) = ([0.0; NP], NO_ROOT, false, 0.0, 0.0, 0.0);
        }
    }
    (b, caught)
}

/// The light reaction of each slot over `dt` at the ground's temperature of `a`: what its share of the light caught
/// (`per` of it per unit of absorbing area times shade) stores at `light_yield`, at most its capacity times its
/// activity and the reaction's pace, both times what the air's share of the reaction's gas allows (`Around::gas`); all
/// slots cut together to the key liquid in the ground that is liquid (`Around::liquid`, mol) and the gas the cell's
/// share of its air node holds.
/// Each slot's `Slot::gain` is set: what one more kg would store, its light that of the bottom of its layer. Returns the
/// energy each slot stored, J.
fn light(chem: &Chemistry, ph: &crate::Physics, c: &mut Cover, p: &Parts, s: &mut [Slot; SLOTS], (per, caught, a, dt): (f64, f64, &Around, f64), b: &mut Books) -> [f64; SLOTS] {
    let (t, liquid, gas) = (a.t, a.liquid, a.gas);
    let rx = &chem.reactions[p.light];
    let (dh, pace) = (chem.dh(p.light), rx.arrhenius(t.max(1.0)) / rx.arrhenius(ph.t_cat));
    let mut x = [0.0; SLOTS];
    for q in 0..SLOTS {
        let need = dec(c.genes[q][MOIST]);
        let cap = dec(c.genes[q][LIGHT]) * need * pace * s[q].act * dt;
        let lit = |f: f64| per * s[q].abs * f * ph.light_yield / dh;
        x[q] = lit(s[q].shade).min(cap * s[q].living).max(0.0) * gas;
        s[q].gain = if s[q].built > 0.0 { (lit(s[q].bottom).min(cap * s[q].living) / s[q].built).max(0.0) * gas * dh } else { 0.0 };
    }
    // Per unit of the reaction its leaves lose the key liquid as vapour, `thirst` per mol of gas taken in; each slot
    // draws it from the pores its roots reach (`Cover::rooted`).
    let gas_in: f64 = rx.inputs.iter().filter(|q| chem.phase(q.0 as usize, t) == Phase::Gas).map(|q| q.1 as f64).sum();
    let lost = a.thirst * gas_in;
    let drink = per_input(rx, p.liquid).unwrap_or(0.0) + lost;
    for (q, xq) in x.iter_mut().enumerate() {
        *xq = xq.min(liquid * c.rooted[q] as f64 / drink.max(1e-300));
    }
    let all: f64 = x.iter().sum();
    if all <= 0.0 {
        return [0.0; SLOTS];
    }
    // What the cell offers of each input that is not a part: the gas of its share of its air node, the rest from its
    // ground (of the key liquid, what is liquid, of which its leaves also lose `lost` a unit as vapour: where the
    // ground cannot give that, or the light its leaves absorbed cannot raise it, `caught` J, the pores close and all
    // slots store less together: a canopy evaporates no more than the radiation it takes in).
    let offer = |kk: usize| match kk {
        _ if chem.phase(kk, t) == Phase::Gas => b.air[kk],
        _ if kk == p.liquid => liquid,
        _ => a.ground[kk],
    };
    let take = |kk: usize, n: f64| n + if kk == p.liquid { lost } else { 0.0 };
    let raise = lost * chem.subs[p.liquid].lv;
    let f = rx.inputs.iter().filter(|q| !p.sub.contains(&(q.0 as usize))).map(|&(kk, n)| offer(kk as usize) / (take(kk as usize, n as f64) * all)).fold(caught / (raise * all).max(1e-300), f64::min).min(1.0);
    for (m, xq) in c.mol.iter_mut().zip(x) {
        b.run(chem, p, m, rx, xq * f);
    }
    b.vapour += all * f * lost;
    b.ext[p.liquid] += all * f * lost;
    x.map(|xq| xq * f * dh)
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
    // the active rate (laws.md 6.7); below its frost gene, hardened by its dormancy, its frozen share dies (laws.md
    // 6.5).
    let worn = active * s.act.max(ph.dormant) / (s.living * ph.e_life * dec(g[REPAIR])).max(1e-300);
    let frozen = ph.frozen(g[FROST] as f64, t, s.act);
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
        // What dies are whole plants: each one left keeps its mass.
        c.plants[x] *= left;
        for q in 0..NP {
            let m = c.mol[x][q].max(0.0) * dies;
            (c.mol[x][q], b.dead[p.sub[q]]) = (c.mol[x][q] - m, b.dead[p.sub[q]] + m);
        }
    }
    (lost, cost)
}

/// Book what the cover of cell `i` (air node `j`, ground at `t` K) took and gave in its month (`Books`): the
/// light set aside for it from its bodies' node; gases from and to the air (taken from the air node with their heat at
/// its temperature and their latent heat, given into the bodies' node with theirs, as bodies breathe); liquids and
/// minerals from and to its ground, with their heat; its remains to its ground. Last its heat comes to the ground's
/// temperature with its ground.
fn book(w: &World, moves: &mut Moves, c: &mut Cover, p: &Parts, (i, j, t): (usize, usize, f64), set_aside: f64, b: &Books) {
    let chem = &w.chem;
    let (mut e, lift, ta, ground) = (c.e + set_aside + b.heat, w.lift(i), moves.t_air(w, j), Node::ground(i));
    moves.take(w, Node::bodies(i), 0, 0.0, set_aside);
    let cap = moves.heat_cap_ground(w, i);
    let held = |kk: usize, m: f64| m * (chem.subs[kk].cp * t + if chem.phase(kk, t) == Phase::Solid { 0.0 } else { chem.subs[kk].lf });
    for (kk, (&ext, &died)) in b.ext.iter().zip(&b.dead).enumerate().take(chem.ns()) {
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
    // The vapour its leaves lost goes to its bodies' node with its heat and latent heat, as a body's sweat does
    // (`Moves::gas`); the liquid it came from was taken from its ground above, and its ground pays the heat that
    // raises it.
    if b.vapour > 0.0 {
        let s = &chem.subs[p.liquid];
        e += moves.take_from_ground(w, i, (0, 0.0, 0.0), b.vapour * s.lv, cap).1;
        moves.gas(w, p.liquid, b.vapour, lift);
        moves.give(w, Node::bodies(i), 0, 0.0, b.vapour * s.cp * t);
        e -= b.vapour * (s.cp * t + s.latent_gas());
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
