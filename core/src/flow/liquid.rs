//! Liquid on and in the loose ground at each liquid step: the stores of the bed drain to the channels inside the cells
//! (`drain::springs`); what may run off runs down the cells in one pass in the order of a priority flood
//! (`solve::downhill`), doing work on its bed; the liquid under the surface moves (`drain::under`); and the basin stands
//! at one level (`basin::level`).

use super::{bare_share, drain::{springs, under}, follow, land::{lift, solid, stand, wet, NONE}, potential::{apply, Lifts}, range, year_weight, Scratch};
use crate::{math::one_minus_exp, solve::downhill::{pass, Adjacency, Crossed}, space::Level, store::{Class, LinkFlux, Move, Node}, units::T_FLOOR, World};

/// Least bond strength wear divides by.
const STRENGTH_MIN: f64 = 0.05;

/// What a cell's liquid finds toward a neighbour in the pass (`surface`): the drop, m, where it is a way down; the
/// volume that may move there, m³; and the fall of its head, m.
type Toward = (Option<f64>, f64, f64);

/// The mobile liquid of every ground node: of each liquid `liq[q]`, `mol[q * n + i]` (its liquid share; all of a
/// dissolved gas), and its volume `vol[i]`, m³ (dissolved gas adds none).
pub(super) struct Mobile {
    pub(super) mol: Vec<f64>,
    pub(super) vol: Vec<f64>,
}

/// The mobile liquid of the ground nodes where `on(i)` (none elsewhere), of the liquids `liq`; what boils below the
/// coldest node, `tmin`, is gas.
pub(super) fn mobile(w: &World, s: &Scratch, liq: &[usize], tmin: f64, on: impl Fn(usize) -> bool) -> Mobile {
    let n = w.mesh.cells().n();
    let mut m = Mobile { mol: vec![0.0; liq.len() * n], vol: vec![0.0; n] };
    for (q, &k) in liq.iter().enumerate() {
        let gas = s.sp[k].boil <= tmin;
        for i in (0..n).filter(|&i| on(i)) {
            let a = w.store.amounts(Class::Ground)[k * n + i] * if gas { 1.0 } else { wet(s, k, i) };
            m.mol[q * n + i] = a;
            m.vol[i] += if gas { 0.0 } else { a * s.sp[k].v[1] };
        }
    }
    m
}

/// Book `fluxes` through the store, and bring the ground nodes' state up to date for what changed (`Scratch::note`).
pub(super) fn book(w: &mut World, s: &mut Scratch, fluxes: &[LinkFlux]) {
    let n = w.mesh.cells().n();
    let mut ks: Vec<usize> = fluxes.iter().flat_map(|x| x.ks.iter()).map(|&k| k as usize).collect();
    ks.sort_unstable();
    ks.dedup();
    // The ground nodes at the ends of the links, in order: no other one changes.
    let mut cells: Vec<u32> = Vec::new();
    for x in fluxes {
        let ends = [(x.class[0], x.a), (x.class[1], x.b)];
        ends.iter().filter(|e| e.0 == Class::Ground).for_each(|e| cells.extend_from_slice(e.1));
    }
    cells.sort_unstable();
    cells.dedup();
    let amount = |w: &World, k: usize, i: u32| w.store.amounts(Class::Ground)[k * n + i as usize];
    let before: Vec<f64> = ks.iter().flat_map(|&k| cells.iter().map(move |&i| (k, i))).map(|(k, i)| amount(w, k, i)).collect();
    apply(w, s, fluxes, &[]);
    for (q, &k) in ks.iter().enumerate() {
        for (&i, &was) in cells.iter().zip(&before[q * cells.len()..(q + 1) * cells.len()]) {
            let now = amount(w, k, i);
            if now != was {
                s.note(k, i as usize, now - was, w.mesh.cells().area()[i as usize]);
            }
        }
    }
}

/// One liquid step of `dt` seconds; last, the gases of the air dissolve in the liquid where it lies (`air::dissolve`).
pub(super) fn liquid(w: &mut World, s: &mut Scratch, dt: f64) {
    let (tmin, tmax) = range(&s.state.tg);
    let liq = s.liquids(&w.chem, (tmin, tmax), true);
    let slope = slopes(w, s);
    let runs = springs(w, s, tmin, &slope, dt);
    surface(w, s, (&liq, tmin), (&slope, &runs), dt);
    under(w, s, &liq, tmin, dt);
    super::basin::level(w, s, &liq, tmin);
    super::air::dissolve(w, s, dt);
}

