// uses: draw.wgsl land.wgsl marks.wgsl
// The marks of the plant cover over the live map (lib/marks.wgsl), each masked at its own foot by the laws inside a cell
// there (lib/land.wgsl), so none stands where liquid stands deeper than its roots reach or under snow, and fewer stand
// where the ground is drier than the heaviest cover of its cell needs, against its cell as a whole.

// A land plant's roots need air in the soil: liquid standing over its foot as deep as this, m, drowns them (half of the
// roots of most kinds of cover lie in the top 0.3 m, Jackson and others 1996).
const ROOTS_M = 0.3;

// The share of the marks whose foot is at `at`, m from the map's north-west corner, that stand: as the liquid there
// leaves their roots air, free of snow, and as their cover thrives there.
fn foot(at: vec2f) -> f32 {
  let st = state_at(at, ground_at(at, false), corners(at / land.map.x));
  return (1.0 - ease(0.0, ROOTS_M, st.depth)) * (1.0 - st.snow) * st.thrive;
}
