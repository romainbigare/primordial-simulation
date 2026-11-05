//! Space: the mesh of a map has its cells (areas, links, the air as a coarser level), cells of many squares among
//! them, and stacks give each cell its column of nodes; what the world keeps per cell and per link follows a change of
//! the cells (`World::remap`).

use pcore::remap::{Remap, Renumber};
use pcore::store::Class;
use pcore::{World, AIR};
use pcore::space::square::Square;
use pcore::space::tree::BLOCK;
use pcore::space::{Leaf, Level, Mesh, Stack};
use std::collections::BTreeSet;

const SIDE: f64 = 2000.0;
const SCALE: usize = 4;

/// Two maps: one the air blocks fit, one they do not (10 x 7 leaves narrow blocks along the south and east edges).
fn maps() -> [(Square, Mesh); 2] {
    [(16, 16), (10, 7)].map(|(w, h)| {
        let sq = Square::new(w, h, SIDE, false, SCALE);
        let mesh = sq.mesh(31.0, 33.3);
        (sq, mesh)
    })
}

/// Each link once, its ends `a < b`; the adjacency lists exactly the links at each cell.
fn links_once(lv: &Level) {
    let pairs: BTreeSet<(u32, u32)> = lv.a().iter().zip(lv.b()).map(|(&a, &b)| (a, b)).collect();
    assert_eq!(pairs.len(), lv.a().len(), "every link appears once");
    assert!(pairs.iter().all(|&(a, b)| a < b));
    for i in 0..lv.n() {
        let (links, cells) = lv.around(i);
        assert!(links.windows(2).all(|l| l[0] < l[1]), "in link order");
        for (&l, &j) in links.iter().zip(cells) {
            let (a, b) = (lv.a()[l as usize] as usize, lv.b()[l as usize] as usize);
            assert!((a, b) == (i, j as usize) || (a, b) == (j as usize, i));
        }
    }
    assert_eq!(lv.start()[lv.n()] as usize, 2 * lv.a().len(), "each link is in the lists of its two cells");
}

#[test]
fn the_square_mesh_has_the_maps_cells_and_links() {
    for (sq, mesh) in maps() {
        let (w, h) = (sq.w as i64, sq.h as i64);
        for l in 0..mesh.levels() {
            let total: f64 = mesh.level(l).area().iter().sum();
            assert_eq!(total, (w * h) as f64 * SIDE * SIDE, "areas sum to the map at level {l}");
            links_once(mesh.level(l));
        }
        let lv = mesh.cells();
        assert_eq!(lv.a().len() as i64, 4 * w * h - 3 * w - 3 * h + 2, "8 neighbours, each link once");
        for l in 0..lv.a().len() {
            let ((xa, ya), (xb, yb)) = (sq.xy(lv.a()[l] as usize), sq.xy(lv.b()[l] as usize));
            let diagonal = xa != xb && ya != yb;
            assert!(xa.abs_diff(xb) <= 1 && ya.abs_diff(yb) <= 1);
            assert_eq!(lv.width()[l], if diagonal { 0.0 } else { SIDE }, "a border is a side wide, a corner nothing");
            assert_eq!(lv.dist()[l], if diagonal { SIDE * std::f64::consts::SQRT_2 } else { SIDE });
        }
        // The air level: the same 8 neighbours, across the borders of the blocks.
        let air = mesh.level(1);
        let (aw, ah) = (sq.w.div_ceil(SCALE) as i64, sq.h.div_ceil(SCALE) as i64);
        assert_eq!(air.a().len() as i64, 4 * aw * ah - 3 * aw - 3 * ah + 2);
        let border: f64 = air.width().iter().sum();
        let fine: f64 = lv.a().iter().zip(lv.b()).zip(lv.width()).filter(|((&a, &b), _)| sq.xy(a as usize).0 / SCALE != sq.xy(b as usize).0 / SCALE || sq.xy(a as usize).1 / SCALE != sq.xy(b as usize).1 / SCALE).map(|(_, &wd)| wd).sum();
        assert_eq!(border, fine, "a block's border is the borders of its squares");
    }
}

