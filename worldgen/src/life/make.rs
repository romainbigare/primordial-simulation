//! One founder body: its genes from its kind's row, its stores, its temperature, booked as new matter.

use super::kinds::{light, KINDS};
use super::book_body;
use pcore::body::{self, Body, Layout};
use pcore::rng::{Purpose, Rng};
use pcore::World;

/// log10 of how much faster the open world burns the energy-rich solid, and body matter, than when the founders'
/// `cat` numbers were set (four times: litter constants of 0.27 and 0.2 a year): a founder's burning genes are that
/// much lower, so its burning and the cost of its enzymes (both go with k x 10^gene) stay what they were.
pub const LITTER_X: f32 = 0.602;
/// log10 of how much more light catalyst a founder light-eater needs under the smooth rate of Law 2: the reaction's
/// contact is the body's liquid, not the gas it exchanges in its interval, and a founder (2 kg, a 20-day interval)
/// exchanges 182 times its liquid. Its gene of the light reaction is that much higher, so its own light reaction
/// goes as before.
pub const LIGHT_X: f32 = 2.26;
/// The share of each solid bodies can build besides their matter (`Chemistry::builds`) in what every founder builds
/// and holds (its keep gene of the solid, `Body::share`): the same for every kind, small so that selection can find
/// what each is worth.
const BUILT_SHARE: f32 = 0.02;
/// A gene that is off starts below zero, so small changes keep it off: catalysts and the rest.
const CAT_OFF: f32 = -1.0;
const GENE_OFF: f32 = -0.1;
/// What a founder keeps, kg per kg of its body (`Body::level`): the liquid (also its share of the body,
/// `Body::size`), the energy-rich solid (or what it is placed with and `KEEP_MARGIN` more, if more), every other store;
/// and the minimums, the floor growth and giving keep (`Body::burns`): about three updates of upkeep of the solid.
pub(super) const KEEP_LIQUID: f32 = 0.75;
const KEEP_SOLID: f32 = 0.9;
const KEEP_MARGIN: f64 = 1.1;
const MIN_LIQUID: f32 = 0.3;
const MIN_SOLID: f32 = 0.25;
/// A founder keeps up to this many times the minerals its matter was made of, per kg of its body.
const MINERAL_KEEP: f64 = 2.0;
/// The largest share of a body that is liquid (as `Body::size` bounds the keep gene).
const MAX_LIQUID_SHARE: f64 = 0.95;

/// How a placed member's mass is shared: (fuel, liquid, body matter), shares of the mass, for a fuel share `store` and
/// the keep gene `h` of the liquid. The body (the rest) is liquid in the share `h` and matter in 1 - `h`
/// (`Body::size`: matter over 1 - `h`). Without body matter in the chemistry the rest is liquid.
pub(super) fn split(store: f64, h: f64, matter: bool) -> (f64, f64, f64) {
    let body = 1.0 - store;
    if !matter {
        return (store, body, 0.0);
    }
    let liquid = h.clamp(0.0, MAX_LIQUID_SHARE) * body;
    (store, liquid, body - liquid)
}

/// A body of starting kind `which` in cell `i`, of `lineage`, born with brain rows `genome`, in a new group of it, at
/// the cell's temperature; its matter and energy are booked like an edit. Not yet placed.
pub fn make(w: &mut World, which: usize, i: usize, lineage: u32, genome: u32) -> Option<Body> {
    make_with(w, which, i, lineage, genome, 0, 1.0, 0)
}

/// The genes of a founder of `which` before any mutation (`None` without the roles of life in the chemistry).
fn genes(w: &World, which: usize) -> Option<Vec<f32>> {
    let ((r_light, rich, liquid, burn), kind, chem) = (w.chem.life_roles()?, KINDS.get(which)?, &w.chem);
    let lay = Layout::of(chem);
    let mut genes = vec![GENE_OFF; lay.len];
    genes[lay.cat..lay.min].fill(CAT_OFF);
    genes[..body::NSCALAR].copy_from_slice(&kind.scalars);
    let lit = if light(kind) { kind.cat[0] + LIGHT_X } else { kind.cat[0] };
    (genes[lay.cat + r_light], genes[lay.cat + burn]) = (lit, kind.cat[1] - LITTER_X);
    (genes[lay.min + liquid], genes[lay.min + rich]) = (MIN_LIQUID, MIN_SOLID);
    (genes[lay.upt + liquid], genes[lay.upt + rich]) = (kind.upt[0], kind.upt[1]);
    // Animals digest body matter as they digest the solid, and take it: light-eaters do not.
    if let Some((m, r)) = chem.matter_roles().filter(|_| !light(kind)) {
        (genes[lay.cat + r], genes[lay.upt + m]) = (kind.cat[1] - LITTER_X, kind.upt[1]);
    }
    genes[lay.keep..].fill(KEEP_SOLID);
    genes[lay.keep + liquid] = KEEP_LIQUID;
    genes[lay.keep + rich] = KEEP_SOLID.max((KEEP_MARGIN * kind.store / (1.0 - kind.store)) as f32);
    chem.materials().for_each(|k| genes[lay.keep + k] = BUILT_SHARE);
    Some(genes)
}

