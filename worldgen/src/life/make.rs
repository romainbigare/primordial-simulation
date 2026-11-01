//! One founder body: its genes from its kind's row, its stores, its temperature, booked as new matter.

use super::kinds::{Bodies, Place, KINDS};
use super::book_body;
use pcore::body::{self, Body, Genes, Layout};
use pcore::rng::{Purpose, Rng};
use pcore::World;

/// log10 of how much faster the open world burns the energy-rich solid and body matter than the founders' `cat`
/// numbers assume (litter constants of 0.27 and 0.2 a year): their burning genes are that much lower.
pub const LITTER_X: f32 = 0.602;
/// Every founder's keep gene of each solid bodies build besides their matter: small, so selection finds its worth.
const BUILT_SHARE: f32 = 0.02;
/// A gene that is off starts below zero, so small changes keep it off: catalysts and the rest.
const CAT_OFF: f32 = -1.0;
const GENE_OFF: f32 = -0.1;
/// What a founder keeps, kg per kg of its body: the liquid, the energy-rich solid (or what it is placed with and
/// `KEEP_MARGIN` more), every other store; and the minimums, about three updates of upkeep of the solid.
pub(super) const KEEP_LIQUID: f32 = 0.75;
const KEEP_SOLID: f32 = 0.9;
const KEEP_MARGIN: f64 = 1.1;
const MIN_LIQUID: f32 = 0.3;
const MIN_SOLID: f32 = 0.25;
/// A founder keeps up to this many times the minerals its matter was made of, per kg of its body.
const MINERAL_KEEP: f64 = 2.0;
/// The largest share of a body that is liquid (as `Body::size` bounds the keep gene).
const MAX_LIQUID_SHARE: f64 = 0.95;

/// How a placed member's mass is shared, (fuel, liquid, body matter), for a fuel share `store` and the liquid's keep
/// gene `h`: the body is liquid in the share `h` and matter in 1 - `h` (all liquid without body matter).
pub(super) fn split(store: f64, h: f64, matter: bool) -> (f64, f64, f64) {
    let body = 1.0 - store;
    if !matter {
        return (store, body, 0.0);
    }
    let liquid = h.clamp(0.0, MAX_LIQUID_SHARE) * body;
    (store, liquid, body - liquid)
}

/// A body of starting kind `which` in cell `i`, of `lineage`, born with brain rows `genome` (its own rows those), at
/// the cell's temperature; its matter and energy are booked like an edit. Not yet placed.
pub fn make(w: &mut World, which: usize, i: usize, lineage: u32, genome: u32) -> Option<Body> {
    make_with(w, which, i, lineage, genome, 0, 0.0, 0)
}

/// The bodies of starting kind `which`, if it places bodies.
fn bodies(which: usize) -> Option<&'static Bodies> {
    match &KINDS.get(which)?.place {
        Place::Bodies(b) => Some(b),
        Place::Field(_) => None,
    }
}

/// The genes of a founder of `which` before any mutation (`None` without the roles of life in the chemistry, or for a
/// kind that places cover).
fn genes(w: &World, which: usize) -> Option<Genes> {
    let ((r_light, rich, liquid, burn), kind, chem) = (w.chem.life_roles()?, bodies(which)?, &w.chem);
    let (lay, bc) = (Layout::of(chem), chem.body());
    let (at, diet) = (|k: usize| bc.place(k).expect("life's roles are stores"), |r: usize| lay.diet(chem, r).expect("bodies may run life's reactions"));
    let mut genes = Genes::filled(GENE_OFF, lay.len);
    genes[lay.cat..lay.min].fill(CAT_OFF);
    genes[..body::NSCALAR].copy_from_slice(&kind.scalars);
    (genes[diet(r_light)], genes[diet(burn)]) = (kind.cat[0], kind.cat[1] - LITTER_X);
    (genes[lay.min + at(liquid)], genes[lay.min + at(rich)]) = (MIN_LIQUID, MIN_SOLID);
    (genes[lay.upt + at(liquid)], genes[lay.upt + at(rich)]) = (kind.upt[0], kind.upt[1]);
    // They digest body matter as they digest the solid, and take it.
    if let Some((m, r)) = chem.matter_roles() {
        (genes[diet(r)], genes[lay.upt + at(m)]) = (kind.cat[1] - LITTER_X, kind.upt[1]);
    }
    genes[lay.keep..].fill(KEEP_SOLID);
    genes[lay.keep + at(liquid)] = KEEP_LIQUID;
    genes[lay.keep + at(rich)] = KEEP_SOLID.max((KEEP_MARGIN * kind.store / (1.0 - kind.store)) as f32);
    chem.materials().for_each(|k| genes[lay.keep + at(k)] = BUILT_SHARE);
    Some(genes)
}

