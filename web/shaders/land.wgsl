// uses: draw.wgsl ragged.wgsl water.wgsl style.wgsl crag.wgsl stone.wgsl ice.wgsl colour.wgsl land.wgsl rock.wgsl relief.wgsl river.wgsl flow.wgsl cover_field.wgsl lush.wgsl sprites.wgsl snag_sprites.wgsl heights.wgsl tex.wgsl detail.wgsl things.wgsl afloat.wgsl fire.wgsl burn.wgsl live_map.wgsl
// least metres a pixel: 0.5 1 3 30
// The live map in view (lib/live_map.wgsl), with the marks of fire on its ground (lib/burn.wgsl; none where the page
// knows no fire, web/src/fire.ts `groundBinds`). Made once more for draws whose pixels take at least 0.5 m, 1 m, 3 m
// and 30 m (web/shade, draw.wgsl `LEAST_E`): seen from afar, the detail of the ground and the plants that cannot show
// is compiled out (from 30 m, the whole island on a phone, the sea ice's floes and the pack's texture: ice.wgsl).

@fragment fn fs(v: Varying) -> @location(0) vec4f { return live_map(v); }
