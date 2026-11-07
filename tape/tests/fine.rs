//! A window finer than its cells (`tape::fine`): its ground keeps each square's height as its mean and has no steps at
//! the cells' edges, a split that shares out its cell by area changes nothing drawn of what it shares so, it does not
//! depend on the window, liquid stands at its own cell's level on the low ground of wet cells, and rivers run down.

use pcore::World;
use tape::{fine, Samples, LAYERS};
use worldgen::testkit;

/// What `fine` hands out for a window of `n` pixels: the river ways, the ground and the liquid's surface, the bytes of
/// the layers, the fields inside the cells (`INSIDE` planes of `n`: wetness, the cover of each slot), the number of
/// each pixel's cell, and of each cell what its slots build per kg (`MAKE` numbers).
struct Fine {
    ways: Vec<[f32; 6]>,
    ground: Vec<f32>,
    water: Vec<f32>,
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
    let rivers = u32::from_le_bytes(b[4..8].try_into().unwrap()) as usize;
    let ways = f32s(8, 6 * rivers).chunks(6).map(|c| c.try_into().unwrap()).collect();
    let at = 8 + 24 * rivers;
    let (planes, end) = (at + 8 * n, at + 8 * n + LAYERS.len() * n);
    let inside = end.next_multiple_of(4);
    let u32s = |from: usize, k: usize| (0..k).map(|i| u32::from_le_bytes(b[from + 4 * i..from + 4 * i + 4].try_into().unwrap())).collect::<Vec<u32>>();
    let numbers = inside + 4 * INSIDE * n;
    let cells = u32s(numbers + 4 * n, 1)[0] as usize;
    assert_eq!(b.len(), numbers + 4 * (n + 1 + MAKE * cells), "a byte a pixel for each layer, then the planes inside the cells and their cells");
    assert!(b[end..inside].iter().all(|&x| x == 0), "zero bytes up to the planes inside the cells");
    let (cell, make) = (u32s(numbers, n), f32s(numbers + 4 * (n + 1), MAKE * cells));
    assert!(cell.iter().all(|&c| (c as usize) < cells), "each pixel in a cell of the window");
    Fine { ways, ground: f32s(at, n), water: f32s(at + 4 * n, n), bytes: b[planes..end].to_vec(), inside: f32s(inside, INSIDE * n), cell, make }
}

/// The window from column `at[0]` and row `at[1]`, 8 x 8 squares, of `w` at `depth`, and its pixels along a side.
fn window(w: &World, s: &mut Samples, at: [f64; 2], depth: u8) -> (Fine, usize) {
    let across = 8 << (depth - w.map.square_depth());
    (read(&fine(w, s, [at[0], at[1], 8.0, 8.0], depth), across * across), across)
}

/// Each pair of pixels next to each other in a window `across` pixels a side, and whether they lie in one cell
/// (`cell`: the cell a pixel shows).
fn pairs(across: usize, cell: impl Fn(usize) -> usize) -> Vec<(usize, usize, bool)> {
    (0..across * across - across)
        .flat_map(|p| [p + 1, p + across].into_iter().filter(move |&q| q % across != 0 || q == p + across).map(move |q| (p, q)))
        .map(|(p, q)| (p, q, cell(p) == cell(q)))
        .collect()
}

