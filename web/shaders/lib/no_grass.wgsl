// The sward without its images (detail.wgsl), for the plants' passes, which draw no sward (cover.wgsl, marks.wgsl).
fn grass_detail(e: f32, straw: f32, reeds: f32) -> vec3f { return vec3f(1.0); }
fn flower_heads(e: f32) -> vec2f { return vec2f(0.0, 1.0); }
fn sward_tufts(c: vec3f, e: f32, s: f32) -> vec4f { return vec4f(c, s); }
