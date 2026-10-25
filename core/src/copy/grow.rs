//! Growth: a body builds body matter (and the other solids it builds) from its stores, paid by work (`grow`).

use crate::body::{self, Body, Layout, ALLOC, WET_TISSUE};
use crate::chem::{Chemistry, Phase, Reaction};
use crate::ground::Melting;
use crate::store::Node;
use crate::units::DAY;
use crate::{life::Domain, World, AIR};

/// Growth: a member builds body matter, paid by work, as far as its allowance, its spare, what its reactions release
/// and the stores it converts allow; past its copy mass a share `ALLOC` goes into the brood (laws.md 6.6). `th` is its
/// plateau state. Returns whether it built anything.
pub fn grow(w: &World, d: &mut Domain, b: &mut Body, days: f64, th: Melting, cover: f64) -> bool {
    let chem = &w.chem;
    let Some(bl) = chem.build.as_ref() else { return false };
    let (m, n) = (bl.outputs[0].0 as usize, b.n());
    b.brood = b.brooded(chem);
    // The matter a body of the size of maturity is made of: the living body is its matter over 1 - `WET_TISSUE` (`Body::built`).
    let full = b.k.copy_mass * n * (1.0 - WET_TISSUE);
    let needs = if b.adult(chem) { 0.0 } else { (full - b.matter_kg(chem)).max(0.0) };
    let (ivl, dry) = (b.interval(w) as f64, b.activity(chem, &w.cfg.phys));
    let room0 = allowance(w, b, th.t, days.min(ivl), dry);
    // (The payer, with the plateau state and the activity, holds until a conversion changes the body; with no allowance
    // nothing converts.)
    let mut payer = (room0 > 0.0).then(|| body::Payer::at(w, d, b, ivl, b.mass(chem), th, (cover, dry)));
    // The other solids it builds first, each with its share (`Body::share`) of the allowance and of what it can spare, and up
    // to the level that share makes of its matter (`Body::level`), by the same work; then matter with the rest.
    let (mut room, mut built) = (room0, false);
    for bk in &chem.builds {
        let k = bk.outputs[0].0 as usize;
        let gap = (b.level(&w.chem, k, 0.0) - b.store[k]).max(0.0) * w.chem.subs[k].molar_mass;
        let made = convert(w, d, b, bk, room0, b.share(&w.chem, k), gap, 0.0, (&mut payer, cover));
        (room, built) = (room - w.cfg.phys.c_copy * made, built || made > 0.0);
    }
    let made = convert(w, d, b, bl, room, 1.0, f64::MAX, full, (&mut payer, cover));
    // What it made: up to what its maturity lacks it is body, of the rest the share `ALLOC` is brood.
    b.brood += (made - made.min(needs)) * b.gene_in_range(ALLOC) / w.chem.subs[m].molar_mass;
    built || made > 0.0
}

/// The kg of the first output of conversion `bl` body `b` at `t` K could make of its stores: each input must cover what
/// the conversion takes of it and, if it burns, the fuel the work burns, and leave its safe minimum (a body is not built
/// of its last fuel: below its minimum it dies); with each input's minimum, mol.
fn reach(chem: &Chemistry, p: &crate::Physics, b: &Body, bl: &Reaction, t: f64) -> (Vec<f64>, f64) {
    let (lay, size, m) = (Layout::of(chem), b.size(chem), bl.outputs[0].0 as usize);
    let (nu0, mm) = (bl.outputs[0].1 as f64, chem.subs[m].molar_mass);
    let floors: Vec<f64> = bl.inputs.iter().map(|&(k, _)| b.gene(lay.min + k as usize).max(0.0) * size / chem.subs[k as usize].molar_mass).collect();
    let reach = bl.inputs.iter().zip(&floors).map(|(&(k, nu), f)| {
        let heat = b.heat_of(chem, k as usize, t);
        (b.store[k as usize] - f).max(0.0) / (nu as f64 / (mm * nu0) + if heat > 0.0 { p.c_copy / heat } else { 0.0 })
    });
    let most = reach.fold(f64::MAX, f64::min);
    (floors, most)
}

/// The work body `b` at `t` K may spend on building over `days`, J: `c_build` a day per kg-equivalent of its supply at
/// `m_ref` (`Body::supply`). Building is chemistry: it goes with the body's activity `dry` (`Body::activity`), so a dry
/// body hardly grows.
fn allowance(w: &World, b: &Body, t: f64, days: f64, dry: f64) -> f64 {
    let p = &w.cfg.phys;
    p.c_build * b.supply(&w.chem, p, t) / p.m_ref * days * dry
}

