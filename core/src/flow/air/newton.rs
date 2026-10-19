//! The Newton step of the air's rounds (`step`): what the ground nodes give their air nodes, summed with its slopes
//! (`gather`), and one implicit step of the air nodes' totals, gases and heat with the wind (`solve`, by `solve::band`).

use super::super::Scratch;
use super::step::{Active, Iterate, FIXED, PER_GAS};
use super::wind::{pushes, Coef, Links, Pushes};
use super::{lerp, over_cell, shares};
use crate::solve::band::{fluxes, reach, Band};
use crate::{World, AIR};
use crate::store::Class;

/// What the ground nodes give the air nodes over the step: heat, J; the net amount of each active gas that comes up
/// (`[gas * nodes + node]`), mol; what condenses, mol and mol × reflectance; and the slopes of what each air node gets
/// with the unknowns of the air nodes within `far` of it (`Gathered::slope`).
pub(super) struct Gathered {
    pub(super) heat: Vec<f64>,
    pub(super) up: Vec<f64>,
    pub(super) rained: Vec<[f64; 2]>,
    slopes: Vec<f64>,
    far: usize,
    kinds: usize,
}

impl Gathered {
    /// Add `v` to the slope of what air node `p` gets with the unknown of air node `q`, of `kind`: 0 its heat with the
    /// temperature, then per active gas `a` its heat with the gas (`1 + 3a`), the gas with the gas, and the gas with the
    /// temperature.
    #[inline]
    fn slope(&mut self, (p, q): (usize, usize), kind: usize, v: f64) {
        self.slopes[((2 * self.far + 1) * p + q + self.far - p) * self.kinds + kind] += v;
    }
}

/// How far apart the air nodes a ground node sees lie, in nodes (its own is among them).
pub(super) fn far(s: &Scratch) -> usize {
    s.near.iter().flat_map(|j| j.iter().flat_map(move |&a| j.iter().map(move |&b| a.abs_diff(b) as usize))).max().unwrap_or(0)
}

/// Per mol of active gas `ac` that rains on ground node `i` out of air node `q`: the vaporisation enthalpy it frees, less
/// its heat and its fall to the ground, J.
fn mol_down(w: &World, s: &Scratch, it: &Iterate, ac: Active, (i, q): (usize, usize)) -> f64 {
    let sp = s.sp[ac.k];
    sp.lv - sp.cp * it.t[q] + sp.mm * s.view().lift(i, q, w.cfg.gravity)
}

/// The rows summed over the air nodes, in the order of the cells, with their slopes: each ground node gives its own air
/// node its heat and what comes up with its heat less its lift, with slopes to the four air nodes it sees by their
/// weights; what rains comes out of those four in their shares (`shares`), which move with each node's gas, and leaves
/// with its node's heat.
pub(super) fn gather(w: &World, s: &Scratch, it: &Iterate, act: &[Active], (rows, width): (&[f64], usize), far: usize) -> Gathered {
    let (na, cells, g, nx) = (w.mesh.level(AIR).n(), w.mesh.cells(), w.cfg.gravity, act.len());
    let kinds = 1 + 3 * nx;
    let mut out = Gathered { heat: vec![0.0; na], up: vec![0.0; nx * na], rained: vec![[0.0; 2]; na], slopes: vec![0.0; na * (2 * far + 1) * kinds], far, kinds };
    let mut own = vec![0.0; kinds];
    for (i, row) in rows.chunks_exact(width).enumerate() {
        let (p, over, wt) = (cells.parent()[i] as usize, over_cell(w, s, i), &s.wt[i]);
        let seen = || wt.iter().zip(&s.near[i]).filter(|e| *e.0 > 0.0).map(|(&wk, &q)| (wk, q as usize));
        out.heat[p] += row[0];
        own[0] = row[1];
        for (x, ac) in act.iter().enumerate() {
            let (sp, o) = (s.sp[ac.k], FIXED + PER_GAS * x);
            let [m, m_col, m_tl, r, r_col, r_tl, h_col] = <[f64; PER_GAS]>::try_from(&row[o..o + PER_GAS]).unwrap();
            let mol_up = sp.cp * row[4] - sp.mm * s.view().lift(i, p, g);
            out.heat[p] += m * mol_up;
            out.up[x * na + p] += m;
            own[0] += m_tl * mol_up;
            own[1 + 3 * x..4 + 3 * x].copy_from_slice(&[(h_col + m_col * mol_up) * over, m_col * over, m_tl]);
            if r <= 0.0 {
                continue;
            }
            let (v, k) = (&it.x[ac.gi * na..(ac.gi + 1) * na], 1 + 3 * x);
            let (sum, r_col) = (lerp(s, v, i), r_col * over);
            for (c, (qc, share)) in shares(s, v, i).into_iter().enumerate().filter(|&(c, _)| wt[c] > 0.0) {
                let down = mol_down(w, s, it, *ac, (i, qc));
                out.heat[qc] += r * share * down;
                out.up[x * na + qc] -= r * share;
                out.rained[qc] = [out.rained[qc][0] + r * share, out.rained[qc][1] + r * share * sp.refl[0]];
                // Its share grows with its own gas and shrinks with the gas the cell sees.
                let mine = r * wt[c] / sum;
                [(0, -r * share * sp.cp), (k, down * mine), (k + 1, -mine)].into_iter().for_each(|(kind, v)| out.slope((qc, qc), kind, v));
                for (wk, q) in seen() {
                    let (by_gas, by_t) = (wk * share * (r_col - r / sum), wk * share * r_tl);
                    [(0, down * by_t), (k, down * by_gas), (k + 1, -by_gas), (k + 2, -by_t)].into_iter().for_each(|(kind, v)| out.slope((qc, q), kind, v));
                }
            }
        }
        for (wk, q) in seen() {
            own.iter().enumerate().for_each(|(kind, &v)| out.slope((p, q), kind, v * wk));
        }
    }
    out
}

