//! The ground's tiles (`tape::Ground`): a texel holds the mean height of its four children, and a place holds the same
//! numbers in every tile that holds it.

use pcore::space::terrain::{Terrain, ISLAND_LEVEL};
use pcore::Config;
use tape::{Ground, GUTTER, TEXEL, TILE};

/// Texels along a side of a tile with its gutter.
const SIDE: usize = TILE + 2 * GUTTER;

/// The ground of the island of seed 1, 16 x 16 squares, with the default laws.
fn ground() -> (Ground, std::ops::RangeInclusive<u8>) {
    let cfg = Config::default();
    let map = worldgen::terrain::island(16, 16);
    let depths = tape::depths(&map);
    (Ground::new(1, map, Terrain::new(1, ISLAND_LEVEL), (cfg.lat_south, cfg.lat_north), cfg.phys.channel_width_exp), depths)
}

/// Number `k` of the texel at column `x` and row `y` of a tile, counted from its own corner (inside the gutter).
fn at(t: &[f32], x: usize, y: usize, k: usize) -> f32 {
    t[((y + GUTTER) * SIDE + x + GUTTER) * TEXEL + k]
}

/// At a depth coarser than the terrain's samples and at one finer, each texel's height is the mean of its four
/// children's, so a parent drawn in a child's place shows the same ground, softer.
#[test]
fn a_texel_is_the_mean_of_its_four_children() {
    let (g, depths) = ground();
    for depth in [*depths.start(), depths.start() + 4] {
        let (parent, child) = (g.tile(depth, 0, 0), g.tile(depth + 1, 0, 0));
        let mut worst = 0.0f64;
        for y in 0..TILE / 2 {
            for x in 0..TILE / 2 {
                let mean = (0..4).map(|q| at(&child, 2 * x + q % 2, 2 * y + q / 2, 0) as f64).sum::<f64>() / 4.0;
                worst = worst.max((at(&parent, x, y, 0) as f64 - mean).abs());
            }
        }
        assert!(worst < 1e-3, "depth {depth}: a texel departs from its children's mean by {worst} m");
    }
}

/// The texels two tiles side by side both hold (each one's gutter and the other's edge) hold the same height, wetness
/// index and channel: no number depends on the tile it is made in.
#[test]
fn a_place_holds_the_same_numbers_in_every_tile() {
    let (g, depths) = ground();
    let depth = depths.start() + 5;
    let (west, east) = (g.tile(depth, 0, 0), g.tile(depth, 1, 0));
    for y in 0..SIDE {
        for i in 0..2 * GUTTER {
            for k in 0..TEXEL {
                let (a, b) = (west[(y * SIDE + TILE + i) * TEXEL + k], east[(y * SIDE + i) * TEXEL + k]);
                assert!((a - b).abs() <= 1e-6 * a.abs().max(1.0), "row {y}, column {i} past the edge, number {k}: {a} against {b}");
            }
        }
    }
}

/// A tile names the pond each texel lies in (`Ground::pond_tile`): at the samples' own depth, where a texel is a
/// sample, every texel of a pond lies under the pond's level, and the island has ponds.
#[test]
fn a_texel_names_its_pond_and_lies_under_its_level() {
    let (g, depths) = ground();
    let depth = pcore::space::terrain::SAMPLE_DEPTH;
    assert!(depths.contains(&depth) && !g.ponds().is_empty(), "the island's ponds at the samples' depth");
    let (tile, ponds) = (g.tile(depth, 0, 0), g.pond_tile(depth, 0, 0));
    let mut held = 0;
    for y in 0..TILE {
        for x in 0..TILE {
            let p = ponds[(y + GUTTER) * SIDE + x + GUTTER];
            if p != pcore::space::network::NONE {
                held += 1;
                let (h, level) = (at(&tile, x, y, 0) as f64, g.ponds()[p as usize].level);
                assert!(h < level, "texel ({x}, {y}) at {h} m in a pond of level {level} m");
            }
        }
    }
    assert!(held > 0, "no texel in a pond");
}
