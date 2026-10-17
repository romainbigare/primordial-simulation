//! The world's state: the fields of ground and air, the bodies and parts, the books of what
//! crossed its boundary, and the counters the shell reads.

use crate::chem::Chemistry;
use crate::event::Event;
use crate::grid::Grid;
use crate::params::Config;
use crate::{bond, flow};
use serde::{Deserialize, Serialize};

/// Energy and matter that crossed the world's boundary, for conservation checks.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Ledger {
    /// Sunlight absorbed, J
    pub sun_in: f64,
    /// Radiation lost to space, J
    pub space_out: f64,
    /// Potential energy turned into heat (negative when matter is lifted), J
    pub gravity: f64,
    /// Energy added by player edits, J
    pub imposed_energy: f64,
    /// Elements added by player edits, mol
    pub imposed_elems: Vec<f64>,
    /// Tiny negative amounts clamped to zero, mol
    pub clamped: f64,
    /// Heat a ground node could not give (it would have fallen below `T_FLOOR`), J: whoever asked
    /// received that much less, so this is not a loss of energy.
    pub clamped_heat: f64,
}

/// Counts and slow averages for the shell only: the laws never read them. The fields not saved
/// (`serde(skip)`) count since the shell last read them.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Diag {
    /// Members moving into each ground cell, per year (moving average over about a year), and
    /// since the last month
    pub traffic: Vec<f32>,
    pub crossings: Vec<f32>,
    /// Decisions drawn, and of them those where the action drawn had the highest score
    #[serde(skip)]
    pub decisions: f64,
    #[serde(skip)]
    pub greedy: f64,
    /// Population nodes split since the start: in two (`copy::split`), and one member off to act
    /// alone (`copy::one`)
    pub splits: [f64; 2],
    /// Members merged by the monthly merge however unlike (the safeguard past twice `Config::crowd`
    /// nodes in a cell, `body::crowd`)
    #[serde(skip)]
    pub unlike: f64,
    /// Members that joined a node (the monthly merge, `body::crowd`, and a child's join, `copy::copy`)
    /// whose means differ from the node's by more than `Physics::alike` steps in some gene
    /// (`copy::alike`)
    #[serde(skip)]
    pub wide: f64,
    /// Members that moved to a neighbour cell, by starting kind (`Lineage::root`): a node moving
    /// counts all its members (`choose::go`)
    #[serde(skip)]
    pub moves: Vec<f64>,
    /// Body matter, kg, that hits tore off members who lived on (bites that left them smaller, bodies
    /// trampled, `body::tear`), by starting kind
    #[serde(skip)]
    pub hurt: Vec<f64>,
    /// Members that could build body matter in an update, by starting kind, as [tried, held back by any mineral, by
    /// the first, by the second...]: held back are those whose store of a mineral (`Chemistry::minerals`) bound what they built,
    /// below what their fuels, their allowance and their spare allowed (`copy::grow`)
    #[serde(skip)]
    pub nlim: Vec<Vec<f64>>,
    /// The land's liquid, m³, by the indices `LAND_*`
    #[serde(skip)]
    pub liquid_land: [f64; 5],
}

impl Diag {
    /// `liquid_land`: rain on land cells
    pub const LAND_RAIN: usize = 0;
    /// evaporation from their ground
    pub const LAND_GROUND: usize = 1;
    /// evaporation from light-eaters
    pub const LAND_PLANTS: usize = 2;
    /// evaporation from the other bodies
    pub const LAND_OTHERS: usize = 3;
    /// what light-eaters took of the ground's liquid (rain they caught and what they soaked)
    pub const LAND_PLANTS_TOOK: usize = 4;

    /// The entry of starting kind `root` (`Lineage::root`) in a tally by kind, grown to hold it; none for a body of no
    /// starting kind (`u32::MAX`).
    pub(crate) fn of_root<T: Clone + Default>(v: &mut Vec<T>, root: u32) -> Option<&mut T> {
        let r = root as usize;
        if root == u32::MAX {
            return None;
        }
        if v.len() <= r {
            v.resize(r + 1, T::default());
        }
        Some(&mut v[r])
    }
}

/// One number per body slot, kept with the generation of the body that set it: a body reads only its own (0 for a new
/// body in a reused slot, and before its first set).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PerSlot(Vec<(u32, f32)>);

impl PerSlot {
    /// The number set by the body in `slot` of generation `gen` (0 if it set none).
    pub fn get(&self, slot: usize, gen: u32) -> f32 {
        self.0.get(slot).filter(|k| k.0 == gen).map_or(0.0, |k| k.1)
    }
    pub fn set(&mut self, slot: usize, gen: u32, v: f32) {
        if self.0.len() <= slot {
            self.0.resize(slot + 1, (u32::MAX, 0.0));
        }
        self.0[slot] = (gen, v);
    }
    /// Every slot's (generation, number); `u32::MAX` for a slot never set.
    pub fn entries(&self) -> &[(u32, f32)] {
        &self.0
    }
}

