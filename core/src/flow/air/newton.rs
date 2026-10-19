//! The Newton step of the air's rounds (`step`): what the ground nodes give their air nodes, summed with its slopes
//! (`gather`), and one implicit step of the air nodes' totals, gases and heat with the wind (`solve`, by `solve::band`).

use super::super::Scratch;
use super::step::{Active, Iterate, FIXED, PER_GAS};
use super::wind::{pushes, Air, Coef, Links};
use super::{air_temp, over_cell};
use crate::solve::band::{fluxes, reach, Band};
use crate::{World, AIR};
use crate::store::Class;

/// What the ground nodes under each air node give it over the step: heat, J; the net amount of each active gas that
/// comes up (`[gas * nodes + node]`), mol; what condenses, mol and mol × reflectance; and per ground node the slopes of
/// its heat and gas with the temperature and gases of the air it sees.
pub(super) struct Gathered {
    pub(super) heat: Vec<f64>,
    pub(super) up: Vec<f64>,
    pub(super) rained: Vec<[f64; 2]>,
    slopes: Vec<f64>,
}

/// The rows summed over each air node, in the order of the cells: the heat the ground gives, the heat of the gas that
/// comes up (and what rain frees) less its lift, and the gas. A ground node's slopes go to the four air nodes it sees by
/// their weights.
pub(super) fn gather(w: &World, s: &Scratch, it: &Iterate, act: &[Active], rows: &[f64], width: usize) -> Gathered {
    let (na, cells, g) = (w.mesh.level(AIR).n(), w.mesh.cells(), w.cfg.gravity);
    let per = 1 + 3 * act.len();
    let mut out = Gathered { heat: vec![0.0; na], up: vec![0.0; act.len() * na], rained: vec![[0.0; 2]; na], slopes: vec![0.0; cells.n() * per] };
    for (i, row) in rows.chunks_exact(width).enumerate() {
        let (p, over) = (cells.parent()[i] as usize, over_cell(w, s, i));
        out.heat[p] += row[0];
        let slopes = &mut out.slopes[i * per..(i + 1) * per];
        slopes[0] = row[1];
        for (x, ac) in act.iter().enumerate() {
            let (sp, o) = (s.sp[ac.k], FIXED + PER_GAS * x);
            let [m, m_col, m_tl, r, r_col, r_tl, h_col] = <[f64; PER_GAS]>::try_from(&row[o..o + PER_GAS]).unwrap();
            // Per mol that comes up: its heat, less its lift; per mol that rains, the vaporisation enthalpy it frees.
            let (mol_up, mol_down) = (sp.cp * row[4] - sp.mm * s.view().lift(i, p, g), sp.lv - sp.cp * it.t[p] + sp.mm * s.view().lift(i, p, g));
            out.heat[p] += m * mol_up + r * mol_down;
            out.up[x * na + p] += m - r;
            slopes[0] += m_tl * mol_up + r_tl * mol_down;
            slopes[1 + 3 * x..4 + 3 * x].copy_from_slice(&[(h_col + m_col * mol_up + r_col * mol_down) * over, (m_col - r_col) * over, m_tl - r_tl]);
            out.rained[p] = [out.rained[p][0] + r, out.rained[p][1] + r * sp.refl[0]];
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

/// One implicit step of the air nodes from the step's start (`x0`) with what the ground gives (`g`, linearised about
/// the round's air: a Newton step): the totals with the evening of pressure, then the gases the ground does not
/// exchange, then the active gases and the heat together, then each trace gas alone. `wind` gets what crosses the links.
pub(super) fn solve(w: &World, s: &Scratch, (act, links): (&[Active], &Links), (x0, g): (&[f64], &Gathered), it: &Iterate, dt: f64, wind: &mut Wind) -> Iterate {
    let (air, na, ng, nl) = (w.mesh.level(AIR), w.mesh.level(AIR).n(), s.state.gas.len(), links.a.len());
    let total = |v: &[f64], f: &dyn Fn(usize) -> f64| (0..na).map(|j| (0..ng).map(|q| v[q * na + j] * f(s.state.gas[q])).sum::<f64>()).collect::<Vec<f64>>();
    let (n1, capa1, mass1) = (total(&it.x, &|_| 1.0), total(&it.x, &|k| s.sp[k].cp), total(&it.x, &|k| s.sp[k].mm));
    let push = pushes(&w.cfg, s, air, links, &Air { n: &n1, capa: &capa1, mass: &mass1, t: &it.t });
    // The total amounts, with what comes up; and what the pressure moves over each link.
    let mut n = total(x0, &|_| 1.0);
    for (x, _) in act.iter().enumerate() {
        n.iter_mut().zip(&g.up[x * na..(x + 1) * na]).for_each(|(v, u)| *v += u);
    }
    let pressure = push.pressure(s, links, &it.t, dt);
    let zero = vec![0.0; na];
    Band::step(na, &links.carry(&pressure), &zero).solve(&mut n);
    let mut flow = vec![0.0; nl];
    fluxes(&links.carry(&pressure), &n, (1, 0), &mut flow);
    // Each half of each node, from the gases of the round.
    let halves: Vec<[f64; 2]> = (0..na)
        .map(|j| (0..ng).fold([0.0; 2], |h, q| {
            let (v, up) = (it.x[q * na + j], s.split[q * na + j]);
            [h[0] + v * (1.0 - up), h[1] + v * up]
        }))
        .collect();
    let coef: Vec<_> = (0..ng).map(|q| push.carry(links, &n, (Some(&s.split[q * na..(q + 1) * na]), &halves), &flow, dt)).collect();
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
    // Each half carries its node's heat per mol; the unknown is the energy over the gases' heat capacity now.
    let capa = total(&x, &|k| s.sp[k].cp);
    let heat = push.carry(links, &n, (None, &halves), &flow, dt);
    let heat = (heat.0.iter().zip(&links.a).map(|(c, &a)| c * capa[a as usize]).collect(), heat.1.iter().zip(&links.b).map(|(c, &b)| c * capa[b as usize]).collect());
    let t = Newton { w, s, act, g, it, links }.coupled((x0, &coef, (&heat, &capa)), &mut x, wind);
    Newton { w, s, act, g, it, links }.traces((x0, &coef), &mut x, wind);
    // The mass that crosses each link is lifted between the mean heights of its nodes, by the heat of the node it enters.
    let mut paid = vec![0.0; na];
    for (l, (&a, &b)) in links.a.iter().zip(&links.b).enumerate() {
        let moved: f64 = (0..ng).map(|q| wind.mol[q * nl + l] * s.sp[s.state.gas[q]].mm).sum();
        let (a, b) = (a as usize, b as usize);
        paid[if moved > 0.0 { b } else { a }] += w.cfg.gravity * moved * (s.zair[b] - s.zair[a]);
    }
    wind.paid = paid;
    let capa_end = total(&x, &|k| s.sp[k].cp);
    Iterate { t: (0..na).map(|j| air_temp(capa[j] * t[j], capa_end[j])).collect(), x, refl: Vec::new() }
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
    /// How far an air node's row reaches to the air nodes its ground nodes see, in nodes.
    fn far(&self) -> usize {
        let cells = self.w.mesh.cells();
        (0..cells.n()).flat_map(|i| self.s.near[i].iter().map(move |&q| (q as usize).abs_diff(cells.parent()[i] as usize))).max().unwrap_or(0)
    }
    /// Each ground node's slopes to the air nodes it sees: `add(i, p, q, slopes, wq)` for ground node `i` of air node `p`
    /// and each air node `q` it sees by the weight `wq`.
    fn each_slope(&self, mut add: impl FnMut(usize, usize, &[f64], f64)) {
        let (cells, per) = (self.w.mesh.cells(), 1 + 3 * self.act.len());
        for (i, sl) in self.g.slopes.chunks_exact(per).enumerate() {
            for (&q, &wq) in self.s.near[i].iter().zip(&self.s.wt[i]).filter(|(_, &wq)| wq > 0.0) {
                add(cells.parent()[i] as usize, q as usize, sl, wq);
            }
        }
    }
    /// The active gases more than a trace and the heat, solved together, each gas's rows scaled by its vaporisation
    /// enthalpy so the system stays near dominant by columns. Writes the gases into `x`, the links into `wind`; returns
    /// each node's heat unknown.
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
        let mut band = Band::zero(na * fields, fields * self.far().max(reach(&self.links.carry(heat))) + fields - 1);
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
        }
        let mut slope = |(r, c): (usize, usize), v: f64| {
            band.add(r, c, -v);
            rhs[r] -= v * now[c];
        };
        self.each_slope(|p, q, sl, wq| {
            slope((he(p), he(q)), sl[0] * wq);
            for &(a, f, ac) in &coupled {
                let lv = s.sp[ac.k].lv;
                slope((he(p), gas(q, f)), sl[1 + 3 * a] * wq);
                slope((gas(p, f), gas(q, f)), lv * sl[2 + 3 * a] * wq);
                slope((gas(p, f), he(q)), lv * sl[3 + 3 * a] * wq);
            }
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
            let mut band = Band::zero(na, self.far().max(reach(&c)));
            band.carry(&c, 1, 0, 1.0);
            let xq = &mut x[ac.gi * na..(ac.gi + 1) * na];
            for j in 0..na {
                band.add(j, j, 1.0);
                xq[j] = x0[ac.gi * na + j] + g.up[a * na + j];
            }
            self.each_slope(|p, q, sl, wq| {
                band.add(p, q, -sl[2 + 3 * a] * wq);
                xq[p] -= sl[2 + 3 * a] * wq * it.x[ac.gi * na + q];
            });
            band.factor();
            band.solve(xq);
            fluxes(&c, xq, (1, 0), &mut wind.mol[ac.gi * nl..(ac.gi + 1) * nl]);
        }
    }
}
