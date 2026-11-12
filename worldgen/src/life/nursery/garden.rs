//! A garden: the cover of one real cell, its month by the cover's own law (`cover::live`) on a year kept (`Year`), with
//! no air and no flow. Its ground is a store of its own: the key liquid the year's ground holds less what its cover took
//! and the rain has not given back yet; and the minerals of its ground, washed out at the year's own rate, taken by its
//! cover and given back by the rot of its remains' body matter.

use super::year::Month;
use pcore::cover::{self, Cover, Landed, Parts, MATTER, NG, NO_ROOT, SLOTS};
use pcore::units::MAX_SUBSTANCES;
use pcore::{Counters, World};

/// A garden in cell `cell`: its cover; the light it has caught per m² of capture area since it began, J/m² (as
/// `World::light`); the key liquid its cover took that the rain has not given back, mol; the minerals in its ground,
/// mol (`Chemistry::minerals`, the rest of its ground is the year's), and the body matter of its remains, mol; the
/// months it has run (they key its seeds' draws); and what its cover lost and rounded (`Counters`, not read).
#[derive(Clone)]
pub(super) struct Garden {
    pub cell: usize,
    pub cover: Cover,
    light: f64,
    owed: f64,
    minerals: [f64; MAX_SUBSTANCES],
    remains: f64,
    months: u64,
    counts: Counters,
}

/// The garden's months key their seeds' draws from this tick on, far past any world's months (`World::month`).
const TICK: u64 = 1 << 60;

impl Garden {
    /// Bare ground in cell `cell`, its minerals as `first` (the first month of its year) holds them.
    pub fn bare(chem: &pcore::chem::Chemistry, cell: usize, first: &Month) -> Garden {
        let mut minerals = [0.0; MAX_SUBSTANCES];
        chem.minerals().iter().for_each(|&(k, _)| minerals[k] = first.around.ground[k]);
        Garden { cell, cover: Cover::default(), light: 0.0, owed: 0.0, minerals, remains: 0.0, months: 0, counts: Counters::default() }
    }

    /// `kg` per m² of cover of genes `genes` and starting kind `root` put here, at the year's temperature `t` K, as seeds
    /// lodge (`cover::lodge`: they always take hold).
    pub fn put(&mut self, w: &World, p: &Parts, (genes, root): ([f32; NG], u32), kg: f64, t: f64) {
        let area = w.mesh.cells().area()[self.cell];
        let s = cover::sown(&w.chem, &w.cfg.phys, p, (self.cell, area, t), genes, root, kg);
        if let Landed::Out(m) = cover::lodge(&mut self.cover, &p.mm(&w.chem), &s, &w.cfg.phys, area) {
            self.remains += m[MATTER];
        }
    }

    /// One month on the year's month `m`, in the order of a world's month: the rain washes its minerals (`wash`: per
    /// mineral the share that stays, or what comes in, mol), its cover lives (`cover::live`), and the open world's rot
    /// frees the minerals of its remains' body matter (`change::open_share` of the reaction that rots it, reaction 7 of
    /// chemistry 0, at the moisture its cover feels). Its seeds that land in its cell lodge there; those that land off
    /// it are gone.
    pub fn month(&mut self, w: &World, p: &Parts, m: &Month, wash: &[(usize, f64, f64)]) {
        let (chem, area, dt) = (&w.chem, w.mesh.cells().area()[self.cell], w.cfg.calendar().month as f64);
        let mm = p.mm(chem);
        for &(k, stays, comes) in wash {
            self.minerals[k] = self.minerals[k] * stays + comes;
        }
        // The light past the clouds over the month, per m² of capture area under the garden's own cover.
        let canopy = (0..SLOTS).map(|x| self.cover.leaf(&mm, x)).sum::<f64>() / area;
        self.light += m.sun * cover::per_capture_area(canopy);
        let held = (m.held - self.owed).max(0.0);
        let liquid = if m.held > 0.0 { held * m.around.liquid / m.held } else { 0.0 };
        let mut a = m.around.clone();
        (a.liquid, a.moist, a.light, a.tick) = (liquid, cover::wetness(w, p, area, liquid), self.light, TICK + self.months);
        a.ground[p.liquid] = held;
        chem.minerals().iter().for_each(|&(k, _)| a.ground[k] = self.minerals[k]);
        let (b, _) = cover::live(w, &mut self.cover, p, &a, (self.cell, dt), &mut self.counts);
        // What its leaves lose through their pores' film (`Cover::film`), which the full world's ground evaporates day
        // by day, at the month's air, from the liquid its roots reach; with what it took of the liquid, owed to the
        // ground until the rain gives it back.
        let film = if a.pores > 0.0 { self.cover.film as f64 * area * dt * a.thirst / a.pores } else { 0.0 };
        let vapour = film.min(held * self.cover.reach as f64);
        self.owed = (self.owed + b.ext[p.liquid] + vapour - m.rain).max(0.0);
        self.cover.e = self.cover.cap(chem, p) * a.t;
        self.months += 1;
        self.remains += b.dead[p.sub[MATTER]];
        let cell = self.cell;
        for s in self.cover.sent().into_iter().filter(|s| s.to as usize == cell) {
            if let Landed::Out(m) = cover::lodge(&mut self.cover, &mm, &s, &w.cfg.phys, area) {
                self.remains += m[MATTER];
            }
        }
        // The rot takes its gas from the air the ground touches (`Around::air`), the remains' matter a solid of its own.
        let (matter, remains) = (p.sub[MATTER], self.remains);
        let gas = |k: usize| if a.air[k].is_finite() { a.air[k] } else { 0.0 };
        let air: f64 = (0..chem.ns()).map(gas).sum();
        let rots = chem.reactions.iter().find(|r| r.open > 1.0 && r.inputs.iter().any(|q| q.0 as usize == matter));
        let held = |k: usize| if k == matter { remains } else { gas(k) };
        let rotted = rots.map_or(0.0, |r| remains * pcore::change::share_over(r, (a.t, a.moist), dt, held, |k| if k == matter { remains } else { air }));
        self.remains -= rotted;
        for &(k, per) in chem.minerals() {
            self.minerals[k] = (self.minerals[k] - b.ext[k] + rotted * per).max(0.0);
        }
    }

    /// Its slots that hold cover: their starting kind, genes and cover per m², kg, heaviest first.
    pub fn stand(&self, w: &World, p: &Parts) -> Vec<(u32, [f32; NG], f64)> {
        let (mm, area) = (p.mm(&w.chem), w.mesh.cells().area()[self.cell]);
        let mut out: Vec<(u32, [f32; NG], f64)> = (0..SLOTS).filter(|&x| self.cover.root[x] != NO_ROOT).map(|x| (self.cover.root[x], self.cover.genes[x], self.cover.kg(&mm, x) / area)).collect();
        out.sort_by(|a, b| b.2.total_cmp(&a.2).then(a.0.cmp(&b.0)));
        out
    }
}
