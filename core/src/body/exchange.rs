//! A body's coat: its heat conductance to its surroundings (laws.md 6.3).

use super::{Body, HEAT};
use crate::Physics;

impl Body {
    /// Surface of its members, m²: each `Physics::meeh` × its mass^(2/3) (Meeh 1879).
    fn surface(&self, p: &Physics, mass: f64) -> f64 {
        let n = self.n();
        n * p.meeh * (mass / n).max(0.0).powf(2.0 / 3.0)
    }
    /// Heat conductance of its coat, W/K: its surface over the still air at it (`Physics::r_air`) and the share `HEAT` of
    /// the best coat (`Physics::r_coat`).
    pub(crate) fn conductance(&self, p: &Physics, mass: f64) -> f64 {
        self.surface(p, mass) / (p.r_air + self.gene_in_range(HEAT) * p.r_coat)
    }
    /// Heat conductance of its bare surface, W/K: its surface over the still air at it. A warm body opens its coat up to
    /// this (`live::keep_warm`).
    pub(crate) fn bare(&self, p: &Physics, mass: f64) -> f64 {
        self.surface(p, mass) / p.r_air
    }
}
