// The ground without its photographs and things (detail.wgsl, things.wgsl, afloat.wgsl), for the programs that draw the world's
// ground but none of it up close (the tape's view, the map's classes): the colour the world gives it alone.
fn soil_detail(p: Spot) -> vec3f { return vec3f(1.0); }
fn rock_detail(e: f32) -> vec3f { return vec3f(1.0); }
fn rock_shows(e: f32) -> f32 { return 0.0; }
fn snow_texture(e: f32) -> vec3f { return vec3f(1.0); }
fn dry_filler(g: vec3f, p: Spot, bare: f32) -> vec3f { return g; }
fn objects(g: vec3f, p: Spot, bare: f32) -> vec3f { return g; }
fn afloat(water: vec3f, p: Spot) -> vec3f { return water; }
// No stone is drawn as a thing: the stones are the tone they give the ground (stone.wgsl `stones`).
fn stone_sprites() -> bool { return false; }
fn stone_sprite(out: vec4f, p: Placed, s: Stony, e: f32, shows: f32, tone: vec3f) -> vec4f { return out; }
