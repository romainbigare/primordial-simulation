//! Space: the mesh of a map has its cells (areas, links, the air as a coarser level), cells of many squares among
//! them; and the terrain's channels on its samples.

use pcore::{World, AIR};
use pcore::space::square::Square;
use pcore::space::tree::BLOCK;
use pcore::space::{Leaf, Level, Mesh};
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

/// Where four cells meet at a point (each has a corner there), the two across it link, unless they differ by more than
/// a depth or one of them has 8 links already; a link of no width joins two cells that meet only at a point; no cell has
/// more than 8 links.
fn corners(sq: &Square, mesh: &Mesh) {
    let (lv, leaves, unit) = (mesh.cells(), sq.leaves(), BLOCK / SCALE as u64);
    let (across, down) = (sq.w as u64 * unit, sq.h as u64 * unit);
    let links = |i: usize| lv.around(i).0.len();
    assert!((0..lv.n()).all(|i| links(i) <= 8), "no cell has more than 8 links");
    // Leaf `l` has a corner at (`x`, `y`), on its east (`east`) or west side and its south (`south`) or north side.
    let corner = |l: &Leaf, (x, y): (u64, u64), east: bool, south: bool| {
        ((l.x + if east { l.side() } else { 0 }) % across, l.y + if south { l.side() } else { 0 }) == (x % across, y)
    };
    for l in 0..lv.a().len() {
        let (p, q) = (&leaves[lv.a()[l] as usize], &leaves[lv.b()[l] as usize]);
        let touch = |p: &Leaf, q: &Leaf| [(true, true), (true, false)].iter().any(|&(e, s)| corner(p, (q.x, if s { q.y } else { q.y + q.side() }), e, s));
        assert!(lv.width()[l] > 0.0 || touch(p, q) || touch(q, p), "a link of no width joins cells that meet at a point");
    }
    for c in leaves {
        let (x, y) = (c.x + c.side(), c.y + c.side());
        if y >= down || (!sq.wrap && x >= across) {
            continue;
        }
        let at = |dx: u64, dy: u64| mesh.tree().cell_at([(x + across - dx) % across, y - dy]);
        let [nw, ne, sw, se] = [at(1, 1), at(0, 1), at(1, 0), at(0, 0)];
        let meet = corner(&leaves[ne], (x, y), false, true) && corner(&leaves[sw], (x, y), true, false) && corner(&leaves[se], (x, y), false, false);
        for (i, j) in [(nw, se), (ne, sw)].into_iter().filter(|_| meet) {
            let near = leaves[i].depth.abs_diff(leaves[j].depth) <= 1;
            assert!(lv.link(i, j).is_some() || !near || links(i) == 8 || links(j) == 8, "the cells across a corner where four meet link");
        }
    }
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
    let (leaf, unit) = (sq.leaves()[i], BLOCK / SCALE as u64);
    (((leaf.x / unit) as usize, (leaf.y / unit) as usize), (leaf.side() / unit) as usize)
}

/// Cells of many sizes, on a flat map and on a globe: numbered by their north-west squares row by row, each covers its
/// squares and lies under their air block, the air level is that of the squares; each cell keeps the borders of its
/// squares with the others, cells of two sizes link across a border as far apart as their centres across it, and
/// across a corner where four cells meet (`corners`).
#[test]
fn cells_of_many_sizes_cover_their_squares_and_keep_the_borders() {
    for (sq, south, north) in mixed() {
        let (mesh, plain) = (sq.mesh(south, north), Square::new(sq.w, sq.h, sq.side, sq.wrap, SCALE).mesh(south, north));
        let lv = mesh.cells();
        links_once(lv);
        links_once(mesh.level(AIR));
        corners(&sq, &mesh);
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
            if lv.width()[l] > 0.0 && !sq.wrap {
                assert!(near(lv.dist()[l], (ka + kb) as f64 / 2.0 * SIDE), "centres lie half their sides apart across a border");
            }
        }
    }
}

/// A tree of three depths in one air block (a fine north-west corner, its finest cells half a square on a side): its
/// cells tile the block, each found at its corner; cells across a border differ by one depth at most and link across
/// it as far apart as their centres across it; each cell's links are as wide as its sides inside the map, so each
/// border's width is the same seen from either side; across a corner where four cells meet the two across it link
/// (`corners`), as far apart as their centres.
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
    corners(&sq, &mesh);
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
            assert!((lv.dist()[l] / way[0].hypot(way[1]) - 1.0).abs() < 1e-12, "as far apart as their centres across the corner");
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

