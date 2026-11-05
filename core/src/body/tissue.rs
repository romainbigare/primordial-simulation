//! The body as tissue: matter and brood, size and supply, the levels of its stores and its materials (laws.md 6.1,
//! 3.6).

use super::{Body, Layout, WET_TISSUE};
use crate::chem::{Chemistry, Phase, Stores};

/// The most share of a body its keep gene of a key liquid, or the solids it builds together, may make up.
pub(crate) const MAX_KEEP: f64 = 0.95;

/// Less than this of a member is nothing left of it, kg.
const EMPTY_KG: f64 = 1e-3;

impl Body {
    /// Key stores (safe minimum above 0), one bit per store.
    pub(crate) fn key_mask(&self, chem: &Chemistry) -> u32 {
        let lay = Layout::of(chem);
        (0..chem.body().n()).filter(|&s| self.genes[lay.min + s] > 0.0).fold(0, |m, s| m | 1 << s)
    }
    /// Key stores that are liquid at `t` K, one bit per store.
    pub fn key_liquids(&self, chem: &Chemistry, t: f64) -> u32 {
        let lay = Layout::of(chem);
        let subs = chem.body().subs();
        (0..subs.len()).filter(|&s| self.genes[lay.min + s] > 0.0 && chem.phase(subs[s], t) == Phase::Liquid).fold(0, |m, s| m | 1 << s)
    }
    /// The place of its body matter among its stores, if bodies grow (`Chemistry::matter`).
    #[inline]
    pub(crate) fn matter_at(chem: &Chemistry) -> Option<usize> {
        chem.matter().and_then(|m| chem.body().place(m))
    }
    /// The brood, mol of body matter (all members), no more than the matter there is.
    pub fn brooded(&self, chem: &Chemistry) -> f64 {
        Body::matter_at(chem).map_or(0.0, |m| self.brood.max(0.0).min(self.store[m].max(0.0)))
    }
    /// Amounts `m` (mol of each store, at most what it holds) leave the stores; the brood falls in the share of the matter
    /// that left.
    pub(crate) fn lose(&mut self, chem: &Chemistry, m: &Stores) {
        let before = self.matter_mol(chem);
        for (s, a) in self.store.iter_mut().zip(m) {
            *s -= a;
        }
        self.brood_share(chem, before);
    }
    /// The body matter it holds, mol (all members, its brood with it).
    pub(crate) fn matter_mol(&self, chem: &Chemistry) -> f64 {
        Body::matter_at(chem).map_or(0.0, |m| self.store[m])
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
        Body::matter_at(chem).map_or(0.0, |m| (self.store[m] - self.brooded(chem)) * chem.body().mm[m])
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
    /// A member body (`member_body`) is at or past the copy mass (`adult`).
    pub fn mature(&self, member: f64) -> bool {
        member >= (1.0 - 1e-9) * self.k.copy_mass
    }
    /// Activity, 0..1: min(1, l / l*)^n, l the key liquids it holds, l* their `level`, n `Physics::dormancy`, as a body's
    /// chemistry runs in its liquid (laws.md 6.5). 1 without a key liquid it keeps a level of.
    pub fn activity(&self, chem: &Chemistry, p: &crate::Physics) -> f64 {
        if self.wet == 0 || chem.matter().is_none() {
            return 1.0;
        }
        let (mut have, mut keep) = (0.0, 0.0);
        for s in self.wet_ks() {
            let mm = chem.body().mm[s];
            have += self.store[s] * mm;
            keep += self.level(chem, s, 0.0) * mm;
        }
        if keep <= 0.0 { 1.0 } else { p.activity(have / keep) }
    }
    /// Its key liquids (`wet`), by their place among its stores, lowest first.
    #[inline]
    pub(crate) fn wet_ks(&self) -> impl Iterator<Item = usize> {
        crate::math::ones(self.wet as u64)
    }
    /// The key liquids it holds, kg, and the most share of its mass it keeps of any (its keep gene, 0..`MAX_KEEP`).
    pub fn wet_h(&self, chem: &Chemistry) -> (f64, f64) {
        let (keep, bc) = (Layout::of(chem).keep, chem.body());
        let (mut liquid, mut h) = (0.0f64, 0.0f64);
        for s in self.wet_ks() {
            (liquid, h) = (liquid + self.store[s] * bc.mm[s], h.max(self.gene(keep + s).clamp(0.0, MAX_KEEP)));
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
        self.equiv(p, self.built(chem)) * self.pace(chem, t)
    }
    /// Most of store `s` the body holds, mol, by its keep gene (laws.md 6.1): a key liquid h / (1 - h) of its matter;
    /// a solid it builds what its `share` makes of its matter; any other, kg per kg of living body; body matter a share
    /// of the whole mass `mass`.
    pub fn level(&self, chem: &Chemistry, s: usize, mass: f64) -> f64 {
        self.level_at(chem, s, mass, self.gene(Layout::of(chem).keep + s))
    }
    /// `level` with the keep gene of store `s` at `f`.
    pub(crate) fn level_at(&self, chem: &Chemistry, s: usize, mass: f64, f: f64) -> f64 {
        let (k, mm) = (chem.body().subs()[s], chem.body().mm[s]);
        let share = |f: f64| f.clamp(0.0, MAX_KEEP) / (1.0 - f.clamp(0.0, MAX_KEEP));
        match chem.matter() {
            Some(_) if chem.is_material(k) => self.share(chem, k) / (1.0 - self.share(chem, usize::MAX)) * self.matter_kg(chem) / mm,
            Some(m) if k != m && self.wet >> s & 1 == 1 => share(f) * self.matter_kg(chem) / mm,
            Some(m) if k != m => f.max(0.0) * self.built(chem) / mm,
            _ => (share(f) * (mass - self.store[s] * mm) / mm).max(0.0),
        }
    }
    /// What it keeps of store `s` when it gives or puts out (of size `size`, kg): `Physics::keep_min` times its safe
    /// minimum, mol.
    pub(crate) fn kept(&self, chem: &Chemistry, p: &crate::Physics, s: usize, size: f64) -> f64 {
        p.keep_min * self.gene(Layout::of(chem).min + s).max(0.0) * size / chem.body().mm[s]
    }
    /// What it can spare of store `s` to give, to a body or into a part (of size `size` and mass `mass`, kg), mol: never
    /// its body matter or a built solid; what it holds above `kept` and, of a key liquid, above its level.
    pub(crate) fn spare(&self, chem: &Chemistry, p: &crate::Physics, s: usize, (size, mass): (f64, f64)) -> f64 {
        let k = chem.body().subs()[s];
        if chem.matter() == Some(k) || chem.is_material(k) {
            return 0.0;
        }
        let level = if self.wet >> s & 1 == 1 { self.level(chem, s, mass) } else { 0.0 };
        (self.store[s] - level.max(self.kept(chem, p, s, size))).max(0.0)
    }
    /// What the body is built of, all members: [volume m³, Σ volume × breaking stress] over its soft tissue (`built` at
    /// `rho_body`) and each solid it builds.
    pub fn solids(&self, chem: &Chemistry, cfg: &crate::Config) -> [f64; 2] {
        let bc = chem.body();
        solids_of(chem, cfg, self.built(chem), |k| self.amount(bc, k), |_| 1.0)
    }
    /// How wide a member is, m: the cube root of the volume it is built of (`solids`).
    pub fn width(&self, chem: &Chemistry, cfg: &crate::Config) -> f64 {
        (self.solids(chem, cfg)[0] / self.n()).cbrt()
    }
    /// The share of the energy to break it, and of the force, that the edge `e` leaves: each material as `Edge::cut`
    /// says for a member's width (laws.md 3.3).
    pub fn cut_by(&self, chem: &Chemistry, cfg: &crate::Config, e: &crate::bond::Edge) -> f64 {
        let (bc, blunt, width) = (chem.body(), self.solids(chem, cfg)[1], self.width(chem, cfg));
        solids_of(chem, cfg, self.built(chem), |k| self.amount(bc, k), |k| e.cut(chem, k, width))[1] / blunt.max(1e-300)
    }
    /// The energy to break the share `f` of the body, J: its materials' volumes times their breaking stresses, so hard
    /// parts protect.
    pub fn breaking(&self, chem: &Chemistry, cfg: &crate::Config, f: f64) -> f64 {
        f * self.solids(chem, cfg)[1]
    }
    /// The share of solid `k` (a substance) in what the body builds (its keep gene, all together at most `MAX_KEEP`;
    /// `usize::MAX`: the sum).
    pub fn share(&self, chem: &Chemistry, k: usize) -> f64 {
        let (keep, bc) = (Layout::of(chem).keep, chem.body());
        let gene = |x: usize| bc.place(x).map_or(0.0, |s| self.gene(keep + s).clamp(0.0, 1.0));
        let all: f64 = chem.materials().map(gene).sum();
        let s = if k == usize::MAX { all } else { gene(k) };
        s * (MAX_KEEP / all.max(1e-30)).min(1.0)
    }
    /// Nothing left of a member (under `EMPTY_KG` each) when the mass is `m`.
    pub(crate) fn empty_at(&self, m: f64) -> bool {
        m < EMPTY_KG * self.n()
    }
    /// A store below its safe minimum, or nothing left of a member, for its mass `m` and temperature `t`.
    pub fn lacking(&self, chem: &Chemistry, m: f64, t: f64) -> bool {
        // A safe minimum is a share of the body, not of the whole mass: holding fuel kills no body.
        let (lay, size, bc) = (Layout::of(chem), self.size(chem), chem.body());
        self.empty_at(m) || (0..bc.n()).any(|s| self.store[s] * bc.mm[s] < self.gene(lay.min + s) * size && !self.burns_at(chem, bc.subs()[s], t))
    }
}

/// What a living thing of `built` kg of soft tissue (`Body::built`) and `mol(k)` of each solid it builds is built of:
/// [volume m³, Σ volume × breaking stress × `cut(k)`], its soft tissue at `rho_body` (as substance `chem.matter()`).
/// `cut(k)`: the share of the energy to break substance `k` that is spent (1 blunt).
pub fn solids_of(chem: &Chemistry, cfg: &crate::Config, built: f64, mol: impl Fn(usize) -> f64, cut: impl Fn(usize) -> f64) -> [f64; 2] {
    let Some(m) = chem.matter() else { return [0.0; 2] };
    let soft = built.max(0.0) / cfg.phys.rho_body;
    let of = |v: f64, k: usize| [v, v * chem.subs[k].rupture * cut(k)];
    chem.materials().map(|k| of(mol(k).max(0.0) * chem.subs[k].molar_mass / chem.subs[k].density[0], k)).fold(of(soft, m), |a, x| [a[0] + x[0], a[1] + x[1]])
}