/// The whole state of a world: what `step` advances, and what a snapshot saves.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct World {
    pub cfg: Config,
    pub chem: Chemistry,
    pub grid: Grid,
    /// Seed of every keyed random draw (`rng::Rng`)
    pub seed: u64,
    /// World clock, days since the start
    pub day: u64,
    /// Loose (unbonded) amounts in ground nodes, [k * n + i], mol
    pub ground: Vec<f64>,
    /// Bonded ground amounts, [k * n + i], mol
    pub bed: Vec<f64>,
    /// Thermal energy of each ground node's active layer, J
    pub e_ground: Vec<f64>,
    /// Amounts in air nodes, [k * na + a], mol
    pub air: Vec<f64>,
    /// Thermal energy of each air node, J
    pub e_air: Vec<f64>,
    /// Elevation of the bottom of the ground column, m
    pub base_z: f64,
    /// Liquid falling on each ground cell, mol/(m² s), a moving average over about a year (`flow::air::exchange`); bodies
    /// catch it (`body::live`)
    pub rain: Vec<f32>,
    /// Liquid moved out of each ground cell, mol/s, a moving average over about a year (`flow::liquid`); it pushes parts
    /// (`bond::load`)
    pub liquid_flux: Vec<f32>,
    pub diag: Diag,
    pub ledger: Ledger,
    /// Bodies, and the parts they made: the sparse graph
    pub life: crate::life::Life,
    pub parts: bond::Parts,
    /// Light-capture area of the bodies of each cell per ground area (they intercept 1 - exp(-a))
    pub canopy: Vec<f32>,
    /// Light intercepted in each cell per m² of light-capture area since the start (the mean of
    /// its bodies: taller members catch more, `body::shade`), J/m²
    pub light: Vec<f64>,
    /// Ground cleared of bodies along each edge, m²: edge `4i + f` joins cell `i` to its
    /// neighbour `FORWARD[f]` (each edge once, used both ways). Movers pushing through bodies raise
    /// it (`choose::go`), cover growing back lowers it (`choose::regrow`). Empty until first used.
    pub pass: Vec<f32>,
    /// Raw physical facts since the shell last drained them (not part of the saved state)
    #[serde(skip)]
    pub events: Vec<Event>,
    /// Working memory of the laws, rebuilt when missing (saved, so a resumed run goes on the same)
    pub scratch: flow::Scratch,
    /// Light set aside in each cell for what its bodies' light reactions will store, J: put
    /// aside as it falls (`flow::air::radiation`), not as heat; a body takes its share at its
    /// update (`body::live`), what no body can claim goes to the ground's heat (`body::survey`).
    pub light_pool: Vec<f64>,
    /// Per body slot: the share of the light it caught that its light reactions stored at its
    /// last update (J per J). Its cell sets that share of the light it absorbs aside.
    pub light_keep: PerSlot,
    /// Liquid (with what is dissolved in it) in the store of the bed under each ground node, [k * n + i], mol, and its
    /// thermal energy with the fusion enthalpy of what is molten, J (`flow::drain`; empty until first used)
    pub deep: Vec<f64>,
    pub e_deep: Vec<f64>,
    /// Heat set aside in each cell for what its bodies draw from the ground between their updates, J (below 0: given to the
    /// ground ahead): each survey holds here what each body drew a second at its last update times the time since
    /// (`body::survey`), so the ground pays it as it goes and a body's update settles only the difference (`body::live`).
    pub heat_pool: Vec<f64>,
    /// Per body slot: the heat it drew from the ground at its last update, W.
    pub heat_keep: PerSlot,
}

impl World {
    pub fn days_per_year(&self) -> u64 {
        (self.cfg.days_per_month * self.cfg.months_per_year) as u64
    }
    pub fn year(&self) -> f64 {
        self.day as f64 / self.days_per_year() as f64
    }
    /// Fraction of the year, 0..1
    pub fn season(&self) -> f64 {
        (self.day % self.days_per_year()) as f64 / self.days_per_year() as f64
    }
    /// Loose amount of substance `k` in ground node `i`, mol.
    #[inline]
    pub fn g(&self, k: usize, i: usize) -> f64 {
        self.ground[k * self.grid.n() + i]
    }
    /// Bonded amount of substance `k` in ground node `i`, mol.
    #[inline]
    pub fn b(&self, k: usize, i: usize) -> f64 {
        self.bed[k * self.grid.n() + i]
    }
    /// Amount of substance `k` in air node `j`, mol.
    #[inline]
    pub fn a(&self, k: usize, j: usize) -> f64 {
        self.air[k * self.grid.na() + j]
    }
}
