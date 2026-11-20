//! Liquid on and in the loose ground at each liquid step: the stores of the bed drain to the channels inside the cells
//! (`drain::springs`); the liquid under the surface moves (`drain::under`); what may run off runs down the terrain's
//! network to the bodies of standing liquid, which stand each at one level, in one pass (`runoff::surface`), doing
//! work on its bed; and each body mixes its heat (`basin::mix`).

use super::{drain::{land_drains, springs, under, OffLand}, land::{cells_state, t_now, wet, NONE}, parts::{Leaves, LAND}, potential::apply, range, runoff::surface, Scratch};
use crate::{store::{Class, LinkFlux}, units::T_FLOOR, World};

/// The liquid under the surface of every ground node, as the laws under it move it (`drain`): of each liquid `liq[q]`,
/// `mol[q * n + i]` (its liquid share, `land::wet`, which leaves what the exchange sites hold of a mineral; all of a
/// dissolved gas), its volume `vol[i]`, m³ (dissolved gas adds none), the pore space of the ground that holds it
/// `pd[i]`, as a depth over that ground, m, the area of that ground `area[i]`, m², and the temperature it leaves at
/// `t[i]`, K. It lies in the ground the laws under the surface reach (`soil_share`): of a cell of parts (1.1) its
/// land (`Piece::liquid`).
pub(super) struct Mobile {
    pub(super) mol: Vec<f64>,
    pub(super) vol: Vec<f64>,
    pub(super) pd: Vec<f64>,
    pub(super) area: Vec<f64>,
    pub(super) t: Vec<f64>,
}

/// The share of cell `i`'s area whose ground the laws under the surface reach (laws.md 1.2): of a cell of parts its
/// land's, for the loose ground under its liquid lies with its pores full and the store under that is not drained;
/// none of a cell under a body that is one node; else all of it.
pub(super) fn soil_share(s: &Scratch, i: usize) -> f64 {
    match s.mixed(i) {
        Some(m) if super::parts::has(s, i) => m.piece[LAND].share,
        _ if s.basin.is[i] => 0.0,
        _ => 1.0,
    }
}

/// The share of a link between cells `i` and `j` that the laws under the surface pass (`soil_share`): each end's share,
/// as the ground they reach on each side meets the other's.
pub(super) fn reached(s: &Scratch, i: usize, j: usize) -> f64 {
    soil_share(s, i) * soil_share(s, j)
}

/// The liquid under the surface of every ground node (`Mobile`), of the liquids `liq`; what boils below the coldest
/// node, `tmin`, is gas.
pub(super) fn mobile(w: &World, s: &Scratch, liq: &[usize], tmin: f64) -> Mobile {
    let (n, area) = (w.mesh.cells().n(), w.mesh.cells().area());
    let mut m = Mobile { mol: vec![0.0; liq.len() * n], vol: vec![0.0; n], pd: vec![0.0; n], area: vec![0.0; n], t: vec![0.0; n] };
    for i in (0..n).filter(|&i| soil_share(s, i) > 0.0) {
        let land = s.mixed(i).filter(|_| super::parts::has(s, i)).map(|x| &x.piece[LAND]);
        for (q, &k) in liq.iter().enumerate() {
            let gas = s.sp[k].boil <= tmin;
            let a = match land {
                Some(l) if gas => l.amt[k],
                Some(l) => l.liquid(k, &s.sp[k]),
                None => w.store.amounts(Class::Ground)[k * n + i] * if gas { 1.0 } else { wet(s, k, i) },
            };
            m.mol[q * n + i] = a;
            m.vol[i] += if gas { 0.0 } else { a * s.sp[k].v[1] };
        }
        (m.pd[i], m.area[i], m.t[i]) = match land {
            Some(l) => (l.pd, area[i] * l.share, l.tg.max(T_FLOOR)),
            None => (s.pd[i], area[i], t_now(w, s, i)),
        };
    }
    m
}

/// Book `fluxes` through the store, and bring the ground nodes' state up to date for what changed (`Scratch::note`;
/// a node of parts books it on them, what leaves it leaving the part `leaves` names, `parts::to_liquid`).
/// Their potential energy goes by the heights their matter leaves from (`potential::apply`).
pub(super) fn book(w: &mut World, s: &mut Scratch, fluxes: &[LinkFlux], leaves: Leaves) {
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
    let heat: Vec<f64> = cells.iter().map(|&i| w.store.heat(Class::Ground)[i as usize]).collect();
    apply(w, s, fluxes, &[]);
    super::parts::to_liquid(w, s, &cells, (&ks, &before), &heat, leaves);
    for (q, &k) in ks.iter().enumerate() {
        for (&i, &was) in cells.iter().zip(&before[q * cells.len()..(q + 1) * cells.len()]) {
            let now = amount(w, k, i);
            if now != was {
                s.note(k, i as usize, now - was, w.mesh.cells().area()[i as usize]);
            }
        }
    }
    // A node on a melting plateau finds its molten share again from its heat (`cells_state`): what moved of the
    // plateau's substance was liquid, so the share of what stays is not the share it had; and a mixed node, from its
    // parts.
    let on: Vec<u32> = cells.into_iter().filter(|&i| s.state.pk[i as usize] != NONE || super::parts::has(s, i as usize)).collect();
    cells_state(w, s, &on, false);
}

/// One liquid step of `dt` seconds (the module's doc): after the runoff's pass the liquid's top and deep swap liquid
/// through the thermocline (`parts::mix`) and each body mixes its heat (`basin::mix`), so the bodies' tops end the step
/// mixed; last, the gases of the air dissolve in the liquid where it lies (`air::dissolve`).
pub(super) fn liquid(w: &mut World, s: &mut Scratch, dt: f64) {
    let (tmin, tmax) = range(&s.state.tg);
    let liq = s.liquids(&w.chem, (tmin, tmax), true);
    let slope = slopes(w, s);
    let runs = springs(w, s, tmin, &slope, dt);
    let mut off = OffLand::new(w);
    under(w, s, &liq, tmin, dt, &mut off);
    land_drains(w, s, (&liq, tmin), &slope, dt);
    surface(w, s, (&liq, tmin), (&slope, &runs, off), dt);
    let mixed = super::parts::mix(w, s, dt);
    cells_state(w, s, &mixed, false);
    super::basin::mix(w, s);
    super::air::dissolve(w, s, dt);
}

/// The slope of the surface of each ground node down its steepest way (`Level::steepest`) to the lower neighbours (by
/// more than `still`), 0 for none: the slope its hillsides drain down to the channels inside it. The surface is the
/// cell's mean ground, or the liquid standing over its pores if higher; of a cell of parts or under a body, its solid
/// ground and theirs: the slope its land drains down to its liquid.
fn slopes(w: &World, s: &Scratch) -> Vec<f64> {
    let (cells, still) = (w.mesh.cells(), w.cfg.phys.still);
    let surface = |i: usize| (s.head[i] - s.pd[i]).max(s.z[i]);
    let drop = |i: usize, k: usize, z: &dyn Fn(usize) -> f64| Some(z(i) - z(cells.around(i).1[k] as usize)).filter(|&d| d > still);
    let at = |i: usize| if s.basin.is[i] || super::parts::has(s, i) { cells.steepest(i, |k| drop(i, k, &|j| s.z[j])) } else { cells.steepest(i, |k| drop(i, k, &surface)) };
    crate::par::map_n(cells.n(), crate::par::UNCOUNTED, |i| at(i).map_or(0.0, |way| way.0))
}
