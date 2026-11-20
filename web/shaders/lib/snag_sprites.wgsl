// The snags' sprites (web/textures/snags.png), written by web/textures/bake/snags.py: their count and their layers,
// each one's width, its height over its foot and its foot over its cell's bottom as shares of its cell, its layer
// and its quarter (-1: the whole layer), and each kind's first sprite and count.
const SNAG_SPRITES = 20; const SNAG_LAYERS = 10;
var<private> SNAG_W: array<f32, 20> = array<f32, 20>(0.387, 0.445, 0.312, 0.461, 0.457, 0.266, 0.992, 0.812, 0.859, 0.984, 0.742, 0.688, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984, 0.984);
var<private> SNAG_H: array<f32, 20> = array<f32, 20>(0.984, 0.982, 0.949, 0.982, 0.973, 0.984, 0.930, 0.984, 0.984, 0.848, 0.984, 0.973, 0.371, 0.426, 0.480, 0.570, 0.523, 0.590, 0.555, 0.750);
var<private> SNAG_F: array<f32, 20> = array<f32, 20>(0.004, 0.006, 0.039, 0.006, 0.016, 0.004, 0.012, 0.008, 0.008, 0.023, 0.008, 0.020, 0.008, 0.180, 0.012, 0.031, 0.008, 0.008, 0.125, 0.008);
var<private> SNAG_LAYER: array<i32, 20> = array<i32, 20>(0, 1, 2, 3, 4, 5, 6, 6, 6, 6, 7, 7, 7, 7, 8, 8, 8, 8, 9, 9);
var<private> SNAG_QUARTER: array<i32, 20> = array<i32, 20>(-1, -1, -1, -1, -1, -1, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1);
const TRUNK_FROM = 0; const TRUNK_N = 6;
const STUMP_FROM = 6; const STUMP_N = 14;
