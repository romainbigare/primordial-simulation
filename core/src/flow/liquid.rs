//! Liquid on and in the loose ground at each liquid step: the stores of the bed drain to the channels inside the cells
//! (`drain::springs`); the liquid under the surface moves (`drain::under`); what may run off runs down the terrain's
//! network to the bodies of standing liquid, which stand each at one level, in one pass (`runoff::surface`), doing
//! work on its bed; and each body mixes its heat (`basin::mix`).

use super::{drain::{springs, under}, land::{cells_state, wet, NONE}, potential::apply, range, runoff::surface, Scratch};
use crate::{store::{Class, LinkFlux}, World};

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

/// Book `fluxes` through the store, and bring the ground nodes' state up to date for what changed (`Scratch::note`;
/// a node of parts books it on them, `parts::to_liquid`).
/// Their potential energy goes by the heights their matter leaves from (`potential::apply`).
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
    let heat: Vec<f64> = cells.iter().map(|&i| w.store.heat(Class::Ground)[i as usize]).collect();
    apply(w, s, fluxes, &[]);
    super::parts::to_liquid(w, s, &cells, (&ks, &before), &heat);
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
    cells_state(w, s, &on);
}

/// One liquid step of `dt` seconds (the module's doc): after the runoff's pass the liquid's top and deep swap liquid
/// through the thermocline (`parts::mix`) and each body mixes its heat (`basin::mix`), so the bodies' tops end the step
/// mixed; last, the gases of the air dissolve in the liquid where it lies (`air::dissolve`).
pub(super) fn liquid(w: &mut World, s: &mut Scratch, dt: f64) {
    let (tmin, tmax) = range(&s.state.tg);
    let liq = s.liquids(&w.chem, (tmin, tmax), true);
    let slope = slopes(w, s);
    let runs = springs(w, s, tmin, &slope, dt);
    under(w, s, &liq, tmin, dt);
    surface(w, s, (&liq, tmin), (&slope, &runs), dt);
    let mixed = super::parts::mix(w, s, dt);
    cells_state(w, s, &mixed);
    super::basin::mix(w, s);
    super::air::dissolve(w, s, dt);
}

/// The slope of the surface of each ground node down its steepest way (`Level::steepest`) to the lower neighbours (by
/// more than `still`), 0 for none; and for a cell under a body, whose liquid stands at one level and drains to no channel.
/// The surface is the cell's mean ground, or the liquid standing over its pores if higher: the slope its hillsides
/// drain down to the channels inside it.
fn slopes(w: &World, s: &Scratch) -> Vec<f64> {
    let (cells, still) = (w.mesh.cells(), w.cfg.phys.still);
    let surface = |i: usize| (s.head[i] - s.pd[i]).max(s.z[i]);
    let drop = |i: usize, k: usize| Some(surface(i) - surface(cells.around(i).1[k] as usize)).filter(|&d| d > still);
    crate::par::map_n(cells.n(), crate::par::UNCOUNTED, |i| if s.basin.is[i] { 0.0 } else { cells.steepest(i, |k| drop(i, k)).map_or(0.0, |way| way.0) })
}
