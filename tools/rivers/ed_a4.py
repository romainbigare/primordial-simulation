def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a, t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)
edit('tape/tests/ground.rs', [
 ("""/// The numbers of a texel of a tile's first plane.
const TEXEL: usize = 4;
""", """/// The numbers of a texel of a tile's first plane, and of its second.
const TEXEL: usize = 4;
const SECOND: usize = 4;
"""),
 ("""/// The texels two tiles side by side both hold (each one's gutter and the other's edge) hold the same height, wetness
/// index and channel: no number depends on the tile it is made in.""", """/// The texels two tiles side by side both hold (each one's gutter and the other's edge) hold the same height, wetness
/// index and channel, with the place along it and its curvature: no number depends on the tile it is made in."""),
 ("""            // The second plane: the channel's sample and the pond.
            let area = SIDE * SIDE * TEXEL;
            for b in 0..2 {
                assert_eq!(west[area + 2 * (y * SIDE + TILE + i) + b], east[area + 2 * (y * SIDE + i) + b], "row {y}, column {i} past the edge, the channel's sample and the pond");
            }""", """            // The second plane: the channel's sample, the pond, the place along the channel and its curvature.
            let area = SIDE * SIDE * TEXEL;
            for b in 0..SECOND {
                assert_eq!(west[area + SECOND * (y * SIDE + TILE + i) + b], east[area + SECOND * (y * SIDE + i) + b], "row {y}, column {i} past the edge, number {b} of the second plane");
            }"""),
 ("tape::unhalf(tile[SIDE * SIDE * TEXEL + 2 * ((y + GUTTER) * SIDE + x + GUTTER) + 1]) as usize;", "tape::unhalf(tile[SIDE * SIDE * TEXEL + SECOND * ((y + GUTTER) * SIDE + x + GUTTER) + 1]) as usize;"),
])
