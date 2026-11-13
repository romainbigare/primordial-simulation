def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

edit('tape/src/ground/belt.rs', [
 ("""//! kink and no jump of its curvature. Each apex has its own place across the valley and its own radius, drawn from the
//! place's random numbers within what its reach of the channel gives (`Room`): the belt's middle wanders about the
//! valley's line, a bend reaches a share of the belt's half width from it, which itself swells and narrows along the
//! way, and turns on a radius of a few channel widths (2 to 3 in most grown bends: Hickin and Nanson 1975; Williams
//! 1986). The length of a half bend is what carries the line from its apex to the next one. So bends differ in size
//! and reach, some stretches run nearly straight, and where a wide reach meets a tight radius the line turns back on
//! itself in a loop, never across itself. Each number is a function of the seed, the channel and the place along it
//! alone.""", """//! kink and no jump of its curvature. Each apex has its own place across the valley and its own radius, drawn from the
//! place's random numbers within what its reach of the channel gives (`Room`): a bend reaches a share of the belt's
//! half width, which itself swells and narrows along the way, and turns on a radius of a few channel widths (2 to 3
//! in most grown bends: Hickin and Nanson 1975; Williams 1986). The length of a half bend is what carries the line
//! from its apex to the next one. So no two bends are alike in length or swing, some stretches run nearly straight,
//! and where a wide reach meets a tight radius the line turns back on itself in a loop, never across itself. On a
//! steep reach the bed is hard: the belt is narrow there (`bends`), and the line wanders in short turns on a radius
//! under its width (`HARD_RADIUS`), no farther from the valley's line than `HARD_HEADING` lets it. Each number is a
//! function of the seed, the channel and the place along it alone."""),
 ("""/// An apex turns on a radius between these, in channel widths: the tighter ones the more often (the draw squared).
const RADIUS_WIDTHS: [f64; 2] = [1.5, 3.5];
/// An apex reaches between these shares of the belt's half width from its middle where the belt is at its widest, and
/// lies at least `FAR_LEAST` of its reach across from the apex before it.
const REACH_SHARE: [f64; 2] = [0.5, 1.0];
const FAR_LEAST: f64 = 0.3;
/// The belt's width swells and narrows along the way over this many half bends, down to `SWELL_LEAST` of its widest
/// (the draw's complement squared: most reaches bend fully, some run nearly straight).
const SWELL_BENDS: u32 = 5;
const SWELL_LEAST: f64 = 0.1;
/// The belt's middle wanders from the valley's line by up to this share of the belt's half width, over this many half
/// bends.
const WANDER: f64 = 0.5;
const WANDER_BENDS: u32 = 7;
/// A half bend's length lies between these, in channel widths, and it carries the line at least `ADVANCE_WIDTHS` down
/// the valley; a straight stretch is walked in pieces of `STRAIGHT_WIDTHS`.
const ARC_WIDTHS: [f64; 2] = [3.0, 24.0];
const ADVANCE_WIDTHS: f64 = 2.5;
const STRAIGHT_WIDTHS: f64 = 6.0;
/// The halvings that find a half bend's length.
const ARC_HALVINGS: usize = 14;
/// The lattices of the walk's smooth noises along the way (`Walk::noise`): the belt's swell and its middle's wander.
const SWELL_LATTICE: u64 = 1;
const WANDER_LATTICE: u64 = 2;

/// What a reach of a channel gives its bends: the width of its bed, m, and the half width of its belt, m (none where
/// it runs straight).
#[derive(Clone, Copy)]
pub(super) struct Room {
    pub width: f64,
    pub belt: f64,
}""", """/// An apex turns on a radius between these, in channel widths: the tighter ones the more often (the draw squared);
/// on a hard bed between `HARD_RADIUS`, and its heading turns by no more than `HARD_HEADING`, rad.
const RADIUS_WIDTHS: [f64; 2] = [1.4, 5.0];
const HARD_RADIUS: [f64; 2] = [0.3, 1.0];
const HARD_HEADING: f64 = 0.8;
/// An apex reaches between these shares of the belt's half width where the belt is at its widest (the smaller ones
/// the more often: the draw to the power `REACH_SKEW`), and lies at least `FAR_LEAST` of its reach across from the
/// apex before it.
const REACH_SHARE: [f64; 2] = [0.2, 1.0];
const REACH_SKEW: f64 = 1.4;
const FAR_LEAST: f64 = 0.3;
/// The belt's width swells and narrows along the way over this many half bends, down to `SWELL_LEAST` of its widest
/// (the draw's complement squared: most reaches bend fully, some run nearly straight).
const SWELL_BENDS: u32 = 4;
const SWELL_LEAST: f64 = 0.08;
/// A half bend's length lies between these, in channel widths, and it carries the line at least `ADVANCE_WIDTHS` down
/// the valley (on a hard bed from `HARD_ARC` on, at least `HARD_ADVANCE` down it); a straight stretch is walked in
/// pieces of `STRAIGHT_WIDTHS`.
const ARC_WIDTHS: [f64; 2] = [3.0, 24.0];
const ADVANCE_WIDTHS: f64 = 2.5;
const HARD_ARC: f64 = 1.2;
const HARD_ADVANCE: f64 = 1.0;
const STRAIGHT_WIDTHS: f64 = 6.0;
/// The halvings that find a half bend's length.
const ARC_HALVINGS: usize = 14;
/// The lattice of the walk's smooth noise along the way (`Walk::noise`): the belt's swell.
const SWELL_LATTICE: u64 = 1;

/// What a reach of a channel gives its bends: the width of its bed, m, the half width of its belt, m (none where it
/// runs straight), and how hard its bed is, 0 on a gentle reach to 1 on a steep one.
#[derive(Clone, Copy)]
pub(super) struct Room {
    pub width: f64,
    pub belt: f64,
    pub hard: f64,
}"""),
 ("""    /// Begin the next half bend at an apex, on a reach that gives `room`: the next apex's place and radius from the
    /// place's draws, on the other side of the belt's middle; on a reach with no room, back to the valley's line and
    /// straight.
    pub(super) fn begin(&mut self, seed: u64, room: Room) {
        let mut r = Rng::new(seed, self.stem as u64, self.half as u64, Purpose::Terrain);
        let (reach_draw, radius_draw, side_draw) = (r.f64(), r.f64(), r.f64());
        let unswollen = 1.0 - self.noise(seed, SWELL_LATTICE, SWELL_BENDS);
        let swell = SWELL_LEAST + (1.0 - SWELL_LEAST) * (1.0 - unswollen * unswollen);
        let wander = WANDER * swell * (2.0 * self.noise(seed, WANDER_LATTICE, WANDER_BENDS) - 1.0);
        let reach = room.belt * swell * (REACH_SHARE[0] + (REACH_SHARE[1] - REACH_SHARE[0]) * reach_draw) * (1.0 - wander.abs());
        // The first bend goes to either side; after it they alternate.
        let side = if self.half > 0 { -self.swing } else if side_draw < 0.5 { -1.0 } else { 1.0 };
        let radius = room.width * (RADIUS_WIDTHS[0] + (RADIUS_WIDTHS[1] - RADIUS_WIDTHS[0]) * radius_draw * radius_draw);
        // A bend that reaches little turns gently: a reach that runs nearly straight has no tight wiggles.
        let next = if reach > 0.0 { swell.sqrt() / radius } else { 0.0 };
        self.curve = [self.curve[1], next];
        let across = (side * (room.belt * wander + side * reach - self.v)).max(FAR_LEAST * reach);""", """    /// Begin the next half bend at an apex, on a reach that gives `room`: the next apex's place and radius from the
    /// place's draws, on the other side of the valley's line; on a reach with no room, back to the line and straight.
    pub(super) fn begin(&mut self, seed: u64, room: Room) {
        let mut r = Rng::new(seed, self.stem as u64, self.half as u64, Purpose::Terrain);
        let (reach_draw, radius_draw, side_draw) = (r.f64(), r.f64(), r.f64());
        let unswollen = 1.0 - self.noise(seed, SWELL_LATTICE, SWELL_BENDS);
        let swell = SWELL_LEAST + (1.0 - SWELL_LEAST) * (1.0 - unswollen * unswollen);
        let reach = room.belt * swell * (REACH_SHARE[0] + (REACH_SHARE[1] - REACH_SHARE[0]) * reach_draw.powf(REACH_SKEW));
        // The first bend goes to either side; after it they alternate.
        let side = if self.half > 0 { -self.swing } else if side_draw < 0.5 { -1.0 } else { 1.0 };
        let between = |a: [f64; 2], x: f64| a[0] + (a[1] - a[0]) * x;
        let radius = room.width * (between(RADIUS_WIDTHS, radius_draw * radius_draw) * (1.0 - room.hard) + between(HARD_RADIUS, radius_draw) * room.hard);
        // A bend that reaches little turns gently: a reach that runs nearly straight has no tight wiggles.
        let next = if reach > 0.0 { swell.sqrt() / radius } else { 0.0 };
        self.curve = [self.curve[1], next];
        let across = (side * (side * reach - self.v)).max(FAR_LEAST * reach);"""),
 ("""            // The length that carries the line to the apex: the farther the longer, up to where its heading would pass
            // `HEADING_MOST` (the heading's largest is about the length times the mean curvature over 4) or it would
            // carry the line less than `ADVANCE_WIDTHS` down the valley.
            let (mut lo, mut hi) = (ARC_WIDTHS[0] * room.width, (8.0 * HEADING_MOST / sum).min(ARC_WIDTHS[1] * room.width));
            lo = lo.min(hi);
            for _ in 0..ARC_HALVINGS {
                let mid = 0.5 * (lo + hi);
                let [over, down] = carried(mid, self.curve);
                if over < far.abs() && down > ADVANCE_WIDTHS * room.width { lo = mid } else { hi = mid }
            }""", """            // The length that carries the line to the apex: the farther the longer, up to where its heading would pass
            // the most it may (the heading's largest is about the length times the mean curvature over 4) or it would
            // carry the line less than `ADVANCE_WIDTHS` down the valley.
            let most = HEADING_MOST + (HARD_HEADING - HEADING_MOST) * room.hard;
            let least = (ARC_WIDTHS[0] + (HARD_ARC - ARC_WIDTHS[0]) * room.hard) * room.width;
            let advance = (ADVANCE_WIDTHS + (HARD_ADVANCE - ADVANCE_WIDTHS) * room.hard) * room.width;
            let (mut lo, mut hi) = (least, (8.0 * most / sum).min(ARC_WIDTHS[1] * room.width));
            lo = lo.min(hi);
            for _ in 0..ARC_HALVINGS {
                let mid = 0.5 * (lo + hi);
                let [over, down] = carried(mid, self.curve);
                if over < far.abs() && down > advance { lo = mid } else { hi = mid }
            }"""),
 ("""
    /// The line's curvature where the walk stands, 1/m: positive where it turns toward the side `v` grows to.
    pub(super) fn curvature(&self) -> f64 {
        let x = self.step as f64 / STEPS as f64;
        self.swing * (self.curve[0] * (1.0 - x) * (1.0 - 3.0 * x) + self.curve[1] * x * (2.0 - 3.0 * x))
    }
""", ""),
])
