//! A month of the cover of each cell of a domain (laws.md 4.2): it catches light and runs its light reaction, pays its
//! upkeep by burning its fuel, loses what is unpaid, worn out or frozen, builds from its fuel and the minerals of the
//! ground (fixing what it lacks if it can), and sends seeds out. Everything it takes and gives is booked through the
//! domain's moves, with its heat; its own side it books itself.

use super::{dec, distance, Cover, Parts, Seed, EMPTY_KG, FIX, FIX_OFF, FROST, FUEL, HARD, LIGHT, MATTER, NG, NO_ROOT, NP, REPAIR, SEED, SLOTS, STORE, WALL, WALLS, MOIST};
use crate::body::{AGE, PHASE, SPENT};
use crate::chem::{Chemistry, Phase, Reaction};
use crate::rng::{Purpose, Rng};
use crate::space::COMPASS;
use crate::store::{Class, Moves, Node};
use crate::units::{DAY, MAX_SUBSTANCES};
use crate::{life::Domain, World, AIR};

/// The month of the cover of every cell of domain `d` (`Domain::cover`), in the order of its cells; its seeds wait in
/// `Cover::seeds` until all domains ran (`land`).
pub fn step(w: &World, d: &mut Domain) {
    let Some(p) = Parts::of(&w.chem) else { return };
    let dt = w.cfg.calendar().month as f64;
    for k in 0..d.cells.len() {
        if Cover::slots(k).any(|x| d.cover.root[x] != NO_ROOT) {
            cell(w, d, &p, k, dt);
        } else {
            (d.cover.mark[k], d.cover.keep[k]) = (w.light[d.cells[k] as usize], 0.0);
        }
    }
}

/// What a slot is in a month: its built cover, kg; its capture area and what of it absorbs, m²; its light per unit of
/// absorbing area over the cover's mean (`math::shade`); its activity.
#[derive(Clone, Copy, Default)]
struct Slot {
    built: f64,
    leaf: f64,
    abs: f64,
    shade: f64,
    act: f64,
}

/// What a cell's cover changes outside itself in a month: per substance the mol it takes in (+) or gives out (-), the
/// heat its reactions release, J, and per substance its remains, mol.
struct Books {
    ext: [f64; MAX_SUBSTANCES],
    heat: f64,
    dead: [f64; MAX_SUBSTANCES],
}

