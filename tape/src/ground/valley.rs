//! The valleys a shell draws about the channels' lines: a channel's line runs smooth between the terrain's samples
//! (`stems`) and swings in its bends (`belt`), where the samples' ground steps from one to the next. So the ground
//! within `VALLEY_M` of the line of a channel that drains `VALLEY_FROM` or more is drawn toward the height of the
//! channel's floor there (`floor`): all of it on the line, less and less across to none at `VALLEY_M`. The lowest line
//! of the ground drawn is then the channel's line, and the samples' steps beside it fade out. A function of the
//! samples and the lines alone.

use super::bends::ease;
use super::channels::{to_segment, Field, Window};
use super::lines::Vertex;
use pcore::space::network::NONE;
use pcore::space::samples::Samples;

/// The channels that drain this much or more, m², out of the ponds, draw the ground within `VALLEY_M` of their line,
/// m, toward their floor, on texels finer than the samples and out of the ponds (`Ground::tile`).
pub const VALLEY_FROM: f64 = 1.0e6;
pub const VALLEY_M: f64 = 250.0;

/// Whether sample `k` of `s` draws a valley about its line (`VALLEY_FROM`, out of the ponds).
pub(super) fn carves(s: &Samples, k: usize) -> bool {
    s.up[k] >= VALLEY_FROM && s.pond[k] == NONE
}
/// A valley follows its line in straight pieces no shorter than this share of `VALLEY_M`: within that of the line the
/// ground is drawn at the floor's height all the same.
const CHORD_SHARE: f64 = 1.0 / 32.0;
/// A straight piece of a line this share of `VALLEY_M` long weighs as one whole line did at a point: the pieces of a
/// line about a point add up, so a valley is as wide as one line's weight makes it about `VALLEY_M` / 12 from the
/// line, a little narrower nearer and wider farther.
const PIECE_SHARE: f64 = 0.25;

/// The weight of a line's floor at the share `x` of `VALLEY_M` from it, against the ground's 1: a share f of the floor
/// where f eases from 1 on the line to 0 at `VALLEY_M`, so f / (1 - f), infinite on the line.
fn weight(x: f64) -> f64 {
    let f = 1.0 - ease(0.0, 1.0, x);
    f / (1.0 - f).max(1e-12)
}

/// The height of a channel's floor along the piece of sample `k` of `s` at the share `t` of its curve (0 to 1), m:
/// from the mean of its height and the one's upstream (its own at a head), through its own at the middle, to the mean
/// of its own and the one's it runs to, or that one's own where a tributary joins it there (a sample's height in a
/// closed pond its level, where that lies higher).
fn floor(s: &Samples, k: usize, t: f64) -> f64 {
    let own = s.h[k];
    let height = |j: usize| if s.pond[j] == NONE { s.h[j] } else { Some(s.ponds[s.pond[j] as usize].level).filter(|l| l.is_finite()).unwrap_or(s.h[j]).max(s.h[j]) };
    let start = if s.main[k] == NONE { own } else { (own + height(s.main[k] as usize)) / 2.0 };
    let end = match s.to[k] {
        NONE => own,
        j if s.main[j as usize] == k as u32 => (own + height(j as usize)) / 2.0,
        j => height(j as usize),
    };
    if t < 0.5 { start + (own - start) * 2.0 * t } else { own + (end - own) * (2.0 * t - 1.0) }
}

/// Add to `f` over `win` the valley of the line `pts` of sample `q` of `s`, which reaches `reach` samples: each of
/// its straight pieces gives each texel within reach the weight of its floor at the texel's nearest point of it, as
/// much as it is long (over `PIECE_SHARE` of the reach), and that floor's height so weighted. A texel's floor is so
/// the mean of the line's floors about it, the nearer the heavier: smooth across the ground, with no step where the
/// nearest point of a line that bends leaps from one limb to the other.
pub(super) fn carve(f: &mut Field, win: &Window, s: &Samples, q: usize, pts: &[Vertex], reach: f64) {
    let mut a = pts[0];
    for (n, &b) in pts.iter().enumerate().skip(1) {
        let long = (b.p[0] - a.p[0]).hypot(b.p[1] - a.p[1]);
        // Short pieces are passed over, up to the line's last point.
        if n + 1 < pts.len() && long < CHORD_SHARE * reach {
            continue;
        }
        for y in win.texels(a.p[1].min(b.p[1]) - reach, a.p[1].max(b.p[1]) + reach, 1) {
            for x in win.texels(a.p[0].min(b.p[0]) - reach, a.p[0].max(b.p[0]) + reach, 0) {
                let (at, i) = win.texel(x, y);
                let (_, d2, t) = to_segment(at, a.p, b.p);
                if d2 < reach * reach {
                    let w = weight(d2.sqrt() / reach) * long / (PIECE_SHARE * reach);
                    (f.valley[i], f.weight[i]) = (f.valley[i] + w * floor(s, q, a.t + (b.t - a.t) * t), f.weight[i] + w);
                }
            }
        }
        a = b;
    }
}
