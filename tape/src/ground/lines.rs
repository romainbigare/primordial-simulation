//! The pieces of the channels' lines as a tile draws them (`channels`): each sample's own piece, its curve moved by
//! its wander and swung in its bends as far as the tile's texels show them, and the same piece without its bends; and
//! where a tributary joins, the piece on to the line it joins.

use super::belt::STEPS;
use super::bends::{ease, Mark, Start, FLOOD, STATION_EXP};
use super::channels::Channels;
use pcore::space::network::NONE;

/// A stretch of a line is drawn in straight pieces no shorter than this many texels (and a stretch without bends in
/// at most `CALM_STEPS`): a curve departs from shorter chords by far less than a texel.
const STEP_TEXELS: f64 = 0.5;
const CALM_STEPS: usize = 48;
/// A tile draws a channel's wander fully where a swing's length is at least `WANDER_TEXELS` texels, and not at all under
/// `WANDER_FROM`: a shell draws a channel from afar as a line several texels wide, and a sway whose turns are tighter
/// than that line is wide reads as the teeth of a saw (a bend turns on about a fifth of its length). Between, it grows,
/// and its turns are wider as it is drawn less.
const WANDER_FROM: f64 = 24.0;
const WANDER_TEXELS: f64 = 72.0;
/// A tile holds a channel's bends whole where their wavelength is at least `BEND_HELD[1]` texels, and none under
/// `BEND_HELD[0]` (its texels draw no tighter turn), between the two the same at both ends of a joint; a shell draws
/// them as far as its pixels show them (web/shaders/lib/land.wgsl `BEND_PX`, from more texels than these), so the
/// bends drawn are the same from every depth of the tiles and a zoom that changes the depth moves none.
const BEND_HELD: [f64; 2] = [12.0, 24.0];
/// The share of its bed's width a channel's water takes at its mean flow: `bends::FLOOD` to the power minus
/// `bends::STATION_EXP` (web/src/land.ts sends the shell the same).
fn water_share() -> f64 {
    FLOOD.powf(-STATION_EXP)
}

/// A point of a channel's line as drawn: its place, samples; the share of its piece's curve it lies along (0 to 1);
/// and how far along the channel's line from its head, m.
#[derive(Clone, Copy)]
pub(super) struct Vertex {
    pub p: [f64; 2],
    pub t: f64,
    pub s: f64,
}

/// A piece of a channel's line as a tile draws it: its points, and the points of the same piece without its bends.
pub(super) struct Drawn {
    pub line: Vec<Vertex>,
    pub calm: Vec<[f64; 2]>,
}

/// Where a tributary's line joins the next sample's (the middle of that one's line, samples), the direction of that
/// line there, half the width of that channel's bed and of its water at its mean flow (`bends::Reaches::bed_of`,
/// `water_share`), samples, and the length of the tributary's last piece, which runs into it, samples. Nearer the join
/// than that last piece is long, the tributary takes no texel within that water of the line it joins (taken straight
/// through the join), and in the rest of that bed only those of its own bed: so in the water of the channel it joins
/// every texel is that channel's, the tributary's own water runs across the other's bars into it, and beside it the
/// other's bars are the other's.
#[derive(Clone, Copy)]
pub(super) struct Join {
    at: [f64; 2],
    along: [f64; 2],
    bank: f64,
    water: f64,
    last: f64,
}

impl Join {
    /// Does the point `p` (samples), `d` samples from the tributary's line whose own bed is `bed` samples half wide,
    /// lie where the channel joined keeps the texels?
    pub(super) fn holds(&self, p: [f64; 2], d: f64, bed: f64) -> bool {
        let (dx, dy) = (p[0] - self.at[0], p[1] - self.at[1]);
        let across = (dx * self.along[1] - dy * self.along[0]).abs();
        dx.hypot(dy) < self.last && (across < self.water || (across < self.bank && d > bed))
    }
}

/// The steps of a half bend (`belt::STEPS`) a tile keeps one of, where each is `stride` m long on texels of `texel` m:
/// the fewest that make the pieces drawn at least `STEP_TEXELS` texels long, and a count that divides the half bend.
fn kept(stride: f64, texel: f64) -> u32 {
    (1..=STEPS).find(|n| STEPS.is_multiple_of(*n) && stride * *n as f64 >= STEP_TEXELS * texel).unwrap_or(STEPS)
}