/// What the wind of the last solve carries over each link over the step: each gas, mol (`[gas * links + link]`), and
/// heat, J; and the energy each node pays to lift the mass that enters it, J.
#[derive(Default)]
pub(super) struct Wind {
    pub(super) mol: Vec<f64>,
    pub(super) heat: Vec<f64>,
    pub(super) paid: Vec<f64>,
}

/// Amount, mol, of each air node in the gases of `v` (`[gas * nodes + node]`), each mol weighted by `f` of its substance.
fn total(s: &Scratch, v: &[f64], f: &dyn Fn(usize) -> f64) -> Vec<f64> {
    let (ng, na) = (s.state.gas.len(), s.ta.len());
    (0..na).map(|j| (0..ng).map(|q| v[q * na + j] * f(s.state.gas[q])).sum::<f64>()).collect()
}

/// The wind of a step from the air at its start (`pushes`; the halves from `x0`), a linearly implicit step: its pushes
/// carry the amounts and heat at the step's end. The links, the pushes, the evening of pressure (with its system factored) and
/// the gas of each half of each node.
pub(super) struct Drive {
    pub(super) links: Links,
    push: Pushes,
    pressure: Coef,
    even: Band,
    halves: Vec<[f64; 2]>,
}

impl Drive {
    pub(super) fn of(w: &World, s: &Scratch, x0: &[f64], dt: f64) -> Drive {
        let (air, na) = (w.mesh.level(AIR), w.mesh.level(AIR).n());
        let links = Links::of(air);
        let push = pushes(&w.cfg, s, air, &links);
        let pressure = push.pressure(s, &links, dt);
        let even = Band::step(na, &links.carry(&pressure), &vec![0.0; na]);
        let half = |j: usize, q: usize| (x0[q * na + j], s.split[q * na + j]);
        let halves = (0..na).map(|j| (0..s.state.gas.len()).map(|q| half(j, q)).fold([0.0; 2], |h, (v, up)| [h[0] + v * (1.0 - up), h[1] + v * up])).collect();
        Drive { links, push, pressure, even, halves }
    }
}

