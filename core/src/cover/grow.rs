//! A month of the cover of each cell of a chunk (laws.md 3): it catches light and runs its light reaction, pays its
//! upkeep by burning its fuel, loses what is unpaid, worn out or frozen, builds from its fuel and the minerals of the
//! ground (fixing what it lacks if it can), and sends seeds out. Everything it takes and gives is booked through the
//! chunk's moves, with its heat; its own side it books itself.

use super::around::{around, Around};
use super::budget::{self, BUDGET, BUILDING, DIED, EXUDED, ROUNDED, SEEDS, SHED, STORED};
use super::upkeep::{keep_per_kg, upkeep};
use super::build::per_input;
use super::{dec, Cover, Parts, EMPTY_KG, FUEL, LEAF, LIGHT, MATTER, MOIST, NO_ROOT, NP, SLOTS, STORE, WALL};
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
            let moist = if crate::contract::land(w, i) > 0.0 { super::wet(w, &p, i, w.g(p.liquid, i)).1 } else { 1.0 };
            let c = d.cover(i);
            (c.mark, c.keep, c.film, c.reach) = (w.light[i], 0.0, 0.0, 0.0);
            remember(c, moist, w.cfg.months_per_year as f32);
        }
    }
}

/// The ground of cover `c` held `moist` of what it holds for roots this month (`cover::wet`): its share of months that
/// held none, a running mean over a year of `year` months (`Cover::idle`).
fn remember(c: &mut Cover, moist: f64, year: f32) {
    c.idle += ((moist <= 0.0) as u8 as f32 - c.idle) / year;
}

