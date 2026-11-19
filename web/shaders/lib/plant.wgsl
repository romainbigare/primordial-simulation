// A plant the marks draw (mark.wgsl decides it, models.wgsl draws it): its numbers, the forms of a tree's crown its
// kind's numbers choose (web/src/form.ts) and how it stands up on the map from its foot.

// A plant: its foot, m from the draw's corner; its crown's width and its height, m; how closed its crown is in leaf; its colour
// and its walls' share; its own random numbers; its design (style.ts `DESIGN`); its form (form.ts `Form`) and how
// surely; its lean (a share of its height, east and south); how far from its foot it may reach, m; how dry its kind's
// ground and how tender to frost it is, 0..1 each (form.ts `Form`); the share of its leaves left this season; how
// wet its kind's ground, 0..1; and how deep liquid stands over its foot, m (0: on land; mark.wgsl `decide`).
struct Plant {
  at: vec2f, d: f32, h: f32, closed: f32, rgb: vec3f, wood: f32, r: vec3f, design: f32,
  form: f32, sure: f32, lean: vec2f, reach: f32, dry: f32, tender: f32, left: f32, wet: f32, wades: f32,
}

// A plant's own draw among the sets its kind's numbers choose between (models.wgsl `model_set`, mark.wgsl `plant_of`),
// from its random numbers: one draw for all of them, so a plant drawn soft is a tree from its first height.
fn set_draw(r: vec3f) -> f32 { return fract(r.x * 31.7); }
// A second draw of its own, apart from that one, for whether its kind's tenderness to frost lets it be drawn soft
// (models.wgsl `model_set`).
fn tender_draw(r: vec3f) -> f32 { return fract(r.z * 41.3); }
// Whether a kind's number `v` lies under `edge` for its plant of set draw `u`: the edge moved by its draw up to half of
// `spread` either way, so as `v` moves through that band the kind's plants cross it one by one, never all at once.
fn under_edge(v: f32, edge: f32, spread: f32, u: f32) -> bool { return v + spread * (u - 0.5) < edge; }

// A woody plant this tall or taller, m, is a tree, a lower one a bush (style.ts `TREE_M`).
const TREE_M = 3.0;

// The forms (form.ts `Form`).
const ROUND = 0; const CONE = 1; const COLUMN = 2; const UMBRELLA = 3; const TIERED = 4; const LOBED = 5;
// A plant stands up this share of its height on the map (textures.ts `STAND`); a bush is as deep as `BUSH_DEEP` of its
// width.
const STAND = 0.5; const BUSH_DEEP = 0.45;