/// One implicit step of the air nodes from the step's start (`x0`) with what the ground gives (`g`, linearised about
/// the round's air: a Newton step) and the wind of `drive`: the totals with the evening of pressure, then the gases the
/// ground does not exchange, then the active gases and the heat together, then each trace gas alone. `wind` gets what
/// crosses the links.
pub(super) fn solve(w: &World, s: &Scratch, (act, drive): (&[Active], &Drive), (x0, g): (&[f64], &Gathered), it: &Iterate, dt: f64, wind: &mut Wind) -> Iterate {
    let (links, push, halves) = (&drive.links, &drive.push, &drive.halves);
    let (na, ng, nl) = (w.mesh.level(AIR).n(), s.state.gas.len(), links.a.len());
    // The total amounts, with what comes up; and what the pressure moves over each link.
    let mut n = total(s, x0, &|_| 1.0);
    (0..act.len()).for_each(|x| n.iter_mut().zip(&g.up[x * na..(x + 1) * na]).for_each(|(v, u)| *v += u));
    drive.even.solve(&mut n);
    let mut flow = vec![0.0; nl];
    fluxes(&links.carry(&drive.pressure), &n, (1, 0), &mut flow);
    let zero = vec![0.0; na];
    let coef: Vec<_> = (0..ng).map(|q| push.carry(links, &n, (Some(&s.split[q * na..(q + 1) * na]), halves), &flow, dt)).collect();
    let mut x = it.x.clone();
    (wind.mol, wind.heat) = (vec![0.0; ng * nl], vec![0.0; nl]);
    // Gases spread alike through their column are carried alike: one system for all of them.
    let mut done: Vec<(usize, Band)> = Vec::new();
    for q in (0..ng).filter(|&q| !act.iter().any(|a| a.gi == q)) {
        let alike = done.iter().position(|(p, _)| coef[*p] == coef[q]);
        if alike.is_none() {
            done.push((q, Band::step(na, &links.carry(&coef[q]), &zero)));
        }
        let xq = &mut x[q * na..(q + 1) * na];
        xq.copy_from_slice(&x0[q * na..(q + 1) * na]);
        done[alike.unwrap_or(done.len() - 1)].1.solve(xq);
        fluxes(&links.carry(&coef[q]), xq, (1, 0), &mut wind.mol[q * nl..(q + 1) * nl]);
    }
    // Each half carries its node's heat per mol at the step's end.
    let capa = total(s, &x, &|k| s.sp[k].cp);
    let heat = push.carry(links, &n, (None, halves), &flow, dt);
    let heat = (heat.0.iter().zip(&links.a).map(|(c, &a)| c * capa[a as usize]).collect(), heat.1.iter().zip(&links.b).map(|(c, &b)| c * capa[b as usize]).collect());
    let t = Newton { w, s, act, g, it, links }.coupled((x0, &coef, (&heat, &capa)), &mut x, wind);
    Newton { w, s, act, g, it, links }.traces((x0, &coef), &mut x, wind);
    // The mass that crosses each link is lifted between the mean heights of its nodes, by the heat of the node it enters.
    wind.paid = vec![0.0; na];
    for (l, (&a, &b)) in links.a.iter().zip(&links.b).enumerate() {
        let (moved, a, b) = ((0..ng).map(|q| wind.mol[q * nl + l] * s.sp[s.state.gas[q]].mm).sum::<f64>(), a as usize, b as usize);
        wind.paid[if moved > 0.0 { b } else { a }] += w.cfg.gravity * moved * (s.zair[b] - s.zair[a]);
    }
    Iterate { t, x }
}

/// What the Newton step of the active gases reads.
struct Newton<'a> {
    w: &'a World,
    s: &'a Scratch,
    act: &'a [Active],
    g: &'a Gathered,
    it: &'a Iterate,
    links: &'a Links,
}

