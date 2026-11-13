def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a, t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)
edit('tape/src/ground/belt.rs', [
 ("""        self.swing = swing;
        let sum = self.curve[0] + self.curve[1];
        if sum <= 0.0 {""", """        self.swing = swing;
        if far != 0.0 && self.curve == [0.0; 2] {
            // Back to the line with no curvature at either end: the far end takes the gentlest.
            self.curve[1] = 1.0 / (RADIUS_WIDTHS[1] * room.width);
        }
        let sum = self.curve[0] + self.curve[1];
        if sum <= 0.0 {"""),
])
edit('tape/src/ground/bends.rs', [
 ("""/// The half width of the belt the bends swing in, in bed widths: a belt is 4 to 10 widths across (Williams 1986).
const BELT_WIDTHS: f64 = 4.0;""", """/// The half width of the belt the bends swing in, in bed widths, where nothing holds it: belts are 4 to 20 widths
/// across (Williams 1986).
const BELT_WIDTHS: f64 = 8.0;"""),
])
