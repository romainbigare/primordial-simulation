//! Physical constants, units and the fixed temperatures the laws refer to.

/// Seconds in a day, the clock's whole number (`clock`).
pub const DAY_S: u64 = 86_400;

/// Seconds in a day, as a real number.
pub const DAY: f64 = DAY_S as f64;

/// Gas constant, J/(mol K).
pub const R_GAS: f64 = 8.314;

/// Stefan-Boltzmann constant, W/(m² K⁴): every surface emits σT⁴ (emissivity 1).
pub const SIGMA: f64 = 5.670e-8;

/// Reference pressure at which saturation and solubility are given, Pa.
pub const P_REF: f64 = 101_325.0;

/// Temperature at which a gas's solubility is given, K (where tables give it).
pub const T_SOL: f64 = 298.0;

/// A typical surface temperature, K: where catalysts' upkeep is measured and phases are judged for the start.
pub const T_SURF: f64 = 290.0;

/// The coldest a ground node may be taken by drawing heat from it, K.
pub const T_FLOOR: f64 = 3.0;

/// Most substances a chemistry may have: sets of substances are bits of a `u64`.
pub const MAX_SUBSTANCES: usize = 64;

/// Most reactions a chemistry may have.
pub const MAX_REACTIONS: usize = 64;
