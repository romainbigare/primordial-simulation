//! Space: the square mesh has the map's squares (areas, links, the air as a coarser level), and stacks give each cell
//! its column of nodes.

use pcore::{World, AIR};
use pcore::space::square::Square;
use pcore::space::{Layer, Level, Mesh, Stack};
use std::collections::BTreeSet;

const SIDE: f64 = 2000.0;
const SCALE: usize = 4;

/// Two maps: one the air blocks fit, one they do not (10 x 7 leaves narrow blocks along the south and east edges).
fn maps() -> [(Square, Mesh); 2] {
    [(16, 16), (10, 7)].map(|(w, h)| {
        let sq = Square { w, h, side: SIDE };
        (sq, sq.mesh(&[SCALE], 31.0, 33.3))
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
        let world = World::mesh_of(&sq, SCALE, &pcore::Config::default());
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

#[test]
fn a_stack_of_three_layers_links_each_cell_up_its_column() {
    let [_, (sq, mesh)] = maps();
    // Today's column: a deep store and the ground on the squares, the air on its coarser level.
    let layers = [Layer { level: 0, depth: 30.0 }, Layer { level: 0, depth: 2.0 }, Layer { level: 1, depth: 1e4 }];
    let st = Stack::new(&mesh, &layers);
    let (n, na) = (sq.n(), mesh.level(1).n());
    assert_eq!(st.nodes(), 2 * n + na);
    assert_eq!(st.lo().len(), 2 * n, "one link per square below the ground, one from each square up to its air");
    let mut into_air = vec![0.0; na];
    for i in 0..n {
        let (deep, ground, air) = (st.over(&mesh, 0, i), st.over(&mesh, 1, i), st.over(&mesh, 2, i));
        assert_eq!((deep, ground, air), (i, n + i, 2 * n + mesh.up(i, 0, 1)));
        assert_eq!((st.lo()[i] as usize, st.hi()[i] as usize, st.dist()[i]), (deep, ground, 16.0));
        assert_eq!((st.lo()[n + i] as usize, st.hi()[n + i] as usize, st.dist()[n + i]), (ground, air, 5001.0));
        assert_eq!(st.area()[i], SIDE * SIDE);
        into_air[mesh.up(i, 0, 1)] += st.area()[n + i];
    }
    assert_eq!(into_air, mesh.level(1).area(), "the links into an air node cross its whole floor");
    // Two layers on the coarse level are linked once per coarse cell.
    let air2 = Stack::new(&mesh, &[layers[1], layers[2], layers[2]]);
    assert_eq!(air2.lo().len(), n + na);
    assert!(air2.lo()[n..].iter().zip(&air2.hi()[n..]).all(|(&lo, &hi)| hi as usize == lo as usize + na));
}
