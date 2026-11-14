//! A month of the cover of each cell of a chunk (laws.md 3): it catches light and runs its light reaction, pays its
//! upkeep by burning its fuel, loses what is unpaid, worn out or frozen, builds from its fuel and the minerals of the
//! ground (fixing what it lacks if it can), and sends seeds out. Everything it takes and gives is booked through the
//! chunk's moves, with its heat; its own side it books itself.

use super::around::{around, Around};
use super::budget::{self, BUDGET, BUILDING, DIED, EXUDED, ROUNDED, SEEDS, SHED, STORED};
use super::upkeep::upkeep;
use super::build::per_input;
use super::{dec, Cover, Parts, EMPTY_KG, FUEL, LIGHT, MATTER, MOIST, NO_ROOT, NP, SLOTS, STORE, WALL};
use crate::units::MAX_SUBSTANCES;
use crate::life::SPENT;
use crate::chem::{Chemistry, Phase};
use crate::store::{Class, Node};
use crate::{life::Chunk, Counters, World};

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
/// it absorbs, m²; its light per unit of absorbing area over the cover's mean, and at the bottom of its layer (its
/// crown's base), and the capture area per unit of ground the light passes through its crown (`math::shade`); its
/// activity; what one more kg of it would store in the month at the bottom of its layer, J (`light`).
#[derive(Clone, Copy, Default)]
pub(super) struct Slot {
    pub(super) built: f64,
    pub(super) living: f64,
    pub(super) leaf: f64,
    pub(super) abs: f64,
    pub(super) shade: f64,
    pub(super) bottom: f64,
    pub(super) layer: f64,
    pub(super) act: f64,
    pub(super) gain: f64,
    pub(super) sapwood: f64,
    pub(super) drunk: f64,
}

/// What a cell's cover changes outside itself in a month, with the flows of its budget (`budget::BUDGET`).
pub type Books = crate::life::Books<{ BUDGET.len() }>;

/// The month of the cover of cell `i` of `d`: what it reads of the world (`around`), its month (`live`), and its
/// books (`life::book`).
fn cell(w: &World, d: &mut Chunk, p: &Parts, i: usize, dt: f64) {
    let (c, moves, counts) = d.cover_with(i);
    moves.open(w, i, Node::new(Class::Life, i));
    let a = around(w, moves, p, i);
    let keep = c.keep;
    let (b, caught) = live(w, c, p, &a, (i, dt), counts);
    // The light its aside node set aside for it (`Cover::keep`), as far as the node holds it.
    let set_aside = (keep as f64 * caught).min(moves.heat(w, Node::aside(i))).max(0.0);
    c.e = crate::life::book(w, moves, (i, a.t), (c.e, set_aside), c.cap(&w.chem, p) * a.t, &b);
}

