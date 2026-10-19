//! Growth: a body builds body matter (and the other solids it builds) from its stores, paid by work (`grow`).

use crate::body::{self, Body, Layout, ALLOC, COPY_MASS, WET_TISSUE};
use crate::chem::{Phase, Reaction};
use crate::ground::Melting;
use crate::store::Node;
use crate::units::DAY;
use crate::{World, AIR};

/// Growth is building body matter, and past the size of maturity a share of it is offspring. A member makes matter
/// (`Chemistry::build`) and pays `Physics::c_copy` per kg of matter made, through `body::pay` (the work leaves as heat),
/// as far as: (1) its allowance, `Physics::c_build` x its `Body::supply` / `Physics::m_ref` x the days of its interval x
/// its activity (building takes time, and it is chemistry: a dry body hardly grows); (2) its spare: after what is paid
/// and what is converted leave its fuel, its reactions must still release its upkeep over an interval at today's rate;
/// (3) what its reactions can release now; (4) the stores it converts, which must also cover the fuel the work burns.
/// Storing fuel costs no building work. A body below its copy mass (the size of maturity) puts all it builds into its
/// own growth; of what it builds past it a share `ALLOC` goes into the brood (`Body::brood`, matter held back for
/// offspring) and the rest into its own growth, so with `ALLOC` 1 it stops at maturity and with less it grows all its
/// life. The liquid that goes with new matter comes by soaking (`body::live`). The other solids it can build
/// (`Chemistry::builds`) it builds the same way, first, each in its share (`Body::share`). `th` is its plateau state
/// (`Body::thermal`). Returns whether it built anything.
pub fn grow(w: &mut World, b: &mut Body, days: f64, th: Melting) -> bool {
    let (chem, p) = (&w.chem, &w.cfg.phys);
    let Some(bl) = chem.build.clone() else { return false };
    let (m, n) = (bl.outputs[0].0 as usize, b.n());
    b.brood = b.brooded(chem);
    // The matter a body of the size of maturity is made of: the living body is its matter over 1 - `WET_TISSUE` (`Body::built`).
    let full = b.dec(COPY_MASS) * n * (1.0 - WET_TISSUE);
    let needs = if b.adult(chem) { 0.0 } else { (full - b.matter_kg(chem)).max(0.0) };
    // (Building is chemistry: it goes with the body's activity, `Body::activity`; a dry body hardly grows.)
    let room0 = p.c_build * b.supply(chem, p, th.t) / p.m_ref * days.min(b.interval(w) as f64) * b.activity(chem, p);
    // (The plateau state holds until a conversion changes the body.)
    let mut th = Some(th);
    // The other solids it builds first, each with its share (`Body::share`) of the allowance and of what it can spare, and up
    // to the level that share makes of its matter (`Body::level`), by the same work; then matter with the rest.
    let (mut room, mut built) = (room0, false);
    for bk in chem.builds.clone() {
        let k = bk.outputs[0].0 as usize;
        let gap = (b.level(&w.chem, k, 0.0) - b.store[k]).max(0.0) * w.chem.subs[k].molar_mass;
        let made = convert(w, b, &bk, room0, b.share(&w.chem, k), gap, 0.0, &mut th);
        (room, built) = (room - w.cfg.phys.c_copy * made, built || made > 0.0);
    }
    let made = convert(w, b, &bl, room, 1.0, f64::MAX, full, &mut th);
    // What it made: up to what its maturity lacks it is body, of the rest the share `ALLOC` is brood.
    b.brood += (made - made.min(needs)) * b.gene_in_range(ALLOC) / w.chem.subs[m].molar_mass;
    built || made > 0.0
}

/// It plans on this share of what its reactions can release, so rounding never leaves it short.
const RELEASE_MARGIN: f64 = 0.999;
/// Less than this share of `full` (or of its mass) made is nothing made.
const LEAST_MADE: f64 = 1e-12;

