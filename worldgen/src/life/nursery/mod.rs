//! The nursery: the cover a world starts with, sorted by its own law on the world's own year before it is placed
//! (laws.md 8). A copy of the world runs a year without life (`Year`); its open land falls in classes of like climate
//! (`year::classes`), and each class's representative cell is a garden (`garden`) where kinds race on bare ground and
//! then try to enter the grown stand as seeds (`rounds`): first draws over the genes' whole range, then also the other
//! classes' champions. A second year is kept under the cover the first sorting grew, as the cover cools its own ground,
//! and the stands go on there. Each class's cells then get the kinds its stand holds, at the mass they hold there, each
//! with a generation's scatter of its genes, and the cells between two classes the other class's heaviest kind as rare
//! seeds. A fixed amount of work, keyed by the world's seed: the same world on every machine.

mod garden;
mod rounds;
mod year;

use super::open_land;
use pcore::cover::{self, Parts, APART, NG};
use pcore::rng::{Purpose, Rng};
use pcore::World;
use rounds::{Class, Entrant, Lived};
use year::{Month, Year};

/// Classes of like climate the open land falls in, at most: each class's garden is one cell's year.
pub const CLASSES: usize = 24;
/// Rounds of the race on bare ground among a class's own draws, then with a guest from another class each, and the
/// years of each round.
pub const RACE_TRIALS: u32 = 24;
pub const GUEST_TRIALS: u32 = 12;
pub const RACE_YEARS: u32 = 12;
/// Kinds of a race that go on to the next round and grow into the stand.
pub const CHAMPIONS: usize = 3;
/// Years the champions grow as a stand before newcomers come.
pub const GROW_YEARS: u32 = 10;
/// Rounds of the invasion on each year kept, and the years of each.
pub const INVADE_ROUNDS: u32 = 24;
pub const INVADE_YEARS: u32 = 4;
/// Cover each kind of a race starts with, kg per m².
pub const START_KG: f64 = 0.1;
/// Seeds a newcomer arrives with in a grown stand, kg per m².
pub const ARRIVE_KG: f64 = 0.005;
/// Generations of mutation (`cover::mutate`) of a mutant newcomer.
pub const MUTANT_GENERATIONS: u32 = 5;
/// A cell lies between two classes where its climate is nearer another class's representative than this many times
/// its own's distance; it gets that class's heaviest kind as seeds of `RARE_KG` kg per m².
pub const BETWEEN: f64 = 1.5;
pub const RARE_KG: f64 = 0.001;

/// Per month of a cell's year, per mineral: the share of it that stays, and what comes in, mol (`wash`).
type Wash = Vec<Vec<(usize, f64, f64)>>;

/// The minerals' change of each month of `months` (a cell's year without life, which only the rain and heat change),
/// from `before`: per mineral the share of what the month before held that stays, or what comes in where it gains.
fn wash(chem: &pcore::chem::Chemistry, before: &[f64; pcore::units::MAX_SUBSTANCES], months: &[Month]) -> Wash {
    let mut prev = *before;
    months
        .iter()
        .map(|m| {
            let out = chem.minerals().iter().map(|&(k, _)| {
                let (was, now) = (prev[k], m.around.ground[k]);
                if now >= was { (k, 1.0, now - was) } else { (k, now / was.max(1e-300), 0.0) }
            });
            let out = out.collect();
            prev = m.around.ground;
            out
        })
        .collect()
}

/// The fixing gene the nursery tries each known kind with (`cover::FIX`), log10 mol per kg of living cover per s:
/// 10^-8.3, about 10 g of the fixed element per m² a year for a kg of cover per m² (2 kg living), what real fixing stands
/// fix (Cleveland et al. 1999).
pub const FIX_GENE: f32 = -8.3;

/// The kinds known before (the hand-made kinds of cover, `Place::Field`), each as it is and with the fixing gene on
/// (`FIX_GENE`), labelled by its row: the first newcomers of every class's race, beside the draws.
fn known() -> Vec<Entrant> {
    let cover = super::KINDS.iter().enumerate().filter_map(|(k, kind)| match &kind.place {
        super::Place::Field(f) => Some((f.genes, k as u32)),
        super::Place::Bodies(_) => None,
    });
    cover
        .flat_map(|(genes, root)| {
            let mut fixer = genes;
            fixer[cover::FIX] = FIX_GENE;
            [Entrant { genes, root, raced: true }, Entrant { genes: fixer, root, raced: true }]
        })
        .collect()
}