#[test]
fn air_of_is_the_parent_at_the_air_level_and_levels_agree() {
    for (sq, mesh) in maps() {
        let world = World::mesh_of(&sq, &pcore::Config::default());
        let parent = world.cells().parent();
        for (i, &p) in parent.iter().enumerate() {
            let (x, y) = sq.xy(i);
            assert_eq!(world.up(i, 0, AIR), (y / SCALE) * sq.w.div_ceil(SCALE) + x / SCALE, "the air block over the cell");
            assert_eq!(world.up(i, 0, AIR), p as usize);
            assert_eq!(p, mesh.cells().parent()[i]);
            assert_eq!(world.share(i, AIR), 1.0 / world.level(AIR).children(p as usize).len() as f64, "a square's share of its block");
        }
        assert_eq!(parent.len(), sq.n());
        let mut seen = vec![0; sq.n()];
        for j in 0..mesh.level(1).n() {
            for &c in mesh.level(1).children(j) {
                assert_eq!(parent[c as usize] as usize, j, "a child's parent is the cell that lists it");
                seen[c as usize] += 1;
            }
        }
        assert!(seen.iter().all(|&s| s == 1), "every cell is the child of one coarse cell");
        assert!(mesh.cells().children(0).is_empty() && mesh.level(1).parent().is_empty());
    }
}

/// On a map of squares, flat or a globe (whose cells near a pole are far narrower than tall), each cell's link in each
/// compass direction goes to its neighbour that way on the grid.
#[test]
fn the_compass_of_a_square_is_its_eight_neighbours() {
    const GRID: [(i32, i32); 8] = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)];
    let globe = Square::new(16, 8, SIDE, true, SCALE);
    let round = globe.mesh(-90.0, 90.0);
    for (sq, mesh) in maps().into_iter().chain([(globe, round)]) {
        let lv = mesh.cells();
        for (i, (d, &g)) in (0..sq.n()).flat_map(|i| GRID.iter().enumerate().map(move |dg| (i, dg))) {
            assert_eq!(lv.way(i, d).map(|e| lv.adj_cell()[e] as usize), sq.step(i, g), "cell {i}, direction {d}");
        }
    }
}

/// A stack numbers each layer's nodes after the layer below: a deep store and the ground on the squares, the air on its
/// coarser level.
#[test]
fn a_stack_numbers_each_layer_after_the_one_below() {
    let [_, (sq, mesh)] = maps();
    let st = Stack::new(&mesh, &[0, 0, 1]);
    let (n, na) = (sq.n(), mesh.level(1).n());
    assert_eq!(st.nodes(), 2 * n + na);
    for i in 0..n {
        let (deep, ground, air) = (st.node(0, i), st.node(1, i), st.node(2, mesh.up(i, 0, 1)));
        assert_eq!((deep, ground, air), (i, n + i, 2 * n + mesh.up(i, 0, 1)));
    }
}

/// A globe: its cells cover the whole sphere, narrower toward the poles; its rows wrap east to west at every level,
/// the way across the wrap the shorter one, and no link crosses a pole.
#[test]
fn a_globe_covers_its_sphere_and_wraps_east_to_west() {
    let (rows, radius) = (8, 1e6);
    let sq = Square::new(2 * rows, rows, std::f64::consts::PI * radius / rows as f64, true, SCALE);
    let mesh = sq.mesh(-90.0, 90.0);
    let sphere = 4.0 * std::f64::consts::PI * radius * radius;
    for l in 0..mesh.levels() {
        let total: f64 = mesh.level(l).area().iter().sum();
        assert!((total / sphere - 1.0).abs() < 1e-12, "level {l} covers the sphere: {total} of {sphere}");
        links_once(mesh.level(l));
    }
    let lv = mesh.cells();
    assert_eq!(lv.a().len(), 4 * sq.w * sq.h - 3 * sq.w, "8 neighbours round each row, none across a pole");
    assert!(lv.area()[sq.idx(0, 0)] < lv.area()[sq.idx(0, rows / 2)] / 4.0, "a cell at a pole is narrow");
    // The first and the last cell of a row, on the cells' level and on the air's.
    let air = |x: usize| mesh.up(sq.idx(x, SCALE), 0, 1);
    for (lv, (east, west), width) in [(lv, (sq.idx(0, 3), sq.idx(sq.w - 1, 3)), sq.side), (mesh.level(1), (air(0), air(sq.w - 1)), SCALE as f64 * sq.side)] {
        let l = lv.link(east, west).expect("a link across the wrap");
        assert!((lv.width()[l] / width - 1.0).abs() < 1e-12, "it crosses a whole border");
        let way = lv.offset(west, east);
        assert!(way[0] > 0.0 && way[1].abs() < 1e-6 && (way[0] / lv.dist()[l] - 1.0).abs() < 1e-9, "the next cell east lies a link away: {way:?}");
    }
}

