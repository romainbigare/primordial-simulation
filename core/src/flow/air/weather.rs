//! STUDY PROBE (branch `wip/years-study`, not a law of main): weather, the synoptic eddies the air's 15-day step
//! cannot hold, as a random field with a physical shape. A field of standard normals on a lattice of places
//! `WEATHER_M` apart on the planet (the map's metres times its stretch), red in time with the eddies' time scale
//! `EDDY_DAYS` (an AR(1) process, its past days summed from keyed draws, so it needs no state), keyed by the seed, the
//! day since the start, the place and the field, so it is the same on any thread count and in any order. Between the
//! places a point reads the four about it bilinearly, the weights normalised so its spread stays one.
//!
//! Which probe runs is read from the environment variable `EW_YEARS_PROBE` (A, B, C, AC; none: the base): a study
//! switch, never for main.

use super::super::Scratch;
use super::cell::Cell;
use super::edge::{beyond_offset, Edges};
use super::wind::{End, Links, Pushes};
use crate::rng::{Purpose, Rng};
use crate::{units::DAY, World, AIR};
use std::collections::HashMap;

/// The distance between the places of the weather field, m on the planet: the synoptic scale, about the Rossby radius
/// of deformation of the middle latitudes, 1000 km (Gill 1982, from memory).
const WEATHER_M: f64 = 1.0e6;
/// The eddies' time scale, days: the decorrelation time of synoptic weather at a place, 2 to 4 days (Lorenz 1969;
/// Blackmon 1976, the 2.5 to 6 day band; from memory).
const EDDY_DAYS: f64 = 3.0;
/// Past days run from rest before a step, in e-folds of the longest time scale read.
const FOLDS: f64 = 4.0;
/// The lognormal spread of the eddies' daily mixing (probe A): the daily eddy heat flux at a place has a coefficient
/// of variation of about 1 to 1.5, a few strong events carrying much of it (Swanson and Pierrehumbert 1997; Messori
/// and Czaja 2013; from memory): sigma 1 gives a coefficient of variation of 1.31.
const MIX_SIGMA: f64 = 1.0;
/// Depth of the planet's sea's top layer the slow part reads, m (`Physics::h_mix`'s anchor, de Boyer Montégut 2004).
const SEA_TOP_M: f64 = 50.0;
/// The air-sea feedback that damps a top layer's warmth anomaly, W/m²/K (Frankignoul and Hasselmann 1977;
/// Frankignoul 1985: 15 to 25; from memory).
const SEA_DAMP: f64 = 20.0;
/// Volumetric heat capacity of the sea's liquid, J/m³/K.
const SEA_RHO_CP: f64 = 4.1e6;

/// The fields of the weather, each its own stream.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) enum Field {
    /// The north-south displacement of the air by the eddies (probes B and C)
    Shift = 0,
    /// The strength of the eddies' mixing (probe A)
    Mix = 1,
}

/// Which probe runs (`EW_YEARS_PROBE`).
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Probe {
    pub(super) mix: bool,
    pub(super) shift: bool,
    pub(super) slow: bool,
}

impl Probe {
    pub(super) fn of(w: &World) -> Probe {
        // The planet's strip (a globe) is built without weather: its table stays the base's.
        if w.map.wrap {
            return Probe::default();
        }
        let v = std::env::var("EW_YEARS_PROBE").unwrap_or_default();
        Probe { mix: v.contains('A'), shift: v.contains('B') || v.contains('C'), slow: v.contains('C') }
    }
    pub(super) fn any(self) -> bool {
        self.mix || self.shift
    }
}

/// The weather of one air step: per place of the lattice and field, the keyed draws of the days the step reads.
pub(super) struct Weather {
    seed: u64,
    first: i64,
    days: usize,
    stretch: f64,
    past: usize,
    phi_fast: f64,
    phi_slow: f64,
    slow_gain: f64,
    cache: std::cell::RefCell<HashMap<((i64, i64), Field), Vec<f64>>>,
    pub(super) probe: Probe,
}

impl Weather {
    /// The weather of the step of `days` days that starts now in `w`, its stretch `stretch`.
    pub(super) fn of(w: &World, days: usize, stretch: f64) -> Weather {
        let probe = Probe::of(w);
        let phi_fast = (-1.0 / EDDY_DAYS).exp();
        // The slow part: the planet's sea's top layer sums the same weather and lets it go in tau = rho cp h / lambda
        // (Hasselmann 1976). Forced by the fast part, its spread over the fast part's is sqrt(tau_fast / tau_slow) for
        // tau_fast << tau_slow.
        let tau_slow = SEA_RHO_CP * SEA_TOP_M / SEA_DAMP / DAY;
        let phi_slow = (-1.0 / tau_slow).exp();
        let slow_gain = if probe.slow { (EDDY_DAYS / tau_slow).sqrt() } else { 0.0 };
        let past = (FOLDS * if probe.slow { tau_slow } else { EDDY_DAYS }).ceil() as usize;
        Weather { seed: w.seed, first: (w.time as f64 / DAY).round() as i64, days, stretch, past, phi_fast, phi_slow, slow_gain, cache: Default::default(), probe }
    }

    /// The keyed white draws of a place and field for the step's days and the past days it sums, oldest first.
    fn draws(&self, place: (i64, i64), field: Field) -> Vec<f64> {
        let key = (place.0 as u64).wrapping_mul(0x9e37_79b9) ^ (place.1 as u64).wrapping_mul(0x85eb_ca6b).rotate_left(21) ^ ((field as u64) << 58);
        (0..self.past + self.days).map(|k| Rng::new(self.seed, (self.first - self.past as i64 + k as i64) as u64, key, Purpose::Weather).normal()).collect()
    }

