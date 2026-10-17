//! Wind: gas and heat between neighbouring air nodes, solved implicitly along lines of nodes.

use super::super::Scratch;
use super::air_temp;
use crate::grid::Grid;
use crate::units::R_GAS;
use crate::{Config, World};

/// Share of the pressure difference between two air nodes evened out per call.
const EVEN: f64 = 0.25;
/// Most of a node's amount the overturning may exchange over one edge per call.
const OVER: f64 = 3.0;

/// Solve x' + (out − in) = x along a line of nodes, in place, where edge p (between nodes
/// p and p + 1) carries al[p]·x'[p] − be[p]·x'[p + 1]. `c` is work space.
fn thomas(al: &[f64], be: &[f64], x: &mut [f64], c: &mut [f64]) {
    let len = x.len();
    for p in 0..len {
        let lo = if p > 0 { -al[p - 1] } else { 0.0 };
        let di = 1.0 + if p > 0 { be[p - 1] } else { 0.0 } + if p + 1 < len { al[p] } else { 0.0 };
        let m = di - if p > 0 { lo * c[p - 1] } else { 0.0 };
        c[p] = if p + 1 < len { -be[p] / m } else { 0.0 };
        x[p] = (x[p] - if p > 0 { lo * x[p - 1] } else { 0.0 }) / m;
    }
    for p in (0..len - 1).rev() {
        x[p] -= c[p] * x[p + 1];
    }
}

/// Work space of the solver for one line of air nodes: per edge the three pushes [mixing,
/// overturning, pressure], mol/s; per node its gas in the [lower, upper] half, mol; the shares
/// each edge carries on and back; the amounts solved and the solver's work space; the amount of
/// each gas solved, [gas * len + node].
#[derive(Default)]
struct Lane {
    push: Vec<[f64; 3]>,
    halves: Vec<[f64; 2]>,
    to_next: Vec<f64>,
    to_prev: Vec<f64>,
    x: Vec<f64>,
    c: Vec<f64>,
    gas: Vec<f64>,
}

/// The lines of air nodes of one direction (0: east-west, 1: north-south), and what their
/// solver reads. A line's row holds per edge the flux of each gas, mol, [gas * len + edge]; then
/// the heat moved, J; the energy spent lifting, J; and the mass moved, kg.
struct Lines<'a> {
    cfg: &'a Config,
    grid: &'a Grid,
    s: &'a Scratch,
    air: &'a [f64],
    e_air: &'a [f64],
    dir: usize,
    len: usize,
    dt: f64,
}