/// The month over `dt` s of the cover `c` of cell `i` (its place on the map, where its seeds go from) that reads `a`:
/// it catches light, runs its light reaction, pays its upkeep, builds and sends seeds (into `Cover::seeds`); alike
/// slots merge, and slots left with no built cover empty into its remains. Returns what it changed outside itself
/// (`Books`) and the light it caught, J; what it lost and rounded is counted in `counts`.
pub fn live(w: &World, c: &mut Cover, p: &Parts, a: &Around, (i, dt): (usize, f64), counts: &mut Counters) -> (Books, f64) {
    let (chem, ph) = (&w.chem, &w.cfg.phys);
    let (area, mm, t, rho) = (super::ground(w, i), p.mm(chem), a.t, chem.subs[p.sub[WALL]].density[0]);
    // The slots its world uses (`Config::variety`).
    let n = w.cfg.variety;
    // The light since the last step: its capture areas share it down through their crowns, each on the share of its
    // ground its roots hold.
    let dl = (a.light - c.mark).max(0.0);
    c.mark = a.light;
    // The seeds lying on its ground first: what wakes takes part in the month as seedlings.
    let mut flows = [0.0; BUDGET.len()];
    let lying = super::lying::month(w, c, p, a, (i, area, dt), &mut flows, counts);
    super::stand::thin(c, &mm, (ph, rho, n), area);
    let mut s = [Slot::default(); SLOTS];
    let mut layers = Vec::with_capacity(n);
    for (q, sq) in s.iter_mut().enumerate().take(n) {
        let (built, leaf) = (c.built(&mm, q), c.leaf(&mm, q));
        if c.root[q] == NO_ROOT || built <= EMPTY_KG {
            continue;
        }
        let act = super::activity(ph, &c.genes[q], a.moist);
        let h = c.height(ph, &mm, rho, q);
        let sapwood = super::stand::sapwood(ph, (leaf, h), c.walls(&mm, q), rho);
        *sq = Slot { built, living: c.living(&mm, q), leaf, abs: leaf * (1.0 - c.reflectance(chem, p, q)), act, sapwood, ..Default::default() };
        // The share of its ground (`cover::ground`) its plants hold: where its loose ground is as deep as their roots need (found again
        // only when their need has moved by more than a bin of depth).
        let need = super::stand::need(ph, h);
        if super::stand::moved(c.need[q], need) {
            c.rooted[q] = w.loose_bins.get(i).map_or(1.0, |b| super::stand::rooted_share(ph, b, need)) as f32;
            c.need[q] = need as f32;
        }
        let (base, top) = c.crown(ph, &mm, area, q, h);
        layers.push((base, top, leaf / area, c.rooted[q] as f64, q));
    }
    crate::math::shade(&layers, |q, f, bottom, layer| (s[q].shade, s[q].bottom, s[q].layer) = (f, bottom, layer));
    let (a_abs, lit): (f64, f64) = (s[..n].iter().map(|x| x.abs).sum(), s[..n].iter().map(|x| x.abs * x.shade).sum());
    let caught = dl * a_abs;
    let mut b = Books::new(a.air, flows);
    (0..NP).for_each(|q| b.dead[p.sub[q]] += lying[q]);
    let e = p.content(chem);
    let before = c.content(&e);
    let stored = light(chem, ph, c, p, &mut s[..n], (caught * a.reach / lit.max(1e-300), w.mesh.cells().area()[i], a, dt), &mut b);
    b.budget[STORED] += c.content(&e) - before;
    let all: f64 = stored.iter().sum();
    c.keep = if caught > 0.0 { (all / caught) as f32 } else { 0.0 };
    let mut order: Vec<usize> = (0..n).filter(|&q| s[q].built > 0.0).collect();
    order.sort_by(|&a, &b| s[b].shade.total_cmp(&s[a].shade));
    let pays = pay(w, c, p, (&s, &stored), &order, (t, dt), (&mut b, counts));
    // Its minerals in the ground, shared out over the slots by what each would take (`build::wants`): the slots' roots
    // share the cell's ground, and where it is short each gets its share of what it holds, whatever slots hold them.
    // Where one more kg would not pay it builds living cover no more, but walls alone while its plants lack them.
    let plans: Vec<_> = order.iter().map(|&x| (pays[x] || super::build::frame_short(w, c, p, x) <= 0.0).then(|| super::build::per_kg(w, c, p, x)).flatten()).collect();
    let wants: Vec<_> = order.iter().zip(&plans).map(|(&x, k)| k.as_ref().map_or([0.0; MAX_SUBSTANCES], |k| super::build::wants(w, (c, k), p, x, &s[x]))).collect();
    let ns = chem.ns();
    let asked: Vec<f64> = (0..ns).map(|k| wants.iter().map(|d| d[k]).sum()).collect();
    for ((&x, want), plan) in order.iter().zip(&wants).zip(&plans) {
        let mut ground = [0.0; MAX_SUBSTANCES];
        (0..ns).filter(|&k| want[k] > 0.0 && asked[k] > 0.0).for_each(|k| ground[k] = a.ground[k] * (want[k] / asked[k]));
        grow_slot(w, c, p, x, (&s[x], pays[x], plan.as_ref()), (i, a.tick, t, dt), &mut ground, (&mut b, counts));
    }
    super::stand::thin(c, &mm, (ph, rho, n), area);
    super::stand::merge_alike(c, &mm, (ph, n));
    for x in 0..n {
        if c.root[x] != NO_ROOT && c.built(&mm, x) <= EMPTY_KG {
            (0..NP).for_each(|q| b.dead[p.sub[q]] += c.mol[x][q]);
            b.budget[DIED] -= budget::held(&e, &c.mol[x]);
            (c.mol[x], c.root[x], c.full[x], c.net[x], c.plants[x], c.need[x]) = ([0.0; NP], NO_ROOT, false, 0.0, 0.0, 0.0);
        }
    }
    counts.cover_budget.iter_mut().zip(b.budget).for_each(|(a, f)| *a += f);
    (b, caught)
}

