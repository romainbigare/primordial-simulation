//! Flames in the remains lying on the ground (laws.md 3c): once a month, after the cover's month, the remains of life
//! on a land cell carry a flame where they cover its ground without gaps and are dry enough that the heat a burning kg
//! sends ahead dries and lights the next kg (Rothermel 1972); a strike lights the joined remains of a region, and the
//! flame runs over them. What burns goes through the chemistry's own burning reactions; the speed comes from the heat
//! balance, not from the reactions' rates (theirs are those of rot). Three stages, each number with one writer: each
//! cell finds its own bed (`bed`, in parallel), the cells that carry a flame join into regions in the order of the cells
//! (`join`, serial), and each cell burns its own share (`burn`, one task per chunk of cells).

mod bed;
mod burn;
mod join;

pub use bed::{bed, Bed};
pub use burn::{burn, reach, Reach};
pub use join::shares;

use crate::cover::Parts;
use crate::World;

/// Remains that cover more than this share of a cell's ground join across it: the site-percolation threshold of the
/// square lattice, 0.592746 (Newman and Ziff 2000; Stauffer and Aharony 1994), a constant of mathematics.
pub const CARRY_SHARE: f64 = 0.592_746;
/// The temperature the sorption curve's numbers are given at (`Physics::sorb_scale`, `sorb_power`): that of the fit
/// they are taken from, 26.7 degrees over the key liquid's melting point (Nelson 2000).
pub const SORB_T: f64 = 299.85;
/// The relative vapour the sorption curve reads at most: at saturation its moisture has no bound (Nelson 2000 sets 0.99).
pub const SORB_HIGHEST: f64 = 0.99;
/// How long a fine particle of the remains flames, s per m of its width: 8 minutes an inch (Anderson 1969).
pub const RESIDENCE_PER_M: f64 = 8.0 * 60.0 / 0.0254;
/// Byram's (1959) flame: L = `FLAME_K` I^`FLAME_EXP` m, I the intensity in kW a m of front.
pub const FLAME_K: f64 = 0.0775;
pub const FLAME_EXP: f64 = 0.46;
/// Van Wagner's (1973) scorch height: `SCORCH_K` I^`SCORCH_EXP` m, I in kW a m (his 13 burns of 70 to 1,250 kW/m).
pub const SCORCH_K: f64 = 0.1483;
pub const SCORCH_EXP: f64 = 2.0 / 3.0;
/// W in a kW, for the fits in kW a m.
pub const W_PER_KW: f64 = 1e3;
/// Work a cell's bed takes to find, in instructions, for the pool (`par::map_n`).
const BED_WORK: usize = 2_000;

/// What flames burnt and killed in the cover (`Counters::flame`), kg (plants: a count): the remains on the ground; the
/// soft part of the cover that stood in the flame; the leaves under the scorch height that died; what else died of the
/// plants whose whole crown stood under it (their walls and fuel); of that their walls; and those plants.
pub const FLOWS: [&str; 6] = ["remains", "living", "scorched", "killed", "killed_walls", "plants"];
pub const REMAINS: usize = 0;
pub const LIVING: usize = 1;
pub const SCORCHED: usize = 2;
pub const KILLED: usize = 3;
pub const KILLED_WALLS: usize = 4;
pub const PLANTS: usize = 5;

/// The flames' month, after the cover's (`life::month`): each land cell's bed (`bed`), the share of each cell that burns
/// (`shares`) and the burning of it (`burn`), on the chunks of `life` only where some burns; the share each cell burnt
/// is added to its year (`Counters::burnt_year`, `burnt_last`).
pub fn month(w: &mut World) {
    let Some(p) = Parts::of(&w.chem) else { return };
    let n = w.mesh.cells().n();
    let beds = crate::par::map_n(n, n * BED_WORK, |i| if crate::contract::land(w, i) > 0.0 { bed(w, &p, i) } else { None });
    let share = shares(w, &beds, w.cfg.calendar().month as f64);
    let load = share.iter().filter(|&&f| f > 0.0).count();
    if load > 0 {
        crate::life::run(w, load, |w, d| {
            for i in d.cells() {
                if let Some(b) = beds[i].filter(|_| share[i] > 0.0) {
                    burn(w, d, &p, i, (share[i], &b));
                }
            }
        });
    }
    tally(w, n);
}

/// The share each cell burnt in the month (`Counters::burnt`) is added to its year (`Counters::burnt_year`, one a cell
/// of `n`); after the year's last month that year is the last whole one (`Counters::burnt_last`), for the tape.
fn tally(w: &mut World, n: usize) {
    let year = w.cfg.months_per_year as u64;
    let closes = w.month() % year == year - 1;
    let c = &mut w.counts;
    c.burnt_year.resize(n, 0.0);
    for (i, f) in std::mem::take(&mut c.burnt) {
        c.burnt_year[i as usize] += f;
    }
    if closes {
        c.burnt_last = std::mem::take(&mut c.burnt_year);
    }
}
