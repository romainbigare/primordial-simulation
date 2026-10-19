//! Upkeep, and paying a cost by running the body's catalysed reactions that breathe, at the rate Change gives them.

use super::exchange::{breath, conductance_to_ground, cover_of, film};
use super::{Body, COPY_MASS, DENATURE, MAX_TAKE_SHARE, REPAIR, STRENGTH};
use crate::chem::{Chemistry, Phase};
use crate::ground::Melting;
use crate::math::mean_decay;
use crate::store::{held_more, Class, Node};
use crate::units::{DAY, MAX_SUBSTANCES};
use crate::{bond, World, AIR};

/// Upkeep over `days` of a body of `mass` kg, J: per kg of the body that is built (`Body::built`, mass^1.0, not the
/// fuel it stores), basal upkeep, its catalysts' capacity (`Consts::cats`), strength, learning and signalling; its
/// capture area by the day; one decision per member (`Physics::think`); a body with no brain pays no learning, signal
/// or decision. Per kg it goes with the size its kind matures at to the power `intake_exp` - 1, so across kinds adults
/// pay Kleiber's mass^0.75, while what one body can supply grows more slowly than the tissue it keeps and it grows until
/// keeping it takes all it can supply (West, Brown and Enquist 2001; docs/laws.md). Repair multiplies it by
/// 1 - c + c 2^`REPAIR` (`Physics::c_repair`). All of it goes with its activity and the pace of its temperature
/// (`Body::pace`): a dry or cold body pays little and ages slowly.
pub fn upkeep(w: &World, b: &Body, days: f64, mass: f64, t: f64) -> f64 {
    let sig = b.sig.iter().map(|s| s * s).sum::<f32>() as f64;
    let p = &w.cfg.phys;
    let mind = if w.life.brainless(b) { 0.0 } else { p.c_learn * b.learn_rate() as f64 + p.c_signal * sig };
    let think = w.life.brains.get(b.lineage as usize).and_then(|l| l.as_ref()).map_or(p.c_brain, |l| l.cost(p, &b.genes));
    let tissue = b.built(&w.chem) * (b.dec(COPY_MASS) / p.m_ref).powf(p.intake_exp - 1.0);
    let u = days * (tissue * (p.c_basal + p.c_cat * b.k.cats + p.c_strength * b.gene(STRENGTH) + mind) + p.c_leaf * b.vis[1] as f64 * b.n()) + p.think(think, mass / b.n()) * b.n();
    u * (1.0 - p.c_repair + p.c_repair * 2f64.powf(b.gene(REPAIR))) * b.activity(&w.chem, p) * b.pace(&w.chem, p, t)
}

/// Reaction `r` takes an input that is gas at `t`: it breathes, so it runs on demand.
pub(super) fn breathes(chem: &Chemistry, r: usize, t: f64) -> bool {
    chem.reactions[r].inputs.iter().any(|p| chem.phase(p.0 as usize, t) == Phase::Gas)
}

/// Energy the body's reactions can release over `days` at its temperature now, J (the most `pay` could pay).
pub fn release(w: &World, b: &Body, days: f64) -> f64 {
    plan(w, b, f64::MAX, &Payer::at(w, b, days, b.mass(&w.chem), b.thermal(&w.chem))).1
}

/// A body about to pay over some days, as its paying sees it (valid until its stores or its heat change): its mass, kg,
/// where it stands among its melting plateaus (`Body::thermal`), the days, the share of its air node it exchanges gas
/// with in them (`breath`, at most `MAX_TAKE_SHARE`) and all that node holds, mol.
pub(crate) struct Payer {
    pub mass: f64,
    pub th: Melting,
    days: f64,
    share: f64,
    tot: f64,
}

impl Payer {
    /// Body `b` of `mass` kg at the plateau state `th`, paying over `days`.
    pub(crate) fn at(w: &World, b: &Body, days: f64, mass: f64, th: Melting) -> Payer {
        let (chem, i) = (&w.chem, b.cell as usize);
        let depth = bond::depth(w, b, w.life.env.get(i).map_or(0.0, |e| e[1]));
        let j = w.mesh.up(i, 0, AIR);
        let tot: f64 = (0..chem.ns()).map(|k| w.a(k, j)).sum();
        let f = film(chem, &w.cfg, b, mass, cover_of(w, b));
        let share = (breath(chem, &w.cfg, b, &f, days, depth) / tot).min(MAX_TAKE_SHARE);
        Payer { mass, th, days, share, tot }
    }
}

/// Pay `cost` J by running the body's catalysed reactions that release heat and breathe, one after the other as far as
/// their inputs allow; gases come from the air the body exchanges with in `days` and go to its cell's air, and the heat
/// is held as `spend` holds it. False, and nothing paid, if they cannot release that much in that time.
pub fn pay(w: &mut World, b: &mut Body, cost: f64, days: f64) -> bool {
    if cost <= 0.0 {
        return true;
    }
    spend(w, b, cost, b.interval(w) as f64 * DAY, false, &Payer::at(w, b, days, b.mass(&w.chem), b.thermal(&w.chem))) == 0.0
}

