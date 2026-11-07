//! The relief a cell holds inside it (laws.md 0): the potential energy its ground holds over its flat form, as
//! the terrain (`space::terrain`) stands inside it. Every cell holds it from the start (`World::relief_at_build`); a
//! split of a cell no join made spends it on its quarters' terrain (`World::uneven`), and a join keeps what it flattens
//! (`World::join`).

use super::sum4;
use crate::chem::Phase;
use crate::space::{Leaf, Level};
use crate::World;

/// What a split of a cell no join made gives its quarters by the terrain (`World::uneven`), row by row: each one's
/// share of the bed, and its relief, J; and the scale of the terrain's departures it stands (1 the whole terrain).
pub(super) struct Uneven {
    pub(super) bed: [f64; 4],
    pub(super) relief: [f64; 4],
    pub(super) scale: f64,
}

impl World {
    /// The mass and the volume of the bed of cell `i`, kg and m³ (its substances as solids, as the stack reads them).
    fn bed_of(&self, i: usize) -> (f64, f64) {
        (0..self.chem.ns()).fold((0.0, 0.0), |(m, v), k| (m + self.b(k, i) * self.chem.subs[k].molar_mass, v + self.b(k, i) * self.chem.molar_volume(k, Phase::Solid)))
    }
    /// The weight of a m³ of the bed of cell `i`, N (its density times gravity; none for a cell with no bed).
    fn bed_weight(&self, i: usize) -> f64 {
        let (mass, vol) = self.bed_of(i);
        if vol > 0.0 { mass / vol * self.cfg.gravity } else { 0.0 }
    }
    /// The terrain inside leaf `l` (`Terrain::inside`): its mean height, m, and the spread of its heights, m⁴.
    fn terrain_in(&self, l: Leaf) -> (f64, f64) {
        self.terrain.inside(self.seed, &self.map, l, (self.cfg.lat_south, self.cfg.lat_north))
    }
    /// The relief of each cell of a world as built, J: a bed standing at the terrain's heights inside it holds
    /// ρ g ∫ (h − h̄)² / 2 more potential energy than a flat bed of its volume (`Terrain::inside`; each column of area a
    /// and height h, from the base up, holds ρ g a h² / 2 over the base's), ρ the density of the cell's bed.
    pub fn relief_at_build(&self) -> Vec<f64> {
        crate::par::map_n(self.mesh.cells().n(), crate::par::UNCOUNTED, |i| self.bed_weight(i) * self.terrain_in(self.map.leaves()[i]).1)
    }
    /// The quarters `kids` of cell `i` (leaf `leaf`) in the cells `cells` of the change, by the terrain: each one's bed
    /// surface the cell's mean one plus its departure d, the terrain's mean height over it (on the drained samples,
    /// `Terrain::samples`) less the mean over the four, so the cell's bed keeps its volume and its mean; each quarter's
    /// relief that of the terrain inside it (`relief_at_build`). The stacks then gain ρ g Σ a d² / 2 (the loose ground,
    /// the liquid and the store stay per m², so they rise and fall with the bed by area, and the air over the cell keeps
    /// its mean height). Where the cell's relief pays less than that and the quarters' reliefs, the departures, and the
    /// terrain below them, are scaled down until it pays; and so that each quarter keeps a bed.
    pub(super) fn uneven(&self, (i, leaf): (usize, Leaf), (cells, kids): (&Level, [usize; 4])) -> Uneven {
        let area = kids.map(|j| cells.area()[j]);
        let (rho_g, vol, whole) = (self.bed_weight(i), self.bed_of(i).1, sum4(area));
        let inside = leaf.quarters().map(|q| self.terrain_in(q));
        let mean = sum4(std::array::from_fn(|q| area[q] * inside[q].0)) / whole;
        let d = inside.map(|x| x.0 - mean);
        let need = rho_g * sum4(std::array::from_fn(|q| area[q] * d[q] * d[q] / 2.0 + inside[q].1));
        let s = if need > self.relief[i].max(0.0) { (self.relief[i].max(0.0) / need).sqrt() } else { 1.0 };
        let thick = vol / whole;
        let s = d.iter().filter(|&&x| x < 0.0).fold(s, |s, &x| s.min(thick / -x));
        let bed: [f64; 4] = std::array::from_fn(|q| area[q] * (thick + s * d[q]));
        let total = sum4(bed);
        Uneven { bed: bed.map(|b| b / total), relief: inside.map(|x| s * s * rho_g * x.1), scale: s }
    }
}
