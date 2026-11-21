//! The makes of the bed (laws.md 0 and 8): the bed is of a few makes, each a mix of the chemistry's bed substances
//! (`worldgen`), each worn by flowing liquid as much faster than the first as Flow's wear breaks it (`Terrain::cut`).
//! They lie in layers, a stack laid flat and then folded, so a surface that cuts it shows bands that scarps and vales
//! follow and its top layers cap the heights; through them stand a few bodies of the hardest make, as bodies of rock
//! that cooled from a melt rise through strata. Thin seams of other makes lie in the stack at depths drawn from the
//! seed (`Lies::Seam`), as beds laid in still basins lie between thicker ones, and one make may stand round some of the
//! bodies' rims (`Lies::Rim`), as what a melt's fluids leave in the rock it rises through gathers about it. Which make a sample's bed is of is a function of the seed, of how deep
//! the ground there has been worn under the top of the stack, and of the place (`Strata::make`); the wear at the
//! world's start reads it as it cuts (`space::wear`), and the cells' beds are made of it.

use super::square::Square;
use super::terrain::{noise, Place};
use crate::rng::{Purpose, Rng};
use serde::{Deserialize, Serialize};

/// The layers of the stack, which repeats under them, and the least and the most thickness of one, m.
const LAYERS: usize = 12;
const THICK_M: [f64; 2] = [60.0, 400.0];
/// The folds of the stack: the key of their noise, its units a square, its octaves, and how far they raise and lower
/// the layers each way from the mean, m.
const FOLD_KEY: u64 = 101;
const FOLD_PER_SQUARE: f64 = 0.25;
const FOLD_OCTAVES: u32 = 3;
const FOLD_M: f64 = 2000.0;
/// The bodies through the stack: the key of their noise, its units a square, its octaves, and the value of the noise
/// above which the bed is of a body.
const BODY_KEY: u64 = 100;
const BODY_PER_SQUARE: f64 = 0.5;
const BODY_OCTAVES: u32 = 4;
const BODY_ABOVE: f64 = 0.66;
/// The bodies' rims: the value of the bodies' noise above which (and under `BODY_ABOVE`) the bed is of the rim's make,
/// where the noise of the key `RIM_KEY` (its units a square, its octaves) passes `RIM_GATE`, so only some bodies have
/// one.
const RIM_ABOVE: f64 = 0.63;
const RIM_KEY: u64 = 103;
const RIM_PER_SQUARE: f64 = 0.5;
const RIM_OCTAVES: u32 = 2;
const RIM_GATE: f64 = 0.7;
/// The key of the draws of the seams' depths.
const SEAM_KEY: u64 = 102;
/// A sample is of a body where most of it is (the bodies' share carried to a grid).
const MOST: f64 = 0.5;

/// What the ground reads of one make of the bed, from the chemistry's numbers of its substances (`worldgen`): how fast
/// flowing liquid wears it against the first make; the share of the particles it breaks into that are coarse (`soil`);
/// the share of it that weathers into fine particles; and its particles' density over the key liquid's, less one (their
/// submerged weight, which their settling reads); and where it lies (`Lies`).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Make {
    pub cut: f64,
    pub coarse: f64,
    pub weathers: f64,
    pub buoyant: f64,
    pub lies: Lies,
}

/// Where a make lies: in the layers of the stack drawn from the seed; as `per_stack` seams of `m` m in each repeat of
/// the stack; or round the bodies' rims.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum Lies {
    #[default]
    Stack,
    Seam { m: f64, per_stack: u8 },
    Rim,
}

/// The makes of the bed as the seed lays them: the depth of the foot of each layer of the stack under its top, m, and
/// its make; the seams, each its top's and its foot's depth under the top of the stack, m, and its make; the make of the
/// bodies (the hardest of the stack's) and of their rims; and how fast flowing liquid wears each make against the first.
pub(super) struct Strata {
    foot: Vec<f64>,
    of: Vec<u8>,
    seams: Vec<(f64, f64, u8)>,
    body: u8,
    rim: Option<u8>,
    cut: Vec<f64>,
}

