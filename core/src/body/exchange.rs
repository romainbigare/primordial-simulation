//! A body's coat: its heat conductance to its surroundings (laws.md 6.3).

use super::Body;

impl Body {
    /// Heat conductance of its coat, W/K (the `HEAT` gene), through each member's surface, as mass^(2/3).
    pub fn conductance(&self, m_ref: f64, mass: f64) -> f64 {
        let n = self.n();
        self.k.heat * n * (m_ref * (mass / n).max(0.0).powi(2)).cbrt()
    }
}
