def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

edit('tape/src/ground/channels.rs', [
 ("""    /// The field over the window `win`: each line near enough drawn into the texels within its reach, and the valleys
    /// about the lines on texels finer than the samples; then the lines without their bends, each into the texels
    /// that hold its own channel's line (the line from the same head, `Bends::head`) or none.
    pub(super) fn field(&self, win: Window) -> Field {
        let n = win.wide * win.high;
        let (near, reach_valley) = self.near(&win, win.k * self.r.metres);""",
  """    /// Give each texel of the field `f`, `wide` texels across, that holds a channel between two texels (left and
    /// right, or above and below) that both hold one other channel which drains more, to that channel: its distances
    /// the mean of the two's. A small channel that crosses the reach of a larger one lies nearer over its scale in a
    /// strip a texel wide, which would notch the line a shell draws the larger one as from afar.
    fn mend(&self, f: &mut Field, wide: usize) {
        let (s, sample, dist, calm) = (self.r.s, f.sample.clone(), f.dist.clone(), f.calm.clone());
        let head = |q: u32| self.bends.head(q as usize);
        let mean = |a: f64, b: f64| if a.abs().max(b.abs()) >= NO_CHANNEL_M { NO_CHANNEL_M } else { 0.5 * (a + b) };
        for i in wide..sample.len() - wide {
            if i % wide == 0 || i % wide == wide - 1 || sample[i] == NONE {
                continue;
            }
            for (a, b) in [(i - 1, i + 1), (i - wide, i + wide)] {
                let (qa, qb) = (sample[a], sample[b]);
                if qa == NONE || qb == NONE || head(qa) != head(qb) || head(qa) == head(sample[i]) || s.up[qa as usize].min(s.up[qb as usize]) <= s.up[sample[i] as usize] {
                    continue;
                }
                let nearer = if dist[a].abs() <= dist[b].abs() { a } else { b };
                (f.sample[i], f.along[i], f.dist[i], f.calm[i]) = (sample[nearer], f.along[nearer], mean(dist[a], dist[b]), mean(calm[a], calm[b]));
                break;
            }
        }
    }

    /// The field over the window `win` (`laid`), its strips mended (`mend`: over a window a texel larger all round,
    /// so each texel has its neighbours).
    pub(super) fn field(&self, win: Window) -> Field {
        let wide = win.wide + 2;
        let mut f = self.laid(Window { x0: win.x0 - 1, y0: win.y0 - 1, wide, high: win.high + 2, k: win.k });
        self.mend(&mut f, wide);
        let cut = |v: &[f64]| (0..win.high).flat_map(|y| v[(y + 1) * wide + 1..(y + 2) * wide - 1].iter().copied()).collect::<Vec<f64>>();
        let sample = (0..win.high).flat_map(|y| f.sample[(y + 1) * wide + 1..(y + 2) * wide - 1].iter().copied()).collect();
        Field { dist: cut(&f.dist), sample, along: cut(&f.along), calm: cut(&f.calm), valley: cut(&f.valley), weight: cut(&f.weight) }
    }

    /// The field over the window `win` as laid: each line near enough drawn into the texels within its reach, and
    /// the valleys about the lines on texels finer than the samples; then the lines without their bends, each into
    /// the texels that hold its own channel's line (the line from the same head, `Bends::head`) or none.
    fn laid(&self, win: Window) -> Field {
        let n = win.wide * win.high;
        let (near, reach_valley) = self.near(&win, win.k * self.r.metres);"""),
])

edit('tape/src/ground/belt.rs', [
 ("""/// An apex turns on a radius between these, in channel widths: the tighter ones the more often (the draw squared);
/// on a hard bed between `HARD_RADIUS`, and its heading turns by no more than `HARD_HEADING`, rad.
const RADIUS_WIDTHS: [f64; 2] = [1.4, 5.0];
const HARD_RADIUS: [f64; 2] = [0.3, 1.0];""",
  """/// An apex turns on a radius between these, in channel widths: the tighter ones the more often (the draw squared);
/// on a hard bed between `HARD_RADIUS` (most turns are gentle, one in ten a sharp one on a radius under a width and
/// a half: a hard bed's channel runs in nearly straight stretches from one sharp turn to the next), and its heading
/// turns by no more than `HARD_HEADING`, rad.
const RADIUS_WIDTHS: [f64; 2] = [1.4, 5.0];
const HARD_RADIUS: [f64; 2] = [0.4, 12.0];"""),
])
