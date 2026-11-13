//! A month of the cover of each cell of a chunk (laws.md 3): it catches light and runs its light reaction, pays its
//! upkeep by burning its fuel, loses what is unpaid, worn out or frozen, builds from its fuel and the minerals of the
//! ground (fixing what it lacks if it can), and sends seeds out. Everything it takes and gives is booked through the
//! chunk's moves, with its heat; its own side it books itself.

use super::around::{around, Around};
use super::budget::{self, BUDGET, BUILDING, DIED, EXUDED, ROUNDED, SEEDS, SHED, STORED, UPKEEP, UPKEEP_PARTS};
use super::build::per_input;
use super::{dec, Cover, Parts, EMPTY_KG, FROST, FUEL, LIGHT, MATTER, MOIST, NO_ROOT, NP, REPAIR, SLOTS, STORE, WALL};
use crate::life::{AGE, PHASE, SPENT};
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
            (c.mark, c.keep, c.film, c.reach) = (w.light[i], 0.0, 0.0, 0.0);
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
    sapwood: f64,
    drunk: f64,
}

/// What a cell's cover changes outside itself in a month: per substance the mol it takes in (+) or gives out (-), the
/// heat its reactions release, J, and per substance its remains, mol; per gas what the cell's share of its air node
/// offers, mol (infinite for what is not a gas: the ground's inputs are limited where they are read); and its budget's
/// flows, J (`budget::BUDGET`).
pub struct Books {
    pub ext: [f64; MAX_SUBSTANCES],
    pub heat: f64,
    pub dead: [f64; MAX_SUBSTANCES],
    air: [f64; MAX_SUBSTANCES],
    pub(super) budget: [f64; BUDGET.len()],
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
    // The light its aside node set aside for it (`Cover::keep`), as far as the node holds it.
    let set_aside = (keep as f64 * caught).min(moves.heat(w, Node::aside(i))).max(0.0);
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
        let sapwood = super::stand::sapwood(ph, (leaf, c.height(ph, &mm, rho, q)), c.walls(&mm, q), rho);
        *sq = Slot { built, living: c.living(&mm, q), leaf, abs: leaf * (1.0 - c.reflectance(chem, p, q)), act, sapwood, ..Default::default() };
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
    let mut b = Books { ext: [0.0; MAX_SUBSTANCES], heat: 0.0, dead: [0.0; MAX_SUBSTANCES], air: a.air, budget: [0.0; BUDGET.len()] };
    let e = p.content(chem);
    let before = c.content(&e);
    let stored = light(chem, ph, c, p, &mut s, (caught * a.reach / lit.max(1e-300), area, a, dt), &mut b);
    b.budget[STORED] += c.content(&e) - before;
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
        if !pays {
            let shed = shed(c, x, (&s[x], cost), area * c.rooted[x].max(f32::MIN_POSITIVE) as f64);
            counts.cover_lost[SPENT as usize] += shed * mm[MATTER];
            b.dead[p.sub[MATTER]] += shed;
            b.budget[SHED] -= shed * e[MATTER];
        }
        // Its balance over about a year: what it stored less its upkeep, per kg.
        c.net[x] += (((stored[x] - cost) / s[x].built) as f32 - c.net[x]) / year;
        // Where one more kg would not pay it builds living cover no more, but walls alone while its plants lack them.
        let (held, sent) = (budget::held(&e, &c.mol[x]), b.budget[SEEDS]);
        if pays || super::build::frame_short(w, c, p, x) <= 0.0 {
            super::build::build(w, c, p, x, (&s[x], pays), (i, a.tick, t, dt), &mut ground, &mut b);
        }
        super::build::frame(w, c, p, x, &mut b);
        b.budget[BUILDING] += budget::held(&e, &c.mol[x]) - held - (b.budget[SEEDS] - sent);
        super::stand::thin(c, &mm, ph, area, x);
        // The fuel it holds past its reserve when it has built what it could goes to its ground, as roots exude the
        // fuel a plant cannot use (Jones, Hodge and Kuzyakov 2004), where the open world's rot takes it.
        let over = (c.mol[x][FUEL] - dec(c.genes[x][STORE]) * c.built(&mm, x) / mm[FUEL]).max(0.0);
        (c.mol[x][FUEL], b.dead[p.sub[FUEL]]) = (c.mol[x][FUEL] - over, b.dead[p.sub[FUEL]] + over);
        b.budget[EXUDED] -= over * e[FUEL];
        // What rounding left below zero is set to zero and counted (a larger overdraw is this law's fault).
        let (size, all) = (c.mol[x].iter().map(|m| m.abs()).sum::<f64>(), c.mol[x]);
        for (q, m) in c.mol[x].iter_mut().enumerate().filter(|(_, m)| **m < 0.0) {
            debug_assert!(-*m <= crate::store::ROUNDING * size, "the cover overdrew a part by {m}: {all:?}");
            b.budget[ROUNDED] -= *m * e[q];
            counts.rounded[0] -= std::mem::take(m);
        }
    }
    super::stand::merge_alike(c, &mm, ph, area);
    for x in 0..SLOTS {
        if c.root[x] != NO_ROOT && c.built(&mm, x) <= EMPTY_KG {
            (0..NP).for_each(|q| b.dead[p.sub[q]] += c.mol[x][q]);
            b.budget[DIED] -= budget::held(&e, &c.mol[x]);
            (c.mol[x], c.root[x], c.full[x], c.net[x], c.plants[x], c.need[x]) = ([0.0; NP], NO_ROOT, false, 0.0, 0.0, 0.0);
        }
    }
    counts.cover_budget.iter_mut().zip(b.budget).for_each(|(a, f)| *a += f);
    (b, caught)
}

