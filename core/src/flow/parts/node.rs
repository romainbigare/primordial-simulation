//! What the node and the shore read of a cell's parts (`parts`): the node's state from them (`row`) and the heat
//! conductance between the land and the liquid's top (`conductance`).

use super::{stands, Found, Parts, DEEP, LAND, PARTS, TOP};
use crate::flow::land::{column, NONE, STATE_ROW};
use crate::flow::Scratch;
use crate::World;

/// The heat conductance between the land of cell `i` and its liquid's top, W/K: through each part's active layer
/// across the edge of the ground under the liquid (`Curve::shore`), each from its middle, half its area over that edge
/// away, in series (two half cells, Patankar 1980, 4.2), conductivity × thickness as heat conducts sideways between
/// cells (`land::conduct`). None without land.
pub(super) fn conductance(w: &World, s: &Scratch, i: usize, p: &Parts, f: &[Found; PARTS]) -> f64 {
    if !p.land() {
        return 0.0;
    }
    let (area, shore) = (w.mesh.cells().area()[i], s.fill(i).shore(stands(f, w.pores(i).porosity)));
    let half = |q: usize| {
        let share = p.share(q);
        let mut kt = s.state.bed[i].cond * w.cfg.skin_m;
        for &k in &s.state.loose {
            let (sp, m) = (s.sp[k], if f[q].th.t < s.sp[k].boil { f[q].th.molten(k) } else { 0.0 });
            kt += p.amt[k][q] * ((1.0 - f[q].th.molten(k)) * sp.v[0] + m * sp.v[1]) * sp.cond / (area * share);
        }
        2.0 * kt * shore * shore / (share * area)
    };
    let (a, b) = (half(LAND), half(TOP));
    if a > 0.0 && b > 0.0 { a * b / (a + b) } else { 0.0 }
}

/// A substance of a node of parts that is not its plateau's counts as molten in it (its bit, `Latent::molten`) where
/// the parts hold at least this share of it molten.
const MOLTEN_SHARE: f64 = 0.5;

/// The row of `land::node_state` of cell `i` from its parts `p`, found as `f`: its heat capacity theirs summed, its
/// fusion enthalpy theirs, and its temperature what its heat holds over that (so its heat is its heat capacity times its
/// temperature and its fusion enthalpy, as every law reads a node); each substance molten by the share of it the parts
/// hold molten, the one of them whose share lies between none and all with the most fusion enthalpy as its plateau's
/// (others at none or all); found again at every solve. Its surface: its layers summed, its soft cover, its
/// reflectance by area (the liquid's of its top over its deep); its pores those under the liquid and what the land's
/// hold, so what stands over them is what stands over the liquid's (what may run off, laws.md 1.2).
pub(super) fn row(w: &World, s: &Scratch, i: usize, p: &Parts, f: &[Found; PARTS]) -> [f64; STATE_ROW] {
    let (area, por) = (w.mesh.cells().area()[i], w.pores(i).porosity);
    let fused = |q: usize| f[q].th.held + f[q].th.on.map_or(0.0, |(k, phi)| phi * p.amt[k][q] * s.sp[k].lf);
    let (c, all) = (f[LAND].cap + f[TOP].cap + f[DEEP].cap, fused(LAND) + fused(TOP) + fused(DEEP));
    let tg = (p.heat[LAND] + p.heat[TOP] + p.heat[DEEP] - all) / c.max(1.0);
    let (mut on, mut bits): (Option<(usize, f64, f64)>, u64) = (None, 0);
    for &k in &s.state.vary {
        let a = p.amt[k][LAND] + p.amt[k][TOP] + p.amt[k][DEEP];
        let m = if a > 0.0 { (p.amt[k][LAND] * f[LAND].th.molten(k) + p.amt[k][TOP] * f[TOP].th.molten(k) + p.amt[k][DEEP] * f[DEEP].th.molten(k)) / a } else { f[TOP].th.molten(k) };
        let lat = a * s.sp[k].lf;
        if m > 0.0 && m < 1.0 && lat > 0.0 && on.is_none_or(|o| lat > o.2) {
            on = Some((k, m, lat));
        }
        if m >= MOLTEN_SHARE {
            bits |= 1 << k;
        }
    }
    let (pk, phi, held) = match on {
        Some((k, m, lat)) => {
            bits |= 1 << k;
            (k, m, all - m * lat)
        }
        None => (NONE, 0.0, all),
    };
    let share = [1.0 - p.wet, p.wet];
    let l: [f64; 6] = std::array::from_fn(|x| f[LAND].l[x] + f[TOP].l[x] + f[DEEP].l[x]);
    // The solid surface under the liquid: its bottom's, with what lies solid in its top over it.
    let z_wet = if p.deep() { f[DEEP].z + (f[TOP].z - w.base_z) } else { f[TOP].z };
    let z = share[LAND] * f[LAND].z + share[1] * z_wet;
    let pd = room(f, por) / area;
    // The soft cover's resistance by area, its melting point by volume.
    let rc = share[LAND] * f[LAND].cover[0] + share[1] * f[TOP].cover[0];
    let tc = if l[0] > 0.0 { (0..PARTS).filter(|&q| f[q].l[0] > 0.0).map(|q| f[q].l[0] * f[q].cover[1]).sum::<f64>() / l[0] } else { f64::INFINITY };
    let wet_l: [f64; 6] = std::array::from_fn(|x| f[TOP].l[x] + f[DEEP].l[x]);
    let alb = share[LAND] * column(w, s, (i, share[LAND]), &f[LAND].l) + share[1] * column(w, s, (i, share[1]), &wet_l);
    [tg, pk as f64, phi, held, f64::INFINITY, f64::NEG_INFINITY, 0.0, 0.0, f64::from_bits(bits), 1.0, z, z + l[2] / area, rc, tc, l[0] / area, alb, pd]
}

/// The room the liquid of the parts found as `f` (porosity `por`) fills before it stands, m³: the pores under the
/// liquid and what the land's hold (the land keeps its own soil's liquid; what lies over its pores runs down,
/// `moves::settle`).
fn room(f: &[Found; PARTS], por: f64) -> f64 {
    f[TOP].pores(por) + f[DEEP].pores(por) + f[LAND].l[2].min(f[LAND].pores(por))
}
