//! The ground's tiles (`tape::Ground`): a texel holds the mean height of its four children, and a place holds the same
//! numbers in every tile that holds it.

use pcore::Config;
use tape::{unhalf, Ground, Law, GUTTER, TILE};

/// Texels along a side of a tile with its gutter.
const SIDE: usize = TILE + 2 * GUTTER;

/// The seed and the side in squares of the island the tiles are made of: the north-west quarter of its tiles, which
/// `a_texel_is_the_mean_of_its_four_children` checks, holds land, and it has ponds.
const SEED: u64 = 3;
const SIDE_SQUARES: usize = 24;

/// The ground of the island of `SEED`, `SIDE_SQUARES` a side, with the default laws.
fn ground() -> (Ground, std::ops::RangeInclusive<u8>) {
    let cfg = Config::default();
    let (map, terrain) = worldgen::terrain::island_ground(SEED, (SIDE_SQUARES, SIDE_SQUARES), pcore::space::terrain::Shape::One);
    let depths = tape::depths(&map);
    (Ground::new(SEED, map, terrain, (cfg.lat_south, cfg.lat_north), Law::of(&cfg.phys)), depths)
}

/// The samples about a pond in which its shore is drawn smooth (`tape/src/ground/shore.rs`: the nearest pond within 2,
/// and the spline through the samples reaches 2 more).
const SHORE_SAMPLES: i64 = 4;

/// The numbers of a texel of a tile's first plane, and of its second.
const TEXEL: usize = 4;
const SECOND: usize = 4;

/// The texel at column `x` and row `y` of tile `t`, counted from its own corner (inside the gutter): its height, m (the
/// sum of its first two numbers), and its other numbers.
fn at(t: &[u16], x: usize, y: usize, k: usize) -> f32 {
    let i = ((y + GUTTER) * SIDE + x + GUTTER) * TEXEL;
    if k == 0 { unhalf(t[i]) + unhalf(t[i + 1]) } else { unhalf(t[i + k]) }
}

/// At a depth coarser than the terrain's samples, each texel's height is the mean of its four children's, so a parent
/// drawn in a child's place shows the same ground, softer; at one finer, so too where no valley about a channel is
/// drawn (`tape::VALLEY_FROM`, `tape::VALLEY_M`: within that of a sample that drains as much, and two samples more for
/// the line's bends and smoothing, the ground is drawn toward the channel's floor) and no shore (within `SHORE_SAMPLES`
/// of a pond the shore is drawn smooth, `tape/src/ground/shore.rs`).
#[test]
fn a_texel_is_the_mean_of_its_four_children() {
    let (g, depths) = ground();
    let cfg = Config::default();
    let (map, terrain) = worldgen::terrain::island_ground(SEED, (SIDE_SQUARES, SIDE_SQUARES), pcore::space::terrain::Shape::One);
    let s = terrain.samples(SEED, &map, (cfg.lat_south, cfg.lat_north));
    let metres = map.length(pcore::space::terrain::SAMPLE_DEPTH);
    let reach = (tape::VALLEY_M / metres).ceil() as i64 + 2;
    let near = |p: [f64; 2], reach: i64, hit: &dyn Fn(usize) -> bool| {
        let (r, c) = (p[1].floor() as i64, p[0].floor() as i64);
        (-reach..=reach).any(|dr| (-reach..=reach).any(|dc| {
            let (r, c) = (r + dr, c + dc);
            (0..s.down as i64).contains(&r) && (0..s.across as i64).contains(&c) && hit(r as usize * s.across + c as usize)
        }))
    };
    let near_valley = |p: [f64; 2]| near(p, reach, &|k| s.up[k] >= tape::VALLEY_FROM);
    let near_shore = |p: [f64; 2]| near(p, SHORE_SAMPLES, &|k| s.pond[k] != pcore::space::network::NONE);
    let (mut checked, mut worst) = (0, 0.0f64);
    // The whole island, and a tile of its middle (the parent's north-west quarter, the child's tile).
    for (depth, t) in [(*depths.start(), 0), (depths.start() + 4, 8)] {
        let (parent, child) = (g.tile(depth, t, t), g.tile(depth + 1, 2 * t, 2 * t));
        let per = 2f64.powi(depth as i32 - pcore::space::terrain::SAMPLE_DEPTH as i32);
        for y in 0..TILE / 2 {
            for x in 0..TILE / 2 {
                let centre = [((t * TILE as i64) as f64 + x as f64 + 0.5) / per, ((t * TILE as i64) as f64 + y as f64 + 0.5) / per];
                if (2.0 * per > 1.0 && near_valley(centre)) || near_shore(centre) {
                    continue;
                }
                let mean = (0..4).map(|q| at(&child, 2 * x + q % 2, 2 * y + q / 2, 0) as f64).sum::<f64>() / 4.0;
                worst = worst.max((at(&parent, x, y, 0) as f64 - mean).abs());
                checked += 1;
            }
        }
        assert!(worst < 1e-3, "depth {depth}: a texel departs from its children's mean by {worst} m");
    }
    assert!(checked > TILE * TILE / 8, "{checked} texels checked");
}

