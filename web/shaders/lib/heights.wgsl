// The heights of the sprites that stand in water, written by web/textures/bake/sprites.cjs from sprites.json
// (web/textures/SOURCES.md): heights.png, 4 to a layer, each texel's height over the ground as a share of its
// sprite's height, its grey.
const HEIGHT_LAYERS = 5;
// The runs of sprites with heights: the first sprite, how many, and the first of their heights.
const HEIGHT_RUNS_N = 2;
const HEIGHT_RUNS: array<vec3i, 2> = array<vec3i, 2>(vec3i(59, 4, 0), vec3i(124, 13, 4));
// The heights of sprite s (their number among the heights), -1 where it has none.
fn heights_of(s: i32) -> i32 {
  for (var r = 0; r < HEIGHT_RUNS_N; r++) { let run = HEIGHT_RUNS[r]; if (s >= run.x && s < run.x + run.y) { return run.z + s - run.x; } }
  return -1;
}
// Of each of the heights, its sprite's stem's foot (the middle of its model's box on the ground) over its lowest point,
// as a share of its height.
const FOOT_OF_TABLE: array<f32, 17> = array<f32, 17>(0.144, 0.147, 0.123, 0.149, 0.077, 0.061, 0.060, 0.074, 0.059, 0.154, 0.176, 0.175, 0.101, 0.079, 0.066, 0.071, 0.128);
fn foot_of(s: i32) -> f32 { return FOOT_OF_TABLE[s]; }