impl Lines<'_> {
    /// Air node `p` of line `line`.
    fn node(&self, line: usize, p: usize) -> usize {
        if self.dir == 0 { line * self.grid.aw() + p } else { p * self.grid.aw() + line }
    }
    /// Offsets in a row of the fluxes of gas, heat, lift and mass, and the row's width.
    fn layout(&self) -> (usize, usize, usize, usize, usize) {
        let (ng, len) = (self.s.gas.len(), self.len);
        (0, ng * len, (ng + 1) * len, (ng + 2) * len, (ng + 3) * len)
    }
    /// Solve line `line` into `row`.
    fn solve(&self, ln: &mut Lane, line: usize, row: &mut [f64]) {
        let (len, ng) = (self.len, self.s.gas.len());
        ln.push.resize(len, [0.0; 3]);
        ln.halves.resize(len, [0.0; 2]);
        for v in [&mut ln.to_next, &mut ln.to_prev, &mut ln.x, &mut ln.c] {
            v.resize(len, 0.0);
        }
        ln.gas.resize(ng * len, 0.0);
        self.pushes(ln, line);
        self.gases(ln, line, row);
        self.heat(ln, line, row);
    }
    /// The three pushes on each edge of the line.
    fn pushes(&self, ln: &mut Lane, line: usize) {
        let (cfg, grid, sc, dir, dt) = (self.cfg, self.grid, self.s, self.dir, self.dt);
        let side = grid.cell_m * grid.air_scale as f64;
        for p in 0..self.len - 1 {
            let (a, b) = (self.node(line, p), self.node(line, p + 1));
            let rho = (sc.nt[a] / sc.vol[a] + sc.nt[b] / sc.vol[b]) / 2.0;
            let depth = cfg.air_top_m - (sc.za[a] + sc.za[b]) / 2.0;
            let lat_row = if dir == 0 { line * grid.air_scale + grid.air_scale / 2 } else { (p + 1) * grid.air_scale };
            let turn = cfg.rotation * grid.lat(lat_row.min(grid.h - 1), cfg.lat_south, cfg.lat_north).to_radians().sin();
            let across = (sc.grad[a][1 - dir] + sc.grad[b][1 - dir]) / 2.0;
            let cap = OVER * sc.nt[a].min(sc.nt[b]) / dt;
            let over = (cfg.wind * (sc.warmth[b] - sc.warmth[a] + turn * across) * rho * side * depth / 2.0).clamp(-cap, cap);
            let even = EVEN * (sc.node_tmp[a] - sc.node_tmp[b]) / (sc.node_tmp[a] / sc.nt[a] + sc.node_tmp[b] / sc.nt[b]) / dt;
            ln.push[p] = [cfg.mix * rho * depth, over, even];
        }
    }
    /// Each gas along the line, split into lower and upper halves by the amounts before the
    /// step. Overturning moves as much up as down: any net amount beyond the pressure push is
    /// handed back. Also the mass each edge moves.
    fn gases(&self, ln: &mut Lane, line: usize, row: &mut [f64]) {
        let (sc, len, ng, na, dt) = (self.s, self.len, self.s.gas.len(), self.grid.na(), self.dt);
        let (flux, _, _, moved, _) = self.layout();
        for p in 0..len {
            ln.halves[p] = [0.0; 2];
            for t in 0..ng {
                let j = self.node(line, p);
                let a = self.air[sc.gas[t] * na + j];
                let u = a * sc.split[t * na + j];
                ln.halves[p][0] += a - u;
                ln.halves[p][1] += u;
            }
        }
        for t in 0..ng {
            for p in 0..len - 1 {
                let (a, b) = (self.node(line, p), self.node(line, p + 1));
                let [mix, over, even] = ln.push[p];
                let (fp, fm, ep, em) = (over.max(0.0), (-over).max(0.0), even.max(0.0), (-even).max(0.0));
                let ([la, ua], [lb, ub]) = (ln.halves[p], ln.halves[p + 1]);
                let (sa, sb) = (sc.split[t * na + a], sc.split[t * na + b]);
                ln.to_next[p] = dt * ((mix + ep) / sc.nt[a] + fp * (1.0 - sa) / la + fm * sa / ua);
                ln.to_prev[p] = dt * ((mix + em) / sc.nt[b] + fp * sb / ub + fm * (1.0 - sb) / lb);
            }
            let k = sc.gas[t];
            for p in 0..len {
                ln.x[p] = self.air[k * na + self.node(line, p)];
            }
            thomas(&ln.to_next[..len - 1], &ln.to_prev[..len - 1], &mut ln.x[..len], &mut ln.c[..len]);
            ln.gas[t * len..(t + 1) * len].copy_from_slice(&ln.x[..len]);
            for p in 0..len - 1 {
                row[flux + t * len + p] = ln.to_next[p] * ln.x[p] - ln.to_prev[p] * ln.x[p + 1];
            }
        }
        for p in 0..len {
            ln.x[p] = (0..ng).map(|t| ln.gas[t * len + p]).sum();
        }
        for p in 0..len - 1 {
            let (a, b, even) = (self.node(line, p), self.node(line, p + 1), ln.push[p][2]);
            let want = dt * (even.max(0.0) * ln.x[p] / sc.nt[a] - (-even).max(0.0) * ln.x[p + 1] / sc.nt[b]);
            let extra = (0..ng).map(|t| row[flux + t * len + p]).sum::<f64>() - want;
            let src = if extra > 0.0 { p + 1 } else { p };
            for t in 0..ng {
                row[flux + t * len + p] -= extra * ln.gas[t * len + src] / ln.x[src];
            }
        }
        for p in 0..len - 1 {
            row[moved + p] = 0.0;
            for t in 0..ng {
                row[moved + p] += row[flux + t * len + p] * sc.sp[sc.gas[t]].mm;
            }
        }
    }
    /// Heat moves with the gas, per mol its node's mean; mass moved up or down is lifted by the
    /// heat of the node it enters.
    fn heat(&self, ln: &mut Lane, line: usize, row: &mut [f64]) {
        let (sc, len, dt, g) = (self.s, self.len, self.dt, self.cfg.gravity);
        let (_, heat, lifted, moved, _) = self.layout();
        for p in 0..len - 1 {
            let (a, b, [mix, over, even]) = (self.node(line, p), self.node(line, p + 1), ln.push[p]);
            ln.to_next[p] = dt * (mix + over.abs() + even.max(0.0)) / sc.nt[a];
            ln.to_prev[p] = dt * (mix + over.abs() + (-even).max(0.0)) / sc.nt[b];
        }
        for p in 0..len {
            ln.x[p] = self.e_air[self.node(line, p)];
        }
        thomas(&ln.to_next[..len - 1], &ln.to_prev[..len - 1], &mut ln.x[..len], &mut ln.c[..len]);
        for p in 0..len - 1 {
            let (a, b) = (self.node(line, p), self.node(line, p + 1));
            row[heat + p] = ln.to_next[p] * ln.x[p] - ln.to_prev[p] * ln.x[p + 1];
            row[lifted + p] = g * row[moved + p] * (sc.zair[b] - sc.zair[a]);
        }
    }
}

