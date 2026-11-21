//! A month of the cover of each cell of a chunk (laws.md 3): it catches light and runs its light reaction, pays its
//! upkeep by burning its fuel, loses what is unpaid, worn out or frozen, builds from its fuel and the minerals of the
//! ground (fixing what it lacks if it can), and sends seeds out. Everything it takes and gives is booked through the
//! chunk's moves, with its heat; its own side it books itself.

use super::around::{around, Around};
use super::budget::{self, BUDGET, BUILDING, DIED, EXUDED, ROUNDED, SEEDS, SHED, STORED};
use super::build::per_input;
use super::{dec, Cover, Parts, EMPTY_KG, FUEL, GROUPS, LEAF, LIGHT, MOIST, NO_ROOT, NP, SLOTS, STORE, WALL};
use crate::units::MAX_SUBSTANCES;
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
            let land = super::ground(w, i);
            let moist = if land > 0.0 { super::soak(w, &p, i, w.g(p.liquid, i)).1.moist } else { 1.0 };
            let c = d.cover(i);
            (c.mark, c.keep, c.film, c.reach) = (w.light[i], 0.0, 0.0, 0.0);
            remember(c, moist, w.cfg.months_per_year as f32);
        }
    }
}

/// The ground of cover `c` held `moist` of what it holds for roots this month (`cover::soak`): its share of months that
/// held none, a running mean over a year of `year` months (`Cover::idle`).
fn remember(c: &mut Cover, moist: f64, year: f32) {
    c.idle += ((moist <= 0.0) as u8 as f32 - c.idle) / year;
}

