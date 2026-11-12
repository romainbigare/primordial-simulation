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
/// A valley follows its line in straight pieces no shorter than this share of `VALLEY_M`: within that of the line the
/// ground is drawn at the floor's height all the same.
const CHORD_SHARE: f64 = 1.0 / 32.0;

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

/// The valleys' work over a window's texels: of each the square of its distance to the line in hand and that line's
/// floor at its nearest point, and the texels the line reached.
pub(super) struct Carve {
    nearest: Vec<(f64, f64)>,
    touched: Vec<usize>,
}

impl Carve {
    pub(super) fn new(n: usize) -> Carve {
        Carve { nearest: vec![(f64::INFINITY, 0.0); n], touched: Vec::new() }
    }

    /// Add to `f` over `win` the valley of the line `pts` of sample `q` of `s`, which reaches `reach` samples: the
    /// weight of its floor at each texel within reach, and its floor's height so weighted.
    pub(super) fn line(&mut self, f: &mut Field, win: &Window, s: &Samples, q: usize, pts: &[Vertex], reach: f64) {
        let mut a = pts[0];
        for (n, &b) in pts.iter().enumerate().skip(1) {
            // Short pieces are passed over, up to the line's last point.
            if n + 1 < pts.len() && (b.p[0] - a.p[0]).hypot(b.p[1] - a.p[1]) < CHORD_SHARE * reach {
                continue;
            }
            for y in win.texels(a.p[1].min(b.p[1]) - reach, a.p[1].max(b.p[1]) + reach, 1) {
                for x in win.texels(a.p[0].min(b.p[0]) - reach, a.p[0].max(b.p[0]) + reach, 0) {
                    let (at, i) = win.texel(x, y);
                    let (_, d2, t) = to_segment(at, a.p, b.p);
                    if d2 <= reach * reach && d2 < self.nearest[i].0 {
                        if self.nearest[i].0 == f64::INFINITY {
                            self.touched.push(i);
                        }
                        self.nearest[i] = (d2, floor(s, q, a.t + (b.t - a.t) * t));
                    }
                }
            }
            a = b;
        }
        for i in self.touched.drain(..) {
            let (bed, w) = (self.nearest[i].1, weight(self.nearest[i].0.sqrt() / reach));
            (f.valley[i], f.weight[i], self.nearest[i]) = (f.valley[i] + w * bed, f.weight[i] + w, (f64::INFINITY, 0.0));
        }
    }
}
