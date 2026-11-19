//! A slot is the bookkeeping of a kind in a cell, not a thing of the world (laws.md 3): the cover's laws act on the
//! stand, so a cell whose slot is cut into alike pieces stands as the uncut cell does.

use pcore::cover::{self, Cover, NO_ROOT, WALL};
use pcore::World;
use worldgen::life::{KINDS, LIGHT_ROW};
use worldgen::testkit::{self, dry_lands, small_world};

/// How near a cut cell stands to the uncut one at the end of the month it is cut in, a share: the genes' 32-bit
/// rounding (6e-8) moves the stand by about 1e-8 in a year (seen); a law that acts on a slot moves it by 1e-4 and more.
/// The pieces merge back as alike slots at that month's end, so each month of a year is cut on its own: in the months
/// after, the world carries the rounding of sums taken in another order as it carries any rounding, and its own edges
/// (a seedling or a lot of seeds at the edge of being) carry a nudge of 1e-14 of the uncut stand to 6e-6 within a year
/// (seen).
const SAME: f64 = 1e-6;

/// How near the large plants stand to as tall when seeds join their slot, a share: the seeds and the walls they bring
/// move the large plants' share of the slot's walls (`Cover::sizes`; seen: 1e-6 to 3e-4).
const SEEDS_WALLS: f64 = 1e-3;

/// How near a stand's large plants end a month with a flush of seedlings to the month without it, a share: the
/// seedlings take their share of the slot's growth, of its light and of its upkeep, of which more goes unpaid with
/// their leaves (seen: up to 3.4e-3).
const FLUSH: f64 = 5e-3;

/// A slot that has shed all but this share of its living cover, whose large plants hold no leaves.
const SHED: f64 = 1e-3;

/// The row of the big slow kind of cover (a tree).
const TREE_ROW: usize = 2;

/// The stand of cover `c` of world `w`: its kg, plants and height by built cover, and its seeds lying, kg.
fn stand(w: &World, c: &Cover) -> [f64; 4] {
    let p = cover::Parts::of(&w.chem).expect("cover parts");
    let (mm, rho) = (p.mm(&w.chem), w.chem.subs[p.sub[WALL]].density[0]);
    let built: f64 = c.held().map(|x| c.built(&mm, x)).sum();
    let h = c.held().map(|x| c.built(&mm, x) * c.height(&w.cfg.phys, &mm, rho, x)).sum::<f64>() / built.max(1e-300);
    [c.held().map(|x| c.kg(&mm, x)).sum(), c.held().map(|x| c.count(x)).sum(), h, c.lying.kg(&mm)]
}

/// Cut the heaviest slot of cover `c` of world `w` into `k` alike pieces: the same genes and state, its amounts and
/// each group's plants a k-th each.
fn cut(w: &World, c: &mut Cover, k: usize) {
    let mm = cover::Parts::of(&w.chem).expect("cover parts").mm(&w.chem);
    let x = c.held().max_by(|&a, &b| c.kg(&mm, a).total_cmp(&c.kg(&mm, b))).expect("a stand");
    (c.mol[x], c.plants[x]) = (c.mol[x].map(|m| m / k as f64), c.plants[x].map(|n| n / k as f64));
    for _ in 1..k {
        let e = (0..cover::SLOTS).find(|&y| c.root[y] == NO_ROOT).expect("room");
        (c.mol[e], c.plants[e], c.share[e], c.genes[e], c.root[e], c.full[e], c.net[e], c.rooted[e], c.need[e]) = (c.mol[x], c.plants[x], c.share[x], c.genes[x], c.root[x], c.full[x], c.net[x], c.rooted[x], c.need[x]);
        (c.line[e], c.stands[e], c.airless[e], c.drained[e]) = (c.line[x], c.stands[x], c.airless[x], c.drained[x]);
    }
}

/// Each number of the cut stand `cut` is the uncut stand's `whole` to `SAME`.
fn assert_same(what: &str, cut: &[f64], whole: &[f64]) {
    for (a, b) in cut.iter().zip(whole) {
        assert!((a - b).abs() <= SAME * b.abs(), "{what}: {cut:?}, uncut {whole:?}");
    }
}

/// A world whose cells hold every slot, with a stand of kind `row` grown a year from `kg` per m² on the dry land cell
/// rain reaches most, and that cell.
fn grown(row: usize, kg: f64) -> (World, usize) {
    let mut w = small_world();
    w.cfg.variety = cover::SLOTS;
    worldgen::life::soil(&mut w, &worldgen::chemistry0().1.soil);
    let i = dry_lands(&w).into_iter().filter(|&i| w.rain[i] > 0.0).max_by(|&a, &b| w.rain[a].total_cmp(&w.rain[b])).expect("dry land with rain");
    assert!(cover::place(&mut w, i, KINDS[row].genes, row as u32, kg));
    (0..12).for_each(|_| pcore::step(&mut w));
    (w, i)
}

