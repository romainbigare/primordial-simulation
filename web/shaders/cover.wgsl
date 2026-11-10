// uses: draw.wgsl marks.wgsl
// The marks of the plant cover over a tape's map (lib/marks.wgsl): the map holds no ground finer than its squares, so
// every mark of the squares' field stands.

// The share of the marks whose foot is at `at` that stand: all.
fn foot(at: vec2f) -> f32 { return 1.0; }
