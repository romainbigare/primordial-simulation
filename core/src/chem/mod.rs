//! The world's chemistry: data the laws read. Substances are numbers only.
//!
//! `thermo`: phases, melting and vapour pressure, solubility, heat capacity. `roles`: what the reactions do for bodies
//! (which burn, which keep their energy, what bodies build of).

mod roles;
mod thermo;

use crate::units::MAX_SUBSTANCES;
use serde::{Deserialize, Serialize};

/// A reaction keeps mass where its two sides differ by no more than this share (the sums of molar masses round).
const MASS_ROUNDING: f64 = 1e-12;

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
    /// 0..1
    pub hardness: f64,
    /// Bond strength of the solid, 0..1
    pub strength: f64,
    /// Visible reflectance in 3 bands as [solid, liquid]
    pub refl: [[f32; 3]; 2],
    /// Absorption of outgoing heat radiation per column amount (m²/mol), gas only
    pub ir_abs: f64,
    /// Column amount past which the absorption grows only as the square root of the amount
    /// (the gas's bands fill up, a stand-in for the logarithm of real gases), mol/m²; 0: never
    pub ir_sat: f64,
    /// Solubility in liquids: the mole fraction that dissolves (for a gas, under the reference
    /// pressure of it alone at `T_SOL`; `Chemistry::henry`)
    pub solubility: f64,
    /// Heat conductivity of the solid, W/(m K)
    pub conductivity: f64,
    /// Vaporisation enthalpy, J/mol (sets the vapour pressure curve)
    pub lv: f64,
    /// Fusion enthalpy, J/mol
    pub lf: f64,
    /// Formation enthalpy of the solid, J/mol: the chemical energy one mole holds (a liquid holds `lf` more, in its holder's
    /// heat; a gas also `lv`, in the air: `check::totals`).
    pub hf: f64,
    /// Young's modulus of the solid as a material, Pa (0: none given): how stiff what bodies build of it is
    /// (`Body::height`). For body matter, that of living soft tissue with its liquid.
    pub modulus: f64,
    /// The stress at which the solid as a material breaks, Pa (0: none given): J per m³ to break what bodies
    /// build of it (`Body::solids`). For body matter, that of living soft tissue with its liquid.
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
    /// The power each input's share enters the rate law at, in the order of `inputs`; empty: the counts of
    /// `inputs`, as mass action says. A reaction of a big molecule is not one meeting of 40 molecules: the
    /// rate law keeps the power it was measured at (`order`), whatever the reaction takes in all.
    pub ord: Vec<u16>,
}

impl Reaction {
    /// A conversion (`Chemistry::build`, `builds`): balanced in elements like a reaction, with no rate.
    pub fn conversion(inputs: Vec<(u16, u16)>, outputs: Vec<(u16, u16)>) -> Self {
        Reaction { inputs, outputs, k: 0.0, ea: 0.0, light: false, ord: Vec::new() }
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
    pub subs: Vec<Substance>,
    pub reactions: Vec<Reaction>,
    /// How bodies grow: a conversion of stores into body matter, balanced in elements like a
    /// reaction, with no rate (Copy runs it, paid by work, `copy::grow`). Its first output is the
    /// body matter; the others are what is left over. None: bodies have no matter apart from
    /// their stores (`Body::size` is their mass, and they do not grow).
    pub build: Option<Reaction>,
    /// The other solids bodies can build: each a conversion like `build`, run by Copy with the same work
    /// (`copy::grow`); its first output is the solid. A body's keep gene of that solid is its share in what the body
    /// builds (`Body::share`); the rest is matter. Empty: bodies build matter only.
    pub builds: Vec<Reaction>,
    /// Mole fractions of the starting air, per substance (empty: none known). The cost of a
    /// catalyst counts the gases of a reaction beyond first order at these shares (`Body::prepare`).
    pub air: Vec<f64>,
}

impl Chemistry {
    /// A chemistry whose starting air has the mole fractions `air` (substance, fraction); the other substances none.
    pub fn new(n_elements: usize, subs: Vec<Substance>, reactions: Vec<Reaction>, build: Option<Reaction>, builds: Vec<Reaction>, air: &[(usize, f64)]) -> Self {
        let mut shares = vec![0.0; subs.len()];
        air.iter().for_each(|&(k, x)| shares[k] = x);
        Chemistry { n_elements, subs, reactions, build, builds, air: shares }
    }