    /// The red field at a place for each day of the step: its fast part (AR(1) of `EDDY_DAYS`, unit spread) and, in
    /// probe C, for the displacement, the slow part the sea sums, both run over the past days from rest.
    fn series(&self, place: (i64, i64), field: Field) -> Vec<f64> {
        let e = self.draws(place, field);
        let (nf, ns) = ((1.0 - self.phi_fast * self.phi_fast).sqrt(), (1.0 - self.phi_slow * self.phi_slow).sqrt());
        let slow_on = self.slow_gain > 0.0 && field == Field::Shift;
        let (mut fast, mut slow) = (0.0, 0.0);
        let mut out = Vec::with_capacity(self.days);
        for (k, &x) in e.iter().enumerate() {
            fast = self.phi_fast * fast + nf * x;
            if slow_on {
                slow = self.phi_slow * slow + ns * x;
            }
            if k >= self.past {
                out.push(fast + self.slow_gain * slow);
            }
        }
        out
    }

    /// The red field at a place for day `d` of the step.
    fn at_place(&self, place: (i64, i64), field: Field, d: usize) -> f64 {
        self.cache.borrow_mut().entry((place, field)).or_insert_with(|| self.series(place, field))[d]
    }

    /// The field at a point of the map, m [east, south] from its north-west corner, on day `d` of the step: unit spread.
    pub(super) fn at(&self, xy: [f64; 2], field: Field, d: usize) -> f64 {
        let (x, y) = (xy[0] * self.stretch / WEATHER_M, xy[1] * self.stretch / WEATHER_M);
        let (ix, iy) = (x.floor(), y.floor());
        let (fx, fy) = (x - ix, y - iy);
        let w = [(1.0 - fx) * (1.0 - fy), fx * (1.0 - fy), (1.0 - fx) * fy, fx * fy];
        let places = [(ix as i64, iy as i64), (ix as i64 + 1, iy as i64), (ix as i64, iy as i64 + 1), (ix as i64 + 1, iy as i64 + 1)];
        let norm = w.iter().map(|v| v * v).sum::<f64>().sqrt().max(1e-9);
        places.iter().zip(w).map(|(&p, wt)| wt * self.at_place(p, field, d)).sum::<f64>() / norm
    }

    /// The eddies' mixing factor at a point on day `d`: lognormal of mean one (probe A); one without it.
    pub(super) fn mix(&self, xy: [f64; 2], d: usize) -> f64 {
        if !self.probe.mix {
            return 1.0;
        }
        (MIX_SIGMA * self.at(xy, Field::Mix, d) - MIX_SIGMA * MIX_SIGMA / 2.0).exp()
    }

    /// The eddies' north-south displacement of the air at a point on day `d`, m on the planet (probes B and C): the
    /// mixing length sqrt(`mix` × `EDDY_DAYS`) (Prandtl 1925; Taylor 1922: a diffusivity is a velocity times a length,
    /// the length the eddy moves air in its time scale) times the field; none without them.
    pub(super) fn shift(&self, xy: [f64; 2], d: usize, mix: f64) -> f64 {
        if !self.probe.shift {
            return 0.0;
        }
        (mix * EDDY_DAYS * DAY).sqrt() * self.at(xy, Field::Shift, d)
    }
}

/// The pushes of day `d` (probe A): each link between blocks (`border`) with its eddies' mixing, and the heat they
/// carry, times the day's factor at its middle (`Weather::mix`).
pub(super) fn day_pushes(w: &World, wx: &Weather, push: &Pushes, (links, border): (&Links, &[usize]), d: usize) -> Pushes {
    let mut p = push.clone();
    for &l in border {
        let (a, b) = (node_xy(w, links.a[l] as usize), node_xy(w, links.b[l] as usize));
        let f = wx.mix([(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0], d);
        p.mix[l] *= f;
        p.stir[l] *= f;
    }
    p
}

/// The weather of day `d` on the links across the map's open edges (`Edges::weather`): at each link's middle the
/// eddies' factor (probe A), and at its column beyond the eddies' displacement (probes B, C), in degrees of latitude.
pub(super) fn edge_day(w: &World, s: &Scratch, wx: &Weather, (ends, cell): (&[End], &Cell), edges: &mut Edges, base: &[[f64; 4]], d: usize) {
    let per_degree = w.cfg.radius_m * std::f64::consts::PI / 180.0;
    let (mut shift, mut mixf) = (Vec::with_capacity(edges.node.len()), Vec::with_capacity(edges.node.len()));
    for (&j, &side) in edges.node.iter().zip(&edges.side) {
        let (xy, sp) = (node_xy(w, j as usize), w.mesh.level(AIR).area()[j as usize].sqrt());
        let off = beyond_offset(side, sp);
        mixf.push(wx.mix([xy[0] + off[0] / 2.0, xy[1] + off[1] / 2.0], d));
        shift.push(wx.shift([xy[0] + off[0], xy[1] + off[1]], d, w.cfg.mix) / per_degree);
    }
    edges.weather(w, s, (ends, cell), (&shift, &mixf), base);
}

/// The middle of air node `j` on the map, m [east, south] from its north-west corner.
pub(super) fn node_xy(w: &World, j: usize) -> [f64; 2] {
    let (bx, by) = w.mesh.tree().block_xy(j);
    let sp = w.mesh.level(AIR).area()[j].sqrt();
    [(bx as f64 + 0.5) * sp, (by as f64 + 0.5) * sp]
}
