//! The ground a shell draws, in tiles (laws.md 10): what never changes of a place, made once and kept. A tile of
//! `depth` holds `TILE` x `TILE` texels, each a leaf of that depth, and `GUTTER` texels more along each edge, so a
//! shader reads a texel's neighbours inside the tile. Per texel: the terrain's mean height over it
//! (`Terrain::heights_on`, so a texel is the mean of its four children), the wetness index of the place (`wetness`), and
//! the channel it is nearest by the channel law (`channels`), with how far along that channel's line it lies and how
//! far from that line without its bends; and the pond whose liquid may stand on it (`shore`).
//! Each is a function of the seed, the map and the place alone: the same texel holds the same numbers in every tile
//! that holds it. A tile is 16-bit floats (`half`), so a graphics card keeps many: the height as the sum of two of
//! them, to a millimetre (a shore is the difference of two heights).

mod belt;
mod bends;
mod channels;
mod frame;
mod lines;
mod mend;
mod half;
mod rebuild;
mod rock;
mod shore;
mod stems;
mod valley;
mod wander;
mod wetness;

pub use bends::{FLOOD, STATION_EXP};
pub use channels::{Law, ALONG_WRAP_M, LINE_GROW, LINE_MOST, LINE_PX, SHOW_M, STEP_MOST, TYPICAL_RUNOFF};
pub use valley::{VALLEY_FROM, VALLEY_M};
pub use half::{half, unhalf};

use pcore::space::network::NONE;
use pcore::space::relief::Relief;
use pcore::space::samples::{Pond, Samples};
use pcore::space::square::Square;
use pcore::space::terrain::{Terrain, SAMPLE_DEPTH};
use pcore::{Physics, World};
use std::sync::Arc;

/// Texels along a side of a tile, inside its gutter.
pub const TILE: usize = 256;
/// Texels along each edge of a tile outside its own: a shader's spline reads two texels each way.
pub const GUTTER: usize = 2;
/// The ponds a tile names exactly (`Ground::tile`): a 16-bit float holds every whole number up to 2^11.
pub const PONDS_EXACT: usize = 2048;
/// The finest tiles' texels are no longer than this, m: finer, a shell magnifies them.
const FINEST_M: f64 = 1.0;

/// Whether the shore a shell draws leaves each of the samples `s` out of its pond's liquid (`shore::Shore::dry`).
pub fn shore_dry(s: &Samples) -> Vec<bool> {
    shore::Shore::new(s).dry()
}

/// The depths of the tiles of `map`, coarsest first: from the finest at which one tile holds the whole map to the first
/// whose texels are no longer than `FINEST_M`.
pub fn depths(map: &Square) -> std::ops::RangeInclusive<u8> {
    let sq = map.square_depth();
    let whole = (TILE as f64 / map.w.max(map.h) as f64).log2().floor().max(0.0) as u8;
    let finest = (map.side / FINEST_M).log2().ceil() as u8;
    sq + whole.min(finest)..=sq + finest
}

/// The ground of an island: its seed, map and terrain, the latitudes of its rows, the terrain's samples (with the
/// wetness index of each, `space::samples::sample_index`), the channel law (`Physics::channel`), by which the channel nearest a texel is found,
/// the point each sample's line runs through (`stems::points`), how each sample's piece of its line starts
/// (`bends::Bends`), the texture of each sample's loose ground (`pcore::soil::textures`), the samples whose line
/// carries their wetness index (`wetness::strips`) and the wetness index of each as the hillslope holds it
/// (`wetness::hill`); and the laws and the gravity it was made under, by which it is made again (`rebuild`).
pub struct Ground {
    seed: u64,
    map: Square,
    terrain: Terrain,
    lat: (f64, f64),
    samples: Arc<Samples>,
    law: Law,
    at: Vec<[f64; 2]>,
    raw: Vec<[f64; 2]>,
    bends: bends::Bends,
    shore: shore::Shore,
    grain: Vec<pcore::soil::Texture>,
    strips: Vec<bool>,
    hill: Vec<f64>,
    phys: Physics,
    gravity: f64,
}

impl Ground {
    /// The ground of `terrain` on the island `map` for `seed`, its rows between latitudes `lat`, under the laws `phys`
    /// at gravity `gravity`: its channels sized by them (`Law`), and its samples' texture (`pcore::soil::textures`).
    pub fn new(seed: u64, map: Square, terrain: Terrain, lat: (f64, f64), phys: &Physics, gravity: f64) -> Ground {
        let samples = terrain.samples(seed, &map, lat);
        Ground::on(seed, (map, terrain, lat), (phys, gravity), samples)
    }

