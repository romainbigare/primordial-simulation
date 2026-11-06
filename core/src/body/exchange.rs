//! A body's coat: its heat conductance to its surroundings (laws.md 6.3).

use super::{Body, HEAT};
use crate::{life::Chunk, Physics, World};

/// What a body's heat passes to (laws.md 6.3, 3.4): the temperature of its surroundings, K, and the heat conductance of
/// its coat and of its bare surface to them, W/K.
pub(super) struct Around {
    pub t: f64,
    pub coat: f64,
    pub bare: f64,
}

/// Outside, its surroundings are its cell's ground as Flow's last step left it, and the standing liquid on it: the share
/// of its height under the liquid (`bond::immersed`) passes its heat to the liquid, the rest to the air. Inside walls they
/// are the outside, raised by the heat the other bodies inside give the room's air (`bond::room_air`), through its coat
/// in series with the walls and openings between that air and the outside: its own heat warms the air too.
pub(super) fn around(w: &World, d: &Chunk, b: &Body, mass: f64) -> Around {
    let (p, tg) = (&w.cfg.phys, w.t_ground_last(b.cell as usize));
    match crate::bond::room_air(d, b) {
        None => {
            let wet = crate::bond::immersed(w, d, b, b.cell as usize);
            Around { t: tg, coat: b.conductance(p, mass, wet), bare: b.bare(p, mass, wet) }
        }
        Some(air) => {
            let through = |c: f64| 1.0 / (1.0 / c + 1.0 / air.g);
            Around { t: tg + air.rise, coat: through(b.conductance(p, mass, 0.0)), bare: through(b.bare(p, mass, 0.0)) }
        }
    }
}

impl Body {
    /// Surface of its members, m²: each `Physics::meeh` × its mass^(2/3) (Meeh 1879).
    fn surface(&self, p: &Physics, mass: f64) -> f64 {
        let n = self.n();
        n * p.meeh * (mass / n).max(0.0).powf(2.0 / 3.0)
    }
    /// Heat conductance of its coat, W/K, the share `wet` of its surface in liquid: its surface over the still air at
    /// it (`Physics::r_air`) and the share `HEAT` of the best coat (`Physics::r_coat`); in liquid over the liquid's film
    /// (`Physics::r_liquid`), which passes heat by convection far better than still air (50 to 1,000 W/(m² K) against
    /// about 8; Incropera and DeWitt 2002), and the share `wet_coat` of that coat, whose still air the liquid displaces
    /// (Scholander et al. 1950).
    pub(crate) fn conductance(&self, p: &Physics, mass: f64, wet: f64) -> f64 {
        let (s, coat) = (self.surface(p, mass), self.gene_in_range(HEAT) * p.r_coat);
        s / (p.r_air + coat) * (1.0 - wet) + s / (p.r_liquid + coat * p.wet_coat) * wet
    }
    /// Heat conductance of its bare surface, W/K, the share `wet` of it in liquid: its surface over the still air or the
    /// liquid's film at it. A warm body opens its coat up to this (`live::keep_warm`).
    pub(crate) fn bare(&self, p: &Physics, mass: f64, wet: f64) -> f64 {
        let s = self.surface(p, mass);
        s / p.r_air * (1.0 - wet) + s / p.r_liquid * wet
    }
}
