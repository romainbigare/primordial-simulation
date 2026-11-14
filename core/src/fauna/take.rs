//! What the slots of takers of a cell take of its cover in a month (laws.md 3b): Holling's disk law over many layers
//! and many slots (Holling 1959), in `SUBSTEPS` steps, each in the exponential form, so the slots share a layer by
//! their rates, switch to the next as one runs out, and never overdraw it in any order.

use super::body::Body;
use super::FS;
use crate::contract::{self, Layer, Shown};
use crate::life::Chunk;
use crate::units::MAX_SUBSTANCES;
use crate::World;

/// The steps of the month the take is found in: each short enough for the slots to switch to the next layer within days
/// of one running out (a rule of the model).
pub const SUBSTEPS: usize = 6;

/// A layer of the cell's cover as the takers find it: the contract's layer, what it shows above the snow, and the
/// height its plants stand at, m.
pub(super) struct Found {
    pub(super) layer: Layer,
    pub(super) shown: Shown,
    pub(super) height: f64,
}

/// The layers of the cover of cell `i` that show anything: each slot of its cover by height, then the seeds lying.
pub(super) fn layers(w: &World, d: &Chunk, i: usize) -> Vec<Found> {
    let cell = d.at(i);
    let slots = contract::slots(w, cell).into_iter().chain([(Layer::Seeds, 0.0)]);
    slots.filter_map(|(layer, height)| Some(Found { layer, shown: contract::shown(w, i, cell, layer)?, height })).filter(|f| f.shown.kg > 0.0).collect()
}

/// A slot as a taker of the scheme (`scheme`): its members, the ground it searches a s per member, m²/s, its working
/// time, s a day, its chewing power, W, and the share of each layer's reachable stock it reaches.
pub struct Eater {
    pub n: f64,
    pub search: f64,
    pub busy: f64,
    pub chew: f64,
    pub reach: Vec<f64>,
}

/// What `eaters` take over `days` days of layers whose stock within reach is `stock` kg on a cell of `area` m², each
/// kg taking `breaking` J to break: kg of each layer per eater, and each eater's searching time, s a day per member, in
/// the mean over the steps. Per member and day of layer l (D its stock per m², w the share it reaches, h = breaking /
/// chewing power its handling time a kg): I_l = T a w_l D_l / (1 + a Σ_k w_k D_k h_k) (Holling's disk law: a thin layer is
/// slow to find, a thick one slow to chew). Over a step of Δ days each layer falls by S (1 − e^(−k Δ)), k = Σ n I_l / S
/// (so it is never overdrawn), shared by the parts of k, the last eater taking what the others leave. The denominators
/// are those of the stocks at the step's middle, found by a half step at the start's rates (the midpoint rule), so a
/// layer that runs low within a step speeds its takers' handling of the rest as it does in time.
pub fn scheme(eaters: &[Eater], (stock, breaking): (&[f64], &[f64]), area: f64, days: f64) -> (Vec<Vec<f64>>, Vec<f64>) {
    let (nl, step) = (stock.len(), days / SUBSTEPS as f64);
    let mut s = stock.to_vec();
    let (mut taken, mut searching) = (vec![vec![0.0; nl]; eaters.len()], vec![0.0; eaters.len()]);
    // Per eater and layer, its members' take a day per kg of the layer's stock at stocks `at`, and its searching time.
    let rates = |at: &[f64]| -> Vec<(Vec<f64>, f64)> {
        let per = |x: &Eater| {
            let found = |l: usize| x.search * x.reach[l] * at[l] / area;
            let slow = 1.0 + (0..nl).map(|l| found(l) * breaking[l] / x.chew.max(1e-300)).sum::<f64>();
            ((0..nl).map(|l| x.n * x.busy * x.search * x.reach[l] / area / slow).collect(), x.busy / slow)
        };
        eaters.iter().map(per).collect()
    };
    let fall = |r: &[(Vec<f64>, f64)], l: usize| r.iter().map(|e| e.0[l]).sum::<f64>();
    for _ in 0..SUBSTEPS {
        let start = rates(&s);
        let middle: Vec<f64> = (0..nl).map(|l| s[l] * (1.0 - fall(&start, l) * step / 2.0).max(0.0)).collect();
        let r = rates(&middle);
        r.iter().enumerate().for_each(|(e, x)| searching[e] += x.1 / SUBSTEPS as f64);
        for l in 0..nl {
            let all = fall(&r, l);
            if all <= 0.0 || s[l] <= 0.0 {
                continue;
            }
            let got = s[l] * crate::math::one_minus_exp(all * step);
            let mut left = got;
            let last = (0..eaters.len()).rev().find(|&e| r[e].0[l] > 0.0).unwrap_or(0);
            for (e, x) in r.iter().enumerate().filter(|(e, x)| x.0[l] > 0.0 && *e != last) {
                let part = got * x.0[l] / all;
                (taken[e][l], left) = (taken[e][l] + part, left - part);
            }
            taken[last][l] += left.max(0.0);
            s[l] -= got;
        }
    }
    (taken, searching)
}

