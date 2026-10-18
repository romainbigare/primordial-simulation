//! Wind: gas and heat between neighbouring air nodes, along the links of the air level (`grid::AIR`): the pushes on
//! each link and the booking, here. Where the links form lines (the rows and columns of a square map) the wind is
//! solved along them (`along`, by `solve::lines`); on any other mesh by `solve::spread` on the links (`mesh`).

mod along;
mod mesh;

use super::super::Scratch;
use super::air_temp;
use crate::grid::AIR;
use crate::space::Level;
use crate::store::{apply, Batch, Class, LinkFlux};
use crate::units::R_GAS;
use crate::{Config, World};

/// Share of the pressure difference between two air nodes evened out per call.
const EVEN: f64 = 0.25;
/// Most of a node's amount the overturning may exchange over one link per call.
const OVER: f64 = 3.0;
/// A fit of a gradient whose neighbours nearly line up (the determinant below this share of the product of its
/// diagonal) is made along each axis alone.
const SINGULAR: f64 = 1e-9;

/// The air as the wind moves it within a call: per gas and node (`[gas * nodes + node]`) the amount, mol; per node the
/// heat, J, and the amount, mol, and heat capacity, J/K (as `Scratch` keeps them).
#[derive(Clone)]
struct State {
    x: Vec<f64>,
    e: Vec<f64>,
    nt: Vec<f64>,
    capa: Vec<f64>,
}

/// What a call moves over each link of the air level, + from its `a` to its `b`: each gas, mol (`[gas * links +
/// link]`), and heat, J; and the energy each node pays to lift the mass that enters it, J.
struct Moved {
    mol: Vec<f64>,
    heat: Vec<f64>,
    paid: Vec<f64>,
}

/// Per air node, from a `State`: the pressure of the column (p = nRT/V), Pa; the warmth at equal height, K; and the
/// gradient of the warmth [east, south], K/m.
struct Columns {
    p: Vec<f64>,
    warmth: Vec<f64>,
    grad: Vec<[f64; 2]>,
}

/// What the pushes on the links read.
struct Ctx<'a> {
    cfg: &'a Config,
    s: &'a Scratch,
    air: &'a Level,
    dt: f64,
}

/// Gas between neighbouring air nodes, by three pushes on each link: the pressure of the whole column drives gas from
/// high to low; the warmth difference at equal height turns the columns over (the lower half flows to the warmer node
/// and the upper half back, both turned sideways by rotation; condensable gases crowd in the lower half); eddies mix
/// the two columns. Solved implicitly, so any speed is stable; what crosses each link is booked on its two nodes
/// (`book`).
pub(crate) fn wind(w: &mut World, s: &mut Scratch, dt: f64) {
    // (The level is copied: the booking changes the world that holds it.)
    let air = w.grid.mesh().level(AIR).clone();
    let moved = blow(w, s, &air, dt);
    book(w, s, &air, &moved);
}

/// The wind of one call of `dt` seconds over the air level `air`, which has the nodes of the world's air level and
/// may have other links (`wind` uses the world's own).
pub fn wind_on(w: &mut World, air: &Level, dt: f64) {
    crate::flow::ensure(w);
    let mut s = std::mem::take(&mut w.scratch);
    let moved = blow(w, &mut s, air, dt);
    book(w, &mut s, air, &moved);
    w.scratch = s;
}

/// What the wind moves over the links of `air` in a call: along lines where they form lines (`along`), else on the
/// mesh (`mesh`); from the sums of the air nodes' amounts now (`air::sums`: Law 2 and bodies change them too).
fn blow(w: &World, s: &mut Scratch, air: &Level, dt: f64) -> Moved {
    super::sums(w, s);
    let na = air.n();
    let x = s.gas.iter().flat_map(|&k| w.air[k * na..(k + 1) * na].iter().copied()).collect();
    let start = State { x, e: w.e_air.clone(), nt: s.nt.clone(), capa: s.capa.clone() };
    let at = Ctx { cfg: &w.cfg, s, air, dt };
    match along::lines(air) {
        Some(lines) => along::along(&at, &lines, start),
        None => mesh::on_mesh(&at, &start),
    }
}

