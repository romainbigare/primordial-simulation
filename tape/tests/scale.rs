//! Coarse against fine: the laws do not depend on the size of the cells. The same island of 64 x 64 squares of 1 km,
//! once in cells of 2 x 2 squares (32 x 32 cells of 2 km) and once on its squares, each seed against itself: the mean
//! over the seeds of what the squares move each key number per area (the mean of its last years) is less than its
//! year-to-year spread in the coarse cells (the mean over the seeds of its highest less its lowest yearly value). The length of the rivers is printed, not held: it still moves by more than its noise
//! (decision 31). And the life of a roster island whose region splits while it runs moves no more than chance moves
//! it, its bodies stepping four times a month there (`life_does_not_depend_on_a_split_region`; the life in the region
//! itself is printed).

mod common;

use common::run_on;
use pcore::World;

const SEEDS: [u64; 3] = [1, 2, 3];
const SIDE: usize = 32;
const YEARS: u64 = 15;
/// The numbers are the means of the years from this one to `YEARS`.
const FROM: u64 = 6;
/// The key numbers: the land's ground temperature, its warmest and coldest month, rain on land, the heat-absorbing gas,
/// the cover's mass and the starting cover's capture area, m² (the first `HELD`, held to the noise), and the length of
/// the rivers, km (river cells times the cells' side).
const KEYS: [&str; 8] = ["t_land", "t_land_summer", "t_land_winter", "rain_land", "air_3", "cover_kg", "root_0_cover_area", "river_km"];
const HELD: usize = 7;

/// The `KEYS` at the end of each year from `FROM` of the island of `seed`, in cells of 2 x 2 squares or on its
/// squares (`fine`).
fn keys(seed: u64, fine: bool) -> Vec<[f64; KEYS.len()]> {
    let island = worldgen::terrain::island(2 * SIDE, 2 * SIDE);
    let map = pcore::space::square::Square::new(island.w, island.h, island.side / 2.0, false, island.scale);
    let square = map.square_depth();
    let map = map.by_blocks(|_, _| if fine { square } else { square - 1 });
    let km = map.side * if fine { 1.0 } else { 2.0 } / 1e3;
    let mut years = vec![];
    run_on(worldgen::terrain::new_map(seed, map), YEARS, false, |_| {}, |y, _, rec| {
        if y >= FROM {
            years.push(KEYS.map(|c| if c == "river_km" { rec.last("rivers") * km } else { rec.last(c) }));
        }
    });
    years
}

#[test]
#[ignore = "slow: three 15-year runs of a 32 x 32 island with life, and three of it on 64 x 64 squares"]
fn the_laws_do_not_depend_on_the_size_of_the_cells() {
    // Per seed, [coarse, fine], all side by side.
    let runs: Vec<[Vec<[f64; KEYS.len()]>; 2]> = std::thread::scope(|s| {
        let runs: Vec<_> = SEEDS.iter().map(|&seed| [false, true].map(|fine| s.spawn(move || keys(seed, fine)))).collect();
        runs.into_iter().map(|pair| pair.map(|r| r.join().expect("a run"))).collect()
    });
    let seeds = runs.len() as f64;
    let mut out = vec![];
    for (q, name) in KEYS.iter().enumerate() {
        let mean = |run: &[[f64; KEYS.len()]]| run.iter().map(|y| y[q]).sum::<f64>() / run.len() as f64;
        let spread = |run: &[[f64; KEYS.len()]]| run.iter().map(|y| y[q]).fold(f64::MIN, f64::max) - run.iter().map(|y| y[q]).fold(f64::MAX, f64::min);
        let offs: Vec<f64> = runs.iter().map(|[coarse, fine]| mean(fine) - mean(coarse)).collect();
        let (off, limit) = (offs.iter().sum::<f64>() / seeds, runs.iter().map(|[coarse, _]| spread(coarse)).sum::<f64>() / seeds);
        let ok = off.abs() <= limit || q >= HELD;
        let coarse: Vec<String> = runs.iter().map(|[coarse, _]| format!("{:.4e}", mean(coarse))).collect();
        let by_seed: Vec<String> = offs.iter().map(|x| format!("{x:.3e}")).collect();
        println!("{name}: off by {off:.3e} (seeds {by_seed:?}) on squares from the coarse cells (seeds {coarse:?}), limit {limit:.3e}{}", if off.abs() <= limit { "" } else if ok { ": out, not held" } else { ": OUT" });
        if !ok {
            out.push(*name);
        }
    }
    assert!(out.is_empty(), "the size of the cells changed: {out:?}");
}

