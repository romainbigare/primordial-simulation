//! Upkeep, and paying a cost by running the reactions that breathe.

use super::exchange::{breath, conductance_to_ground, cover_of};
use super::{Body, COPY_MASS, DENATURE, REPAIR, STRENGTH};
use crate::chem::{Chemistry, Phase};
use crate::math::mean_decay;
use crate::units::{DAY, MAX_SUBSTANCES};
use crate::{bond, World};

/// Upkeep over `days` of a body of `mass` kg, J: the cost of keeping tissue, by the kg of the body
/// itself (`Body::built`: its matter and the liquid it keeps with it, as if it held all it keeps,
/// not the fuel it stores nor the brood), mass^1.0: basal, which every body pays whatever its genes (`Physics::c_basal`), catalysts
/// (by their capacity, the catalysed rate per unit of input at `Physics::t_cat` summed over the genes
/// that are on, see `Physics::c_cat`, dearer if they keep their shape when warmer,
/// `Physics::t_firm`), strength, learning and signalling; capture area by the day; and one decision
/// (per member, by the brain's cost and weakly the member's mass, `Physics::think`, mass^0.25);
/// a body with no brain pays none of the learning, signal and decision. Repair is a share `c_repair`
/// of it at `REPAIR` 0 and doubles with each unit of the gene: all of it times 1 - c + c
/// 2^`REPAIR`, so a longer life costs more upkeep, a shorter one less, but never less than the rest
/// (`Physics::c_repair`).
///
/// **Why mass^1.0 while what a body takes and builds goes with mass^0.75** (the growth law of
/// West, Brown and Enquist 2001, ontogenetic growth): supply (`Body::equiv`: intake, the
/// `c_build` allowance, leaf) grows more slowly than the tissue it must keep, so a body grows until
/// keeping what it has takes all it can supply and there is nothing left to build with
/// (`copy::grow` builds from the spare after upkeep): its size is set by its own costs, not by a cap.
/// **Across kinds by the size the kind grows to** (`docs/reports/big-animals.md`): the
/// cost per kg of tissue is the kind's own, times (copy mass / `m_ref`)^(`intake_exp` - 1), the copy mass being the
/// size its genes mature at (`COPY_MASS`). Inside one body its cost goes by its kg as it grows;
/// across kinds an adult pays Kleiber's mass^0.75, as real adults do. The mass is the body that
/// is built, not the whole mass: storing fuel is cheap, tissue dear; and not `size`, which stops
/// at the liquid held now, a lack that dormancy (`activity`, below) already counts.
///
/// **It follows the body's temperature**: the whole of it times
/// `Body::pace`, the rate law of the reactions that pay it, exp(-Ea (1/T - 1/T_ref)), T the body's temperature,
/// Ea the barrier of its main burning reaction, T_ref = `Physics::t_cat` where the costs are anchored.
/// A cold body pays less and, as wear goes with upkeep, ages slower (dormancy in the cold); a warm one
/// pays more. Needs `prepare`.
pub fn upkeep(w: &World, b: &Body, days: f64, mass: f64) -> f64 {
    let sig = b.sig.iter().map(|s| s * s).sum::<f32>() as f64;
    let p = &w.cfg.phys;
    let mind = if w.life.brainless(b) { 0.0 } else { p.c_learn * b.learn_rate() as f64 + p.c_signal * sig };
    let think = w.life.brains.get(b.lineage as usize).and_then(|l| l.as_ref()).map_or(p.c_brain, |l| l.cost(p, &b.genes));
    // The cost of keeping tissue goes with the kg of the body (`Body::built`: not the fuel it
    // stores, nor the liquid it lacks, which dormancy counts below), supply with the mass to the
    // power 0.75 (see above): storing fuel is cheap, building and keeping tissue is dear.
    // And per kg by the size the kind matures at, to the power -0.25 (Kleiber across kinds).
    let tissue = b.built(&w.chem) * (b.dec(COPY_MASS) / p.m_ref).powf(p.intake_exp - 1.0);
    let u = days * (tissue * (p.c_basal + p.c_cat * b.k.cats + p.c_strength * b.gene(STRENGTH) + mind) + p.c_leaf * b.vis[1] as f64 * b.n()) + p.think(think, mass / b.n()) * b.n();
    // Everything it pays is chemistry, which runs in its liquid: a dry body pays almost nothing (and,
    // as wear grows with what it pays, hardly ages): all of it, the decision's cost too, times its
    // activity. And it runs at the pace of the body's temperature, by the same rate law as the
    // reactions that pay it (`Body::pace`).
    u * (1.0 - p.c_repair + p.c_repair * 2f64.powf(b.gene(REPAIR))) * b.activity(&w.chem, p) * b.pace(&w.chem, p, b.temp(&w.chem))
}

