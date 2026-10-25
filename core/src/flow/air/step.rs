//! One step of the air: radiation, the exchange with the ground (evaporation and rain) and the wind, solved together
//! by backward Euler, so a long step gives the world of short ones (laws.md 1.4). Each round finds every ground node's
//! exchange and its slopes (`cell`), then solves the air level (`newton`); what the last round found is booked whole
//! (`book`), so the books are exact whether or not the rounds converged.

use super::super::scratch::may_evaporate;
use super::super::{range, Scratch};
use super::book::book;
use super::cell::{cell, fixed, Ctx, FIXED_CELL};
use super::exchange::dissolve;
use super::newton::{alone, far, gather, solve, Drive, Wind};
use super::radiation::{declination, insolation};
use super::lerp;
use crate::{ground::heat_above_floor, store::Class, survey::Draws, World, AIR};

/// The rounds stop when no air node's temperature moves by more than this from one round to the next, K,
const STILL_K: f64 = 5e-2;
/// and no gas of an air node by more than this share of the node's gas,
const STILL_SHARE: f64 = 2e-4;
/// or after this many rounds.
const ROUNDS: usize = 16;
/// A ground node keeps its row when the air it sees moved less than this since the round its row stands at, K,
const CALM_K: f64 = 1e-2;
/// and each active gas less than this share of its amount.
const CALM_SHARE: f64 = 1e-4;
/// A round leaves each gas of a node at least this share of what it held, a bound on that one unknown alone.
const KEEP: f64 = 0.5;
/// The saturation tables reach this far above the warmest node at a step's start, K: no node warms more in a step.
const SAT_REACH: f64 = 60.0;
/// A gas making up less than this share of the air is a trace (`newton::alone`).
const TRACE_AIR: f64 = 1e-7;

/// A gas that may leave the ground or condense: its index in `Scratch::gas`, its substance, whether it may evaporate or
/// condense, and (one that may condense, more than a trace) its place among the gases solved with the heat in each
/// round; the others are solved once after the rounds (`newton::alone`).
#[derive(Clone, Copy)]
pub(super) struct Active {
    pub(super) gi: usize,
    pub(super) k: usize,
    pub(super) up: bool,
    pub(super) cond: bool,
    pub(super) field: Option<usize>,
}

/// A ground node's row (`cell`): the heat it gives its air node, J, and its slope with the air's temperature, J/K; the
/// heat to space through the air, J; the sunlight it absorbs, J; its end temperature, K; the light per unit of its
/// bodies' capture area, J/m²; the light set aside for them, W/m²; the heat it gives its bodies' node, J; the reflectance
/// of its surface at its end state; then `PER_GAS` numbers per active gas.
pub(super) const FIXED: usize = 9;
/// Per active gas: what evaporates, mol, and its slopes with the column's gas and the air's temperature; what rains and
/// its two slopes; the slope of the heat the node gives its air with the column's gas, J/mol; the fusion enthalpy a mol
/// of its rain brings the ground, J/mol.
pub(super) const PER_GAS: usize = 8;
/// The numbers of a row with `nx` active gases: `FIXED`, `PER_GAS` per gas, then the air the row stands at (`keep`):
/// the temperature, K, and each active gas, mol (`seen_air`).
fn width(nx: usize) -> usize {
    FIXED + PER_GAS * nx + 1 + nx
}

/// The air during the rounds: per node the temperature, K; per gas and node (`[gas * nodes + node]`) the amount, mol.
#[derive(Clone)]
pub(super) struct Iterate {
    pub(super) t: Vec<f64>,
    pub(super) x: Vec<f64>,
}

/// The air at the step's start with the gas of the bodies' nodes, which comes up into each cell's air node over the
/// step (laws.md 1.5): per gas and node (`[gas * nodes + node]`) the amount, mol; per node its heat less the lift of
/// that gas, J, and the lift, J.
pub(super) struct Start {
    pub(super) x: Vec<f64>,
    pub(super) e: Vec<f64>,
    pub(super) lift: Vec<f64>,
}