/// The slope of the surface of each ground node down its steepest way (`Level::steepest`) to the lower neighbours (by
/// more than `still`), 0 for none; and for the low basin, whose liquid stands at one level and drains to no channel.
fn slopes(w: &World, s: &Scratch) -> Vec<f64> {
    let (cells, still) = (w.mesh.cells(), w.cfg.phys.still);
    let drop = |i: usize, k: usize| Some(stand(s, i) - stand(s, cells.around(i).1[k] as usize)).filter(|&d| d > still);
    crate::par::map_n(cells.n(), crate::par::UNCOUNTED, |i| if s.basin.is[i] { 0.0 } else { cells.steepest(i, |k| drop(i, k)).map_or(0.0, |way| way.0) })
}

/// The liquid that may run off (what stands, what came up from the store, and the pores' drainage to the channels, a
/// share 1 - exp(-`seep` × slope × time / (`porosity` × `hill`))) moves down the cells in one pass (`solve::downhill`;
/// laws.md 1.2), with its heat; lost head heats what runs out and does work on the bed (`load`, `wear`).
fn surface(w: &mut World, s: &mut Scratch, (liq, tmin): (&[usize], f64), (slope, runs): (&[f64], &[f64]), dt: f64) {
    let (n, area, ph, g) = (w.mesh.cells().n(), w.mesh.cells().area(), w.cfg.phys, w.cfg.gravity);
    let mob = mobile(w, s, liq, tmin, |_| true);
    // Rows that move together per node (`q[i * width + r]`): each liquid of `liq`, each load of `sol`, their heat, J, and
    // the liquid's volume, m³.
    let sol = s.solids(tmin);
    let ks: Vec<usize> = liq.iter().chain(&sol).copied().collect();
    let (nl, rows) = (liq.len(), ks.len());
    let (heat, vol, width) = (rows, rows + 1, rows + 2);
    let mut q = vec![0.0; width * n];
    for i in 0..n {
        let (pores, own) = (s.pd[i] * area[i], mob.vol[i] - runs[i]);
        let lp = own.min(pores);
        let free = if s.head[i] > s.z[i] { lp - ph.field * pores } else { 0.0 };
        let hill = if free > 0.0 && slope[i] > 0.0 { free.min(lp * one_minus_exp(ph.seep * slope[i] * dt / (ph.porosity * ph.hill))) } else { 0.0 };
        let off = if mob.vol[i] > 0.0 { (((own - pores).max(0.0) + runs[i] + hill) / mob.vol[i]).min(1.0) } else { 0.0 };
        (0..nl).for_each(|r| q[i * width + r] = mob.mol[r * n + i] * off);
        q[i * width + vol] = mob.vol[i] * off;
    }
    let head = |q: &[f64], j: usize| s.z[j] + q[j * width + vol] / area[j];
    let (q0, cells) = (q.clone(), w.mesh.cells());
    let adj = Adjacency { start: cells.start(), link: cells.adj_link(), other: cells.adj_cell(), a: cells.a() };
    let mut order = std::mem::take(&mut s.order);
    order.update(&adj, &s.z, |i| head(&q, i), &s.basin.cells);
    let (mut sent, mut worn, mut outs, mut rest) = (vec![0.0; n], vec![0.0; n], Vec::<Toward>::new(), Vec::new());
    // A land cell, as `World::is_land` says: its solid surface above 0 m (Flow's state of it).
    let (land, mut off_land): (Vec<bool>, Vec<f64>) = (s.z.iter().map(|&z| z > 0.0).collect(), vec![0.0; n]);
    let (mut crossed, mut given) = (Crossed::default(), vec![0.0; n]);
    let (ww, ss) = (&*w, &*s);
    pass(&order, &adj, &mut q, &mut crossed, |i, q, share| {
        let (zi, hd, sv) = (ss.z[i], head(q, i), q[i * width + vol]);
        if sv <= 0.0 {
            return;
        }
        // Each lower neighbour: the drop toward it, the volume that may move toward it (to a neighbour already passed,
        // what levels the two heads, whatever their areas, up to its spill level), and the fall of the head.
        // The surface of a cell whose liquid stands is its head, but no higher than its spill level (what stands over it
        // passes on); of one its liquid passes on through, its ground (a neighbour still to come holds the head back only
        // by what it keeps). Liquid never goes back to a neighbour already passed whose liquid passes on: that one lies
        // upstream in the flood, and what it got would come back the next step; to one whose liquid stands it levels the
        // two heads, but no higher than that one's spill level: what stands over it would pass on, and that one has
        // passed. It takes the steepest way down (`Level::steepest`), and on level ground the flood's way out; what that
        // way cannot take runs down the others (`run_on`).
        outs.clear();
        let (top, mut way) = (if order.stands[i] { hd } else { zi }, None);
        for (k, j) in cells.around(i).1.iter().map(|&j| j as usize).enumerate() {
            let (hj, passed) = (head(q, j), order.before(j, i));
            let to = match (passed, order.stands[j]) {
                (true, _) => hj,
                (false, true) => hj.min(order.spill[j]),
                (false, false) => ss.z[j],
            };
            let from = if passed { top } else { hd };
            let room = if passed { ((from - to) * (area[i] * area[j] / (area[i] + area[j]))).min((order.spill[j] - hj) * area[j]) } else { (hd - to.max(zi)) * area[i] };
            let down = from - to > ph.still && (!passed || order.stands[j]) && room > 0.0;
            if down && j == order.outlet[i] as usize {
                way = Some([(k, 1.0), (k, 0.0)]);
            }
            outs.push((down.then_some((top - to).max(0.0)), room, (hd - hj).max(0.0)));
        }
        let Some(ways) = cells.steepest(i, |k| outs[k].0).map(|w| w.1).or(way) else { return };
        let (mut out, mut drop, mut short) = (0.0, 0.0, false);
        for (k, f) in ways.into_iter().filter(|p| p.1 > 0.0) {
            share[k] = (sv * f).min(outs[k].1) / sv;
            (out, drop, short) = (out + share[k], drop + share[k] * outs[k].2, short || sv * f > outs[k].1);
        }
        if short {
            (out, drop) = run_on(cells, i, (&outs, &mut rest), share, sv, (out, drop));
        }
        // The node's heat capacity and fusion enthalpy with what came in. Liquid that came in holds its fusion enthalpy
        // as it passes, whatever the node's own phase (its heat brought it); a load holds none.
        let (mut c, mut held) = (ss.cap[i], ss.latent[i].held);
        let (cell, was) = (i * width..(i + 1) * width, &q0[i * width..(i + 1) * width]);
        for (r, &k) in ks.iter().enumerate().filter(|p| q[cell.start + p.0] != was[p.0]) {
            let d = q[cell.start + r] - was[r];
            (c, held) = (c + d * ss.sp[k].cp, held + if r < nl { d * ss.sp[k].lf } else { 0.0 });
        }
        let carried = {
            let q = &mut q[cell];
            // The temperature of the node mixed with what came in, at which what runs out leaves: off a melting plateau
            // its energy over its fusion enthalpy and heat capacity; on one, what it gains or loses on the plateau's
            // temperature melts or freezes the plateau's substance first (`FlowView::fused`), and only the rest warms
            // or cools it. So the liquid passing on leaves at a temperature the node pays for, and what stays keeps it.
            let lat = match ss.state.pk[i] { NONE => 0.0, k => ww.store.amounts(Class::Ground)[k * n + i] * ss.sp[k].lf };
            let x = ww.store.heat(Class::Ground)[i] + q[heat] - held - c * ss.state.tg[i];
            let t = (ss.state.tg[i] + (x.min(0.0) + (x - lat).max(0.0)) / c.max(1.0)).max(T_FLOOR);
            let kg = |q: &[f64], rs: std::ops::Range<usize>| rs.map(|r| q[r] * ss.sp[ks[r]].mm).sum::<f64>();
            // The work, on the bed below the liquid left standing (which hides it); none below 0 m, where the load
            // settles.
            let left = sv * (1.0 - out) / area[i];
            let bedwork = if land[i] { g * drop * kg(q, 0..nl) * ph.hide[1] / (ph.hide[1] + left) } else { 0.0 };
            worn[i] = load(ww, ss, i, q, (nl..rows, &sol), bedwork, out);
            q[heat] = ks.iter().enumerate().map(|(r, &k)| q[r] * (ss.sp[k].cp * t + if r < nl { ss.sp[k].lf } else { 0.0 })).sum();
            // What leaves a land cell for cells that are not land runs off the land.
            let off = (0..share.len()).filter(|&k| land[i] && !land[cells.around(i).1[k] as usize]).map(|k| share[k]);
            let liquid = q[..nl].iter().sum::<f64>();
            (sent[i], off_land[i]) = (out * liquid, off.sum::<f64>() * liquid);
            kg(q, 0..rows)
        };
        if let Some(l) = ss.lifts.as_ref() {
            give_falls(l, cells, (i, share, carried * g), (q, width, heat), &mut given);
        }
    });
    // The crossings as a link flux: each substance once (liquid and load rows of one substance share it), then the heat.
    let kinds: Vec<u16> = liq.iter().chain(sol.iter().filter(|k| !liq.contains(k))).map(|&k| k as u16).collect();
    let col_of: Vec<usize> = ks.iter().map(|&k| kinds.iter().position(|&x| x as usize == k).unwrap_or(0)).chain([kinds.len()]).collect();
    let (a, b, mol) = by_link(&crossed, rows + 2, &col_of, (cells.a(), cells.b()));
    let at = kinds.len() * a.len();
    if let Some(l) = s.lifts.as_mut() {
        l.give(w, &given);
    }
    book(w, s, &[LinkFlux { class: [Class::Ground; 2], a: &a, b: &b, ks: &kinds, mol: &mol[..at], heat: &mol[at..] }]);
    s.order = order;
    let (ema, mut broke) = (year_weight(w, dt), Vec::new());
    for i in 0..n {
        follow(&mut w.liquid_flux[i], (sent[i] / dt) as f32, ema);
        follow(&mut w.off_land[i], (off_land[i] / dt) as f32, ema);
        if worn[i] > 0.0 {
            wear(w, s, i, worn[i], &mut broke);
        }
    }
    apply(w, s, &[], &broke);
}

