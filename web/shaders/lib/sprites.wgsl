// The rendered models' table, written by web/textures/bake/sprites.cjs from sprites.json (web/textures/SOURCES.md):
// the trees (trees.png) one to a layer, then the rest (models.png) four to a layer.
const TREES = 33; const MODEL_LAYERS = 10;
// Each set of sprites: its first and how many.
const ROUND_FROM = 0; const ROUND_N = 6;
const JACARANDA_FROM = 6; const JACARANDA_N = 3;
const FIR_FROM = 9; const FIR_N = 9;
const PINE_FROM = 18; const PINE_N = 3;
const PINE_YOUNG_FROM = 21; const PINE_YOUNG_N = 2;
const PINE_SMALL_FROM = 23; const PINE_SMALL_N = 3;
const SPRUCE_YOUNG_FROM = 26; const SPRUCE_YOUNG_N = 3;
const QUIVER_FROM = 29; const QUIVER_N = 4;
const SHRUB_FROM = 33; const SHRUB_N = 4;
const ROCK_FROM = 37; const ROCK_N = 9;
const WOOD_FROM = 46; const WOOD_N = 3;
const DEAD_BUSH_FROM = 49; const DEAD_BUSH_N = 4;
const TUFT_FROM = 53; const TUFT_N = 8;
const SEARSIA_FROM = 61; const SEARSIA_N = 5;
const OTHONNA_FROM = 66; const OTHONNA_N = 6;
// Each sprite's content, as a share of its layer (a tree) or its quarter: its width and its height.
fn model_w(s: i32) -> f32 {
  var v = array<f32, 72>(0.938, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.413, 0.451, 0.463, 0.486, 0.538, 0.533, 0.527, 0.502, 0.504, 0.433, 0.578, 0.503, 0.525, 0.634, 0.580, 0.747, 0.653, 0.610, 0.658, 0.765, 0.481, 0.522, 0.621, 0.627, 0.840, 0.855, 0.984, 0.982, 0.984, 0.984, 0.984, 0.984, 0.984, 0.897, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.753, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.980, 0.870, 0.984, 0.868, 0.984, 0.945, 0.636, 0.672, 0.606, 0.700, 0.680, 0.953);
  return v[s];
}

fn model_h(s: i32) -> f32 {
  var v = array<f32, 72>(0.984, 0.852, 0.901, 0.795, 0.896, 0.773, 0.696, 0.846, 0.787, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.613, 0.984, 0.971, 0.852, 0.692, 0.449, 0.425, 0.984, 0.927, 0.465, 0.490, 0.093, 0.217, 0.269, 0.860, 0.953, 0.832, 0.984, 0.913, 0.623, 0.735, 0.749, 0.846, 0.674, 0.826, 0.984, 0.984, 0.929, 0.984, 0.866, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984);
  return v[s];
}

// Each sprite's mean cover over its content's box.
fn model_fill(s: i32) -> f32 {
  var v = array<f32, 72>(0.348, 0.357, 0.354, 0.353, 0.301, 0.313, 0.521, 0.447, 0.376, 0.363, 0.345, 0.345, 0.333, 0.296, 0.312, 0.138, 0.142, 0.151, 0.308, 0.303, 0.282, 0.273, 0.282, 0.268, 0.253, 0.293, 0.271, 0.252, 0.284, 0.194, 0.206, 0.375, 0.378, 0.124, 0.127, 0.124, 0.103, 0.647, 0.587, 0.599, 0.709, 0.715, 0.697, 0.720, 0.693, 0.711, 0.632, 0.428, 0.323, 0.464, 0.414, 0.432, 0.413, 0.109, 0.261, 0.217, 0.136, 0.151, 0.290, 0.068, 0.092, 0.379, 0.372, 0.377, 0.326, 0.252, 0.522, 0.467, 0.446, 0.329, 0.405, 0.480);
  return v[s];
}