/// Each slot of cover `c` in `order` pays its upkeep over `dt` at `t` K (`upkeep`), what it lost counted in `counts`;
/// where one more kg of it would not pay (its gain at the bottom of its layer, of its slot `s`, under its upkeep a kg)
/// it sheds the layers that do not pay (`shed`); its balance over about a year moves by what it stored (`stored`) less
/// its upkeep. Returns whether each pays.
fn pay(w: &World, c: &mut Cover, p: &Parts, (s, stored): (&[Slot; SLOTS], &[f64; SLOTS]), order: &[usize], (t, dt): (f64, f64), (b, counts): (&mut Books, &mut Counters)) -> [bool; SLOTS] {
    let (mm, e, year) = (p.mm(&w.chem), p.content(&w.chem), w.cfg.months_per_year as f32);
    let mut pays = [false; SLOTS];
    for &x in order {
        let (lost, cost) = upkeep(w, c, p, x, &s[x], (t, dt), b);
        counts.cover_lost.iter_mut().zip(lost).for_each(|(a, l)| *a += l);
        pays[x] = s[x].gain * s[x].built > cost;
        c.full[x] = !pays[x];
        if !pays[x] {
            let shed = shed(c, x, (&s[x], cost));
            counts.cover_lost[SPENT as usize] += shed * mm[MATTER];
            b.dead[p.sub[MATTER]] += shed;
            b.budget[SHED] -= shed * e[MATTER];
        }
        // Its balance over about a year: what it stored less its upkeep, per kg.
        c.net[x] += (((stored[x] - cost) / s[x].built) as f32 - c.net[x]) / year;
    }
    pays
}

