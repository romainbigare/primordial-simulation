//! The wander of a channel's line about its valley's line: the terrain's samples place a valley's line no closer
//! than a share of a sample, smoothed over several of them (`stems`), and a channel does not keep to the middle of
//! its valley: it leans to one side for a stretch, then to the other, in long swings of no fixed length. The wander
//! is a smooth noise along the way down the valley, counted in samples (a corner of its lattice every `WANDER_EVERY`
//! of them, and `WANDER_OCTAVES` in all, each `WANDER_FINE` times as often as the last and `WANDER_FINE_SHARE` as
//! strong: less than its turns are closer, so each finer octave turns the line more gently than the last and the
//! line's turns are round, of uneven size, never a saw's teeth; the finest turns the line every few metres, so no
//! channel, however small, runs straight; a channel takes
//! the octaves whose corners lie `TURN_WIDTHS` of its bed's widths apart or more, since a wider bed's finer turns are
//! its bends'), whose slope
//! runs on across the corners (gradient noise, Perlin 2002), so the line it moves has no kink. A reach's bends swing about the line
//! so moved (`belt`); from afar, where a shell draws a channel as a line wider than its bends, the wander is all of
//! the line's sway that shows. A function of the seed, the channel and the place along it alone.

use pcore::rng::{Purpose, Rng};

/// The samples down the valley between two corners of the wander's lattice: a swing to one side and back takes
/// about twice that.
pub(super) const WANDER_EVERY: f64 = 1.6;
/// The wander's octaves: each one's corners `WANDER_FINE` times as often as the last's, and it `WANDER_FINE_SHARE`
/// as strong (its steepest lean across the way down is `WANDER_FINE` times `WANDER_FINE_SHARE` of the last's).
const WANDER_OCTAVES: u64 = 5;
const WANDER_FINE: f64 = 2.7;
const WANDER_FINE_SHARE: f64 = 0.25;
/// A channel wanders in the octaves whose corners lie at least this many of its bed's widths apart (fully from twice
/// that): a turn every few widths is a bend's, and a wide bed's bends are walked (`belt`).
const TURN_WIDTHS: f64 = 8.0;
/// The octaves' sum is taken this many times: gradient noise seldom passes a third of its range, and a wander that
/// used a third of its room would leave a line nearly straight; more would steepen its swings into straight limbs
/// between sharp turns.
const WANDER_GAIN: f64 = 1.5;
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

/// The wander `down` samples (over `WANDER_EVERY`) down the valley of the channel whose line began at `head`, its
/// bed `width` of that many samples wide there: the share of its most it leans to one side (positive) or the other,
/// about -1 to 1.
pub(super) fn wander(seed: u64, head: u32, down: f64, width: f64) -> f64 {
    let (mut sum, mut often, mut strong) = (0.0, 1.0, 1.0);
    for octave in 0..WANDER_OCTAVES {
        // (The first octave is every channel's; the corners of this one lie 1 / `often` apart.)
        let ease = super::bends::ease;
        let takes = if octave == 0 { 1.0 } else { ease(TURN_WIDTHS * width, 2.0 * TURN_WIDTHS * width, 1.0 / often) };
        if takes <= 0.0 {
            break;
        }
        sum += takes * strong * noise(seed, head, octave, often * down);
        (often, strong) = (often * WANDER_FINE, strong * WANDER_FINE_SHARE);
    }
    // Held within -1 to 1 smoothly.
    let x = WANDER_GAIN * sum;
    x / (1.0 + x * x).sqrt()
}