/// The wear and the high stand at the world's start keep what they move (`Samples::books`): what they take off the land
/// lies in the closed basins and on the shelves of the open low ground (none leaves this island's map), to a part in
/// 1e9; the loose ground they leave is nowhere less than none, and every height and age is finite.
#[test]
fn the_wear_keeps_what_it_moves() {
    let w = worldgen::testkit::small_world();
    let s = w.terrain_samples();
    let b = s.books;
    println!("taken {:.4e} m3, in the basins {:.4e}, on the shelf {:.4e}", b.taken, b.settled, b.shelf);
    assert!(b.taken > 0.0 && b.settled > 0.0 && b.shelf > 0.0, "{b:?}");
    assert!(((b.settled + b.shelf + b.high + b.beyond) / b.taken - 1.0).abs() < 1e-9 && b.beyond.abs() < 1e-9 * b.taken, "{b:?}");
    assert!(s.loose.iter().all(|&d| d >= 0.0), "loose ground under none");
    assert!(s.age.iter().chain(&s.h).all(|x| x.is_finite()), "a height or an age not finite");
}

/// An island of drawn rises (`space::rises`): its land stays off the map's edges, so the open low ground runs along every
/// edge as one part and joins the low ground between its lands; and its wear and its high stand keep what they move,
/// the high stand laying load at its mouths.
#[test]
fn a_drawn_islands_low_ground_is_one_and_rings_the_map() {
    use pcore::space::network::NONE;
    let cfg = pcore::Config::default();
    let (map, terrain) = worldgen::terrain::island_ground(2, (32, 32), pcore::space::terrain::Shape::Drawn, &cfg.phys);
    let s = terrain.samples(2, &map, (cfg.lat_south, cfg.lat_north));
    let open: Vec<usize> = (0..s.ponds.len()).filter(|&p| s.ponds[p].pass == NONE).collect();
    assert_eq!(open.len(), 1, "one open low ground");
    let edge = |k: usize| k.is_multiple_of(s.across) || k % s.across == s.across - 1 || k / s.across == 0 || k / s.across == s.down - 1;
    assert!((0..s.h.len()).filter(|&k| edge(k)).all(|k| s.pond[k] == open[0] as u32), "the open low ground along every edge");
    let land = (0..s.h.len()).filter(|&k| s.pond[k] == NONE).count() as f64 / s.h.len() as f64;
    let b = s.books;
    println!("land {:.1}%, taken {:.4e} m3, in the basins {:.4e}, on the shelf {:.4e}", 100.0 * land, b.taken, b.settled, b.shelf);
    assert!(land > 0.1, "land {land}");
    assert!(b.high > 0.0, "{b:?}");
    assert!(((b.settled + b.shelf + b.high + b.beyond) / b.taken - 1.0).abs() < 1e-9, "{b:?}");
}

/// The terrain's channels (`space::network`): each sample's liquid runs to a neighbour no higher, but in a pond, where
/// it runs on toward the pass; from every sample of land the way leads to the open low ground under 0 m; a pond's
/// samples lie under its level, which is its pass's height. The area that drains through a sample is its own and all
/// that runs to it, so the sinks gather the whole map's.
#[test]
fn the_terrains_channels_run_down() {
    use pcore::space::network::NONE;
    let w = worldgen::testkit::small_world();
    let s = w.terrain_samples();
    let mut own = s.up.clone();
    for (k, &j) in s.to.iter().enumerate().filter(|p| *p.1 != NONE) {
        assert!(s.h[j as usize] <= s.h[k] || s.pond[k] != NONE, "sample {k} runs down");
        own[j as usize] -= s.up[k];
    }
    let sinks: f64 = (0..s.up.len()).filter(|&k| s.to[k] == NONE).map(|k| s.up[k]).sum();
    let all: f64 = own.iter().sum();
    assert!(own.iter().all(|&a| a > 0.0) && (sinks / all - 1.0).abs() < 1e-12, "the sinks gather {sinks} m² of {all} m²");
    let end = |mut k: usize| {
        for _ in 0..s.h.len() {
            match s.to[k] {
                NONE => return Some(k),
                j => k = j as usize,
            }
        }
        None
    };
    assert!((0..s.h.len()).filter(|&k| s.h[k] >= 0.0).all(|k| end(k).is_some_and(|e| s.h[e] < 0.0)), "every sample of land drains to the low ground");
    for (k, &p) in s.pond.iter().enumerate().filter(|q| *q.1 != NONE && s.ponds[*q.1 as usize].pass != NONE) {
        let pond = s.ponds[p as usize];
        assert!(s.h[k] < pond.level && s.pond[pond.pass as usize] == NONE && s.h[pond.pass as usize] == pond.level, "sample {k} under its pond's pass");
    }
    println!("{} ponds over {} samples", s.ponds.len(), s.pond.iter().filter(|&&p| p != NONE).count());
}