/// Two maps of cells of 1, 2 and 4 squares on a side (depths 2, 1 and 0 in a block of `SCALE` x `SCALE` squares, by
/// block row by row), one flat and one a globe (wrapping a block of cells of 2 beside one of 1), with their latitudes.
fn mixed() -> [(Square, f64, f64); 2] {
    let (flat, globe): (&[[u8; 4]], &[[u8; 4]]) = (&[[1, 1, 1, 0], [1, 2, 2, 1], [1, 1, 1, 1]], &[[1, 0, 1, 2], [2, 1, 2, 2]]);
    let flat = Square::new(16, 12, SIDE, false, SCALE).by_blocks(|bx, by| flat[by][bx]);
    [(flat, 31.0, 33.3), (Square::new(16, 8, SIDE, true, SCALE).by_blocks(|bx, by| globe[by][bx]), -90.0, 90.0)]
}

/// The north-west square of cell `i` of `sq` (column, row) and its side, squares.
fn tile(sq: &Square, i: usize) -> ((usize, usize), usize) {
    let (leaf, unit) = (sq.leaves()[i], BLOCK / SCALE as u32);
    (((leaf.x / unit) as usize, (leaf.y / unit) as usize), (leaf.side() / unit) as usize)
}

/// Cells of many sizes, on a flat map and on a globe: numbered by their north-west squares row by row, each covers its
/// squares and lies under their air block, the air level is that of the squares; each cell keeps the borders of its
/// squares with the others, cells of two sizes link only across a border, as far apart as their centres across it,
/// and a corner links only cells of one size.
#[test]
fn cells_of_many_sizes_cover_their_squares_and_keep_the_borders() {
    for (sq, south, north) in mixed() {
        let (mesh, plain) = (sq.mesh(south, north), Square::new(sq.w, sq.h, sq.side, sq.wrap, SCALE).mesh(south, north));
        let lv = mesh.cells();
        links_once(lv);
        links_once(mesh.level(AIR));
        let corner = |i: usize| sq.idx(tile(&sq, i).0 .0, tile(&sq, i).0 .1);
        assert!((1..lv.n()).all(|i| corner(i - 1) < corner(i)), "numbered by their north-west squares");
        let cell_of = |s: usize| mesh.tree().cell_at(sq.point(s));
        let mut area = vec![0.0; lv.n()];
        for s in 0..sq.n() {
            area[cell_of(s)] += plain.cells().area()[s];
            assert_eq!(mesh.up(cell_of(s), 0, AIR), plain.up(s, 0, AIR), "square {s} under its air block");
        }
        assert!((0..lv.n()).all(|i| cell_of(corner(i)) == i));
        let near = |a: f64, b: f64| (a / b - 1.0).abs() < 1e-12;
        assert!((0..lv.n()).all(|i| near(area[i], lv.area()[i])), "the cells cover their squares");
        let (air, plain_air) = (mesh.level(AIR), plain.level(AIR));
        assert!(air.n() == plain_air.n() && (0..air.n()).all(|j| near(air.area()[j], plain_air.area()[j])), "the air level is that of the squares");
        assert!(air.width().iter().zip(plain_air.width()).all(|(&a, &b)| a == b || near(a, b)), "with the borders of the squares");
        let mut border = vec![0.0; lv.a().len()];
        for p in 0..plain.cells().a().len() {
            let (a, b) = (cell_of(plain.cells().a()[p] as usize), cell_of(plain.cells().b()[p] as usize));
            if a != b && plain.cells().width()[p] > 0.0 {
                border[lv.link(a, b).expect("cells across a border link")] += plain.cells().width()[p];
            }
        }
        assert!(border.iter().zip(lv.width()).all(|(&b, &w)| b == w || near(b, w)), "each cell keeps the borders of its squares");
        for l in 0..lv.a().len() {
            let (ka, kb) = (tile(&sq, lv.a()[l] as usize).1, tile(&sq, lv.b()[l] as usize).1);
            if lv.width()[l] == 0.0 {
                assert_eq!(ka, kb, "a corner links cells of one size");
            } else if !sq.wrap {
                assert!(near(lv.dist()[l], (ka + kb) as f64 / 2.0 * SIDE), "centres lie half their sides apart across a border");
            }
        }
    }
}

