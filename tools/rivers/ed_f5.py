def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

edit('tape/src/ground/channels.rs', [
 ("""//! from afar a shell draws a channel as a line wider than its bends, about that one). A channel a shell may draw as a line wider than
//! itself (its drained area at least `SHOW_M` × the texel's size over `TEXEL_PX_MOST` × `WET_TIMES`) reaches
//! `LINE_REACH` texels too. Each number is a function of the seed, the map and the place alone.""",
  """//! from afar a shell draws a channel as a line wider than its bends, about that one). A channel a shell may draw as
//! a line wider than itself (its drained area at least `SHOW_M` × the texel's size over `TEXEL_PX_MOST` ×
//! `WET_TIMES`) reaches as far as the widest line the shell draws it as on these texels too (`line_reach`). Each
//! number is a function of the seed, the map and the place alone."""),
 ("""/// The texels a line reaches from its centre: past the widest line a shell draws.
const LINE_REACH: f64 = 2.5;""",
  """/// The line a shell draws a channel as from afar: `LINE_PX` css pixels wide where its discharge is the one that shows
/// (`SHOW_M`), wider as its discharge over that to the power `LINE_GROW` (more steeply than the channel law widens a
/// channel), up to `LINE_MOST` times. A map's choice, set by eye: a stream is a hair and the main river many times
/// wider, a river widens down its way and below each tributary, and the line is as wide as a bed of a few metres
/// where a view with a scale bar of 50 m draws that bed itself.
pub const LINE_PX: f64 = 1.6;
pub const LINE_GROW: f64 = 0.55;
pub const LINE_MOST: f64 = 8.0;
/// A css pixel takes at most this many texels: a texel takes a screen's pixel or more (`TEXEL_PX_MOST`), and a
/// phone's screen has three pixels to a css pixel.
const CSS_TEXELS_MOST: f64 = 3.0;
/// A line reaches this many texels past its edge: a shell blends the distance between texels and spreads the edge
/// over a pixel.
const LINE_MARGIN: f64 = 1.5;"""),
 ("""        let reach_of = |a: f64| {
            let own = self.cap * a.powf(self.r.law.width_exp);
            let reach = if a >= line_from { own.max(LINE_REACH * texel) } else { own };
            (own, (reach / metres).min(most_reach))
        };""",
  """        // Half the widest line a shell draws a channel that drains `a` m² as on these texels, with its margin, m:
        // the line law where a css pixel takes the most texels (the line's width in metres grows with the pixel).
        let line_reach = |a: f64| {
            let css = CSS_TEXELS_MOST * texel;
            0.5 * LINE_PX * css * (WET_TIMES * a / (SHOW_M * css)).powf(LINE_GROW).min(LINE_MOST) + LINE_MARGIN * texel
        };
        let reach_of = |a: f64| {
            let own = self.cap * a.powf(self.r.law.width_exp);
            let reach = if a >= line_from { own.max(line_reach(a)) } else { own };
            (own, (reach / metres).min(most_reach))
        };"""),
])
edit('tape/src/ground/mod.rs', [
 ("""pub use channels::{Law, ALONG_WRAP_M, SHOW_M, STEP_MOST, TYPICAL_RUNOFF};""", """pub use channels::{Law, ALONG_WRAP_M, LINE_GROW, LINE_MOST, LINE_PX, SHOW_M, STEP_MOST, TYPICAL_RUNOFF};"""),
])
edit('tape/src/lib.rs', [
 ("""Law, ALONG_WRAP_M, FLOOD, GUTTER, PONDS_EXACT, SHOW_M,""", """Law, ALONG_WRAP_M, FLOOD, GUTTER, LINE_GROW, LINE_MOST, LINE_PX, PONDS_EXACT, SHOW_M,"""),
])
edit('wasm/src/lib.rs', [
 ("""/// pixel takes past which a channel shows as a line, m³/s per m (`tape::TYPICAL_RUNOFF` × `tape::SHOW_M`), and how many
/// times the mean flow the flood that shapes a channel's bed passes (`tape::FLOOD`).""",
  """/// pixel takes past which a channel shows as a line, m³/s per m (`tape::TYPICAL_RUNOFF` × `tape::SHOW_M`), that line's
/// law (`tape::LINE_PX`, `LINE_GROW`, `LINE_MOST`), and how many times the mean flow the flood that shapes a channel's
/// bed passes (`tape::FLOOD`)."""),
 (""""line": tape::TYPICAL_RUNOFF * tape::SHOW_M, "flood": tape::FLOOD},""", """"line": tape::TYPICAL_RUNOFF * tape::SHOW_M, "line_law": [tape::LINE_PX, tape::LINE_GROW, tape::LINE_MOST], "flood": tape::FLOOD},"""),
])
edit('web/src/land.ts', [
 ("""/** The numbers of the laws inside a cell as the core sends them (wasm/src/lib.rs `meta`, `Physics`), and the discharge
 * per metre a css pixel takes past which a channel shows as a line, m³/s per m. */
export interface Laws { soil_m: number; wet_depth: number; dormancy: number; channel_width: number; channel_width_exp: number; channel_depth: number; channel_depth_exp: number; line: number; flood: number }""",
  """/** The numbers of the laws inside a cell as the core sends them (wasm/src/lib.rs `meta`, `Physics`), the discharge
 * per metre a css pixel takes past which a channel shows as a line, m³/s per m, and that line's law (its css pixels,
 * the power it widens by, the most times: tape/src/ground/channels.rs `LINE_PX`). */
export interface Laws { soil_m: number; wet_depth: number; dormancy: number; channel_width: number; channel_width_exp: number; channel_depth: number; channel_depth_exp: number; line: number; line_law: [number, number, number]; flood: number }"""),
 ("""const LAND_VEC4S = 7;""", """const LAND_VEC4S = 8;"""),
 (""".vec4(tm.along_wrap, l.flood, this.slid(performance.now()));""", """.vec4(tm.along_wrap, l.flood, this.slid(performance.now())).vec4(...l.line_law);"""),
])
