//! Physical constants, units and the fixed temperatures the laws refer to.

/// Seconds in a day.
pub const DAY: f64 = 86_400.0;

/// Gas constant, J/(mol K).
pub const R_GAS: f64 = 8.314;

/// Stefan-Boltzmann constant, W/(m² K⁴): every surface emits σT⁴ (emissivity 1).
pub const SIGMA: f64 = 5.670e-8;

/// Reference pressure at which saturation and solubility are given, Pa.
pub const P_REF: f64 = 101_325.0;

/// Temperature at which a gas's solubility is given, K (where tables give it).
pub const T_SOL: f64 = 298.0;

/// A typical surface temperature, K: where the upkeep of catalysts is measured (`Physics::t_cat`)
/// and where the starting kinds and the richness checks look at the phase of a substance.
pub const T_SURF: f64 = 290.0;

/// The coldest a ground node may be taken by drawing heat from it, K.
pub const T_FLOOR: f64 = 3.0;

/// Most substances a chemistry may have: sets of substances are bits of a `u64`, and some
/// buffers hold one number per substance in a fixed array.
pub const MAX_SUBSTANCES: usize = 64;
