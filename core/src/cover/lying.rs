//! Seeds lying on the ground (laws.md 3): what lands in a cell lies on its ground as a few lots until the ground lets it
//! grow. A seed is a small dormant plant, matter with its store of fuel and its kind's genes, so the cover's own laws
//! hold for it: it wakes as its kind wakes (its activity on that ground at its chemistry's pace), wears at the dormant
//! floor and freezes below its hardened limit as dormant cover does, and what wakes takes hold in the cover as seeds
//! always did (`seed::lodge`).

use super::budget::{held, DIED, SEEDS_DIED};
use super::seed::{lodge, Landed, Seed};
use super::{built_kg, dec, distance, frozen, Around, APART, Cover, Parts, FROST, MOIST, NG, NO_ROOT, NP, REPAIR, SLOTS};
use crate::life::kin;
use crate::units::DAY;
use crate::{Counters, World};
use serde::{Deserialize, Serialize};

/// The flows of seeds counted for the tape (`Counters::seeds`), kg: landed on a ground, sprouted (taken hold in the
/// cover), worn out, frozen, taken by a taker.
pub const SEED_FLOWS: [&str; 5] = ["landed", "sprouted", "worn", "frozen", "taken"];
pub const LANDED: usize = 0;
pub const SPROUTED: usize = 1;
pub const WORN: usize = 2;
pub const FROZEN: usize = 3;
pub const TAKEN_LYING: usize = 4;

/// The seeds lying on a cell's ground, in up to `SLOTS` lots, whatever the kinds its cover may hold: each its mol of each
/// part (`Parts`), its genes and its starting kind (`NO_ROOT`: empty). Their heat is the cover's (`Cover::e`): they lie
/// at its temperature.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lying {
    pub mol: [[f64; NP]; SLOTS],
    pub genes: [[f32; NG]; SLOTS],
    pub root: [u32; SLOTS],
}

impl Default for Lying {
    /// No seeds.
    fn default() -> Lying {
        Lying { mol: [[0.0; NP]; SLOTS], genes: [[0.0; NG]; SLOTS], root: [NO_ROOT; SLOTS] }
    }
}

impl Lying {
    /// Its lots that hold seeds.
    pub fn held(&self) -> impl Iterator<Item = usize> + '_ {
        (0..SLOTS).filter(|&x| self.root[x] != NO_ROOT)
    }
    /// All its seeds, kg.
    pub fn kg(&self, mm: &[f64; NP]) -> f64 {
        self.mol.iter().map(|m| (0..NP).map(|q| m[q] * mm[q]).sum::<f64>()).sum()
    }
    /// Seeds `s` lie down: they join the most alike lot of their starting kind (`kin::kin`), the genes the mean by built
    /// matter; else an empty lot; else the two nearest lots join (the lighter into the heavier, `join`) and they take
    /// the lot it frees. Their heat is the caller's to give the cover.
    pub fn lodge(&mut self, mm: &[f64; NP], s: &Seed, alike: f32) {
        let kg = built_kg(mm, &s.mol);
        if let Some(x) = kin::kin(&self.genes, &self.root, (&s.genes, s.root), APART * alike, distance) {
            let f = (kg / (kg + built_kg(mm, &self.mol[x])).max(1e-300)) as f32;
            (0..NG).for_each(|g| self.genes[x][g] += f * (s.genes[g] - self.genes[x][g]));
            (0..NP).for_each(|q| self.mol[x][q] += s.mol[q]);
            return;
        }
        let x = kin::empty(&self.root).unwrap_or_else(|| self.join(mm));
        (self.mol[x], self.genes[x], self.root[x]) = (s.mol, s.genes, s.root);
    }
    /// The two lots nearest each other by their genes join, the lighter into the heavier: amounts added, genes the mean
    /// by built matter, its starting kind the heavier's. Returns the lot left empty.
    fn join(&mut self, mm: &[f64; NP]) -> usize {
        let (a, z) = kin::nearest(&self.genes, distance);
        let (big, small) = if built_kg(mm, &self.mol[z]) > built_kg(mm, &self.mol[a]) { (z, a) } else { (a, z) };
        let f = (built_kg(mm, &self.mol[small]) / (built_kg(mm, &self.mol[big]) + built_kg(mm, &self.mol[small])).max(1e-300)) as f32;
        let (gs, ms) = (self.genes[small], self.mol[small]);
        (0..NG).for_each(|g| self.genes[big][g] += f * (gs[g] - self.genes[big][g]));
        (0..NP).for_each(|q| self.mol[big][q] += ms[q]);
        (self.mol[small], self.root[small]) = ([0.0; NP], NO_ROOT);
        small
    }
}

