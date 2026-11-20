// No marks of fire (burn.wgsl), for the maps where none is in view (land.wgsl) or none is known (a tape's map,
// earth.wgsl, and its classes, class.wgsl): their programs hold none of the marks' code.
fn burnt_spot(p: Spot) -> Spot { return p; }
fn burnt_floor(c: vec3f, p: Spot) -> vec4f { return vec4f(c, 0.0); }
fn burnt_truth(truth: vec3f, p: Spot) -> vec3f { return truth; }