    pub fn ns(&self) -> usize {
        self.subs.len()
    }

    /// Element counts of a reaction side.
    fn side_elems(&self, v: &[(u16, u16)]) -> Vec<u32> {
        let mut e = vec![0u32; self.n_elements];
        for &(k, n) in v {
            for (j, &c) in self.subs[k as usize].elems.iter().enumerate() {
                e[j] += c as u32 * n as u32;
            }
        }
        e
    }

    /// Mass of a reaction side, kg.
    fn side_mass(&self, v: &[(u16, u16)]) -> f64 {
        v.iter().map(|&(k, n)| n as f64 * self.subs[k as usize].molar_mass).sum()
    }

    /// Does a reaction or conversion keep mass (its molar masses agree with its elements), to rounding?
    fn keeps_mass(&self, rx: &Reaction) -> bool {
        let (a, b) = (self.side_mass(&rx.inputs), self.side_mass(&rx.outputs));
        (a - b).abs() <= MASS_ROUNDING * a.max(b)
    }

    /// Is the chemistry one the laws can run? At most `MAX_SUBSTANCES` substances, each with its element counts, a
    /// vapour pressure curve and densities; every reaction and conversion names substances that exist and balances its
    /// elements and its mass; every conversion has inputs and a product.
    pub fn validate(&self) -> Result<(), String> {
        let ns = self.ns();
        if ns > MAX_SUBSTANCES {
            return Err(format!("{ns} substances: at most {MAX_SUBSTANCES}"));
        }
        for (k, s) in self.subs.iter().enumerate() {
            if s.elems.len() != self.n_elements {
                return Err(format!("substance {k}: wrong element count"));
            }
            if s.lv <= 0.0 {
                return Err(format!("substance {k}: vaporisation enthalpy must be above 0"));
            }
            if s.density.iter().any(|&d| d <= 0.0) {
                return Err(format!("substance {k}: densities must be above 0"));
            }
        }
        let known = |rx: &Reaction| rx.inputs.iter().chain(&rx.outputs).all(|p| (p.0 as usize) < ns);
        for (r, rx) in self.reactions.iter().enumerate() {
            if !known(rx) {
                return Err(format!("reaction {r}: a substance beyond the {ns}"));
            }
            if rx.ord.len() > rx.inputs.len() {
                return Err(format!("reaction {r}: more orders than inputs"));
            }
            if self.side_elems(&rx.inputs) != self.side_elems(&rx.outputs) {
                return Err(format!("reaction {r} does not balance"));
            }
            if !self.keeps_mass(rx) {
                return Err(format!("reaction {r} does not keep mass"));
            }
        }
        let named = self.build.iter().map(|b| (String::from("build"), b));
        for (name, b) in named.chain(self.builds.iter().enumerate().map(|(q, b)| (format!("builds {q}"), b))) {
            if b.outputs.is_empty() || b.inputs.is_empty() {
                return Err(format!("{name} needs inputs and a product"));
            }
            if !known(b) {
                return Err(format!("{name}: a substance beyond the {ns}"));
            }
            if self.side_elems(&b.inputs) != self.side_elems(&b.outputs) {
                return Err(format!("{name} does not balance"));
            }
            if !self.keeps_mass(b) {
                return Err(format!("{name} does not keep mass"));
            }
        }
        Ok(())
    }

    /// Add element totals of `amount` moles of substance `k` into `out`.
    #[inline]
    pub fn add_elems(&self, k: usize, amount: f64, out: &mut [f64]) {
        for (j, &c) in self.subs[k].elems.iter().enumerate() {
            out[j] += c as f64 * amount;
        }
    }
}
