//! Liquid on and in the loose ground at each liquid step: the stores of the bed drain to the channels inside the cells
//! (`drain::springs`); what may run off runs down the cells in one pass in the order of a priority flood
//! (`solve::downhill`), doing work on its bed; the liquid under the surface moves (`drain::under`); and the basin stands
//! at one level (`basin::level`).

use super::{bare_share, drain::{springs, under}, follow, land::{lift, solid, stand, wet, NONE}, potential::apply, range, year_weight, Scratch};
use crate::solve::sweep::HALF_WAY;
use crate::{math::one_minus_exp, solve::downhill::{pass, Adjacency, Crossed}, store::{Class, LinkFlux, Move, Node}, units::T_FLOOR, World};

/// Least bond strength wear divides by.
const STRENGTH_MIN: f64 = 0.05;

/// The mobile liquid of every ground node: of each liquid `liq[q]`, `mol[q * n + i]` (its liquid share; all of a
/// dissolved gas), and its volume `vol[i]`, m³ (dissolved gas adds none).
pub(super) struct Mobile {
    pub(super) mol: Vec<f64>,
    pub(super) vol: Vec<f64>,
}

/// The mobile liquid of every ground node, of the liquids `liq`; what boils below the coldest node, `tmin`, is gas.
pub(super) fn mobile(w: &World, s: &Scratch, liq: &[usize], tmin: f64) -> Mobile {
    let n = w.mesh.cells().n();
    let mut m = Mobile { mol: vec![0.0; liq.len() * n], vol: vec![0.0; n] };
    for (q, &k) in liq.iter().enumerate() {
        let gas = s.sp[k].boil <= tmin;
        for i in 0..n {
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
    let before: Vec<f64> = ks.iter().flat_map(|&k| w.store.amounts(Class::Ground)[k * n..(k + 1) * n].iter().copied()).collect();
    apply(w, s, fluxes, &[]);
    for (q, &k) in ks.iter().enumerate() {
        for (i, (&now, &was)) in w.store.amounts(Class::Ground)[k * n..(k + 1) * n].iter().zip(&before[q * n..(q + 1) * n]).enumerate().filter(|p| p.1 .0 != p.1 .1) {
            s.note(k, i, now - was, w.mesh.cells().area()[i]);
        }
    }
}

/// One liquid step of `dt` seconds.
pub(super) fn liquid(w: &mut World, s: &mut Scratch, dt: f64) {
    let (tmin, tmax) = range(&s.state.tg);
    let liq = s.liquids(&w.chem, (tmin, tmax), true);
    let slope = slopes(w, s);
    let runs = springs(w, s, tmin, &slope, dt);
    surface(w, s, (&liq, tmin), (&slope, &runs), dt);
    under(w, s, &liq, tmin, dt);
    super::basin::level(w, s, &liq, tmin);
}

/// The slope of the surface of each ground node down its steepest way (`Level::steepest`) to the lower neighbours (by
/// more than `still`), 0 for none.
fn slopes(w: &World, s: &Scratch) -> Vec<f64> {
    let (cells, still) = (w.mesh.cells(), w.cfg.phys.still);
    let drop = |i: usize, k: usize| Some(stand(s, i) - stand(s, cells.around(i).1[k] as usize)).filter(|&d| d > still);
    crate::par::map_n(cells.n(), |i| cells.steepest(i, |k| drop(i, k)).map_or(0.0, |way| way.0))
}

/// The liquid that may run off (what stands, what came up from the store, and the pores' drainage to the channels, a
/// share 1 - exp(-`seep` × slope × time / (`porosity` × `hill`))) moves down the cells in one pass (`solve::downhill`;
/// laws.md 1.2), with its heat; lost head heats what runs out and does work on the bed (`load`, `wear`).
fn surface(w: &mut World, s: &mut Scratch, (liq, tmin): (&[usize], f64), (slope, runs): (&[f64], &[f64]), dt: f64) {
    let (n, area, ph, g) = (w.mesh.cells().n(), w.mesh.cells().area(), w.cfg.phys, w.cfg.gravity);
    let mob = mobile(w, s, liq, tmin);
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
    let (mut sent, mut worn, mut outs) = (vec![0.0; n], vec![0.0; n], Vec::new());
    // A land cell, as `World::is_land` says: its solid surface above 0 m (Flow's state of it).
    let (land, mut off_land): (Vec<bool>, Vec<f64>) = (s.z.iter().map(|&z| z > 0.0).collect(), vec![0.0; n]);
    let mut crossed = Crossed::default();
    let (ww, ss) = (&*w, &*s);
    pass(&order, &adj, &mut q, &mut crossed, |i, q, share| {
        let (zi, hd, sv) = (ss.z[i], head(q, i), q[i * width + vol]);
        if sv <= 0.0 {
            return;
        }
        // Each lower neighbour: the drop toward it, how far the head may fall toward it, and the fall of the head.
        // The surface of a cell whose liquid stands is its head; of one its liquid passes on through, its ground (a
        // neighbour still to come holds the head back only by what it keeps). Liquid passing on never goes back to a
        // neighbour already passed; it takes the steepest way down (`Level::steepest`), and on level ground the flood's
        // way out.
        outs.clear();
        let (top, mut way) = (if order.stands[i] { hd } else { zi }, None);
        for (k, j) in cells.around(i).1.iter().map(|&j| j as usize).enumerate() {
            let (hj, passed) = (head(q, j), order.before(j, i));
            let (from, to) = (if passed { top } else { hd }, if passed || order.stands[j] { hj } else { ss.z[j] });
            let (down, room) = (from - to > ph.still, if passed { HALF_WAY * (from - to) } else { hd - to.max(zi) });
            if down && j == order.outlet[i] as usize {
                way = Some([(k, 1.0), (k, 0.0)]);
            }
            outs.push((down.then_some((top - to).max(0.0)), room, (hd - hj).max(0.0)));
        }
        let Some(ways) = cells.steepest(i, |k| outs[k].0).map(|w| w.1).or(way) else { return };
        let (mut out, mut drop) = (0.0, 0.0);
        for (k, f) in ways.into_iter().filter(|p| p.1 > 0.0) {
            share[k] = (sv * f).min(area[i] * outs[k].1) / sv;
            (out, drop) = (out + share[k], drop + share[k] * outs[k].2);
        }
        // The node's heat capacity and fusion enthalpy with what came in. Liquid that came in holds its fusion enthalpy
        // as it passes, whatever the node's own phase (its heat brought it); a load holds none.
        let (mut c, mut held) = (ss.cap[i], ss.latent[i].held);
        let (cell, was) = (i * width..(i + 1) * width, &q0[i * width..(i + 1) * width]);
        for (r, &k) in ks.iter().enumerate().filter(|p| q[cell.start + p.0] != was[p.0]) {
            let d = q[cell.start + r] - was[r];
            (c, held) = (c + d * ss.sp[k].cp, held + if r < nl { d * ss.sp[k].lf } else { 0.0 });
        }
        let q = &mut q[cell];
        // The temperature of the node mixed with what came in, at which what runs out leaves: off a melting plateau its
        // energy over its fusion enthalpy and heat capacity; on one, what it gains or loses on the plateau's
        // temperature melts or freezes the plateau's substance first (`FlowView::fused`), and only the rest warms or
        // cools it. So the liquid passing on leaves at a temperature the node pays for, and what stays keeps it.
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
    });
    // The crossings as a link flux: each substance once (liquid and load rows of one substance share it), then the heat.
    let kinds: Vec<u16> = liq.iter().chain(sol.iter().filter(|k| !liq.contains(k))).map(|&k| k as u16).collect();
    let col_of: Vec<usize> = ks.iter().map(|&k| kinds.iter().position(|&x| x as usize == k).unwrap_or(0)).chain([kinds.len()]).collect();
    let (a, b, mol) = by_link(&crossed, rows + 2, &col_of, (cells.a(), cells.b()));
    let at = kinds.len() * a.len();
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