/// Each class's heaviest kind of its stand, or its first champion before it grew one.
fn heaviest(w: &World, p: &Parts, gs: &[Class]) -> Vec<Option<Entrant>> {
    gs.iter().map(|g| g.set(w, p).first().map(|e| e.0).or(g.entrants.first().copied())).collect()
}

/// `f` on each class with what it lives on (`Lived`) and its guests: the other classes' heaviest kinds as they stood
/// before, the nearest climate first (`near`); in parallel, each class its own.
fn each(w: &World, p: &Parts, gs: &mut [Class], (near, lived): (&[Vec<usize>], &[Lived]), f: impl Fn(&mut Class, Lived, &[Entrant]) + Sync + Send) {
    let first = heaviest(w, p, gs);
    let mut items: Vec<(usize, &mut Class)> = gs.iter_mut().enumerate().collect();
    pcore::par::each_init(&mut items, near.len(), || (), |_, (k, g)| {
        let guests: Vec<Entrant> = near[*k].iter().filter_map(|&c| first[c]).collect();
        f(g, lived[*k], &guests);
    });
}

/// Sort the cover of `w` (at its life's start, its soil placed) in the nursery and place it. Returns the line to print.
pub fn start(w: &mut World) -> String {
    let Some(p) = Parts::of(&w.chem) else { return String::new() };
    let land: Vec<usize> = (0..w.mesh.cells().n()).filter(|&i| open_land(w, i)).collect();
    let months = w.cfg.months_per_year;
    // The first year: without life.
    let before: Vec<_> = land.iter().map(|&i| cover::around_now(w, &p, i).ground).collect();
    let first = Year::of(w, &p, &land, months);
    let features = first.features(w, &p);
    let classes = year::classes(&features, CLASSES);
    let near = year::neighbours(&features, &classes);
    let washes: Vec<Wash> = classes.iter().map(|c| wash(&w.chem, &before[c[0]], &first.months[c[0]])).collect();
    let mut gardens: Vec<Class> = classes.iter().enumerate().map(|(k, c)| Class::new(k, first.cells[c[0]])).collect();
    let lived = |y: &Year| -> Vec<Vec<Month>> { classes.iter().map(|c| y.months[c[0]].clone()).collect() };
    let one = lived(&first);
    let on: Vec<Lived> = one.iter().zip(&washes).map(|(m, x)| (m.as_slice(), x.as_slice())).collect();
    let w0: &World = w;
    let known = known();
    each(w0, &p, &mut gardens, (&near, &on), |g, l, _| g.race(w0, &p, l, (&known, &[]), RACE_TRIALS));
    each(w0, &p, &mut gardens, (&near, &on), |g, l, guests| {
        g.race(w0, &p, l, (&[], guests), GUEST_TRIALS);
        g.grow(w0, &p, l);
    });
    each(w0, &p, &mut gardens, (&near, &on), |g, l, guests| g.invade(w0, &p, l, guests, (0, INVADE_ROUNDS)));
    // The second year: under the cover the first sorting grew.
    let mut covered = w.clone();
    place(&mut covered, &p, (&classes, &first.cells, &features), &gardens);
    let second = Year::of(&covered, &p, &land, months);
    drop(covered);
    // Its liquid stays the year's without life (`Month::keep`, the rain), which the gardens' own covers draw on: the
    // copy's covered ground already lost what its cover took. Of it the share liquid is the covered year's.
    let mut two = lived(&second);
    for (t, o) in two.iter_mut().flatten().zip(one.iter().flatten()) {
        let share = if t.held > 0.0 { t.around.liquid / t.held } else { 0.0 };
        (t.held, t.rain, t.keep, t.around.liquid) = (o.held, o.rain, o.keep, o.held * share);
    }
    let on: Vec<Lived> = two.iter().zip(&washes).map(|(m, x)| (m.as_slice(), x.as_slice())).collect();
    let w0: &World = w;
    each(w0, &p, &mut gardens, (&near, &on), |g, l, guests| g.invade(w0, &p, l, guests, (INVADE_ROUNDS, INVADE_ROUNDS)));
    let placed = place(w, &p, (&classes, &first.cells, &features), &gardens);
    line(w, &p, &gardens, placed)
}

/// What the nursery placed: cells, kg, and the starting kinds (`Cover::root`) it labelled the kinds with.
struct Placed {
    cells: usize,
    kg: f64,
    roots: usize,
}

