//! Room the passes of Flow keep from step to step (`Spare`): no state, each pass writes all it reads of it first, or
//! reads only where to start a work whose result does not depend on it.

/// Room for the rows of passes over every cell, kept so that a step does not ask the allocator for megabytes anew (at
/// 128 x 128 a column of the cells is 128 kB).
#[derive(Clone, Debug, Default)]
pub(super) struct Spare {
    /// The rows of the state of ground nodes (`land::ground_state`)
    pub(super) state: Vec<f64>,
    /// The rows of the cells' response to the air (`air::cells::Response`)
    pub(super) response: Vec<f64>,
    /// What runs through each of the terrain's samples in the runoff's pass, mol and kg (`runoff::surface`)
    pub(super) samples: Vec<[f64; 2]>,
    /// The order each body's floors took at its last level (`level::level_over`): where its sort starts
    pub(super) orders: Vec<Vec<u32>>,
}