impl Books {
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

/// The month of the cover of the `k`th cell of `d`.
fn cell(w: &World, d: &mut Domain, p: &Parts, k: usize, dt: f64) {
    let (chem, ph) = (&w.chem, &w.cfg.phys);
    let Domain { cover: c, moves, cells, counts, .. } = d;
    let i = cells[k] as usize;
    let (j, area, mm, t) = (w.mesh.up(i, 0, AIR), w.mesh.cells().area()[i], p.mm(chem), w.t_ground_last(i));
    moves.open(w, i, Node::new(Class::Cover, i));
    // How wet its ground is (`Physics::wetness`, as bodies feel it): its key liquid's depth, in its pores and standing.
    let depth = moves.amount(w, Node::ground(i), p.liquid) * chem.molar_volume(p.liquid, Phase::Liquid) / area;
    let moist = ph.wetness(depth);
    let reach = (-w.life.env.get(i).map_or(0.0, |e| e[1]) / ph.light_depth).exp();
    // The light since the last step: its capture areas share it by height (their walls per m²), tallest first.
    let dl = (w.light[i] - c.mark[k]).max(0.0);
    c.mark[k] = w.light[i];
    let mut s = [Slot::default(); SLOTS];
    let mut layers = Vec::with_capacity(SLOTS);
    for (q, x) in Cover::slots(k).enumerate() {
        let (built, leaf) = (c.built(&mm, x), c.leaf(&mm, x));
        if c.root[x] == NO_ROOT || built <= EMPTY_KG {
            continue;
        }
        let act = ph.activity(moist / dec(c.genes[x][MOIST]));
        s[q] = Slot { built, leaf, abs: leaf * (1.0 - c.reflectance(chem, p, x)), shade: 0.0, act };
        layers.push(((c.mol[x][WALL] * mm[WALL] / area) as f32, leaf / area, q));
    }
    crate::math::shade(&mut layers, |q, f| s[q].shade = f);
    let (a_abs, lit): (f64, f64) = (s.iter().map(|x| x.abs).sum(), s.iter().map(|x| x.abs * x.shade).sum());
    let caught = dl * a_abs;
    // The light its bodies' node set aside for it (`Cover::keep`), as far as the node holds it.
    let set_aside = (c.keep[k] as f64 * caught).min(moves.heat(w, Node::bodies(i))).max(0.0);
    let mut b = Books { ext: [0.0; MAX_SUBSTANCES], heat: 0.0, dead: [0.0; MAX_SUBSTANCES] };
    let stored = light(w, (moves, c), p, (k, i, j), &s, caught * reach / lit.max(1e-300), (t, dt), &mut b);
    c.keep[k] = if caught > 0.0 { (stored / caught) as f32 } else { 0.0 };
    // Its minerals in the ground, shared out slot by slot (the tallest first) as each builds.
    let mut ground: [f64; MAX_SUBSTANCES] = std::array::from_fn(|x| if x < chem.ns() { moves.amount(w, Node::ground(i), x) } else { 0.0 });
    let mut order: Vec<usize> = (0..SLOTS).filter(|&q| s[q].built > 0.0).collect();
    order.sort_by(|&a, &b| s[b].shade.total_cmp(&s[a].shade));
    for q in order {
        let x = k * SLOTS + q;
        let lost = upkeep(w, c, p, x, &s[q], (t, dt), &mut b);
        counts.cover_lost.iter_mut().zip(lost).for_each(|(a, l)| *a += l);
        build(w, c, p, x, &s[q], (i, t, dt), &mut ground, &mut b);
        // What rounding left below zero is set to zero and counted (a larger overdraw is this law's fault).
        let size: f64 = c.mol[x].iter().map(|m| m.abs()).sum();
        for m in c.mol[x].iter_mut().filter(|m| **m < 0.0) {
            debug_assert!(-*m <= crate::store::ROUNDING * size, "the cover overdrew a part by {m}");
            counts.rounded[0] -= std::mem::take(m);
        }
    }
    merge_alike(c, &mm, k, &w.cfg.phys);
    book(w, moves, c, p, (k, i, j, t), set_aside, &b);
}

/// The light reaction of each slot over `dt` at `t` K: what its share of the light caught (`per` of it per unit of
/// absorbing area times shade) stores at `light_yield`, at most its capacity times its activity and the reaction's
/// pace, all slots cut together to the key liquid in the ground and the gas the cell's share of its air node holds.
/// Returns the energy stored, J.
#[allow(clippy::too_many_arguments)]
fn light(w: &World, (moves, c): (&Moves, &mut Cover), p: &Parts, (k, i, j): (usize, usize, usize), s: &[Slot; SLOTS], per: f64, (t, dt): (f64, f64), b: &mut Books) -> f64 {
    let (chem, ph) = (&w.chem, &w.cfg.phys);
    let rx = &chem.reactions[p.light];
    let (dh, pace) = (chem.dh(p.light), rx.arrhenius(t.max(1.0)) / rx.arrhenius(ph.t_cat));
    let mut x = [0.0; SLOTS];
    for (q, v) in Cover::slots(k).enumerate() {
        let need = dec(c.genes[v][MOIST]);
        let cap = dec(c.genes[v][LIGHT]) * need * s[q].built * pace * s[q].act * dt;
        x[q] = (per * s[q].abs * s[q].shade * ph.light_yield / dh).min(cap).max(0.0);
    }
    let all: f64 = x.iter().sum();
    if all <= 0.0 {
        return 0.0;
    }
    // What the cell offers of each input that is not a part: the gas of its share of its air node, the rest from its ground.
    let share = w.mesh.share(i, AIR);
    let seen = moves.seen(w, j);
    let offer = |kk: usize| if chem.phase(kk, t) == Phase::Gas { seen(kk) * share } else { moves.amount(w, Node::ground(i), kk) };
    let f = rx.inputs.iter().filter(|q| !p.sub.contains(&(q.0 as usize))).map(|&(kk, n)| offer(kk as usize) / (n as f64 * all)).fold(1.0, f64::min);
    for (q, v) in Cover::slots(k).enumerate() {
        b.run(chem, p, &mut c.mol[v], rx, x[q] * f);
    }
    all * f * dh
}

/// Slot `x` pays its upkeep over `dt` at `t` K by burning its fuel, then loses the share unpaid (`SPENT`), the share
/// its wear gives (`AGE`) and the share frozen (`PHASE`) to its cell's remains. Returns the kg lost by cause.
fn upkeep(w: &World, c: &mut Cover, p: &Parts, x: usize, s: &Slot, (t, dt): (f64, f64), b: &mut Books) -> [f64; 6] {
    let (chem, ph, g) = (&w.chem, &w.cfg.phys, c.genes[x]);
    let (burn, days) = (&chem.reactions[p.burn], dt / DAY);
    let pace = burn.arrhenius(t.max(1.0)) / burn.arrhenius(ph.t_cat);
    // A capacity that could store `cap` W per kg costs the share `c_light` of that, the dearer the colder it stands (as
    // a firm catalyst costs more, `t_firm`).
    let cap = dec(g[LIGHT]) * dec(g[MOIST]) * chem.dh(p.light) * DAY;
    let firm = 1.0 + (chem.subs[p.liquid].melt - g[FROST] as f64).max(0.0) / ph.c_firm;
    let cost = (s.built * days * (ph.c_basal + ph.c_light * cap * firm) + ph.c_leaf * s.leaf * days) * ph.repair(g[REPAIR] as f64) * s.act * pace;
    let (units, fuel) = (cost / -chem.dh(p.burn), per_input(burn, p.sub[FUEL]).unwrap_or(1.0));
    let short = if units * fuel > c.mol[x][FUEL] { 1.0 - c.mol[x][FUEL] / (units * fuel) } else { 0.0 };
    b.run(chem, p, &mut c.mol[x], burn, units * (1.0 - short));
    // Members of a field wear out at the rate of living as a body's do (laws.md 6.7); below its frost gene its frozen
    // share dies as far as it is active, within `unfold` K.
    let worn = cost / (s.built * ph.e_life * dec(g[REPAIR]));
    let frozen = if t < g[FROST] as f64 { ((g[FROST] as f64 - t) / ph.unfold).min(1.0) * s.act } else { 0.0 };
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
            let m = c.mol[x][q] * dies;
            (c.mol[x][q], b.dead[p.sub[q]]) = (c.mol[x][q] - m, b.dead[p.sub[q]] + m);
        }
    }
    lost
}

