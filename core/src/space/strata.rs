//! The makes of the bed (laws.md 0 and 8): the bed is of a few makes, each a mix of the chemistry's bed substances
//! (`worldgen`), each worn by flowing liquid as much faster than the first as Flow's wear breaks it (`Terrain::cut`).
//! They lie in layers, a stack laid flat and then folded, so a surface that cuts it shows bands that scarps and vales
//! follow and its top layers cap the heights; through them stand a few bodies of the hardest make, as bodies of rock
//! that cooled from a melt rise through strata. Which make a sample's bed is of is a function of the seed, of how deep
//! the ground there has been worn under the top of the stack, and of the place (`Strata::make`); the wear at the
//! world's start reads it as it cuts (`space::wear`), and the cells' beds are made of it.

use super::square::Square;
use super::terrain::{noise, Place};
use crate::rng::{Purpose, Rng};

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
/// A sample is of a body where most of it is (the bodies' share carried to a grid).
const MOST: f64 = 0.5;

/// The makes of the bed as the seed lays them: the depth of the foot of each layer of the stack under its top, m, and
/// its make; the make of the bodies (the hardest); and how fast flowing liquid wears each make against the first.
pub(super) struct Strata {
    foot: Vec<f64>,
    of: Vec<u8>,
    body: u8,
    cut: Vec<f64>,
}

impl Strata {
    /// The strata of `seed` for makes worn `cut` times as fast as the first: each layer of a thickness and a make
    /// drawn, never the make of the layer over it.
    pub(super) fn of(seed: u64, cut: &[f64]) -> Strata {
        let draw = |k: usize| Rng::new(seed, FOLD_KEY, k as u64, Purpose::Terrain).f64();
        let n = cut.len().max(1);
        let (mut foot, mut of, mut depth) = (Vec::with_capacity(LAYERS), Vec::<u8>::with_capacity(LAYERS), 0.0);
        for l in 0..LAYERS {
            depth += THICK_M[0] + (THICK_M[1] - THICK_M[0]) * draw(2 * l);
            foot.push(depth);
            let make = match of.last() {
                Some(&above) if n > 1 => (above as usize + 1 + (draw(2 * l + 1) * (n - 1) as f64) as usize) % n,
                _ => (draw(2 * l + 1) * n as f64) as usize,
            };
            of.push(make as u8);
        }
        let body = (0..n).min_by(|&a, &b| cut[a].total_cmp(&cut[b])).unwrap_or(0) as u8;
        Strata { foot, of, body, cut: cut.to_vec() }
    }

    /// The fold of the layers at `place` of `map` for `seed`, m (up is positive), and the share of a body there (0 or
    /// 1).
    pub(super) fn at(seed: u64, map: &Square, place: Place) -> (f64, f64) {
        let fold = (2.0 * noise(seed ^ FOLD_KEY, map, place, FOLD_PER_SQUARE, FOLD_OCTAVES) - 1.0) * FOLD_M;
        (fold, if noise(seed ^ BODY_KEY, map, place, BODY_PER_SQUARE, BODY_OCTAVES) > BODY_ABOVE { 1.0 } else { 0.0 })
    }

    /// The make of the bed worn `worn` m under the top of the stack where its layers are folded `fold` m up and a body
    /// stands over the share `body` of the place.
    pub(super) fn make(&self, worn: f64, fold: f64, body: f64) -> u8 {
        if body > MOST {
            return self.body;
        }
        let depth = (worn.max(0.0) + fold).rem_euclid(self.foot[LAYERS - 1]);
        self.of[self.foot.partition_point(|&f| f <= depth).min(LAYERS - 1)]
    }

    /// How fast flowing liquid wears the bed of `make` (`Strata::make`) against the first make.
    pub(super) fn cut(&self, make: u8) -> f64 {
        self.cut.get(make as usize).copied().unwrap_or(1.0)
    }
}
