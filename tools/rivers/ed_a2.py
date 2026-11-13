def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a, t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)
edit('tape/src/ground/bends.rs', [("""/// How a piece starts: on the valley's line""", """impl Entry {
    /// How far along the channel's line the piece starts, m.
    pub(super) fn along(&self) -> f64 {
        self.edge[1]
    }
}

/// How a piece starts: on the valley's line""")])
edit('tape/src/ground/channels.rs', [("""        let from = match start {
            Start::Straight(s) if r.room(k, &frame).belt == 0.0 => Some(s),
            Start::Bent(e) if ends == [0.0; 2] => Some(e.along()),
            Start::Straight(s) if ends == [0.0; 2] => Some(s),
            _ => None,
        };
        if let Some(from) = from {
            let n""", """        let (from, bends) = match start {
            Start::Straight(s) => (s, r.room(k, &frame).belt > 0.0),
            Start::Bent(e) => (e.along(), true),
        };
        if !bends || ends == [0.0; 2] {
            let n""")])
edit('tape/src/lib.rs', [("pub use ground::{depths, half, unhalf, Ground, Law, GUTTER, PONDS_EXACT, SHOW_M, STEP_MOST, TILE, TYPICAL_RUNOFF, VALLEY_FROM, VALLEY_M};",
                          "pub use ground::{depths, half, unhalf, Ground, Law, ALONG_WRAP_M, FLOOD, GUTTER, PONDS_EXACT, SHOW_M, STEP_MOST, TILE, TYPICAL_RUNOFF, VALLEY_FROM, VALLEY_M};")])
