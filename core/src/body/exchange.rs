//! A body's coat: its heat conductance to its surroundings (laws.md 6.3).

use super::Body;
use crate::Physics;

impl Body {
    /// Heat conductance of its coat, W/K: its `HEAT` gene through each member's surface (as mass^(2/3)), held between
    /// the best a coat can insulate (`Physics::r_fur` over the still air at its surface, `Physics::r_air`) and its bare
    /// surface (`bare`).
    pub fn conductance(&self, p: &Physics, mass: f64) -> f64 {
        let (n, bare) = (self.n(), self.bare(p, mass));
        let gene = self.k.heat * n * (p.m_ref * (mass / n).max(0.0).powi(2)).cbrt();
        gene.clamp(bare * p.r_air / (p.r_fur + p.r_air), bare)
    }
    /// Heat conductance of its bare surface, W/K: each member's surface, `Physics::meeh` × its mass^(2/3) (Meeh 1879),
    /// over the still air at it (`Physics::r_air`). A warm body opens its coat up to this (`live::keep_warm`).
    pub fn bare(&self, p: &Physics, mass: f64) -> f64 {
        let n = self.n();
        n * p.meeh * (mass / n).max(0.0).powf(2.0 / 3.0) / p.r_air
    }
}
