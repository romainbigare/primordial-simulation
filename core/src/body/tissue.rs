//! The body as tissue: matter and brood, size and supply, the levels of its stores, its materials and height (laws.md
//! 6.1, 3.6).

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
    /// Amounts `m` (mol, at most what it holds) leave the stores; the brood falls in the share of the matter that left.
    pub(crate) fn lose(&mut self, chem: &Chemistry, m: &[f64]) {
        let before = self.matter_mol(chem);
        for (s, a) in self.store.iter_mut().zip(m) {
            *s -= a;
        }
        self.brood_share(chem, before);
    }
    /// The body matter it holds, mol (all members, its brood with it).
    pub(crate) fn matter_mol(&self, chem: &Chemistry) -> f64 {
        chem.matter().map_or(0.0, |k| self.store[k])
    }
    /// Its body matter changed from `before` mol by what acts on all of it alike (matter leaving, its reactions, its
    /// burnings): the brood keeps its share of it.
    pub(crate) fn brood_share(&mut self, chem: &Chemistry, before: f64) {
        if before > 0.0 {
            self.brood = self.brood.max(0.0).min(before) * (self.matter_mol(chem) / before);
        }
    }
    /// The matter of the body itself, kg (all members): the matter in the store less the brood.
    pub fn matter_kg(&self, chem: &Chemistry) -> f64 {
        chem.matter().map_or(0.0, |m| (self.store[m] - self.brooded(chem)) * chem.subs[m].molar_mass)
    }
    /// The body, kg (all members): its matter (less the brood) over 1 - `h`, `h` the most share it keeps of a key liquid
    /// (`wet_h`), and no more than its matter and all its liquid. Fuel is not body. The safe minimums go by it.
    pub fn size(&self, chem: &Chemistry) -> f64 {
        if chem.matter().is_none() {
            return self.mass(chem);
        }
        let matter = self.matter_kg(chem);
        let (liquid, h) = self.wet_h(chem);
        (matter / (1.0 - h)).min(matter + liquid)
    }
    /// The living body its matter makes, kg (all members): its matter (less the brood) over 1 - `WET_TISSUE`, whatever
    /// liquid it holds. Growth, intake, upkeep, wear, force and the leaf go by it.
    pub fn built(&self, chem: &Chemistry) -> f64 {
        if chem.matter().is_none() {
            return self.size(chem);
        }
        self.matter_kg(chem) / (1.0 - WET_TISSUE)
    }
    /// Whether the mean member's body (`built`) is at or past its copy mass (`COPY_MASS`).
    pub fn adult(&self, chem: &Chemistry) -> bool {
        self.mature(self.member_body(chem))
    }
    /// The body of the mean member, kg (`built` per member).
    pub fn member_body(&self, chem: &Chemistry) -> f64 {
        self.built(chem) / self.n()
    }
    /// The mean and the variance of log10 of its members' bodies, kg: the mean is kept as `size_lag` below log10 of the
    /// mean member's body, so it rises as much when all grow alike in proportion.
    pub fn size_moments(&self, chem: &Chemistry) -> (f32, f32) {
        ((self.member_body(chem).max(f64::MIN_POSITIVE).log10() - self.size_lag as f64) as f32, self.size_spread)
    }
    /// Its members' log10 bodies (kg) have the mean `mean` and the variance `v` (`size_moments`).
    pub(crate) fn set_size(&mut self, chem: &Chemistry, mean: f64, v: f64) {
        self.size_lag = (self.member_body(chem).max(f64::MIN_POSITIVE).log10() - mean) as f32;
        self.size_spread = v.max(0.0) as f32;
    }
    /// A member body (`member_body`) is at or past the copy mass (`adult`).
    pub fn mature(&self, member: f64) -> bool {
        member >= (1.0 - 1e-9) * self.dec(COPY_MASS)
    }
    /// Activity, 0..1: min(1, l / l*)^n, l the key liquids it holds, l* their `level`, n `Physics::dormancy`, as a body's
    /// chemistry runs in its liquid (laws.md 6.5). 1 without a key liquid it keeps a level of.
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
    /// The key liquids it holds, kg, and the most share of its mass it keeps of any (its keep gene, 0..`MAX_KEEP`).
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
    /// Mass-equivalent for what goes with mass^0.75 in life, kg: per member `m_ref` (`mass` / `m_ref`)^`intake_exp`, times
    /// the count. Intake, building and the leaf go by it; upkeep goes with mass^1.0.
    pub fn equiv(&self, p: &crate::Physics, mass: f64) -> f64 {
        let n = self.n();
        n * p.m_ref * (mass / n / p.m_ref).powf(p.intake_exp)
    }
    /// What a body can take and build, in kg-equivalents: `equiv` of its living body times the pace of its temperature,
    /// as the whole tempo of a body goes with exp(-Ea/T) (Gillooly et al. 2001).
    pub fn supply(&self, chem: &Chemistry, p: &crate::Physics, t: f64) -> f64 {
        self.equiv(p, self.built(chem)) * self.pace(chem, p, t)
    }
    /// Most of substance `k` the body holds, mol, by its keep gene (laws.md 6.1): a key liquid h / (1 - h) of its matter;
    /// a solid it builds what its `share` makes of its matter; any other, kg per kg of living body; body matter a share
    /// of the whole mass `mass`.
    pub fn level(&self, chem: &Chemistry, k: usize, mass: f64) -> f64 {
        self.level_at(chem, k, mass, self.gene(Layout::of(chem).keep + k))
    }
    /// `level` with the keep gene of `k` at `f`.
    pub(crate) fn level_at(&self, chem: &Chemistry, k: usize, mass: f64, f: f64) -> f64 {
        let mm = chem.subs[k].molar_mass;
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
    /// What it keeps of `k` when it gives or puts out (of size `size`, kg): `Physics::keep_min` times its safe minimum,
    /// mol.
    pub(crate) fn kept(&self, chem: &Chemistry, p: &crate::Physics, k: usize, size: f64) -> f64 {
        p.keep_min * self.gene(Layout::of(chem).min + k).max(0.0) * size / chem.subs[k].molar_mass
    }
    /// What it can spare of store `k` to give, to a body or into a part (of size `size` and mass `mass`, kg), mol: never
    /// its body matter or a built solid; what it holds above `kept` and, of a key liquid, above its level.
    pub(crate) fn spare(&self, chem: &Chemistry, p: &crate::Physics, k: usize, (size, mass): (f64, f64)) -> f64 {
        if chem.matter() == Some(k) || chem.is_material(k) {
            return 0.0;
        }
        let level = if self.wet >> k & 1 == 1 { self.level(chem, k, mass) } else { 0.0 };
        (self.store[k] - level.max(self.kept(chem, p, k, size))).max(0.0)
    }
    /// What the body is built of, all members: [volume m³, Σ volume × Young's modulus, Σ volume × breaking stress] over
    /// its soft tissue (`built` at `rho_body`) and each solid it builds.
    pub fn solids(&self, chem: &Chemistry, cfg: &crate::Config) -> [f64; 3] {
        let Some(m) = chem.matter() else { return [0.0; 3] };
        let soft = self.built(chem).max(0.0) / cfg.phys.rho_body;
        let of = |v: f64, s: &crate::chem::Substance| [v, v * s.modulus, v * s.rupture];
        chem.materials().map(|k| of(self.store[k].max(0.0) * chem.subs[k].molar_mass / chem.subs[k].density[0], &chem.subs[k])).fold(of(soft, &chem.subs[m]), |a, x| [a[0] + x[0], a[1] + x[1], a[2] + x[2]])
    }
    /// A member's height, m: what it is built of holds up against its own weight. A column buckles past h³ = `GREENHILL`
    /// E I / w (Greenhill 1881; real stems near a quarter of it, McMahon 1973); one round section (I = A² / 4π, A = V / h)
    /// gives h⁴ = `GREENHILL` / 4π × Σ E V × V / (m g).
    pub fn height(&self, chem: &Chemistry, cfg: &crate::Config) -> f64 {
        let (n, mass) = (self.n(), self.mass(chem));
        if chem.matter().is_none() {
            return (mass / n / cfg.phys.rho_body).cbrt();
        }
        let [v, ev, _] = self.solids(chem, cfg);
        (GREENHILL / (4.0 * std::f64::consts::PI) * ev * v / (n * mass * cfg.gravity).max(1e-30)).max(0.0).powf(0.25)
    }
    /// The energy to break the share `f` of the body, J: its materials' volumes times their breaking stresses, so hard
    /// parts protect.
    pub fn breaking(&self, chem: &Chemistry, cfg: &crate::Config, f: f64) -> f64 {
        f * self.solids(chem, cfg)[2]
    }
    /// The share of solid `k` in what the body builds (its keep gene, all together at most `MAX_KEEP`; `usize::MAX`: the sum).
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
    /// A store below its safe minimum, or nothing left of a member, for its mass `m` and temperature `t`.
    pub fn lacking(&self, chem: &Chemistry, m: f64, t: f64) -> bool {
        // A safe minimum is a share of the body, not of the whole mass: holding fuel kills no body.
        let (lay, size) = (Layout::of(chem), self.size(chem));
        self.empty_at(m) || (0..chem.ns()).any(|k| !self.burns_at(chem, k, t) && self.store[k] * chem.subs[k].molar_mass < self.gene(lay.min + k) * size)
    }
}
