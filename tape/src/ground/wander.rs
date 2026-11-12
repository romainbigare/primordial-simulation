//! The wander of a channel's line about its valley's line: the terrain's samples place a valley's line no closer
//! than a share of a sample, smoothed over several of them (`stems`), and a channel does not keep to the middle of
//! its valley: it leans to one side for a stretch, then to the other, in long swings of no fixed length. The wander
//! is a smooth noise along the way down the valley, counted in samples (a corner of its lattice every `WANDER_EVERY`
//! of them, and a finer one `WANDER_FINE` times as often, `WANDER_FINE_SHARE` as strong), whose slope runs on across
//! the corners (gradient noise, Perlin 2002), so the line it moves has no kink. A reach's bends swing about the line
//! so moved (`belt`); from afar, where a shell draws a channel as a line wider than its bends, the wander is all of
//! the line's sway that shows. A function of the seed, the channel and the place along it alone.

use pcore::rng::{Purpose, Rng};

/// The samples down the valley between two corners of the wander's lattice: a swing to one side and back takes
/// about twice that.
pub(super) const WANDER_EVERY: f64 = 1.6;
/// The finer wander: its corners this many times as often, and it this share as strong.
const WANDER_FINE: f64 = 2.7;
const WANDER_FINE_SHARE: f64 = 0.4;
/// The lattice of the wander's draws among a channel's (`belt::Walk::noise` takes the others).
const WANDER_LATTICE: u64 = 3;

/// Gradient noise at `x` along the channel whose bends began at `head`, octave `octave`: about -1 to 1, 0 at each
/// corner of its lattice, smooth with its slope.
fn noise(seed: u64, head: u32, octave: u64, x: f64) -> f64 {
    let slope = |i: f64| 2.0 * Rng::new(seed, head as u64, ((WANDER_LATTICE + octave) << 40) | (i as i64 as u64 & 0xff_ffff_ffff), Purpose::Terrain).f64() - 1.0;
    let (i, f) = (x.floor(), x - x.floor());
    let fade = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    2.0 * (slope(i) * f * (1.0 - fade) + slope(i + 1.0) * (f - 1.0) * fade)
}

/// The wander `down` samples (over `WANDER_EVERY`) down the valley of the channel whose line began at `head`: the
/// share of its most it leans to one side (positive) or the other, about -1 to 1.
pub(super) fn wander(seed: u64, head: u32, down: f64) -> f64 {
    (noise(seed, head, 0, down) + WANDER_FINE_SHARE * noise(seed, head, 1, WANDER_FINE * down)) / (1.0 + WANDER_FINE_SHARE)
}