/// `make` after `rounds` rounds of mutation of its genes, the mutation size raised by `spray_decades` decades (10^that
/// times a generation's step; a random walk keyed by `id`).
#[allow(clippy::too_many_arguments)]
pub(super) fn make_with(w: &mut World, which: usize, i: usize, lineage: u32, genome: u32, rounds: u32, spray_decades: f32, id: u64) -> Option<Body> {
    let ((_, rich, liquid, _), kind, mut genes) = (w.chem.life_roles()?, bodies(which)?, genes(w, which)?);

    let mut rg = Rng::new(w.seed, w.day(), id, Purpose::Make);
    let (chem, t) = (&w.chem, w.life.env[i][0]);
    let (lay, ns) = (Layout::of(chem), chem.ns());
    // Its mutation size is raised by `spray_decades` for the rounds (and taken back, whatever the walk made of it).
    genes[body::MUT] += spray_decades;
    let brain = w.life.brains[lineage as usize].as_ref().is_some_and(|l| !l.none());
    for _ in 0..rounds {
        genes = body::vary(&lay, &genes, &[], 1.0, 0.0, brain, &mut rg).0;
    }
    genes[body::MUT] -= spray_decades;
    // A founder is an adult: its living body is its copy mass after the spray, with the share `store` of fuel on top.
    let matter = chem.matter_roles();
    let at = |k: usize| chem.body().place(k).expect("life's roles are stores");
    let h = genes[lay.keep + at(liquid)].clamp(0.0, MAX_LIQUID_SHARE as f32) as f64;
    let mass = 10f64.powf(genes[body::COPY_MASS] as f64) * if matter.is_some() { (1.0 - body::WET_TISSUE) / (1.0 - h) / (1.0 - kind.store) } else { 1.0 };
    let count = 1.0;
    let mut store = vec![0.0; ns];
    let (fuel, liq, mat) = split(kind.store, genes[lay.keep + at(liquid)] as f64, matter.is_some());
    store[rich] = fuel * count * mass / chem.subs[rich].molar_mass;
    store[liquid] = liq * count * mass / chem.subs[liquid].molar_mass;
    // A member holds the minerals its matter was made of, so it can build that again, and keeps up to `MINERAL_KEEP`
    // times that per kg of body.
    if let Some((m, _)) = matter {
        store[m] = mat * count * mass / chem.subs[m].molar_mass;
        for &(k, per) in chem.minerals() {
            store[k] = store[m] * per;
            genes[lay.keep + at(k)] = (MINERAL_KEEP * per * chem.subs[k].molar_mass / chem.subs[m].molar_mass * (1.0 - h)) as f32;
        }
    }
    let mut b = Body { cell: i as u32, count, lineage, store: chem.body().gather(|k| store[k]), genes, genome, ..Default::default() };
    b.prepare(chem, &w.cfg);
    // It holds what its shares make of its matter (`Body::level`).
    for s in chem.materials().filter_map(|k| chem.body().place(k)) {
        b.store[s] = b.level(chem, s, 0.0);
    }
    // Its key liquids first: its heat holds their fusion enthalpy, and `visible` reads the body (`Body::size`) through
    // them.
    b.wet = b.key_liquids(chem, t);
    b.e = b.energy_at(chem, t);
    b.vis = b.visible(chem, &w.cfg);
    book_body(w, &b);
    Some(b)
}