/// What a slot is in a month: its built cover and its living cover (`Cover::living`), kg; its capture area and what of
/// it absorbs, m²; its light per unit of absorbing area over the cover's mean, and at the bottom of its layer (its
/// crown's base), and the capture area per unit of ground the light passes through its crown (`canopy::shade`); its
/// activity (`cover::activity`), the share of its roots that work (`cover::working`), the share of its ground under the
/// liquid, and the share of its plants on ground under the liquid past its roots' reach with the line over which none
/// lies (`Footing`); what one more kg of it would
/// store in the month at the bottom of its layer, J (`light`); its conducting walls, kg (`stand::sapwood`), and the key
/// liquid it drew, mol; and the share of what a leaf of it gains that the fine
/// roots drawing that leaf's liquid cost (`upkeep::root_share`).
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
    pub(super) work: f64,
    pub(super) wet: f64,
    pub(super) airless: f64,
    pub(super) rise: f64,
    pub(super) gain: f64,
    pub(super) sapwood: f64,
    pub(super) drunk: f64,
    pub(super) roots: f64,
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
/// each slot stands on the ground above its line, its roots working where they get the oxidising gas (`feet`); it
/// catches light, runs its light reaction, pays its upkeep, builds and sends seeds (into
/// `Cover::seeds`); alike slots merge, and slots left with no built cover empty into its remains. With no ground it holds
/// no plants, and its seeds lying wait on the ground under the liquid. Returns what it changed outside itself
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
    let e = p.content(chem);
    let feet = feet(w, c, (&mm, rho, n), i, &a.soak);
    let lying = super::lying::month(w, c, p, a, (i, area, dt), &mut flows, counts);
    if area <= 0.0 {
        let mut b = Books::new(a.air, flows);
        (0..NP).for_each(|q| b.dead[p.sub[q]] += lying[q]);
        empty(c, (p, &mm, &e, n), &mut b);
        (c.keep, c.film, c.reach) = (0.0, 0.0, 0.0);
        remember(c, a.soak.moist, w.cfg.months_per_year as f32);
        counts.cover_budget.iter_mut().zip(b.budget).for_each(|(a, f)| *a += f);
        return (b, 0.0);
    }
    super::stand::thin(c, &mm, (ph, rho, n), area);
    let mut s = [Slot::default(); SLOTS];
    // Per unit of the light reaction, the key liquid a leaf draws in this air (`drink`).
    let per_unit = drink(chem, p, a).0;
    let (mut layers, mut n_layers) = ([(0.0, 0.0, 0.0, 0.0, 0); SLOTS], 0);
    for (q, sq) in s.iter_mut().enumerate().take(n) {
        let (built, leaf) = (c.built(&mm, q), c.leaf(&mm, q));
        if c.root[q] == NO_ROOT || built <= EMPTY_KG {
            continue;
        }
        let sizes = c.sizes(ph, &mm, q);
        let h = sizes.height(ph, rho, sizes.large());
        let need = super::stand::need(ph, h);
        // Its activity by its drought: on its land by the liquid of the ground its plants stand on (`Footing::act`),
        // under the liquid full.
        let foot = feet[q];
        let (act, work) = ((1.0 - foot.wet) * foot.act + foot.wet * super::activity(&c.genes[q], 1.0), foot.work);
        let sapwood = super::stand::sapwood(ph, (leaf, sizes.lift(ph, rho)), c.walls(&mm, q), rho);
        let roots = super::upkeep::root_share(w, p, (t, dt), per_unit, &c.genes[q]);
        *sq = Slot { built, living: c.living(&mm, q), leaf, abs: leaf * (1.0 - c.reflectance(chem, p, q)), act, work, wet: foot.wet, airless: foot.airless, rise: foot.rise, sapwood, roots, ..Default::default() };
        // The share of its ground (`cover::ground`) its plants hold: where its loose ground is as deep as their roots need (found again
        // only when their need has moved by more than a bin of depth).
        if super::stand::moved(c.need[q], need) {
            c.rooted[q] = w.loose_bins.get(i).map_or(1.0, |b| super::stand::rooted_share(ph, b, need)) as f32;
            c.need[q] = need as f32;
        }
        let (base, top) = c.crown(ph, &mm, area, q, h);
        (layers[n_layers], n_layers) = ((base, top, leaf / area, c.on(q), q), n_layers + 1);
    }
    super::canopy::shade(&layers[..n_layers], |q, f, bottom, layer| (s[q].shade, s[q].bottom, s[q].layer) = (f, bottom, layer));
    let (a_abs, lit): (f64, f64) = (s[..n].iter().map(|x| x.abs).sum(), s[..n].iter().map(|x| x.abs * x.shade).sum());
    let caught = dl * a_abs;
    let mut b = Books::new(a.air, flows);
    (0..NP).for_each(|q| b.dead[p.sub[q]] += lying[q]);
    let before = c.content(&e);
    // The light per unit of absorbing area at the top of its layers: with no leaves, all that reaches the ground.
    let per = if lit > 0.0 { caught * a.reach / lit } else { dl * a.reach };
    let stored = light(chem, ph, c, p, &mut s[..n], (per, w.mesh.cells().area()[i], a, dt), &mut b);
    b.budget[STORED] += c.content(&e) - before;
    let all: f64 = stored.iter().sum();
    c.keep = if caught > 0.0 { (all / caught) as f32 } else { 0.0 };
    let (mut order, mut n_order) = ([0; SLOTS], 0);
    for q in (0..n).filter(|&q| s[q].built > 0.0) {
        (order[n_order], n_order) = (q, n_order + 1);
    }
    let order = &mut order[..n_order];
    order.sort_by(|&a, &b| s[b].shade.total_cmp(&s[a].shade));
    let order = &*order;
    let (pays, season) = super::season::pay(w, c, (p, &e), (&mut s, &stored), order, (t, dt), (&mut b, counts));
    grow(w, c, (p, &e), (&s, pays, season), order, (i, a, dt), (&mut b, counts));
    // Its plants grow where their roots gain (`soak::strips_share`): of what each slot holds now, what it stored this
    // month went to its strips and its rest by what a plant gains on each; the rest stands where it stood.
    for &q in order {
        let now = budget::held(&e, &c.mol[q]);
        if now > 0.0 {
            let (grew, on) = (stored[q].clamp(0.0, now), feet[q].strip);
            c.strip[q] = ((on * (now - grew) + super::strips_share(on, feet[q].gain) * grew) / now) as f32;
        }
    }
    super::stand::thin(c, &mm, (ph, rho, n), area);
    super::stand::merge_alike(c, &mm, (ph, n));
    remember(c, a.soak.moist, w.cfg.months_per_year as f32);
    empty(c, (p, &mm, &e, n), &mut b);
    fall(&mut c.cured, (p, &e), (ph.cured_life, dt), (&mut b, SHED));
    fall(&mut c.snag, (p, &e), (ph.snag_life, dt), (&mut b, DIED));
    counts.cover_budget.iter_mut().zip(b.budget).for_each(|(a, f)| *a += f);
    (b, caught)
}

