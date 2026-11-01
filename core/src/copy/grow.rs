//! Growth: a body builds body matter (and the other solids it builds) from its stores, paid by work (`grow`).

use crate::body::{self, Body, Layout, ALLOC, WET_TISSUE};
use crate::chem::{Chemistry, Phase, Reaction};
use crate::store::Node;
use crate::{life::Chunk, World};

/// Growth: a member builds body matter, paid by work, as far as its allowance, its spare, its fuel and the stores it
/// converts allow; past its copy mass a share `ALLOC` goes into the brood (laws.md 6.6). It stands at `t` K. Returns
/// whether it built anything.
pub fn grow(w: &World, d: &mut Chunk, b: &mut Body, days: f64, t: f64) -> bool {
    let chem = &w.chem;
    let Some(bl) = chem.build.as_ref() else { return false };
    let (m, n) = (bl.outputs[0].0 as usize, b.n());
    b.brood = b.brooded(chem);
    // The matter a body of the size of maturity is made of: the living body is its matter over 1 - `WET_TISSUE` (`Body::built`).
    let full = b.k.copy_mass * n * (1.0 - WET_TISSUE);
    let needs = if b.adult(chem) { 0.0 } else { (full - b.matter_kg(chem)).max(0.0) };
    let room0 = allowance(w, b, t, days, b.activity(chem, &w.cfg.phys));
    // The other solids it builds first, each with its share (`Body::share`) of the allowance, and up to the level that
    // share makes of its matter (`Body::level`), by the same work; then matter with the rest.
    let (mut room, mut built) = (room0, false);
    for bk in &chem.builds {
        let k = bk.outputs[0].0 as usize;
        let s = chem.body().place(k).expect("a built solid is a store");
        let gap = (b.level(&w.chem, s, 0.0) - b.store[s]).max(0.0) * w.chem.subs[k].molar_mass;
        let made = convert(w, d, b, bk, room0 * b.share(&w.chem, k), gap, 0.0, t);
        (room, built) = (room - w.cfg.phys.c_copy * made, built || made > 0.0);
    }
    let made = convert(w, d, b, bl, room, f64::MAX, full, t);
    // What it made: up to what its maturity lacks it is body, of the rest the share `ALLOC` is brood.
    b.brood += (made - made.min(needs)) * b.gene_in_range(ALLOC) / w.chem.subs[m].molar_mass;
    built || made > 0.0
}

/// The kg of the first output of conversion `bl` body `b` at `t` K could make of its stores: each input must cover what
/// the conversion takes of it and, if it burns, the fuel the work burns, and leave its safe minimum (a body is not built
/// of its last fuel: below its minimum it dies); with each input's minimum, mol.
fn reach(chem: &Chemistry, p: &crate::Physics, b: &Body, bl: &Reaction, t: f64) -> (Vec<f64>, f64) {
    let (lay, size, m, bc) = (Layout::of(chem), b.size(chem), bl.outputs[0].0 as usize, chem.body());
    let (nu0, mm) = (bl.outputs[0].1 as f64, chem.subs[m].molar_mass);
    let floors: Vec<f64> = bl.inputs.iter().map(|&(k, _)| b.gene(lay.min + k as usize).max(0.0) * size / chem.subs[k as usize].molar_mass).collect();
    let reach = bl.inputs.iter().zip(&floors).map(|(&(k, nu), f)| {
        let heat = b.heat_of(chem, k as usize, t);
        (b.amount(bc, k as usize) - f).max(0.0) / (nu as f64 / (mm * nu0) + if heat > 0.0 { p.c_copy / heat } else { 0.0 })
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

/// Less than this share of `full` (or of its mass) made is nothing made.
const LEAST_MADE: f64 = 1e-12;

/// One conversion of growth (`Chemistry::build` or one of `Chemistry::builds`) at `t` K: up to `most` kg of its first
/// output with `room` J of the work allowance, as far as its fuel spares after a month's upkeep and its stores
/// reach (`reach`); a gas it makes leaves to the air with its heat and its latent heat, and it keeps its temperature.
/// Returns the kg made: none below `LEAST_MADE` of `full` (kg of matter of the size of maturity; 0 for the other
/// solids).
#[allow(clippy::too_many_arguments)]
fn convert(w: &World, d: &mut Chunk, b: &mut Body, bl: &Reaction, room: f64, most: f64, full: f64, t: f64) -> f64 {
    if room <= 0.0 || most <= 0.0 {
        return 0.0;
    }
    let (chem, p, month) = (&w.chem, &w.cfg.phys, w.cfg.update_days());
    let m = bl.outputs[0].0 as usize;
    // (A unit of growth makes `nu0` mol of what it makes; chemistry 0: 1.)
    let (nu0, mm, mass) = (bl.outputs[0].1 as f64, chem.subs[m].molar_mass, b.mass(chem));
    // The fuel it spares: what is paid and the inputs converted leave its fuel, less the product's own fuel if it burns
    // that too. Per kg made, J.
    let (ins, outs) = (&bl.inputs, &bl.outputs);
    let fuel = ins.iter().map(|&(k, nu)| nu as f64 * b.heat_of(chem, k as usize, t)).sum::<f64>() / (mm * nu0);
    let per_kg = p.c_copy + fuel - b.heat_of(chem, m, t) / mm;
    let spare = (b.fuel(chem, t) - body::upkeep(w, b, month, mass, t, b.activity(chem, p))).max(0.0);
    let (floors, reach) = reach(chem, p, b, bl, t);
    let kg = (room / p.c_copy).min(spare / per_kg.max(1e-300)).min(reach).min(most);
    if kg <= LEAST_MADE * full.max(LEAST_MADE * mass) || !body::pay(w, d, b, p.c_copy * kg) {
        return 0.0;
    }
    // What was paid may have burnt some of the solid: convert what is left of it, to its minimum.
    let at = |k: u16| chem.body().place(k as usize).expect("growth's substances are stores");
    let ext = ins.iter().zip(&floors).fold(kg / (mm * nu0), |e, (&(k, nu), f)| e.min((b.store[at(k)] - f).max(0.0) / nu as f64));
    for &(k, nu) in ins {
        body::add_store(&mut b.store[at(k)], -(nu as f64) * ext, &mut d.counts.rounded[0]);
    }
    b.e -= chem.dh_of(bl) * ext;
    let (lift, i) = (w.lift(b.cell as usize), b.cell as usize);
    for &(k, nu) in outs {
        let (k, x) = (k as usize, nu as f64 * ext);
        // (What it makes and cannot hold, a store of no body, leaves it as a gas does.)
        if let Some(s) = chem.body().place(k).filter(|_| w.chem.phase(k, t) != Phase::Gas) {
            b.store[s] += x;
        } else {
            // (Its heat goes with it, and the body pays the latent heat a gas holds in the air, `Substance::latent_gas`.)
            let q = x * w.chem.subs[k].cp * t;
            b.e = b.e - q - x * w.chem.subs[k].latent_gas();
            d.moves.give(w, Node::bodies(i), 0, 0.0, q);
            d.moves.gas(w, k, x, lift);
        }
    }
    body::settle_heat(w, d, b, t);
    nu0 * ext * mm
}
