//! The world's chemistry: data the laws read; substances are numbers only. `thermo`: phases, melting, vapour pressure,
//! solubility, heat capacity; `roles`: what the reactions do for life.

mod roles;
mod thermo;

use crate::{params::require, units::{MAX_REACTIONS, MAX_SUBSTANCES}};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// A reaction keeps mass where its two sides differ by no more than this share (the sums of molar masses round).
const MASS_ROUNDING: f64 = 1e-12;
/// What a change of amounts may lose or gain of an element to rounding, relative to all the elements it changes.
const ELEMENT_ROUNDING: f64 = 1e-9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Solid,
    Liquid,
    Gas,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Substance {
    /// Count of each base element.
    pub elems: Vec<u16>,
    /// kg/mol
    pub molar_mass: f64,
    /// J/(mol K)
    pub cp: f64,
    /// K
    pub melt: f64,
    /// K
    pub boil: f64,
    /// kg/m³ as [solid, liquid]
    pub density: [f64; 2],
    /// The temperature at which its pure liquid is densest, K (at or below its melting point: densest where it freezes)
    pub dense: f64,
    /// How far what is dissolved in it lowers that temperature, K per kg dissolved per kg of it (Despretz's law)
    pub dense_drop: f64,
    /// 0..1
    pub hardness: f64,
    /// Bond strength of the solid, 0..1
    pub strength: f64,
    /// Visible reflectance in 3 bands as [solid, liquid]
    pub refl: [[f32; 3]; 2],
    /// Absorption of outgoing heat radiation per column amount (m²/mol), gas only
    pub ir_abs: f64,
    /// Column amount past which absorption grows as the square root (bands fill up), mol/m²; 0: never
    pub ir_sat: f64,
    /// Solubility in liquids: the mole fraction that dissolves (a gas: alone at the reference pressure, `T_SOL`)
    pub solubility: f64,
    /// Heat conductivity of the solid, W/(m K)
    pub conductivity: f64,
    /// Vaporisation enthalpy, J/mol (sets the vapour pressure curve)
    pub lv: f64,
    /// Fusion enthalpy, J/mol
    pub lf: f64,
    /// Formation enthalpy of the solid, J/mol (a liquid holds `lf` more in its holder's heat, a gas also `lv`)
    pub hf: f64,
    /// The stress at which the solid as a material breaks, Pa (0: none given; body matter: soft tissue with its liquid)
    pub rupture: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reaction {
    /// (substance, count)
    pub inputs: Vec<(u16, u16)>,
    pub outputs: Vec<(u16, u16)>,
    /// Speed factor, 1/s
    pub k: f64,
    /// Energy barrier as a temperature, K
    pub ea: f64,
    /// Takes absorbed light instead of heat
    pub light: bool,
    /// The decomposers' catalyst of the open world (ground, liquid, air and parts: wherever no body's catalysts run it),
    /// a factor that works as far as the node is moist (`change::open_catalyst`); 1 for a reaction they do not speed
    pub open: f64,
    /// The power of each input's share in the rate law; empty: the counts (mass action). A big molecule is not a
    /// meeting of 40 molecules: the rate law keeps the power it was measured at.
    pub ord: Vec<u16>,
}

impl Reaction {
    /// A conversion (`Chemistry::build`, `builds`): balanced in elements like a reaction, with no rate.
    pub fn conversion(inputs: Vec<(u16, u16)>, outputs: Vec<(u16, u16)>) -> Self {
        Reaction { inputs, outputs, k: 0.0, ea: 0.0, light: false, open: 1.0, ord: Vec::new() }
    }

    /// Its rate constant at `t` K (Arrhenius): k exp(-Ea / T), per second.
    #[inline]
    pub fn arrhenius(&self, t: f64) -> f64 {
        self.k * (-self.ea / t).exp()
    }

    /// The power of the share of input `q` in the rate law (`ord`, else its count).
    #[inline]
    pub fn order(&self, q: usize) -> u16 {
        self.ord.get(q).copied().unwrap_or(self.inputs[q].1)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chemistry {
    pub n_elements: usize,
    /// The substances, fixed after the chemistry's first use: what follows from them (`Derived`) is made once (a test
    /// that changes one builds the chemistry again, as a saved one is read back)
    pub subs: Vec<Substance>,
    pub reactions: Vec<Reaction>,
    /// How life grows: a conversion of fuel and minerals into body matter (its first output), with no rate, paid by work
    /// (`cover::build`). None: life has no matter apart from its fuel and does not grow.
    pub build: Option<Reaction>,
    /// The other solids life can build, each a conversion like `build` whose first output is the solid (the cover's walls).
    pub builds: Vec<Reaction>,
    /// Mole fractions of the starting air, per substance (empty: none known), for its heat capacity (`Chemistry::air_cp`).
    pub air: Vec<f64>,
    /// What follows from the fields above, made at its first use (not saved)
    #[serde(skip)]
    derived: OnceLock<Derived>,
}

/// Numbers that follow from a chemistry, made once (`Chemistry::derived`).
#[derive(Clone, Debug)]
struct Derived {
    /// `Chemistry::minerals`
    minerals: Vec<(usize, f64)>,
    /// `Chemistry::air_cp`
    air_cp: f64,
    /// `Chemistry::life_roles` and `Chemistry::fixing`
    roles: Option<(usize, usize, usize, usize)>,
    fixing: Option<usize>,
    /// `Chemistry::mixes` of each substance
    mixes: Vec<Option<Vec<(usize, bool, bool)>>>,
    /// The substances others dissolve in (`Chemistry::mixes` is some), a bit per substance
    solvents: u64,
    /// The light reactions, by number
    light: Vec<usize>,
}

impl Chemistry {
    /// A chemistry whose starting air has the mole fractions `air` (substance, fraction); the other substances none.
    pub fn new(n_elements: usize, subs: Vec<Substance>, reactions: Vec<Reaction>, build: Option<Reaction>, builds: Vec<Reaction>, air: &[(usize, f64)]) -> Self {
        let mut shares = vec![0.0; subs.len()];
        air.iter().for_each(|&(k, x)| shares[k] = x);
        Chemistry { n_elements, subs, reactions, build, builds, air: shares, derived: OnceLock::new() }
    }

    pub fn ns(&self) -> usize {
        self.subs.len()
    }

    /// What follows from the chemistry (`Derived`), made at the first call (`derive`).
    #[inline]
    fn derived(&self) -> &Derived {
        match self.derived.get() {
            Some(d) => d,
            None => self.derive(),
        }
    }

    /// `derived` made, at the first call.
    #[cold]
    fn derive(&self) -> &Derived {
        self.derived.get_or_init(|| {
            // The inputs of growth no reaction burns, per mol of matter; the starting air's heat capacity per mol.
            let minerals = self.build.as_ref().map_or(Vec::new(), |bl| bl.inputs.iter().filter(|p| self.burner(p.0 as usize).is_none()).map(|p| (p.0 as usize, p.1 as f64 / bl.outputs[0].1 as f64)).collect());
            let air_cp: f64 = self.air.iter().zip(&self.subs).map(|(x, s)| x * s.cp).sum();
            let air_cp = if air_cp > 0.0 { air_cp } else { thermo::DIATOMIC_CP };
            let mixes: Vec<_> = (0..self.ns()).map(|k| self.find_mixes(k)).collect();
            let solvents = (0..self.ns()).filter(|&k| mixes[k].is_some()).fold(0, |m, k| m | 1 << k);
            let light = (0..self.reactions.len()).filter(|&r| self.reactions[r].light).collect();
            let (roles, fixing) = (self.find_life_roles(), self.find_fixing(&minerals));
            Derived { minerals, air_cp, roles, fixing, mixes, solvents, light }
        })
    }


    /// Element counts of a reaction side.
    fn side_elems(&self, v: &[(u16, u16)]) -> Vec<u32> {
        let mut e = vec![0u32; self.n_elements];
        for &(k, n) in v {
            self.subs[k as usize].elems.iter().enumerate().for_each(|(j, &c)| e[j] += c as u32 * n as u32);
        }
        e
    }

    /// Is the chemistry one the laws can run? At most `MAX_SUBSTANCES` substances with element counts, a vapour pressure
    /// curve and densities, and at most `MAX_REACTIONS` reactions; every reaction and conversion names substances that
    /// exist and keeps its elements and mass.
    pub fn validate(&self) -> Result<(), String> {
        let ns = self.ns();
        require(ns <= MAX_SUBSTANCES, || format!("{ns} substances: at most {MAX_SUBSTANCES}"))?;
        let nr = self.reactions.len();
        require(nr <= MAX_REACTIONS, || format!("{nr} reactions: at most {MAX_REACTIONS}"))?;
        for (k, s) in self.subs.iter().enumerate() {
            require(s.elems.len() == self.n_elements, || format!("substance {k}: wrong element count"))?;
            require(s.lv > 0.0, || format!("substance {k}: vaporisation enthalpy must be above 0"))?;
            require(s.density.iter().all(|&d| d > 0.0), || format!("substance {k}: densities must be above 0"))?;
            require(s.dense > 0.0 && s.dense_drop >= 0.0, || format!("substance {k}: the temperature of its densest liquid must be above 0, its lowering not below"))?;
        }
        // Each reaction, then each conversion: a conversion needs inputs and a product, a reaction no more orders than
        // inputs. Its molar masses must agree with its elements to rounding: it keeps mass.
        let mass = |v: &[(u16, u16)]| v.iter().map(|&(k, n)| n as f64 * self.subs[k as usize].molar_mass).sum::<f64>();
        let reactions = self.reactions.iter().enumerate().map(|(r, rx)| (format!("reaction {r}"), rx, false));
        let builds = self.build.iter().map(|b| (String::from("build"), b, true));
        for (name, rx, conversion) in reactions.chain(builds).chain(self.builds.iter().enumerate().map(|(q, b)| (format!("builds {q}"), b, true))) {
            let whole = !conversion || !(rx.outputs.is_empty() || rx.inputs.is_empty());
            require(whole, || format!("{name} needs inputs and a product"))?;
            let known = rx.inputs.iter().chain(&rx.outputs).all(|p| (p.0 as usize) < ns);
            require(known, || format!("{name}: a substance beyond the {ns}"))?;
            require(rx.ord.len() <= rx.inputs.len(), || format!("{name}: more orders than inputs"))?;
            require(rx.open > 0.0, || format!("{name}: its speed in the open world must be above 0"))?;
            let balanced = self.side_elems(&rx.inputs) == self.side_elems(&rx.outputs);
            require(balanced, || format!("{name} does not balance"))?;
            let (m_in, m_out) = (mass(&rx.inputs), mass(&rx.outputs));
            require((m_in - m_out).abs() <= MASS_ROUNDING * m_in.max(m_out), || format!("{name} does not keep mass"))?;
        }
        Ok(())
    }

    /// Add element totals of `amount` moles of substance `k` into `out` (of the elements it holds).
    #[inline]
    pub fn add_elems(&self, k: usize, amount: f64, out: &mut [f64]) {
        self.subs[k].elems.iter().enumerate().filter(|e| *e.1 != 0).for_each(|(j, &c)| out[j] += c as f64 * amount);
    }

    /// Does `dmol` (substance, mol) keep every element of the table, to rounding? `sums` is room for the element totals.
    pub(crate) fn keeps_elements(&self, dmol: impl IntoIterator<Item = (u16, f64)>, sums: &mut [f64]) -> bool {
        let mut size = 0.0;
        sums.fill(0.0);
        for (k, d) in dmol {
            self.add_elems(k as usize, d, sums);
            size += d.abs() * self.subs[k as usize].elems.iter().map(|&c| c as f64).sum::<f64>();
        }
        sums.iter().all(|e| e.abs() <= ELEMENT_ROUNDING * size)
    }
}