/// The month over `dt` s of the seeds lying in cover `c` of cell `i` of `area` m², on ground as `a` reads it. Of each
/// lot the share its wear gives dies (dormant cover's wear: its upkeep at full activity over a kg, at the floor
/// `Physics::dormant`, over `e_life` × 10^`REPAIR`), and the share frozen below its hardened limit (`cover::frozen` at
/// activity 0); of the rest the share min(1, its activity on that ground × the pace of its burning at the ground's
/// temperature over its pace at `t_cat`) wakes, so at `t_cat` all that is active wakes in a month (seeds of temperate
/// plants sprout in days to weeks once warm and wet; Bewley 1997, from memory), and takes hold in the cover
/// (`seed::lodge`); on no ground none wakes. Returns what dies, mol of each part (with a slot in deficit the waking seeds push out), for the
/// cell's remains; its budget's flows and the seeds' flows are counted in `budget` and `counts`.
pub(super) fn month(w: &World, c: &mut Cover, p: &Parts, a: &Around, (i, area, dt): (usize, f64, f64), budget: &mut [f64], counts: &mut Counters) -> [f64; NP] {
    let mut dead = [0.0; NP];
    if c.lying.held().next().is_none() {
        return dead;
    }
    let (chem, ph) = (&w.chem, &w.cfg.phys);
    let (mm, e) = (p.mm(chem), p.content(chem));
    let burn = &chem.reactions[p.burn];
    let pace = burn.relative_rate(a.t, ph.t_cat);
    let melt = chem.subs[p.liquid].melt;
    for x in 0..SLOTS {
        if c.lying.root[x] == NO_ROOT {
            continue;
        }
        let (g, root) = (c.lying.genes[x], c.lying.root[x]);
        let rate: f64 = super::upkeep::upkeep_rates(chem, ph, p, &g, pace).iter().sum();
        let worn = crate::math::one_minus_exp(dt / DAY * rate * ph.dormant / (ph.e_life * dec(g[REPAIR])));
        let cold = frozen(ph, melt, g[FROST] as f64, (a.t, 0.0));
        let lot = c.lying.mol[x];
        let kg = built_kg(&mm, &lot) + lot[super::FUEL] * mm[super::FUEL];
        counts.seeds[WORN] += worn * kg;
        counts.seeds[FROZEN] += (1.0 - worn) * cold * kg;
        let dies = 1.0 - (1.0 - worn) * (1.0 - cold);
        // A seed is dry tissue: it wakes as the liquid it can take in lets it, by the seeds' rule (`Physics::activity`;
        // full under the liquid), and as far as its first roots get the oxidising gas, on the ground its kind stands on
        // at the least root need (`footing`); where that is none it waits.
        let foot = super::footing(w, i, &g, ph.root_least, &a.soak, None);
        let wet = ph.activity(a.soak.moist / dec(g[MOIST])) * (1.0 - foot.wet) + foot.wet;
        let wakes = if area * foot.share > 0.0 { (wet * pace * foot.work).min(1.0) * (1.0 - dies) } else { 0.0 };
        let (gone, up): ([f64; NP], [f64; NP]) = (lot.map(|m| m * dies), lot.map(|m| m * wakes));
        budget[SEEDS_DIED] -= held(&e, &gone);
        (0..NP).for_each(|q| dead[q] += gone[q]);
        // What wakes takes hold where the cover has room for it (`seed::lodge`); else it stays lying.
        // Those that join a stand of their kind take, as seedlings, its ground below its line where they land
        // (`seed::lines`): its grown plants stand where they stood.
        let mut recruits = [0.0; SLOTS];
        let landed = if up.iter().any(|&m| m > 0.0) { lodge(c, &mm, &Seed { to: i as u32, mol: up, genes: g, root, heat: 0.0 }, &w.cfg, (area, foot.share, foot.strip), &mut recruits) } else { Landed::Refused };
        c.line = super::seed::lines(w, c, i, &a.soak, (&recruits, built_kg(&mm, &up)), area * foot.share);
        let woke = match landed {
            Landed::In => true,
            Landed::Out(m) => {
                budget[DIED] -= held(&e, &m);
                (0..NP).for_each(|q| dead[q] += m[q]);
                true
            }
            Landed::Refused => false,
        };
        c.lying.mol[x] = std::array::from_fn(|q| (lot[q] - gone[q] - if woke { up[q] } else { 0.0 }).max(0.0));
        if woke {
            counts.seeds[SPROUTED] += wakes * kg;
        }
        if built_kg(&mm, &c.lying.mol[x]) <= super::EMPTY_KG {
            let rest = std::mem::take(&mut c.lying.mol[x]);
            budget[SEEDS_DIED] -= held(&e, &rest);
            (0..NP).for_each(|q| dead[q] += rest[q]);
            c.lying.root[x] = NO_ROOT;
        }
    }
    dead
}
