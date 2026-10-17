//! The body as tissue: its matter and brood, the liquid it keeps, its size and supply, the levels of its stores, what
//! it is built of, its height, and when it lacks a store.

use super::{Body, Layout, COPY_MASS, WET_TISSUE};
use crate::chem::{Chemistry, Phase};

/// The most share of a body its keep gene of a key liquid, or the solids it builds together, may make up.
pub(crate) const MAX_KEEP: f64 = 0.95;

/// Less than this of a member is nothing left of it, kg.
const EMPTY_KG: f64 = 1e-3;

/// A column standing under its own weight buckles past h^3 = this x E I / w (Greenhill 1881).
const GREENHILL: f64 = 7.837;

impl Body {
    /// Key stores (safe minimum above 0), one bit per substance.
    pub(crate) fn key_mask(&self, chem: &Chemistry) -> u64 {
        let lay = Layout::of(chem);
        (0..chem.ns()).filter(|&k| self.genes[lay.min + k] > 0.0).fold(0, |m, k| m | 1 << k)
    }
    /// Key stores that are liquid now.
    pub fn key_liquids(&self, chem: &Chemistry) -> u64 {
        let (t, lay) = (self.temp(chem), Layout::of(chem));
        (0..chem.ns()).filter(|&k| self.genes[lay.min + k] > 0.0 && chem.phase_in(k, t, |j| self.store[j]) == Phase::Liquid).fold(0, |m, k| m | 1 << k)
    }
    /// The brood, mol of body matter (all members), no more than the matter there is.
    pub fn brooded(&self, chem: &Chemistry) -> f64 {
        chem.matter().map_or(0.0, |m| self.brood.max(0.0).min(self.store[m].max(0.0)))
    }
    /// Amounts `m` (mol of each substance, at most what it holds) leave the stores; the brood falls in the share of the
    /// body matter that left (it is part of it).
    pub(crate) fn lose(&mut self, chem: &Chemistry, m: &[f64]) {
        let matter = chem.matter().map(|k| (k, self.store[k], self.brooded(chem)));
        for (s, a) in self.store.iter_mut().zip(m) {
            *s -= a;
        }
        if let Some((k, before, brood)) = matter.filter(|x| x.1 > 0.0) {
            self.brood = brood * (self.store[k] / before);
        }
    }
    /// The matter of the body itself, kg (all members): the matter in the store less the brood.
    pub fn matter_kg(&self, chem: &Chemistry) -> f64 {
        chem.matter().map_or(0.0, |m| (self.store[m] - self.brooded(chem)) * chem.subs[m].molar_mass)
    }
    /// The body, kg (all members): its body matter (less the brood) and the key liquid it keeps with it, `h` the most share
    /// of its mass it keeps of any (`wet_h`): its matter over 1 - `h`, and no more than its matter and all its liquid (a
    /// body short of liquid is as big as it is wet). The fuel it stores and liquid held past `h` are not body. The safe
    /// minimums go by it. A chemistry with no body matter: the whole mass.
    pub fn size(&self, chem: &Chemistry) -> f64 {
        if chem.matter().is_none() {
            return self.mass(chem);
        }
        let matter = self.matter_kg(chem);
        let (liquid, h) = self.wet_h(chem);
        (matter / (1.0 - h)).min(matter + liquid)
    }
    /// The living body its matter makes, kg (all members): its matter (less the brood) over 1 - `WET_TISSUE`, whatever
    /// liquid it holds. Growth, maturity, intake, building, upkeep, wear, force and the leaf go by it. Without body
    /// matter: `size`.
    pub fn built(&self, chem: &Chemistry) -> f64 {
        if chem.matter().is_none() {
            return self.size(chem);
        }
        self.matter_kg(chem) / (1.0 - WET_TISSUE)
    }
    /// Whether the mean member's body (`built`) is at or past its copy mass (`COPY_MASS`): grown, and it puts a share of
    /// what it builds into offspring (`copy::grow`). A node mixes ages, so its mean member says only what most of it is.
    pub fn adult(&self, chem: &Chemistry) -> bool {
        self.mature(self.member_body(chem))
    }
    /// The body of the mean member, kg (`built` per member).
    pub fn member_body(&self, chem: &Chemistry) -> f64 {
        self.built(chem) / self.n()
    }
    /// A member body (`member_body`) is at or past the copy mass (`adult`).
    pub fn mature(&self, member: f64) -> bool {
        member >= (1.0 - 1e-9) * self.dec(COPY_MASS)
    }
    /// Activity, 0..1: a body's chemistry runs in its liquid, so it runs as min(1, l / l*)^n, l the key liquids it holds,
    /// l* what `level` keeps of them, n `Physics::dormancy`: a dry body is nearly dormant. A body with no key liquid, or
    /// none that it keeps a level of, has 1. Upkeep (`upkeep`), its catalysts (`catalyst`), its building (`copy::grow`)
    /// and the share of a freezing that kills (`live`) go with it.
    pub fn activity(&self, chem: &Chemistry, p: &crate::Physics) -> f64 {
        if self.wet == 0 || chem.matter().is_none() {
            return 1.0;
        }
        let (mut have, mut keep) = (0.0, 0.0);
        for k in (0..chem.ns()).filter(|&k| self.wet >> k & 1 == 1) {
            let mm = chem.subs[k].molar_mass;
            have += self.store[k] * mm;
            keep += self.level(chem, k, 0.0) * mm;
        }
        if keep <= 0.0 {
            return 1.0;
        }
        (have / keep).clamp(0.0, 1.0).powf(p.dormancy)
    }
    /// The melting point of its liquid `k`, lowered by what is dissolved in it (`Chemistry::melt_in`).
    pub fn melt(&self, chem: &Chemistry, k: usize) -> f64 {
        chem.melt_in(k, |j| self.store[j])
    }
    /// The key liquids it holds, kg (all members), and the most share of its mass it keeps of any of them (its keep gene,
    /// 0..`MAX_KEEP`).
    pub fn wet_h(&self, chem: &Chemistry) -> (f64, f64) {
        let (keep, ns) = (Layout::of(chem).keep, chem.ns());
        // The key liquids are a few of the substances: visit the bits set, lowest first.
        let mut bits = if ns < 64 { self.wet & ((1 << ns) - 1) } else { self.wet };
        let (mut liquid, mut h) = (0.0f64, 0.0f64);
        while bits != 0 {
            let k = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            (liquid, h) = (liquid + self.store[k] * chem.subs[k].molar_mass, h.max(self.gene(keep + k).clamp(0.0, MAX_KEEP)));
        }
        (liquid, h)
    }
    /// Mass-equivalent a body's size is counted by, kg, for what goes with mass^0.75 in life: per member `m_ref` times its
    /// `mass` over `m_ref` to the power `intake_exp`, times the count. Intake, building (`c_build`) and the leaf go by it;
    /// upkeep does not (mass^1.0, `upkeep`).
    pub fn equiv(&self, p: &crate::Physics, mass: f64) -> f64 {
        let n = self.n();
        n * p.m_ref * (mass / n / p.m_ref).powf(p.intake_exp)
    }
    /// What a body can take and build, in kg-equivalents: `equiv` of the body that is built (the fuel it stores is carried,
    /// not working tissue), times the pace of its temperature (`pace`): the whole tempo of a body goes with exp(-Ea/T)
    /// (metabolic theory of ecology, Gillooly et al. 2001), as what it pays to live does.
    pub fn supply(&self, chem: &Chemistry, p: &crate::Physics) -> f64 {
        self.equiv(p, self.built(chem)) * self.pace(chem, p, self.temp(chem))
    }
    /// Most of substance `k` the body holds, mol, by its keep gene: for a key liquid the largest share of the body it may
    /// make up, `h` (below `MAX_KEEP`), so h / (1 - h) of its matter; for a solid it builds (`Chemistry::materials`), what
    /// its share in what it builds (`share`) makes of its matter; for every other substance but body matter, kg of it per
    /// kg of its body (`built`). Body matter itself is a share of the whole mass `mass` (also for a chemistry with none).
    /// What it takes stops there.
    pub fn level(&self, chem: &Chemistry, k: usize, mass: f64) -> f64 {
        let mm = chem.subs[k].molar_mass;
        let f = self.gene(Layout::of(chem).keep + k);
        let share = |f: f64| f.clamp(0.0, MAX_KEEP) / (1.0 - f.clamp(0.0, MAX_KEEP));
        match chem.matter() {
            Some(_) if chem.is_material(k) => self.share(chem, k) / (1.0 - self.share(chem, usize::MAX)) * self.matter_kg(chem) / mm,
            Some(m) if k != m => {
                if self.wet >> k & 1 == 1 {
                    share(f) * self.matter_kg(chem) / mm
                } else {
                    f.max(0.0) * self.built(chem) / mm
                }
            }
            _ => (share(f) * (mass - self.store[k] * mm) / mm).max(0.0),
        }
    }
    /// What the body is built of as a material, all members: [volume m³, the sum of volume x Young's modulus, the sum of
    /// volume x the stress it breaks at] over its soft tissue (`built` at `Physics::rho_body`, with the numbers of body
    /// matter) and each solid it builds (`Chemistry::materials`). Its fuel and free liquid are in none of them.
    pub fn solids(&self, chem: &Chemistry, cfg: &crate::Config) -> [f64; 3] {
        let Some(m) = chem.matter() else { return [0.0; 3] };
        let soft = self.built(chem).max(0.0) / cfg.phys.rho_body;
        let of = |v: f64, s: &crate::chem::Substance| [v, v * s.modulus, v * s.rupture];
        chem.materials().map(|k| of(self.store[k].max(0.0) * chem.subs[k].molar_mass / chem.subs[k].density[0], &chem.subs[k])).fold(of(soft, &chem.subs[m]), |a, x| [a[0] + x[0], a[1] + x[1], a[2] + x[2]])
    }
    /// A member's height, m: what what it is built of holds up against its own weight, the order of light in
    /// `survey::shade`. A column under its own weight buckles past h³ = `GREENHILL` E I / w (Greenhill 1881; McMahon 1973
    /// found real stems near a quarter of it), w its weight per length; with its solids in one round section (I = A² / 4π,
    /// A = V / h), h⁴ = `GREENHILL` / 4π x (sum of E V) x V / (m g) (`solids`), m the member's whole mass. Without body
    /// matter: (mass / `rho_body`)^(1/3).
    pub fn height(&self, chem: &Chemistry, cfg: &crate::Config) -> f64 {
        let (n, mass) = (self.n(), self.mass(chem));
        if chem.matter().is_none() {
            return (mass / n / cfg.phys.rho_body).cbrt();
        }
        let [v, ev, _] = self.solids(chem, cfg);
        (GREENHILL / (4.0 * std::f64::consts::PI) * ev * v / (n * mass * cfg.gravity).max(1e-30)).max(0.0).powf(0.25)
    }
    /// The energy to break the share `f` of the body, J: what it is built of times the stress each breaks at (`solids`, J
    /// per m³), so hard parts protect. A bite pays it (`choose::take_body`); a mover pushing through breaks as much as
    /// the energy it spends there pays for (`choose::wear`).
    pub fn breaking(&self, chem: &Chemistry, cfg: &crate::Config, f: f64) -> f64 {
        f * self.solids(chem, cfg)[2]
    }
    /// The share of solid `k` in what the body builds: its keep gene of `k`, 0..1, the shares cut together to `MAX_KEEP` at
    /// most; the rest is matter. `usize::MAX`: their sum.
    pub fn share(&self, chem: &Chemistry, k: usize) -> f64 {
        let keep = Layout::of(chem).keep;
        let all: f64 = chem.materials().map(|x| self.gene(keep + x).clamp(0.0, 1.0)).sum();
        let s = if k == usize::MAX { all } else { self.gene(keep + k).clamp(0.0, 1.0) };
        s * (MAX_KEEP / all.max(1e-30)).min(1.0)
    }
    /// Nothing left of a member (under `EMPTY_KG` each) when the mass is `m`.
    pub(crate) fn empty_at(&self, m: f64) -> bool {
        m < EMPTY_KG * self.n()
    }
    /// A store below its safe minimum, or nothing left of a member.
    pub fn lacking(&self, chem: &Chemistry) -> bool {
        self.lacking_at(chem, self.mass(chem))
    }
    /// `lacking` when the mass `m` is known.
    pub fn lacking_at(&self, chem: &Chemistry, m: f64) -> bool {
        // (A safe minimum is a share of the body, not of the whole mass: a body is not killed by holding fuel.)
        let (lay, size, t) = (Layout::of(chem), self.size(chem), self.temp(chem));
        self.empty_at(m) || (0..chem.ns()).any(|k| !self.burns_at(chem, k, t) && self.store[k] * chem.subs[k].molar_mass < self.gene(lay.min + k) * size)
    }
}
