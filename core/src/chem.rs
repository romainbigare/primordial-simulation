//! The world's chemistry: data the laws read. Substances are numbers only.

use serde::{Deserialize, Serialize};

pub const R_GAS: f64 = 8.314;
/// Atmospheric reference pressure used for saturation (Pa).
pub const P_REF: f64 = 101_325.0;
/// Temperature at which a gas's solubility is given, K (where tables give it).
pub const T_SOL: f64 = 298.0;

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
    /// Solubility in liquids: the mole fraction that dissolves (for a gas, under the reference
    /// pressure of it alone at `T_SOL`; `Chemistry::henry`)
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
    /// Young's modulus of the solid as a material, Pa (16 Oct 2026; 0: none given): how stiff what bodies build of it is
    /// (`Body::height`). For body matter, that of living soft tissue with its liquid.
    #[serde(default)]
    pub modulus: f64,
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
    #[serde(default)]
    pub ord: Vec<u16>,
}

impl Reaction {
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
    #[serde(default)]
    pub build: Option<Reaction>,
    /// The other solids bodies can build (16 Oct 2026): each a conversion like `build`, run by Copy with the same work
    /// (`copy::grow`); its first output is the solid. A body's keep gene of that solid is its share in what the body
    /// builds (`Body::share`); the rest is matter. Empty: bodies build matter only.
    #[serde(default)]
    pub builds: Vec<Reaction>,
    /// Mole fractions of the starting air, per substance (empty: none known). The cost of a
    /// catalyst counts the gases of a reaction beyond first order at these shares (`Body::prepare`).
    #[serde(default)]
    pub air: Vec<f64>,
    /// What the loose ground of every land cell is given when life starts, mol per m² per substance: the
    /// nitrogen mineral of the soil, 0.357 mol (10 g of nitrogen) in chemistry 0 (`worldgen::life::soil`). Empty: none.
    #[serde(default)]
    pub soil: Vec<(usize, f64)>,
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

    /// Mole fraction of gas `k` that a liquid at `t` holds under its partial pressure `p`, Pa
    /// (Henry's law). It falls with warmth by van 't Hoff's law, with the heat of solution taken
    /// as the vaporisation enthalpy: dissolving is like condensing (for the carbon gas of
    /// chemistry 0 that gives 2,100 K for the 2,400 K measured; for the others 800 to 1,200 K
    /// for 1,300 to 1,700 K).
    pub fn henry(&self, k: usize, t: f64, p: f64) -> f64 {
        let s = &self.subs[k];
        s.solubility * p / P_REF * (s.lv / R_GAS * (1.0 / t.max(1.0) - 1.0 / T_SOL)).exp()
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

    /// Does reaction `r` get less heat out of a solid or liquid it takes than a burning of it does (a reaction that releases heat
    /// from it with a gas)? Then it keeps the rest in what it makes (a fuel gas, a mineral): it is no burning, and a body runs it at
    /// its catalyst's pace and never pays its upkeep with its heat (`Body::prepare`).
    pub fn keeps(&self, r: usize) -> bool {
        let gas = |k: u16| self.phase(k as usize, crate::T_SURF) == Phase::Gas;
        let per = |q: usize, k: u16| self.reactions[q].inputs.iter().find(|p| p.0 == k).map_or(f64::NEG_INFINITY, |p| -self.dh(q) / p.1 as f64);
        let burning = |q: usize| self.dh(q) < 0.0 && self.reactions[q].inputs.iter().any(|p| gas(p.0));
        self.reactions[r].inputs.iter().filter(|p| !gas(p.0)).any(|p| (0..self.reactions.len()).any(|q| burning(q) && per(q, p.0) > (1.0 + 1e-9) * per(r, p.0)))
    }

    /// Enthalpy change of one unit of growth, J (positive takes energy): 0 in
    /// chemistry 0, where 6 of the energy-rich solid make 1 of matter and 1 of liquid (hf of the
    /// matter = 6 hf of the solid less hf of the liquid).
    pub fn dh_build(&self) -> f64 {
        self.build.as_ref().map_or(0.0, |b| self.dh_of(b))
    }

    /// Enthalpy change of one unit of a conversion (`build`, `builds`), J.
    pub fn dh_of(&self, b: &Reaction) -> f64 {
        let sum = |v: &[(u16, u16)]| v.iter().map(|&(k, n)| n as f64 * self.subs[k as usize].hf).sum::<f64>();
        sum(&b.outputs) - sum(&b.inputs)
    }

    /// The solids bodies build besides their matter (the first output of each of `builds`).
    pub fn materials(&self) -> impl Iterator<Item = usize> + '_ {
        self.builds.iter().filter_map(|b| b.outputs.first()).map(|p| p.0 as usize)
    }

    /// The body matter: the first output of growth (`build`), if bodies grow.
    pub fn matter(&self) -> Option<usize> {
        self.build.as_ref().and_then(|b| b.outputs.first()).map(|p| p.0 as usize)
    }

    /// What growth (`build`) makes body matter of besides the fuel (the nitrogen mineral and the salt of the rock element, both
    /// of the ground): each one's substance and the mol of it a mol of matter is made of. They are the inputs that no
    /// reaction of the chemistry burns (releases heat from with a gas, as a fuel burns), so a body takes them from the
    /// ground or what it eats and does not burn them. Empty where bodies do not grow or grow from one fuel.
    pub fn minerals(&self) -> Vec<(usize, f64)> {
        let Some(bl) = self.build.as_ref() else { return Vec::new() };
        let gas = |k: u16| self.phase(k as usize, crate::T_SURF) == Phase::Gas;
        let burns = |k: u16| self.reactions.iter().enumerate().any(|(r, rx)| self.dh(r) < 0.0 && rx.inputs.iter().any(|p| p.0 == k) && rx.inputs.iter().any(|p| gas(p.0)));
        bl.inputs.iter().filter(|p| !burns(p.0)).map(|p| (p.0 as usize, p.1 as f64 / bl.outputs[0].1 as f64)).collect()
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
        for b in self.build.iter().chain(&self.builds) {
            if self.side_elems(&b.inputs) != self.side_elems(&b.outputs) {
                return Err("growth does not balance".into());
            }
            if b.outputs.is_empty() || b.inputs.is_empty() {
                return Err("growth needs inputs and a body matter".into());
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
