//! The map's stretch (laws.md 1.6): the air over a small map behaves as the air over the band of latitudes its rows
//! stand for, while the ground keeps its true metres. Only the air's sideways laws read it: the wind's pushes on each
//! link (`wind::pushes`) and the side of the blocks the eddies even within a day (`daily::blocks`), and with the blocks
//! the mixing of a body's liquid under each (`basin::mix`). The exchanges up and down (ground and air, sunlight, heat
//! radiation, rain out) are per m² and do not read it, nor does the rain's drift (`Config::rain_drift_m`) or anything
//! on the ground.

use crate::Config;

/// A column of the air stands for one `Config::stretch` times longer and wider: its horizontal lengths count that many
/// times, so a face between two columns counts that many times and a column's area, and with it all it holds, the
/// square of it. A rate between two columns as the stretched air has it, mol/s, moves the column of the map the
/// share of it its own column holds (`per_column`), so its warmth and shares change as those of the column it stands
/// for.
#[derive(Clone, Copy)]
pub(super) struct Stretch(f64);

impl Stretch {
    pub(super) fn of(cfg: &Config) -> Stretch {
        Stretch(cfg.stretch)
    }
    /// A horizontal length on the map, m, as the air's sideways laws take it, m: a link's length, a face's width, a
    /// spacing of the nodes.
    #[inline]
    pub(super) fn length(self, m: f64) -> f64 {
        m * self.0
    }
    /// A rate between two stretched columns, mol/s, as a rate between the columns of the map, mol/s: over the stretch
    /// squared, the share of the stretched column's air the map's holds.
    #[inline]
    pub(super) fn per_column(self, rate: f64) -> f64 {
        rate / (self.0 * self.0)
    }
}