impl Newton<'_> {
    /// The slopes of what the ground nodes give the air nodes (`Gathered::slope`): `add(p, q, kind, v)`.
    fn each_slope(&self, mut add: impl FnMut(usize, usize, usize, f64)) {
        let (g, span) = (self.g, 2 * self.g.far + 1);
        for (at, &v) in g.slopes.iter().enumerate().filter(|e| *e.1 != 0.0) {
            let (p, off, kind) = (at / g.kinds / span, at / g.kinds % span, at % g.kinds);
            add(p, p + off - g.far, kind, v);
        }
    }
    /// The active gases more than a trace and the heat, solved together, each gas's rows scaled by its vaporisation
    /// enthalpy so the system stays near dominant by columns. A node's energy at the end is its gases' heat capacity
    /// then times its temperature, linearised about the round in the active gases. Writes the gases into `x`, the links
    /// into `wind`; returns each node's temperature at the end, K.
    fn coupled(&self, (x0, coef, (heat, capa)): (&[f64], &[Coef], (&Coef, &[f64])), x: &mut [f64], wind: &mut Wind) -> Vec<f64> {
        let (s, it, g, na, nl) = (self.s, self.it, self.g, self.w.mesh.level(AIR).n(), self.links.a.len());
        let fields = self.act.iter().filter(|a| a.field.is_some()).count() + 1;
        let (gas, he) = (|j: usize, f: usize| j * fields + f, |j: usize| j * fields + fields - 1);
        let coupled: Vec<(usize, usize, Active)> = self.act.iter().enumerate().filter_map(|(a, ac)| ac.field.map(|f| (a, f, *ac))).collect();
        let mut now = vec![0.0; na * fields];
        for j in 0..na {
            now[he(j)] = it.t[j];
            coupled.iter().for_each(|&(_, f, ac)| now[gas(j, f)] = it.x[ac.gi * na + j]);
        }
        let mut band = Band::zero(na * fields, fields * self.g.far.max(reach(&self.links.carry(heat))) + fields - 1);
        let mut rhs = vec![0.0; na * fields];
        band.carry(&self.links.carry(heat), fields, fields - 1, 1.0);
        for &(a, f, ac) in &coupled {
            let lv = s.sp[ac.k].lv;
            band.carry(&self.links.carry(&coef[ac.gi]), fields, f, lv);
            for j in 0..na {
                band.add(gas(j, f), gas(j, f), lv);
                rhs[gas(j, f)] = lv * (x0[ac.gi * na + j] + g.up[a * na + j]);
            }
        }
        for j in 0..na {
            band.add(he(j), he(j), capa[j]);
            rhs[he(j)] = self.w.store.heat(Class::Air)[j] + g.heat[j] - wind.paid.get(j).copied().unwrap_or(0.0);
            for &(_, f, ac) in &coupled {
                let c = s.sp[ac.k].cp * it.t[j];
                band.add(he(j), gas(j, f), c);
                rhs[he(j)] += c * it.x[ac.gi * na + j];
            }
        }
        let mut slope = |(r, c): (usize, usize), v: f64| {
            band.add(r, c, -v);
            rhs[r] -= v * now[c];
        };
        self.each_slope(|p, q, kind, v| match ((kind > 0).then(|| self.act[(kind - 1) / 3]).map(|ac| (ac.field, s.sp[ac.k].lv)), (kind + 2) % 3) {
            (None, _) => slope((he(p), he(q)), v),
            (Some((Some(f), _)), 0) => slope((he(p), gas(q, f)), v),
            (Some((Some(f), lv)), 1) => slope((gas(p, f), gas(q, f)), lv * v),
            (Some((Some(f), lv)), _) => slope((gas(p, f), he(q)), lv * v),
            _ => {}
        });
        band.factor();
        band.solve(&mut rhs);
        for &(_, f, ac) in &coupled {
            (0..na).for_each(|j| x[ac.gi * na + j] = rhs[gas(j, f)]);
            fluxes(&self.links.carry(&coef[ac.gi]), &rhs, (fields, f), &mut wind.mol[ac.gi * nl..(ac.gi + 1) * nl]);
        }
        fluxes(&self.links.carry(heat), &rhs, (fields, fields - 1), &mut wind.heat);
        (0..na).map(|j| rhs[he(j)]).collect()
    }
    /// Each trace gas alone, at the round's temperature: its latent heat moves the air too little to solve with it.
    fn traces(&self, (x0, coef): (&[f64], &[Coef]), x: &mut [f64], wind: &mut Wind) {
        let (it, g, na, nl) = (self.it, self.g, self.w.mesh.level(AIR).n(), self.links.a.len());
        for (a, ac) in self.act.iter().enumerate().filter(|(_, ac)| ac.field.is_none()) {
            let c = self.links.carry(&coef[ac.gi]);
            let mut band = Band::zero(na, self.g.far.max(reach(&c)));
            band.carry(&c, 1, 0, 1.0);
            let xq = &mut x[ac.gi * na..(ac.gi + 1) * na];
            for j in 0..na {
                band.add(j, j, 1.0);
                xq[j] = x0[ac.gi * na + j] + g.up[a * na + j];
            }
            self.each_slope(|p, q, kind, v| {
                if kind == 2 + 3 * a {
                    band.add(p, q, -v);
                    xq[p] -= v * it.x[ac.gi * na + q];
                }
            });
            band.factor();
            band.solve(xq);
            fluxes(&c, xq, (1, 0), &mut wind.mol[ac.gi * nl..(ac.gi + 1) * nl]);
        }
    }
}