/// What the steepest way of cell `i` cannot take (`share`, of `out` of its `sv` m³, the head falling by `drop`) runs
/// down its other ways (`outs`: the drop, the room and the fall of the head toward each), the steepest first, each as
/// far as it takes. Returns the share that leaves and the fall of the head, as `out` and `drop`.
fn run_on(cells: &Level, i: usize, (outs, rest): (&[Toward], &mut Vec<usize>), share: &mut [f64], sv: f64, (mut out, mut drop): (f64, f64)) -> (f64, f64) {
    let slope = |k: usize| {
        let u = cells.offset(i, cells.around(i).1[k] as usize);
        outs[k].0.unwrap_or(0.0) / u[0].hypot(u[1])
    };
    rest.clear();
    rest.extend((0..outs.len()).filter(|&k| outs[k].0.is_some() && outs[k].1 > share[k] * sv));
    rest.sort_by(|&a, &b| slope(b).total_cmp(&slope(a)).then(a.cmp(&b)));
    for &k in rest.iter() {
        let more = ((1.0 - out) * sv).min(outs[k].1 - share[k] * sv) / sv;
        (share[k], out, drop) = (share[k] + more, out + more, drop + more * outs[k].2);
    }
    (out, drop)
}

/// The fall of what cell `i` sends down each way (its share `share[k]` of what runs out, `weight` N), from its head to
/// the lower head of the cell it lands in, as the booking of Flow's lifts and falls has them at its start (`l`), heats
/// what lands there (the heat row `q[j * width + heat]`, and `given[j]`, J): what runs on from there takes its share
/// along, as the liquid that brought it would.
fn give_falls(l: &Lifts, cells: &Level, (i, share, weight): (usize, &[f64], f64), (q, width, heat): (&mut [f64], usize, usize), given: &mut [f64]) {
    for (k, &j) in cells.around(i).1.iter().enumerate().filter(|p| share[p.0] > 0.0) {
        let j = j as usize;
        let e = share[k] * weight * (l.head_at_start(i) - l.head_at_start(j)).max(0.0);
        (q[j * width + heat], given[j]) = (q[j * width + heat] + e, given[j] + e);
    }
}