impl Start {
    fn of(w: &World, s: &Scratch) -> Start {
        let (n, na, g, parent) = (w.mesh.cells().n(), w.mesh.level(AIR).n(), w.cfg.gravity, w.mesh.cells().parent());
        let (mut all, mut lift) = (w.store.amounts(Class::Air).to_vec(), vec![0.0; na]);
        for (at, &m) in w.store.amounts(Class::Bodies).iter().enumerate().filter(|e| *e.1 != 0.0) {
            let (k, i, j) = (at / n, at % n, parent[at % n] as usize);
            all[k * na + j] += m;
            lift[j] += m * s.sp[k].mm * s.view().lift(i, j, g);
        }
        let x = s.state.gas.iter().flat_map(|&k| all[k * na..(k + 1) * na].iter().copied()).collect();
        Start { x, e: w.store.heat(Class::Air).iter().zip(&lift).map(|(e, l)| e - l).collect(), lift }
    }
}

/// The air's step of `dt` seconds: gases dissolve (from the air as it is), then radiation, evaporation, rain, the wind
/// and the heat the bodies draw from the ground (`draws`) together, in rounds.
pub(crate) fn step(w: &mut World, s: &mut Scratch, dt: f64, draws: &Draws) {
    let (tmin, tmax) = range(&s.state.tg);
    let henry: Vec<(usize, usize)> = s.state.gas.iter().copied().enumerate().filter(|&(_, k)| s.sp[k].boil < tmin && w.chem.subs[k].solubility > 0.0).collect();
    dissolve(w, s, dt, &henry, (tmin, tmax));
    let act = active(w, s, &henry);
    let n = w.mesh.cells().n();
    // The sun at the end of the step, as backward Euler takes it (and as the state at the step's end then shows).
    let decl = declination(w.cfg.tilt, w.cfg.calendar().season(w.time + dt as u64));
    let (start, na) = (Start::of(w, s), w.mesh.level(AIR).n());
    // The rounds start from the air without the bodies' gas, as the last step left it near saturation: with the vapour
    // the bodies gave off since, the first round would rain out far more than there is and the second take it back.
    let mut it = Iterate { t: s.ta.clone(), x: s.state.gas.iter().flat_map(|&k| w.store.amounts(Class::Air)[k * na..(k + 1) * na].iter().copied()).collect() };
    let drive = Drive::of(w, s, &start.x, dt);
    let (mut ts, mut alb, mut rows, mut wind) = (s.state.tg.clone(), s.alb.clone(), std::mem::take(&mut s.rad), Wind::default());
    let width = width(act.len());
    rows.resize(n * width, 0.0);
    let (fixed, far) = (fixed(w, s, &act), far(s));
    let room: Vec<f64> = (0..n).map(|i| if draws.any(i) { heat_above_floor(w.store.heat(Class::Ground)[i], w.heat_cap_ground(i)) } else { 0.0 }).collect();
    for round in 0.. {
        pass(&Ctx { w, s, it: &it, act: &act, ts: &ts, alb: &alb, fixed: &fixed, draws, room: &room, dt, decl }, &mut rows, (width, round > 0));
        ts.iter_mut().zip(alb.iter_mut()).zip(rows.chunks_exact(width)).for_each(|((t, a), row)| (*t, *a) = (row[4], row[8]));
        let g = gather(w, s, &it, &act, (&rows, width), far);
        let (next, coef) = solve(w, s, (&act, &drive), (&start, &g), &it, dt, &mut wind);
        let (moved_t, moved_x) = moved(&next, &it);
        if (moved_t < STILL_K && moved_x < STILL_SHARE) || round + 1 == ROUNDS {
            alone(w, s, (&act, &drive.links), (&start, &g), &it, &coef, &mut wind);
            book(w, s, (&act, &g.rained), (&rows, width), &wind, (&it, &start), dt);
            let given: Vec<f64> = rows.chunks_exact(width).map(|row| row[7]).collect();
            draws.owe(w, |i| ts[i], &given, dt);
            break;
        }
        let x = next.x.iter().zip(&it.x).map(|(&u, &v)| u.max(KEEP * v)).collect();
        it = Iterate { t: next.t, x };
    }
    s.rad = rows;
}

