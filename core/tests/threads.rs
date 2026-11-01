//! The parallel pass of bodies (laws.md 0): the number of threads and the count of chunks the cells are cut into change
//! nothing, and a cell takes no more than its own ground holds and its share of its air node.

use pcore::store::Node;
use pcore::{brain, AIR};
use worldgen::life::{seed, ROSTER};
use worldgen::testkit::{eater, set_amount, start, Books, OX, RICH};

/// Years the roster's world runs in the test of threads and chunks.
const YEARS: u64 = 3;
/// A count of chunks other than the default, that cuts the rows of the 16 x 16 map at other places.
const OTHER_CHUNKS: u32 = 7;

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
        // (The count of chunks is a setting of the run, not of its world.)
        w.cfg.chunks = pcore::Config::default().chunks;
        serde_json::to_string(&w).expect("a world serialises")
    })
}

/// One world on 1 and on 4 threads, and with another count of chunks: every order is fixed by the cells.
#[test]
fn the_threads_and_the_chunks_change_nothing() {
    let chunks = pcore::Config::default().chunks;
    let one = roster(1, chunks);
    assert!(one == roster(4, chunks), "4 threads changed the world");
    assert!(one == roster(4, OTHER_CHUNKS), "{OTHER_CHUNKS} chunks changed the world");
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
    books.assert_kept(&w, 1e-12);
}