/// What body `b` at `t` K holds that is or soon becomes body, kg: its matter (its brood with it), and of the matter its
/// stores could make (`reach`) as much as one interval's allowance builds. Selection inside a node counts its members'
/// gain in it (`copy::select`): a gene that brings more of what growth converts gains though the update builds no
/// more, but a store past what growth can use in an interval gains nothing, and a solid it builds counts only by what
/// it does (its height and strength), as neither becomes children.
pub fn buildable(w: &World, b: &Body, t: f64, dry: f64) -> f64 {
    let (Some(bl), Some(m)) = (w.chem.build.as_ref(), w.chem.matter()) else { return 0.0 };
    let soon = allowance(w, b, t, b.interval(w) as f64, dry) / w.cfg.phys.c_copy;
    b.store[m] * w.chem.subs[m].molar_mass + reach(&w.chem, &w.cfg.phys, b, bl, t).1.min(soon)
}

/// It plans on this share of what its reactions can release, so rounding never leaves it short.
const RELEASE_MARGIN: f64 = 0.999;
/// Less than this share of `full` (or of its mass) made is nothing made.
const LEAST_MADE: f64 = 1e-12;

/// One conversion of growth (`Chemistry::build` or one of `Chemistry::builds`): up to `most` kg of its first output with
/// the share `part` of `room` J of the work allowance and of what the body can spare, as `grow` says; a gas it makes
/// leaves to the air with its heat and its latent heat. `known` is the body's payer (its plateau state and activity) if
/// nothing changed it since it was made; a conversion that changes the body takes it; `cover` is the cover of other
/// bodies over it, which an update does not change. Returns the kg made: none below `LEAST_MADE` of `full`
/// (kg of matter of the size of maturity; 0 for the other solids).
#[allow(clippy::too_many_arguments)]
fn convert(w: &World, d: &mut Domain, b: &mut Body, bl: &Reaction, room: f64, part: f64, most: f64, full: f64, (known, cover): (&mut Option<body::Payer>, f64)) -> f64 {
    if room <= 0.0 || most <= 0.0 {
        return 0.0;
    }
    let (chem, p, ivl) = (&w.chem, &w.cfg.phys, b.interval(w) as f64);
    let m = bl.outputs[0].0 as usize;
    // (A unit of growth makes `nu0` mol of what it makes; chemistry 0: 1.)
    let (nu0, mm) = (bl.outputs[0].1 as f64, chem.subs[m].molar_mass);
    let payer = known.take().unwrap_or_else(|| body::Payer::at(w, d, b, ivl, b.mass(chem), b.thermal(chem), (cover, b.activity(chem, p))));
    let (mass, t) = (payer.mass, payer.th.t);
    let r = RELEASE_MARGIN * body::plan(w, d, b, f64::MAX, &payer).1;
    if r <= 0.0 {
        *known = Some(payer);
        return 0.0;
    }
    // The fuel it spares: what is paid and the inputs converted leave its fuel, less the product's own
    // fuel if it burns that too. Per kg made, J.
    let (ins, outs) = (&bl.inputs, &bl.outputs);
    let fuel = ins.iter().map(|&(k, nu)| nu as f64 * b.heat_of(chem, k as usize, t)).sum::<f64>() / (mm * nu0);
    let per_kg = p.c_copy + fuel - b.heat_of(chem, m, t) / mm;
    let spare = b.fuel(chem, t) * (1.0 - body::upkeep(w, b, ivl, mass, t, payer.dry) / r);
    let (floors, reach) = reach(chem, p, b, bl, t);
    let free = part * (room / p.c_copy).min(r / p.c_copy).min(spare / per_kg.max(1e-300));
    let kg = free.min(reach).min(most);
    if kg <= LEAST_MADE * full.max(LEAST_MADE * mass) {
        *known = Some(payer);
        return 0.0;
    }
    let (dh, cost) = (chem.dh_of(bl), p.c_copy * kg);
    if body::spend(w, d, b, cost, ivl * DAY, false, &payer) > 0.0 {
        return 0.0;
    }
    // What was paid may have burnt some of the solid: convert what is left of it, to its minimum.
    let ext = ins.iter().zip(&floors).fold(kg / (mm * nu0), |e, (&(k, nu), f)| e.min((b.store[k as usize] - f).max(0.0) / nu as f64));
    for &(k, nu) in ins {
        body::add_store(&mut b.store[k as usize], -(nu as f64) * ext, &mut d.counts.rounded[0]);
    }
    b.e -= dh * ext;
    let (j, lift, t) = (w.mesh.up(b.cell as usize, 0, AIR), w.lift(b.cell as usize), b.temp(&w.chem));
    for &(k, nu) in outs {
        let (k, x) = (k as usize, nu as f64 * ext);
        if w.chem.phase(k, t) == Phase::Gas {
            // (Its heat goes with it, and the body pays the latent heat a gas holds in the air, `Substance::latent_gas`.)
            let q = x * w.chem.subs[k].cp * t;
            b.e = b.e - q - x * w.chem.subs[k].latent_gas();
            d.moves.give(w, Node::air(j), 0, 0.0, q);
            d.moves.gas(w, k, x, lift);
        } else {
            b.store[k] += x;
        }
    }
    nu0 * ext * mm
}

