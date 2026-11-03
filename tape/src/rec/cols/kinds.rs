//! Each starting kind (`root_<k>_*`, by `Lineage::root`): its members, bodies and mass, its births and deaths of
//! the year, its lives, and the means by members of its mass, body, genes and catalysts; and for a kind of cover
//! (`cover::Cover::root`) its slots, mass and the means of its genes by built cover. A kind that died out reads 0 (its
//! means NaN); the columns are there for every kind up to the largest seen so far.

use super::{ratio, Row};
use crate::layers::Ctx;
use pcore::par;
use crate::rec::{Recorder, CAUSES};
use pcore::body::{self, Body};
use pcore::cover;
use pcore::World;
use std::collections::BTreeSet;

/// The genes averaged per kind: gene, column name and unit.
const GENES: [(usize, &str, &str); 5] = [
    (body::COPY_MASS, "copy_log", "log10"),
    (body::HIDDEN, "hidden", "1"),
    (body::HEAT, "heat", "log10"),
    (body::KEEP_T, "keep_t", "K"),
    (body::REPAIR, "repair", "log10"),
];
/// The genes of the cover averaged per kind: gene, column name and unit.
const COVER_GENES: [(usize, &str, &str); 9] = [
    (cover::LIGHT, "light", "log10"),
    (cover::LEAF, "leaf", "log10"),
    (cover::WALLS, "walls", "log10"),
    (cover::STORE, "store", "log10"),
    (cover::FROST, "frost", "K"),
    (cover::MOIST, "moist", "log10"),
    (cover::SEED, "seed", "1"),
    (cover::REPAIR, "repair", "log10"),
    (cover::FIX, "fix", "log10"),
];
/// A member below this activity (`Body::activity`) counts as dry.
const DRY: f64 = 0.5;

/// Sums over the live members of one starting kind: counts, life, masses and brood; by members the genes and the
/// activity; per catalyst gene its sum over the members that carry it on, and those members.
#[derive(Clone)]
struct Kind {
    members: f64,
    bodies: f64,
    life: f64,
    mass: f64,
    matter: f64,
    body: f64,
    copy_mass: f64,
    brood: f64,
    alloc: f64,
    child: f64,
    act: f64,
    dry: f64,
    wet: f64,
    genes: [f64; GENES.len()],
    cat: Vec<(f64, f64)>,
}

impl Kind {
    fn new(nr: usize) -> Self {
        Kind { members: 0.0, bodies: 0.0, life: 0.0, mass: 0.0, matter: 0.0, body: 0.0, copy_mass: 0.0, brood: 0.0, alloc: 0.0, child: 0.0, act: 0.0, dry: 0.0, wet: 0.0, genes: [0.0; GENES.len()], cat: vec![(0.0, 0.0); nr] }
    }

    /// Add body `b` (of the kind), with what is costly to work out for it (`Heavy`).
    fn add(&mut self, w: &World, b: &Body, h: &Heavy) {
        let (chem, n) = (&w.chem, b.n());
        let matter_mm = chem.matter().map(|m| (m, chem.subs[m].molar_mass));
        self.members += n;
        self.bodies += 1.0;
        self.life += n * h.life;
        self.mass += h.mass;
        self.matter += matter_mm.map_or(0.0, |(m, mm)| b.amount(chem.body(), m) * mm);
        self.body += h.size;
        self.copy_mass += n * b.k.copy_mass;
        (self.alloc, self.child) = (self.alloc + n * b.gene(body::ALLOC), self.child + n * b.gene(body::CHILD));
        self.brood += h.brooded * matter_mm.map_or(0.0, |(_, mm)| mm);
        let a = h.activity;
        (self.act, self.dry, self.wet) = (self.act + n * a, self.dry + n * (a < DRY) as u8 as f64, self.wet + n * b.gene(body::WET));
        for (x, &(g, ..)) in self.genes.iter_mut().zip(&GENES) {
            *x += n * b.gene(g);
        }
        for (r, x) in self.cat.iter_mut().enumerate() {
            if let Some(g) = b.diet(chem, r).filter(|&g| g >= 0.0) {
                (x.0, x.1) = (x.0 + n * g as f64, x.1 + n);
            }
        }
    }
}

/// What a body adds to its kind that is costly to work out: the years a member lives (`life_years`), its mass, its
/// size, its brood and its activity.
struct Heavy {
    life: f64,
    mass: f64,
    size: f64,
    brooded: f64,
    activity: f64,
}

impl Heavy {
    fn of(c: &Ctx, q: usize, b: &Body) -> Heavy {
        let (w, chem) = (c.w, &c.w.chem);
        Heavy { life: life_years(w, b, c.mass[q]), mass: c.mass[q], size: b.size(chem), brooded: b.brooded(chem), activity: b.activity(chem, &w.cfg.phys) }
    }
}

