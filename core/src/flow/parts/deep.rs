//! The liquid's top and its deep (laws.md 1.1, `parts`): the top holds what stands within `h_mix` under the liquid's
//! level, the deep what stands deeper and the ground under the liquid (`layer`); once a month they swap liquid through the
//! thermocline, all the way where the top is the denser (`mix`).

use super::moves::ROUNDING;
use super::{Found, Parts, Piece, DEEP, PARTS, TOP};
use crate::flow::{curve::Curve, Scratch};
use crate::{math::one_minus_exp, World};

/// The liquid of the deep of a cell of ground `curve` where `v` m³ of liquid stand over the pores of its ground under
/// the liquid: what stands deeper than `h_mix` under its level (`Curve::volume`), m³; none where none does.
pub(super) fn under(w: &World, curve: &Curve, v: f64) -> Option<f64> {
    Some(curve.volume(curve.level(v) - w.cfg.phys.h_mix)).filter(|&d| d > 0.0)
}

/// The deep of `p` in cell `i` (its parts found as `f`, porosity `por`) comes to hold `deep` m³ of standing liquid
/// (`under`) and what its pores hold, the top the rest, the liquid moving with its heat at the temperature of the part
/// it leaves: a deep that holds none gives the top all it holds; a top over a deep it has not had gives it the ground
/// under the liquid (the bed's skin and its fixed solids) and the liquid its pores hold besides. Whether any moved.
pub(super) fn layer(s: &Scratch, i: usize, p: &mut Parts, f: &[Found; PARTS], (deep, por): (Option<f64>, f64)) -> bool {
    match (deep, p.deep()) {
        (None, false) => false,
        (None, true) => {
            p.amt.iter_mut().for_each(|a| (a[TOP], a[DEEP]) = (a[TOP] + a[DEEP], 0.0));
            (p.heat[TOP], p.heat[DEEP]) = (p.heat[TOP] + p.heat[DEEP], 0.0);
            true
        }
        (Some(v), false) => {
            let (top, t) = (&f[TOP], f[TOP].th.t);
            let mut h = s.skin[i] * p.wet * t;
            for &k in &s.state.fixed {
                let m = p.amt[k][TOP];
                (p.amt[k][TOP], p.amt[k][DEEP]) = (0.0, m);
                h += m * s.sp[k].cp * t;
            }
            p.heat[TOP] -= h;
            p.heat[DEEP] += h;
            give(s, p, (top, TOP, DEEP), ((top.pores(por).min(top.l[2]) + v) / top.l[2]).min(1.0));
            true
        }
        (Some(v), true) => {
            let d = v - f[DEEP].over(por);
            if d.abs() <= ROUNDING * v {
                return false;
            }
            if d > 0.0 {
                give(s, p, (&f[TOP], TOP, DEEP), (d / f[TOP].l[2]).min(1.0));
            } else {
                give(s, p, (&f[DEEP], DEEP, TOP), (-d / f[DEEP].l[2]).min(1.0));
            }
            true
        }
    }
}

/// Part `from` of `p` (found as `f`) gives part `to` the share `x` of each liquid it holds, each mol with its heat at
/// its temperature and the fusion enthalpy it holds.
fn give(s: &Scratch, p: &mut Parts, (f, from, to): (&Found, usize, usize), x: f64) {
    let t = f.th.t;
    let mut h = 0.0;
    for &k in s.state.vary.iter().filter(|&&k| t < s.sp[k].boil) {
        let m = p.amt[k][from] * f.th.molten(k) * x;
        (p.amt[k][from], p.amt[k][to]) = (p.amt[k][from] - m, p.amt[k][to] + m);
        h += m * (s.sp[k].cp * t + s.sp[k].lf);
    }
    (p.heat[from], p.heat[to]) = (p.heat[from] - h, p.heat[to] + h);
}

/// What the top of a cell gains in a swap with its deep (`mix`): the cell; of each liquid, mol (what the deep gives
/// less what the top gives); and the heat, J.
type Swap = (usize, Vec<(usize, f64)>, f64);

