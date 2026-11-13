def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a, t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)
edit('wasm/src/lib.rs', [
 ('"step_most": tape::STEP_MOST, "row"', '"step_most": tape::STEP_MOST, "along_wrap": tape::ALONG_WRAP_M, "row"'),
 ('"line": tape::TYPICAL_RUNOFF * tape::SHOW_M}', '"line": tape::TYPICAL_RUNOFF * tape::SHOW_M, "flood": tape::FLOOD}'),
 ("""/// by which a texel names its channel's sample; the numbers of a row of the table, `tape::Table`; the reflectance of the
/// walls the cover builds), the terrain's samples (across, down, their depth), the numbers of the laws the shell applies
/// inside a cell (`Physics`), and the discharge per metre a pixel takes past which a channel shows as a line, m³/s per
/// m (`tape::TYPICAL_RUNOFF` × `tape::SHOW_M`).""",
  """/// by which a texel names its channel's sample, and the length its place along its channel is kept modulo; the numbers
/// of a row of the table, `tape::Table`; the reflectance of the walls the cover builds), the terrain's samples (across,
/// down, their depth), the numbers of the laws the shell applies inside a cell (`Physics`), the discharge per metre a
/// pixel takes past which a channel shows as a line, m³/s per m (`tape::TYPICAL_RUNOFF` × `tape::SHOW_M`), and how many
/// times the mean flow the flood that shapes a channel's bed passes (`tape::FLOOD`)."""),
])
edit('web/src/ground.ts', [
 ("""/** The tiles of the ground as the core makes them (wasm/src/lib.rs `meta`): texels a side inside the gutter, texels of
 * the gutter along each edge, the depths from the coarsest to the finest, the most steps along each axis by which a
 * texel names its channel's sample, numbers of a row of the cells' table, and the reflectance of the walls the cover
 * builds. */
export interface TileMeta { size: number; gutter: number; depths: [number, number]; step_most: number; row: number;""",
  """/** The tiles of the ground as the core makes them (wasm/src/lib.rs `meta`): texels a side inside the gutter, texels of
 * the gutter along each edge, the depths from the coarsest to the finest, the most steps along each axis by which a
 * texel names its channel's sample, the length its place along its channel is kept modulo, m, numbers of a row of the
 * cells' table, and the reflectance of the walls the cover builds. */
export interface TileMeta { size: number; gutter: number; depths: [number, number]; step_most: number; along_wrap: number; row: number;"""),
 ("""above, and the coarsest; the rest is room for a pan. Each takes 676 kB on the card (10 bytes a texel), so 56 take
 * 38 MB. */""", """above, and the coarsest; the rest is room for a pan. Each takes 1.1 MB on the card (16 bytes a texel), so 56 take
 * 61 MB. */"""),
 ("""/** Of a tile, 16-bit floats a texel: the first plane four (the height as two, the wetness index, the signed distance to
 * its channel's line), the second two (that channel's sample, as its step from the texel's own, and one more than the
 * pond the texel lies in; 0: none). */
const PLANE_A = 4, PLANE_B = 2;""", """/** Of a tile, 16-bit floats a texel: the first plane four (the height as two, the wetness index, the signed distance to
 * its channel's line), the second four (that channel's sample, as its step from the texel's own, and one more than the
 * pond the texel lies in; 0: none; how far along its channel's line the texel lies, and the line's curvature there). */
const PLANE_A = 4, PLANE_B = 4;"""),
 ("this.areas ??= gpu.texture(side, side, 'rg16float', MOST_TILES);", "this.areas ??= gpu.texture(side, side, 'rgba16float', MOST_TILES);"),
])
edit('web/src/land.ts', [
 ("channel_depth: number; channel_depth_exp: number; line: number }", "channel_depth: number; channel_depth_exp: number; line: number; flood: number }"),
 ("const LAND_VEC4S = 6;", "const LAND_VEC4S = 7;"),
 (".vec4(s.across, s.down, s.depth, devicePixelRatio);", ".vec4(s.across, s.down, s.depth, devicePixelRatio).vec4(tm.along_wrap, l.flood);"),
])