/// A tree of three depths in one air block (a fine north-west corner, its finest cells half a square on a side): its
/// cells tile the block, each found at its corner; cells across a border differ by one depth at most and link across
/// it as far apart as their centres across it; each cell's links are as wide as its sides inside the map, so each
/// border's width is the same seen from either side; a corner links only cells of one size.
#[test]
fn a_tree_of_three_depths_keeps_the_borders_and_the_area_of_its_block() {
    // The block in quarters, the north-west one in quarters, and the north-west one of those in quarters.
    let mut leaves = vec![Leaf { x: 0, y: 0, depth: 0 }];
    for _ in 0..3 {
        leaves = leaves.into_iter().flat_map(|l| l.split(l.depth + u8::from(l.x == 0 && l.y == 0))).collect();
    }
    let sq = Square::new(SCALE, SCALE, SIDE, false, SCALE).with(leaves);
    let mesh = sq.mesh(31.0, 33.3);
    let (lv, cells) = (mesh.cells(), sq.leaves());
    assert_eq!([1, 2, 3].map(|d| cells.iter().filter(|l| l.depth == d).count()), [3, 3, 4]);
    links_once(lv);
    let block = SCALE as f64 * SIDE;
    assert_eq!(lv.area().iter().sum::<f64>(), block * block, "the cells cover their block");
    assert_eq!(mesh.level(AIR).area(), [block * block]);
    assert!((0..lv.n()).all(|i| mesh.tree().cell_at(cells[i].corner()) == i && lv.around(i).0.len() <= 8));
    // The length of a cell's sides inside the map, m.
    let inside = |l: &Leaf| [l.x > 0, l.y > 0, l.x + l.side() < BLOCK, l.y + l.side() < BLOCK].iter().filter(|&&s| s).count() as f64 * l.side() as f64 / BLOCK as f64 * block;
    let mut borders = vec![0.0; lv.n()];
    for l in 0..lv.a().len() {
        let (a, b) = (lv.a()[l] as usize, lv.b()[l] as usize);
        assert!(cells[a].depth.abs_diff(cells[b].depth) <= 1, "cells across a border differ by one depth at most");
        let way = lv.offset(a, b);
        if lv.width()[l] == 0.0 {
            assert_eq!(cells[a].depth, cells[b].depth, "a corner links cells of one size");
            continue;
        }
        assert_eq!(lv.dist()[l], way[0].abs().max(way[1].abs()), "as far apart as their centres across the border");
        borders[a] += lv.width()[l];
        borders[b] += lv.width()[l];
    }
    assert!((0..lv.n()).all(|i| borders[i] == inside(&cells[i])), "each cell's links are as wide as its sides inside the map: {borders:?}");
}

