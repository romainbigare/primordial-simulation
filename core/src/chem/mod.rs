//! The world's chemistry: data the laws read; substances are numbers only. `thermo`: phases, melting, vapour pressure,
//! solubility, heat capacity; `roles`: what the reactions do for bodies.

mod roles;
mod thermo;

use crate::units::MAX_SUBSTANCES;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

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
    /// Young's modulus of the solid as a material, Pa (0: none given; body matter: soft tissue with its liquid)
    pub modulus: f64,
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
    /// The power of each input's share in the rate law; empty: the counts (mass action). A big molecule is not a
    /// meeting of 40 molecules: the rate law keeps the power it was measured at.
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
    /// How bodies grow: a conversion of stores into body matter (its first output), with no rate, paid by work
    /// (`copy::grow`). None: bodies have no matter apart from their stores and do not grow.
    pub build: Option<Reaction>,
    /// The other solids bodies can build, each a conversion like `build` whose first output is the solid (`Body::share`).
    pub builds: Vec<Reaction>,
    /// Mole fractions of the starting air, per substance (empty: none known), for the cost of catalysts (`Body::prepare`).
    pub air: Vec<f64>,
    /// What follows from the fields above, made at its first use (not saved)
    #[serde(skip)]
    derived: OnceLock<Derived>,
    /// `Chemistry::contact_rates` at the first temperature asked (not saved)
    #[serde(skip)]
    contact: OnceLock<(f64, Vec<f64>)>,
}

/// Numbers that follow from a chemistry, made once (`Chemistry::derived`).
#[derive(Clone, Debug)]
struct Derived {
    /// `Chemistry::minerals`
    minerals: Vec<(usize, f64)>,
    /// `Chemistry::air_cp`
    air_cp: f64,
    /// `Chemistry::burners`
    burners: Vec<(usize, f64)>,
}

impl Chemistry {
    /// A chemistry whose starting air has the mole fractions `air` (substance, fraction); the other substances none.
    pub fn new(n_elements: usize, subs: Vec<Substance>, reactions: Vec<Reaction>, build: Option<Reaction>, builds: Vec<Reaction>, air: &[(usize, f64)]) -> Self {
        let mut shares = vec![0.0; subs.len()];
        air.iter().for_each(|&(k, x)| shares[k] = x);
        Chemistry { n_elements, subs, reactions, build, builds, air: shares, derived: OnceLock::new(), contact: OnceLock::new() }
    }

    pub fn ns(&self) -> usize {
        self.subs.len()
    }

    /// What follows from the chemistry (`Derived`), made at the first call.
    fn derived(&self) -> &Derived {
        self.derived.get_or_init(|| {
            // The inputs of growth no reaction burns, per mol of matter; the starting air's heat capacity per mol.
            let minerals = self.build.as_ref().map_or(Vec::new(), |bl| bl.inputs.iter().filter(|p| !self.burnt(p.0 as usize)).map(|p| (p.0 as usize, p.1 as f64 / bl.outputs[0].1 as f64)).collect());
            let air_cp: f64 = self.air.iter().zip(&self.subs).map(|(x, s)| x * s.cp).sum();
            let burners = (0..self.reactions.len()).filter(|&r| self.dh(r) < 0.0 && !self.keeps(r)).map(|r| (r, -self.dh(r))).collect();
            Derived { minerals, air_cp: if air_cp > 0.0 { air_cp } else { thermo::DIATOMIC_CP }, burners }
        })
    }

    /// The catalysed rate of each reaction per unit of contact at `t`, its gases at their share of the starting air to
    /// their order, per second (the cost of catalysts, `Body::prepare`); kept for the first `t` asked.
    pub(crate) fn contact_rates(&self, t: f64) -> std::borrow::Cow<'_, [f64]> {
        let open = |rx: &Reaction| -> f64 {
            rx.inputs.iter().enumerate().filter(|(_, x)| self.phase(x.0 as usize, t) == Phase::Gas).map(|(q, &(k, _))| self.air.get(k as usize).copied().filter(|&x| x > 0.0).unwrap_or(1.0).powi(rx.order(q) as i32)).product()
        };
        let rates = || self.reactions.iter().map(|rx| rx.arrhenius(t) * open(rx)).collect::<Vec<f64>>();
        let kept = self.contact.get_or_init(|| (t, rates()));
        if kept.0 == t { std::borrow::Cow::Borrowed(&kept.1) } else { std::borrow::Cow::Owned(rates()) }
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
    /// curve and densities; every reaction and conversion names substances that exist and keeps its elements and mass.
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
        // Each reaction, then each conversion: a conversion needs inputs and a product, a reaction no more orders than
        // inputs. Its molar masses must agree with its elements to rounding: it keeps mass.
        let mass = |v: &[(u16, u16)]| v.iter().map(|&(k, n)| n as f64 * self.subs[k as usize].molar_mass).sum::<f64>();
        let reactions = self.reactions.iter().enumerate().map(|(r, rx)| (format!("reaction {r}"), rx, false));
        let builds = self.build.iter().map(|b| (String::from("build"), b, true));
        for (name, rx, conversion) in reactions.chain(builds).chain(self.builds.iter().enumerate().map(|(q, b)| (format!("builds {q}"), b, true))) {
            if conversion && (rx.outputs.is_empty() || rx.inputs.is_empty()) {
                return Err(format!("{name} needs inputs and a product"));
            }
            if !rx.inputs.iter().chain(&rx.outputs).all(|p| (p.0 as usize) < ns) {
                return Err(format!("{name}: a substance beyond the {ns}"));
            }
            if rx.ord.len() > rx.inputs.len() {
                return Err(format!("{name}: more orders than inputs"));
            }
            if self.side_elems(&rx.inputs) != self.side_elems(&rx.outputs) {
                return Err(format!("{name} does not balance"));
            }
            if (mass(&rx.inputs) - mass(&rx.outputs)).abs() > MASS_ROUNDING * mass(&rx.inputs).max(mass(&rx.outputs)) {
                return Err(format!("{name} does not keep mass"));
            }
        }
        Ok(())
    }

    /// Add element totals of `amount` moles of substance `k` into `out` (of the elements it holds).
    #[inline]
    pub fn add_elems(&self, k: usize, amount: f64, out: &mut [f64]) {
        self.subs[k].elems.iter().enumerate().filter(|e| *e.1 != 0).for_each(|(j, &c)| out[j] += c as f64 * amount);
    }
}