/// Gas between neighbouring air nodes, by three pushes on each edge: the pressure of the whole
/// column (p = nRT/V) drives gas from high to low; the warmth difference at equal height turns the
/// columns over (the lower half flows to the warmer node and the upper half back, both turned
/// sideways by rotation; condensable gases crowd in the lower half); eddies mix the two columns.
/// Solved implicitly along lines, so any speed is stable. A call is long against the time the air
/// takes to mix from node to node, so one order of the two directions would leave the air with
/// stripes along the lines solved last. So the lines are solved in both orders from the same start
/// (`sweep`), and the air keeps the mean of the two (symmetrically weighted sequential splitting:
/// Strang 1968; Csomós, Faragó & Havasi 2005).
pub(crate) fn wind(w: &mut World, s: &mut Scratch, dt: f64) {
    let start = (w.air.clone(), w.e_air.clone(), s.nt.clone(), s.capa.clone(), s.mass.clone());
    let lift = sweep(w, s, dt, [0, 1]);
    let one = (
        std::mem::replace(&mut w.air, start.0),
        std::mem::replace(&mut w.e_air, start.1),
        std::mem::replace(&mut s.nt, start.2),
        std::mem::replace(&mut s.capa, start.3),
        std::mem::replace(&mut s.mass, start.4),
    );
    let lift = (lift + sweep(w, s, dt, [1, 0])) / 2.0;
    for (now, other) in [(&mut w.air, &one.0), (&mut w.e_air, &one.1), (&mut s.nt, &one.2), (&mut s.capa, &one.3), (&mut s.mass, &one.4)] {
        now.iter_mut().zip(other).for_each(|(x, y)| *x = (*x + y) / 2.0);
    }
    w.potential += lift;
    for (j, &e) in w.e_air.iter().enumerate().take(w.grid.na()) {
        s.ta[j] = air_temp(e, s.capa[j]);
    }
}