/// Years a member of `b` (of mass `m`, kg) lives at its upkeep now while active: `e_life` × 10^`REPAIR` per kg of living
/// body over its upkeep at an activity of 1 (a dormant body pays nearly nothing, so its life would be unbounded).
fn life_years(w: &World, b: &Body, m: f64) -> f64 {
    let d = b.ahead(w);
    w.cfg.phys.e_life * b.dec(body::REPAIR) * b.built(&w.chem).max(1e-9 * m) * d / body::upkeep(w, b, d, m, b.temp(&w.chem), 1.0).max(1e-300) / w.days_per_year() as f64
}

impl Recorder {
    pub(super) fn kind_cols(&self, c: &Ctx, row: &mut Row) {
        let (w, y) = (c.w, &self.year);
        let nr = w.chem.reactions.len();
        let seen = self.files.series.iter().filter(|c| c.name.starts_with("root_") && c.name.ends_with("_members")).count();
        let mut kinds = vec![Kind::new(nr); seen.max(y.kinds.len())];
        let bodies: Vec<&Body> = w.life.alive().collect();
        let heavy = par::map_n(bodies.len(), |q| Heavy::of(c, q, bodies[q]));
        for (b, h) in bodies.iter().zip(&heavy) {
            if let Some(l) = &w.life.brains[b.lineage as usize] {
                let k = l.root as usize;
                if k >= kinds.len() {
                    kinds.resize(k + 1, Kind::new(nr));
                }
                kinds[k].add(w, b, h);
            }
        }
        // The cover of each kind of cover seen so far: its slots, mass, capture area per m² of the world's land, and its
        // genes' means by built cover.
        let sums = cover::sums(w);
        let seen = self.files.series.iter().filter(|c| c.name.starts_with("root_") && c.name.ends_with("_slots")).count();
        let land_area: f64 = (0..w.mesh.cells().n()).filter(|&i| w.is_land(i)).map(|i| w.mesh.cells().area()[i]).sum::<f64>().max(f64::MIN_POSITIVE);
        for k in 0..sums.len().max(seen) {
            let s = sums.get(k).cloned().unwrap_or_default();
            let n = |x: &str| format!("root_{k}_{x}");
            row.push(n("slots"), "1", s.slots);
            row.push(n("cover_kg"), "kg", s.kg);
            row.push(n("cover_area"), "m2/m2", s.leaf / land_area);
            for (&(g, name, unit), &x) in COVER_GENES.iter().zip(&s.genes) {
                debug_assert!(g < cover::NG);
                row.push(n(&format!("cover_{name}")), unit, ratio(x, s.built));
            }
        }
        // The catalyst genes on in some member of the kind so far keep their columns.
        let ever: BTreeSet<(usize, usize)> = self.files.series.iter().filter_map(|c| c.name.strip_prefix("root_")?.split_once("_cat_")).filter_map(|(k, r)| Some((k.parse().ok()?, r.parse().ok()?))).collect();
        for (k, v) in kinds.iter().enumerate() {
            let per = |x: f64| ratio(x, v.members);
            let year = y.kinds.get(k).copied().unwrap_or_default();
            let stand = y.stand.get(k).copied().unwrap_or(0.0) / y.months.max(1.0);
            let c = |n: &str| format!("root_{k}_{n}");
            row.push(c("members"), "1", v.members);
            row.push(c("kg"), "kg", v.mass);
            row.push(c("bodies"), "1", v.bodies);
            // The life the genes allow at the upkeep now, and the life lived: the standing members over the deaths.
            row.push(c("life"), "yr", per(v.life));
            row.push(c("life_real"), "yr", ratio(stand, year.deaths.iter().sum::<f64>()));
            row.push(c("births"), "1/yr", year.births);
            // The share of the year's births with two parents (a partner alike in the cell).
            row.push(c("mixed"), "1", ratio(year.mixed, year.births));
            for (cause, &x) in CAUSES.iter().zip(&year.deaths) {
                row.push(c(&format!("died_{cause}")), "1/yr", x);
            }
            row.push(c("mass"), "kg", per(v.mass));
            row.push(c("copy_mass"), "kg", per(v.copy_mass));
            row.push(c("matter"), "1", ratio(v.matter, v.mass));
            row.push(c("body"), "kg", per(v.body));
            row.push(c("activity"), "1", per(v.act));
            row.push(c("dry"), "1", per(v.dry));
            row.push(c("wet_gene"), "log10", per(v.wet));
            // The genes of reproduction and the brood as a share of the members' mass.
            row.push(c("alloc"), "1", per(v.alloc));
            row.push(c("child"), "log10", per(v.child));
            row.push(c("brood"), "1", ratio(v.brood, v.mass));
            for (&(_, name, unit), &x) in GENES.iter().zip(&v.genes) {
                row.push(c(name), unit, per(x));
            }
            // Each catalyst gene: its mean over the members that carry it on (log10), and the share that do.
            for r in (0..nr).filter(|&r| v.cat[r].1 > 0.0 || ever.contains(&(k, r))) {
                row.push(c(&format!("cat_{r}")), "log10", ratio(v.cat[r].0, v.cat[r].1));
                row.push(c(&format!("cat_{r}_on")), "1", per(v.cat[r].1));
            }
        }
    }
}