/// `pay` by the payer `p` (body `b` as it stands) with the heat released held over `dt` s; returns what was not paid
/// (with `part` it pays what it can, else all or nothing).
pub(crate) fn spend(w: &mut World, b: &mut Body, cost: f64, dt: f64, part: bool, p: &Payer) -> f64 {
    let (left, mass, th0) = (pay_at(w, b, cost, p, part), p.mass, &p.th);
    // The heat released leaves through its surface (`conductance_to_ground`) over `dt`: the body keeps the share `hold`
    // and the rest goes to the ground. A cost that leaves it colder than it was is not held back.
    let (keep, i) = (b.energy_in(&w.chem, th0), b.cell as usize);
    if b.e > keep {
        let held = hold(w, b, mass, th0.t, dt);
        w.moves.give(&w.chem, Node::ground(i), 0, 0.0, (b.e - keep) * (1.0 - held));
        b.e = keep + (b.e - keep) * held;
    }
    left
}

/// The share of a heat released over `dt` s that a body of `mass` kg at `t` keeps: the mean over that time of a pulse
/// that decays with its time constant, `mean_decay` of g dt / heat capacity.
pub(super) fn hold(w: &World, b: &Body, mass: f64, t: f64, dt: f64) -> f64 {
    mean_decay(conductance_to_ground(w, b, mass, t) * dt / b.heat_cap(&w.chem).max(1e-9))
}

/// How far each reaction of `pay` runs (one entry for each of `Consts::burn`, in its order) to release up to `cost` J, and
/// the energy released, by the payer `p`. Each goes at most as far as Change's rate at the body's temperature takes it
/// over the payer's days (`change::pace`), its gases the air it exchanges (`Payer`), its other inputs its own stores. So a
/// cold body releases little: the cold cost is the rate, not a rule.
pub(crate) fn plan(w: &World, b: &Body, cost: f64, p: &Payer) -> (Vec<f64>, f64) {
    let (chem, j) = (&w.chem, w.mesh.up(b.cell as usize, 0, AIR));
    let air = |k: usize| w.a(k, j);
    let (t, days, share, tot) = (p.th.t, p.days, p.share, p.tot);
    let dry = b.activity(chem, &w.cfg.phys);
    let gas = |k: usize| chem.phase(k, t) == Phase::Gas;
    let mut have = [0.0f64; MAX_SUBSTANCES];
    for &(r, _) in &b.k.burn {
        for &(k, _) in &chem.reactions[r].inputs {
            let k = k as usize;
            have[k] = if gas(k) { air(k) * share } else { b.store[k] };
        }
    }
    let (mut ext, mut got) = (vec![0.0f64; b.k.burn.len()], 0.0);
    for (q, &(r, e)) in b.k.burn.iter().enumerate() {
        let rx = &chem.reactions[r];
        if got >= cost || !breathes(chem, r, t) {
            continue;
        }
        let lim = crate::change::scarce(&rx.inputs, |k| have[k]).0;
        if lim <= 0.0 {
            continue;
        }
        let act = crate::change::pace(rx, lim, |k| have[k], |k| if gas(k) { have[k] * tot / air(k) } else { have[k] });
        let rate = crate::change::denature(Body::catalyst(b.k.cat[r], dry), t, b.gene(DENATURE), w.cfg.phys.unfold) * rx.arrhenius(t) * act;
        ext[q] = (lim * crate::change::frac(rate * days * DAY)).min((cost - got) / e);
        got += e * ext[q];
        for &(k, nu) in &rx.inputs {
            have[k as usize] -= nu as f64 * ext[q];
        }
    }
    (ext, got.min(cost))
}

/// `pay` by the payer `p`; returns what was not paid. If `part`, it pays what it can; else all or nothing. The heat stays
/// in the body, less the latent heat of the gases taken and put out.
pub(super) fn pay_at(w: &mut World, b: &mut Body, cost: f64, p: &Payer, part: bool) -> f64 {
    if cost <= 0.0 {
        return 0.0;
    }
    let ((ext, paid), t) = (plan(w, b, cost, p), p.th.t);
    let left = cost - paid;
    if left > 1e-9 * cost && !part {
        return cost;
    }
    // (Gas is taken from the air with no heat of its own and given to it with its latent heat, which the body pays:
    // the air holds it, `store::held_more`.)
    let air = Node::air(w.mesh.up(b.cell as usize, 0, AIR));
    let gas = |k: usize| w.chem.phase(k, t) == Phase::Gas;
    let mut latent = 0.0;
    for (q, &(r, _)) in b.k.burn.iter().enumerate().filter(|p| ext[p.0] > 0.0) {
        for &(k, nu) in &w.chem.reactions[r].inputs {
            let (k, lat) = (k as usize, w.chem.subs[k as usize].latent_gas());
            let d = nu as f64 * ext[q];
            if gas(k) {
                w.moves.take(&w.chem, air, k, d, 0.0);
                latent -= d * lat;
            } else {
                super::add_store(&mut b.store[k], -d, &mut w.counts.rounded[0]);
            }
        }
        for &(k, nu) in &w.chem.reactions[r].outputs {
            let (k, d) = (k as usize, nu as f64 * ext[q]);
            if gas(k) {
                w.moves.give(&w.chem, air, k, d, d * held_more(&w.chem, Class::Body, Class::Air, k));
                latent += d * w.chem.subs[k].latent_gas();
            } else {
                b.store[k] += d;
            }
        }
    }
    b.e += paid - latent;
    if left > 1e-9 * cost { left } else { 0.0 }
}
