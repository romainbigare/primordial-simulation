import sys
p = 'tape/src/ground/mod.rs'
t = open(p).read()
def sub(a, b, n=1):
    global t
    assert t.count(a) == n, (a, t.count(a))
    t = t.replace(a, b)
sub("""//! the channel it is nearest by the channel law (`channels`); and apart, the pond it lies in (`Ground::pond_tile`).""",
    """//! the channel it is nearest by the channel law (`channels`), with how far along that channel's line it lies and how
//! the line curves there; and apart, the pond it lies in (`Ground::pond_tile`).""")
sub("""mod bends;
mod channels;
mod half;
mod rock;
mod shore;
mod stems;
mod wetness;

pub use channels::{Law, SHOW_M, STEP_MOST, TYPICAL_RUNOFF, VALLEY_FROM, VALLEY_M};
""", """mod belt;
mod bends;
mod channels;
mod half;
mod rock;
mod shore;
mod stems;
mod valley;
mod wetness;

pub use bends::FLOOD;
pub use channels::{Law, ALONG_WRAP_M, SHOW_M, STEP_MOST, TYPICAL_RUNOFF};
pub use valley::{VALLEY_FROM, VALLEY_M};
""")
sub("""/// the point each sample's line runs through (`stems::points`) and the phase of each sample's bends (`bends::phases`).""",
    """/// the point each sample's line runs through (`stems::points`) and how each sample's piece of its line starts
/// (`bends::Bends`).""")
sub("    phase: Vec<f64>,\n", "    bends: bends::Bends,\n")
sub("""        let phase = bends::phases(&law, &samples, &at, map.length(SAMPLE_DEPTH));
        let shore = shore::Shore::new(&samples);
        Ground { seed, map, terrain, lat, samples, twi, law, at, phase, shore }""",
    """        let bends = bends::Bends::new(&bends::Reaches { seed, s: &samples, at: &at, law, metres: map.length(SAMPLE_DEPTH) });
        let shore = shore::Shore::new(&samples);
        Ground { seed, map, terrain, lat, samples, twi, law, at, bends, shore }""")
sub("""    /// (`channels::Field`; 60,000 where none reaches it). The second, two a texel: that channel's sample, as its
    /// step from the sample that holds the texel's centre (`channels::code`; 0 for none), and one more than the pond the
    /// texel lies in (`pond_tile`; 0 for none), whole numbers a 16-bit float holds exactly up to `PONDS_EXACT`. Texels
    /// off the map are those of its edge.""",
    """    /// (`channels::Field`; 60,000 where none reaches it). The second, four a texel: that channel's sample, as its
    /// step from the sample that holds the texel's centre (`channels::code`; 0 for none), and one more than the pond the
    /// texel lies in (`pond_tile`; 0 for none), whole numbers a 16-bit float holds exactly up to `PONDS_EXACT`; and of
    /// the nearest point of that channel's line, how far along the channel it lies, m, modulo `ALONG_WRAP_M`, and the
    /// line's curvature there, 1/m, positive where it turns toward the side the distance is positive on (0 for none).
    /// Texels off the map are those of its edge.""")
sub("""        let near = channels::Channels::new(&self.map, &self.samples, self.law, (&self.at, &self.phase)).field([win[0], win[1]], [wide, high], k);""",
    """        let reaches = bends::Reaches { seed: self.seed, s: &self.samples, at: &self.at, law: self.law, metres: self.map.length(SAMPLE_DEPTH) };
        let near = channels::Channels::new(reaches, &self.bends).field(channels::Window { x0: win[0], y0: win[1], wide, high, k });""")
sub("        let mut out = Vec::with_capacity(6 * side * side);", "        let mut out = Vec::with_capacity(8 * side * side);")
sub("""            out.extend([half(channels::code(&self.samples, texel, k, near.sample[i]) as f32), half(pond)]);""",
    """            let along = near.along[i].rem_euclid(ALONG_WRAP_M);
            out.extend([half(channels::code(&self.samples, texel, k, near.sample[i]) as f32), half(pond), half(along as f32), half(near.curve[i] as f32)]);""")
sub("""    /// The rock of each of the terrain's samples, row by row in two planes of four 16-bit floats a sample: the depth of
    /// its loose ground, m, and the largest stone its channel moves, m; and the reflectance of its bed's make, of
    /// `colours` one a make (`rock::rock`).""",
    """    /// The rock of each of the terrain's samples, row by row in two planes of four 16-bit floats a sample: the depth of
    /// its loose ground, m, the largest stone its channel moves, m, and its channel's links; and the reflectance of
    /// its bed's make, of `colours` one a make (`rock::rock`).""")
open(p, 'w').write(t)