/// One conversion of growth (`Chemistry::build` or one of `Chemistry::builds`): up to `most` kg of its first output with
/// the share `part` of `room` J of the work allowance and of what the body can spare, as `grow` says; a gas it makes
/// leaves to the air with its heat and its latent heat. `known` is the body's plateau state if nothing changed it since
/// it was found; a conversion that changes the body takes it. Returns the kg made: none below `LEAST_MADE` of `full`
/// (kg of matter of the size of maturity; 0 for the other solids).
#[allow(clippy::too_many_arguments)]
fn convert(w: &mut World, b: &mut Body, bl: &Reaction, room: f64, part: f64, most: f64, full: f64, known: &mut Option<Melting>) -> f64 {
    if room <= 0.0 || most <= 0.0 {
        return 0.0;
    }
    let (chem, p) = (&w.chem, &w.cfg.phys);
    let m = bl.outputs[0].0 as usize;
    // (A unit of growth makes `nu0` mol of what it makes; chemistry 0: 1.)
    let (nu0, mm) = (bl.outputs[0].1 as f64, chem.subs[m].molar_mass);
    let ivl = b.interval(w) as f64;
    let mass = b.mass(chem);
    let th = known.take().unwrap_or_else(|| b.thermal(chem));
    let payer = body::Payer::at(w, b, ivl, mass, th);
    let t = payer.th.t;
    let r = RELEASE_MARGIN * body::plan(w, b, f64::MAX, &payer).1;
    if r <= 0.0 {
        *known = Some(payer.th);
        return 0.0;
    }
    // The fuel it spares: what is paid and the inputs converted leave its fuel, less the product's own
    // fuel if it burns that too. Per kg made, J.
    let (ins, outs) = (&bl.inputs, &bl.outputs);
    let fuel = ins.iter().map(|&(k, nu)| nu as f64 * b.heat_of(chem, k as usize, t)).sum::<f64>() / (mm * nu0);
    let per_kg = p.c_copy + fuel - b.heat_of(chem, m, t) / mm;
    let spare = b.fuel(chem, t) * (1.0 - body::upkeep(w, b, ivl, mass, t) / r);
    // The stores it converts must also cover the fuel the work burns (each input that burns), and
    // leave the safe minimum of each (a body is not built of its last fuel: below its minimum it dies).
    let lay = Layout::of(chem);
    let size = b.size(chem);
    let floors: Vec<f64> = ins.iter().map(|&(k, _)| b.gene(lay.min + k as usize).max(0.0) * size / chem.subs[k as usize].molar_mass).collect();
    let reach: Vec<f64> = ins.iter().zip(&floors).map(|(&(k, nu), f)| {
        let heat = b.heat_of(chem, k as usize, t);
        (b.store[k as usize] - f).max(0.0) / (nu as f64 / (mm * nu0) + if heat > 0.0 { p.c_copy / heat } else { 0.0 })
    }).collect();
    let free = part * (room / p.c_copy).min(r / p.c_copy).min(spare / per_kg.max(1e-300));
    let kg = free.min(reach.iter().copied().fold(f64::MAX, f64::min)).min(most);
    if kg <= LEAST_MADE * full.max(LEAST_MADE * mass) {
        *known = Some(payer.th);
        return 0.0;
    }
    let (ins, outs, dh, cost) = (ins.clone(), outs.clone(), chem.dh_of(bl), p.c_copy * kg);
    if body::spend(w, b, cost, ivl * DAY, false, &payer) > 0.0 {
        return 0.0;
    }
    // What was paid may have burnt some of the solid: convert what is left of it, to its minimum.
    let ext = ins.iter().zip(&floors).fold(kg / (mm * nu0), |e, (&(k, nu), f)| e.min((b.store[k as usize] - f).max(0.0) / nu as f64));
    for &(k, nu) in &ins {
        // (A store used up to its last mol is zero, not a rounding below it.)
        b.store[k as usize] = (b.store[k as usize] - nu as f64 * ext).max(0.0);
    }
    b.e -= dh * ext;
    let (j, lift, t) = (w.mesh.up(b.cell as usize, 0, AIR), w.lift(b.cell as usize), b.temp(&w.chem));
    for &(k, nu) in &outs {
        let (k, x) = (k as usize, nu as f64 * ext);
        if w.chem.phase(k, t) == Phase::Gas {
            // (Its heat goes with it, and the body pays the latent heat a gas holds in the air, `Substance::latent_gas`.)
            let q = x * w.chem.subs[k].cp * t;
            b.e = b.e - q - x * w.chem.subs[k].latent_gas();
            w.moves.give(&w.chem, Node::air(j), 0, 0.0, q);
            w.moves.gas(&w.chem, k, x, lift);
        } else {
            b.store[k] += x;
        }
    }
    nu0 * ext * mm
}

