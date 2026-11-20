//! The soil (laws.md 4): what the loose ground is made of. Its texture, the shares of fine, middle and coarse
//! particles, is what the laws that make it leave in each of the terrain's samples (`texture`); a cell's is its
//! samples' mean. Its organic share is the dead matter its ground holds now (`month`), and its pores follow from both
//! (`pores`); its fine particles and organic matter hold a share of its dissolved minerals (`exchange`).

mod exchange;
mod organic;
mod pores;
mod texture;

pub(crate) use exchange::dissolved;
pub use organic::{holdings, Holdings};
pub use pores::Pores;
pub use texture::{samples, textures};
pub(crate) use texture::{flood_shear, passes};

use crate::chem::Phase;
use crate::space::samples::Samples;
use crate::units::T_SURF;
use crate::World;
use serde::{Deserialize, Serialize};

/// The texture of loose ground: the shares of its mineral particles that are coarse (50 µm to 2 mm) and fine
/// (under 2 µm), the rest middle (`Texture::middle`); and the share of its solid volume that is organic matter (a
/// cell's, `organic`; none in the terrain's samples).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Texture {
    pub coarse: f64,
    pub fine: f64,
    pub organic: f64,
}

/// A loam: its particles 0.4 coarse and 0.2 fine, with no organic matter.
pub const LOAM: Texture = Texture { coarse: 0.4, fine: 0.2, organic: 0.0 };
/// The erodibility of loose ground grows as the power `ERODIBLE_EXP` of its middle share times its share that is not
/// fine, and falls in a straight line with its organic matter, from none at the per cent `ORGANIC_SPAN` (Wischmeier
/// and Smith 1978), read to `ORGANIC_MOST` per cent (the range of their data).
const ERODIBLE_EXP: f64 = 1.14;
/// The share of the decomposers' rate on what fine and middle particles hold (`Substance::bound`) that loose ground all
/// of such particles takes away (Parton et al. 1987).
const SHELTERED: f64 = 0.75;
const ORGANIC_SPAN: f64 = 12.0;
const ORGANIC_MOST: f64 = 4.0;

impl Texture {
    /// The share of middle particles.
    pub fn middle(&self) -> f64 {
        1.0 - self.coarse - self.fine
    }
    /// How easily flowing liquid carries off loose ground of this texture, against a loam (`LOAM`, 1): the soil
    /// erodibility factor of Wischmeier and Smith (1978), M^1.14 (12 − OM), M the middle share times the share that
    /// is not fine (their two finest classes times what is not of the finest), OM the organic per cent. Middle particles
    /// are carried most easily: coarse ones are heavy, fine ones bind together, and organic matter binds them too.
    /// The share of the decomposers' rate on what its fine and middle particles hold (`Substance::bound`) that they
    /// take away by holding it: `SHELTERED` times their share, as the texture slows the rot of the organic matter held
    /// on mineral faces (Parton et al. 1987; Six et al. 2002).
    pub fn shelters(&self) -> f64 {
        SHELTERED * (self.fine + self.middle()).clamp(0.0, 1.0)
    }
    pub fn erodibility(&self) -> f64 {
        let k = |t: &Texture| (t.middle().max(0.0) * (1.0 - t.fine)).powf(ERODIBLE_EXP) * (ORGANIC_SPAN - (100.0 * t.organic).min(ORGANIC_MOST));
        k(self) / k(&LOAM)
    }
}

/// The soil of a cell: its texture; its pores (`Pores::of` the texture, found anew each month its organic share moves,
/// `month`); and the liquid whose dissolved minerals its exchange sites hold as much of as it does, m³
/// (`exchange::holds`, found each month).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Soil {
    pub texture: Texture,
    pub pores: Pores,
    pub holds: f64,
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
        Soil { texture, pores: Pores::of(&texture), holds: 0.0 }
    })
}

/// Each cell's organic share anew from what its ground holds now (`solids`), and its pores where the share or its
/// texture moved (`flow::new_pores`, which books the potential energy of the liquid they hold): the dead matter its
/// cover leaves on its ground, which rots there as the open world's reactions rot it (laws.md 2.1), so ground under a
/// dense cover grows richer in organic matter and holds more liquid, and bare ground loses it (found again where it
/// moved by more than `ORGANIC_STEP`); the load that settles on it mixes into its texture (`flow::runoff`; found again
/// where its field capacity moved by more than `TEXTURE_STEP`); and its exchange sites (`exchange::holds`).
/// Once a month, at its start.
pub fn month(w: &mut World) {
    let Some(parts) = crate::life::Parts::of(&w.chem) else { return };
    let organics: Vec<usize> = parts.sub.iter().copied().chain((0..w.chem.ns()).filter(|&k| w.chem.subs[k].bound)).collect();
    crate::flow::new_pores(w, |w| organic_pores(w, &organics));
}

/// A cell's organic share and its pores are found again once its organic matter has moved by more than this share of
/// what it was when they were last found: its pores move by less than a twentieth of what the organic share moves
/// them over a loam's (a speed tolerance).
const ORGANIC_STEP: f64 = 0.05;
/// A cell's pores are found again once its texture has moved its field capacity by more than this share of what it
/// was when they were last found (a speed tolerance: the liquid its ground holds moves by less).
const TEXTURE_STEP: f64 = 0.02;

/// Each cell's exchange sites anew (`solids`, of the substances `organics`), and its organic share and pores where the
/// share has moved by more than `ORGANIC_STEP` or its texture its field capacity by more than `TEXTURE_STEP`. Returns
/// the cells whose pores changed.
fn organic_pores(w: &mut World, organics: &[usize]) -> Vec<u32> {
    let mut moved = Vec::new();
    for i in 0..w.mesh.cells().n() {
        let [organic, all] = solids(w, organics, i);
        let share = if all > 0.0 { organic / all } else { 0.0 };
        let soil = &mut w.soil[i];
        soil.holds = exchange::holds(all - organic, soil.texture.fine, organic);
        let organic = (share - soil.texture.organic).abs() > ORGANIC_STEP * soil.texture.organic || (share == 0.0) != (soil.texture.organic == 0.0);
        let texture = (Pores::of(&soil.texture).field - soil.pores.field).abs() > TEXTURE_STEP * soil.pores.field;
        if organic || texture {
            soil.texture.organic = share;
            soil.pores = Pores::of(&soil.texture);
            moved.push(i as u32);
        }
    }
    moved
}

/// The loose ground of cell `i`: of the volume of the solids of its ground node that stay solid at the surface
/// (`T_SURF`: not its frozen liquid), m³, what is organic (`organics`: what life's parts are made of and what
/// particles hold), and all.
fn solids(w: &World, organics: &[usize], i: usize) -> [f64; 2] {
    let (mut of_life, mut all) = (0.0, 0.0);
    for k in (0..w.chem.ns()).filter(|&k| w.chem.subs[k].melt > T_SURF) {
        let v = w.g(k, i) * w.chem.molar_volume(k, Phase::Solid);
        all += v;
        of_life += if organics.contains(&k) { v } else { 0.0 };
    }
    [of_life, all]
}