impl Channels<'_> {
    /// Sample `k`'s own piece as drawn on texels of `texel` m, from its start to its end (none for a sink none runs
    /// into): its curve moved by its wander (`bends::Bends::leans`), and that with its walked bends
    /// (`bends::Reaches::walk`), each as far across as the tile holds it (`BEND_HELD`, `WANDER_FROM`: at each end as the
    /// joint there is drawn, so two pieces meet; no bend where the line ends, at a join or a sink).
    pub(super) fn piece(&self, k: usize, texel: f64) -> Option<Drawn> {
        let (r, s) = (&self.r, self.r.s);
        let frame = r.frame(k)?;
        let (len, start) = (frame.len(), self.bends.start(k));
        // How far a sway of length `own` (its neighbour `j`'s by `of`) is drawn where the two pieces meet.
        let joint = |own: f64, j: u32, of: &dyn Fn(usize) -> f64| ease(BEND_HELD[0], BEND_HELD[1], if j == NONE { own } else { own.min(of(j as usize)) } / texel);
        let runs_on = s.to[k] != NONE && s.main[s.to[k] as usize] == k as u32;
        let bends = [joint(r.wavelength(k), s.main[k], &|j| r.wavelength(j)), if runs_on { joint(r.wavelength(k), s.to[k], &|j| r.wavelength(j)) } else { 0.0 }];
        let wanders = ease(WANDER_FROM, WANDER_TEXELS, r.wander_length() / texel);
        let between = |ends: [f64; 2], u: f64| ends[0] + (ends[1] - ends[0]) * (u / len).clamp(0.0, 1.0);
        let lean = |u: f64| wanders * self.bends.leans(r, k, &frame, u.clamp(0.0, len));
        let n = ((len / (STEP_TEXELS * texel)).ceil() as usize).clamp(1, CALM_STEPS);
        let calm: Vec<[f64; 2]> = (0..=n).map(|i| i as f64 * len / n as f64).map(|u| frame.at(u, lean(u))).collect();
        if bends == [0.0; 2] || (matches!(start, Start::Straight(_)) && r.room(k, &frame).belt == 0.0) {
            // No bends to draw: its wander alone, the line's own length spread evenly along it (the bends' too, where
            // they are too small to draw).
            let (from, to) = (start.along(), r.walk(k, &frame, start, &mut |_| {}).along());
            let line = calm.iter().enumerate().map(|(i, &p)| Vertex { p, t: frame.share(i as f64 * len / n as f64), s: from + (to - from) * i as f64 / n as f64 }).collect();
            return Some(Drawn { line, calm });
        }
        let mut marks = Vec::new();
        r.walk(k, &frame, start, &mut |m: Mark| {
            if m.step.is_multiple_of(kept(m.stride, texel)) {
                marks.push(m);
            }
        });
        // A bend drawn less than fully is drawn that much from the wander: across it, and along it from where the
        // line's own length puts the point (a loop's limb that runs back up the valley would else fold the line over
        // itself).
        let (from, run) = (marks[0].s, (marks[marks.len() - 1].s - marks[0].s).max(1e-9));
        let point = |m: &Mark| {
            let even = len * (m.s - from) / run;
            let drawn = between(bends, even);
            let u = even + (m.u - even) * drawn;
            Vertex { p: frame.at(u, lean(u) + m.v * drawn), t: frame.share(u), s: m.s }
        };
        Some(Drawn { line: marks.iter().map(point).collect(), calm })
    }

    /// The line of sample `k` on texels of `texel` m: its own piece, and on to the middle of the next sample's line
    /// where a tributary joins (with its bends and without); and there, that point and the direction of the line it
    /// joins (`Join`).
    pub(super) fn line(&self, k: usize, texel: f64) -> Option<(Drawn, Option<Join>)> {
        let (r, s) = (&self.r, self.r.s);
        let mut own = self.piece(k, texel)?;
        let j = s.to[k];
        if j == NONE || s.main[j as usize] == k as u32 {
            return Some((own, None));
        }
        let join = self.piece(j as usize, texel).map(|next| {
            let i = (0..next.line.len()).min_by(|&a, &b| (next.line[a].t - 0.5).abs().total_cmp(&(next.line[b].t - 0.5).abs())).expect("a piece has points");
            let (a, c, at, from) = (next.line[i.saturating_sub(1)].p, next.line[(i + 1).min(next.line.len() - 1)].p, next.line[i].p, own.line[own.line.len() - 1]);
            let len = (c[0] - a[0]).hypot(c[1] - a[1]).max(f64::MIN_POSITIVE);
            let last = (at[0] - from.p[0]).hypot(at[1] - from.p[1]);
            // The way on to the join sways by the tributary's wander (`bends::Bends::joins`), as far as the tile draws
            // a wander: no channel runs straight into the one it joins.
            let (calm_from, calm_at, run) = (own.calm[own.calm.len() - 1], next.calm[next.calm.len() / 2], last * r.metres);
            let wanders = ease(WANDER_FROM, WANDER_TEXELS, r.wander_length() / texel);
            let across = [-(at[1] - from.p[1]) / last.max(f64::MIN_POSITIVE), (at[0] - from.p[0]) / last.max(f64::MIN_POSITIVE)];
            let n = ((run / (STEP_TEXELS * texel)).ceil() as usize).clamp(1, CALM_STEPS);
            let curve = r.frame(k).map_or(0.0, |f| f.len());
            for x in (1..=n).map(|i| i as f64 / n as f64) {
                let off = wanders * self.bends.joins(r, k, curve, x * run, run) / r.metres;
                let on = |from: [f64; 2], to: [f64; 2]| [from[0] + (to[0] - from[0]) * x + across[0] * off, from[1] + (to[1] - from[1]) * x + across[1] * off];
                own.line.push(Vertex { p: on(from.p, at), t: 1.0, s: from.s + x * run });
                own.calm.push(on(calm_from, calm_at));
            }
            Join { at, along: [(c[0] - a[0]) / len, (c[1] - a[1]) / len], bank: r.bed_of(j as usize).0 / (2.0 * r.metres), water: water_share() * r.bed_of(j as usize).0 / (2.0 * r.metres), last }
        });
        Some((own, join))
    }
}