/// How far the solve moved the air from `it` to `next`: the most of any node's temperature, K, and the most of any
/// gas of a node, as a share of the node's gas.
fn moved(next: &Iterate, it: &Iterate) -> (f64, f64) {
    let (na, ng) = (it.t.len(), it.x.len() / it.t.len().max(1));
    let t = next.t.iter().zip(&it.t).map(|(a, b)| (a - b).abs()).fold(0.0, f64::max);
    let x = (0..na).map(|j| (0..ng).map(|q| (next.x[q * na + j] - it.x[q * na + j]).abs()).fold(0.0, f64::max) / (0..ng).map(|q| next.x[q * na + j]).sum::<f64>()).fold(0.0, f64::max);
    (t, x)
}

/// The gases that leave the ground or may condense (not those of `henry`, which dissolve), with their saturation
/// pressures up to the warmest ground or air node plus `SAT_REACH` (`Scratch::sat`).
fn active(w: &World, s: &mut Scratch, henry: &[(usize, usize)]) -> Vec<Active> {
    let tmax = range(&s.state.tg).1;
    let hi = tmax.max(range(&s.ta).1) + SAT_REACH;
    s.sat.resize(w.chem.ns(), Default::default());
    let (na, all) = (w.mesh.level(AIR).n(), w.store.amounts(Class::Air).iter().sum::<f64>());
    let (mut act, mut fields) = (Vec::new(), 0);
    for (gi, &k) in s.state.gas.iter().enumerate() {
        let up = s.state.loose.contains(&k) && may_evaporate(w, k, tmax) && !henry.iter().any(|h| h.1 == k);
        if up || s.cond[gi] {
            s.sat[k].fit(&w.chem, k, hi);
            let rounds = s.cond[gi] && w.store.amounts(Class::Air)[k * na..(k + 1) * na].iter().sum::<f64>() >= TRACE_AIR * all;
            act.push(Active { gi, k, up, cond: s.cond[gi], field: rounds.then_some(fields) });
            fields += rounds as usize;
        }
    }
    act
}

/// Every ground node's row (`cell`) from the air of the round, in rows (in parallel, with `threads`). After the first
/// round (`later`), a node whose air is calm keeps its row, carried to the round's air by its own slopes (`keep`).
fn pass(cx: &Ctx, rows: &mut [f64], (width, later): (usize, bool)) {
    let (cfg, lat, per) = (&cx.w.cfg, cx.w.mesh.cells().lat(), FIXED_CELL + cx.act.len());
    // Each task keeps the last latitude and its sunlight: neighbouring cells often share a latitude (a row of the
    // square map), and the sunlight is the same function of it; and room for the air a node sees.
    crate::par::rows(rows, width, || (f64::NAN, 0.0, vec![0.0; 1 + cx.act.len()]), |(lat_seen, q, seen), i, row| {
        if lat[i] != *lat_seen {
            (*lat_seen, *q) = (lat[i], insolation(cfg.sun, lat[i].to_radians(), cx.decl));
        }
        seen_air(cx, i, seen);
        if later && keep(row, seen, cx.fixed[i * per]) {
            return;
        }
        cell(cx, i, *q, row);
        let at = row.len() - seen.len();
        row[at..].copy_from_slice(seen);
    });
}

/// The air ground node `i` sees in the round, into `out`: its temperature, K, then each active gas, mol.
fn seen_air(cx: &Ctx, i: usize, out: &mut [f64]) {
    let na = cx.w.mesh.level(AIR).n();
    out[0] = lerp(cx.s, &cx.it.t, i);
    for (x, ac) in cx.act.iter().enumerate() {
        out[1 + x] = lerp(cx.s, &cx.it.x[ac.gi * na..(ac.gi + 1) * na], i);
    }
}

