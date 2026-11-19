//! The soil (laws.md 4): what the loose ground is made of. Its texture, the shares of fine, middle and coarse
//! particles, is what the laws that make it leave in each of the terrain's samples (`texture`); a cell's is its
//! samples' mean, and its pores follow from it (`pores`).

mod pores;
mod texture;

pub use pores::Pores;
pub use texture::samples;

use crate::space::samples::Samples;
use crate::World;
use serde::{Deserialize, Serialize};

/// The texture of loose ground: the shares of its mineral particles that are coarse (50 µm to 2 mm) and fine
/// (under 2 µm), the rest middle (`Texture::middle`); and the share of its matter that is organic, which no law
/// makes yet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Texture {
    pub coarse: f64,
    pub fine: f64,
    pub organic: f64,
}

impl Texture {
    /// The share of middle particles.
    pub fn middle(&self) -> f64 {
        1.0 - self.coarse - self.fine
    }
}

/// The soil of a cell: its texture, and its pores (`Pores::of` the texture).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Soil {
    pub texture: Texture,
    pub pores: Pores,
}

/// The soil of each cell of world `w` as built: of its samples' textures (`samples`), the mean weighted by the loose
/// ground each gives the cell's ground node (at most `skin_m` of it; by area where the cell holds none), and the pores
/// of that texture.
pub fn of_world(w: &World) -> Vec<Soil> {
    let (tex, lat, skin) = (samples(w), (w.cfg.lat_south, w.cfg.lat_north), w.cfg.skin_m);
    crate::par::map_n(w.mesh.cells().n(), crate::par::UNCOUNTED, |i| {
        let over = |x: &dyn Fn(&Samples, usize) -> f64| w.terrain.over(w.terrain.seed, &w.map, w.map.leaves()[i], lat, x);
        let held = over(&|s, k| s.loose[k].min(skin));
        let mean = |share: fn(&Texture) -> f64| {
            if held > 0.0 { over(&|s, k| share(&tex[k]) * s.loose[k].min(skin)) / held } else { over(&|_, k| share(&tex[k])) }
        };
        let texture = Texture { coarse: mean(|t| t.coarse), fine: mean(|t| t.fine), organic: mean(|t| t.organic) };
        Soil { texture, pores: Pores::of(&texture) }
    })
}