/// The first starting kind (`Cover::root`) of the nursery's kinds: past the rows of the hand-made kinds (`KINDS`).
const FIRST_ROOT: u32 = super::KINDS.len() as u32;

/// Place the kinds each class's stand holds (`Class::set`) on its cells (`classes`: places in `cells`), each at the
/// mass per m² it holds there with a generation's scatter of its genes (`cover::mutate`, keyed by the cell); then, on
/// each cell whose climate (`features`) lies `BETWEEN` it and another class, that class's heaviest kind as `RARE_KG`
/// of seeds. Kinds within `APART` × `alike` of each other (`cover::distance`) share a starting kind, numbered from
/// `FIRST_ROOT` in the order of the classes and their kinds.
fn place(w: &mut World, p: &Parts, (classes, cells, features): (&[Vec<usize>], &[usize], &[year::Features]), gardens: &[Class]) -> Placed {
    let alike = APART * w.cfg.phys.alike;
    let sets: Vec<Vec<(Entrant, f64)>> = gardens.iter().map(|g| g.set(w, p)).collect();
    let mut roots: Vec<[f32; NG]> = Vec::new();
    let mut root_of = |g: &[f32; NG]| match roots.iter().position(|r| cover::distance(r, g) <= alike) {
        Some(r) => FIRST_ROOT + r as u32,
        None => {
            roots.push(*g);
            FIRST_ROOT + roots.len() as u32 - 1
        }
    };
    let labelled: Vec<Vec<([f32; NG], u32, f64)>> = sets.iter().map(|s| s.iter().map(|(e, kg)| (e.genes, root_of(&e.genes), *kg)).collect()).collect();
    let (mut out, cover_mut) = (Placed { cells: 0, kg: 0.0, roots: roots.len() }, w.cfg.phys.cover_mut);
    let spread = year::spread(features);
    for (k, members) in classes.iter().enumerate() {
        for &m in members {
            let i = cells[m];
            let mut r = Rng::new(w.seed, k as u64, i as u64, Purpose::Nursery);
            let mut any = false;
            for &(genes, root, kg) in &labelled[k] {
                if cover::place(w, i, cover::mutate(&genes, cover_mut, &mut r), root, kg) {
                    (any, out.kg) = (true, out.kg + kg * w.mesh.cells().area()[i]);
                }
            }
            out.cells += any as usize;
            let d = |c: usize| year::distance(&features[m], &features[classes[c][0]], &spread);
            let own = d(k);
            if let Some(o) = (0..classes.len()).filter(|&c| c != k && d(c) <= BETWEEN * BETWEEN * own).min_by(|&a, &b| d(a).total_cmp(&d(b))) {
                if let Some(&(genes, root, _)) = labelled[o].first() {
                    cover::place(w, i, cover::mutate(&genes, cover_mut, &mut r), root, RARE_KG);
                }
            }
        }
    }
    out
}

/// The line that reports the nursery: its classes and their stands, what came by the race and as seeds, and what it
/// placed.
fn line(w: &World, p: &Parts, gardens: &[Class], placed: Placed) -> String {
    let sets: Vec<Vec<(Entrant, f64)>> = gardens.iter().map(|g| g.set(w, p)).collect();
    let kinds: usize = sets.iter().map(Vec::len).sum();
    let raced: usize = sets.iter().flatten().filter(|e| e.0.raced).count();
    let (came, held): (u32, u32) = gardens.iter().fold((0, 0), |a, g| (a.0 + g.came, a.1 + g.held));
    let mut per = vec![0; cover::SLOTS + 1];
    sets.iter().for_each(|s| per[s.len()] += 1);
    let frost: Vec<String> = sets.iter().map(|s| s.first().map_or("-".into(), |e| format!("{:.0}", e.0.genes[cover::FROST]))).collect();
    format!(
        "year {:.1}: the nursery sorted the cover in {} classes (kinds per stand, 0 to {}: {per:?}): {kinds} kinds hold the stands, {raced} by the race and {} as seeds into a grown stand ({held} of {came} newcomers held); placed {:.3e} kg on {} cells as {} starting kinds; heaviest kind's frost gene by class {}",
        w.year(),
        gardens.len(),
        cover::SLOTS,
        kinds - raced,
        placed.kg,
        placed.cells,
        placed.roots,
        frost.join(" ")
    )
}
