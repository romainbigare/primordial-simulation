//! Liquid on and in the loose ground at each liquid step: the stores of the bed drain to the channels inside the cells
//! (`drain::springs`); what may run off runs down the cells in the order of their heads in one pass
//! (`solve::downhill`), doing work on its bed; then the liquid under the surface moves (`drain::under`).

use super::drain::{springs, under};
use super::land::{lift, solid, stand, wet, HALF_WAY};
use super::{bare_share, follow, range, year_weight, Scratch};
use crate::math::one_minus_exp;
use crate::solve::downhill::{pass, Adjacency, Crossed};
use crate::store::{apply, Class, LinkFlux, Move};
use crate::units::T_FLOOR;
use crate::World;

/// A node with no load and less carrying capacity than this does no work on its bed, m³.
const CARRY_MIN: f64 = 1.0;
/// Most of each loose solid picked up, and of each bonded substance worn, per call.
const MOST_TAKEN: f64 = 0.5;
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
    let n = w.grid.n();
    let mut m = Mobile { mol: vec![0.0; liq.len() * n], vol: vec![0.0; n] };
    for (q, &k) in liq.iter().enumerate() {
        let gas = s.sp[k].boil <= tmin;
        for i in 0..n {
            let a = w.ground[k * n + i] * if gas { 1.0 } else { wet(s, k, i) };
            m.mol[q * n + i] = a;
            m.vol[i] += if gas { 0.0 } else { a * s.sp[k].v[1] };
        }
    }
    m
}

/// Book `fluxes` and `moves` through the store (`store::apply`), and bring the state of the ground nodes up to date
/// for what that changed of their amounts (`Scratch::note`).
pub(super) fn book(w: &mut World, s: &mut Scratch, fluxes: &[LinkFlux], moves: &[Move]) {
    let n = w.grid.n();
    let mut ks: Vec<usize> = fluxes.iter().flat_map(|x| x.ks.iter()).chain(moves.iter().map(|m| &m.k)).map(|&k| k as usize).collect();
    ks.sort_unstable();
    ks.dedup();
    if ks.is_empty() {
        return;
    }
    let before: Vec<f64> = ks.iter().flat_map(|&k| w.ground[k * n..(k + 1) * n].iter().copied()).collect();
    apply(w, fluxes, moves, &[]);
    for (q, &k) in ks.iter().enumerate() {
        for (i, (&now, &was)) in w.ground[k * n..(k + 1) * n].iter().zip(&before[q * n..(q + 1) * n]).enumerate().filter(|p| p.1 .0 != p.1 .1) {
            s.note(k, i, now - was);
        }
    }
}

/// One liquid step of `dt` seconds.
pub(super) fn liquid(w: &mut World, s: &mut Scratch, dt: f64) {
    let (tmin, tmax) = range(&s.tg);
    let liq = s.liquids(&w.chem, (tmin, tmax), true);
    let slope = slopes(w, s);
    let runs = springs(w, s, tmin, &slope, dt);
    surface(w, s, (&liq, tmin), (&slope, &runs), dt);
    under(w, s, &liq, tmin, dt);
}

/// The slope of the surface of each ground node down to its steepest lower neighbour (by more than `still`), 0 for none.
fn slopes(w: &World, s: &Scratch) -> Vec<f64> {
    let cells = w.grid.mesh().cells();
    let lower = |i: usize, l: u32, j: u32| {
        let dh = stand(s, i) - stand(s, j as usize);
        if dh > w.cfg.phys.still { dh / cells.dist()[l as usize] } else { 0.0 }
    };
    (0..w.grid.n()).map(|i| (cells.around(i).0.iter().zip(cells.around(i).1)).map(|(&l, &j)| lower(i, l, j)).fold(0.0, f64::max)).collect()
}