/// Slot `x` builds what its fuel over its reserve (`STORE`) and the minerals of its ground (`ground`, mol, shared out
/// as slots build) allow, by the conversions of growth (`Chemistry::build`, `builds`) at its shares (`WALLS`,
/// `Physics::cover_hard`), paying `c_copy` per kg by burning fuel; a fixer (`FIX`) makes the mineral it lacks, at its
/// capacity, paid by the fixing reaction's fuel. The share `SEED` of what it builds leaves as seeds for a neighbour
/// cell drawn (`Seed`), with a generation's mutation.
#[allow(clippy::too_many_arguments)]
fn build(w: &World, c: &mut Cover, p: &Parts, x: usize, s: &Slot, (i, t, dt): (usize, f64, f64), ground: &mut [f64; MAX_SUBSTANCES], b: &mut Books) {
    let (chem, ph, g) = (&w.chem, &w.cfg.phys, c.genes[x]);
    let (Some(bl), mm) = (chem.build.as_ref(), p.mm(chem)) else { return };
    let conv = |q: usize| chem.builds.iter().find(|r| r.outputs[0].0 as usize == p.sub[q]);
    let (Some(wall), Some(hard)) = (conv(WALL), conv(HARD)) else { return };
    // Per kg built: the units of each conversion, and what they take.
    let fm = 1.0 / (1.0 + dec(g[WALLS]) + ph.cover_hard);
    let units = [(bl, fm / mm[MATTER] / bl.outputs[0].1 as f64), (wall, dec(g[WALLS]) * fm / mm[WALL] / wall.outputs[0].1 as f64), (hard, ph.cover_hard * fm / mm[HARD] / hard.outputs[0].1 as f64)];
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
    let fixed = p.fix.filter(|_| g[FIX] > FIX_OFF).and_then(|r| {
        let rx = &chem.reactions[r];
        let &(m, out) = rx.outputs.iter().find(|o| need[o.0 as usize] > 0.0)?;
        Some((rx, m as usize, out as f64, per_input(rx, p.sub[FUEL])?, dec(g[FIX]) * s.built * rx.arrhenius(t.max(1.0)) / rx.arrhenius(ph.t_cat) * s.act * dt))
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
    let seed = g[SEED].clamp(0.0, 1.0) as f64;
    if seed > 0.0 {
        let mut mol = [0.0; NP];
        for q in [MATTER, WALL, HARD] {
            mol[q] = (c.mol[x][q] - before[q]).max(0.0) * seed;
            c.mol[x][q] -= mol[q];
        }
        let heat = (0..NP).map(|q| mol[q] * chem.subs[p.sub[q]].cp * t).sum::<f64>();
        b.heat -= heat;
        let mut r = Rng::new(w.seed, w.month(), (i * SLOTS + x % SLOTS) as u64, Purpose::Cover);
        let cells = w.mesh.cells();
        let to = cells.way(i, r.below(COMPASS)).map_or(i, |e| cells.adj_cell()[e] as usize);
        let genes = super::mutate(&g, ph.cover_mut, &mut r);
        c.seeds.push(Seed { to: to as u32, mol, genes, root: c.root[x], heat });
    }
}

/// Alike kinds of the `k`th cell (`Physics::alike` steps in every gene, `distance`) share a slot: the lighter joins the
/// heavier, the genes the mean by built cover, its starting kind the heavier's.
pub(super) fn merge_alike(c: &mut Cover, mm: &[f64; NP], k: usize, ph: &crate::Physics) {
    for a in Cover::slots(k) {
        for z in a + 1..(k + 1) * SLOTS {
            if c.root[a] == NO_ROOT || c.root[z] == NO_ROOT || distance(&c.genes[a], &c.genes[z]) > ph.alike {
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
    (c.mol[small], c.root[small]) = ([0.0; NP], NO_ROOT);
}

/// Book what the `k`th cell's cover (cell `i`, air node `j`, ground at `t` K) took and gave in its month (`Books`): the
/// light set aside for it from its bodies' node; gases from and to the air (taken from the air node with their heat at
/// its temperature and their latent heat, given into the bodies' node with theirs, as bodies breathe); liquids and
/// minerals from and to its ground, with their heat; its remains to its ground. Its slots left with no built cover go
/// to the ground too. Last its heat comes to the ground's temperature with its ground.
fn book(w: &World, moves: &mut Moves, c: &mut Cover, p: &Parts, (k, i, j, t): (usize, usize, usize, f64), set_aside: f64, b: &Books) {
    let chem = &w.chem;
    let mm = p.mm(chem);
    let (mut e, lift, ta, ground) = (c.e[k] + set_aside + b.heat, w.lift(i), moves.t_air(w, j), Node::ground(i));
    moves.take(w, Node::bodies(i), 0, 0.0, set_aside);
    let mut dead = b.dead;
    for x in Cover::slots(k) {
        if c.root[x] != NO_ROOT && c.built(&mm, x) <= EMPTY_KG {
            (0..NP).for_each(|q| dead[p.sub[q]] += c.mol[x][q]);
            (c.mol[x], c.root[x]) = ([0.0; NP], NO_ROOT);
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
    let target = c.cap(chem, p, k) * t;
    if target > e {
        e += moves.take_from_ground(w, i, (0, 0.0, 0.0), target - e, cap).1;
    } else {
        moves.give(w, ground, 0, 0.0, e - target);
        e = target;
    }
    c.e[k] = e;
}