/// The lines of both directions in `order` (0: east-west, 1: north-south), each direction from the
/// amounts the one before left. The lines of one direction share no node, so each is solved from
/// the amounts before the direction (in parallel, with `threads`) into a row per line (`Lines`);
/// the rows are applied in the order of the lines. Returns the energy spent lifting, J.
fn sweep(w: &mut World, s: &mut Scratch, dt: f64, order: [usize; 2]) -> f64 {
    let World { cfg, grid, air, e_air, .. } = w;
    let (na, aw, ah, ng) = (grid.na(), grid.aw(), grid.ah(), s.gas.len());
    let mut rows = std::mem::take(&mut s.lanes);
    let mut lift = 0.0;
    for dir in order {
        s.nt_new.clone_from(&s.nt);
        warmth(s, e_air, aw, ah);
        let (lines, len) = if dir == 0 { (ah, aw) } else { (aw, ah) };
        let at = Lines { cfg, grid, s: &*s, air: &*air, e_air: &*e_air, dir, len, dt };
        let (flux, heat, lifted, moved, width) = at.layout();
        rows.resize(lines * width, 0.0);
        crate::par::rows(&mut rows, width, Lane::default, |ln, line, row| at.solve(ln, line, row));
        let node = |line: usize, p: usize| if dir == 0 { line * aw + p } else { p * aw + line };
        for (line, row) in rows.chunks(width).enumerate() {
            for t in 0..ng {
                let (k, sp) = (s.gas[t], s.sp[s.gas[t]]);
                for p in 0..len - 1 {
                    let (a, b) = (node(line, p), node(line, p + 1));
                    // (A node gives no more than it holds: the solve keeps every amount at 0 or more but for rounding.)
                    let f = row[flux + t * len + p].clamp(-air[k * na + b], air[k * na + a]);
                    air[k * na + a] -= f;
                    air[k * na + b] += f;
                    (s.nt_new[a], s.nt_new[b]) = (s.nt_new[a] - f, s.nt_new[b] + f);
                    (s.capa[a], s.capa[b]) = (s.capa[a] - f * sp.cp, s.capa[b] + f * sp.cp);
                    (s.mass[a], s.mass[b]) = (s.mass[a] - f * sp.mm, s.mass[b] + f * sp.mm);
                }
            }
            for p in 0..len - 1 {
                let (a, b, f, l) = (node(line, p), node(line, p + 1), row[heat + p], row[lifted + p]);
                e_air[a] -= f;
                e_air[b] += f;
                e_air[if row[moved + p] > 0.0 { b } else { a }] -= l;
                lift += l;
            }
        }
        s.nt.clone_from(&s.nt_new);
    }
    s.lanes = rows;
    lift
}

/// Temperature, pressure (in `node_tmp`, Pa) and warmth at equal height of each air node, and the
/// gradient of the warmth [east, north], K per node.
fn warmth(s: &mut Scratch, e_air: &[f64], aw: usize, ah: usize) {
    let na = aw * ah;
    for (j, &e) in e_air.iter().enumerate().take(na) {
        s.ta[j] = air_temp(e, s.capa[j]);
        s.node_tmp[j] = s.nt[j] * R_GAS * s.ta[j] / s.vol[j];
        s.warmth[j] = s.ta[j] + s.gam[j] * s.zair[j];
    }
    for j in 0..na {
        let (x, y) = (j % aw, j / aw);
        let (e, wst) = (if x + 1 < aw { j + 1 } else { j }, if x > 0 { j - 1 } else { j });
        let (no, so) = (if y > 0 { j - aw } else { j }, if y + 1 < ah { j + aw } else { j });
        s.grad[j] = [(s.warmth[e] - s.warmth[wst]) / (e - wst).max(1) as f64, (s.warmth[no] - s.warmth[so]) / ((so - no) / aw).max(1) as f64];
    }
}