/// Where each of the `n` slots in use of cover `c` of cell `i` stands this month (`cover::footing` above its line, a
/// slot in its first month taking the line of all the ground it may hold, at the roots its large plants need), its land
/// meeting their roots as `soak`: its share of the cell's ground is `Cover::stands`.
fn feet(w: &World, c: &mut Cover, (mm, rho, n): (&[f64; NP], f64, usize), i: usize, soak: &super::Soak) -> [super::Footing; SLOTS] {
    let ph = &w.cfg.phys;
    let mut out = [super::Footing::default(); SLOTS];
    for x in (0..n).filter(|&x| c.root[x] != NO_ROOT) {
        let need = super::stand::need(ph, c.height(ph, mm, rho, x));
        let line = (c.line[x] >= 0.0).then_some(c.line[x] as f64);
        out[x] = super::footing(w, i, &c.genes[x], need, soak, line.map(|l| (l, c.strip[x] as f64)));
        (c.line[x], c.stands[x], c.strip[x]) = (out[x].line as f32, out[x].share as f32, out[x].strip as f32);
    }
    out
}

/// What stands dead in a cover, `standing` (its cured leaves, `Cover::cured`, or its dead walls, `Cover::snag`), falls
/// to its remains over `dt` s, the share 1 − e^(−dt / `life`) of it, booked as the budget's flow `flow` (its parts `p`
/// hold `e` J a mol).
fn fall(standing: &mut [f64; NP], (p, e): (&Parts, &[f64; NP]), (life, dt): (f64, f64), (b, flow): (&mut Books, usize)) {
    let share = 1.0 - (-dt / life).exp();
    for q in 0..NP {
        let m = standing[q].max(0.0) * share;
        (standing[q], b.dead[p.sub[q]]) = (standing[q] - m, b.dead[p.sub[q]] + m);
        b.budget[flow] -= m * e[q];
    }
}

/// The slots of the `n` in use of cover `c` (its parts `p` of masses `mm`, holding `e` J a mol) left with no built cover
/// empty into its remains (`Books::dead`).
fn empty(c: &mut Cover, (p, mm, e, n): (&Parts, &[f64; NP], &[f64; NP], usize), b: &mut Books) {
    for x in 0..n {
        if c.root[x] != NO_ROOT && c.built(mm, x) <= EMPTY_KG {
            (0..NP).for_each(|q| b.dead[p.sub[q]] += c.mol[x][q]);
            b.budget[DIED] -= budget::held(e, &c.mol[x]);
            c.clear(x);
        }
    }
}

/// The slots of cover `c` in `order` (cell `i`, that reads `a`) grow over `dt` (`grow_slot`), each as its slot `s` in
/// the month says, whether it pays (`pays`) and whether it is in season (`season`), its parts `p` holding `e` J a mol.
/// Its minerals in the ground are shared out over the slots (`shares`): where the ground holds what they would take
/// (`build::wants`) each gets its share of all of it by that; where it is short, by their fine roots, as roots compete
/// below ground in proportion to their size (Casper and Jackson 1997), none more than it would take. Where
/// one more kg would not pay a slot builds living cover no more, but walls alone while its plants lack them; out of
/// season it builds nothing: its fuel is its store, kept for the leaves it builds anew. What a slot builds goes to its
/// groups of plants by `Cover::growth`.
fn grow(w: &World, c: &mut Cover, (p, e): (&Parts, &[f64; NP]), (s, pays, season): (&[Slot; SLOTS], [bool; SLOTS], [bool; SLOTS]), order: &[usize], (i, a, dt): (usize, &Around, f64), (b, counts): (&mut Books, &mut Counters)) {
    let mm = p.mm(&w.chem);
    let growth = c.growth(&w.cfg.phys, (&mm, w.chem.subs[p.sub[WALL]].density[0]), super::ground(w, i), order);
    let plans: Vec<_> = order.iter().map(|&x| (season[x] && (pays[x] || super::build::frame_short(w, c, p, x) <= 0.0)).then(|| super::build::per_kg(w, c, p, x, (&s[x], &growth[x]))).flatten()).collect();
    // (A lone slot's share is all of it.)
    let shared = order.len() > 1;
    let wants: Vec<_> = order.iter().zip(&plans).filter(|_| shared).map(|(&x, k)| k.as_ref().map_or([0.0; MAX_SUBSTANCES], |k| super::build::wants(w, (c, k), p, x, &s[x]))).collect();
    let ns = w.chem.ns();
    let roots: Vec<f64> = order.iter().map(|&x| s[x].drunk / super::upkeep::uptake(&w.cfg.phys, &c.genes[x])).collect();
    let mut got = vec![[0.0; MAX_SUBSTANCES]; order.len()];
    for k in (0..ns).filter(|_| shared) {
        let want: Vec<f64> = wants.iter().map(|d| d[k]).collect();
        shares(a.ground[k], &want, &roots).into_iter().enumerate().for_each(|(q, g)| got[q][k] = g);
    }
    for (q, (&x, plan)) in order.iter().zip(&plans).enumerate() {
        let mut ground = if shared { got[q] } else { a.ground };
        grow_slot(w, c, (p, e), x, (&s[x], pays[x], season[x], plan.as_ref(), &growth[x]), (i, a.tick, a.t, dt), &mut ground, (b, counts));
    }
}

