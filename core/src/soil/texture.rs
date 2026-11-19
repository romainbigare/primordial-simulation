//! The texture of the loose ground (laws.md 4): the shares of fine, middle and coarse particles that the laws that
//! make it leave in each of the terrain's samples (`samples`). Three laws, each over what the wear at the world's start
//! left (`space::wear`):
//! - the bed breaks into particles as its substances do (`Make::coarse`, from the chemistry's numbers, `worldgen`);
//! - the share of it that weathers (`Make::weathers`) turns into fine particles at a first-order rate over the time
//!   the loose ground has lain (`Samples::age`; a mineral's lifetime against weathering, White and Brantley 2003);
//! - what settled on a sample (`Samples::settled`) is sorted by how fast the liquid that laid it ran: a particle
//!   settles where it sinks faster than the flow's shear velocity stirs it (Bagnold 1966), as Stokes's law and its
//!   drag at larger sizes set its sinking (Ferguson et al. 2004), so coarse particles drop where the flow is fast
//!   and the fine only where it is still (Hjulström 1935).

use super::Texture;
use crate::space::network::NONE;
use crate::space::samples::Samples;
use crate::units::LIQUID_VISCOSITY;
use crate::World;

/// The particle classes, fine, middle and coarse, by their least and most diameter, m (fine under 2 µm, middle to
/// 50 µm, coarse to 2 mm; Soil Survey Staff 1993); within a class the particles spread evenly over the logarithm of
/// their size.
const CLASSES: [[f64; 2]; 3] = [[2e-7, 2e-6], [2e-6, 5e-5], [5e-5, 2e-3]];
const FINE: usize = 0;
const MIDDLE: usize = 1;
const COARSE: usize = 2;
/// The years over which the weathering share of the loose ground turns e times nearer to fine particles.
const WEATHER_YEARS: f64 = 1e5;
/// A deposit spans at least this factor of particle sizes: no flow sorts its particles to one size.
const SORTING: f64 = 2.0;
/// The flood that sorts what settles: its runoff from the area that drains through a sample, m/s.
const FLOOD_RUNOFF: f64 = 1e-6;
/// The drag constants of a natural particle's sinking (Ferguson et al. 2004).
const DRAG_STOKES: f64 = 18.0;
const DRAG_FORM: f64 = 1.0;
/// The halvings that find the least particle that settles in a flow (`settles`): its size to a few parts in a million.
const HALVINGS: usize = 40;

/// The texture of each of the terrain's samples of world `w` (`Samples`, in their order): its bed's particles
/// (`Make::coarse`), weathered over the time its loose ground has lain, and the share of it that settled there sorted
/// by the flow that laid it (the module's doc). A sample under standing liquid (in a pond, or the open low ground) was
/// laid by still liquid. The flow at a sample is the flood's (`FLOOD_RUNOFF` from the area that drains through it), as
/// deep as Flow's channels carry it (`Physics::channel_depth`), down its slope; the load that reaches it is what the
/// flow at its main sample upstream (`Samples::main`) carried on, all particles where none runs to it.
pub fn samples(w: &World) -> Vec<Texture> {
    let s = of(w);
    let (ph, g) = (&w.cfg.phys, w.cfg.gravity);
    let shear = |k: usize| {
        let depth = ph.channel_depth * (FLOOD_RUNOFF * s.up[k]).powf(ph.channel_depth_exp);
        if s.pond[k] != NONE { 0.0 } else { (g * depth * s.slope[k]).sqrt() }
    };
    crate::par::map_n(s.h.len(), crate::par::UNCOUNTED, |k| {
        let m = w.terrain.makes.get(s.make[k] as usize).copied().unwrap_or_default();
        let laid = if s.loose[k] > 0.0 { s.settled[k] / s.loose[k] } else { 0.0 };
        let mut x = weathered([0.0, 1.0 - m.coarse, m.coarse], m.weathers, s.age[k]);
        if laid > 0.0 {
            let upstream = if s.main[k] == NONE { f64::INFINITY } else { shear(s.main[k] as usize) };
            let sinks = |u: f64| settles(u, m.buoyant, g);
            let d = sorted(x, sinks(shear(k)), sinks(upstream));
            x = [0, 1, 2].map(|c| (1.0 - laid) * x[c] + laid * d[c]);
        }
        Texture { coarse: x[COARSE], fine: x[FINE], organic: 0.0 }
    })
}

