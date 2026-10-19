//! Wind: gas and heat between neighbouring air nodes, along the links of the air level (`AIR`) that share a border.
//! Here the three pushes on each link and what they carry over a step, as the coefficients of an implicit step
//! (`solve::band`); the air step (`step`) solves them with the exchange with the ground.

use super::super::Scratch;
use crate::solve::band::Carry;
use crate::space::Level;
use crate::units::R_GAS;
use crate::Config;

/// A fit of a gradient whose neighbours nearly line up (the determinant below this share of the product of its
/// diagonal) is made along each axis alone.
const SINGULAR: f64 = 1e-9;

/// The coefficients `(fwd, back)` of a transport over the links of `Links`, per link (`solve::band::Carry`).
pub(super) type Coef = (Vec<f64>, Vec<f64>);

/// The links of the air level that share a border (width above 0), with their ends.
pub(super) struct Links {
    pub(super) id: Vec<usize>,
    pub(super) a: Vec<u32>,
    pub(super) b: Vec<u32>,
}

impl Links {
    pub(super) fn of(air: &Level) -> Links {
        let id: Vec<usize> = (0..air.a().len()).filter(|&l| air.width()[l] > 0.0).collect();
        let (a, b) = id.iter().map(|&l| (air.a()[l], air.b()[l])).unzip();
        Links { id, a, b }
    }
    /// The coefficients `(fwd, back)` of a transport over these links, for `solve::band`.
    pub(super) fn carry<'a>(&'a self, (fwd, back): &'a Coef) -> Carry<'a> {
        Carry { a: &self.a, b: &self.b, fwd, back }
    }
}

/// The air of each node as the wind sees it: the amount, mol, the heat capacity, J/K, the mass, kg, and the
/// temperature, K.
pub(super) struct Air<'a> {
    pub(super) n: &'a [f64],
    pub(super) capa: &'a [f64],
    pub(super) mass: &'a [f64],
    pub(super) t: &'a [f64],
}

/// The pushes on each link (`Links`), + from its `a` to its `b`, mol/s: mixing by eddies, overturning, and the
/// conductance of the evening of pressure, mol/(Pa s).
pub(super) struct Pushes {
    mix: Vec<f64>,
    over: Vec<f64>,
    even: Vec<f64>,
}

/// Three pushes on each link: the pressure of the whole column (p = nRT/V) drives gas from high to low, evened in the
/// time sound takes to cross the link (the pressure of neighbouring columns evens out in about that time: Lamb waves
/// travel at the speed of sound, Lindzen 1990, 9.2); the warmth difference at equal height turns the columns over (the
/// lower half flows to the warmer node and the upper half back, at the `wind` speed per K through half the column
/// across the link's width, both turned sideways by the planet's rotation by the gradient of the warmth across the
/// link, at its latitude); eddies mix the two columns (`mix` across the link's width over its length).
pub(super) fn pushes(cfg: &Config, s: &Scratch, air: &Level, links: &Links, at: &Air) -> Pushes {
    let na = air.n();
    let warmth: Vec<f64> = (0..na).map(|j| at.t[j] + s.gam[j] * s.zair[j]).collect();
    let grad = gradient(air, &warmth);
    let mut p = Pushes { mix: Vec::with_capacity(links.id.len()), over: Vec::with_capacity(links.id.len()), even: Vec::with_capacity(links.id.len()) };
    for &l in &links.id {
        let (a, b, wd, d) = (air.a()[l] as usize, air.b()[l] as usize, air.width()[l], air.dist()[l]);
        let rho = (at.n[a] / s.vol[a] + at.n[b] / s.vol[b]) / 2.0;
        let depth = cfg.air_top_m - (s.za[a] + s.za[b]) / 2.0;
        let turn = cfg.rotation * ((air.lat()[a] + air.lat()[b]) / 2.0).to_radians().sin();
        let (ca, cb) = (air.centre()[a], air.centre()[b]);
        let (ga, gb) = (grad[a], grad[b]);
        let across = ((ga[0] + gb[0]) * (cb[1] - ca[1]) - (ga[1] + gb[1]) * (cb[0] - ca[0])) / 2.0;
        p.mix.push(cfg.mix * rho * depth * wd / d);
        p.over.push(cfg.wind * (warmth[b] - warmth[a] + turn * across) * rho * wd * depth / 2.0);
        // p = c n with c = RT/V; sound crosses the link in d / sqrt(gamma R T / M), gamma = cp / (cp - R) per mol.
        let c = |j: usize| R_GAS * at.t[j] / s.vol[j];
        let sound = |j: usize| {
            let cp = at.capa[j] / at.n[j];
            (cp / (cp - R_GAS) * R_GAS * at.t[j] * at.n[j] / at.mass[j]).sqrt()
        };
        p.even.push((sound(a) + sound(b)) / 2.0 / (d * (c(a) + c(b))));
    }
    p
}

