//! The body as tissue: its matter, its brood, the liquid it keeps, its size, height and strength.

use super::{Body, Layout, COPY_MASS};
use crate::chem::{Chemistry, Phase};

/// The liquid share of living tissue: a kg of body matter makes 1 / (1 - this) kg of working body
/// (`Body::built`). Anchored: the liquid of living tissue, about 0.7 to 0.75 in animals' lean tissue and 0.8 to 0.9 in
/// herbs (from memory); the founders keep 0.75 (`worldgen::life`).
pub const WET_TISSUE: f64 = 0.75;

/// The most share of a body's mass its keep gene of a key liquid, or of the solids it builds together, may make up.
pub(crate) const MAX_KEEP: f64 = 0.95;

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
    /// The brood, mol of body matter (all members), no more than the matter there is (a body that
    /// burnt or lost matter holds no more brood than matter).
    pub fn brooded(&self, chem: &Chemistry) -> f64 {
        chem.matter().map_or(0.0, |m| self.brood.max(0.0).min(self.store[m].max(0.0)))
    }
    /// The matter of the body itself, kg (all members): the matter in the store less the brood.
    pub fn matter_kg(&self, chem: &Chemistry) -> f64 {
        chem.matter().map_or(0.0, |m| (self.store[m] - self.brooded(chem)) * chem.subs[m].molar_mass)
    }
    /// The body, kg (all members): its body matter and the liquid it holds with it. The matter is
    /// `Chemistry::matter` (what growth makes, `copy::grow`) less the brood (matter held for offspring,
    /// `brooded`, which is not yet a body); the liquid is the key liquid it keeps,
    /// `h` the most share of its mass it keeps of it (the keep gene, below 0.95, `level`), so a body
    /// is its matter over 1 - `h`, and no more than its matter and all its liquid (a body short of
    /// liquid is as big as it is wet). The fuel it stores and liquid held past `h` are not body.
    /// The light it catches and the gas it breathes (its leaf, `visible`, `open_area`) and the safe minimums go by it;
    /// intake, upkeep and heat by the whole mass.
    /// A chemistry with no body matter (`Chemistry::build`): the whole mass.
    pub fn size(&self, chem: &Chemistry) -> f64 {
        if chem.matter().is_none() {
            return self.mass(chem);
        }
        let matter = self.matter_kg(chem);
        let (liquid, h) = self.wet_h(chem);
        (matter / (1.0 - h)).min(matter + liquid)
    }
    /// The living body its matter makes, kg (all members): its matter (less the brood) at the liquid share of living
    /// tissue, `WET_TISSUE`: matter over 1 - `WET_TISSUE`, whatever liquid it keeps or holds. What growth builds to
    /// (`copy::grow`), what maturity reads (`adult`) and a child is counted in (`copy::copy`), and what a body takes,
    /// builds, pays to keep, wears, pushes with and catches light with go by. Without body matter: `size`.
    /// The liquid a body keeps is where its chemistry runs (`activity`), its heat and its volume
    /// (`size`), and gives no free capability.
    pub fn built(&self, chem: &Chemistry) -> f64 {
        if chem.matter().is_none() {
            return self.size(chem);
        }
        self.matter_kg(chem) / (1.0 - WET_TISSUE)
    }
    /// Whether the mean member's body (`built`) is at or past its copy mass (the size of maturity its
    /// genes set, `COPY_MASS`): grown, and it may reproduce (`copy::grow` puts a share of what it
    /// builds into offspring, `copy::copy` makes them). A node mixes ages, so its mean member says
    /// only what most of it is.
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
    /// Activity, 0..1: a body's chemistry runs in its liquid, so it runs in proportion to the key
    /// liquid the body holds over the level it keeps, to the power `Physics::dormancy`:
    /// min(1, l / l*)^n, l the key liquids it holds, l* what `level` keeps of them (h / (1 - h) of
    /// its matter). A body that holds all it keeps (or more) has 1; a dry one is nearly dormant
    /// (n = 5.7: a third of the liquid is 512 times less). The ratio is the mean over the members
    /// of a node. A body with no key liquid, or none that it keeps a level of, has nothing to dry
    /// and has 1. Upkeep and its decisions' cost (`body::upkeep`), the speed-up of its catalysts
    /// (`catalyst`) and its building (`copy::grow`) go with it; so does the share of a freezing that
    /// kills (`body::live`).
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
    /// The key liquids it holds, kg (all members), and the most share of its mass it keeps of any
    /// of them (its keep gene, 0..0.95).
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
    /// Mass-equivalent a body's size is counted by, kg (in place of the mass, for what goes with
    /// mass^0.75 in life): per member `m_ref` times its `mass` over `m_ref`, to the power
    /// `intake_exp` (0.75), times the count. What it takes, sweeps and receives as a gift, what it
    /// can build (`c_build`) and its leaf (`visible`, `open_area`) go by it: the supply. What it
    /// pays to be does not (mass^1.0 of its body, `body::upkeep`). A member of `m_ref` (1 kg) is
    /// 1 kg; one of 30 kg 12.8 kg-equivalents, 2.3 times less per kg; of 10 t 1,778, so it takes 180
    /// times what a 10 kg one (10^0.75 = 5.6) does, not 1,000. Light, rain, liquid and gas go by
    /// area. What it carries is bounded by its body (`Body::level`): a full store is its satiety.
    pub fn equiv(&self, p: &crate::Physics, mass: f64) -> f64 {
        let n = self.n();
        n * p.m_ref * (mass / n / p.m_ref).powf(p.intake_exp)
    }
    /// What a body can take and build, in kg-equivalents: `equiv` of the body that is built
    /// (`Body::built`), not of the whole mass: the fuel it stores is carried, not working tissue,
    /// so it raises neither what the body can take nor what it pays to keep (`body::upkeep`).
    /// Intake (`take`, sweep, gifts), the `c_build` allowance (`copy::grow`) and satiety go by it;
    /// the leaf by the body too (`visible`). **And by the pace of the body's temperature**
    /// (`Body::pace`): the whole tempo of a body goes with exp(-Ea/T) (metabolic theory of
    /// ecology), so what it can take and build per day goes by the same factor as what it pays to
    /// live. Light (a flux), the catalysed reactions (which follow T by their own rate) and the
    /// cost of a metre are not in it.
    pub fn supply(&self, chem: &Chemistry, p: &crate::Physics) -> f64 {
        self.equiv(p, self.built(chem)) * self.pace(chem, p, self.temp(chem))
    }
    /// Most of substance `k` the body holds, mol. Its keep gene is how much of it the body
    /// carries: for a key liquid the largest share of the body it may make up, `h` (below 0.95),
    /// so it holds `h` / (1 - `h`) of its matter and its body is its matter over 1 - `h`
    /// (`size`); for every other substance but body matter, kg of it per kg of its body (`size`;
    /// 0.9 at the start, more for a kind placed with more fuel than that); for a solid it builds
    /// (`Chemistry::materials`), what its share in what it builds makes of its matter (`share`). Body matter itself is
    /// a share of the whole mass. Liquid caught, taken or put stops there; the liquid a body
    /// keeps is that of its body matter, not of the brood. It bounds the stores, not the size.
    /// `mass` is the whole mass, for matter and for a chemistry with none.
    pub fn level(&self, chem: &Chemistry, k: usize, mass: f64) -> f64 {
        let mm = chem.subs[k].molar_mass;
        let f = self.gene(Layout::of(chem).keep + k);
        let share = |f: f64| f.clamp(0.0, MAX_KEEP) / (1.0 - f.clamp(0.0, MAX_KEEP));
        match chem.matter() {
            // (A solid it builds: what its share in what it builds makes of its matter, `share`.)
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
    /// What the body is built of as a material (Bond, 16 Oct 2026), all members: [volume m^3, the sum of volume x Young's modulus,
    /// the sum of volume x the stress it breaks at] over its living soft tissue (`built` over `Physics::rho_body`, at the numbers of
    /// body matter) and each solid it builds (`Chemistry::materials`). Its fuel and free liquid are in none of them.
    pub fn solids(&self, chem: &Chemistry, cfg: &crate::Config) -> [f64; 3] {
        let Some(m) = chem.matter() else { return [0.0; 3] };
        let soft = self.built(chem).max(0.0) / cfg.phys.rho_body;
        let of = |v: f64, s: &crate::chem::Substance| [v, v * s.modulus, v * s.rupture];
        chem.materials().map(|k| of(self.store[k].max(0.0) * chem.subs[k].molar_mass / chem.subs[k].density[0], &chem.subs[k])).fold(of(soft, &chem.subs[m]), |a, x| [a[0] + x[0], a[1] + x[1], a[2] + x[2]])
    }
    /// A member's height, m: what what it is built of holds up against its own weight (Bond, 16 Oct 2026), the order of light in
    /// `shade`. A column standing under its own weight buckles past h^3 = 7.837 E I / w (Greenhill 1881; McMahon 1973 found
    /// trees near a quarter of it), w its weight per length; with its solids side by side in one round section (I = A^2 / 4 pi, E
    /// their mean by volume, A = V / h), h^4 = 7.837 / 4 pi x (sum of E V) x V / (m g) (`solids`), m the member's whole mass (its
    /// fuel and liquid are load too). Soft tissue alone holds a low body: 0.4 m for a 1 kg one at 1 MPa. Without body matter:
    /// (mass / `rho_body`)^(1/3).
    pub fn height(&self, chem: &Chemistry, cfg: &crate::Config) -> f64 {
        let (n, mass) = (self.n(), self.mass(chem));
        if chem.matter().is_none() {
            return (mass / n / cfg.phys.rho_body).cbrt();
        }
        let [v, ev, _] = self.solids(chem, cfg);
        (7.837 / (4.0 * std::f64::consts::PI) * ev * v / (n * mass * cfg.gravity).max(1e-30)).max(0.0).powf(0.25)
    }
    /// The energy to break the share `f` of the body, J (Bond, 16 Oct 2026): what it is built of times the stress each breaks at
    /// (`solids`, J per m^3: the work to crush a solid through its own size), so hard parts protect. A bite pays it
    /// (`choose::take_body`); a mover pushing through breaks as much as the energy it spends there pays for (`choose::wear`).
    pub fn breaking(&self, chem: &Chemistry, cfg: &crate::Config, f: f64) -> f64 {
        f * self.solids(chem, cfg)[2]
    }
    /// The share of solid `k` in what the body builds (`Chemistry::builds`, 16 Oct 2026): its keep gene of `k`, 0..1, the
    /// shares cut together to 0.95 at most; the rest is matter. `usize::MAX`: their sum.
    pub fn share(&self, chem: &Chemistry, k: usize) -> f64 {
        let keep = Layout::of(chem).keep;
        let all: f64 = chem.materials().map(|x| self.gene(keep + x).clamp(0.0, 1.0)).sum();
        let s = if k == usize::MAX { all } else { self.gene(keep + k).clamp(0.0, 1.0) };
        s * (MAX_KEEP / all.max(1e-30)).min(1.0)
    }
    /// Nothing left of a member (under 1 g each) when the mass is `m`.
    pub(crate) fn empty_at(&self, m: f64) -> bool {
        m < 1e-3 * self.n()
    }
    /// A store below its safe minimum, or nothing left of a member.
    pub fn lacking(&self, chem: &Chemistry) -> bool {
        self.lacking_at(chem, self.mass(chem))
    }
    /// `lacking` when the mass `m` is known.
    pub fn lacking_at(&self, chem: &Chemistry, m: f64) -> bool {
        // (A safe minimum is a share of the body, not of the whole mass: a body is not killed by holding fuel.)
        let (lay, size) = (Layout::of(chem), self.size(chem));
        self.empty_at(m) || (0..chem.ns()).any(|k| !self.burns(chem, k) && self.store[k] * chem.subs[k].molar_mass < self.gene(lay.min + k) * size)
    }
}