/// The shares of `held` mol of a mineral of the slots that would take `want` of it, their fine roots `roots`: where
/// `held` covers all they would take, each its share of it by what it would take; where it is short, by their roots,
/// none more than it would take, what that leaves shared again over the others by their roots (all by what they would
/// take where no slot has roots).
fn shares(held: f64, want: &[f64], roots: &[f64]) -> Vec<f64> {
    let asked: f64 = want.iter().sum();
    let mut got = vec![0.0; want.len()];
    if asked <= 0.0 {
        return got;
    }
    if asked <= held || roots.iter().all(|&r| r <= 0.0) {
        return want.iter().map(|&x| held * x / asked).collect();
    }
    // By roots: each slot short of what it would take gets its share of what is left; those its share fills drop out.
    let mut left = held;
    let mut open: Vec<usize> = (0..want.len()).filter(|&q| want[q] > 0.0).collect();
    while left > 0.0 && !open.is_empty() {
        let r: f64 = open.iter().map(|&q| roots[q]).sum();
        if r <= 0.0 {
            break;
        }
        let full: Vec<usize> = open.iter().copied().filter(|&q| got[q] + left * roots[q] / r >= want[q]).collect();
        if full.is_empty() {
            open.iter().for_each(|&q| got[q] += left * roots[q] / r);
            break;
        }
        for &q in &full {
            left -= want[q] - got[q];
            got[q] = want[q];
        }
        open.retain(|q| !full.contains(q));
    }
    got
}