/// Particle shares `x` (fine, middle, coarse) of which the share `weathers` weathers, after `years`: that share of the
/// middle and coarse particles turned fine as 1 − exp(−years / `WEATHER_YEARS`).
fn weathered(x: [f64; 3], weathers: f64, years: f64) -> [f64; 3] {
    let turned = weathers * -(-years / WEATHER_YEARS).exp_m1();
    [x[FINE] + (x[MIDDLE] + x[COARSE]) * turned, x[MIDDLE] * (1.0 - turned), x[COARSE] * (1.0 - turned)]
}

/// How fast a particle of diameter `d` m sinks through the key liquid, m/s, its submerged weight `buoyant` times the
/// liquid's at gravity `g`: Stokes's drag for small particles, a form drag for large ones (Ferguson et al. 2004).
fn sinking(d: f64, buoyant: f64, g: f64) -> f64 {
    buoyant * g * d * d / (DRAG_STOKES * LIQUID_VISCOSITY + (0.75 * DRAG_FORM * buoyant * g * d * d * d).sqrt())
}

/// The least particle that settles under a flow of shear velocity `u`, m/s: the diameter that sinks at `u` (Bagnold
/// 1966), m; none in still liquid, and the coarsest class's most where no particle of the classes settles.
fn settles(u: f64, buoyant: f64, g: f64) -> f64 {
    let top = CLASSES[COARSE][1];
    if u <= 0.0 {
        return 0.0;
    }
    if sinking(top, buoyant, g) <= u {
        return top;
    }
    // Sinking grows with the size: halve the span of its logarithm between the finest class's least and the top.
    let (mut lo, mut hi) = (CLASSES[FINE][0].ln(), top.ln());
    for _ in 0..HALVINGS {
        let mid = 0.5 * (lo + hi);
        if sinking(mid.exp(), buoyant, g) < u { lo = mid } else { hi = mid }
    }
    hi.exp()
}

/// What of a load of particle shares `load` (fine, middle, coarse) settles where particles from `least` m up settle and
/// the flow upstream let none from `most` m up reach (`settles`): the share of each class within those sizes, as shares
/// of what settles. The span is at least `SORTING` wide and inside the classes; where it holds none of the load, what
/// settles is the load's class nearest it.
fn sorted(load: [f64; 3], least: f64, most: f64) -> [f64; 3] {
    let (bottom, top) = (CLASSES[FINE][0], CLASSES[COARSE][1]);
    let lo = least.clamp(bottom, top / SORTING);
    let hi = most.max(lo * SORTING).min(top);
    let within = |c: usize| {
        let [a, b] = CLASSES[c];
        (hi.min(b).ln() - lo.max(a).ln()).max(0.0) / (b / a).ln()
    };
    let x = [0, 1, 2].map(|c| load[c] * within(c));
    let sum: f64 = x.iter().sum();
    if sum > 0.0 {
        return x.map(|v| v / sum);
    }
    let centre = 0.5 * (lo.ln() + hi.ln());
    let far = |c: usize| (0.5 * (CLASSES[c][0].ln() + CLASSES[c][1].ln()) - centre).abs();
    let near = (0..3).filter(|&c| load[c] > 0.0).min_by(|&a, &b| far(a).total_cmp(&far(b))).unwrap_or(MIDDLE);
    std::array::from_fn(|c| if c == near { 1.0 } else { 0.0 })
}

/// The samples of world `w` (`Terrain::samples`).
fn of(w: &World) -> std::sync::Arc<Samples> {
    w.terrain.samples(w.terrain.seed, &w.map, (w.cfg.lat_south, w.cfg.lat_north))
}
