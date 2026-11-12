//! The ground a shell draws, in tiles (laws.md 10): what never changes of a place, made once and kept. A tile of
//! `depth` holds `TILE` x `TILE` texels, each a leaf of that depth, and `GUTTER` texels more along each edge, so a
//! shader reads a texel's neighbours inside the tile. Per texel: the terrain's mean height over it
//! (`Terrain::heights`, so a texel is the mean of its four children), the wetness index of the place (`wetness`), and
//! the channel it is nearest by the channel law (`channels`), with how far along that channel's line it lies and how
//! the line curves there; and apart, the pond it lies in (`Ground::pond_tile`).
//! Each is a function of the seed, the map and the place alone: the same texel holds the same numbers in every tile
//! that holds it. A tile is 16-bit floats (`half`), so a graphics card keeps many: the height as the sum of two of
//! them, to a millimetre (a shore is the difference of two heights).

mod belt;
mod bends;
mod channels;
mod half;
mod rock;
mod shore;
mod stems;
mod valley;
mod wetness;

pub use bends::FLOOD;
pub use channels::{Law, ALONG_WRAP_M, SHOW_M, STEP_MOST, TYPICAL_RUNOFF};
pub use valley::{VALLEY_FROM, VALLEY_M};
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
/// The samples each way about a texel's own in which a pond may take it (`Ground::pond_tile`): as far as the drawn shore
/// may lie from the samples' (`shore`).
const REACH: i64 = 2;
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
/// index of each (`sample_index`), the channel law (`Physics::channel`), by which the channel nearest a texel is found,
/// the point each sample's line runs through (`stems::points`) and how each sample's piece of its line starts
/// (`bends::Bends`).
pub struct Ground {
    seed: u64,
    map: Square,
    terrain: Terrain,
    lat: (f64, f64),
    samples: Arc<Samples>,
    twi: Vec<f64>,
    law: Law,
    at: Vec<[f64; 2]>,
    bends: bends::Bends,
    shore: shore::Shore,
}

impl Ground {
    /// The ground of `terrain` on the island `map` for `seed`, its rows between latitudes `lat`, its channels sized by
    /// `law`.
    pub fn new(seed: u64, map: Square, terrain: Terrain, lat: (f64, f64), law: Law) -> Ground {
        assert!(!map.wrap, "tiles of the ground are made for an island");
        let samples = terrain.samples(seed, &map, lat);
        let twi = sample_index(&map, &samples);
        let at = stems::points(&samples);
        let bends = bends::Bends::new(&bends::Reaches { seed, s: &samples, at: &at, law, metres: map.length(SAMPLE_DEPTH) });
        let shore = shore::Shore::new(&samples);
        Ground { seed, map, terrain, lat, samples, twi, law, at, bends, shore }
    }

    /// The ground of world `w`.
    pub fn of(w: &World) -> Ground {
        Ground::new(w.terrain.seed, w.map.clone(), w.terrain.clone(), (w.cfg.lat_south, w.cfg.lat_north), Law::of(&w.cfg.phys))
    }

