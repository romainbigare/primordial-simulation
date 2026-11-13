P = 'tape/src/ground/channels.rs'
t = open(P).read()
i = t.index("    /// Give each texel of the field `f`, `wide` texels across, that holds a channel between two texels")
j = t.index("    /// The field over the window `win` as laid:")
k = t.index("        let mut f = Field { dist: vec![NO_CHANNEL_M; n]", j)
new = '''    /// The field over the window `win`: the lines laid into its texels (`laid`) and its strips mended (`mend`), over
    /// a window a texel larger all round, so each texel has its neighbours.
    pub(super) fn field(&self, win: Window) -> Field {
        let wide = win.wide + 2;
        let all = Window { x0: win.x0 - 1, y0: win.y0 - 1, wide, high: win.high + 2, k: win.k };
        let (near, reach_valley) = self.near(&all, all.k * self.r.metres);
        let mut f = self.laid(&all, &near, reach_valley);
        self.mend(&mut f, &all, &near);
        let cut = |v: &[f64]| (0..win.high).flat_map(|y| v[(y + 1) * wide + 1..(y + 2) * wide - 1].iter().copied()).collect::<Vec<f64>>();
        let sample = (0..win.high).flat_map(|y| f.sample[(y + 1) * wide + 1..(y + 2) * wide - 1].iter().copied()).collect();
        Field { dist: cut(&f.dist), sample, along: cut(&f.along), calm: cut(&f.calm), valley: cut(&f.valley), weight: cut(&f.weight) }
    }

    /// The lines `near` laid into the texels of the window `win`: each line drawn into the texels within its reach,
    /// and the valleys about the lines (as far as `reach_valley` samples); then the lines without their bends, each
    /// into the texels that hold its own channel's line (the line from the same head, `Bends::head`) or none.
    fn laid(&self, win: &Window, near: &[Near], reach_valley: f64) -> Field {
        let n = win.wide * win.high;
'''
t = t[:i] + new + t[k:]
for a, b in [
    ("        for l in &near {\n            for w in l.drawn.line.windows(2) {\n                self.lay(&win, l,", "        for l in near {\n            for w in l.drawn.line.windows(2) {\n                self.lay(win, l,"),
    ("                carve.line(&mut f, &win, self.r.s,", "                carve.line(&mut f, win, self.r.s,"),
    ("        for l in &near {\n            for w in l.drawn.calm.windows(2) {\n                self.lay(&win, l,", "        for l in near {\n            for w in l.drawn.calm.windows(2) {\n                self.lay(win, l,"),
    ("struct Near {\n    q: usize,\n    drawn: Drawn,\n    join: Option<Join>,\n    scale: f64,\n    reach: f64,\n    carves: bool,\n}", "pub(super) struct Near {\n    pub q: usize,\n    pub drawn: Drawn,\n    join: Option<Join>,\n    scale: f64,\n    reach: f64,\n    carves: bool,\n}"),
    ("const NO_CHANNEL_M: f64 = 6.0e4;", "pub(super) const NO_CHANNEL_M: f64 = 6.0e4;"),
]:
    assert t.count(a) == 1, a[:60]
    t = t.replace(a, b)
open(P, 'w').write(t)

open('tape/src/ground/mend.rs', 'w').write('''//! The strips of a field of channels mended (`channels::Channels::field`): a small channel that crosses the reach of a
//! larger one lies nearer over its scale in a strip a texel wide, which would notch the line a shell draws the larger
//! one as from afar. A texel that holds a channel between two texels (left and right, or above and below) that both
//! hold one other channel, which drains more, is given to that channel.

use super::channels::{to_segment, Channels, Field, Near, Window, NO_CHANNEL_M};
use pcore::space::network::NONE;
use std::collections::HashMap;

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
        for i in wide..sample.len() - wide {
            if i % wide == 0 || i % wide == wide - 1 || sample[i] == NONE {
                continue;
            }
            for (a, b) in [(i - 1, i + 1), (i - wide, i + wide)] {
                let (qa, qb) = (sample[a], sample[b]);
                if qa == NONE || qb == NONE || head(qa) != head(qb) || head(qa) == head(sample[i]) || s.up[qa as usize].min(s.up[qb as usize]) <= s.up[sample[i] as usize] {
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
''')

def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)
edit('tape/src/ground/mod.rs', [("mod lines;", "mod lines;\nmod mend;")])
edit('tape/src/ground/tests.rs', [
 ("""/// Each texel a tile holds a channel in names the sample whose line it holds the distance to: the distance from the
/// texel's centre to that sample's own line is the texel's (so the discharge a shell reads there is the channel's own),
/// and the sample comes back from the texel's step as a shell finds it.""",
  """/// Each texel a tile holds a channel's line in names the sample whose line it holds the distance to: the distance
/// from the texel's centre to that sample's own line is the texel's (so the discharge a shell reads there is the
/// channel's own), and the sample comes back from the texel's step as a shell finds it."""),
 ("""    for (i, &q) in f.sample.iter().enumerate().filter(|p| *p.1 != NONE) {""", """    for (i, &q) in f.sample.iter().enumerate().filter(|p| *p.1 != NONE && f.dist[p.0].abs() < NO_CHANNEL_M) {"""),
 ("""use super::{code, to_segment, Channels, Law, Window, STEP_MOST};""", """use super::{code, to_segment, Channels, Law, Window, NO_CHANNEL_M, STEP_MOST};"""),
])
