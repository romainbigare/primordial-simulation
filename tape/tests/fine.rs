//! A window finer than its cells (`tape::fine`): its ground is the terrain, whatever the cells and the window, and a
//! split that shares out its cell by area changes nothing drawn of what it shares so; each cell's liquid fills the
//! terrain inside it from its lowest point, holding the cell's own, so shores lie inside cells; and the channels are
//! the terrain's, as wide as their flux makes them, the same whatever the cells.

use pcore::World;
use tape::{fine, Samples, LAYERS};
use worldgen::testkit;

/// What `fine` hands out for a window of `n` pixels: the ground, the liquid's surface, the channels' share and depth, the bytes of
/// the layers, the fields inside the cells (`INSIDE` planes of `n`: wetness, the cover of each slot), the number of
/// each pixel's cell, and of each cell what its slots build per kg (`MAKE` numbers).
struct Fine {
    ground: Vec<f32>,
    water: Vec<f32>,
    channel: Vec<f32>,
    deep: Vec<f32>,
    bytes: Vec<u8>,
    inside: Vec<f32>,
    cell: Vec<u32>,
    make: Vec<f32>,
}

/// The planes inside the cells: wetness and one for each slot of cover.
const INSIDE: usize = 1 + pcore::cover::SLOTS;
/// Of each cell of a window: the walls and the capture area per kg of each slot.
const MAKE: usize = 2 * pcore::cover::SLOTS;

/// `b` read as `fine` lays it out: a byte a pixel for each layer, then zero bytes up to a multiple of 4, then the
/// planes inside the cells, the number of each pixel's cell, the count of the cells and what their slots build, and
/// nothing more.
fn read(b: &[u8], n: usize) -> Fine {
    let f32s = |from: usize, k: usize| (0..k).map(|i| f32::from_le_bytes(b[from + 4 * i..from + 4 * i + 4].try_into().unwrap())).collect::<Vec<f32>>();
    let at = 4;
    let (planes, end) = (at + 16 * n, at + 16 * n + LAYERS.len() * n);
    let inside = end.next_multiple_of(4);
    let u32s = |from: usize, k: usize| (0..k).map(|i| u32::from_le_bytes(b[from + 4 * i..from + 4 * i + 4].try_into().unwrap())).collect::<Vec<u32>>();
    let numbers = inside + 4 * INSIDE * n;
    let cells = u32s(numbers + 4 * n, 1)[0] as usize;
    assert_eq!(b.len(), numbers + 4 * (n + 1 + MAKE * cells), "a byte a pixel for each layer, then the planes inside the cells and their cells");
    assert!(b[end..inside].iter().all(|&x| x == 0), "zero bytes up to the planes inside the cells");
    let (cell, make) = (u32s(numbers, n), f32s(numbers + 4 * (n + 1), MAKE * cells));
    assert!(cell.iter().all(|&c| (c as usize) < cells), "each pixel in a cell of the window");
    Fine { ground: f32s(at, n), water: f32s(at + 4 * n, n), channel: f32s(at + 8 * n, n), deep: f32s(at + 12 * n, n), bytes: b[planes..end].to_vec(), inside: f32s(inside, INSIDE * n), cell, make }
}

/// The window from column `at[0]` and row `at[1]`, 8 x 8 squares, of `w` at `depth`, and its pixels along a side.
fn window(w: &World, s: &mut Samples, at: [f64; 2], depth: u8) -> (Fine, usize) {
    let across = 8 << (depth - w.map.square_depth());
    (read(&fine(w, s, [at[0], at[1], 8.0, 8.0], depth, |_| true), across * across), across)
}

