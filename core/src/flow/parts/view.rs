//! What other laws and an edit read and set of a cell's parts (`parts`).

use super::{has, Mixed, Parts, Piece, DEEP, LAND, TOP};
use crate::flow::Scratch;
use crate::{ground::amounts_cap, store::Class, units::MAX_SUBSTANCES, World};

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
            let cap = skin * p.ground(q) + amounts_cap(0..self.chem.ns(), amt, |k| self.chem.subs[k].cp);
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
    /// Of a cell of parts that has land: the mol of substance `k` its land part holds and that part's share of the cell's
    /// area, as Flow last set them (`Parts`); none for any other cell.
    pub(crate) fn land_part(&self, i: usize, k: usize) -> Option<(f64, f64)> {
        let p = self.state.parts.get(i)?.as_ref().filter(|p| p.land())?;
        Some((p.amt[k][LAND], p.share(LAND)))
    }
    /// The pore space of the ground life stands on in cell `i`, as a depth of liquid over it, m: of a cell of parts that
    /// has land its land's (`Piece::pd`), else the node's, as Flow's last solve found them.
    pub(crate) fn land_pores(&self, i: usize) -> f64 {
        let land = self.state.parts.get(i).and_then(Option::as_ref).is_some_and(Parts::land);
        match self.mixed(i) {
            Some(m) if land => m.piece[LAND].pd,
            _ => self.pd.get(i).copied().unwrap_or(0.0),
        }
    }
    /// The depth of the soft solid lying on the land of cell `i`, m, at its solid's density: of a cell of parts its
    /// land's, else the node's (`Scratch::cover`).
    pub(crate) fn soft_land(&self, i: usize) -> f64 {
        self.mixed(i).map_or_else(|| self.cover.get(i).map_or(0.0, |c| c[2]), |m| m.piece[LAND].cover[2])
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

/// What the ground the liquid of cell `i` comes to cover takes first, as its pores fill (pores first): the liquid its
/// pores lack, as a depth over that ground, m, and what the cell's own pores give them before what stands does, m³. Of
/// a cell of parts, what its land's pores lack (`Piece::pd` less what they hold), all paid by what stands
/// (`moves::strip`); of a node that is one, its pore space, paid first by the liquid all its pores hold
/// (`parts::create`).
pub(in crate::flow) fn pores_first(s: &Scratch, i: usize) -> [f64; 2] {
    match s.mixed(i).map(|m| &m.piece[LAND]) {
        Some(land) if land.share > 0.0 => {
            let held: f64 = (0..land.amt.len()).map(|k| land.liquid(k, &s.sp[k]) * s.sp[k].v[1]).sum();
            [(land.pd - held / (s.curves[i].area() * land.share)).max(0.0), 0.0]
        }
        Some(_) => [0.0; 2],
        None => [s.pd[i], (s.head[i] - s.z[i]).clamp(0.0, s.pd[i]) * s.curves[i].area()],
    }
}

/// The share of cell `i`'s area under its liquid as its last solve left it (`Parts::wet`); none for a node that is one.
pub(in crate::flow) fn under_liquid(s: &Scratch, i: usize) -> Option<f64> {
    s.state.parts.get(i).and_then(Option::as_ref).map(|p| p.wet)
}
