//! Each starting kind (`root_<k>_*`, by `Lineage::root`): its members, bodies and mass, its births and deaths of
//! the year, its lives, and the means by members of its mass, body, genes and catalysts. A kind that died out reads
//! 0 (its means NaN); the columns are there for every kind up to the largest seen so far.

use super::{ratio, Row};
use crate::layers::Ctx;
use crate::rec::{Recorder, CAUSES};
use pcore::body::{self, Body};
use pcore::World;
use std::collections::BTreeSet;

/// The genes averaged per kind: gene, column name and unit.
const GENES: [(usize, &str, &str); 8] = [
    (body::GAS, "gas", "log10"),
    (body::LEAF, "leaf", "log10"),
    (body::COPY_MASS, "copy_log", "log10"),
    (body::INTERVAL, "interval", "log10"),
    (body::HIDDEN, "hidden", "1"),
    (body::HEAT, "heat", "log10"),
    (body::KEEP_T, "keep_t", "K"),
    (body::REPAIR, "repair", "log10"),
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

    fn add(&mut self, w: &World, b: &Body, cat: usize) {
        let (chem, n) = (&w.chem, b.n());
        let matter_mm = chem.matter().map(|m| (m, chem.subs[m].molar_mass));
        self.members += n;
        self.bodies += 1.0;
        self.life += n * life_years(w, b);
        self.mass += b.mass(chem);
        self.matter += matter_mm.map_or(0.0, |(m, mm)| b.store[m] * mm);
        self.body += b.size(chem);
        self.copy_mass += n * b.dec(body::COPY_MASS);
        (self.alloc, self.child) = (self.alloc + n * b.gene(body::ALLOC), self.child + n * b.gene(body::CHILD));
        self.brood += b.brooded(chem) * matter_mm.map_or(0.0, |(_, mm)| mm);
        let a = b.activity(chem, &w.cfg.phys);
        (self.act, self.dry, self.wet) = (self.act + n * a, self.dry + n * (a < DRY) as u8 as f64, self.wet + n * b.gene(body::WET));
        for (x, &(g, ..)) in self.genes.iter_mut().zip(&GENES) {
            *x += n * b.gene(g);
        }
        let nr = self.cat.len();
        for (x, &g) in self.cat.iter_mut().zip(&b.genes[cat..cat + nr]) {
            if g >= 0.0 {
                (x.0, x.1) = (x.0 + n * g as f64, x.1 + n);
            }
        }
    }
}

/// Years a member of `b` lives at its upkeep now: `e_life` × 10^`REPAIR` per kg of living body over its upkeep.
fn life_years(w: &World, b: &Body) -> f64 {
    let mut x = Body::default();
    let b = if b.k.cat.is_empty() {
        x.clone_from(b);
        x.prepare(&w.chem, &w.cfg.phys);
        &x
    } else {
        b
    };
    let (m, d) = (b.mass(&w.chem), b.interval(w) as f64);
    w.cfg.phys.e_life * b.dec(body::REPAIR) * b.built(&w.chem).max(1e-9 * m) * d / body::upkeep(w, b, d, m, b.temp(&w.chem)).max(1e-300) / w.days_per_year() as f64
}

impl Recorder {
    pub(super) fn kind_cols(&self, c: &Ctx, row: &mut Row) {
        let (w, y) = (c.w, &self.year);
        let (nr, cat) = (w.chem.reactions.len(), body::Layout::of(&w.chem).cat);
        let seen = self.series.iter().filter(|c| c.name.starts_with("root_") && c.name.ends_with("_members")).count();
        let mut kinds = vec![Kind::new(nr); seen.max(y.kinds.len())];
        for b in w.life.alive() {
            if let Some(l) = &w.life.brains[b.lineage as usize] {
                let k = l.root as usize;
                if k >= kinds.len() {
                    kinds.resize(k + 1, Kind::new(nr));
                }
                kinds[k].add(w, b, cat);
            }
        }
        // The catalyst genes on in some member of the kind so far keep their columns.
        let ever: BTreeSet<(usize, usize)> = self.series.iter().filter_map(|c| c.name.strip_prefix("root_")?.split_once("_cat_")).filter_map(|(k, r)| Some((k.parse().ok()?, r.parse().ok()?))).collect();
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
