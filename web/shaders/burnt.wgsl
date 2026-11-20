// uses: draw.wgsl ragged.wgsl water.wgsl style.wgsl crag.wgsl stone.wgsl ice.wgsl colour.wgsl land.wgsl rock.wgsl relief.wgsl river.wgsl flow.wgsl cover_field.wgsl sprites.wgsl snag_sprites.wgsl heights.wgsl tex.wgsl detail.wgsl things.wgsl afloat.wgsl fire.wgsl burn.wgsl live_map.wgsl
// least metres a pixel: 0.5 1 3 30
// The live map in view (lib/live_map.wgsl) with the burn scars on its ground (lib/burn.wgsl): drawn only over the
// squares that hold a scar (web/src/fire.ts), the rest of the map by land.wgsl. Made for the same sizes of a pixel.

@fragment fn fs(v: Varying) -> @location(0) vec4f { return live_map(v); }
