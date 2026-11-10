//! The ground a shell draws, in tiles (laws.md 10): what never changes of a place, made once and kept. A tile of
//! `depth` holds `TILE` x `TILE` texels, each a leaf of that depth, and `GUTTER` texels more along each edge, so a
//! shader reads a texel's neighbours inside the tile. Per texel: the terrain's mean height over it
//! (`Terrain::heights`, so a texel is the mean of its four children), the wetness index of the place (`wetness`), and
//! the channel it is nearest by the channel law (`channels`); and apart, the pond it lies in (`Ground::pond_tile`).
//! Each is a function of the seed, the map and the place alone: the same texel holds the same numbers in every tile
//! that holds it.

mod channels;
mod wetness;

pub use wetness::sample_index;

use pcore::space::samples::{Pond, Samples};
use pcore::space::square::Square;
use pcore::space::terrain::Terrain;
use pcore::World;
use std::sync::Arc;

/// Texels along a side of a tile, inside its gutter.
pub const TILE: usize = 256;
/// Texels along each edge of a tile outside its own: a shader's spline reads two texels each way.
pub const GUTTER: usize = 2;
/// Numbers a texel holds (`Ground::tile`).
pub const TEXEL: usize = 4;
/// The finest tiles' texels are no longer than this, m: finer, a shell magnifies them.
const FINEST_M: f64 = 1.0;

/// The depths of the tiles of `map`, coarsest first: from the finest at which one tile holds the whole map to the first
/// whose texels are no longer than `FINEST_M`.
pub fn depths(map: &Square) -> std::ops::RangeInclusive<u8> {
    let sq = map.square_depth();
    let whole = (TILE as f64 / map.w.max(map.h) as f64).log2().floor().max(0.0) as u8;
    let finest = (map.side / FINEST_M).log2().ceil() as u8;
    sq + whole.min(finest)..=sq + finest
}

/// The ground of an island: its seed, map and terrain, the latitudes of its rows, the terrain's samples and the wetness
/// index of each (`sample_index`), and the exponent of a channel's width in its discharge (`Physics::channel`), by
/// which the channel nearest a texel is found.
pub struct Ground {
    seed: u64,
    map: Square,
    terrain: Terrain,
    lat: (f64, f64),
    samples: Arc<Samples>,
    twi: Vec<f64>,
    width_exp: f64,
}

impl Ground {
    /// The ground of `terrain` on the island `map` for `seed`, its rows between latitudes `lat`, its channels as wide as a
    /// discharge to the power `width_exp`.
    pub fn new(seed: u64, map: Square, terrain: Terrain, lat: (f64, f64), width_exp: f64) -> Ground {
        assert!(!map.wrap, "tiles of the ground are made for an island");
        let samples = terrain.samples(seed, &map, lat);
        let twi = sample_index(&map, &samples);
        Ground { seed, map, terrain, lat, samples, twi, width_exp }
    }

    /// The ground of world `w`.
    pub fn of(w: &World) -> Ground {
        Ground::new(w.terrain.seed, w.map.clone(), w.terrain.clone(), (w.cfg.lat_south, w.cfg.lat_north), w.cfg.phys.channel_width_exp)
    }

    /// The tile of `depth` at column `tx` and row `ty` of tiles: (`TILE` + 2 `GUTTER`)² texels row by row from its
    /// north-west corner, each `TEXEL` f32 numbers: the terrain's mean height over the texel, m; the wetness index of
    /// the place (`wetness::at`); and of the channel nearest it by the channel law (`channels::nearest`), the distance to
    /// its centre line over its drained area to the power of a channel's width (m per m² to that power), and its
    /// drained area, m². Texels off the map are those of its edge.
    pub fn tile(&self, depth: u8, tx: i64, ty: i64) -> Vec<f32> {
        assert!(depths(&self.map).contains(&depth), "a tile of a depth the ground is drawn at");
        let side = TILE + 2 * GUTTER;
        let (x0, y0) = (tx * TILE as i64 - GUTTER as i64, ty * TILE as i64 - GUTTER as i64);
        let per = 1i64 << (depth - self.map.square_depth());
        let (across, down) = (self.map.w as i64 * per, self.map.h as i64 * per);
        let clamp = |v: i64, n: i64| v.clamp(0, n - 1);
        let win = [clamp(x0, across), clamp(y0, down), clamp(x0 + side as i64 - 1, across), clamp(y0 + side as i64 - 1, down)];
        let heights = self.terrain.heights(self.seed, &self.map, self.lat, win, depth);
        let wide = (win[2] - win[0] + 1) as usize;
        let texel = |p: usize| (clamp(x0 + (p % side) as i64, across), clamp(y0 + (p / side) as i64, down));
        let near = channels::Near::new(&self.map, &self.samples, self.width_exp, [self.centre(depth, win[0], win[1]), self.centre(depth, win[2], win[3])]);
        let mut out = Vec::with_capacity(side * side * TEXEL);
        for (x, y) in (0..side * side).map(texel) {
            let h = heights[(y - win[1]) as usize * wide + (x - win[0]) as usize];
            let (ratio, area) = near.nearest(self.centre(depth, x, y));
            out.extend([h, wetness::at(&self.samples, &self.twi, depth, (x, y)), ratio, area].map(|v| v as f32));
        }
        out
    }

    /// The pond each texel of the tile of `depth` at column `tx` and row `ty` lies in, laid out as `tile`'s texels: the
    /// pond (`Samples::pond`, an index into `ponds`; `network::NONE` for none) of the sample that holds the texel's
    /// centre. So a shell can draw each pond flat at its own level.
    pub fn pond_tile(&self, depth: u8, tx: i64, ty: i64) -> Vec<u32> {
        let side = TILE + 2 * GUTTER;
        let (x0, y0) = (tx * TILE as i64 - GUTTER as i64, ty * TILE as i64 - GUTTER as i64);
        let s = &self.samples;
        let sample = |c: f64, n: usize| (c.floor() as i64).clamp(0, n as i64 - 1) as usize;
        (0..side * side)
            .map(|p| {
                let [x, y] = self.centre(depth, x0 + (p % side) as i64, y0 + (p / side) as i64);
                s.pond[sample(y, s.down) * s.across + sample(x, s.across)]
            })
            .collect()
    }

    /// The ponds of the ground, each with its pass and its level (`Samples::ponds`).
    pub fn ponds(&self) -> &[Pond] {
        &self.samples.ponds
    }

    /// The centre of the texel at column `x` and row `y` of leaves of `depth`, in samples from the map's north-west corner.
    fn centre(&self, depth: u8, x: i64, y: i64) -> [f64; 2] {
        let k = 2f64.powi(pcore::space::terrain::SAMPLE_DEPTH as i32 - depth as i32);
        [(x as f64 + 0.5) * k, (y as f64 + 0.5) * k]
    }
}