/// The liquid that may run off, down the cells in order of their heads, highest first, in one pass (`solve::downhill`):
/// what stands over the pores, what came up from the store (`runs`, m³), and what the pores hold over their field
/// capacity drains to the channels inside the cell, `hill` from the divides, by Darcy's law down the slope of its
/// surface (`slope`): a share 1 - exp(-`seep` x slope x time / (`porosity` x `hill`)) of the pore liquid. Each node
/// passes it to its lower neighbours, more to the steeper (`split`, `still`), so it can cross the map in one pass while
/// depressions fill and spill; its head falls no further than to a neighbour still to come (or to its own bed), and
/// half way to one already passed, so heads never cross. What leaves carries the node's heat at its temperature then
/// (with what came in), and the head it loses heats the node. The power lost also carries loose solids, drops what is
/// over capacity and breaks bonded ground (`load`, `wear`).
fn surface(w: &mut World, s: &mut Scratch, (liq, tmin): (&[usize], f64), (slope, runs): (&[f64], &[f64]), dt: f64) {
    let (n, area, ph, g) = (w.grid.n(), s.area, w.cfg.phys, w.cfg.gravity);
    let mob = mobile(w, s, liq, tmin);
    // Rows that move together, side by side for each node (`q[i * width + r]`): the liquid that may run off, of each of
    // `liq`; the load, of each of `sol`; their heat, J; and the volume of the liquid, m³ (for the heads).
    let sol = s.solids(tmin);
    let ks: Vec<usize> = liq.iter().chain(&sol).copied().collect();
    let (nl, rows) = (liq.len(), ks.len());
    let (heat, vol, width) = (rows, rows + 1, rows + 2);
    let mut q = vec![0.0; width * n];
    for i in 0..n {
        let (pores, own) = (s.pd[i] * area, mob.vol[i] - runs[i]);
        let lp = own.min(pores);
        let free = if s.head[i] > s.z[i] { lp - ph.field * pores } else { 0.0 };
        let hill = if free > 0.0 && slope[i] > 0.0 { free.min(lp * one_minus_exp(ph.seep * slope[i] * dt / (ph.porosity * ph.hill))) } else { 0.0 };
        let off = if mob.vol[i] > 0.0 { (((own - pores).max(0.0) + runs[i] + hill) / mob.vol[i]).min(1.0) } else { 0.0 };
        for r in 0..nl {
            q[i * width + r] = mob.mol[r * n + i] * off;
        }
        q[i * width + vol] = mob.vol[i] * off;
    }
    let head = |q: &[f64], j: usize| s.z[j] + q[j * width + vol] / area;
    let mut order = std::mem::take(&mut s.order);
    order.update(&(0..n).map(|i| head(&q, i)).collect::<Vec<_>>());
    let (q0, cells) = (q.clone(), w.grid.mesh().cells());
    let adj = Adjacency { start: cells.start(), link: cells.adj_link(), other: cells.adj_cell(), a: cells.a() };
    let (mut power, mut sent, mut worn, mut outs) = (vec![0.0; n], vec![0.0; n], vec![0.0; n], Vec::new());
    let mut crossed = Crossed::default();
    let (ww, ss) = (&*w, &*s);
    pass(&order, &adj, &mut q, &mut crossed, |i, q, share| {
        let (zi, hd, sv) = (ss.z[i], head(q, i), q[i * width + vol]);
        if sv <= 0.0 {
            return;
        }
        // Each lower neighbour: its weight by slope, how far the head may fall toward it, and the drop.
        outs.clear();
        let mut wmax = 0.0f64;
        for e in adj.start[i] as usize..adj.start[i + 1] as usize {
            let (j, l) = (adj.other[e] as usize, adj.link[e] as usize);
            let (hj, mut o) = (head(q, j), [0.0; 3]);
            if hd - hj > ph.still {
                let room = if order.before(j, i) { HALF_WAY * (hd - hj) } else { hd - hj.max(zi) };
                o = [(hd - hj) / cells.dist()[l], room, hd - hj];
            }
            wmax = wmax.max(o[0]);
            outs.push(o);
        }
        // Neighbours much less steep than the steepest get nothing.
        let mut wsum = 0.0;
        for o in outs.iter_mut() {
            if o[0] < ph.split * wmax {
                o[0] = 0.0;
            }
            wsum += o[0];
        }
        if wsum <= 0.0 {
            return;
        }
        let (mut out, mut drop) = (0.0, 0.0);
        for (sh, o) in share.iter_mut().zip(&outs).filter(|p| p.1[0] > 0.0) {
            *sh = (sv * o[0] / wsum).min(area * o[1]) / sv;
            (out, drop) = (out + *sh, drop + *sh * o[2]);
        }
        // The node's temperature with what came in (no warmer than at the last `ground_state`).
        let (mut c, mut held) = (ss.cap[i], ss.latent[i].held);
        let (cell, was) = (i * width..(i + 1) * width, &q0[i * width..(i + 1) * width]);
        for (r, &k) in ks.iter().enumerate().filter(|p| q[cell.start + p.0] != was[p.0]) {
            let d = q[cell.start + r] - was[r];
            (c, held) = (c + d * ss.sp[k].cp, held + ss.fusion(k, i, d));
        }
        let q = &mut q[cell];
        let t = ((ww.e_ground[i] + q[heat] - held) / c.max(1.0)).min(ss.tg[i]).max(T_FLOOR);
        let kg = |q: &[f64], rs: std::ops::Range<usize>| rs.map(|r| q[r] * ss.sp[ks[r]].mm).sum::<f64>();
        // The work, on the bed below the liquid left standing (which hides it).
        let left = sv * (1.0 - out) / area;
        let bedwork = g * drop * kg(q, 0..nl) * ph.hide[1] / (ph.hide[1] + left);
        worn[i] = load(ww, ss, i, q, (nl..rows, &sol), bedwork, out);
        q[heat] = ks.iter().enumerate().map(|(r, &k)| q[r] * (ss.sp[k].cp * t + if r < nl { ss.sp[k].lf } else { 0.0 })).sum();
        power[i] = g * drop * kg(q, 0..rows);
        sent[i] = out * q[..nl].iter().sum::<f64>();
    });
    // The crossings as a link flux: each substance's column, then the heat (the volume is not booked).
    // Its columns: each substance once (one both liquid and loose solid moves in two rows), then the heat.
    let (mut kinds, mut col_of) = (Vec::new(), Vec::new());
    for &k in &ks {
        if !kinds.contains(&(k as u16)) {
            kinds.push(k as u16);
        }
        col_of.push(kinds.iter().position(|&x| x as usize == k).unwrap_or(0));
    }
    col_of.push(kinds.len());
    let (a, b, mol) = by_link(&crossed, rows + 2, &col_of, (cells.a(), cells.b()));
    let at = kinds.len() * a.len();
    book(w, s, &[LinkFlux { class: [Class::Ground; 2], a: &a, b: &b, ks: &kinds, mol: &mol[..at], heat: &mol[at..] }], &[]);
    s.order = order;
    let ema = year_weight(w, dt);
    for i in 0..n {
        (w.e_ground[i], w.potential) = (w.e_ground[i] + power[i], w.potential - power[i]);
        follow(&mut w.liquid_flux[i], (sent[i] / dt) as f32, ema);
        if worn[i] > 0.0 {
            wear(w, s, i, worn[i]);
        }
    }
}