/// The light reaction of each slot over `dt` at the ground's temperature of `a`: what its share of the light caught
/// (`per` of it per unit of absorbing area times shade) stores at `light_yield`, at most its capacity times its
/// activity and the reaction's pace, both times what the air's share of the reaction's gas allows (`Around::gas`); all
/// slots cut together to the key liquid in the ground that is liquid (`Around::liquid`, mol) and the gas the cell's
/// share of its air node holds.
/// Each slot's `Slot::gain` is set: what one more kg would store, its light that of the bottom of its layer. Returns the
/// energy each slot stored, J.
fn light(chem: &Chemistry, ph: &crate::Physics, c: &mut Cover, p: &Parts, s: &mut [Slot; SLOTS], (per, area, a, dt): (f64, f64, &Around, f64), b: &mut Books) -> [f64; SLOTS] {
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
    // Per unit of the reaction its leaves would lose the key liquid as vapour, `thirst` per mol of gas taken in; each
    // slot draws it from the pores its roots reach (`Cover::rooted`), and opens its pores no wider than that pays for.
    let gas_in: f64 = rx.inputs.iter().filter(|q| chem.phase(q.0 as usize, t) == Phase::Gas).map(|q| q.1 as f64).sum();
    let lost = a.thirst * gas_in;
    let drink = per_input(rx, p.liquid).unwrap_or(0.0) + lost;
    for (q, xq) in x.iter_mut().enumerate() {
        *xq = xq.min(liquid * c.rooted[q] as f64 / drink.max(1e-300));
    }
    let all: f64 = x.iter().sum();
    (c.film, c.reach) = (0.0, 0.0);
    if all <= 0.0 {
        return [0.0; SLOTS];
    }
    // What the cell offers of each input that is not a part: the gas of its share of its air node, the rest from its
    // ground (of the key liquid, what is liquid, of which its leaves would also lose `lost` a unit as vapour: where the
    // ground cannot give that, the pores close and all slots store less together).
    let offer = |kk: usize| match kk {
        _ if chem.phase(kk, t) == Phase::Gas => b.air[kk],
        _ if kk == p.liquid => liquid,
        _ => a.ground[kk],
    };
    let take = |kk: usize, n: f64| n + if kk == p.liquid { lost } else { 0.0 };
    let f = rx.inputs.iter().filter(|q| !p.sub.contains(&(q.0 as usize))).map(|&(kk, n)| offer(kk as usize) / (take(kk as usize, n as f64) * all)).fold(1.0, f64::min);
    for ((m, sq), xq) in c.mol.iter_mut().zip(s.iter_mut()).zip(x) {
        b.run(chem, p, m, rx, xq * f);
        sq.drunk = xq * f * drink;
    }
    // Its pores as open as that leaves them: the film the key liquid's vapour leaves its leaves through over the month,
    // per m² of the cell, which the ground's evaporation passes it through day by day (`World::second_film`), from the
    // share of the ground's liquid its roots reach, by each slot's gas taken in.
    let taken = all * f * gas_in / dt;
    c.film = (a.pores * taken / area) as f32;
    c.reach = (x.iter().zip(&c.rooted).map(|(xq, &r)| xq * r as f64).sum::<f64>() / all) as f32;
    x.map(|xq| xq * f * dh)
}

