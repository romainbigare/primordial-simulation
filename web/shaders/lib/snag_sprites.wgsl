// The snags' sprites (web/textures/snags.png), written by web/textures/bake/snags.py: their count and their layers,
// each one's width, its height over its foot and its foot over its cell's bottom as shares of its cell, its layer
// and its quarter (-1: the whole layer), and each kind's first sprite and count.
const SNAG_SPRITES = 16; const SNAG_LAYERS = 9;
const SNAG_W: array<f32, 16> = array<f32, 16>(0.387, 0.445, 0.312, 0.461, 0.457, 0.266, 0.977, 0.695, 0.859, 0.984, 0.648, 0.688, 0.398, 0.414, 0.398, 0.414);
const SNAG_H: array<f32, 16> = array<f32, 16>(0.984, 0.982, 0.949, 0.982, 0.973, 0.984, 0.930, 0.973, 0.984, 0.848, 0.984, 0.910, 0.348, 0.371, 0.293, 0.406);
const SNAG_F: array<f32, 16> = array<f32, 16>(0.004, 0.006, 0.039, 0.006, 0.016, 0.004, 0.012, 0.020, 0.008, 0.023, 0.008, 0.082, 0.008, 0.180, 0.199, 0.195);
const SNAG_LAYER: array<i32, 16> = array<i32, 16>(0, 1, 2, 3, 4, 5, 6, 6, 6, 6, 7, 7, 7, 7, 8, 8);
const SNAG_QUARTER: array<i32, 16> = array<i32, 16>(-1, -1, -1, -1, -1, -1, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1);
const TRUNK_FROM = 0; const TRUNK_N = 6;
const STUMP_FROM = 6; const STUMP_N = 10;
