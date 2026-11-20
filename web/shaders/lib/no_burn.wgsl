// No marks of fire (burn.wgsl), for the live map where none is in view (land.wgsl): its program holds none of their code.
fn burnt_spot(p: Spot) -> Spot { return p; }
fn burnt_truth(truth: vec3f, p: Spot) -> vec3f { return truth; }