/// The roster island of the check of a split region (spec 5.2 for life): its side in cells, its years, the year at
/// whose end its runs branch, and its seeds.
const REGION_SIDE: usize = 32;
const REGION_YEARS: u64 = 50;
const SPLIT_AT: u64 = 25;
const REGION_SEEDS: [u64; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
/// A starting kind is alive while it holds at least this share of the mass of its own life (a kind of bodies of the
/// animals', a kind of cover of the cover's).
const KIND_SHARE: f64 = 0.01;
/// The numbers compared, each the mean of the years after the branch: the animals' mass, kg, the cover's, kg, and the
/// starting kinds alive (`KIND_SHARE`), the first `LIFE_HELD`, held to chance; then in the region (where its bodies
/// step four times a month once it is split) the animals' mass, kg, their members, their mean temperature by members,
/// K, and the cover's mass, kg, printed, not held: the region is one air node of a small island, whose animals die out
/// or thrive there by chance. One body law at every grain is held where a node lives on: `core/tests/steps.rs`.
const LIFE: [&str; 7] = ["animal_kg", "cover_kg", "kinds_alive", "region_animal_kg", "region_members", "region_t", "region_cover_kg"];
const LIFE_HELD: usize = 3;
/// The share of false alarms the test allows for each number held: at most 1 in 100 for the `LIFE_HELD` numbers
/// together (Bonferroni).
const FALSE_ALARM: f64 = 1.0 / 300.0;

/// The `LIFE` numbers of `w`, whose region is the cells inside the leaves `region`.
fn life(w: &World, region: &[pcore::space::Leaf]) -> [f64; LIFE.len()] {
    let inside = |i: usize| {
        let l = w.map.leaves()[i];
        region.iter().any(|r| l.x >= r.x && l.x < r.x + r.side() && l.y >= r.y && l.y < r.y + r.side())
    };
    let mut animal = vec![0.0; worldgen::life::KINDS.len()];
    for b in w.life.alive() {
        if let Some(l) = w.life.brain(b.lineage) {
            animal[l.root as usize] += b.mass(&w.chem);
        }
    }
    let cover: Vec<f64> = pcore::cover::sums(w).iter().map(|s| s.kg).collect();
    let alive = |kg: &[f64]| {
        let all: f64 = kg.iter().sum();
        kg.iter().filter(|&&x| x > 0.0 && x >= KIND_SHARE * all).count() as f64
    };
    let here: Vec<&pcore::body::Body> = w.life.alive().filter(|b| inside(b.cell as usize)).collect();
    let members: f64 = here.iter().map(|b| b.n()).sum();
    let t = here.iter().map(|b| b.temp(&w.chem) * b.n()).sum::<f64>() / members.max(f64::MIN_POSITIVE);
    let (per_m2, area) = (pcore::cover::per_m2(w, |c, mm, x| c.kg(mm, x)), w.mesh.cells().area());
    let region_cover = (0..per_m2.len()).filter(|&i| inside(i)).map(|i| per_m2[i] * area[i]).sum();
    let animals = w.life.alive().map(|b| b.mass(&w.chem)).sum();
    [animals, cover.iter().sum(), alive(&animal) + alive(&cover), here.iter().map(|b| b.mass(&w.chem)).sum(), members, t, region_cover]
}

/// How a run goes on from the end of year `SPLIT_AT`: unsplit, unsplit with another stream of random draws, or with its
/// region split two depths and a third stream.
#[derive(Clone, Copy)]
enum Branch {
    Unsplit,
    Redrawn,
    Split,
}

impl Branch {
    /// The bits the branch flips in the run's seed: its own stream of draws. Past the split the seed keys every draw
    /// (`rng::Rng`) and nothing else, so another seed leaves the world and its laws as they are and changes only chance.
    fn stream(self) -> u64 {
        match self {
            Branch::Unsplit => 0,
            Branch::Redrawn => 0x9e37_79b9_7f4a_7c15,
            Branch::Split => 0xd1b5_4a32_d192_ed03,
        }
    }
}

/// The means of the `LIFE` numbers over the years after `SPLIT_AT` of the roster island of `seed`, run to the end of
/// year `SPLIT_AT` once and then on in each `Branch` (its region the cells of its first air node over land only, split
/// with the relief of the run's seed before the seed changes).
fn branches(seed: u64) -> [[f64; LIFE.len()]; 3] {
    let w = worldgen::new_world(seed, REGION_SIDE, REGION_SIDE);
    let region = worldgen::testkit::land_block(&w);
    let (w, _) = common::run_stepped(w, SPLIT_AT, true, |_| {}, pcore::step, |_, _, _| {});
    std::thread::scope(|s| {
        let runs = [Branch::Unsplit, Branch::Redrawn, Branch::Split].map(|branch| {
            let (mut w, region) = (w.clone(), &region);
            s.spawn(move || {
                if let Branch::Split = branch {
                    worldgen::testkit::split_down(&mut w, region, 2);
                }
                w.seed ^= branch.stream();
                let (mut rec, mut sum, mut years) = (tape::Recorder::new(&w, 0, 0), [0.0; LIFE.len()], 0.0);
                common::go_on(w, &mut rec, REGION_YEARS - SPLIT_AT, true, pcore::step, |_, w, _| {
                    sum.iter_mut().zip(life(w, region)).for_each(|(a, x)| *a += x);
                    years += 1.0;
                });
                sum.map(|x| x / years)
            })
        });
        runs.map(|r| r.join().expect("a branch"))
    })
}

/// The mean over the seeds of what the split shifts a number by, and the bound chance sets on it: `x` holds a number of
/// each seed in the order of `Branch`. A branch's shift is its number less the mean of the other two. Each of the 3^N
/// relabellings (each seed's split label on any of its three branches) gives a mean shift; the bound is the
/// (⌊3^N `FALSE_ALARM`⌋ + 1)-th largest in size, so no more than a share `FALSE_ALARM` of them lie above it.
fn shift_and_bound(x: &[[f64; 3]]) -> (f64, f64) {
    let shifts: Vec<[f64; 3]> = x.iter().map(|v| std::array::from_fn(|b| v[b] - (v.iter().sum::<f64>() - v[b]) / 2.0)).collect();
    let n = shifts.len();
    let mut all: Vec<f64> = (0..3usize.pow(n as u32))
        .map(|mut label| {
            let sum: f64 = shifts.iter().map(|e| {
                let b = label % 3;
                label /= 3;
                e[b]
            }).sum();
            sum.abs() / n as f64
        })
        .collect();
    all.sort_by(|a, b| b.total_cmp(a));
    let split = shifts.iter().map(|e| e[Branch::Split as usize]).sum::<f64>() / n as f64;
    (split, all[(all.len() as f64 * FALSE_ALARM) as usize])
}

/// Spec 5.2 for life: a roster island whose region of one air node splits two depths (its cells a quarter of the side
/// of the others, its bodies stepping four times a month) at the end of year `SPLIT_AT` keeps its animals' mass, its
/// cover's mass and its kinds alive (`KIND_SHARE`; the means of the years after) within what chance alone moves them.
///
/// Each seed runs to the end of year `SPLIT_AT` once and goes on three ways from that one state (`Branch`): unsplit,
/// unsplit with another stream of draws, and split with a third, so that the split and the unsplit runs of a seed share
/// all their history and differ after it only by the split and by chance, and the second unsplit run measures chance.
/// Each branch has a stream of its own: the draws are keyed by the day and the body, so a split run on the unsplit
/// run's stream would share most of its draws and differ from it by less than chance does. Each number is compared as
/// its logarithm, so a shift is relative and seeds of many and of few animals weigh alike.
///
/// If the split changes nothing beyond chance, the three branches of a seed are alike draws of its years after, and
/// which of them is the split is a label that changes nothing. The test is a permutation test over those labels
/// (Fisher 1935): the mean over the N seeds of the split's shift (its number less the mean of the other two) must not
/// exceed the bound that no more than `FALSE_ALARM` of the 3^N relabellings exceed (`shift_and_bound`; with 8 seeds,
/// 6561 relabellings, the 22nd largest). The bound is measured from the runs, with no assumption on the shape of the
/// noise or on its size in each seed, and the false alarms are at most `FALSE_ALARM` for each number by construction.
/// A split that shifts a number by more than chance in every seed gives the most extreme labelling of all.
///
/// A kind counts as alive while it holds `KIND_SHARE` of its life's mass: one of a few members out of thousands lives or
/// dies by chance in every branch, and a count that moves by a whole kind for it decides the test by a coin. A kind
/// that matters to the island's life holds a share, and a split that kills it or lets a new one rise moves the count
/// by a whole kind. The life in the region is printed (`LIFE`).
#[test]
#[ignore = "slow: eight 25-year roster runs of a 32 x 32 island, each going on 25 years three ways, one with a region split two depths"]
fn life_does_not_depend_on_a_split_region() {
    let runs: Vec<[[f64; LIFE.len()]; 3]> = std::thread::scope(|s| {
        let runs: Vec<_> = REGION_SEEDS.iter().map(|&seed| s.spawn(move || branches(seed))).collect();
        runs.into_iter().map(|r| r.join().expect("a run")).collect()
    });
    let mut out = vec![];
    for (q, name) in LIFE.iter().enumerate() {
        let x: Vec<[f64; 3]> = runs.iter().map(|run| run.map(|b| b[q])).collect();
        let list = |b: Branch| x.iter().map(|v| format!("{:.4e}", v[b as usize])).collect::<Vec<_>>().join(", ");
        let values = format!("{name} (seeds {REGION_SEEDS:?}): unsplit [{}], redrawn [{}], split [{}]", list(Branch::Unsplit), list(Branch::Redrawn), list(Branch::Split));
        if q >= LIFE_HELD {
            println!("{values}, not held");
            continue;
        }
        let (shift, bound) = shift_and_bound(&x.iter().map(|v| v.map(f64::ln)).collect::<Vec<_>>());
        println!("{values}; the split shifts its log by {shift:+.4e}, chance bounds it at {bound:.4e}{}", if shift.abs() > bound { ": OUT" } else { "" });
        if shift.abs() > bound {
            out.push(*name);
        }
    }
    assert!(out.is_empty(), "the split region changed the life: {out:?}");
}