/// The steepest way down (`Level::steepest`, D-infinity) of a tilted flat surface is its own slope on squares and on
/// cells of many sizes alike, so the way liquid runs does not depend on the size of the cells; on squares a surface
/// falling 30° south of east sends a third east and two thirds south-east (Tarboton 1997).
#[test]
fn the_steepest_way_down_a_tilted_surface_is_its_slope_on_any_cells() {
    let fall = [0.02 * 30f64.to_radians().cos(), 0.02 * 30f64.to_radians().sin()];
    let [(flat, south, north), _] = mixed();
    for sq in [Square::new(flat.w, flat.h, SIDE, false, SCALE), flat] {
        let mesh = sq.mesh(south, north);
        let lv = mesh.cells();
        let h = |i: usize| -(fall[0] * lv.centre()[i][0] + fall[1] * lv.centre()[i][1]);
        let inside = |i: usize| {
            let ((x, y), k) = tile(&sq, i);
            x > 0 && y > 0 && x + k < sq.w && y + k < sq.h
        };
        for i in (0..lv.n()).filter(|&i| inside(i)) {
            let drop = |k: usize| Some(h(i) - h(lv.around(i).1[k] as usize)).filter(|&d| d > 0.0);
            let (slope, ways) = lv.steepest(i, drop).expect("a way down");
            assert!((slope / fall[0].hypot(fall[1]) - 1.0).abs() < 1e-12, "cell {i}: slope {slope}");
            if lv.n() == sq.n() {
                let share = |d: usize| ways.iter().filter(|w| lv.way(i, d) == Some(lv.start()[i] as usize + w.0)).map(|w| w.1).sum::<f64>();
                assert!((share(0) - 1.0 / 3.0).abs() < 1e-12 && (share(1) - 2.0 / 3.0).abs() < 1e-12, "cell {i}: {ways:?}");
            }
        }
    }
}

/// Months a world of bodies and a part lives before its cells are renumbered: long enough that each per-cell array holds
/// numbers (the nodes' sleep, the traffic, Flow's state).
const LIVED_MONTHS: usize = 3;

/// The small world with its cover and eaters and a part in a dry cell, after `LIVED_MONTHS` months, its events taken.
fn lived() -> World {
    let mut w = worldgen::testkit::small_world();
    worldgen::life::seed(&mut w, 0);
    worldgen::life::seed(&mut w, 1);
    let i = worldgen::testkit::dry_land(&w);
    worldgen::testkit::plate(&mut w, i, 1.0, true);
    (0..LIVED_MONTHS).for_each(|_| pcore::step(&mut w));
    w.drain_events();
    w
}

/// The whole saved state of `w`.
fn saved(w: &World) -> serde_json::Value {
    serde_json::to_value(w).expect("a world serialises")
}

/// Months of the lived world at whose start the identity remap runs, one after the other (a year: each month of the
/// refreshes and of the creep of Flow); and the months each remapped world goes on for.
const REMAPPED: usize = 12;
const AFTER_REMAP: usize = 12;

/// A remap that changes nothing drops Flow's caches, as a load does: stepping on from it, at any month of the slow cycles,
/// gives the world of no remap, bit for bit.
#[test]
fn the_identity_remap_changes_no_step() {
    let mut w = lived();
    let (n, links) = (w.mesh.cells().n(), w.mesh.cells().a().len());
    let same = Remap { cells: Renumber::new(n, (0..n as u32).map(Some).collect()), links: Renumber::new(links, (0..links as u32).map(Some).collect()) };
    let month = |w: &mut World| {
        pcore::step(w);
        w.drain_events();
    };
    // The worlds remapped and still going on, the first remapped first.
    let mut remapped = std::collections::VecDeque::new();
    for m in 0..REMAPPED + AFTER_REMAP - 1 {
        if m < REMAPPED {
            let mut v = w.clone();
            v.remap(&same);
            remapped.push_back(v);
        }
        remapped.iter_mut().for_each(month);
        month(&mut w);
        if m + 1 >= AFTER_REMAP {
            let v = remapped.pop_front().expect("a world remapped a year ago");
            assert!(saved(&v) == saved(&w), "remapped at month {}, then {AFTER_REMAP} months", w.cfg.calendar().months(v.time) - AFTER_REMAP as u64);
        }
    }
}