    /// The same on the terrain's samples `samples` (the seed's, or as a world built them: `Terrain::built`).
    pub fn on(seed: u64, (map, terrain, lat): (Square, Terrain, (f64, f64)), (phys, gravity): (&Physics, f64), samples: Arc<Samples>) -> Ground {
        let law = Law::of(phys);
        assert!(!map.wrap, "tiles of the ground are made for an island");
        let shore = shore::Shore::new(&samples);
        let (at, raw) = stems::points(&samples, &shore);
        let bends = bends::Bends::new(&bends::Reaches { seed, s: &samples, at: &at, law, metres: map.length(SAMPLE_DEPTH) });
        let grain = pcore::soil::textures(&samples, &terrain.makes, gravity);
        let strips = wetness::strips(&samples, phys);
        let hill = wetness::hill(&samples, &strips);
        Ground { seed, map, terrain, lat, samples, law, at, raw, bends, shore, grain, strips, hill, phys: *phys, gravity }
    }

    /// The ground of world `w`, on its samples as last built (`World::terrain_samples`).
    pub fn of(w: &World) -> Ground {
        // (Its samples first, so the terrain it keeps holds them and how they were raised, `Terrain::built`.)
        let samples = w.terrain_samples();
        Ground::on(w.terrain.seed, (w.map.clone(), w.terrain.clone(), (w.cfg.lat_south, w.cfg.lat_north)), (&w.cfg.phys, w.cfg.gravity), samples)
    }

    /// The relief a world built its samples from (`World::relief`), the years since its start at its last build and the
    /// world's map, whose cells the relief's offsets are of (`Relief::offset`), as bytes (bincode), for a shell's ground
    /// to build the same samples (`rebuild_from`): the ground's own map may cut the island into other cells (a page's
    /// ground is made of its squares, `worldgen::terrain::island_ground`).
    pub fn relief_bytes(w: &World) -> Vec<u8> {
        bincode::serialize(&(&w.relief, w.built_years(), &w.map)).expect("a relief serializes")
    }

    /// The ground made again on the samples the relief `bytes` builds on the world's cells (`relief_bytes`,
    /// `Terrain::built`; `rebuild`). Returns the squares whose tiles changed, or why the bytes are refused.
    pub fn rebuild_from(&mut self, bytes: &[u8]) -> Result<Vec<bool>, String> {
        let (r, years, cells): (Relief, f64, Square) = bincode::deserialize(bytes).map_err(|e| format!("a relief: {e}"))?;
        let s = self.terrain.built(self.seed, &cells, self.lat, &r, years);
        Ok(self.rebuild(s))
    }

    /// The ground made again on the samples `new` (as a world built them again, `Terrain::built`): what each sample
    /// holds of it found again, but the shore (`shore::Shore::update`), the points the lines run through
    /// (`stems::update`), the texture (`pcore::soil::texture_at`) and the pieces of the channels (`bends::Bends`), found
    /// again only where they may read a sample that changed (`rebuild::near`, `BENDS_READ`; and every piece down
    /// the ways from those, whose start they set): the rest read what they did. Returns the squares whose tiles changed (`rebuild`), row by row, true for each.
    pub fn rebuild(&mut self, new: Arc<Samples>) -> Vec<bool> {
        let old = self.samples.clone();
        let mut shore = std::mem::take(&mut self.shore);
        let shore_moved = shore.update(&old, &new);
        let ways: Vec<bool> = (0..new.h.len()).map(|k| old.main[k] != new.main[k] || old.to[k] != new.to[k]).collect();
        let (mut at, mut raw) = (self.at.clone(), std::mem::take(&mut self.raw));
        stems::update(&new, &shore, (&mut at, &mut raw), (&ways, &shore_moved));
        let mut grain = std::mem::take(&mut self.grain);
        let read = rebuild::near(&new, &rebuild::moved(&old, &new), rebuild::GRAIN_READ);
        (0..new.h.len()).filter(|&k| read[k]).for_each(|k| grain[k] = pcore::soil::texture_at(&new, &self.terrain.makes, self.gravity, k));
        let strips = wetness::strips(&new, &self.phys);
        let hill = wetness::hill(&new, &strips);
        let changed = rebuild::changed(&self.read(), &rebuild::Read { s: &new, hill: &hill, strips: &strips, at: &at });
        let reaches = bends::Reaches { seed: self.seed, s: &new, at: &at, law: self.law, metres: self.map.length(SAMPLE_DEPTH) };
        self.bends.update(&reaches, &rebuild::readers(&new, &rebuild::near(&new, &changed, rebuild::BENDS_READ)));
        let squares = rebuild::squares(&self.map, &new, &rebuild::readers(&new, &changed));
        (self.samples, self.at, self.raw, self.shore, self.grain, self.strips, self.hill) = (new, at, raw, shore, grain, strips, hill);
        squares
    }

