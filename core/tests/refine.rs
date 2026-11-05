//! Refinement while a world runs (laws.md 0): at each month's start the cells of most interest split and the quietest
//! quartets join, under a budget of cells and no finer than a floor, the camera an input of the run. With no budget
//! nothing changes; a camera window refines its region to the depth it asks and the cells join back once it goes; the
//! budget and the 2:1 rule hold and the books close through a year of refinement with life, the same world on any count
//! of threads.

use pcore::check;
use pcore::event::EventKind;
use pcore::refine::{Camera, Shot};
use pcore::World;
use worldgen::testkit::{self, sound, Books};

/// `n` months as the shell runs them: refine by `camera`, step, take the events, then `each(world)`. Returns the
/// splits and the joins made.
fn months(w: &mut World, camera: &Camera, n: usize, mut each: impl FnMut(&World)) -> [usize; 2] {
    let mut made = [0; 2];
    for _ in 0..n {
        w.refine(camera);
        pcore::step(w);
        for e in w.drain_events() {
            made[0] += (e.kind == EventKind::Split) as usize;
            made[1] += (e.kind == EventKind::Join) as usize;
        }
        each(w);
    }
    made
}

/// A window of `side` x `side` squares from the north-west corner of cell `i` of `w`, from `month` on, asking for
/// cells of `depth`.
fn window(w: &World, i: usize, side: f64, month: u64, depth: u8) -> Shot {
    let [x, y, ..] = w.map.bounds(i);
    Shot { month, x, y, w: side, h: side, depth }
}

/// The camera going away from `month` on.
fn away(month: u64) -> Shot {
    Shot { month, x: 0.0, y: 0.0, w: 0.0, h: 0.0, depth: 0 }
}

/// With no budget of cells (`Config::cell_budget` 0, every run's default) a camera changes nothing: a living island
/// refined each month is, bit for bit, the island stepped alone.
#[test]
fn with_no_budget_the_cells_stay_as_built() {
    let mut a = testkit::living(16, 2);
    a.drain_events();
    let mut b = a.clone();
    let camera = Camera { shots: vec![window(&a, testkit::dry_land(&a), 2.0, 0, a.map.square_depth() + 2)] };
    assert_eq!(months(&mut a, &camera, 3, |_| {}), [0, 0]);
    months(&mut b, &Camera::default(), 3, |_| {});
    assert_eq!(serde_json::to_string(&a).expect("a world"), serde_json::to_string(&b).expect("a world"));
}

/// The months the camera of `a_camera_window_refines_its_region_and_the_cells_join_back` looks, then is away.
const LOOK: usize = 3;
const AWAY: usize = 4;

/// On the small island without life, a camera window of 2 x 2 squares of land asking for cells two depths finer than a
/// square (the floor) refines it in a few months: every cell it sees is of that depth, the cells within the budget and
/// 2:1 kept. Once the camera goes the cells join back to the mesh as built.
#[test]
fn a_camera_window_refines_its_region_and_the_cells_join_back() {
    let mut w = testkit::small_world();
    w.drain_events();
    let (built, depth) = (w.map.leaves().to_vec(), w.map.square_depth() + 2);
    (w.cfg.cell_budget, w.cfg.finest_depth) = (built.len() as u32 + 200, depth);
    let shot = window(&w, testkit::dry_land(&w), 2.0, w.month(), depth);
    let camera = Camera { shots: vec![shot, away(w.month() + LOOK as u64)] };
    let budget = w.cfg.cell_budget as usize;
    let [splits, _] = months(&mut w, &camera, LOOK, |w| {
        assert!(w.mesh.cells().n() <= budget, "{} cells over the budget of {budget}", w.mesh.cells().n());
        sound(w);
    });
    let seen: Vec<u8> = (0..w.mesh.cells().n()).filter(|&i| shot.sees(w.map.bounds(i), 0.0)).map(|i| w.map.leaves()[i].depth).collect();
    assert!(seen.len() == 64 && seen.iter().all(|&d| d == depth), "the window is cells of depth {depth}: {seen:?}");
    assert!(splits >= 20, "the window's 4 squares, their 16 quarters, and the squares beside them: {splits} splits");
    let [_, joins] = months(&mut w, &camera, AWAY, sound);
    assert_eq!(joins, splits, "each split joined back");
    assert_eq!(w.map.leaves(), &built[..], "the mesh as built");
}

/// A camera window that crosses the land of a living island, a new place every `STRIDE` months, under a budget of
/// `BUDGET` times the cells as built.
const STRIDE: u64 = 2;
const BUDGET: f64 = 1.25;

/// A year of refinement on a living island (the camera crossing its land, the other terms of the score at work): every
/// month the cells stay within the budget, 2:1 holds and the books close within `check::DRIFT`; cells split and join;
/// and on 1 and on 4 threads the year gives the same world.
#[test]
fn a_year_of_refinement_with_life_keeps_the_budget_and_the_books() {
    let year = |threads: usize| {
        let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build().expect("a pool");
        pool.install(|| {
            let mut w = testkit::living(22, 3);
            w.drain_events();
            let depth = w.map.square_depth() + 2;
            (w.cfg.cell_budget, w.cfg.finest_depth) = ((w.mesh.cells().n() as f64 * BUDGET) as u32, depth);
            let lands = testkit::dry_lands(&w);
            let months_now = w.month();
            let shots = (0..12 / STRIDE).map(|k| window(&w, lands[k as usize * lands.len() / 6], 3.0, months_now + k * STRIDE, depth)).collect();
            let (books, budget) = (Books::now(&w), w.cfg.cell_budget as usize);
            let made = months(&mut w, &Camera { shots }, 12, |w| {
                assert!(w.mesh.cells().n() <= budget, "{} cells over the budget of {budget}", w.mesh.cells().n());
                sound(w);
                books.assert_kept(w, check::DRIFT);
            });
            println!("splits and joins {made:?}, cells {} of a budget of {budget}, bodies {}", w.mesh.cells().n(), w.life.alive().count());
            assert!(made[0] > 0 && made[1] > 0, "cells split and joined: {made:?}");
            assert!(w.life.alive().count() > 0, "life goes on");
            serde_json::to_string(&w).expect("a world")
        })
    };
    assert!(year(1) == year(4), "one world on 1 and on 4 threads");
}