/// `make` after `rounds` rounds of mutation of its genes, `gmul` times a generation's step (a random walk keyed by
/// `id`).
#[allow(clippy::too_many_arguments)]
pub(super) fn make_with(w: &mut World, which: usize, i: usize, lineage: u32, genome: u32, rounds: u32, gmul: f32, id: u64) -> Option<Body> {
    let ((_, rich, liquid, _), kind, mut genes) = (w.chem.life_roles()?, KINDS.get(which)?, genes(w, which)?);
    let group = w.life.new_group(lineage, w.life.genomes[genome as usize].rows.clone());
    let (chem, ledger, t) = (&w.chem, &mut w.ledger, w.life.env[i][0]);
    let (lay, ns) = (Layout::of(chem), chem.ns());
    let mut rg = Rng::new(w.seed, w.day, id, Purpose::Make);
    // Its mutation size is raised by `gmul` for the rounds (and taken back, whatever the walk made of it).
    genes[body::MUT] += gmul.log10();
    for _ in 0..rounds {
        genes = pcore::body::mutate(&lay, &genes, &mut rg);
    }
    genes[body::MUT] -= gmul.log10();
    // A founder is an adult of its kind: its body (`Body::built`, its matter at the liquid share of living tissue) is
    // its copy mass, from its own genes after the spray, and fuel the share `store` of its mass on top; the liquid it
    // keeps with it in the share of its keep gene. Without body matter the whole mass is the body.
    let matter = chem.matter_roles();
    let h = genes[lay.keep + liquid].clamp(0.0, MAX_LIQUID_SHARE as f32) as f64;
    let mass = 10f64.powf(genes[body::COPY_MASS] as f64) * if matter.is_some() { (1.0 - body::WET_TISSUE) / (1.0 - h) / (1.0 - kind.store) } else { 1.0 };
    let count = if kind.cover > 0.0 { kind.cover * w.grid.area() / mass } else { 1.0 };
    let mut store = vec![0.0; ns];
    let (fuel, liq, mat) = split(kind.store, genes[lay.keep + liquid] as f64, matter.is_some());
    store[rich] = fuel * count * mass / chem.subs[rich].molar_mass;
    store[liquid] = liq * count * mass / chem.subs[liquid].molar_mass;
    // The minerals growth also needs: a member is placed holding what its matter was made of, so it can build a
    // matter's worth again; it keeps up to `MINERAL_KEEP` times that per kg of its body (the matter over 1 - h), and a
    // light-eater takes them from the ground (an animal has them from the matter it digests).
    if let Some((m, _)) = matter {
        store[m] = mat * count * mass / chem.subs[m].molar_mass;
        for (k, per) in chem.minerals() {
            store[k] = store[m] * per;
            genes[lay.keep + k] = (MINERAL_KEEP * per * chem.subs[k].molar_mass / chem.subs[m].molar_mass * (1.0 - h)) as f32;
            if light(kind) {
                genes[lay.upt + k] = 1.0;
            }
        }
    }
    let mut b = Body { cell: i as u32, count, lineage, group, near: u32::MAX, mark: w.light[i], store, genes, genome, ..Default::default() };
    // It holds what its shares make of its matter (`Body::level`).
    chem.materials().for_each(|k| b.store[k] = b.level(chem, k, 0.0));
    b.e = b.heat_cap(chem) * t;
    book_body(ledger, chem, &b);
    // Its key liquids first: `visible` reads the body (`Body::size`) through them.
    b.wet = b.key_liquids(chem);
    b.vis = b.visible(chem, &w.cfg.phys);
    Some(b)
}
