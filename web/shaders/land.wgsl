// uses: draw.wgsl ragged.wgsl water.wgsl style.wgsl crag.wgsl stone.wgsl ice.wgsl colour.wgsl land.wgsl rock.wgsl relief.wgsl river.wgsl flow.wgsl cover_field.wgsl sprites.wgsl heights.wgsl tex.wgsl detail.wgsl things.wgsl afloat.wgsl no_burn.wgsl live_map.wgsl
// least metres a pixel: 0.5 1 3 30
// The live map in view (lib/live_map.wgsl) where no mark of fire is in view (else burnt.wgsl). Made once more for
// draws whose pixels take at least 0.5 m, 1 m, 3 m and 30 m (web/shade, draw.wgsl `LEAST_E`): seen from afar, the
// detail of the ground and the plants that cannot show is compiled out (from 30 m, the whole island on a phone, the sea
// ice's floes and the pack's texture: ice.wgsl).

@fragment fn fs(v: Varying) -> @location(0) vec4f { return live_map(v); }
