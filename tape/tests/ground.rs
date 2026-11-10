//! The ground's tiles (`tape::Ground`): a texel holds the mean height of its four children, and a place holds the same
//! numbers in every tile that holds it.

use pcore::space::terrain::{Terrain, ISLAND_LEVEL};
use pcore::Config;
use tape::{unhalf, Ground, GUTTER, TILE};

/// Texels along a side of a tile with its gutter.
const SIDE: usize = TILE + 2 * GUTTER;

/// The ground of the island of seed 1, 16 x 16 squares, with the default laws.
fn ground() -> (Ground, std::ops::RangeInclusive<u8>) {
    let cfg = Config::default();
    let map = worldgen::terrain::island(16, 16);
    let depths = tape::depths(&map);
    (Ground::new(1, map, Terrain::new(1, ISLAND_LEVEL), (cfg.lat_south, cfg.lat_north), [cfg.phys.channel_width, cfg.phys.channel_width_exp]), depths)
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
            // The second plane: the channel's area.
            let area = SIDE * SIDE * TEXEL;
            assert_eq!(west[area + y * SIDE + TILE + i], east[area + y * SIDE + i], "row {y}, column {i} past the edge, the area");
        }
    }
}