/// The ground drawn is the terrain per pixel whatever the cells and the window; a split leaves what it shares by area
/// drawn as it was, and each cell's continuous fields (ice, the ground's temperature) on the mean its own value.
#[test]
fn the_fine_ground_is_the_terrain_whatever_the_cells_and_the_window() {
    let mut w = testkit::small_world();
    let depth = 8;
    let (before, across) = window(&w, &mut Samples::default(), [4.0, 4.0], depth);
    let n = across * across;
    // The same pixels in a second window: pixel (x, y) of the first is pixel (x - 2 per, y - per) of the second.
    let (other, _) = window(&w, &mut Samples::default(), [6.0, 5.0], depth);
    let per = across / 8;
    let moved = (per..across).flat_map(|y| (2 * per..across).map(move |x| (y * across + x, (y - per) * across + x - 2 * per)));
    assert!(moved.into_iter().all(|(p, q)| before.ground[p] == other.ground[q]), "a pixel's ground is the same in two windows");
    // Cells of many sizes: the cell in the window's middle split down to 1/32 of a square (its neighbours split with it).
    for _ in 0..5 {
        let i = w.mesh.tree().cell_at(w.map.point_at(8.3, 8.3));
        w.split(w.map.leaves()[i]).expect("a split");
    }
    let got = window(&w, &mut Samples::default(), [4.0, 4.0], depth).0;
    assert!(got.ground == before.ground, "the ground drawn does not change where cells split");
    assert!(got.inside[..n].iter().all(|w| (0.0..=1.0).contains(w)) && got.inside[n..].iter().all(|&x| x >= 0.0), "a wetness 0 to 1 and cover of none or more");
    assert!(got.make.iter().all(|&x| x >= 0.0), "walls and capture area of none or more");
    let at = |p: usize| [((p % across) as f64 + 0.5) / per as f64, ((p / across) as f64 + 0.5) / per as f64];
    let cell = |p: usize| w.mesh.tree().cell_at(w.map.point_at(4.0 + at(p)[0], 4.0 + at(p)[1]));
    let named: std::collections::BTreeSet<(usize, u32)> = (0..n).map(|p| (cell(p), got.cell[p])).collect();
    let (cells, numbers): (std::collections::BTreeSet<_>, std::collections::BTreeSet<_>) = named.iter().copied().unzip();
    assert!(named.len() == cells.len() && named.len() == numbers.len(), "a pixel's number names its cell");
    // The mean of each square's pixels against the terrain's mean over the square, the height of a cell as built.
    let lat = (w.cfg.lat_south, w.cfg.lat_north);
    let mut drawn = vec![(0.0, 0.0); 64];
    (0..n).for_each(|p| {
        let k = at(p)[1].floor() as usize * 8 + at(p)[0].floor() as usize;
        drawn[k] = (drawn[k].0 + got.ground[p] as f64, drawn[k].1 + 1.0);
    });
    let side = pcore::space::tree::BLOCK >> w.map.square_depth();
    let square = |k: usize| pcore::space::Leaf { x: (4 + k as u64 % 8) * side, y: (4 + k as u64 / 8) * side, depth: w.map.square_depth() };
    let off = (0..64).map(|k| (drawn[k].0 / drawn[k].1 - w.terrain.inside(w.seed, &w.map, square(k), lat).0).abs()).fold(0.0, f64::max);
    let relief = got.ground.iter().fold(f32::MIN, |a, &b| a.max(b)) - got.ground.iter().fold(f32::MAX, |a, &b| a.min(b));
    // The pixels sample a square (the midpoint rule), so their mean is the terrain's to a thousandth of the relief.
    assert!(off < 1e-3 * relief as f64, "a square's pixels hold the terrain's height on the mean, off by {off} m of a relief of {relief} m");
    // A split shares out its cell's rain by area, and here its look as it was: what is drawn of them does not change.
    // (The liquid in its pores goes by the ground inside it, laws.md 0, "Inside a cell".)
    for name in ["rain", "true_r", "true_g", "true_b"] {
        let k = LAYERS.iter().position(|l| l.name == name).expect("a layer");
        assert!(before.bytes[k * n..(k + 1) * n] == got.bytes[k * n..(k + 1) * n], "the {name} drawn does not change where cells split");
    }
    // The ice and the ground's temperature are continuous fields across the cells (`fine/keep.rs`), so beside a split
    // they may move; each cell's pixels hold its own value on the mean, to the byte they are drawn in.
    let f = tape::fields(&w);
    for (name, of) in [("ice", &f.ice), ("t_ground", &f.t)] {
        let (k, l) = LAYERS.iter().enumerate().find(|e| e.1.name == name).expect("a layer");
        for (&c, &i) in named.iter().map(|e| (e.1 as usize, e.0)).collect::<std::collections::BTreeMap<usize, usize>>().iter() {
            let scale = (l.scale, l.min, l.max);
            let px: Vec<f64> = (0..n).filter(|&p| got.cell[p] as usize == c).map(|p| of_byte(scale, got.bytes[k * n + p])).collect();
            let (mean, v) = (px.iter().sum::<f64>() / px.len() as f64, of[i]);
            assert!((mean - v).abs() <= byte(scale, v), "cell {i}'s {name} drawn {mean} on the mean, held {v}");
        }
    }
}

