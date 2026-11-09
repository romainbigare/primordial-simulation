//! The channel a cell's liquid runs in (laws.md 1.2): its width and its depth grow as powers of the discharge it carries
//! (hydraulic geometry downstream, Leopold and Maddock 1953), and its liquid runs at the discharge over the channel's
//! cross-section. Every law and every shell reads a channel here.

use crate::{chem::Phase, params::Physics, World};

/// A channel: its width and depth, m, and the mean speed of its liquid, m/s (none where it carries nothing).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Channel {
    pub width: f64,
    pub depth: f64,
    pub speed: f64,
}

impl Physics {
    /// The channel that carries `q` m³/s: width `channel_width` × q^`channel_width_exp`, depth `channel_depth` ×
    /// q^`channel_depth_exp`, speed q over width × depth.
    pub fn channel(&self, q: f64) -> Channel {
        if q <= 0.0 {
            return Channel::default();
        }
        let (width, depth) = (self.channel_width * q.powf(self.channel_width_exp), self.channel_depth * q.powf(self.channel_depth_exp));
        Channel { width, depth, speed: q / (width * depth) }
    }
}

impl World {
    /// The liquid of cell `i` whose ground holds `g` mol of each substance: the substances liquid at the cell's
    /// temperature of the last survey (`Life::env`), their density, kg/m³, and mol per m³; none where none is liquid.
    pub fn liquid_in(&self, i: usize, g: impl Fn(usize) -> f64) -> Option<(f64, f64)> {
        let t = self.life.env.get(i)?[0];
        let (mut m, mut v, mut mol) = (0.0, 0.0, 0.0);
        for k in (0..self.chem.ns()).filter(|&k| self.chem.phase_in(k, t, &g) == Phase::Liquid) {
            let a = g(k);
            (m, v, mol) = (m + a * self.chem.subs[k].molar_mass, v + a * self.chem.molar_volume(k, Phase::Liquid), mol + a);
        }
        (v > 0.0).then(|| (m / v, mol / v))
    }

    /// The channel in cell `i` that carries `flux` mol/s (its own: `World::liquid_flux`), as a volume at the mol per m³
    /// of the liquid its ground holds (`liquid_in`); none where it holds none.
    pub fn channel(&self, i: usize, flux: f64) -> Channel {
        self.liquid_in(i, self.stock(crate::store::Node::ground(i))).map_or_else(Channel::default, |(_, per_m3)| self.cfg.phys.channel(flux / per_m3))
    }
}
