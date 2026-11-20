//! Physical constants, units and the fixed temperatures the laws refer to.

/// Seconds in a day, the clock's whole number (`clock`).
pub const DAY_S: u64 = 86_400;

/// Seconds in a day, as a real number.
pub const DAY: f64 = DAY_S as f64;

/// Units the laws fitted on the one planet known are given in: m² in a km², m³ in a km³, m in a km, kg in a Mt.
pub const M2_PER_KM2: f64 = 1e6;
pub const M3_PER_KM3: f64 = 1e9;
pub const M_PER_KM: f64 = 1e3;
pub const KG_PER_MT: f64 = 1e9;
/// Grams in a kilogram, the unit a shell is given the load a liquid carries in (g/m³).
pub const G_PER_KG: f64 = 1e3;

/// The planet's spin, rad/s: one turn in a sidereal day of 86,164 s.
pub const SPIN: f64 = 7.292e-5;

/// Gas constant, J/(mol K).
pub const R_GAS: f64 = 8.314;

/// A gas's diffusivity in the air, m²/s, for a gas of molar mass `GAS_DIFFUSIVITY_MM` at 293 K and `P_REF`; another
/// gas's by Graham's law, as the root of the ratio of their molar masses (`cover::soak`). It grows as T^1.75 / p (Fuller,
/// Schettler and Giddings 1966), left out: within about 15% over the ground's 270 to 310 K.
pub const GAS_DIFFUSIVITY: f64 = 2.0e-5;

/// The molar mass `GAS_DIFFUSIVITY` is given for, kg/mol.
pub const GAS_DIFFUSIVITY_MM: f64 = 0.032;

/// Stefan-Boltzmann constant, W/(m² K⁴): every surface emits σT⁴ (emissivity 1).
pub const SIGMA: f64 = 5.670e-8;

/// Planck's second radiation constant h c / k, m K.
pub const C2: f64 = 1.438_777e-2;

/// Reference pressure at which saturation and solubility are given, Pa.
pub const P_REF: f64 = 101_325.0;

/// Temperature at which a gas's solubility is given, K (where tables give it).
pub const T_SOL: f64 = 298.0;

/// A typical surface temperature, K: where catalysts' upkeep is measured and phases are judged for the start.
pub const T_SURF: f64 = 290.0;

/// The kinematic viscosity of the key liquid at `T_SURF`, m²/s: what a particle settling through it meets (`soil`).
pub const LIQUID_VISCOSITY: f64 = 1.0e-6;

/// The coldest a ground node may be taken by drawing heat from it, K.
pub const T_FLOOR: f64 = 3.0;

/// Most substances a chemistry may have: sets of substances are bits of a `u64`.
pub const MAX_SUBSTANCES: usize = 64;

/// Most reactions a chemistry may have.
pub const MAX_REACTIONS: usize = 64;

/// The energy a living tissue's fuel frees for it where no oxidising gas reaches it, over what it frees burnt with that
/// gas: glycolysis and fermentation make 2 ATP from a glucose, its full burning about 32 (Berg, Tymoczko and Stryer,
/// Biochemistry; flooded roots: Bailey-Serres and Voesenek 2008).
pub const FERMENT_YIELD: f64 = 2.0 / 32.0;