/// Reaction `r` takes an input that is gas at `t`: it breathes, so it runs on demand.
pub(super) fn breathes(chem: &Chemistry, r: usize, t: f64) -> bool {
    chem.reactions[r].inputs.iter().any(|p| chem.phase(p.0 as usize, t) == Phase::Gas)
}

/// Energy the body's reactions can release over `days` at its temperature now, J (the most
/// `pay` could pay). Needs `prepare`.
pub fn release(w: &World, b: &Body, days: f64) -> f64 {
    let (mass, cap) = b.mass_cap(&w.chem);
    plan(w, b, f64::MAX, days, mass, cap).1
}

/// Pay `cost` J by running the body's catalysed reactions that release heat and breathe, one
/// after the other as far as their inputs allow; gases come from the air the body exchanges
/// with in `days` and go to its cell's air, and the heat stays in the body. False, and nothing
/// paid, if they cannot release that much in that time.
pub fn pay(w: &mut World, b: &mut Body, cost: f64, days: f64) -> bool {
    cost <= 0.0 || spend(w, b, cost, days, b.interval() as f64 * DAY, false) == 0.0
}

/// `pay` with the heat released held over `dt` s; returns what was not paid (with `part` it pays what it can, else all or nothing).
pub(super) fn spend(w: &mut World, b: &mut Body, cost: f64, days: f64, dt: f64, part: bool) -> f64 {
    let (mass, cap) = b.mass_cap(&w.chem);
    let t0 = b.e / cap.max(1e-9);
    let left = pay_at(w, b, cost, days, mass, cap, part);
    // The heat released stays in the body and leaves through its surface: by the conductance of its
    // genes, cover and walls and what it radiates (`surface`, `radiated`), as in its update (`live`).
    // Its time constant (heat capacity over conductance, hours for a body of kilograms) is far under
    // the days to its next update, so what it makes warms it at a steady rate: spread over the
    // interval to the next update, the heat Q warms it by Q / (g dt) K for that time. The body keeps
    // that warmth (`hold`) and the rest goes to the ground. A cost that leaves it colder than it was is not held back.
    let (keep, i) = (t0 * b.heat_cap(&w.chem), b.cell as usize);
    if b.e > keep {
        let held = hold(w, b, mass, t0, dt);
        w.e_ground[i] += (b.e - keep) * (1.0 - held);
        b.e = keep + (b.e - keep) * held;
    }
    left
}

/// The share of a heat released over `dt` s that a body of `mass` kg at `t` keeps: the mean over that time of a pulse that
/// decays with its time constant, (1 - exp(-x)) / x, x = g dt / heat capacity (all of it if the time constant is the longer).
pub(super) fn hold(w: &World, b: &Body, mass: f64, t: f64, dt: f64) -> f64 {
    mean_decay(conductance_to_ground(w, b, mass, t) * dt / b.heat_cap(&w.chem).max(1e-9))
}