/// A stand of kind `row` grown a year from `kg` per m² on the dry land cell rain reaches most; then, at the start of
/// each month of the next year, the same world with its slot cut into two and into four alike pieces: at that month's
/// end the cell's cover, its plants, their height, the seeds lying on its ground and all the cover and seeds of the
/// world are the uncut world's to `SAME` (what is left is rounding: of sums taken in another order, and of the genes,
/// kept in 32 bits, of seeds that join a lot one packet at a time).
fn cut_stands_as_whole(row: usize, kg: f64) {
    let (mut w, i) = grown(row, kg);
    let mm = cover::Parts::of(&w.chem).expect("cover parts").mm(&w.chem);
    let month = |mut w: World| {
        pcore::step(&mut w);
        let all: f64 = (0..w.mesh.cells().n()).map(|j| w.life.cover(j)).map(|c| c.held().map(|x| c.kg(&mm, x)).sum::<f64>() + c.lying.kg(&mm)).sum();
        let s = stand(&w, w.life.cover(i));
        (w, [s[0], s[1], s[2], s[3], all])
    };
    for m in 0..12 {
        let (next, whole) = month(w.clone());
        for k in [2, 4] {
            let mut c = w.clone();
            c.in_cell(i, |w, d| cut(w, d.cover(i), k));
            let cut = month(c).1;
            println!("kind {row}, month {m}, cut in {k}: kg, plants, height m, seeds lying kg, the world's kg {cut:?}, uncut {whole:?}");
            assert_same(&format!("month {m}, cut in {k}"), &cut, &whole);
        }
        w = next;
    }
}

#[test]
fn a_herb_stand_cut_into_slots_stands_as_the_whole() {
    cut_stands_as_whole(LIGHT_ROW, 0.1);
}

#[test]
fn a_tall_stand_cut_into_slots_stands_as_the_whole() {
    cut_stands_as_whole(TREE_ROW, 5.0);
}

/// The share of the last month's liquid for roots the garden's ground holds in a month it dries.
const DRYING: f64 = 0.5;

/// What the garden's ground holds for roots in month `m` of its year (`Around::moist`): all of it for four months, then
/// `DRYING` of the month before for six months, then none for two.
fn moist(m: u64) -> f64 {
    match m % 12 {
        k if k < 4 => 1.0,
        k if k < 10 => DRYING.powi(k as i32 - 3),
        _ => 0.0,
    }
}

/// A tree stand (2 kg per m²) in a garden whose ground dries for eight months of each year at the same warmth
/// (`testkit::garden`, `moist`; `Cover::idle` a half at the start): cut into two and into four alike pieces at the
/// start of each month of two years, its month (`cover::live`) is the uncut stand's to `SAME`, the light it caught and
/// the seeds it sent with it: the months in season, the drying months in which its top leaves stop paying and it sheds
/// for its dry season (Kikuzawa 1991), the leafless months on its store and the month it leafs out again, whatever
/// slots hold it. It does lose all its leaves in its first dry season and grows them again (checked), so the cut holds
/// those laws.
#[test]
fn a_stand_cut_into_slots_keeps_its_seasons_as_the_whole() {
    let (mut w, i) = testkit::start();
    w.cfg.variety = cover::SLOTS;
    assert!(cover::place(&mut w, i, KINDS[TREE_ROW].genes, TREE_ROW as u32, 2.0));
    let p = cover::Parts::of(&w.chem).expect("cover parts");
    let (mm, dt) = (p.mm(&w.chem), w.cfg.calendar().month as f64);
    let mut c = w.life.cover(i).clone();
    c.idle = 0.5;
    let mut living = Vec::new();
    for m in 0..24u64 {
        let a = testkit::garden(&w, &c, (295.0, moist(m), 200.0, m));
        let month = |mut c: Cover| {
            let mut counts = w.counts.clone();
            let caught = cover::live(&w, &mut c, &p, &a, (i, dt), &mut counts).1;
            let sent: f64 = c.sent().iter().map(|s| cover::built_kg(&mm, &s.mol)).sum();
            let s = stand(&w, &c);
            (c, [s[0], s[1], s[2], s[3], sent, caught])
        };
        let (next, whole) = month(c.clone());
        for k in [2, 4] {
            let mut cut_c = c.clone();
            cut(&w, &mut cut_c, k);
            let cut = month(cut_c).1;
            println!("month {m:2}, cut in {k}: kg, plants, height m, seeds lying kg, sent kg, light J {cut:?}, uncut {whole:?}");
            assert_same(&format!("month {m}, cut in {k}"), &cut, &whole);
        }
        c = next;
        living.push(c.held().map(|x| c.living(&mm, x)).sum::<f64>());
    }
    let (wet, bare) = (living[3], living[4..12].iter().copied().fold(f64::INFINITY, f64::min));
    assert!(bare < 1e-3 * wet && living[12] > 0.5 * wet, "it kept its leaves through its dry season or grew none again: {living:?}");
}

/// The heaviest slot of cell `i` of world `w`: its slot, the height of its large plants, m, its plants, and its plants
/// by group (`Cover::sizes`).
fn tallest(w: &World, i: usize) -> (usize, f64, f64, cover::Sizes) {
    let p = cover::Parts::of(&w.chem).expect("cover parts");
    let (mm, rho, c) = (p.mm(&w.chem), w.chem.subs[p.sub[WALL]].density[0], w.life.cover(i));
    let x = c.held().max_by(|&a, &b| c.built(&mm, a).total_cmp(&c.built(&mm, b))).expect("a stand");
    (x, c.height(&w.cfg.phys, &mm, rho, x), c.count(x), c.sizes(&w.cfg.phys, &mm, x))
}