#[test]
fn the_fine_ground_keeps_each_squares_height_and_shows_no_cells_edge() {
    let mut w = testkit::small_world();
    let depth = 8;
    let before = window(&w, &mut Samples::new(&w), [4.0, 4.0], depth).0;
    // Cells of many sizes: the cell in the window's middle split down to 1/32 of a square (its neighbours split with it).
    for _ in 0..5 {
        let i = w.mesh.tree().cell_at(w.map.point_at(8.3, 8.3));
        w.split(w.map.leaves()[i]).expect("a split");
    }
    let mut s = Samples::new(&w);
    let f = tape::fields(&w);
    let (got, across) = window(&w, &mut s, [4.0, 4.0], depth);
    let n = across * across;
    assert!(got.inside[..n].iter().all(|w| (0.0..=1.0).contains(w)) && got.inside[n..].iter().all(|&x| x >= 0.0), "a wetness 0 to 1 and cover of none or more");
    assert!(got.make.iter().all(|&x| x >= 0.0), "walls and capture area of none or more");
    assert!(w.map.leaves().iter().all(|l| l.depth >= w.map.square_depth()), "the coarsest cells are squares");
    let per = (across / 8) as f64;
    let at = |p: usize| [((p % across) as f64 + 0.5) / per, ((p / across) as f64 + 0.5) / per];
    let cell = |p: usize| w.mesh.tree().cell_at(w.map.point_at(4.0 + at(p)[0], 4.0 + at(p)[1]));
    let named: std::collections::BTreeSet<(usize, u32)> = (0..n).map(|p| (cell(p), got.cell[p])).collect();
    let (cells, numbers): (std::collections::BTreeSet<_>, std::collections::BTreeSet<_>) = named.iter().copied().unzip();
    assert!(named.len() == cells.len() && named.len() == numbers.len(), "a pixel's number names its cell");
    // The mean of each square's pixels against the mean height by area of its cells.
    let square = |[x, y]: [f64; 2]| y.floor() as usize * 8 + x.floor() as usize;
    let (mut drawn, mut held) = (vec![(0.0, 0.0); 64], vec![(0.0, 0.0); 64]);
    (0..across * across).for_each(|p| drawn[square(at(p))] = (drawn[square(at(p))].0 + got.ground[p] as f64, drawn[square(at(p))].1 + 1.0));
    for i in 0..w.mesh.cells().n() {
        let ([x0, y0, ..], a) = (w.map.bounds(i), w.mesh.cells().area()[i]);
        if (4.0..12.0).contains(&x0) && (4.0..12.0).contains(&y0) {
            let k = square([x0 - 4.0, y0 - 4.0]);
            held[k] = (held[k].0 + a * f.z[i], held[k].1 + a);
        }
    }
    let off = drawn.iter().zip(&held).map(|(d, h)| (d.0 / d.1 - h.0 / h.1).abs()).fold(0.0, f64::max);
    let relief = got.ground.iter().fold(f32::MIN, |a, &b| a.max(b)) - got.ground.iter().fold(f32::MAX, |a, &b| a.min(b));
    // The pixels sample a square (the midpoint rule), so their mean is the field's to a thousandth of the relief.
    assert!(off < 1e-3 * relief as f64, "a square's pixels hold its height on the mean, off by {off} m of a relief of {relief} m");
    let pairs = pairs(across, cell);
    // The mean step of the ground between pixels across a cell's edge and inside a cell.
    let step = |edge: bool| {
        let s: Vec<f64> = pairs.iter().filter(|t| t.2 != edge).map(|&(p, q, _)| (got.ground[q] - got.ground[p]).abs() as f64).collect();
        s.iter().sum::<f64>() / s.len() as f64
    };
    assert!(step(true) < 1.5 * step(false), "no step of the ground at a cell's edge: {} m against {} m inside a cell", step(true), step(false));
    // A split shares out its cell's rain by area, and here its ice and look as they were: what is drawn of them does not
    // change. (The liquid in its pores goes by the ground inside it, laws.md 0, "Inside a cell".)
    for name in ["ice", "rain", "true_r", "true_g", "true_b"] {
        let k = LAYERS.iter().position(|l| l.name == name).expect("a layer");
        assert!(before.bytes[k * n..(k + 1) * n] == got.bytes[k * n..(k + 1) * n], "the {name} drawn does not change where cells split");
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
    // The liquid drawn stands at its own cell's level (the shore moves with it, never past the cell's own ground),
    // only in a cell that holds it standing, and a cell wholly under it holds its depth over the first `STANDING_M` on
    // the mean (f32).
    let mut drawn = std::collections::BTreeMap::new();
    for p in 0..across * across {
        let (i, level) = (cell(p), got.water[p]);
        assert!(level.is_nan() || f.depth[i] > tape::STANDING_M, "liquid drawn only in a cell that holds it standing");
        let d = drawn.entry(i).or_insert((level, 0.0, 0.0, true));
        assert!(d.0.to_bits() == level.to_bits(), "the liquid of a cell stands at one level");
        *d = (level, d.1 + (level - got.ground[p]) as f64, d.2 + 1.0, d.3 && level > got.ground[p]);
    }
    let under: Vec<_> = drawn.iter().filter(|d| d.1 .3).collect();
    assert!(!under.is_empty() && under.iter().all(|(&i, d)| (d.1 / d.2 - (f.depth[i] - tape::STANDING_M)).abs() < 1e-5 * f.depth[i]), "a cell under its liquid holds its depth");
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

#[test]
fn the_fine_grounds_slope_has_no_kinks_at_the_leaves_edges() {
    let w = testkit::small_world();
    // A square at leaves of 1/256 of it, 16 to a sample of the terrain (`SAMPLE_DEPTH`), 4 depths of displacement below.
    let (depth, across) = (10, 256);
    let ground = read(&fine(&w, &mut Samples::new(&w), [7.0, 7.0, 1.0, 1.0], depth), across * across).ground;
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