/// How far each reaction of `pay` runs (in the order of `Consts::burn`) to release up to `cost`
/// J, and the energy released. Each goes at most as far as Change's rate at the body's
/// temperature takes it over `days`: the same law as `change` (`change::pace`), its gases the air
/// it exchanges at the air's shares, its other inputs its own stores, each a phase of its own. So
/// a cold body releases little: the cold cost is the rate, not a rule.
pub(super) fn plan(w: &World, b: &Body, cost: f64, days: f64, mass: f64, cap: f64) -> ([f64; MAX_SUBSTANCES], f64) {
    let (chem, i) = (&w.chem, b.cell as usize);
    let depth = bond::depth(w, b, w.life.env.get(i).map_or(0.0, |e| e[1]));
    let (ns, na, j, air) = (chem.ns(), w.grid.na(), w.grid.air_of(i), &w.air);
    let tot: f64 = (0..ns).map(|k| air[k * na + j]).sum();
    let share = (breath(chem, &w.cfg, b, mass, cover_of(w, b), days, depth) / tot).min(0.5);
    let t = b.e / cap.max(1e-9);
    let dry = b.activity(chem, &w.cfg.phys);
    let gas = |k: usize| chem.phase(k, t) == Phase::Gas;
    let mut have = [0.0f64; MAX_SUBSTANCES];
    for &(r, _) in &b.k.burn {
        for &(k, _) in &chem.reactions[r].inputs {
            let k = k as usize;
            have[k] = if gas(k) { air[k * na + j] * share } else { b.store[k] };
        }
    }
    let (mut ext, mut got) = ([0.0f64; MAX_SUBSTANCES], 0.0);
    for (q, &(r, e)) in b.k.burn.iter().enumerate() {
        let rx = &chem.reactions[r];
        if got >= cost || !breathes(chem, r, t) {
            continue;
        }
        let lim = crate::change::scarce(&rx.inputs, |k| have[k]).0;
        if lim <= 0.0 {
            continue;
        }
        let act = crate::change::pace(rx, lim, |k| have[k], |k| if gas(k) { have[k] * tot / air[k * na + j] } else { have[k] });
        let rate = crate::change::denature(Body::catalyst(b.k.cat[r], dry), t, b.gene(DENATURE), w.cfg.phys.unfold) * rx.k * (-rx.ea / t).exp() * act;
        ext[q] = (lim * crate::change::frac(rate * days * DAY)).min((cost - got) / e);
        got += e * ext[q];
        for &(k, nu) in &rx.inputs {
            have[k as usize] -= nu as f64 * ext[q];
        }
    }
    (ext, got.min(cost))
}

/// `pay` when the body's mass and heat capacity are known; returns what was not paid. If
/// `part`, it pays what it can; else all or nothing.
pub(super) fn pay_at(w: &mut World, b: &mut Body, cost: f64, days: f64, mass: f64, cap: f64, part: bool) -> f64 {
    if cost <= 0.0 {
        return 0.0;
    }
    b.prepare(&w.chem, &w.cfg.phys);
    let (ext, paid) = plan(w, b, cost, days, mass, cap);
    let left = cost - paid;
    if left > 1e-9 * cost && !part {
        return cost;
    }
    let World { chem, grid, air, .. } = w;
    let (na, j, t) = (grid.na(), grid.air_of(b.cell as usize), b.e / cap.max(1e-9));
    let gas = |k: usize| chem.phase(k, t) == Phase::Gas;
    let mut latent = 0.0;
    for (q, &(r, _)) in b.k.burn.iter().enumerate().filter(|p| ext[p.0] > 0.0) {
        let rx = &chem.reactions[r];
        for &(k, nu) in &rx.inputs {
            let k = k as usize;
            let a = if gas(k) { &mut air[k * na + j] } else { &mut b.store[k] };
            let d = (nu as f64 * ext[q]).min(*a);
            *a -= d;
            if gas(k) {
                latent -= d * chem.subs[k].latent_gas();
            }
        }
        for &(k, nu) in &rx.outputs {
            let (k, d) = (k as usize, nu as f64 * ext[q]);
            *if gas(k) { &mut air[k * na + j] } else { &mut b.store[k] } += d;
            if gas(k) {
                latent += d * chem.subs[k].latent_gas();
            }
        }
    }
    b.e += paid - latent;
    if left > 1e-9 * cost { left } else { 0.0 }
}
