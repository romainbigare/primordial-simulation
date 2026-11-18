// A plant the marks draw (mark.wgsl decides it, models.wgsl draws it): its numbers, the forms of a tree's crown its
// kind's numbers choose (web/src/form.ts) and how it stands up on the map from its foot.

// A plant: its foot, m from the draw's corner; its crown's width and its height, m; how closed its crown is; its colour
// and its walls' share; its own random numbers; its design (style.ts `DESIGN`); its form (form.ts `Form`) and how
// surely; its lean (a share of its height, east and south); how far from its foot it may reach, m; how dry its kind's
// ground, 0..1 (form.ts `Form`); and the share of its leaves left this season.
struct Plant {
  at: vec2f, d: f32, h: f32, closed: f32, rgb: vec3f, wood: f32, r: vec3f, design: f32,
  form: f32, sure: f32, lean: vec2f, reach: f32, dry: f32, left: f32,
}

// The forms (form.ts `Form`).
const ROUND = 0; const CONE = 1; const COLUMN = 2; const UMBRELLA = 3; const TIERED = 4; const LOBED = 5;
// A plant stands up this share of its height on the map (textures.ts `STAND`); a bush is as deep as `BUSH_DEEP` of its
// width.
const STAND = 0.5; const BUSH_DEEP = 0.45;
