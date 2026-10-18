//! The wind along lines: the links of the air level as lines of nodes, each solved implicitly in one piece
//! (`solve::lines`).

use super::{Ctx, Moved, State};
use crate::solve::lines::along_line;
use crate::space::Level;

/// No link (`u32::MAX`).
const NO_LINK: u32 = u32::MAX;

/// The wind of one call along `lines`, from `start`. The lines are solved one direction after the other. A call is long
/// against the time the air takes to mix from node to node, so one order of the two directions would leave the air with
/// stripes along the lines solved last: the lines are solved in both orders from the same start, and the air keeps the
/// mean of the two (symmetrically weighted sequential splitting: Strang 1968; Csomós, Faragó & Havasi 2005).
pub(super) fn along(at: &Ctx, lines: &[Vec<Line>; 2], start: State) -> Moved {
    let one = at.sweep(lines, start.clone(), [0, 1]);
    let two = at.sweep(lines, start, [1, 0]);
    let mean = |u: &[f64], v: &[f64]| u.iter().zip(v).map(|(p, q)| (p + q) / 2.0).collect();
    Moved { mol: mean(&one.mol, &two.mol), heat: mean(&one.heat, &two.heat), paid: mean(&one.paid, &two.paid) }
}

/// A line of air nodes in order, and the link from each to the next.
pub(super) struct Line {
    nodes: Vec<u32>,
    links: Vec<u32>,
}

impl Line {
    /// 1 where the line runs along its `p`-th link from the link's `a` to its `b`, -1 the other way.
    fn sign(&self, air: &Level, p: usize) -> f64 {
        if air.a()[self.links[p] as usize] == self.nodes[p] { 1.0 } else { -1.0 }
    }
}

/// The links of `air` that have a width, as the lines of two directions (0: more east-west than north-south, 1: the
/// other way), each line in the direction east or south. None when a node has more than one such link one way in a
/// direction, or a direction closes on itself: then the level is solved on the mesh (`mesh::on_mesh`).
pub(super) fn lines(air: &Level) -> Option<[Vec<Line>; 2]> {
    let (n, c) = (air.n(), air.centre());
    let (mut next, mut entered, mut count) = ([vec![NO_LINK; n], vec![NO_LINK; n]], [vec![false; n], vec![false; n]], [0; 2]);
    for l in (0..air.a().len()).filter(|&l| air.width()[l] > 0.0) {
        let (a, b) = (air.a()[l] as usize, air.b()[l] as usize);
        let d = [c[b][0] - c[a][0], c[b][1] - c[a][1]];
        let f = (d[1].abs() > d[0].abs()) as usize;
        let (from, to) = if d[f] > 0.0 { (a, b) } else { (b, a) };
        if next[f][from] != NO_LINK || entered[f][to] {
            return None;
        }
        (next[f][from], entered[f][to], count[f]) = (l as u32, true, count[f] + 1);
    }
    let lines = [0, 1].map(|f| {
        let starts = (0..n).filter(|&j| !entered[f][j] && next[f][j] != NO_LINK);
        starts.map(|mut j| {
            let mut line = Line { nodes: vec![j as u32], links: Vec::new() };
            while next[f][j] != NO_LINK {
                let l = next[f][j];
                j = (air.a()[l as usize] + air.b()[l as usize]) as usize - j;
                line.nodes.push(j as u32);
                line.links.push(l);
            }
            line
        }).collect::<Vec<_>>()
    });
    (0..2).all(|f| lines[f].iter().map(|ln| ln.links.len()).sum::<usize>() == count[f]).then_some(lines)
}

/// Work space of the solve of one line: per link the three pushes [mixing, overturning, pressure] along the line,
/// mol/s; per node its gas in the [lower, upper] half, mol; the shares each link carries on and back; the amounts
/// solved and the solver's work space; the amount of each gas solved, [gas * len + node].
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

