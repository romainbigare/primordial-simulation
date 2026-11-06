//! A window finer than its cells (`tape::fine`): its ground keeps each cell's height as its mean and has no steps at
//! the cells' edges, it does not depend on the window, liquid stands on the low ground of wet cells, and rivers run
//! down.

use pcore::World;
use tape::{fine, Samples, LAYERS};
use worldgen::testkit;

/// What `fine` hands out for a window of `n` pixels.
struct Fine {
    ways: Vec<[f32; 6]>,
    ground: Vec<f32>,
    water: Vec<f32>,
    layers: usize,
}

fn read(b: &[u8], n: usize) -> Fine {
    let f32s = |from: usize, k: usize| (0..k).map(|i| f32::from_le_bytes(b[from + 4 * i..from + 4 * i + 4].try_into().unwrap())).collect::<Vec<f32>>();
    let rivers = u32::from_le_bytes(b[4..8].try_into().unwrap()) as usize;
    let ways = f32s(8, 6 * rivers).chunks(6).map(|c| c.try_into().unwrap()).collect();
    let at = 8 + 24 * rivers;
    Fine { ways, ground: f32s(at, n), water: f32s(at + 4 * n, n), layers: b.len() - at - 8 * n }
}

/// The window from column `at[0]` and row `at[1]`, 8 x 8 squares, of `w` at `depth`, and its pixels along a side.
fn window(w: &World, s: &mut Samples, at: [f64; 2], depth: u8) -> (Fine, usize) {
    let across = 8 << (depth - w.map.square_depth());
    (read(&fine(w, s, [at[0], at[1], 8.0, 8.0], depth), across * across), across)
}

#[test]
fn the_fine_ground_keeps_each_cells_height_and_has_no_steps() {
    let mut w = testkit::small_world();
    // Cells of many sizes: the cell in the window's middle split down to 1/32 of a square (its neighbours split with it).
    for _ in 0..5 {
        let i = w.mesh.tree().cell_at(w.map.point_at(8.3, 8.3));
        w.split(w.map.leaves()[i]).expect("a split");
    }
    let mut s = Samples::new(&w);
    let f = tape::fields(&w);
    for depth in [6, 8] {
        let (got, across) = window(&w, &mut s, [4.0, 4.0], depth);
        assert_eq!(got.layers, across * across * LAYERS.len(), "a byte a pixel for each layer");
        let per = (across / 8) as f64;
        let cell = |p: usize| w.mesh.tree().cell_at(w.map.point_at(4.0 + ((p % across) as f64 + 0.5) / per, 4.0 + ((p / across) as f64 + 0.5) / per));
        // The mean of each cell's pixels against its height.
        let mut sums = vec![(0.0, 0usize); w.mesh.cells().n()];
        (0..across * across).for_each(|p| sums[cell(p)] = (sums[cell(p)].0 + got.ground[p] as f64, sums[cell(p)].1 + 1));
        // The cells inside the window that hold 64 pixels or more (a pixel shows the cell that holds its centre).
        let inside = |i: usize| w.map.bounds(i).iter().all(|&b| (4.0..=12.0).contains(&b)) && sums[i].1 >= 64;
        let off = (0..w.mesh.cells().n()).filter(|&i| inside(i)).map(|i| (sums[i].0 / sums[i].1 as f64 - f.z[i]).abs()).fold(0.0, f64::max);
        // The steps between pixels next to each other across a cell's edge and inside a cell.
        let (mut edge, mut within) = ((0.0, 0), (0.0, 0));
        for p in 0..across * across - across {
            for q in [p + 1, p + across].into_iter().filter(|&q| q % across != 0 || q == p + across) {
                let step = (got.ground[q] - got.ground[p]).abs() as f64;
                let t = if cell(p) == cell(q) { &mut within } else { &mut edge };
                *t = (t.0 + step, t.1 + 1);
            }
        }
        let (edge, within) = (edge.0 / edge.1 as f64, within.0 / within.1 as f64);
        let relief = got.ground.iter().fold(f32::MIN, |a, &b| a.max(b)) - got.ground.iter().fold(f32::MAX, |a, &b| a.min(b));
        // The pixels sample a cell (the midpoint rule), so their mean is the blend's to a thousandth of the relief.
        assert!(off < 1e-3 * relief as f64, "depth {depth}: a cell's pixels hold its height on the mean, off by {off} m of a relief of {relief} m");
        assert!(edge < 1.5 * within, "depth {depth}: no step at a cell's edge: {edge} m against {within} m inside a cell");
    }
}

#[test]
fn the_fine_ground_does_not_depend_on_the_window_and_liquid_stands_low() {
    let w = testkit::small_world();
    let mut s = Samples::new(&w);
    let depth = 8;
    let ((a, across), (b, _)) = (window(&w, &mut s, [4.0, 4.0], depth), window(&w, &mut Samples::new(&w), [6.0, 5.0], depth));
    let per = across / 8;
    // Pixel (x, y) of the first window is pixel (x - 2 per, y - per) of the second.
    let mut most = 0.0f32;
    for y in per..across {
        for x in 2 * per..across {
            most = most.max((a.ground[y * across + x] - b.ground[(y - per) * across + x - 2 * per]).abs());
        }
    }
    assert_eq!(most, 0.0, "a pixel's ground is the same in two windows");
    let f = tape::fields(&w);
    let (got, across) = window(&w, &mut s, [0.0, 0.0], 6);
    let per = (across / 8) as f64;
    let cell = |p: usize| w.mesh.tree().cell_at(w.map.point_at(((p % across) as f64 + 0.5) / per, ((p / across) as f64 + 0.5) / per));
    let deep = (0..across * across).filter(|&p| f.depth[cell(p)] > 50.0).collect::<Vec<_>>();
    let covered = deep.iter().filter(|&&p| got.water[p] > got.ground[p]).count() as f64 / deep.len() as f64;
    assert!(!deep.is_empty() && covered > 0.9, "the sea covers its deep cells but for some of their shores: {covered}");
    for r in &got.ways {
        let head = |x: f32, y: f32| {
            let i = w.mesh.tree().cell_at(w.map.point_at(x as f64, y as f64));
            f.z[i] + f.depth[i]
        };
        assert!(head(r[0], r[1]) > head(r[2], r[3]), "a river runs down: {r:?}");
    }
}
