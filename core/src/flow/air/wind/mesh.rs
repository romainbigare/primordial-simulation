//! The wind on any mesh: the links of the air level as the networks of `solve::spread`.

use super::{Ctx, Moved, State};
use crate::solve::spread::{within, Net, Spread};

/// The wind of one call on the links of any air level, from `st`, by three implicit steps of `solve::spread` (in the
/// work space `solver`), each
/// from the start of the call: the mixing and the overturning carry each gas by its share of the column (capacity:
/// the column's amount) and heat per mol; the evening of pressure moves the amount by the pressure (capacity: the
/// amount per Pa, n/p), and each gas and the heat go with it at the share of the node it leaves.
///
/// The overturning carries the lower half of a column one way and the upper half back. On lines that is an
/// exchange solved from each half's own share; here it is an exchange of whole columns, at the same rate each way, so
/// a gas crowded in the lower half (a vapour) is carried as if it filled its column evenly: the vapour the overturning
/// brings to the warmer column on a square map is not carried on another mesh. The evening of pressure closes
/// `EVEN`/(1 + `EVEN`) of a lone pair's difference, as the implicit step closes it, not `EVEN`.
pub(super) fn on_mesh(at: &Ctx, st: &State, solver: &mut Spread) -> Moved {
    let (s, air, ng, na, nl) = (at.s, at.air, at.s.state.gas.len(), at.air.n(), at.air.a().len());
    let (a, b) = (air.a(), air.b());
    let col = at.columns(st);
    let (mut g_mix, mut g_even) = (vec![0.0; nl], vec![0.0; nl]);
    for l in (0..nl).filter(|&l| air.width()[l] > 0.0) {
        let [mix, over, _] = at.push(st, &col, l);
        (g_mix[l], g_even[l]) = (mix + over.abs(), at.even(st, &col, a[l] as usize, b[l] as usize));
    }
    let mut m = Moved { mol: vec![0.0; ng * nl], heat: vec![0.0; nl], paid: vec![0.0; na] };
    // (A step stops when it has converged; at most one iteration per node.)
    let mixing = Net { a, b, g: &g_mix, cap: &st.nt };
    for t in 0..ng {
        let mut share: Vec<f64> = (0..na).map(|j| st.x[t * na + j] / st.nt[j]).collect();
        solver.step(&mixing, &mut share, at.dt, na, &mut m.mol[t * nl..(t + 1) * nl]);
    }
    let mut per_mol: Vec<f64> = (0..na).map(|j| st.e[j] / st.nt[j]).collect();
    solver.step(&mixing, &mut per_mol, at.dt, na, &mut m.heat);
    let (mut p, mut total) = (col.p.clone(), vec![0.0; nl]);
    let per_pa: Vec<f64> = (0..na).map(|j| st.nt[j] / col.p[j]).collect();
    solver.step(&Net { a, b, g: &g_even, cap: &per_pa }, &mut p, at.dt, na, &mut total);
    for (l, &f) in total.iter().enumerate() {
        let up = if f > 0.0 { a[l] } else { b[l] } as usize;
        for t in 0..ng {
            m.mol[t * nl + l] += f * st.x[t * na + up] / st.nt[up];
        }
        m.heat[l] += f * st.e[up] / st.nt[up];
    }
    // The mixing and the evening of pressure, each from the start of the call, give no more of a gas than a node holds.
    for t in 0..ng {
        within(a, b, &mut m.mol[t * nl..(t + 1) * nl], &st.x[t * na..(t + 1) * na]);
    }
    for l in 0..nl {
        let moved = (0..ng).map(|t| m.mol[t * nl + l] * s.sp[s.state.gas[t]].mm).sum();
        let (j, lift) = at.lift(a[l] as usize, b[l] as usize, moved);
        m.paid[j] += lift;
    }
    m
}
