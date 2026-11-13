def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a, t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)
edit('tape/src/ground/bends.rs', [
 ("""impl Entry {
    /// How far along the channel's line the piece starts, m.
    pub(super) fn along(&self) -> f64 {
        self.edge[1]
    }
}

""", ""),
 ("""/// The channels of the samples `s` whose lines run through""", """impl Start {
    /// How far along the channel's line the piece starts, m.
    pub(super) fn along(&self) -> f64 {
        match self {
            Start::Straight(s) => *s,
            Start::Bent(e) => e.edge[1],
        }
    }
}

/// The channels of the samples `s` whose lines run through"""),
 ("""/// A bed narrower than this, m, is drawn without bends: its bends are smaller than the finest texels show.
const BENDS_FROM_M: f64 = 0.6;""", """/// A bed narrower than this, m, is drawn without bends: its bends are smaller than the finest texels show.
const BENDS_FROM_M: f64 = 1.0;"""),
])
edit('tape/src/ground/channels.rs', [
 ("""        let (from, bends) = match start {
            Start::Straight(s) => (s, r.room(k, &frame).belt > 0.0),
            Start::Bent(e) => (e.along(), true),
        };
        if !bends || ends == [0.0; 2] {
            let n = ((len / (STEP_TEXELS * texel)).ceil() as usize).clamp(1, CURVE_STEPS);
            return Some((0..=n).map(|i| i as f64 * len / n as f64).map(|u| Vertex { p: frame.at(u, 0.0), t: frame.share(u), s: from + u, curve: 0.0 }).collect());
        }""", """        if ends == [0.0; 2] || (matches!(start, Start::Straight(_)) && r.room(k, &frame).belt == 0.0) {
            // Its curve, the line's own length spread evenly along it (the bends' too, where they are too small to draw).
            let (from, to) = (start.along(), r.walk(k, &frame, start, &mut |_| {}).along());
            let n = ((len / (STEP_TEXELS * texel)).ceil() as usize).clamp(1, CURVE_STEPS);
            return Some((0..=n).map(|i| i as f64 / n as f64).map(|x| Vertex { p: frame.at(x * len, 0.0), t: frame.share(x * len), s: from + (to - from) * x, curve: 0.0 }).collect());
        }"""),
])
edit('tape/src/ground/tests.rs', [
 ("""        assert!((a.s - b.s).abs() < 1e-2, "sample {k}""", """        assert!((a.s - b.s).abs() < 1e-3, "sample {k}"""),
])
