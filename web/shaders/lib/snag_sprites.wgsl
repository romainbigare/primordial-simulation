// The snags' sprites (web/textures/snags.png), written by web/textures/bake/snags.py: their count and their layers,
// each one's width and height as shares of its quarter, and each kind's first sprite and count.
const SNAG_SPRITES = 22; const SNAG_LAYERS = 6;
var<private> SNAG_W: array<f32, 22> = array<f32, 22>(0.391, 0.418, 0.289, 0.461, 0.434, 0.246, 0.984, 0.988, 0.582, 0.508, 0.688, 0.586, 0.449, 0.551, 0.953, 0.973, 0.961, 0.699, 0.727, 0.785, 0.707, 0.781);
var<private> SNAG_H: array<f32, 22> = array<f32, 22>(0.988, 0.988, 0.988, 0.988, 0.988, 0.988, 0.539, 0.496, 0.941, 0.996, 0.996, 0.871, 0.996, 0.996, 0.379, 0.609, 0.492, 0.605, 0.535, 0.598, 0.680, 0.762);
const TRUNK_FROM = 0; const TRUNK_N = 6;
const STEMS_FROM = 6; const STEMS_N = 2;
const STUMP_FROM = 8; const STUMP_N = 14;
