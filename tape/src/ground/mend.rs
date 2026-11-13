//! The strips of a field of channels mended (`channels::Channels::field`): a small channel that crosses the reach of a
//! larger one lies nearer over its scale in a strip a texel or two wide, which would notch the line a shell draws the
//! larger one as from afar. A texel that holds a channel and lies within `STRIP` texels, to its left and to its right
//! or above and below, of texels that hold one other channel, which drains more (with none but its own channel's
//! between), is given to that channel.

use super::channels::{to_segment, Channels, Field, Near, Window, NO_CHANNEL_M};
use pcore::space::network::NONE;
use std::collections::HashMap;

/// The widest strip mended, texels.
pub(super) const STRIP: usize = 2;

/// The point of the line through `pts` nearest `at` (samples): the square of its distance, the side `at` lies on, and
/// the piece and the share of it the point lies along; none for a line of less than two points.
fn nearest(at: [f64; 2], pts: impl Iterator<Item = [f64; 2]>) -> Option<(f64, f64, usize, f64)> {
    let pts: Vec<[f64; 2]> = pts.collect();
    pts.windows(2).enumerate().map(|(n, w)| (to_segment(at, w[0], w[1]), n)).map(|((side, d2, t), n)| (d2, side, n, t)).min_by(|a, b| a.0.total_cmp(&b.0))
}

impl Channels<'_> {
    /// Mend the strips of the field `f` over the window `win`, whose lines are `near`: a texel given to the channel of
    /// its two neighbours holds the sample of the nearer of their two lines, its distance to that line and to the same
    /// sample's line without bends.
    pub(super) fn mend(&self, f: &mut Field, win: &Window, near: &[Near]) {
        let (s, wide, sample) = (self.r.s, win.wide, f.sample.clone());
        let lines: HashMap<u32, &Near> = near.iter().map(|l| (l.q as u32, l)).collect();
        let head = |q: u32| self.bends.head(q as usize);
        for i in (0..sample.len()).filter(|&i| sample[i] != NONE) {
            // The first texel of another channel within `STRIP` of this one along `step` (none past the window's row,
            // or past a texel that holds no channel).
            let beside = |step: isize| {
                (1..=STRIP as isize).map(|n| i as isize + n * step).take_while(|&j| j >= 0 && (j as usize) < sample.len() && (step.unsigned_abs() > 1 || j as usize / wide == i / wide) && sample[j as usize] != NONE).map(|j| j as usize).find(|&j| head(sample[j]) != head(sample[i]))
            };
            for step in [1, wide as isize] {
                let (Some(a), Some(b)) = (beside(-step), beside(step)) else { continue };
                let (qa, qb) = (sample[a], sample[b]);
                if head(qa) != head(qb) || s.up[qa as usize].min(s.up[qb as usize]) <= s.up[sample[i] as usize] {
                    continue;
                }
                let at = win.texel(win.x0 + (i % wide) as i64, win.y0 + (i / wide) as i64).0;
                let to = |q: u32| nearest(at, lines[&q].drawn.line.iter().map(|v| v.p)).map(|(d2, side, n, t)| (d2, side, n, t, q));
                let Some((d2, side, n, t, q)) = [to(qa), to(qb)].into_iter().flatten().min_by(|x, y| x.0.total_cmp(&y.0)) else { continue };
                let line = &lines[&q].drawn;
                let calm = nearest(at, line.calm.iter().copied()).map_or(NO_CHANNEL_M, |(d2, side, ..)| side * d2.sqrt() * self.r.metres);
                (f.sample[i], f.dist[i], f.along[i], f.calm[i]) = (q, side * d2.sqrt() * self.r.metres, line.line[n].s + (line.line[n + 1].s - line.line[n].s) * t, calm);
                break;
            }
        }
    }
}
