//! Wind: what moves air between neighbouring air nodes (laws.md 1.6): the three pushes on each link as rates, from the
//! air of the step's start, which the air's days apply (`daily`).

use super::super::Scratch;
use crate::{space::Level, units::R_GAS, Config};

/// A fit of a gradient whose neighbours nearly line up (the determinant below this share of the product of its
/// diagonal) is made along each axis alone.
const SINGULAR: f64 = 1e-9;

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
}

/// The pushes on each link (`Links`): mixing by eddies, mol/s each way; overturning, mol/s of the lower half from its
/// `a` to its `b` (the upper half comes back); and the conductance of the evening of pressure, mol/(Pa s).
pub(super) struct Pushes {
    pub(super) mix: Vec<f64>,
    pub(super) over: Vec<f64>,
    pub(super) even: Vec<f64>,
}

/// Three pushes on each link: the column's pressure, evened in the time sound takes to cross it (Lamb waves, Lindzen
/// 1990, 9.2); overturning by the warmth difference, the lower half toward the warmer node at `wind` per K, turned by
/// the rotation at the link's latitude; and eddy mixing (`mix` × width / length). From the air of each node as the
/// last state left it (`Scratch::nt`, `capa`, `mass`, `ta`).
pub(super) fn pushes(cfg: &Config, s: &Scratch, air: &Level, links: &Links) -> Pushes {
    let na = air.n();
    let warmth: Vec<f64> = (0..na).map(|j| s.ta[j] + s.gam[j] * s.zair[j]).collect();
    let grad = gradient(air, &warmth);
    let mut p = Pushes { mix: Vec::with_capacity(links.id.len()), over: Vec::with_capacity(links.id.len()), even: Vec::with_capacity(links.id.len()) };
    for &l in &links.id {
        let (a, b, wd, d) = (air.a()[l] as usize, air.b()[l] as usize, air.width()[l], air.dist()[l]);
        let (rho, depth) = ((s.nt[a] / s.vol[a] + s.nt[b] / s.vol[b]) / 2.0, cfg.air_top_m - (s.za[a] + s.za[b]) / 2.0);
        let turn = cfg.rotation * ((air.lat()[a] + air.lat()[b]) / 2.0).to_radians().sin();
        let (ab, ga, gb) = (air.offset(a, b), grad[a], grad[b]);
        let across = ((ga[0] + gb[0]) * ab[1] - (ga[1] + gb[1]) * ab[0]) / 2.0;
        p.mix.push(cfg.mix * rho * depth * wd / d);
        p.over.push(cfg.wind * (warmth[b] - warmth[a] + turn * across) * rho * wd * depth / 2.0);
        // p = c n with c = RT/V; sound crosses the link in d / sqrt(gamma R T / M), gamma = cp / (cp - R) per mol.
        let c = |j: usize| R_GAS * s.ta[j] / s.vol[j];
        let sound = |j: usize| {
            let cp = s.capa[j] / s.nt[j];
            (cp / (cp - R_GAS) * R_GAS * s.ta[j] * s.nt[j] / s.mass[j]).sqrt()
        };
        p.even.push((sound(a) + sound(b)) / 2.0 / (d * (c(a) + c(b))));
    }
    p
}

/// The gradient of `v` at each node [east, south], per m: the least-squares fit of its differences to its bordering
/// nodes (0 along an axis with none).
fn gradient(air: &Level, v: &[f64]) -> Vec<[f64; 2]> {
    (0..air.n())
        .map(|j| {
            let (mut m, mut r) = ([0.0; 3], [0.0; 2]);
            for (&l, &k) in air.around(j).0.iter().zip(air.around(j).1) {
                if air.width()[l as usize] > 0.0 {
                    let (d, dv) = (air.offset(j, k as usize), v[k as usize] - v[j]);
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