/// Cells and links renumbered (`World::remap`): each cell holds at its new number what it held, its bodies and parts
/// are found there by their ids, and its serial goes with it; renumbered back, the world is what it was, bit for bit.
#[test]
fn a_world_renumbered_and_back_is_the_same() {
    let w = lived();
    let (n, links) = (w.mesh.cells().n(), w.mesh.cells().a().len());
    let to: Vec<u32> = (0..n).map(|j| ((7 * j + 3) % n) as u32).collect();
    let mut back = vec![0; n];
    to.iter().enumerate().for_each(|(j, &i)| back[i as usize] = j as u32);
    let reversed: Vec<Option<u32>> = (0..links as u32).rev().map(Some).collect();
    let remap = |from: &[u32]| Remap { cells: Renumber::new(n, from.iter().copied().map(Some).collect()), links: Renumber::new(links, reversed.clone()) };
    let before = saved(&w);
    let mut v = w.clone();
    v.remap(&remap(&to));
    let (a, b) = (w.store.amounts(Class::Ground), v.store.amounts(Class::Ground));
    assert!((0..w.chem.ns() * n).all(|x| b[x] == a[x / n * n + to[x % n] as usize]), "each ground holds what its cell held");
    assert_eq!(v.life.unsound(), None);
    assert!(w.life.alive().all(|x| v.life.find(x.id).map(|h| h.cell) == Some(back[x.cell as usize])), "each body is found in its cell");
    assert!(w.life.parts().count() > 0 && w.life.parts().all(|p| v.life.part_cell(p.id) == Some(back[p.cell as usize] as usize)), "each part is found in its cell");
    let cells = saved(&v)["life"]["cells"].clone();
    assert!((0..n).all(|j| cells[j]["serial"] == to[j]), "a cell's serial goes with it");
    v.remap(&remap(&back));
    assert_eq!(saved(&v), before);
}

/// The saved state's arrays (by their path) of one number per cell of `n` cells, or of blocks of them, with the count
/// of blocks: all but the map's squares and what lies inside the life of a cell.
fn per_cell(x: &serde_json::Value, n: usize, path: &str, out: &mut Vec<(String, usize)>) {
    if ["/map", "/life/cells/"].iter().any(|p| path.starts_with(p)) {
        return;
    }
    let inner: Vec<(String, &serde_json::Value)> = match x {
        serde_json::Value::Array(v) => {
            if !v.is_empty() && v.len().is_multiple_of(n) {
                out.push((path.to_string(), v.len() / n));
            }
            v.iter().enumerate().map(|(k, y)| (format!("{path}/{k}"), y)).collect()
        }
        serde_json::Value::Object(m) => m.iter().map(|(k, y)| (format!("{path}/{k}"), y)).collect(),
        _ => Vec::new(),
    };
    inner.into_iter().for_each(|(p, y)| per_cell(y, n, &p, out));
}

/// A cell appended holds nothing and takes the next serial, and every array of one number per cell has one more; taken
/// away again, it leaves the world as it was but for that serial, which no cell takes again.
#[test]
fn a_new_cell_takes_a_serial_never_given_again() {
    let w = lived();
    let (n, ns, links) = (w.mesh.cells().n(), w.chem.ns(), w.mesh.cells().a().len());
    let same_links = || Renumber::new(links, (0..links as u32).map(Some).collect());
    let grown = Remap { cells: Renumber::new(n, (0..n as u32).map(Some).chain([None]).collect()), links: same_links() };
    let shrunk = Remap { cells: Renumber::new(n + 1, (0..n as u32).map(Some).collect()), links: same_links() };
    let (mut v, mut old) = (w.clone(), Vec::new());
    per_cell(&saved(&w), n, "", &mut old);
    for round in 0..2 {
        v.remap(&grown);
        let mut new = Vec::new();
        per_cell(&saved(&v), n + 1, "", &mut new);
        assert!(!old.is_empty() && new == old, "the arrays of one number per cell, before {old:?}, after {new:?}");
        let (a, b) = (w.store.amounts(Class::Ground), v.store.amounts(Class::Ground));
        assert!((0..ns).all(|k| b[k * (n + 1) + n] == 0.0 && (0..n).all(|i| b[k * (n + 1) + i] == a[k * n + i])), "the old cells keep what they held, the new one holds nothing");
        let cell = saved(&v)["life"]["cells"][n].clone();
        assert_eq!((cell["serial"].as_u64(), cell["bodies"].as_array().map(Vec::len)), (Some((n + round) as u64), Some(0)));
        v.remap(&shrunk);
    }
    let (mut a, mut b) = (saved(&w), saved(&v));
    let next = |x: &mut serde_json::Value| x["life"].as_object_mut().and_then(|l| l.remove("next_serial")).and_then(|s| s.as_u64());
    assert_eq!((next(&mut a), next(&mut b)), (Some(n as u64), Some(n as u64 + 2)));
    assert_eq!(a, b);
}