impl Pushes {
    /// The coefficients of the evening of pressure over `dt` for the total amounts (`solve::band`).
    pub(super) fn pressure(&self, s: &Scratch, links: &Links, t: &[f64], dt: f64) -> Coef {
        let c = |j: u32| R_GAS * t[j as usize] / s.vol[j as usize];
        let fwd = links.a.iter().zip(&self.even).map(|(&a, &g)| dt * g * c(a)).collect();
        let back = links.b.iter().zip(&self.even).map(|(&b, &g)| dt * g * c(b)).collect();
        (fwd, back)
    }
    /// The coefficients over `dt` of a gas whose share in the upper half of each node is `split` (`Scratch::split`),
    /// given the total amounts after the step `n`, the gas of each node's [lower, upper] half `halves` and what the
    /// pressure moves over each link over the step, `flow`, mol: mixing evens its share of the gas, the overturning carries
    /// the share of each half that is this gas, and the pressure's flow carries its share of the node it leaves. With
    /// `split` None, the heat: each half carries its node's heat per mol.
    pub(super) fn carry(&self, links: &Links, n: &[f64], (split, halves): (Option<&[f64]>, &[[f64; 2]]), flow: &[f64], dt: f64) -> Coef {
        let (mut fwd, mut back) = (Vec::with_capacity(flow.len()), Vec::with_capacity(flow.len()));
        for (l, (&a, &b)) in links.a.iter().zip(&links.b).enumerate() {
            let (a, b, o, f) = (a as usize, b as usize, self.over[l], flow[l]);
            let (op, om) = (o.max(0.0), (-o).max(0.0));
            let (to_b, to_a) = match split {
                Some(up) => (op * (1.0 - up[a]) / halves[a][0] + om * up[a] / halves[a][1], op * up[b] / halves[b][1] + om * (1.0 - up[b]) / halves[b][0]),
                None => (o.abs() / n[a], o.abs() / n[b]),
            };
            fwd.push((dt * self.mix[l] + f.max(0.0)) / n[a] + dt * to_b);
            back.push((dt * self.mix[l] + (-f).max(0.0)) / n[b] + dt * to_a);
        }
        (fwd, back)
    }
}

/// The gradient of `v` at each node of `air` [east, south], per m: the least-squares fit of its differences to the
/// nodes it shares a border with (links of width > 0). Along an axis with no such neighbour it is 0.
fn gradient(air: &Level, v: &[f64]) -> Vec<[f64; 2]> {
    let c = air.centre();
    (0..air.n())
        .map(|j| {
            let (mut m, mut r) = ([0.0; 3], [0.0; 2]);
            for (&l, &k) in air.around(j).0.iter().zip(air.around(j).1) {
                if air.width()[l as usize] > 0.0 {
                    let (d, dv) = ([c[k as usize][0] - c[j][0], c[k as usize][1] - c[j][1]], v[k as usize] - v[j]);
                    m = [m[0] + d[0] * d[0], m[1] + d[0] * d[1], m[2] + d[1] * d[1]];
                    r = [r[0] + d[0] * dv, r[1] + d[1] * dv];
                }
            }
            let det = m[0] * m[2] - m[1] * m[1];
            if det > SINGULAR * m[0] * m[2] {
                [(m[2] * r[0] - m[1] * r[1]) / det, (m[0] * r[1] - m[1] * r[0]) / det]
            } else {
                [r[0] / m[0].max(f64::MIN_POSITIVE), r[1] / m[2].max(f64::MIN_POSITIVE)]
            }
        })
        .collect()
}