    /// The tile of `depth` at column `tx` and row `ty` of tiles, `TILE` + 2 `GUTTER` texels a side row by row from its
    /// north-west corner, as 16-bit floats in two planes. The first, four a texel: the terrain's mean height over the
    /// texel, m, as the sum of two (the nearest 16-bit float to it, and what it lacks); the wetness index of the place
    /// (`wetness::field`); and the distance to the line of the channel it keeps, m, signed by the side
    /// (`channels::Field`; 60,000 where none reaches it). The second, four a texel: that channel's sample, as its
    /// step from the sample that holds the texel's centre (`channels::code`; 0 for none), and one more than the pond
    /// whose liquid may stand on the texel (`shore::Shore::drawn`; 0 for none), whole numbers a 16-bit float holds
    /// exactly up to `PONDS_EXACT`; and of
    /// the nearest point of that channel's line, how far along the channel it lies, m, modulo `ALONG_WRAP_M`, and the
    /// distance to the nearest channel's line without its bends, m, signed by the side (60,000 where none reaches it).
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
        let heights = self.terrain.heights_on(&self.samples, &self.map, self.lat, win, depth);
        let k = 2f64.powi(SAMPLE_DEPTH as i32 - depth as i32);
        let near = channels::Channels::new(self.reaches(), &self.bends, &self.strips).field(channels::Window { x0: win[0], y0: win[1], wide, high, k });
        let twi = wetness::field(&self.samples, &self.hill, depth, [win[0], win[1]], [wide, high], &near.bank);
        // The valleys about the channels drawn into the ground out of the ponds (`channels::Field::drawn`); near a pond
        // (`shore::Shore::beside`) a valley only lowers the ground, and never under the pond's level where the ground is
        // not: its mouth meets the level of the pond it runs into. Then the shore drawn smooth (`shore`). Which of
        // these a texel takes is blended between the samples' centres (the share of its weight on the samples in a
        // pond, and on those beside one), so the ground has no step along a sample's edge. The pond whose liquid may
        // stand on a texel is the shore's (`shore::Shore::drawn`): a function of the place, the same in every tile.
        let (heights, ponds): (Vec<f64>, Vec<f32>) = heights
            .iter()
            .enumerate()
            .map(|(i, &h)| {
                let p = self.centre(depth, win[0] + (i % wide) as i64, win[1] + (i / wide) as i64);
                let by = self.shore.beside(p);
                let in_pond = self.shore.share(p, |k| self.samples.pond[k] != NONE);
                let valley = near.drawn(i, h);
                let met = if by.level.is_nan() { valley } else { valley + (valley.min(h).max(h.min(by.level)) - valley) * by.share };
                let mouth = [near.mouth[i][0], near.mouth[i][1] / self.map.length(SAMPLE_DEPTH)];
                let (drawn, wet) = self.shore.drawn(p, met + (h - met) * in_pond, &by, mouth);
                (drawn, if wet { (by.pond + 1) as f32 } else { 0.0 })
            })
            .unzip();
        let at = |p: usize| (clamp(x0 + (p % side) as i64, across) - win[0]) as usize + (clamp(y0 + (p / side) as i64, down) - win[1]) as usize * wide;
        let mut out = Vec::with_capacity(8 * side * side);
        for i in (0..side * side).map(at) {
            let hi = half(heights[i] as f32);
            out.extend([hi, half((heights[i] - unhalf(hi) as f64) as f32), half(twi[i] as f32), half(near.dist[i] as f32)]);
        }
        for i in (0..side * side).map(at) {
            let pond = ponds[i];
            let texel = [win[0] + (i % wide) as i64, win[1] + (i / wide) as i64];
            let along = near.along[i].rem_euclid(ALONG_WRAP_M);
            out.extend([half(channels::code(&self.samples, texel, k, near.sample[i]) as f32), half(pond), half(along as f32), half(near.calm[i] as f32)]);
        }
        out
    }

    /// The rock of each of the terrain's samples, row by row in four planes of four 16-bit floats a sample: the depth
    /// of its loose ground, m, the largest stone its channel moves, m, and its channel's links; the reflectance of its
    /// bed's make, of `colours` one a make; the texture of its loose ground; and its pores and the deposit on it
    /// (`rock::rock`).
    pub fn rock(&self, colours: &[[f64; 3]]) -> Vec<u16> {
        rock::rock(&self.reaches(), colours, &self.grain)
    }

    /// The ponds of the ground, each with its pass and its level (`Samples::ponds`).
    pub fn ponds(&self) -> &[Pond] {
        &self.samples.ponds
    }

    /// What a tile reads of the ground's samples (`rebuild::Read`).
    fn read(&self) -> rebuild::Read<'_> {
        rebuild::Read { s: &self.samples, hill: &self.hill, strips: &self.strips, at: &self.at }
    }

    /// The reaches of the ground's channels (`bends::Reaches`).
    fn reaches(&self) -> bends::Reaches<'_> {
        bends::Reaches { seed: self.seed, s: &self.samples, at: &self.at, law: self.law, metres: self.map.length(SAMPLE_DEPTH) }
    }

    /// The centre of the texel at column `x` and row `y` of leaves of `depth`, in samples from the map's north-west corner.
    fn centre(&self, depth: u8, x: i64, y: i64) -> [f64; 2] {
        let k = 2f64.powi(pcore::space::terrain::SAMPLE_DEPTH as i32 - depth as i32);
        [(x as f64 + 0.5) * k, (y as f64 + 0.5) * k]
    }
}