/// What the slots of a cell can reach of its layers: the reachable stock of each layer, kg (the widest share any slot
/// reaches of what it shows above the snow), the energy to break a kg of it, J, and each slot's share of each layer's
/// reachable stock.
pub(super) struct Reach {
    pub(super) stock: Vec<f64>,
    pub(super) breaking: Vec<f64>,
    pub(super) share: Vec<Vec<f64>>,
}

/// What the slots of cell `i`, its members `bodies`, can reach of `layers` (`Reach`, `reach`).
pub(super) fn reachable(w: &World, i: usize, layers: &[Found], bodies: &[Body; FS]) -> Reach {
    let snow = contract::place(w, i).snow;
    let reach: Vec<Vec<f64>> = bodies.iter().map(|b| layers.iter().map(|f| reach(b, f, snow)).collect()).collect();
    let widest: Vec<f64> = (0..layers.len()).map(|l| reach.iter().map(|r| r[l]).fold(0.0, f64::max)).collect();
    let stock = layers.iter().zip(&widest).map(|(f, &r)| f.shown.kg * r).collect();
    let breaking = layers.iter().map(|f| f.shown.breaking / f.shown.kg).collect();
    let share = reach.iter().map(|r| r.iter().zip(&widest).map(|(r, &m)| if m > 0.0 { r / m } else { 0.0 }).collect()).collect();
    Reach { stock, breaking, share }
}

/// The slots of `bodies` as eaters of the scheme, each of its members at work the share `work` of its working time (a
/// slot cut to the share f works as f of its members would).
pub(super) fn eaters(bodies: &[Body; FS], share: &[Vec<f64>], work: &[f64; FS]) -> Vec<Eater> {
    (0..FS).map(|x| Eater { n: bodies[x].n * work[x], search: bodies[x].search, busy: bodies[x].busy, chew: bodies[x].chew, reach: share[x].clone() }).collect()
}

/// One bite of each of `layers` of cell `i` of chunk `d` for all the slots (`contract::take`), `taken` kg of each layer
/// per slot, shared by the slots' parts of it. Returns the mol of each substance each slot took, and the heat that came
/// with it, J.
pub(super) fn bite(w: &World, d: &mut Chunk, i: usize, layers: &[Found], taken: &[Vec<f64>]) -> ([[f64; MAX_SUBSTANCES]; FS], [f64; FS]) {
    let (mut mol, mut heat) = ([[0.0; MAX_SUBSTANCES]; FS], [0.0; FS]);
    for (l, f) in layers.iter().enumerate() {
        let want: f64 = taken.iter().map(|t| t[l]).sum();
        if want <= 0.0 {
            continue;
        }
        let got = contract::take(w, d, i, f.layer, want);
        for x in 0..FS {
            let share = taken[x][l] / want;
            mol[x].iter_mut().zip(&got.mol).for_each(|(a, b)| *a += b * share);
            heat[x] += got.heat * share;
        }
    }
    (mol, heat)
}

/// The share of what layer `f` shows above `snow` m of soft solid that a member `b` reaches: its band, over its bite's
/// floor (`contract::reached`).
fn reach(b: &Body, f: &Found, snow: f64) -> f64 {
    let shown = contract::above(snow, f.height);
    if b.n <= 0.0 || shown <= 0.0 { 0.0 } else { (contract::reached(b.band, b.floor, f.height) / shown).min(1.0) }
}