/// Slot `x` of cover `c`, whose lowest leaves do not pay for themselves (`Cover::full`), sheds the living cover of the
/// layers of its canopy that do not, as a canopy drops its leaves below its compensation point (Monsi and Saeki 1953;
/// Hirose 2005): its light falls as e^-z with the capture area z above a leaf in its patch of `patch` m², so what one
/// more kg would store at depth z is its gain at its bottom (`Slot::gain`) times e^(y - z), y its whole depth, and the
/// layers deeper than where that falls to its upkeep a kg, `cost` J over its built cover, pay nothing. Returns the mol of
/// body matter shed (to its remains); its walls and its plants stay.
fn shed(c: &mut Cover, x: usize, (s, cost): (&Slot, f64), patch: f64) -> f64 {
    let (y, per_kg) = (s.leaf / patch, cost / s.built.max(1e-300));
    if s.gain <= 0.0 || y <= 0.0 || per_kg <= 0.0 {
        return 0.0;
    }
    let keep = ((y + (s.gain / per_kg).ln()) / y).clamp(0.0, 1.0);
    let m = c.mol[x][MATTER].max(0.0) * (1.0 - keep);
    c.mol[x][MATTER] -= m;
    m
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
    let k = ph.repair(g[REPAIR] as f64) * pace;
    let living = [ph.c_basal, ph.c_light * cap, ph.c_light * cap * (firm - 1.0)].map(|x| s.living * days * x * k);
    let active = living.iter().sum::<f64>() + ph.c_leaf * s.leaf * days * k;
    // Its conducting walls breathe whatever the season gives (`stand::sapwood`), at the burning's pace. Its fine roots,
    // as many as the liquid it drew this month needs (`root_uptake`; the minerals come in that liquid by mass flow,
    // Barber 1995), breathe as living cover and are built anew `root_turnover` times a year (Gill and Jackson 2000).
    let roots = s.drunk / dt / ph.root_uptake;
    let renew = roots * ph.root_turnover * super::build::living_cost(chem, ph, p) / w.cfg.months_per_year as f64;
    let parts: [f64; UPKEEP_PARTS] = [living[0] * s.act, living[1] * s.act, living[2] * s.act, ph.c_leaf * s.leaf * days * k * s.act, ph.c_sapwood * s.sapwood * days * pace, ph.c_basal * roots * days * pace, renew];
    let cost: f64 = parts.iter().sum();
    let (units, fuel) = (cost / -chem.dh(p.burn), per_input(burn, p.sub[FUEL]).unwrap_or(1.0));
    // (What it burns is worked out first: one less a share near one would lose a small fuel to rounding.) It burns no
    // more than its fuel and the gas the cell's share of its air node still offers.
    let burnt = (units * fuel).min(c.mol[x][FUEL].max(0.0)).min(b.room(p, burn) * fuel);
    let short = if units > 0.0 { 1.0 - burnt / (units * fuel) } else { 0.0 };
    let e = p.content(chem);
    let unburnt = budget::held(&e, &c.mol[x]);
    b.run(chem, p, &mut c.mol[x], burn, burnt / fuel);
    // What the burning took of its content, shared out over the parts of the upkeep as they asked for it.
    let paid = budget::held(&e, &c.mol[x]) - unburnt;
    (0..UPKEEP_PARTS).for_each(|q| b.budget[UPKEEP + q] += if cost > 0.0 { paid * parts[q] / cost } else { 0.0 });
    // Members of a field wear out at the rate of living (Rubner 1908), a dormant one at least at `Physics::dormant` of
    // the active rate; below its frost gene, hardened by its dormancy, its frozen share dies (laws.md 3).
    let worn = active * s.act.max(ph.dormant) / (s.living * ph.e_life * dec(g[REPAIR])).max(1e-300);
    let frozen = super::frozen(ph, chem.subs[p.liquid].melt, g[FROST] as f64, (t, s.act));
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
        let alive = budget::held(&e, &c.mol[x]);
        for q in 0..NP {
            let m = c.mol[x][q].max(0.0) * dies;
            (c.mol[x][q], b.dead[p.sub[q]]) = (c.mol[x][q] - m, b.dead[p.sub[q]] + m);
        }
        b.budget[DIED] += budget::held(&e, &c.mol[x]) - alive;
    }
    (lost, cost)
}

/// Book what the cover of cell `i` (air node `j`, ground at `t` K) took and gave in its month (`Books`): the
/// light set aside for it from its aside node; gases from and to the air (taken from the air node with their heat at
/// its temperature and their latent heat, given into the aside node with theirs, as bodies breathe); liquids and
/// minerals from and to its ground, with their heat; its remains to its ground. Last its heat comes to the ground's
/// temperature with its ground.
fn book(w: &World, moves: &mut Moves, c: &mut Cover, p: &Parts, (i, j, t): (usize, usize, f64), set_aside: f64, b: &Books) {
    let chem = &w.chem;
    let (mut e, lift, ta, ground) = (c.e + set_aside + b.heat, w.lift(i), moves.t_air(w, j), Node::ground(i));
    moves.take(w, Node::aside(i), 0, 0.0, set_aside);
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
                moves.give(w, Node::aside(i), 0, 0.0, -m * s.cp * t);
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