/// The value byte `b` stands for in a layer of scale, least and most value `(scale, min, max)` (`tape::quantise`,
/// inverted).
fn of_byte((scale, min, max): (&str, f64, f64), b: u8) -> f64 {
    match scale {
        "log" if b == 0 => 0.0,
        "log" => (min.ln() + (b as f64 - 1.0) * (max.ln() - min.ln()) / 254.0).exp(),
        _ => min + b as f64 * (max - min) / 255.0,
    }
}

/// How far a value `v` of such a layer may lie from what its byte stands for: half a byte's step (a log layer's in
/// proportion to `v`), and below a log layer's least value, that value.
fn byte((scale, min, max): (&str, f64, f64), v: f64) -> f64 {
    match scale {
        "log" => v * ((0.5 * (max.ln() - min.ln()) / 254.0).exp() - 1.0) + min,
        _ => 0.5 * (max - min) / 255.0,
    }
}

/// The liquid drawn stands over the terrain the cells stand on, only within half a cell of the cells that hold liquid
/// standing on ground that lies under the surface of the pixel's own cell, at their levels evened over half a cell
/// (`standing`): over the window's cells it holds their own together, a body at one level is drawn flat, and shores lie
/// inside cells.
#[test]
fn the_liquid_drawn_stands_at_the_cells_levels_over_their_own_ground() {
    let w = testkit::small_world();
    // Pixels the leaves of the cells' own samples (`World::ground_in`), the window's cells all whole squares.
    let depth = w.map.square_depth() + pcore::inside::DEPTHS;
    let (got, across) = window(&w, &mut Samples::default(), [0.0, 0.0], depth);
    let per = across / 8;
    let cell = |p: usize| w.mesh.tree().cell_at(w.map.point_at(((p % across) as f64 + 0.5) / per as f64, ((p / across) as f64 + 0.5) / per as f64));
    let mut shown: Vec<usize> = (0..across * across).map(cell).collect();
    shown.sort_unstable();
    shown.dedup();
    assert!(shown.iter().all(|&i| w.map.leaves()[i].depth == w.map.square_depth()), "the window's cells are squares");
    let means = pcore::flow::means(&w, &shown).expect("Flow's state of the ground");
    let levels = pcore::flow::levels(&w, &shown).expect("Flow's state of the ground");
    // Of each cell: its ground, the depth drawn over it by area, and its pixels under liquid.
    let mut drawn: Vec<(pcore::inside::Ground, f64, usize)> = shown.iter().map(|&i| (w.ground_in(w.map.leaves()[i]), 0.0, 0)).collect();
    // The sea's level: the one most cells that hold liquid standing hold it at.
    let wet: Vec<f64> = levels.iter().zip(&means).filter(|(_, m)| m.stands > 0.0).map(|(l, _)| l[1]).collect();
    let alike = |x: f64| wet.iter().filter(|&&y| (y - x).abs() < 1e-6).count();
    let sea = wet.iter().copied().max_by_key(|&x| alike(x)).expect("cells that hold liquid standing");
    let (mut flat, mut of_sea) = (0, 0);
    for p in 0..across * across {
        let k = shown.binary_search(&cell(p)).expect("a cell shown");
        let (g, depth, under) = &mut drawn[k];
        let s = (p / across % per) * per + p % across % per;
        assert!((got.ground[p] as f64 - g.h[s]).abs() < 1e-3, "the ground drawn is the terrain the cell's liquid stands on: {} m against {} m", got.ground[p], g.h[s]);
        let near = |dx: i64, dy: i64| {
            let (x, y) = ((p % across) as i64 + dx * per as i64 / 2, (p / across) as i64 + dy * per as i64 / 2);
            let q = (y.clamp(0, across as i64 - 1) * across as i64 + x.clamp(0, across as i64 - 1)) as usize;
            let j = shown.binary_search(&cell(q)).expect("a cell shown");
            means[j].stands > 0.0 && levels[j][0] < levels[k][1]
        };
        let wet_near = (-1..=1).any(|dx| (-1..=1).any(|dy| near(dx, dy)));
        assert!(got.water[p].is_nan() || wet_near, "liquid drawn only within half a cell of a cell that holds it standing on ground under the surface of the pixel's own cell");
        *depth += (got.water[p] - got.ground[p]).max(0.0) as f64 * g.area[s];
        *under += (got.water[p] > got.ground[p]) as usize;
        if (levels[k][1] - sea).abs() < 1e-6 && !got.water[p].is_nan() {
            (flat, of_sea) = (flat + ((got.water[p] as f64 - sea).abs() < 1e-3) as usize, of_sea + 1);
        }
    }
    // Together the cells' liquid drawn holds theirs (the levels evened move a little of it across their borders).
    let area = |k: usize| w.mesh.cells().area()[shown[k]];
    let (held, own) = drawn.iter().enumerate().fold((0.0, 0.0), |(h, o), (k, d)| (h + d.1 * area(k), o + means[k].stands * area(k)));
    println!("liquid drawn {held:.4e} m3 against the cells' {own:.4e} m3; {flat} of the sea's {of_sea} pixels at its level {sea:.3} m");
    assert!((held - own).abs() <= HELD * own, "the liquid drawn holds the cells' own: {held} m3 against {own} m3");
    // The sea drawn flat at its level, but within half a cell of a cell that holds liquid at another level.
    assert!(2 * flat > of_sea, "the sea drawn flat at its level: {flat} of {of_sea} pixels");
    let all = per * per;
    assert!(drawn.iter().any(|d| d.2 == all) && drawn.iter().any(|d| d.2 > 0 && d.2 < all), "cells wholly under liquid, and shores inside cells");
}