/// The texels two tiles side by side both hold (each one's gutter and the other's edge) hold the same height, wetness
/// index and channel, with the place along it and its curvature: no number depends on the tile it is made in.
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
            // The second plane: the channel's sample, the pond, the place along the channel and its curvature.
            let area = SIDE * SIDE * TEXEL;
            for b in 0..SECOND {
                assert_eq!(west[area + SECOND * (y * SIDE + TILE + i) + b], east[area + SECOND * (y * SIDE + i) + b], "row {y}, column {i} past the edge, number {b} of the second plane");
            }
        }
    }
}

/// How finely a tile holds a height, m: the sum of two 16-bit floats.
const HEIGHT_M: f64 = 1e-3;

/// The level the open low ground's liquid is drawn at, m (`tape/src/ground/shore.rs` `SEA_M`).
const SEA_M: f64 = 0.0;

/// The pond a texel is named in (whose liquid may stand on it), one more than it (0: none), and its height, m: of the
/// texel at column `x` and row `y` of tile `t`, counted from its own corner.
fn named(t: &[u16], x: usize, y: usize) -> (usize, f64) {
    (unhalf(t[SIDE * SIDE * TEXEL + SECOND * ((y + GUTTER) * SIDE + x + GUTTER) + 1]) as usize, at(t, x, y, 0) as f64)
}

/// A texel is named in a pond by the place alone (`tape/src/ground/shore.rs`), so every tile agrees on where a lake
/// is: where a texel finer than the samples named in a pond meets one named in none, that one lies over its level (to
/// `HEIGHT_M`), so no tile cuts liquid off along its texels; and the finer tile names each place that lies under a
/// pond's level at the samples' depth in that pond. The island has ponds a tile names exactly.
#[test]
fn every_tile_agrees_on_where_a_pond_is() {
    let (g, depths) = ground();
    let depth = pcore::space::terrain::SAMPLE_DEPTH;
    assert!(depths.contains(&(depth + 1)) && !g.ponds().is_empty(), "the island's ponds at the samples' depth and one finer");
    assert!(g.ponds().len() < tape::PONDS_EXACT, "{} ponds, more than a tile names", g.ponds().len());
    let level = |q: usize| { let p = &g.ponds()[q - 1]; if p.pass == pcore::space::network::NONE { SEA_M } else { p.level } };
    let (coarse, fine) = (g.tile(depth, 0, 0), [[0, 0], [1, 0], [0, 1], [1, 1]].map(|[x, y]| g.tile(depth + 1, x, y)));
    let fine_at = |x: usize, y: usize| named(&fine[(y / TILE) * 2 + x / TILE], x % TILE, y % TILE);
    let (mut edges, mut under) = (0, 0);
    let n = 2 * TILE;
    for y in 0..n {
        for x in 0..n {
            let (q, h) = fine_at(x, y);
            for (q2, h2) in [(x + 1 < n).then(|| fine_at(x + 1, y)), (y + 1 < n).then(|| fine_at(x, y + 1))].into_iter().flatten() {
                for (named_in, beyond) in [(q, h2), (q2, h)].into_iter().filter(|_| q != q2 && q * q2 == 0) {
                    if named_in > 0 {
                        assert!(beyond > level(named_in) - HEIGHT_M, "past an edge of pond {named_in} texel ({x}, {y}) at {beyond} m, under its level {} m", level(named_in));
                        edges += 1;
                    }
                }
            }
        }
    }
    for y in 0..TILE {
        for x in 0..TILE {
            let (q, h) = named(&coarse, x, y);
            if q > 0 && h < level(q) - HEIGHT_M {
                under += 1;
                for (fx, fy) in [(2 * x, 2 * y), (2 * x + 1, 2 * y), (2 * x, 2 * y + 1), (2 * x + 1, 2 * y + 1)] {
                    assert_eq!(fine_at(fx, fy).0, q, "the place of texel ({x}, {y}) under pond {q} in the coarser tile");
                }
            }
        }
    }
    assert!(edges > 100 && under > 100, "{edges} edges of named texels, {under} texels under a pond");
}
