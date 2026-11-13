def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

edit('tape/src/ground/mod.rs', [
 ("""pub use shore::SHORE_M;
""", ""),
 ("""/// The depths of the tiles of `map`, coarsest first:""", """/// Whether the shore a shell draws leaves each of the samples `s` out of its pond's liquid (`shore::Shore::dry`).
pub fn shore_dry(s: &Samples) -> Vec<bool> {
    shore::Shore::new(s).dry()
}

/// The depths of the tiles of `map`, coarsest first:"""),
])
edit('tape/src/lib.rs', [("PONDS_EXACT, SHORE_M, SHOW_M, STATION_EXP,", "PONDS_EXACT, SHOW_M, STATION_EXP,")])
edit('tape/src/table.rs', [
 ("""use crate::ground::{sample_index, SHORE_M};""", """use crate::ground::{sample_index, shore_dry};"""),
 ("""/// (`ground::sample_index`); and the cell of each of the terrain's samples.
pub struct Table {
    square: Vec<usize>,
    z: Vec<f64>,
    inside: Vec<(Vec<f64>, Vec<f64>)>,
    cell: Vec<u32>,
}""", """/// (`ground::sample_index`); the cell of each of the terrain's samples; and whether the shore a shell draws leaves
/// each sample out of its pond's liquid (`ground::shore_dry`).
pub struct Table {
    square: Vec<usize>,
    z: Vec<f64>,
    inside: Vec<(Vec<f64>, Vec<f64>)>,
    cell: Vec<u32>,
    dry: Vec<bool>,
}"""),
 ("""        Table { square, z, inside, cell: of.iter().map(|&i| i as u32).collect() }""", """        Table { square, z, inside, cell: of.iter().map(|&i| i as u32).collect(), dry: shore_dry(&s) }"""),
 ("""    /// that holds no liquid, or whose ground lies within `SHORE_M` under its pond's level, carries no less than its
    /// main way upstream brings: a river runs on along a dry body's floor, and to the shore a shell draws, which
    /// lies up to that far from the samples' (`ground::shore`). Deeper under a body's liquid no channel runs.""",
  """    /// that holds no liquid, or that the shore a shell draws leaves out of the liquid (`dry`), carries no less than
    /// its main way upstream brings: a river runs on along a dry body's floor, and to the shore drawn, which lies up
    /// to a few samples from the samples' own. Under a body's liquid no channel runs."""),
 ("""        let shore = |k: usize| { let l = levels[s.pond[k] as usize]; l.is_nan() || s.h[k] >= l - SHORE_M };""", """        let shore = |k: usize| self.dry[k] || levels[s.pond[k] as usize].is_nan();"""),
])
edit('docs/tape.md', [
 ("""sample of a pond that holds no liquid, or whose ground lies within 4 m (`SHORE_M`) under its pond's level, carries no
less than its main way upstream brings, so a river's line runs on along a dry body's floor and to the shore a shell
draws; deeper under a body's liquid no channel runs).""", """sample of a pond that holds no liquid, or that the shore drawn leaves out of the liquid (`ground::shore_dry`), carries
no less than its main way upstream brings, so a river's line runs on along a dry body's floor and to the shore a shell
draws; under a body's liquid no channel runs)."""),
])
t = open('docs/numbers.md').read()
a = "`MOUTH_STEP` (0.1 samples), `SHORE_M` (4 m), `tape/src/ground/shore.rs`"
assert t.count(a) == 1
t = t.replace(a, "`MOUTH_STEP` (0.1 samples), `tape/src/ground/shore.rs`")
a = "a pond's sample within `SHORE_M` under its level (the band the drawn shore lies in) or of a dry pond carries its river's flow;"
assert t.count(a) == 1
t = t.replace(a, "a pond's sample that the drawn shore leaves out of the liquid, or of a dry pond, carries its river's flow;")
open('docs/numbers.md', 'w').write(t)

edit('web/shaders/lib/land.wgsl', [
 ("""  let own = a4[corner].z;
  var line = vec2f(0.0);
  for (var n = 0; n < 4; n++) {
    let step = a4[n].z - own - land.reach.x * round((a4[n].z - own) / land.reach.x);
    if (abs(step) <= SAME_STRETCH_TEXELS * texel) { line += bilinear[n] * vec2f(step, 1.0); }
  }
  let along = own + line.x / line.y;""", """  // (In a program made for draws from afar, where no bed's shape shows, the nearest texel's as it is, and no swing.)
  let own = a4[corner].z;
  var along = own;
  let shaped = !unseen(BED_VARY * BED_MOST_M);
  if (shaped) {
    var line = vec2f(0.0);
    for (var n = 0; n < 4; n++) {
      let step = a4[n].z - own - land.reach.x * round((a4[n].z - own) / land.reach.x);
      if (abs(step) <= SAME_STRETCH_TEXELS * texel) { line += bilinear[n] * vec2f(step, 1.0); }
    }
    along = own + line.x / line.y;
  }"""),
 ("""  let lean = select(0.0, calm - signed, c_hi < NO_LINE_M && hi < NO_LINE_M);""", """  let lean = select(0.0, calm - signed, shaped && c_hi < NO_LINE_M && hi < NO_LINE_M);"""),
])
