//! The ground a shell draws, in tiles (laws.md 10): what never changes of a place, made once and kept. A tile of
//! `depth` holds `TILE` x `TILE` texels, each a leaf of that depth, and `GUTTER` texels more along each edge, so a
//! shader reads a texel's neighbours inside the tile. Per texel: the terrain's mean height over it
//! (`Terrain::heights`, so a texel is the mean of its four children), the wetness index of the place (`wetness`), and
//! the channel it is nearest by the channel law (`channels`); and apart, the pond it lies in (`Ground::pond_tile`).
//! Each is a function of the seed, the map and the place alone: the same texel holds the same numbers in every tile
//! that holds it. A tile is 16-bit floats (`half`), so a graphics card keeps many: the height as the sum of two of
//! them, to a millimetre (a shore is the difference of two heights).

mod channels;
mod half;
mod wetness;

pub use channels::MOST_RUNOFF;
pub use half::{half, unhalf};
pub use wetness::sample_index;

use pcore::space::network::NONE;
use pcore::space::samples::{Pond, Samples};
use pcore::space::square::Square;
use pcore::space::terrain::{Terrain, SAMPLE_DEPTH};
use pcore::World;
use std::sync::Arc;

/// Texels along a side of a tile, inside its gutter.
pub const TILE: usize = 256;
/// Texels along each edge of a tile outside its own: a shader's spline reads two texels each way.
pub const GUTTER: usize = 2;
/// The ponds a tile names exactly (`Ground::tile`): a 16-bit float holds every whole number up to 2^11.
pub const PONDS_EXACT: usize = 2048;
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
/// index of each (`sample_index`), and a channel's width at a discharge of 1 m³/s, m, and the exponent of its width in
/// its discharge (`Physics::channel`), by which the channel nearest a texel is found.
pub struct Ground {
    seed: u64,
    map: Square,
    terrain: Terrain,
    lat: (f64, f64),
    samples: Arc<Samples>,
    twi: Vec<f64>,
    channel: [f64; 2],
}

impl Ground {
    /// The ground of `terrain` on the island `map` for `seed`, its rows between latitudes `lat`, its channels `channel[0]`
    /// m wide at a discharge of 1 m³/s and as wide as a discharge to the power `channel[1]`.
    pub fn new(seed: u64, map: Square, terrain: Terrain, lat: (f64, f64), channel: [f64; 2]) -> Ground {
        assert!(!map.wrap, "tiles of the ground are made for an island");
        let samples = terrain.samples(seed, &map, lat);
        let twi = sample_index(&map, &samples);
        Ground { seed, map, terrain, lat, samples, twi, channel }
    }

    /// The ground of world `w`.
    pub fn of(w: &World) -> Ground {
        Ground::new(w.terrain.seed, w.map.clone(), w.terrain.clone(), (w.cfg.lat_south, w.cfg.lat_north), [w.cfg.phys.channel_width, w.cfg.phys.channel_width_exp])
    }

    /// The tile of `depth` at column `tx` and row `ty` of tiles, `TILE` + 2 `GUTTER` texels a side row by row from its
    /// north-west corner, as 16-bit floats in two planes. The first, four a texel: the terrain's mean height over the
    /// texel, m, as the sum of two (the nearest 16-bit float to it, and what it lacks); the wetness index of the place
    /// (`wetness::field`); and of the channel nearest it by the channel law, its distance to its centre line over its
    /// drained area to the power of a channel's width, over the cap (`channels::Field`, at most 1). The second, two a
    /// texel: the logarithm of that channel's drained area in m² (0 where none comes within reach), and one more than
    /// the pond the texel lies in (`pond_tile`; 0 for none), whole numbers a 16-bit float holds exactly up to
    /// `PONDS_EXACT`. Texels off the map are those of its edge.
    pub fn tile(&self, depth: u8, tx: i64, ty: i64) -> Vec<u16> {
        assert!(depths(&self.map).contains(&depth), "a tile of a depth the ground is drawn at");
        let side = TILE + 2 * GUTTER;
        let (x0, y0) = (tx * TILE as i64 - GUTTER as i64, ty * TILE as i64 - GUTTER as i64);
        let per = 1i64 << (depth - self.map.square_depth());
        let (across, down) = (self.map.w as i64 * per, self.map.h as i64 * per);
        let clamp = |v: i64, n: i64| v.clamp(0, n - 1);
        // The window of the map's texels the tile holds; the tile's texels off the map repeat its edge.
        let win = [clamp(x0, across), clamp(y0, down), clamp(x0 + side as i64 - 1, across), clamp(y0 + side as i64 - 1, down)];
        let (wide, high) = ((win[2] - win[0] + 1) as usize, (win[3] - win[1] + 1) as usize);
        let heights = self.terrain.heights(self.seed, &self.map, self.lat, win, depth);
        let k = 2f64.powi(SAMPLE_DEPTH as i32 - depth as i32);
        let twi = wetness::field(&self.samples, &self.twi, depth, [win[0], win[1]], [wide, high]);
        let near = channels::Channels::new(&self.map, &self.samples, self.channel[0], self.channel[1]).field([win[0], win[1]], [wide, high], k);
        let at = |p: usize| (clamp(x0 + (p % side) as i64, across) - win[0]) as usize + (clamp(y0 + (p / side) as i64, down) - win[1]) as usize * wide;
        let mut out = Vec::with_capacity(6 * side * side);
        for i in (0..side * side).map(at) {
            let hi = half(heights[i] as f32);
            out.extend([hi, half((heights[i] - unhalf(hi) as f64) as f32), half(twi[i] as f32), half(near.ratio[i] as f32)]);
        }
        let ponds = self.pond_tile(depth, tx, ty, |p| heights[at(p)]);
        for (p, i) in (0..side * side).map(|p| (p, at(p))) {
            let pond = if ponds[p] == NONE { 0.0 } else { (ponds[p] + 1) as f32 };
            out.extend([half(if near.area[i] > 0.0 { near.area[i].ln() as f32 } else { 0.0 }), half(pond)]);
        }
        out
    }

    /// The pond each texel of the tile of `depth` at column `tx` and row `ty` lies in, laid out as `tile`'s texels, its
    /// height `h(p)`, m: the pond (`Samples::pond`, an index into `ponds`; `network::NONE` for none) of the sample that
    /// holds the texel's centre, or where that lies in none, of the first of its eight neighbours that lies in a pond
    /// whose pass the texel lies under. So a shell draws each pond flat at its own level, its shore where the ground
    /// rises over that level, not at the edges of the samples.
    pub fn pond_tile(&self, depth: u8, tx: i64, ty: i64, h: impl Fn(usize) -> f64) -> Vec<u32> {
        let side = TILE + 2 * GUTTER;
        let (x0, y0) = (tx * TILE as i64 - GUTTER as i64, ty * TILE as i64 - GUTTER as i64);
        let s = &self.samples;
        let pond_at = |r: i64, c: i64| s.pond[r.clamp(0, s.down as i64 - 1) as usize * s.across + c.clamp(0, s.across as i64 - 1) as usize];
        (0..side * side)
            .map(|p| {
                let [x, y] = self.centre(depth, x0 + (p % side) as i64, y0 + (p / side) as i64);
                let (r, c) = (y.floor() as i64, x.floor() as i64);
                let own = pond_at(r, c);
                if own != NONE {
                    return own;
                }
                let around = (-1..=1i64).flat_map(|dr| (-1..=1i64).map(move |dc| (r + dr, c + dc)));
                around.map(|(r, c)| pond_at(r, c)).find(|&q| q != NONE && h(p) < s.ponds[q as usize].level).unwrap_or(NONE)
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
