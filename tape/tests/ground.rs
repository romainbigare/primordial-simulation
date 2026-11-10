//! The ground's tiles (`tape::Ground`): a texel holds the mean height of its four children, and a place holds the same
//! numbers in every tile that holds it.

use pcore::space::terrain::{Terrain, ISLAND_LEVEL};
use pcore::Config;
use tape::{unhalf, Ground, Law, GUTTER, TILE};

/// Texels along a side of a tile with its gutter.
const SIDE: usize = TILE + 2 * GUTTER;

/// The ground of the island of seed 1, 16 x 16 squares, with the default laws.
fn ground() -> (Ground, std::ops::RangeInclusive<u8>) {
    let cfg = Config::default();
    let map = worldgen::terrain::island(16, 16);
    let depths = tape::depths(&map);
    (Ground::new(1, map, Terrain::new(1, ISLAND_LEVEL), (cfg.lat_south, cfg.lat_north), Law::of(&cfg.phys)), depths)
}

/// The numbers of a texel of a tile's first plane.
const TEXEL: usize = 4;

/// The texel at column `x` and row `y` of tile `t`, counted from its own corner (inside the gutter): its height, m (the
/// sum of its first two numbers), and its other numbers.
fn at(t: &[u16], x: usize, y: usize, k: usize) -> f32 {
    let i = ((y + GUTTER) * SIDE + x + GUTTER) * TEXEL;
    if k == 0 { unhalf(t[i]) + unhalf(t[i + 1]) } else { unhalf(t[i + k]) }
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
                assert_eq!(a, b, "row {y}, column {i} past the edge, number {k}");
            }
            // The second plane: the channel's sample and the pond.
            let area = SIDE * SIDE * TEXEL;
            for b in 0..2 {
                assert_eq!(west[area + 2 * (y * SIDE + TILE + i) + b], east[area + 2 * (y * SIDE + i) + b], "row {y}, column {i} past the edge, the channel's sample and the pond");
            }
        }
    }
}

/// How finely a tile holds a height, m: the sum of two 16-bit floats.
const HEIGHT_M: f64 = 1e-3;

/// A tile names the pond each texel lies in (`Ground::pond_tile`): at the samples' own depth, where a texel is a
/// sample, every texel a tile names in a pond lies under the pond's level (to `HEIGHT_M`), in it or beside it, and the
/// island has ponds that a tile names exactly.
#[test]
fn a_texel_names_its_pond_and_lies_under_its_level() {
    let (g, depths) = ground();
    let depth = pcore::space::terrain::SAMPLE_DEPTH;
    assert!(depths.contains(&depth) && !g.ponds().is_empty(), "the island's ponds at the samples' depth");
    assert!(g.ponds().len() < tape::PONDS_EXACT, "{} ponds, more than a tile names", g.ponds().len());
    let tile = g.tile(depth, 0, 0);
    let mut held = 0;
    for y in 0..TILE {
        for x in 0..TILE {
            let named = tape::unhalf(tile[SIDE * SIDE * TEXEL + 2 * ((y + GUTTER) * SIDE + x + GUTTER) + 1]) as usize;
            if named > 0 {
                held += 1;
                let (h, level) = (at(&tile, x, y, 0) as f64, g.ponds()[named - 1].level);
                assert!(h < level + HEIGHT_M, "texel ({x}, {y}) at {h} m in a pond of level {level} m");
            }
        }
    }
    assert!(held > 0, "no texel in a pond");
}
