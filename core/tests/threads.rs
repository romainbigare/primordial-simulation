//! The parallel passes (laws.md 0): the number of threads and the count of chunks the cells are cut into change
//! nothing.

use worldgen::life::{place_takers, seed, ROSTER};

/// Years the roster's world runs in the test of threads and chunks, so that seeds from cells of two chunks land in one
/// cell.
const YEARS: u64 = 4;
/// Counts of chunks other than the default: one chunk of all cells, and one that cuts the rows of the 16 x 16 map at
/// other places.
const OTHER_CHUNKS: [u32; 2] = [1, 3];

/// The 16 x 16 world of the roster, every starting kind of cover and of takers placed after its first month (and young born to them), run `YEARS`
/// years on a pool of `threads` threads with its cells cut into `chunks` chunks. Its whole saved state.
fn roster(threads: usize, chunks: u32) -> String {
    let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build().expect("a pool");
    pool.install(|| {
        let mut w = worldgen::testkit::one_rise(1, 16, 16);
        w.cfg.chunks = chunks;
        pcore::step(&mut w);
        for &k in &ROSTER {
            seed(&mut w, k);
        }
        assert!(!place_takers(&mut w).is_empty(), "no takers placed");
        for _ in 0..YEARS * w.cfg.months_per_year as u64 {
            pcore::step(&mut w);
        }
        assert!(pcore::cover::sums(&w).iter().any(|s| s.slots > 0.0), "nothing lives");
        assert!((0..w.mesh.cells().n()).any(|i| w.life.cell_at(i).fauna.n[pcore::fauna::FS..].iter().any(|&n| n > 0.0)), "no takers were born");
        serde_json::to_string(&w).expect("a world serialises")
    })
}

/// The roster's world on 1 and on 4 threads, and with other counts of chunks (one chunk of all cells among them): the
/// month of the cover and the takers runs on the chunks and the seeds land after, so every order is fixed by the cells.
#[test]
fn the_threads_and_the_chunks_change_nothing() {
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
            let mut worlds = [worldgen::testkit::one_rise(1, 128, 128), worldgen::terrain::new_globe(1, 64, [0; 4])];
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