/// How near the liquid the cells of a window hold together the liquid drawn over them is, relative: the levels evened
/// over half a cell move a little of each cell's liquid across its borders, and the window's edge cuts what it holds.
const HELD: f64 = 0.01;

/// The channels drawn are the terrain's (`space::network`), each sample's way carrying its cell's flux per m² of the
/// drained area that leaves the cell: a split, which shares its cell's flux out that way (`World::split`), draws the
/// same channels (to the f32 the flux is kept in). A channel covers a share of a pixel, all of one narrower than it,
/// and is as deep as its flux makes it (`World::channel`); where it covers the whole pixel, the share is 1.
#[test]
fn the_channels_drawn_are_the_terrains_whatever_the_cells() {
    let mut w = testkit::small_world();
    // A month, so its liquid runs off and its cells pass some on.
    pcore::step(&mut w);
    let depth = 9;
    let (before, _) = window(&w, &mut Samples::default(), [4.0, 4.0], depth);
    assert!(before.channel.iter().any(|&s| s > 0.0), "channels drawn");
    assert!(before.channel.iter().zip(&before.deep).all(|(&s, &d)| (0.0..=1.0).contains(&s) && (s > 0.0) == (d > 0.0)), "a share 0 to 1, and a depth where a channel is");
    for _ in 0..3 {
        let i = w.mesh.tree().cell_at(w.map.point_at(8.3, 8.3));
        w.split(w.map.leaves()[i]).expect("a split");
    }
    let after = window(&w, &mut Samples::default(), [4.0, 4.0], depth).0;
    let worst = before.channel.iter().zip(&after.channel).map(|(a, b)| (a - b).abs()).fold(0.0f32, f32::max);
    assert!(worst < 1e-4, "a split draws the same channels: shares differ by {worst}");
}

#[test]
fn the_fine_grounds_slope_has_no_kinks_at_the_leaves_edges() {
    let w = testkit::small_world();
    // A square at leaves of 1/256 of it, 16 to a sample of the terrain (`SAMPLE_DEPTH`), 4 depths of displacement below.
    let (depth, across) = (10, 256);
    let ground = read(&fine(&w, &mut Samples::default(), [7.0, 7.0, 1.0, 1.0], depth, |_| true), across * across).ground;
    // The kink of the slope at each pixel along each axis, summed by where it lies against the leaves of 8 pixels.
    let (mut edge, mut within) = ((0.0, 0), (0.0, 0));
    for y in 1..across - 1 {
        for x in 1..across - 1 {
            let at = |dx: usize, dy: usize| ground[(y + dy - 1) * across + x + dx - 1] as f64;
            for (kink, from) in [((at(2, 1) - 2.0 * at(1, 1) + at(0, 1)).abs(), x), ((at(1, 2) - 2.0 * at(1, 1) + at(1, 0)).abs(), y)] {
                // A kink at a leaf's edge shows at the pixels on both sides of it.
                let t = if from % 8 == 0 || from % 8 == 7 { &mut edge } else { &mut within };
                *t = (t.0 + kink, t.1 + 1);
            }
        }
    }
    let (edge, within) = (edge.0 / edge.1 as f64, within.0 / within.1 as f64);
    assert!(edge < 1.1 * within, "the slope bends no more at the edges of coarser leaves: {edge} m against {within} m");
}