    /// The tile of `depth` at column `tx` and row `ty` of tiles, `TILE` + 2 `GUTTER` texels a side row by row from its
    /// north-west corner, as 16-bit floats in two planes. The first, four a texel: the terrain's mean height over the
    /// texel, m, as the sum of two (the nearest 16-bit float to it, and what it lacks); the wetness index of the place
    /// (`wetness::field`); and the distance to the line of the channel it keeps, m, signed by the side
    /// (`channels::Field`; 60,000 where none reaches it). The second, four a texel: that channel's sample, as its
    /// step from the sample that holds the texel's centre (`channels::code`; 0 for none), and one more than the pond the
    /// texel lies in (`pond_tile`; 0 for none), whole numbers a 16-bit float holds exactly up to `PONDS_EXACT`; and of
    /// the nearest point of that channel's line, how far along the channel it lies, m, modulo `ALONG_WRAP_M`, and the
    /// line's curvature there, 1/m, positive where it turns toward the side the distance is positive on (0 for none).
    /// Texels off the map are those of its edge.
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
        let reaches = bends::Reaches { seed: self.seed, s: &self.samples, at: &self.at, law: self.law, metres: self.map.length(SAMPLE_DEPTH) };
        let near = channels::Channels::new(reaches, &self.bends).field(channels::Window { x0: win[0], y0: win[1], wide, high, k });
        // The valleys about the channels drawn into the ground out of the ponds (`channels::Field::drawn`); near a pond
        // (`shore::Shore::level`) a valley only lowers the ground, and never under the pond's level where the ground is
        // not: its mouth meets the level of the pond it runs into. Then the shore drawn smooth (`shore`).
        let heights: Vec<f64> = heights
            .iter()
            .enumerate()
            .map(|(i, &h)| {
                let p = self.centre(depth, win[0] + (i % wide) as i64, win[1] + (i / wide) as i64);
                let (c, r) = ((p[0].floor() as i64).min(self.samples.across as i64 - 1), (p[1].floor() as i64).min(self.samples.down as i64 - 1));
                let level = self.shore.level(c, r);
                let carved = if self.samples.pond[r as usize * self.samples.across + c as usize] != NONE {
                    h
                } else if level.is_nan() {
                    near.drawn(i, h)
                } else {
                    near.drawn(i, h).min(h).max(h.min(level))
                };
                self.shore.drawn(p, carved, level)
            })
            .collect();
        let at = |p: usize| (clamp(x0 + (p % side) as i64, across) - win[0]) as usize + (clamp(y0 + (p / side) as i64, down) - win[1]) as usize * wide;
        let mut out = Vec::with_capacity(8 * side * side);
        for i in (0..side * side).map(at) {
            let hi = half(heights[i] as f32);
            out.extend([hi, half((heights[i] - unhalf(hi) as f64) as f32), half(twi[i] as f32), half(near.dist[i] as f32)]);
        }
        let ponds = self.pond_tile(depth, tx, ty, |p| heights[at(p)]);
        for (p, i) in (0..side * side).map(|p| (p, at(p))) {
            let pond = if ponds[p] == NONE { 0.0 } else { (ponds[p] + 1) as f32 };
            let texel = [win[0] + (i % wide) as i64, win[1] + (i / wide) as i64];
            let along = near.along[i].rem_euclid(ALONG_WRAP_M);
            out.extend([half(channels::code(&self.samples, texel, k, near.sample[i]) as f32), half(pond), half(along as f32), half(near.curve[i] as f32)]);
        }
        out
    }

    /// The pond each texel of the tile of `depth` at column `tx` and row `ty` lies in, laid out as `tile`'s texels, its
    /// height `h(p)`, m: the pond (`Samples::pond`, an index into `ponds`; `network::NONE` for none) of the sample that
    /// holds the texel's centre, or where that lies in none, of the nearest of the samples within `REACH` that lies in a pond
    /// whose pass the texel lies under, where that sample stands over the pond's level (its rim: not the pass, nor the
    /// ground its liquid runs down below it) or the pond is of the open low ground. So a shell draws each pond flat at
    /// its own level, its shore where the ground rises over that level, not at the edges of the samples.
    pub fn pond_tile(&self, depth: u8, tx: i64, ty: i64, h: impl Fn(usize) -> f64) -> Vec<u32> {
        let side = TILE + 2 * GUTTER;
        let (x0, y0) = (tx * TILE as i64 - GUTTER as i64, ty * TILE as i64 - GUTTER as i64);
        let s = &self.samples;
        let pond_at = |r: i64, c: i64| s.pond[r.clamp(0, s.down as i64 - 1) as usize * s.across + c.clamp(0, s.across as i64 - 1) as usize];
        let sample = |x: i64, y: i64| self.centre(depth, x, y).map(|v| v.floor() as i64);
        // The samples within `REACH`, nearest first; the samples that hold the tile's texels' centres: of each, the
        // ponds of those about it, each once, in the order they are tried.
        let mut by: Vec<(i64, i64)> = (-REACH..=REACH).flat_map(|dr| (-REACH..=REACH).map(move |dc| (dc, dr))).collect();
        by.sort_by_key(|&(dc, dr)| dc * dc + dr * dr);
        let ([c0, r0], [c1, r1]) = (sample(x0, y0), sample(x0 + side as i64 - 1, y0 + side as i64 - 1));
        let wide = (c1 - c0 + 1) as usize;
        let near: Vec<Vec<u32>> = (r0..=r1)
            .flat_map(|r| (c0..=c1).map(move |c| (r, c)))
            .map(|(r, c)| {
                let own = s.h[r.clamp(0, s.down as i64 - 1) as usize * s.across + c.clamp(0, s.across as i64 - 1) as usize];
                let rim = |q: u32| q != NONE && (s.ponds[q as usize].pass == NONE || own > s.ponds[q as usize].level);
                let mut ponds: Vec<u32> = Vec::new();
                for q in by.iter().map(|(dc, dr)| pond_at(r + dr, c + dc)).filter(|&q| rim(q)) {
                    if !ponds.contains(&q) {
                        ponds.push(q);
                    }
                }
                ponds
            })
            .collect();
        (0..side * side)
            .map(|p| {
                let [c, r] = sample(x0 + (p % side) as i64, y0 + (p / side) as i64);
                let own = pond_at(r, c);
                if own != NONE {
                    return own;
                }
                near[(r - r0) as usize * wide + (c - c0) as usize].iter().copied().find(|&q| h(p) < s.ponds[q as usize].level).unwrap_or(NONE)
            })
            .collect()
    }

    /// The rock of each of the terrain's samples, row by row in two planes of four 16-bit floats a sample: the depth of
    /// its loose ground, m, the largest stone its channel moves, m, and its channel's links; and the reflectance of
    /// its bed's make, of `colours` one a make (`rock::rock`).
    pub fn rock(&self, colours: &[[f64; 3]]) -> Vec<u16> {
        rock::rock(&self.samples, self.map.length(SAMPLE_DEPTH), colours)
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
