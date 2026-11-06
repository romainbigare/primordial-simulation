//! Liquid on and in the loose ground at each liquid step: the stores of the bed drain to the channels inside the cells
//! (`drain::springs`); what may run off runs down the cells in one pass in the order of a priority flood
//! (`runoff::surface`), doing work on its bed; the liquid under the surface moves (`drain::under`); and the basin stands
//! at one level (`basin::level`).

use super::{drain::{springs, under}, land::{stand, wet}, potential::{apply, apply_carried}, range, runoff::surface, Scratch};
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

/// Book `fluxes` through the store, and bring the ground nodes' state up to date for what changed (`Scratch::note`).
/// Their potential energy is `carried` where their part books it itself (`potential::apply_carried`), else by the
/// heights their matter leaves from (`potential::apply`).
pub(super) fn book(w: &mut World, s: &mut Scratch, fluxes: &[LinkFlux], carried: Option<&[f64]>) {
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
    match carried {
        Some(e) => apply_carried(w, s, fluxes, e),
        None => apply(w, s, fluxes, &[]),
    }
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