impl Strata {
    /// The strata of `seed` for `makes` (`Make::cut`, `Make::lies`): each layer of a thickness and a make of the
    /// stack's drawn, never the make of the layer over it; then the seams, the stack split into as many even slots as
    /// there are seams, each seam at a depth drawn in its own slot, so no two overlap.
    pub(super) fn of(seed: u64, makes: &[Make]) -> Strata {
        let draw = |k: usize| Rng::new(seed, FOLD_KEY, k as u64, Purpose::Terrain).f64();
        let cut: Vec<f64> = makes.iter().map(|m| m.cut).collect();
        let stack: Vec<usize> = (0..makes.len()).filter(|&m| matches!(makes[m].lies, Lies::Stack)).collect();
        let n = stack.len().max(1);
        let (mut foot, mut drawn, mut depth) = (Vec::with_capacity(LAYERS), Vec::<usize>::with_capacity(LAYERS), 0.0);
        for l in 0..LAYERS {
            depth += THICK_M[0] + (THICK_M[1] - THICK_M[0]) * draw(2 * l);
            foot.push(depth);
            let make = match drawn.last() {
                Some(&above) if n > 1 => (above + 1 + (draw(2 * l + 1) * (n - 1) as f64) as usize) % n,
                _ => (draw(2 * l + 1) * n as f64) as usize,
            };
            drawn.push(make);
        }
        let of = drawn.iter().map(|&i| stack.get(i).copied().unwrap_or(0) as u8).collect();
        let body = stack.iter().copied().min_by(|&a, &b| cut[a].total_cmp(&cut[b])).unwrap_or(0) as u8;
        let rim = makes.iter().position(|m| matches!(m.lies, Lies::Rim)).map(|m| m as u8);
        let laid: Vec<(f64, u8)> = (makes.iter().enumerate())
            .flat_map(|(k, m)| match m.lies {
                Lies::Seam { m, per_stack } => vec![(m, k as u8); per_stack as usize],
                _ => Vec::new(),
            })
            .collect();
        let slot = depth / laid.len().max(1) as f64;
        let seams = (laid.iter().enumerate())
            .map(|(j, &(m, make))| {
                let top = j as f64 * slot + (slot - m).max(0.0) * Rng::new(seed, SEAM_KEY, j as u64, Purpose::Terrain).f64();
                (top, top + m, make)
            })
            .collect();
        Strata { foot, of, seams, body, rim, cut }
    }

    /// The fold of the layers at `place` of `map` for `seed`, m (up is positive), and the shares of a body and of a
    /// body's rim there (each 0 or 1).
    pub(super) fn at(seed: u64, map: &Square, place: Place) -> (f64, [f64; 2]) {
        let fold = (2.0 * noise(seed ^ FOLD_KEY, map, place, FOLD_PER_SQUARE, FOLD_OCTAVES) - 1.0) * FOLD_M;
        let body = noise(seed ^ BODY_KEY, map, place, BODY_PER_SQUARE, BODY_OCTAVES);
        let rim = body > RIM_ABOVE && body <= BODY_ABOVE && noise(seed ^ RIM_KEY, map, place, RIM_PER_SQUARE, RIM_OCTAVES) > RIM_GATE;
        (fold, [if body > BODY_ABOVE { 1.0 } else { 0.0 }, if rim { 1.0 } else { 0.0 }])
    }

    /// The make of the bed worn `worn` m under the top of the stack where its layers are folded `fold` m up and a body
    /// and a body's rim stand over the shares `bodies` of the place: a body's, a rim's, a seam's or a layer's.
    pub(super) fn make(&self, worn: f64, fold: f64, bodies: [f64; 2]) -> u8 {
        if bodies[0] > MOST {
            return self.body;
        }
        if let Some(rim) = self.rim.filter(|_| bodies[1] > MOST) {
            return rim;
        }
        let depth = (worn.max(0.0) + fold).rem_euclid(self.foot[LAYERS - 1]);
        if let Some(s) = self.seams.iter().find(|s| s.0 <= depth && depth < s.1) {
            return s.2;
        }
        self.of[self.foot.partition_point(|&f| f <= depth).min(LAYERS - 1)]
    }

    /// How fast flowing liquid wears the bed of `make` (`Strata::make`) against the first make.
    pub(super) fn cut(&self, make: u8) -> f64 {
        self.cut.get(make as usize).copied().unwrap_or(1.0)
    }
}