/// Slot `x` of cover `c` (cell `i`) builds from its fuel and its share `ground` of its ground's minerals where it builds
/// (`plan`, `build`), its walls (`frame`), counts what it built (`Counters::grown`), exudes the fuel past its reserve it
/// could not use, and sets what rounding left below zero to zero.
#[allow(clippy::too_many_arguments)]
fn grow_slot(w: &World, c: &mut Cover, p: &Parts, x: usize, (s, pays, plan): (&Slot, bool, Option<&super::build::PerKg>), (i, tick, t, dt): (usize, u64, f64, f64), ground: &mut [f64; MAX_SUBSTANCES], (b, counts): (&mut Books, &mut Counters)) {
    let (mm, e) = (p.mm(&w.chem), p.content(&w.chem));
    let (held, sent, made, out) = (budget::held(&e, &c.mol[x]), b.budget[SEEDS], c.built(&mm, x), c.seeds.len());
    if let Some(k) = plan {
        super::build::build(w, (c, k), p, x, (s, pays), (i, tick, t, dt), ground, b);
    }
    super::build::frame(w, c, p, x, b);
    b.budget[BUILDING] += budget::held(&e, &c.mol[x]) - held - (b.budget[SEEDS] - sent);
    // What it built, kg of dry matter: what it holds more of, and what it sent as seeds.
    let seeds: f64 = c.seeds[out..].iter().map(|s| super::built_kg(&mm, &s.mol)).sum();
    counts.grown.push((i as u32, c.built(&mm, x) - made + seeds));
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

/// The light reaction of each slot in use (`s`, the first of the cover's) over `dt` at the ground's temperature of `a`: what its share of the light caught
/// (`per` of it per unit of absorbing area times shade) stores at `light_yield`, at most its capacity times its
/// activity and the reaction's pace, both times what the air's share of the reaction's gas allows (`Around::gas`); all
/// slots cut together to the key liquid in the ground that is liquid (`Around::liquid`, mol) and the gas the cell's
/// share of its air node holds.
/// Each slot's `Slot::gain` is set: what one more kg would store, its light that of the bottom of its layer. Returns the
/// energy each slot stored, J.
fn light(chem: &Chemistry, ph: &crate::Physics, c: &mut Cover, p: &Parts, s: &mut [Slot], (per, cell, a, dt): (f64, f64, &Around, f64), b: &mut Books) -> [f64; SLOTS] {
    let (t, liquid, gas) = (a.t, a.liquid, a.gas);
    let rx = &chem.reactions[p.light];
    let (dh, pace) = (chem.dh(p.light), rx.arrhenius(t.max(1.0)) / rx.arrhenius(ph.t_cat));
    let mut x = [0.0; SLOTS];
    for q in 0..s.len() {
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
    drawn(c, &mut x[..s.len()], liquid / drink.max(1e-300));
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
        b.run(chem, &p.sub, m, rx, xq * f);
        sq.drunk = xq * f * drink;
    }
    // Its pores as open as that leaves them: the film the key liquid's vapour leaves its leaves through over the month,
    // per m² of the cell, which the ground's evaporation passes it through day by day (`World::second_film`), from the
    // share of the ground's liquid its roots reach, by each slot's gas taken in.
    let taken = all * f * gas_in / dt;
    c.film = (a.pores * taken / cell) as f32;
    c.reach = (x.iter().zip(&c.rooted).map(|(xq, &r)| xq * r as f64).sum::<f64>() / all) as f32;
    x.map(|xq| xq * f * dh)
}

/// The units of the light reaction `x` the slots in use of cover `c` would run, cut to the key liquid their roots reach:
/// the liquid of each ring of its ground (`math::rings`, by the shares the slots' roots hold, `Cover::rooted`) is the
/// slots' that stand on it, `units` of the reaction a unit of the ground in all; where they would draw more there, each
/// draws it in proportion to what it would draw there. So the slots of a cell draw on the ground they share, whatever
/// slots hold them.
fn drawn(c: &Cover, x: &mut [f64], units: f64) {
    let share = |q: usize| (c.rooted[q] as f64).max(1e-300);
    let mut got = [0.0; SLOTS];
    for (at_least, ring) in crate::math::rings((0..x.len()).map(share)) {
        let on = || (0..x.len()).filter(|&q| share(q) >= at_least);
        let f = (units * ring / on().map(|q| x[q] * ring / share(q)).sum::<f64>().max(1e-300)).min(1.0);
        on().for_each(|q| got[q] += x[q] * ring / share(q) * f);
    }
    x.copy_from_slice(&got[..x.len()]);
}

/// Slot `x` of cover `c`, whose lowest leaves do not pay for themselves (`Cover::full`), sheds the living cover of the
/// layers of its canopy that do not, as a canopy drops its leaves below its compensation point (Monsi and Saeki 1953;
/// Hirose 2005): its light falls as e^-z with the capture area z above a leaf in its layer (`Slot::layer`, whatever
/// slots hold it, each leaf as deep in it as any), so what one more kg would store at depth z is its gain at its bottom
/// (`Slot::gain`) times e^(y - z), y the layer's whole depth, and the share of its leaves deeper than where that falls
/// to its upkeep a kg, `cost` J over its built cover, pays nothing. Returns the mol of body matter shed (to its remains);
/// its walls and its plants stay.
fn shed(c: &mut Cover, x: usize, (s, cost): (&Slot, f64)) -> f64 {
    let (y, per_kg) = (s.layer, cost / s.built.max(1e-300));
    if s.gain <= 0.0 || y <= 0.0 || per_kg <= 0.0 {
        return 0.0;
    }
    let keep = ((y + (s.gain / per_kg).ln()) / y).clamp(0.0, 1.0);
    let m = c.mol[x][MATTER].max(0.0) * (1.0 - keep);
    c.mol[x][MATTER] -= m;
    m
}
