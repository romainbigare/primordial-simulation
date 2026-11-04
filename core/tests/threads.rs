//! The parallel pass of bodies (laws.md 0): the number of threads and the count of chunks the cells are cut into change
//! nothing, and a cell takes no more than its own ground holds and its share of its air node.

use pcore::store::Node;
use pcore::{brain, AIR};
use worldgen::life::{seed, ROSTER};
use pcore::body::HIDDEN;
use worldgen::testkit::{dry_lands, eater, set_amount, start, Books, Brain, Line, OX, RICH};

/// Years the roster's world runs in the test of threads and chunks: long enough for it to crowd, so that moves and
/// litters from cells of two chunks land in one cell.
const YEARS: u64 = 10;
/// Eaters that move to new brain sizes in the test of chunks, and the months it runs.
const RESIZERS: usize = 6;
const RESIZE_MONTHS: usize = 2;
/// Counts of chunks other than the default: one chunk of all cells, and one that cuts the rows of the 16 x 16 map at
/// other places.
const OTHER_CHUNKS: [u32; 2] = [1, 7];

/// The 16 x 16 world of the roster, every starting kind placed after its first month, run `YEARS` years on a pool of
/// `threads` threads with its cells cut into `chunks` chunks: its whole saved state.
fn roster(threads: usize, chunks: u32) -> String {
    let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build().expect("a pool");
    pool.install(|| {
        let mut w = worldgen::new_world(1, 16, 16);
        w.cfg.chunks = chunks;
        pcore::step(&mut w);
        for &k in &ROSTER {
            seed(&mut w, k);
        }
        for _ in 0..YEARS * w.cfg.months_per_year as u64 {
            pcore::step(&mut w);
        }
        assert!(w.life.alive().count() > 0, "nothing lives");
        serde_json::to_string(&w).expect("a world serialises")
    })
}

/// The world of `start` with an eater in each of its first `RESIZERS` dry cells whose hidden-units gene is two units
/// over its lineage's (all of one lineage), run `RESIZE_MONTHS` months with its cells cut into `chunks` chunks: at their
/// first update they move to new brain sizes (`lineage::resize`). Its whole saved state, and the lineages made.
fn resizing(chunks: u32) -> (String, usize) {
    let (mut w, _) = start();
    w.cfg.chunks = chunks;
    let line = Line::new(&mut w, &Brain::only(brain::REST), 1);
    for &i in dry_lands(&w).iter().take(RESIZERS) {
        let mut b = line.founder(&mut w, 1, i);
        b.genes[HIDDEN] += 2.0;
        b.prepare(&w.chem, &w.cfg);
        w.add(b);
    }
    let founded = w.life.brains.len();
    for _ in 0..RESIZE_MONTHS {
        pcore::step(&mut w);
    }
    (serde_json::to_string(&w).expect("a world serialises"), w.life.brains.len() - founded)
}

/// One world on 1 and on 4 threads, and with other counts of chunks (one chunk of all cells among them): in the
/// roster's world, and in one where bodies of one lineage in many cells move to the same new brain sizes in one pass
/// (each makes a lineage of its own, whoever shares its chunk). Every order is fixed by the cells.
#[test]
fn the_threads_and_the_chunks_change_nothing() {
    let (one, made) = resizing(pcore::Config::default().chunks);
    assert!(made >= RESIZERS, "{made} lineages made");
    for other in OTHER_CHUNKS {
        assert!(one == resizing(other).0, "{other} chunks changed the world where brains resize");
    }
    let chunks = pcore::Config::default().chunks;
    let one = roster(1, chunks);
    assert!(one == roster(4, chunks), "4 threads changed the world");
    for other in OTHER_CHUNKS {
        assert!(one == roster(4, other), "{other} chunks changed the world");
    }
}

/// A small globe (8 x 4 air nodes, each its own block, so the wind runs on every link and its gases and heat on the
/// pool) a year on 1 and on 4 threads: the same world.
#[test]
fn the_threads_change_nothing_in_the_wind() {
    let year = |threads: usize| {
        let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build().expect("a pool");
        pool.install(|| {
            let mut w = worldgen::terrain::new_globe(1, 16, [0; 4]);
            for _ in 0..w.cfg.months_per_year {
                pcore::step(&mut w);
            }
            serde_json::to_string(&w).expect("a world serialises")
        })
    };
    assert!(year(1) == year(4), "4 threads changed the globe");
}

/// Months the worlds of the pool's test run (`the_threads_change_nothing_where_the_air_pays_for_the_pool`).
const POOL_MONTHS: usize = 2;

/// Worlds just big enough that the air's day sends its counted phases to the pool (`par::pays`): a 128 x 128 island
/// (what the ground gives, the air its rain meets and the air touching its places) and a globe of 64 rows (the vapour's
/// days, each node its own block). `POOL_MONTHS` months of each on 1 and on 4 threads: the same worlds.
#[test]
fn the_threads_change_nothing_where_the_air_pays_for_the_pool() {
    let months = |threads: usize| {
        let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build().expect("a pool");
        pool.install(|| {
            let mut worlds = [worldgen::new_world(1, 128, 128), worldgen::terrain::new_globe(1, 64, [0; 4])];
            for w in &mut worlds {
                for _ in 0..POOL_MONTHS {
                    pcore::step(w);
                }
            }
            worlds.map(|w| serde_json::to_string(&w).expect("a world serialises"))
        })
    };
    assert!(months(1) == months(4), "4 threads changed the island or the globe");
}

/// Bodies in every land cell under one air node that holds little of the gas they burn with, on ground that holds little
/// of what they eat: each cell takes no more than its ground holds and its share of the air (the cells lie in several
/// chunks), so nothing is overdrawn and the books are kept.
#[test]
fn a_cell_takes_no_more_than_its_ground_and_its_share_of_the_air() {
    let (mut w, i) = start();
    let j = w.mesh.up(i, 0, AIR);
    let under: Vec<usize> = w.mesh.level(AIR).children(j).iter().map(|&c| c as usize).filter(|&c| w.is_land(c)).collect();
    assert!(under.len() > 1, "one land cell under the air node");
    for &c in &under {
        for _ in 0..4 {
            let e = eater(&mut w, c, brain::TAKE, 1.0);
            w.add(e);
        }
        set_amount(&mut w, Node::ground(c), RICH, 1e-3);
    }
    set_amount(&mut w, Node::air(j), OX, 1.0);
    pcore::survey::survey(&mut w);
    let (books, rounded) = (Books::now(&w), w.counts.rounded[0]);
    pcore::survey::batch(&mut w);
    let lost = w.counts.rounded[0] - rounded;
    assert!(w.a(OX, j) >= 0.0 && lost <= 1e-9, "the air holds {} mol of the gas; {lost} mol rounded away", w.a(OX, j));
    assert!(under.iter().all(|&c| w.g(RICH, c) >= 0.0), "a ground was overdrawn");
    books.assert_kept(&w, pcore::check::DRIFT);
}