/// The crossings of a pass (of `rows` rows) as the columns of a link flux, row `r` in column `col_of[r]` (for the rows
/// it names): the links that carried something in their order (so the store reads the cells in order), their ends (of
/// the links' ends `a` and `b`), and what crossed each (`[c * links + u]`; a link that crossed twice carries both).
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

/// The work of the liquid leaving node `i` on its bed (`bedwork`, J) for its load, the rows `rows` of its rows `q` (of
/// the solids `sol`), a share `out` of which leaves: it drops what is over its capacity, else picks up loose solids up
/// to it (no more than half of each). Returns the work left to break bonded ground (`wear`).
fn load(w: &World, s: &Scratch, i: usize, q: &mut [f64], (rows, sol): (std::ops::Range<usize>, &[usize]), bedwork: f64, out: f64) -> f64 {
    let held: f64 = rows.clone().zip(sol).map(|(r, &k)| q[r] * s.sp[k].v[0]).sum();
    let capv = w.cfg.carry * bedwork * bare_share(w, i);
    if (held <= 0.0 && capv < CARRY_MIN) || out <= 0.0 {
        return 0.0;
    }
    if held > capv {
        rows.for_each(|r| q[r] *= capv / held);
        return 0.0;
    }
    let avail: f64 = sol.iter().map(|&k| solid(w, s, k, i) * s.sp[k].v[0]).sum();
    let f = if avail > 0.0 { ((capv - held) / avail).min(MOST_TAKEN) } else { 0.0 };
    for (r, &k) in rows.zip(sol) {
        q[r] += solid(w, s, k, i) * f;
    }
    bedwork
}

/// Break bonded ground of node `i` into loose by `bedwork` over bond strength (`wear`), each
/// substance by its share of the bed's volume, no more than half of it. What breaks off was in
/// the bed's skin, whose heat is the node's: it keeps it, and joins the loose ground with its heat
/// capacity (`Scratch::put`); the bed's surface falls by its volume.
fn wear(w: &mut World, s: &mut Scratch, i: usize, bedwork: f64) {
    let n = w.grid.n();
    for bi in 0..s.bedk.len() {
        let (k, bv) = (s.bedk[bi], s.bed[i].vol);
        let b = w.bed[k * n + i];
        if b > 0.0 && bv > 0.0 {
            let v = w.cfg.wear * bedwork * (b * s.sp[k].v[0] / bv) / w.chem.subs[k].strength.max(STRENGTH_MIN);
            let m = (v / s.sp[k].v[0]).min(MOST_TAKEN * b);
            let dz = m * s.sp[k].v[0] / s.area;
            w.bed[k * n + i] -= m;
            s.bed[i].vol -= m * s.sp[k].v[0];
            s.add_loose(k, false);
            s.put(&mut w.ground, k, i, m);
            lift(s, i, [-dz, -dz]);
        }
    }
}
