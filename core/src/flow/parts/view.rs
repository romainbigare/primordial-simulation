//! What other laws and an edit read and set of a cell's parts (`parts`).

use super::{has, Mixed, Parts, Piece, DEEP, LAND, TOP};
use crate::flow::Scratch;
use crate::{store::Class, units::MAX_SUBSTANCES, World};

impl World {
    /// The ground of cell `i`, which holds its heat now, held at one temperature `t` K: of a cell of parts (`Parts`) the
    /// heat of its liquid's top and deep that of their matter at `t` (its heat capacity and the fusion enthalpy of what
    /// is molten there) and the land the rest. For an edit that brings a ground to a temperature (`World::energy_ground`).
    pub fn even_ground(&mut self, i: usize, t: f64) {
        let e = self.store.heat(Class::Ground)[i];
        let skin = self.heat_cap_of(i, |_| 0.0, |k| self.b(k, i));
        let Some(Some(p)) = self.scratch.state.parts.get(i) else { return };
        let held = |q: usize| {
            let amt = |k: usize| p.amt[k][q];
            let cap = skin * p.ground(q) + (0..self.chem.ns()).map(|k| amt(k) * self.chem.subs[k].cp).sum::<f64>();
            cap * t + (0..self.chem.ns()).map(|k| amt(k) * self.chem.fused_heat(k, t, amt)).sum::<f64>()
        };
        let (top, deep) = (held(TOP), held(DEEP));
        if let Some(Some(p)) = self.scratch.state.parts.get_mut(i) {
            p.heat = [e - top - deep, top, deep];
        }
    }
}

impl Scratch {
    /// The ground of cell `i` as life feels it: of a cell of parts its land's temperature and plateau (its liquid's
    /// top's where it has no land), else the node's (`Scratch::plateau`).
    pub(crate) fn land(&self, i: usize) -> Option<(f64, Option<(usize, f64)>)> {
        match self.state.parts.get(i) {
            Some(Some(p)) => {
                let q = if p.land() { LAND } else { TOP };
                Some((p.t[q], p.on[q]))
            }
            _ => self.plateau(i),
        }
    }
    /// The depth of the liquid standing on the ground of cell `i` as life stands on it, m, and its solid surface, m
    /// (`FlowView::standing`): none on the land of a cell of parts, whose liquid runs down to the part under the liquid.
    pub(crate) fn standing_land(&self, i: usize) -> (f64, f64) {
        let (d, z) = self.view().standing(i);
        (if self.state.parts.get(i).and_then(Option::as_ref).is_some_and(Parts::land) { 0.0 } else { d }, z)
    }
    /// The temperature of the surface of cell `i`, K: of a cell of parts its land's and its liquid's top's mean by
    /// area, else the node's.
    pub(crate) fn surface_t(&self, i: usize) -> Option<f64> {
        match self.state.parts.get(i) {
            Some(Some(p)) => Some((1.0 - p.wet) * p.t[LAND] + p.wet * p.t[TOP]),
            _ => self.plateau(i).map(|x| x.0),
        }
    }
    /// The room the pores of the land of cell `i` have left, m³ (`Parts::dry`; none for a cell that is one): what the
    /// cell's liquid does not fill before it stands.
    pub(in crate::flow) fn dry(&self, i: usize) -> f64 {
        match self.state.parts.get(i) {
            Some(Some(p)) => p.dry,
            _ => 0.0,
        }
    }
    /// The parts of cell `i` as the last solve found them, if it has parts.
    pub(in crate::flow) fn mixed(&self, i: usize) -> Option<&Mixed> {
        self.mixed.get(i).and_then(Option::as_deref)
    }
    /// The liquid's top of cell `i` where it alone faces the air: a cell of parts with no land.
    pub(in crate::flow) fn top_alone(&self, i: usize) -> Option<&Piece> {
        self.mixed(i).filter(|m| !m.two_faces()).map(|m| &m.piece[TOP])
    }
    /// Does cell `i` face the air with two parts, its land and its liquid's top (`Mixed::two_faces`)?
    pub(in crate::flow) fn two_faces(&self, i: usize) -> bool {
        has(self, i) && self.mixed(i).is_some_and(Mixed::two_faces)
    }
}

/// The amounts of a part a reader asks for, mol, `[k]`: those of `piece`, else those of ground node `i`.
pub(in crate::flow) fn amounts(w: &World, i: usize, piece: Option<&Piece>) -> [f64; MAX_SUBSTANCES] {
    let mut amt = [0.0; MAX_SUBSTANCES];
    match piece {
        Some(p) => amt.iter_mut().zip(&p.amt).for_each(|(a, &x)| *a = x),
        None => amt.iter_mut().zip(w.store.amounts(Class::Ground)[i..].iter().step_by(w.mesh.cells().n())).for_each(|(a, &x)| *a = x),
    }
    amt
}

/// The liquid the pores of the land of cell `i` lack, as a depth over the land, m: its pore space (`Piece::pd`) less
/// the liquid they hold, which ground its liquid comes to cover takes from what stands (`moves::strip`, pores first);
/// none for a cell that is one node.
pub(in crate::flow) fn dry_pores(s: &Scratch, i: usize) -> f64 {
    let Some(land) = s.mixed(i).map(|m| &m.piece[LAND]).filter(|l| l.share > 0.0) else { return 0.0 };
    let held: f64 = (0..land.amt.len()).map(|k| land.liquid(k, &s.sp[k]) * s.sp[k].v[1]).sum();
    (land.pd - held / (s.curves[i].area() * land.share)).max(0.0)
}