/// Book `m` on the world over the links of `air` through `store::apply`: the gas and heat, then the lift from the heat
/// of the node the mass enters into the world's potential energy (as `Sky::cross` books it); then the sums the state
/// keeps of each air node, from its amounts (`air::sums`).
fn book(w: &mut World, s: &mut Scratch, air: &Level, m: &Moved) {
    let ks: Vec<u16> = s.gas.iter().map(|&k| k as u16).collect();
    let mut lift = Batch::falls(Class::Air);
    m.paid.iter().enumerate().for_each(|(j, &q)| lift.fall(j, -q));
    apply(w, &[LinkFlux { class: [Class::Air; 2], a: air.a(), b: air.b(), ks: &ks, mol: &m.mol, heat: &m.heat }, lift.flux()], &[], &[]);
    super::sums(w, s);
    for (j, &e) in w.e_air.iter().enumerate() {
        s.ta[j] = air_temp(e, s.capa[j]);
    }
}

impl Ctx<'_> {
    /// Pressure and warmth of each node of `st` (at its temperature), and the gradient of the warmth.
    fn columns(&self, st: &State) -> Columns {
        let (s, na) = (self.s, self.air.n());
        let ta: Vec<f64> = (0..na).map(|j| air_temp(st.e[j], st.capa[j])).collect();
        let p = (0..na).map(|j| st.nt[j] * R_GAS * ta[j] / s.vol[j]).collect();
        let warmth: Vec<f64> = (0..na).map(|j| ta[j] + s.gam[j] * s.zair[j]).collect();
        let grad = gradient(self.air, &warmth);
        Columns { p, warmth, grad }
    }
    /// The three pushes on link `l`, + from its `a` to its `b`, mol/s: mixing (the eddies' `mix` across the link's
    /// width over its length), overturning (the `wind` speed through half the column across the link's width, no more
    /// than `OVER` times the smaller column per call) and the evening of pressure. The planet's rotation turns the
    /// overturning by the gradient of the warmth across the link, at the latitude of the link (the mean of its
    /// nodes').
    fn push(&self, st: &State, col: &Columns, l: usize) -> [f64; 3] {
        let (cfg, s, air, dt) = (self.cfg, self.s, self.air, self.dt);
        let (a, b, wd, d) = (air.a()[l] as usize, air.b()[l] as usize, air.width()[l], air.dist()[l]);
        let rho = (st.nt[a] / s.vol[a] + st.nt[b] / s.vol[b]) / 2.0;
        let depth = cfg.air_top_m - (s.za[a] + s.za[b]) / 2.0;
        let turn = cfg.rotation * ((air.lat()[a] + air.lat()[b]) / 2.0).to_radians().sin();
        let (ca, cb) = (air.centre()[a], air.centre()[b]);
        let (ga, gb) = (col.grad[a], col.grad[b]);
        let across = ((ga[0] + gb[0]) * (cb[1] - ca[1]) - (ga[1] + gb[1]) * (cb[0] - ca[0])) / 2.0;
        let cap = OVER * st.nt[a].min(st.nt[b]) / dt;
        let over = (cfg.wind * (col.warmth[b] - col.warmth[a] + turn * across) * rho * wd * depth / 2.0).clamp(-cap, cap);
        [cfg.mix * rho * depth * wd / d, over, self.even(st, col, a, b) * (col.p[a] - col.p[b])]
    }
    /// The conductance that evens the pressure of nodes `a` and `b` by `EVEN` of their difference over a call,
    /// mol/(Pa s): the amount that crosses changes their difference by the sum of their pressures per mol.
    fn even(&self, st: &State, col: &Columns, a: usize, b: usize) -> f64 {
        EVEN / ((col.p[a] / st.nt[a] + col.p[b] / st.nt[b]) * self.dt)
    }
    /// The mass `moved` from node `a` to node `b`, kg, is lifted between their mean heights: by the heat of the node
    /// it enters, J. Returns that node and the energy.
    fn lift(&self, a: usize, b: usize, moved: f64) -> (usize, f64) {
        let s = self.s;
        (if moved > 0.0 { b } else { a }, self.cfg.gravity * moved * (s.zair[b] - s.zair[a]))
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