/// The crossings of a pass as the columns of a link flux, row `r` in column `col_of[r]`: the links that carried
/// something, in order, their ends, and what crossed each (`[c * links + u]`).
fn by_link(crossed: &Crossed, rows: usize, col_of: &[usize], (a, b): (&[u32], &[u32])) -> (Vec<u32>, Vec<u32>, Vec<f64>) {
    let cols = col_of.iter().max().map_or(0, |c| c + 1);
    let mut slot = vec![u32::MAX; a.len()];
    crossed.links.iter().for_each(|&l| slot[l as usize] = 0);
    let used: Vec<u32> = (0..a.len() as u32).filter(|&l| slot[l as usize] == 0).collect();
    used.iter().enumerate().for_each(|(u, &l)| slot[l as usize] = u as u32);
    let (links, mut mol) = (used.len(), vec![0.0; cols * used.len()]);
    for (&l, m) in crossed.links.iter().zip(crossed.amount.chunks_exact(rows)) {
        let u = slot[l as usize] as usize;
        col_of.iter().enumerate().for_each(|(r, &c)| mol[c * links + u] += m[r]);
    }
    let ends = |v: &[u32]| used.iter().map(|&l| v[l as usize]).collect();
    (ends(a), ends(b), mol)
}

/// The work `bedwork`, J, of the liquid leaving node `i` (a share `out`) on its load: it drops what is over capacity,
/// else picks up loose solids up to it (at most all that lies loose). Returns the work left to break bonded ground: the
/// share of the capacity its load leaves free (the cover effect, Sklar and Dietrich 2004).
fn load(w: &World, s: &Scratch, i: usize, q: &mut [f64], (rows, sol): (std::ops::Range<usize>, &[usize]), bedwork: f64, out: f64) -> f64 {
    let held: f64 = rows.clone().zip(sol).map(|(r, &k)| q[r] * s.sp[k].v[0]).sum();
    let capv = w.cfg.carry * bedwork * bare_share(w, i);
    if out <= 0.0 {
        return 0.0;
    }
    if held > capv {
        rows.for_each(|r| q[r] *= capv / held);
        return 0.0;
    }
    let avail: f64 = sol.iter().map(|&k| solid(w, s, k, i) * s.sp[k].v[0]).sum();
    let f = if avail > 0.0 { ((capv - held) / avail).min(1.0) } else { 0.0 };
    rows.zip(sol).for_each(|(r, &k)| q[r] += solid(w, s, k, i) * f);
    if capv > 0.0 { bedwork * (1.0 - (held + f * avail) / capv).max(0.0) } else { 0.0 }
}