/// The large plants of the heaviest slot of cell `i` of world `w` after a month: their count and height, m.
fn large_after_a_month(mut w: World, i: usize) -> (f64, f64) {
    pcore::step(&mut w);
    let (_, tall, _, s) = tallest(&w, i);
    (s.count[s.large()], tall)
}

/// Nothing happens to a plant because smaller plants arrived (laws.md 3): a flush of seedlings (0.01 kg per m²) in a
/// tree stand's slot, or in a slot of another kind on its cell, leaves the stand's large plants as they were at the end
/// of their month, to `FLUSH` of their count and height. The seeds of its kind join the slot beside its large plants,
/// which stand as tall when they come, to `SEEDS_WALLS`.
#[test]
fn a_flush_of_seedlings_leaves_the_large_plants_as_they_were() {
    let (w, i) = grown(TREE_ROW, 5.0);
    let (x, tall, plants, sizes) = tallest(&w, i);
    let mut own = w.clone();
    assert!(cover::place(&mut own, i, KINDS[TREE_ROW].genes, TREE_ROW as u32, 0.01));
    let (y, now, joined, with) = tallest(&own, i);
    println!("height {tall} m, then {now} m; plants {plants}, then {joined}; groups {sizes:?}, then {with:?}");
    assert!(y == x && joined > 2.0 * plants && with.count[with.large()] == sizes.count[sizes.large()], "the seeds did not join the slot beside its large plants");
    assert!((now / tall - 1.0).abs() < SEEDS_WALLS, "the large plants stood {tall} m, then {now} m");
    let mut other = w.clone();
    let mut herb = KINDS[LIGHT_ROW].genes;
    herb[cover::FROST] -= 30.0;
    assert!(cover::place(&mut other, i, herb, LIGHT_ROW as u32, 0.01));
    let alone = large_after_a_month(w, i);
    for (what, flushed) in [("its own", own), ("another kind's", other)] {
        let after = large_after_a_month(flushed, i);
        println!("with {what} seedlings: large plants {after:?}, alone {alone:?}");
        assert!((after.0 / alone.0 - 1.0).abs() < FLUSH && (after.1 / alone.1 - 1.0).abs() < FLUSH, "{what} seedlings moved the large plants: {after:?}, alone {alone:?}");
    }
}

/// The walls the groups of slot `x` of cover `c` hold, kg, and the most share of walls of one (`Cover::sizes`).
fn group_walls(c: &Cover, mm: &[f64; cover::NP], ph: &pcore::Physics, x: usize) -> (f64, f64) {
    let s = c.sizes(ph, mm, x);
    let kg = (0..s.held).map(|g| s.count[g] * s.mass[g] * s.walls[g]).sum();
    (kg, s.walls[..s.held].iter().copied().fold(0.0, f64::max))
}

/// Through a year of a tree stand that seeds itself, every slot of the world keeps its groups whole at each month's
/// end: the held groups first, each of real plants, the smallest plants first, their shares of the built cover summing
/// to 1 (to the 32-bit rounding of the shares), its groups holding all its walls, none more than all of its plant, also
/// where it has shed nearly all its living cover; and the books are kept.
#[test]
fn a_slot_keeps_its_groups_whole() {
    let (mut w, _) = grown(TREE_ROW, 5.0);
    let books = testkit::Books::now(&w);
    let mm = cover::Parts::of(&w.chem).expect("cover parts").mm(&w.chem);
    let mut most = 0;
    for _ in 0..12 {
        pcore::step(&mut w);
        for c in (0..w.mesh.cells().n()).map(|j| w.life.cover(j)) {
            for x in c.held() {
                let s = c.sizes(&w.cfg.phys, &mm, x);
                let share: f64 = c.share[x].iter().sum();
                most = most.max(s.held);
                assert!(s.held > 0 && c.plants[x][s.held..].iter().all(|&n| n == 0.0), "{:?}", c.plants[x]);
                assert!(s.mass[..s.held].windows(2).all(|m| m[0] <= m[1]), "the groups are out of order: {s:?}");
                assert!((share - 1.0).abs() < 1e-12, "the shares sum to {share}: {:?}", c.share[x]);
                let mut shed = c.clone();
                shed.mol[x][cover::MATTER] *= SHED;
                for (c, what) in [(c, "as it is"), (&shed, "shed")] {
                    let ((held, most), walls) = (group_walls(c, &mm, &w.cfg.phys, x), c.walls(&mm, x));
                    assert!((held - walls).abs() <= 1e-9 * walls && most <= 1.0, "{what}: its groups hold {held} kg of its {walls} kg of walls, one {most} of its plant");
                }
            }
        }
    }
    assert!(most > 1, "no slot held more than one group");
    books.assert_kept(&w, 1e-9);
}