/// Slot `x` of cover `c` (cell `i`), its parts `p` holding `e` J a mol, in season builds from its fuel and its share
/// `ground` of its ground's minerals where it builds (`plan`, `build`) and builds its walls (`frame`), shared over its
/// groups of plants by `growth` (`Cover::grew`); it counts what it built (`Counters::grown`), exudes the fuel past its
/// reserve it could not use (out of season its fuel is its store), and sets what rounding left below zero to zero.
#[allow(clippy::too_many_arguments)]
fn grow_slot(w: &World, c: &mut Cover, (p, e): (&Parts, &[f64; NP]), x: usize, (s, pays, season, plan, growth): (&Slot, bool, bool, Option<&super::build::PerKg>, &[f64; GROUPS]), (i, tick, t, dt): (usize, u64, f64, f64), ground: &mut [f64; MAX_SUBSTANCES], (b, counts): (&mut Books, &mut Counters)) {
    let mm = p.mm(&w.chem);
    let (held, sent, made, out) = (budget::held(e, &c.mol[x]), b.budget[SEEDS], c.built(&mm, x), c.seeds.len());
    if let Some(k) = plan {
        super::build::build(w, (c, k), p, x, (s, pays), (i, tick, t, dt), ground, b);
    }
    if season {
        super::build::frame(w, c, p, x, b);
    }
    c.grew(x, (made, c.built(&mm, x)), growth);
    b.budget[BUILDING] += budget::held(e, &c.mol[x]) - held - (b.budget[SEEDS] - sent);
    // What it built, kg of dry matter: what it holds more of, and what it sent as seeds.
    let seeds: f64 = c.seeds[out..].iter().map(|s| super::built_kg(&mm, &s.mol)).sum();
    counts.grown.push((i as u32, c.built(&mm, x) - made + seeds));
    // The fuel it holds past its reserve when it has built what it could goes to its ground, as roots exude the
    // fuel a plant cannot use (Jones, Hodge and Kuzyakov 2004), where the open world's rot takes it.
    let over = if season { (c.mol[x][FUEL] - dec(c.genes[x][STORE]) * c.built(&mm, x) / mm[FUEL]).max(0.0) } else { 0.0 };
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

/// Per unit of the light reaction a leaf at the ground's temperature of `a` takes in: the key liquid it draws, mol (what
/// the reaction takes and what its pores lose for its gas, `Around::thirst` per mol of gas taken in), what of that its
/// pores lose, mol, and the mol of gas it takes in.
fn drink(chem: &Chemistry, p: &Parts, a: &Around) -> (f64, f64, f64) {
    let rx = &chem.reactions[p.light];
    let gas_in: f64 = rx.inputs.iter().filter(|q| chem.phase(q.0 as usize, a.t) == Phase::Gas).map(|q| q.1 as f64).sum();
    let lost = a.thirst * gas_in;
    (per_input(rx, p.liquid).unwrap_or(0.0) + lost, lost, gas_in)
}

/// The light reaction of each slot in use (`s`, the first of the cover's) over `dt` at the ground's temperature of `a`: what its share of the light caught
/// (`per` of it per unit of absorbing area times shade) stores at `light_yield`, at most its capacity times its
/// activity and the reaction's pace, both times what the air's share of the reaction's gas allows (`Around::gas`); all
/// slots cut together to the key liquid in the ground that is liquid (`Around::liquid`, mol) and the gas the cell's
/// share of its air node holds.
/// Each slot's `Slot::gain` is set: what one more kg would store, its light that of the bottom of its layer. A slot whose
/// fine roots would cost all a leaf gains (`Slot::roots`) loses more through its pores than they bring, and keeps them
/// shut: it stores nothing (Cowan and Farquhar 1977, from memory). Returns the energy each slot stored, J.
#[allow(clippy::too_many_arguments)]
fn light(chem: &Chemistry, ph: &crate::Physics, c: &mut Cover, p: &Parts, s: &mut [Slot], (per, cell, a, dt): (f64, f64, &Around, f64), b: &mut Books) -> [f64; SLOTS] {
    (c.film, c.reach) = (0.0, 0.0);
    // The key liquid its roots reach: its land's, and where a slot stands under the liquid, the rest of the cell's.
    let pond = if s.iter().any(|x| x.wet > 0.0) { a.soak.pond } else { 0.0 };
    let (t, liquid, gas) = (a.t, a.liquid + pond, a.gas);
    let rx = &chem.reactions[p.light];
    let (dh, pace) = (chem.dh(p.light), rx.relative_rate(t, ph.t_cat));
    let mut x = [0.0; SLOTS];
    for q in 0..s.len() {
        let need = dec(c.genes[q][MOIST]);
        let cap = dec(c.genes[q][LIGHT]) * need * pace * s[q].act * dt;
        let lit = |f: f64| per * s[q].abs * f * ph.light_yield / dh;
        // Only its roots that get the oxidising gas draw the liquid its pores lose (`Slot::work`): the rest close.
        x[q] = if s[q].roots < 1.0 { lit(s[q].shade).min(cap * s[q].living).max(0.0) * gas * s[q].work } else { 0.0 };
        // One more kg of living cover: its capture area's absorbing share (`LEAF`, its parts' colours) at the bottom of
        // its layer, so a slot that shed all its leaves still knows what a leaf would gain.
        let abs_kg = dec(c.genes[q][LEAF]) * (1.0 - c.reflectance(chem, p, q));
        s[q].gain = if s[q].built > 0.0 { (per * abs_kg * s[q].bottom * ph.light_yield / dh).min(cap).max(0.0) * gas * s[q].work * dh } else { 0.0 };
    }
    // Per unit of the reaction its leaves would lose the key liquid as vapour, `thirst` per mol of gas taken in; each
    // slot draws it from the pores its roots reach (`Cover::rooted`), and opens its pores no wider than that pays for.
    let (drink, lost, gas_in) = drink(chem, p, a);
    super::canopy::drawn(c, &mut x[..s.len()], liquid / drink.max(1e-300));
    let all: f64 = x.iter().sum();
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
    c.reach = ((0..s.len()).map(|q| x[q] * c.on(q)).sum::<f64>() / all) as f32;
    x.map(|xq| xq * f * dh)
}