impl Ctx<'_> {
    /// The lines of both directions in `order`, each from the state the one before left; the lines of one direction
    /// share no node, so each is solved from the state before the direction (in parallel, with `threads`) into a row
    /// per line, and the rows are booked in the order of the lines.
    fn sweep(&self, lines: &[Vec<Line>; 2], mut st: State, order: [usize; 2]) -> Moved {
        let (s, air, ng, na, nl) = (self.s, self.air, self.s.gas.len(), self.air.n(), self.air.a().len());
        let mut m = Moved { mol: vec![0.0; ng * nl], heat: vec![0.0; nl], paid: vec![0.0; na] };
        let mut push = vec![[0.0; 3]; nl];
        for dir in order {
            let col = self.columns(&st);
            for &l in lines[dir].iter().flat_map(|ln| &ln.links) {
                push[l as usize] = self.push(&st, &col, l as usize);
            }
            let len = lines[dir].iter().map(|ln| ln.nodes.len()).max().unwrap_or(0);
            // A direction with no link (one air node across the map) moves nothing.
            if len == 0 {
                continue;
            }
            let mut rows = vec![0.0; lines[dir].len() * (ng + 1) * len];
            crate::par::rows(&mut rows, (ng + 1) * len, Lane::default, |ln, i, row| self.solve(&st, &push, &lines[dir][i], ln, len, row));
            for (line, row) in lines[dir].iter().zip(rows.chunks((ng + 1) * len)) {
                for (p, &l) in line.links.iter().enumerate() {
                    let (l, sign) = (l as usize, line.sign(air, p));
                    let (a, b) = (air.a()[l] as usize, air.b()[l] as usize);
                    let mut moved = 0.0;
                    for t in 0..ng {
                        let (f, sp) = (sign * row[t * len + p], s.sp[s.gas[t]]);
                        (st.x[t * na + a], st.x[t * na + b]) = (st.x[t * na + a] - f, st.x[t * na + b] + f);
                        (st.nt[a], st.nt[b]) = (st.nt[a] - f, st.nt[b] + f);
                        (st.capa[a], st.capa[b]) = (st.capa[a] - f * sp.cp, st.capa[b] + f * sp.cp);
                        m.mol[t * nl + l] += f;
                        moved += f * sp.mm;
                    }
                    let h = sign * row[ng * len + p];
                    (st.e[a], st.e[b]) = (st.e[a] - h, st.e[b] + h);
                    m.heat[l] += h;
                    let (j, lift) = self.lift(a, b, moved);
                    st.e[j] -= lift;
                    m.paid[j] += lift;
                }
            }
        }
        m
    }

    /// Solve `line` from `st` into `row`: per link p of the line and gas t the amount it carries along the line,
    /// `row[t * len + p]`, mol, and the heat, `row[gases * len + p]`, J.
    fn solve(&self, st: &State, push: &[[f64; 3]], line: &Line, ln: &mut Lane, len: usize, row: &mut [f64]) {
        let (s, air, n, ng, na, dt) = (self.s, self.air, line.nodes.len(), self.s.gas.len(), self.air.n(), self.dt);
        let node = |p: usize| line.nodes[p] as usize;
        ln.push.clear();
        for (p, &l) in line.links.iter().enumerate() {
            let ([mix, over, even], sign) = (push[l as usize], line.sign(air, p));
            ln.push.push([mix, sign * over, sign * even]);
        }
        ln.halves.resize(n, [0.0; 2]);
        for v in [&mut ln.to_next, &mut ln.to_prev, &mut ln.x, &mut ln.c] {
            v.resize(n, 0.0);
        }
        ln.gas.resize(ng * n, 0.0);
        // Each gas, split into lower and upper halves by the amounts before the step. Overturning moves as much up as
        // down: any net amount beyond the pressure push is handed back.
        for p in 0..n {
            ln.halves[p] = [0.0; 2];
            for t in 0..ng {
                let (a, u) = (st.x[t * na + node(p)], st.x[t * na + node(p)] * s.split[t * na + node(p)]);
                ln.halves[p] = [ln.halves[p][0] + (a - u), ln.halves[p][1] + u];
            }
        }
        for t in 0..ng {
            for p in 0..n - 1 {
                let (a, b) = (node(p), node(p + 1));
                let [mix, over, even] = ln.push[p];
                let (fp, fm, ep, em) = (over.max(0.0), (-over).max(0.0), even.max(0.0), (-even).max(0.0));
                let ([la, ua], [lb, ub]) = (ln.halves[p], ln.halves[p + 1]);
                let (sa, sb) = (s.split[t * na + a], s.split[t * na + b]);
                ln.to_next[p] = dt * ((mix + ep) / st.nt[a] + fp * (1.0 - sa) / la + fm * sa / ua);
                ln.to_prev[p] = dt * ((mix + em) / st.nt[b] + fp * sb / ub + fm * (1.0 - sb) / lb);
            }
            ln.carry(n, |p| st.x[t * na + node(p)], &mut row[t * len..]);
            ln.gas[t * n..(t + 1) * n].copy_from_slice(&ln.x[..n]);
        }
        for p in 0..n {
            ln.x[p] = (0..ng).map(|t| ln.gas[t * n + p]).sum();
        }
        for p in 0..n - 1 {
            let (a, b, even) = (node(p), node(p + 1), ln.push[p][2]);
            let want = dt * (even.max(0.0) * ln.x[p] / st.nt[a] - (-even).max(0.0) * ln.x[p + 1] / st.nt[b]);
            let extra = (0..ng).map(|t| row[t * len + p]).sum::<f64>() - want;
            let src = if extra > 0.0 { p + 1 } else { p };
            for t in 0..ng {
                row[t * len + p] -= extra * ln.gas[t * n + src] / ln.x[src];
            }
        }
        // Heat moves with the gas, per mol its node's mean.
        for p in 0..n - 1 {
            let (a, b, [mix, over, even]) = (node(p), node(p + 1), ln.push[p]);
            ln.to_next[p] = dt * (mix + over.abs() + even.max(0.0)) / st.nt[a];
            ln.to_prev[p] = dt * (mix + over.abs() + (-even).max(0.0)) / st.nt[b];
        }
        ln.carry(n, |p| st.e[node(p)], &mut row[ng * len..]);
    }
}

impl Lane {
    /// The amounts `x0(p)` of the `n` nodes of a line moved by `to_next` and `to_prev` (`solve::lines`), into `x`; what
    /// each link carries forward, into `out`.
    fn carry(&mut self, n: usize, x0: impl Fn(usize) -> f64, out: &mut [f64]) {
        for p in 0..n {
            self.x[p] = x0(p);
        }
        along_line(&self.to_next[..n - 1], &self.to_prev[..n - 1], &mut self.x[..n], &mut self.c[..n]);
        for (p, o) in out[..n - 1].iter_mut().enumerate() {
            *o = self.to_next[p] * self.x[p] - self.to_prev[p] * self.x[p + 1];
        }
    }
}