/// What a slot is in a month: its built cover and its living cover (`Cover::living`), kg; its capture area and what of
/// it absorbs, m²; its light per unit of absorbing area over the cover's mean, and at the bottom of its layer
/// (`math::shade`); its activity; what one more kg of it would store in the month at the bottom of its layer, J (`light`).
#[derive(Clone, Copy, Default)]
pub(super) struct Slot {
    pub(super) built: f64,
    pub(super) living: f64,
    pub(super) leaf: f64,
    pub(super) abs: f64,
    pub(super) shade: f64,
    pub(super) bottom: f64,
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
    // The light since the last step: its capture areas share it by the height of their plants, tallest first, each on
    // the share of its ground its roots hold.
    let dl = (a.light - c.mark).max(0.0);
    c.mark = a.light;
    // The seeds lying on its ground first: what wakes takes part in the month as seedlings.
    let mut flows = [0.0; BUDGET.len()];
    let lying = super::lying::month(w, c, p, a, (i, area, dt), &mut flows, counts);
    let mut s = [Slot::default(); SLOTS];
    let mut layers = Vec::with_capacity(n);
    for (q, sq) in s.iter_mut().enumerate().take(n) {
        let (built, leaf) = (c.built(&mm, q), c.leaf(&mm, q));
        if c.root[q] == NO_ROOT || built <= EMPTY_KG {
            continue;
        }
        let act = super::activity(&c.genes[q], a.moist);
        let sapwood = super::stand::sapwood(ph, (leaf, c.height(ph, &mm, rho, q)), c.walls(&mm, q), rho);
        *sq = Slot { built, living: c.living(&mm, q), leaf, abs: leaf * (1.0 - c.reflectance(chem, p, q)), act, sapwood, ..Default::default() };
        // The share of its ground (`cover::ground`) its plants hold: where its loose ground is as deep as their roots need (found again
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
    let (a_abs, lit): (f64, f64) = (s[..n].iter().map(|x| x.abs).sum(), s[..n].iter().map(|x| x.abs * x.shade).sum());
    let caught = dl * a_abs;
    let mut b = Books::new(a.air, flows);
    (0..NP).for_each(|q| b.dead[p.sub[q]] += lying[q]);
    let e = p.content(chem);
    let before = c.content(&e);
    // The light per unit of absorbing area at the top of its layers: with no leaves, all that reaches the ground.
    let per = if lit > 0.0 { caught * a.reach / lit } else { dl * a.reach };
    let stored = light(chem, ph, c, p, &mut s[..n], (per, w.mesh.cells().area()[i], a, dt), &mut b);
    b.budget[STORED] += c.content(&e) - before;
    let all: f64 = stored.iter().sum();
    c.keep = if caught > 0.0 { (all / caught) as f32 } else { 0.0 };
    let (year, rebuild) = (w.cfg.months_per_year as f32, p.living_cost(chem, ph));
    // The months its kinds expect without gain: those of the last year its ground held nothing for roots (`Cover::idle`).
    let months = c.idle as f64 * w.cfg.months_per_year as f64;
    // Its minerals in the ground, shared out slot by slot (the tallest first) as each builds.
    let mut ground = a.ground;
    let mut order: Vec<usize> = (0..n).filter(|&q| s[q].built > 0.0).collect();
    order.sort_by(|&a, &b| s[b].shade.total_cmp(&s[a].shade));
    for x in order {
        // It is in season while its top leaves gain what a kg of leaf costs to keep for the month (`keep_per_kg`). In a
        // month out of season it drops, before it pays the month's upkeep, the leaves it would cost more to keep through
        // the months it expects without gain than to build anew.
        let keep = keep_per_kg(w, c, p, x, (t, dt));
        let patch = area * c.rooted[x].max(f32::MIN_POSITIVE) as f64;
        let season = s[x].gain * (s[x].leaf / patch).exp() > keep;
        if !season {
            let shed = shed(c, x, &s[x], floor(keep, (keep, rebuild), Some(months)), patch);
            book_shed(&mut b, counts, (p, &mm, &e), shed);
            s[x] = after_shed(ph, c, (&mm, rho), x, s[x]);
        }
        let (lost, cost) = upkeep(w, c, p, x, &s[x], (t, dt), &mut b);
        counts.cover_lost.iter_mut().zip(lost).for_each(|(a, l)| *a += l);
        // What a kg of its living cover cost this month (with none left, what a kg would cost to keep), against what one
        // more kg would gain at the bottom of its layer: in season a layer that does not pay lies in its own shade.
        let per_kg = if s[x].living > 0.0 { cost / s[x].living } else { keep };
        let pays = s[x].gain > per_kg;
        c.full[x] = !pays;
        if !pays && season {
            let shed = shed(c, x, &s[x], floor(per_kg, (keep, rebuild), None), patch);
            book_shed(&mut b, counts, (p, &mm, &e), shed);
        }
        // Its balance over about a year: what it stored less its upkeep, per kg.
        c.net[x] += (((stored[x] - cost) / s[x].built) as f32 - c.net[x]) / year;
        // Where one more kg would not pay it builds living cover no more, but walls alone while its plants lack them; out
        // of season it builds nothing: its fuel is its store, kept for the leaves it builds anew.
        let (held, sent, made, out) = (budget::held(&e, &c.mol[x]), b.budget[SEEDS], c.built(&mm, x), c.seeds.len());
        if season {
            if pays || super::build::frame_short(w, c, p, x) <= 0.0 {
                super::build::build(w, c, p, x, (&s[x], pays), (i, a.tick, t, dt), &mut ground, &mut b);
            }
            super::build::frame(w, c, p, x, &mut b);
        }
        b.budget[BUILDING] += budget::held(&e, &c.mol[x]) - held - (b.budget[SEEDS] - sent);
        // What it built, kg of dry matter: what it holds more of, and what it sent as seeds.
        let seeds: f64 = c.seeds[out..].iter().map(|s| super::built_kg(&mm, &s.mol)).sum();
        counts.grown.push((i as u32, c.built(&mm, x) - made + seeds));
        super::stand::thin(c, &mm, ph, area, x);
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
    super::stand::merge_alike(c, &mm, (ph, n), area);
    remember(c, a.moist, year);
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
        // One more kg of living cover: its capture area's absorbing share (`LEAF`, its parts' colours) at the bottom of
        // its layer, so a slot that shed all its leaves still knows what a leaf would gain.
        let abs_kg = dec(c.genes[q][LEAF]) * (1.0 - c.reflectance(chem, p, q));
        s[q].gain = if s[q].built > 0.0 { (per * abs_kg * s[q].bottom * ph.light_yield / dh).min(cap).max(0.0) * gas * dh } else { 0.0 };
    }
    // Per unit of the reaction its leaves would lose the key liquid as vapour, `thirst` per mol of gas taken in; each
    // slot draws it from the pores its roots reach (`Cover::rooted`), and opens its pores no wider than that pays for.
    let gas_in: f64 = rx.inputs.iter().filter(|q| chem.phase(q.0 as usize, t) == Phase::Gas).map(|q| q.1 as f64).sum();
    let lost = a.thirst * gas_in;
    let drink = per_input(rx, p.liquid).unwrap_or(0.0) + lost;
    for (q, xq) in x.iter_mut().enumerate().take(s.len()) {
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

/// What slot `x` shed, `shed` mol of body matter, goes to its remains, counted as spent (`SPENT`) and booked (`SHED`).
fn book_shed(b: &mut Books, counts: &mut Counters, (p, mm, e): (&Parts, &[f64; NP], &[f64; NP]), shed: f64) {
    counts.cover_lost[SPENT as usize] += shed * mm[MATTER];
    b.dead[p.sub[MATTER]] += shed;
    b.budget[SHED] -= shed * e[MATTER];
}

/// What slot `x` of cover `c` is after it shed some of its living cover: its built and living cover, capture area, what
/// of that absorbs and its conducting walls (`stand::sapwood`) by what it holds now, its walls of density `rho`.
fn after_shed(ph: &crate::Physics, c: &Cover, (mm, rho): (&[f64; NP], f64), x: usize, s: Slot) -> Slot {
    let k = if s.living > 0.0 { c.living(mm, x) / s.living } else { 0.0 };
    let (leaf, height) = (s.leaf * k, c.height(ph, mm, rho, x));
    Slot { built: c.built(mm, x), living: c.living(mm, x), leaf, abs: s.abs * k, sapwood: super::stand::sapwood(ph, (leaf, height), c.walls(mm, x), rho), ..s }
}

/// The least a layer of a slot's leaves must gain a month, J per kg of living cover, to be kept (Kikuzawa 1991): a leaf
/// is kept while what it loses until it pays again costs less than building it anew, `rebuild` J a kg
/// (`Parts::living_cost`). In season (`months` None: its top leaves pay) a layer that does not pay lies in its own
/// canopy's shade and pays no more, so it must pay its month's upkeep, `per_kg` J a kg. Out of season none of its
/// leaves pay until the season comes back, after the months it expects without gain (`Cover::idle`), over which a kg
/// costs `keep` J a month to keep: it keeps those whose loss over them is less than `rebuild`.
fn floor(per_kg: f64, (keep, rebuild): (f64, f64), months: Option<f64>) -> f64 {
    match months {
        None => per_kg,
        Some(m) => keep - rebuild / m.max(f64::MIN_POSITIVE),
    }
}

/// Slot `x` of cover `c`, whose lowest leaves do not pay for themselves (`Cover::full`), sheds the living cover of the
/// layers of its canopy that gain less than `floor` J a kg (`floor`), as a canopy drops its leaves below its
/// compensation point (Monsi and Saeki 1953; Hirose 2005) and drops them all for a season it would cost more to keep
/// them through than to build them anew (Kikuzawa 1991): its light falls as e^-z with the capture area z above a leaf
/// in its patch of `patch` m², so what one more kg would gain at depth z is its gain at its bottom (`Slot::gain`) times
/// e^(y - z), y its whole depth, and the layers deeper than where that falls to `floor` go. A slot that gains nothing
/// sheds all its leaves or none, as its layers are alike. Returns the mol of body matter shed (to its remains); its
/// walls, its fuel and its plants stay.
fn shed(c: &mut Cover, x: usize, s: &Slot, floor: f64, patch: f64) -> f64 {
    let y = s.leaf / patch;
    if floor <= 0.0 || y <= 0.0 {
        return 0.0;
    }
    let keep = if s.gain > 0.0 { ((y + (s.gain / floor).ln()) / y).clamp(0.0, 1.0) } else { 0.0 };
    let m = c.mol[x][MATTER].max(0.0) * (1.0 - keep);
    c.mol[x][MATTER] -= m;
    m
}