/// Break bonded ground of node `i` into loose by `bedwork` over bond strength (`wear`), each substance by its share of
/// the bed's volume, at most all of it: moves from `Class::Bed` gathered in `broke`. Its heat was already the node's.
fn wear(w: &World, s: &mut Scratch, i: usize, bedwork: f64, broke: &mut Vec<Move>) {
    let n = w.mesh.cells().n();
    for bi in 0..s.state.bedk.len() {
        let (k, bv) = (s.state.bedk[bi], s.state.bed[i].vol);
        let b = w.store.amounts(Class::Bed)[k * n + i];
        if b > 0.0 && bv > 0.0 {
            let v = w.cfg.wear * bedwork * (b * s.sp[k].v[0] / bv) / w.chem.subs[k].strength.max(STRENGTH_MIN);
            let go = Move { from: Node::new(Class::Bed, i), to: Node::ground(i), k: k as u16, mol: (v / s.sp[k].v[0]).min(b), heat: 0.0 };
            let area = w.mesh.cells().area()[i];
            let (m, dz) = (go.mol, go.mol * s.sp[k].v[0] / area);
            broke.push(go);
            s.state.bed[i].vol -= m * s.sp[k].v[0];
            s.add_loose(k, false);
            s.note(k, i, m, area);
            lift(s, i, [-dz, -dz]);
        }
    }
}
