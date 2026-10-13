//! The world's chemistry: data the laws read. Substances are numbers only.

use serde::{Deserialize, Serialize};

pub const R_GAS: f64 = 8.314;
/// Atmospheric reference pressure used for saturation (Pa).
pub const P_REF: f64 = 101_325.0;

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
    #[serde(default)]
    pub ir_sat: f64,
    /// Solubility in liquids (mol fraction that can dissolve)
    pub solubility: f64,
    /// Heat conductivity of the solid, W/(m K)
    pub conductivity: f64,
    /// Vaporisation enthalpy, J/mol (sets the vapour pressure curve)
    pub lv: f64,
    /// Fusion enthalpy, J/mol
    #[serde(default)]
    pub lf: f64,
    /// Formation enthalpy, J/mol. Chemical energy held by one mole.
    pub hf: f64,
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chemistry {
    pub n_elements: usize,
    pub subs: Vec<Substance>,
    pub reactions: Vec<Reaction>,
}

impl Chemistry {
    pub fn ns(&self) -> usize {
        self.subs.len()
    }

    pub fn phase(&self, k: usize, t: f64) -> Phase {
        let s = &self.subs[k];
        if t < s.melt {
            Phase::Solid
        } else if t < s.boil {
            Phase::Liquid
        } else {
            Phase::Gas
        }
    }

    /// m³/mol of a condensed phase (gas has no fixed volume here).
    pub fn molar_volume(&self, k: usize, p: Phase) -> f64 {
        let s = &self.subs[k];
        match p {
            Phase::Solid => s.molar_mass / s.density[0],
            Phase::Liquid => s.molar_mass / s.density[1],
            Phase::Gas => 0.0,
        }
    }

    /// Equilibrium vapour pressure (Pa) from the boiling point (Clausius-Clapeyron).
    pub fn p_sat(&self, k: usize, t: f64) -> f64 {
        let (tb, l) = (self.subs[k].boil, self.subs[k].lv);
        P_REF * (l / R_GAS * (1.0 / tb - 1.0 / t.max(1.0))).exp()
    }

    /// Temperature at which the vapour pressure of `k` is `x` times the reference pressure, K.
    pub fn t_sat(&self, k: usize, x: f64) -> f64 {
        let (tb, l) = (self.subs[k].boil, self.subs[k].lv);
        1.0 / (1.0 / tb - R_GAS * x.max(1e-30).ln() / l)
    }

    /// Optical depth to outgoing heat radiation of a column of `c` mol/m² of substance `k`.
    pub fn depth(&self, k: usize, c: f64) -> f64 {
        let s = &self.subs[k];
        s.ir_abs * if s.ir_sat > 0.0 && c > s.ir_sat { (s.ir_sat * c).sqrt() } else { c }
    }

    /// Enthalpy change of one reaction unit, J (positive takes energy).
    pub fn dh(&self, r: usize) -> f64 {
        let rx = &self.reactions[r];
        let sum = |v: &[(u16, u16)]| v.iter().map(|&(k, n)| n as f64 * self.subs[k as usize].hf).sum::<f64>();
        sum(&rx.outputs) - sum(&rx.inputs)
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

    /// Every reaction must balance its elements.
    pub fn validate(&self) -> Result<(), String> {
        for (k, s) in self.subs.iter().enumerate() {
            if s.elems.len() != self.n_elements {
                return Err(format!("substance {k}: wrong element count"));
            }
        }
        for (r, rx) in self.reactions.iter().enumerate() {
            if self.side_elems(&rx.inputs) != self.side_elems(&rx.outputs) {
                return Err(format!("reaction {r} does not balance"));
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
