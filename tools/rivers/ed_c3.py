def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

edit('tape/src/ground/bends.rs', [
 ("""        // The reach: up the main way and down the channel's own, out of the ponds.
        let (mut top, mut bottom, mut run, mut across, mut n) = (k, k, 0.0, self.rise_across(k, to as usize), 1.0);
        for _ in 0..REACH_SAMPLES {
            if s.main[top] != NONE {
                (run, top) = (run + self.apart(top, s.main[top] as usize), s.main[top] as usize);
                (across, n) = (across + self.rise_across(top, s.to[top] as usize), n + 1.0);
            }
            let j = s.to[bottom];
            if j != NONE && s.pond[j as usize] == NONE {
                (run, bottom) = (run + self.apart(bottom, j as usize), j as usize);
                if s.to[bottom] != NONE {
                    (across, n) = (across + self.rise_across(bottom, s.to[bottom] as usize), n + 1.0);
                }
            }
        }
        let slope = if run > 0.0 { (s.h[top] - s.h[bottom]).max(0.0) / run } else { f64::INFINITY };
        let floor = if across > 0.0 { WALL_DEPTHS * depth * n / across } else { f64::INFINITY };
        let free = BELT_WIDTHS * width * (1.0 - ease(BENDS_BELOW, STRAIGHT_ABOVE, slope));
        Room { width, belt: free.min(floor).min(TURN_SHARE * frame.radius()).min(BEND_MOST * self.metres) }
    }
""", """        let [slope, across] = self.fall(k);
        let floor = if across > 0.0 { WALL_DEPTHS * depth / across } else { f64::INFINITY };
        let free = BELT_WIDTHS * width * (1.0 - ease(BENDS_BELOW, STRAIGHT_ABOVE, slope));
        Room { width, belt: free.min(floor).min(TURN_SHARE * frame.radius()).min(BEND_MOST * self.metres) }
    }

    /// The fall of the reach about sample `k`, which runs on (`REACH_SAMPLES` up its main way and down its own, out
    /// of the ponds): its slope along the channel, and how fast the ground rises across the valley on its mean, each
    /// rise over run.
    pub(super) fn fall(&self, k: usize) -> [f64; 2] {
        let s = self.s;
        let (mut top, mut bottom, mut run, mut across, mut n) = (k, k, 0.0, self.rise_across(k, s.to[k] as usize), 1.0);
        for _ in 0..REACH_SAMPLES {
            if s.main[top] != NONE {
                (run, top) = (run + self.apart(top, s.main[top] as usize), s.main[top] as usize);
                (across, n) = (across + self.rise_across(top, s.to[top] as usize), n + 1.0);
            }
            let j = s.to[bottom];
            if j != NONE && s.pond[j as usize] == NONE {
                (run, bottom) = (run + self.apart(bottom, j as usize), j as usize);
                if s.to[bottom] != NONE {
                    (across, n) = (across + self.rise_across(bottom, s.to[bottom] as usize), n + 1.0);
                }
            }
        }
        [if run > 0.0 { (s.h[top] - s.h[bottom]).max(0.0) / run } else { f64::INFINITY }, across / n]
    }
"""),
])
edit('tape/src/ground/rock.rs', [
 ("""//! The rock of the ground a shell draws, per terrain sample: how deep its loose ground lies (`Samples::loose`; none where
//! the bed is bare) and the stones its channel moves. The largest stone a stream moves grows with the power it spends on
//! its bed, which goes as its discharge times its slope (Bagnold 1966); the wear's stream power law takes the discharge
//! as the square root of the area drained (`space::wear`, K √A S), so a stone's size is here `STONE_M` per unit of √A S.""",
  """//! The rock of the ground a shell draws, per terrain sample: how deep its loose ground lies (`Samples::loose`; none where
//! the bed is bare) and the stones its channel moves. The largest stone a stream moves grows with the power it spends on
//! its bed, which goes as its discharge times its slope (Bagnold 1966); the wear's stream power law takes the discharge
//! as the square root of the area drained (`space::wear`, K √A S), so a stone's size is here `STONE_M` per unit of √A S,
//! S the slope of the reach about the sample (`bends::Reaches::fall`: from one sample to the next the ground steps)."""),
 ("""use super::half;
use pcore::space::network::NONE;
use pcore::space::samples::Samples;
""", """use super::bends::Reaches;
use super::half;
use pcore::space::network::NONE;
use pcore::space::samples::Samples;
"""),
 ("""/// Of each sample of `s`, `apart` m between neighbours' centres, row by row, four 16-bit floats: the depth of its loose""",
  """/// Of each sample of the reaches `r`, row by row, four 16-bit floats: the depth of its loose"""),
 ("""pub fn rock(s: &Samples, apart: f64, colours: &[[f64; 3]]) -> Vec<u16> {
    let mut out = Vec::with_capacity(8 * s.h.len());
    for k in 0..s.h.len() {
        let j = s.to[k];
        let stone = if j == NONE || s.pond[k] != NONE {
            0.0
        } else {
            let j = j as usize;
            let run = apart * if j % s.across != k % s.across && j / s.across != k / s.across { std::f64::consts::SQRT_2 } else { 1.0 };
            let slope = ((s.h[k] - s.h[j]) / run).max(0.0);
            (STONE_M * s.up[k].sqrt() * slope).min(STONE_MOST_M)
        };""", """pub(super) fn rock(r: &Reaches, colours: &[[f64; 3]]) -> Vec<u16> {
    let s = r.s;
    let mut out = Vec::with_capacity(8 * s.h.len());
    for k in 0..s.h.len() {
        let stone = if s.to[k] == NONE || s.pond[k] != NONE { 0.0 } else { (STONE_M * s.up[k].sqrt() * r.fall(k)[0]).min(STONE_MOST_M) };"""),
])
edit('tape/src/ground/mod.rs', [
 ("""        rock::rock(&self.samples, self.map.length(SAMPLE_DEPTH), colours)""", """        rock::rock(&self.reaches(), colours)"""),
 ("""        let reaches = bends::Reaches { seed: self.seed, s: &self.samples, at: &self.at, law: self.law, metres: self.map.length(SAMPLE_DEPTH) };
        let near = channels::Channels::new(reaches, &self.bends).field(""", """        let near = channels::Channels::new(self.reaches(), &self.bends).field("""),
 ("""    /// The centre of the texel at column `x` and row `y` of leaves of `depth`, in samples from the map's north-west corner.""",
  """    /// The reaches of the ground's channels (`bends::Reaches`).
    fn reaches(&self) -> bends::Reaches<'_> {
        bends::Reaches { seed: self.seed, s: &self.samples, at: &self.at, law: self.law, metres: self.map.length(SAMPLE_DEPTH) }
    }

    /// The centre of the texel at column `x` and row `y` of leaves of `depth`, in samples from the map's north-west corner."""),
])