/// The top and the deep of each cell that has both swap equal volumes of their liquid over `dt` seconds, each mol
/// with its heat: through the thermocline by eddy diffusion (`thermocline`, about 1 cm²/s: Hansen et al. 1984), over
/// the area of the ground under the deep's level and the distance between the middles of the two, as two volumes that
/// even out exactly as they would alone, (1 - e^(-K a dt / dz (1/v_top + 1/v_deep))) of the way; and all the way where
/// the top is the denser, so the column overturns: its main liquid (the most by volume) nearer the temperature at which
/// that liquid is densest (`Substance::dense`) than the deep's. The cells whose parts moved.
pub(in crate::flow) fn mix(w: &World, s: &mut Scratch, dt: f64) -> Vec<u32> {
    let (area, kz) = (w.mesh.cells().area(), w.cfg.phys.thermocline);
    let swaps: Vec<Swap> = (0..s.state.parts.len())
        .filter_map(|i| {
            let (m, p) = (s.mixed(i)?, s.state.parts[i].as_ref()?);
            if !p.deep() {
                return None;
            }
            let (top, deep) = (&m.piece[TOP], &m.piece[DEEP]);
            let (vt, vd) = (volume(s, top), volume(s, deep));
            // The deep's standing liquid, over its pores, and the area of the ground under its level.
            let stand = vd - deep.pd * area[i] * p.wet;
            let under = s.curves[i].wet(stand) * area[i];
            if vt <= 0.0 || stand <= 0.0 || under <= 0.0 {
                return None;
            }
            let (dz, x) = (0.5 * (vt / (area[i] * p.wet) + stand / under), 1.0 / vt + 1.0 / vd);
            let v = if denser(w, s, top, deep) { 1.0 / x } else { one_minus_exp(kz * under / dz * dt * x) / x };
            // What each mol of each liquid moves to the top, mol (the deep's less the top's), and the heat, J.
            let (mut moved, mut heat) = (Vec::new(), 0.0);
            for &k in s.state.vary.iter() {
                let sp = &s.sp[k];
                let (up, down) = (deep.liquid(k, sp) * v / vd, top.liquid(k, sp) * v / vt);
                if up != 0.0 || down != 0.0 {
                    moved.push((k, up - down));
                    heat += up * (sp.cp * deep.tg + sp.lf) - down * (sp.cp * top.tg + sp.lf);
                }
            }
            Some((i, moved, heat))
        })
        .collect();
    for (i, moved, heat) in &swaps {
        let Some(Some(p)) = s.state.parts.get_mut(*i) else { continue };
        for &(k, m) in moved {
            (p.amt[k][TOP], p.amt[k][DEEP]) = (p.amt[k][TOP] + m, p.amt[k][DEEP] - m);
        }
        (p.heat[TOP], p.heat[DEEP]) = (p.heat[TOP] + heat, p.heat[DEEP] - heat);
    }
    swaps.into_iter().map(|t| t.0 as u32).collect()
}

/// The volume of the liquid part `piece` holds, m³ (`Piece::liquid`).
fn volume(s: &Scratch, piece: &Piece) -> f64 {
    s.state.vary.iter().map(|&k| piece.liquid(k, &s.sp[k]) * s.sp[k].v[1]).sum()
}

/// Is the liquid of the top `top` denser than that of the deep `deep` under it? Its main liquid lies nearer the
/// temperature at which that liquid is densest (`Substance::dense`): a liquid grows lighter both ways from there.
fn denser(w: &World, s: &Scratch, top: &Piece, deep: &Piece) -> bool {
    let main = s.state.vary.iter().copied().max_by(|&a, &b| (top.liquid(a, &s.sp[a]) * s.sp[a].v[1]).total_cmp(&(top.liquid(b, &s.sp[b]) * s.sp[b].v[1])));
    main.is_some_and(|k| (top.tg - w.chem.subs[k].dense).abs() < (deep.tg - w.chem.subs[k].dense).abs())
}