/// Whether a ground node keeps its `row` in a later round: where the air it sees, `seen`, moved less than `CALM_K` and
/// each active gas less than `CALM_SHARE` of its amount since the air the row stands at, and no amount it gives ends
/// below zero. A kept row is carried to `seen` to first order by its own slopes (`over` the share of the air it sees
/// over it), so it stays the node's exchange linearised about the air of the round it was made in, and its slopes still
/// enter the Newton step (an inexact Newton step, Dembo, Eisenstat and Steihaug 1982). The calm test bounds each
/// round's move and the stored air moves at each keep, so a row may drift up to about (rounds - 1) x `CALM_K` from the
/// air it was made at. The numbers the Newton step does not read (the heat to space `row[2]`, the end temperature
/// `row[4]`, the heat given to bodies `row[7]` and the end reflectance `row[8]`) stay at the made-at air: an error of
/// about 4 sigma T^3 per 0.01 K of drift, a few hundredths of a W/m^2, and the books stay exact.
fn keep(row: &mut [f64], seen: &[f64], over: f64) -> bool {
    let (at, nx) = (row.len() - seen.len(), seen.len() - 1);
    let d_tl = seen[0] - row[at];
    if d_tl.abs() >= CALM_K || (0..nx).any(|x| (seen[1 + x] - row[at + 1 + x]).abs() > CALM_SHARE * seen[1 + x]) {
        return false;
    }
    // Gas `x`'s evaporation and rain, mol, and its share of the heat to the air, J, carried to `seen`.
    let carried = |row: &[f64], x: usize| {
        let (o, d_col) = (FIXED + PER_GAS * x, (seen[1 + x] - row[at + 1 + x]) * over);
        (row[o] + row[o + 1] * d_col + row[o + 2] * d_tl, row[o + 3] + row[o + 4] * d_col + row[o + 5] * d_tl, row[o + 6] * d_col)
    };
    if (0..nx).map(|x| carried(row, x)).any(|(m, r, _)| m < 0.0 || r < 0.0) {
        return false;
    }
    let mut heat = row[1] * d_tl;
    for x in 0..nx {
        let ((m, r, h), o) = (carried(row, x), FIXED + PER_GAS * x);
        (row[o], row[o + 3], heat) = (m, r, heat + h);
    }
    row[0] += heat;
    row[at..].copy_from_slice(seen);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_kept_row_is_linearised_about_the_air_it_was_made_at() {
        // One gas: [m, m_col, m_tl, r, r_col, r_tl, h_col, lf].
        let gas = [3.0, 0.02, 0.1, 2.0, 0.01, -0.05, 4.0, 1e3];
        let (a0, over) = ([280.0, 1000.0], 0.8);
        let mut made = vec![0.0; width(1)];
        made[..FIXED].copy_from_slice(&[500.0, 30.0, 7.0, 8.0, 281.0, 9.0, 1.0, 2.0, 0.3]);
        made[FIXED..FIXED + PER_GAS].copy_from_slice(&gas);
        made[FIXED + PER_GAS..].copy_from_slice(&a0);
        let (mut row, a1, a2) = (made.clone(), [280.004, 1000.05], [280.008, 1000.09]);
        assert!(keep(&mut row, &a1, over) && keep(&mut row, &a2, over));
        let (d_tl, d_col) = (a2[0] - a0[0], (a2[1] - a0[1]) * over);
        let near = |got: f64, want: f64| assert!((got - want).abs() < 1e-12, "{got} != {want}");
        near(row[FIXED], gas[0] + gas[1] * d_col + gas[2] * d_tl);
        near(row[FIXED + 3], gas[3] + gas[4] * d_col + gas[5] * d_tl);
        near(row[0], 500.0 + 30.0 * d_tl + gas[6] * d_col);
        assert_eq!(row[FIXED + PER_GAS..], a2);
        // A move of the temperature or of the gas alone, past its bound, is refused and leaves the row whole.
        let mut far = made.clone();
        assert!(!keep(&mut far, &[280.02, 1000.0], over) && far == made);
        assert!(!keep(&mut far, &[280.0, 1000.2], over) && far == made);
        // A rain that the carry would push below zero is refused: 1e-4 - 0.05 x 0.004 < 0.
        far[FIXED + 3] = 1e-4;
        let thin = far.clone();
        assert!(!keep(&mut far, &[280.004, 1000.0], over) && far == thin);
        far[FIXED + 3] = -1.0;
        assert!(!keep(&mut far, &[280.0, 1000.0], over));
    }
}
