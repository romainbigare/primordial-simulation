// The Earth view's style: the step from the physical colours (reflectances in linear light, colour.wgsl) to a
// hand-painted map, for the Earth view only; the layer views keep the physical colours under their ramp. Each look
// here still means what the model holds: the water's colour is its depth, a beach lies where the ground is low and
// gentle by the liquid, a slope's light is its slope. The kinds' colours are styled on the page (web/src/style.ts).
// Nothing here reads the view: a place keeps its look at any pan or zoom.

// How far the step goes, 0 (the physical colours) .. 1.
const STYLE = 0.7;
// At full strength: the colours' departure from their grey this much stronger, their departure from the middle grey
// this much stronger, and the whole lifted towards white by this share.
const SATURATE = 0.35; const CONTRAST = 0.15; const LIFT = 0.06;

// The relief: the light's departure from flat ground's this many times stronger at full strength, held between the
// shade's soft ambient light and the brightest lit side; the shaded side cool, the lit side warm.
const RELIEF = 2.0; const AMBIENT = 0.30; const LIT_TOP = 1.7;
// Steep ground is darker whichever way it faces, by up to `STEEP_SHADE` from a slope of `STEEP_SLOPES.x` (rise over
// run) to `STEEP_SLOPES.y`: a ridge that runs along the light, whose two sides it lights alike, still shows its sides.
// Ground the light does not reach, in the shade of higher ground toward it, is darkened to `CAST_SHADE`: where the
// slope up to that ground (rock.wgsl `SoilAt.horizon`) passes the light's rise, eased from `CAST_EDGE.x` under it to
// `CAST_EDGE.y` over it (a soft edge: the sun is a disc and the air scatters its light). The relief finer than the
// samples (relief.wgsl) raises a knob toward the light and lowers a hollow: the slope up to the shading ground falls by
// its height over `CAST_REACH_M`, so a knob rises out of the shade before the ground about it and the shade's edge
// breaks into lit knobs and dark hollows. The edge widens by the knobs' change over `CAST_KNOB_PX` pixels: a knob of a
// few pixels would break it into a pixel's steps.
const STEEP_SHADE = 0.6; const STEEP_SLOPES = vec2f(0.03, 0.7);
const CAST_SHADE = 0.62; const CAST_EDGE = vec2f(-0.10, 0.06); const CAST_REACH_M = 60.0; const CAST_KNOB_PX = 3.0;
// Ground of one colour over kilometres is not of one tone: the Earth view lightens and darkens it by up to `TONE_SWAY`
// either way over the noise of the samples' places (rock.wgsl `sample_place`, from hundreds of metres to a kilometre),
// as a painted map's wash is uneven.
const TONE_SWAY = 0.04;
const SHADE_TINT = vec3f(0.80, 0.88, 1.08); const LIT_TINT = vec3f(1.06, 1.02, 0.92);
// Bare ground is drawn by its grain (a painted map's earth): its colour times a tint at full strength, the tints of its
// grains by their shares. Coarse grain (sand) pale and warm: quartz grains bare of coatings; middle grain (silt) warm;
// fine grain (clay) darker and redder, as the iron oxides that coat it stain it (Schwertmann 1993). Organic matter
// darkens it further towards black-brown, at full strength whatever the style: none of it up to `HUMUS_FROM` of its
// solid volume, all of it from `HUMUS_FULL` (about 1.5 % of its mass), evenly in the logarithm of the share between: a
// soil's lightness falls about with the logarithm of its organic carbon, and a few per cent make it dark (Schulze and
// others 1993; Konen and others 2003).
const SAND_TINT = vec3f(1.9, 1.62, 1.08);
const SILT_TINT = vec3f(1.25, 1.08, 0.82);
const CLAY_TINT = vec3f(1.0, 0.5, 0.36);
const HUMUS_TINT = vec3f(0.22, 0.16, 0.11); const HUMUS_FROM = 0.002; const HUMUS_FULL = 0.02;

// Water seen through: the attenuation of the clearest coastal water, its green and blue drawn stretched (Jerlov 1976,
// type 1 and clearer), so the bed shows to tens of metres, its red half as much as water itself takes it (Pope and
// Fry 1997), so clear shallows over sand turn a little turquoise; beyond the bed's last light the open water's colour, darkening from `OPEN_M` to
// `DEEP_M` deep (linear reflectances).
const CLEAR_ATTENUATION = vec3f(0.18, 0.04, 0.026);
const OPEN_WATER = vec3f(0.006, 0.07, 0.20); const DEEP_WATER_PAINT = vec3f(0.004, 0.022, 0.10);
const OPEN_M: f32 = 30.0; const DEEP_M: f32 = 600.0;
// Inland water holds the matter dissolved from the soil, which takes the blue first (Kirk 1994): running water is drawn
// through `INLAND_ATTENUATION` (stretched as the clear water's) over `INLAND_DEEP`, the olive green of a deep pool, so
// a stream shows its bed's own colour, greener and darker as it deepens; a lake's attenuation is `LAKE_CLEAR` of the
// way to the sea's clear water, over `LAKE_WATER`, the turquoise of a clear lake whose fine pale grains send the green
// back with the blue.
const INLAND_ATTENUATION = vec3f(0.22, 0.12, 0.2); const INLAND_DEEP = vec3f(0.012, 0.035, 0.035); const LAKE_CLEAR = 0.85;
const LAKE_WATER = vec3f(0.008, 0.17, 0.24);
// Foam at the sea's edge: white, at most this share, over the water shallower than `FOAM_M`. Close up it is the swash's
// lines, which follow the shore (lines of the bed's depth): one at the water's edge and `FOAM_LINES` more, each
// `FOAM_LINE_M` of depth farther out and `FOAM_LINE_FALL` as strong as the last, `FOAM_LINE_W` of that spacing wide to
// each side, soft, and broken along the shore over `FOAM_BREAK_M` (the first at the edge thinned to `FOAM_EDGE_KEPT`
// where it breaks, not gone). Out from the edge the broken waves' foam lies in ragged bands: each wanders across its
// line by `FOAM_WANDER` of the spacing over `FOAM_WANDER_M`, its width varies by half over that, and it breaks where two
// octaves of noise (`FOAM_BREAK_M` and a third of it) fall, into patches with no two alike. Where the lines lie under `FOAM_LINE_PX` pixels apart they are one rim, `FOAM_RIM` of
// the foam.
const FOAM = 0.55; const FOAM_M = 0.4; const FOAM_RGB = vec3f(0.85, 0.92, 0.95);
const FOAM_LINES = 2.0; const FOAM_LINE_M = 0.05; const FOAM_LINE_FALL = 0.55; const FOAM_LINE_W = 0.25;
const FOAM_BREAK_M = 6.0; const FOAM_LINE_PX = vec2f(1.5, 4.0); const FOAM_RIM = 0.6; const FOAM_EDGE_KEPT = 0.4;
const FOAM_WANDER = 0.3; const FOAM_WANDER_M = 4.0;
// The pixels across the foam's band over which it fades in.
const FOAM_PX = vec2f(0.5, 1.5);
// The bed under any water is drawn this far towards pale sand (`BEACH_SAND` times `BED_TONE`), so shallows read a
// little light over a dark soaked bed too, and the bed's own stones and sand still show.
const BED_LIGHTEN = 0.35; const BED_TONE = 0.85;
// Lakes and rivers are metres deep where the sea is tens: each is coloured as if `LAKE_SCALE` times as deep, and by
// one law (`river_seen`): lighter towards its shore, where it is less than a rim's depth deep (`LAKE_RIM_M`): a band
// as wide as the bed's slope makes it, wide where the bed is gentle and no narrower than `LAKE_RIM_PX` pixels where
// it is steep, so every shore has its light shallows. Under lake and river alike the bed shows `BED_SHOWN` of its
// light: it reads through the shallows, and the water's own colour stays.
const LAKE_SCALE = 1.5;
const LAKE_RIM_M = 0.3; const LAKE_RIM_PX = 2.5;
const BED_SHOWN = 0.8;

// Beach sand (colour.wgsl `beach`), bright and dry; a beach takes this share of its colour from its loose ground's
// own, lifted this many times (washed and dry, a ground is lighter).
const BEACH_SAND = vec3f(0.62, 0.53, 0.36);
const BEACH_OWN = 0.3; const BEACH_LIFT = 1.8;

// The weights of red, green and blue in a colour's lightness (Rec. 601).
const LUMA = vec3f(0.299, 0.587, 0.114);

// A display colour (the screen's share, 0..1) in the style: more saturated, more contrast, lifted.
fn painted(c: vec3f) -> vec3f {
  let grey = dot(c, LUMA);
  var out = vec3f(grey) + (c - vec3f(grey)) * (1.0 + SATURATE * STYLE);
  out = vec3f(0.5) + (out - vec3f(0.5)) * (1.0 + CONTRAST * STYLE);
  return mix(out, vec3f(1.0), LIFT * STYLE);
}

// The light on ground whose light is `x` times flat ground's: its departure made stronger, held softly between the
// ambient light and the lit top.
fn relief_light(x: f32) -> f32 {
  let k = 1.0 + (x - 1.0) * (1.0 + (RELIEF - 1.0) * STYLE);
  // Below 1 it falls towards the ambient light, above towards the top, each as steeply as k at 1 (no crease).
  if (k < 1.0) { return AMBIENT + (1.0 - AMBIENT) * exp((k - 1.0) / (1.0 - AMBIENT)); }
  return LIT_TOP - (LIT_TOP - 1.0) * exp(-(k - 1.0) / (LIT_TOP - 1.0));
}

// What the colour of dry ground of slope `slope` (rise over run) is multiplied by where the slope up to the highest
// ground toward the light is `horizon` and the fine relief raises the point `knob.x` m, that changing by `knob.y` m
// across a pixel: its steepness' shade and the shade cast on it (`STEEP_SHADE`, `CAST_SHADE`).
fn relief_shade(slope: f32, horizon: f32, knob: vec2f) -> f32 {
  let wide = CAST_KNOB_PX * knob.y / CAST_REACH_M;
  let unlit = smoothstep(CAST_EDGE.x - wide, CAST_EDGE.y + wide, horizon - knob.x / CAST_REACH_M - LIGHT.z / length(LIGHT.xy));
  return mix(1.0, STEEP_SHADE, smoothstep(STEEP_SLOPES.x, STEEP_SLOPES.y, slope)) * mix(1.0, CAST_SHADE, unlit);
}

// What the colour of the dry ground is multiplied by where the noise of the samples' places is `n` (`TONE_SWAY`).
fn tone_of(n: f32) -> f32 { return 1.0 + TONE_SWAY * clamp(n, -1.0, 1.0); }

// What the colour of ground whose share `bare` is bare, and whose share `seen` shows its soil from above
// (`Ground.seen`), is multiplied by: its bare part tinted by its grain `grain` (`Spot.grain`: the shares of coarse and
// fine grain and of organic matter) as `SAND_TINT` to `CLAY_TINT` say, and the soil seen darkened by its organic matter
// (`humus`). The grain's tint is even in its shares, so the mean of patches of grain is the tint of their mean grain;
// the organic share is the cells', with no patches.
fn bare_tint(bare: f32, seen: f32, grain: vec4f) -> vec3f {
  let mineral = grain.x * SAND_TINT + grain.y * CLAY_TINT + max(1.0 - grain.x - grain.y, 0.0) * SILT_TINT;
  return mix(vec3f(1.0), mineral, bare * STYLE) * mix(vec3f(1.0), HUMUS_TINT, seen * humus(grain.z));
}

// How far organic matter of the share `organic` of the solid volume darkens bare ground, 0..1 (`HUMUS_FROM`).
fn humus(organic: f32) -> f32 { return ease(log(HUMUS_FROM), log(HUMUS_FULL), log(max(organic, HUMUS_FROM))); }

// Linear colour `c` lit by `lit` (`relief_light`), tinted cool in the shade and warm in the light.
fn relief_tint(c: vec3f, lit: f32) -> vec3f {
  let tint = select(mix(vec3f(1.0), SHADE_TINT, ease(1.0, AMBIENT, lit)), mix(vec3f(1.0), LIT_TINT, ease(1.0, LIT_TOP, lit)), lit >= 1.0);
  return c * lit * mix(vec3f(1.0), tint, STYLE);
}

// How far from its shore standing liquid `depth` m deep lies, 0 at its edge to 1 where it is as deep as its rim: its
// bed rises at `slope` (rise over run) to a rim `rim_m` deep, seen at `e` m a pixel (`LAKE_RIM_PX`). The share of
// the deep water's colour it takes (`river_seen`'s `rel`: a lake's shallows are a river's).
fn shore_rim(depth: f32, slope: f32, e: f32, rim_m: f32) -> f32 {
  return ease(0.0, rim_depth(slope, e, rim_m), depth);
}
// The depth from which standing liquid is past its shore's band, m (`shore_rim`).
fn rim_depth(slope: f32, e: f32, rim_m: f32) -> f32 { return max(rim_m, slope * LAKE_RIM_PX * e); }

// A channel is a thin line of water decimetres deep: coloured by that depth alone it shows only its bed, a grey line
// of gravel. A map draws a river as water, and its bed's shape is what reads: it is coloured as its own depth as a
// lake's (`LAKE_SCALE`) and `RIVER_SEEN_M` more where it is as deep as the channel law makes its middle, that share
// (`rel`) to the power `RIVER_SEEN_POW`: the shallows show their bed, the deep thread is darker and greener. Standing
// liquid past its shore's band is coloured `STANDING_SEEN_M` deeper (`rel` how far through the band), so its open
// water reads as water over a shallow shelf too, and its shallows grade to it.
const RIVER_SEEN_M = 2.0; const RIVER_SEEN_POW = 1.6; const STANDING_SEEN_M = 8.0;
// The depth a channel's water `depth` m deep is coloured as, `rel` of the depth of its middle by the channel law.
fn river_seen(depth: f32, rel: f32) -> f32 { return LAKE_SCALE * depth + RIVER_SEEN_M * pow(max(rel, 0.0), RIVER_SEEN_POW); }
// The depth standing liquid `depth` m deep (as a lake's) is coloured as, `rel` of the way through its shore's band.
fn standing_seen(depth: f32, rel: f32) -> f32 { return LAKE_SCALE * depth + STANDING_SEEN_M * pow(max(rel, 0.0), RIVER_SEEN_POW); }

// Bed `bed` (linear, lit) seen through `depth` m of water in the style, whose surface has slope g, of attenuation `k`,
// 1/m, over the light `deep` that comes back where it is too deep to show its bed, carrying `load` g/m³ (water.wgsl
// `loaded_k`): through its stretched attenuation (the bed lightened, `BED_LIGHTEN`), over its deep colour.
fn painted_through(bed: vec3f, depth: f32, g: vec2f, k: vec3f, deep: vec3f, load: f32) -> vec3f {
  return seen_through(mix(bed, BEACH_SAND * BED_TONE, BED_LIGHTEN), depth, g, loaded_k(k, load), loaded_deep(deep, load));
}
// The open sea far from any shore, as the map paints it (the screen's share of full scale): what the page shows past
// the map's edge where it is drawn out to the screen's (picture.wgsl).
fn open_sea_painted() -> vec3f { return painted(srgb(standing_painted(vec3f(0.0), DEEP_M, vec2f(0.0), 1.0, 0.0))); }
// Running water (river.wgsl), inland water all.
fn water_painted(bed: vec3f, depth: f32, g: vec2f, load: f32) -> vec3f { return painted_through(bed, depth, g, INLAND_ATTENUATION, INLAND_DEEP, load); }
// Standing liquid, `sea` of it the sea's (the rest a lake's): the sea's clear water over the open sea's colour, which
// darkens with depth; a lake's (`LAKE_CLEAR`) over `LAKE_WATER`.
fn standing_painted(bed: vec3f, depth: f32, g: vec2f, sea: f32, load: f32) -> vec3f {
  let open = mix(OPEN_WATER, DEEP_WATER_PAINT, ease(log(OPEN_M), log(DEEP_M), log(max(depth, 1.0))));
  let k = mix(INLAND_ATTENUATION, CLEAR_ATTENUATION, mix(LAKE_CLEAR, 1.0, sea));
  return painted_through(bed, depth, g, k, mix(LAKE_WATER, open, sea), load);
}

// A point of a plant of colour `c` (linear) `height` m over the ground at its foot, where liquid stands `depth` m deep
// over its foot (0: on land): over the surface as it is; under it seen as the water drawn shows its bed
// (`standing_painted`: a pond's, where the plants stand), through the share of the water's column over the foot that
// lies over the point, the column seen as deep as the map draws it on the flat ground the plants stand on
// (`standing_seen`, `shore_rim`): a shallow root shows dimly, one near the bed takes the bed's look and the water's
// colour; where it meets the surface, within `e` m of height (a pixel), lighter by `WATERLINE` of the sky's light (the
// surface's ring about a stem).
const WATERLINE = 0.25;
fn seen_in_liquid(c: vec3f, depth: f32, height: f32, e: f32) -> vec3f {
  if (depth <= 0.0) { return c; }
  let below = depth - height;
  var seen = c;
  if (below > 0.0) {
    let column = standing_seen(depth, shore_rim(depth, 0.0, e, LAKE_RIM_M));
    seen = standing_painted(BED_SHOWN * c, column * min(below / depth, 1.0), vec2f(0.0), 0.0, 0.0);
  }
  return seen + WATERLINE * SKY * (1.0 - smoothstep(0.0, e, abs(below)));
}

// The foam over water `depth` m deep at `at` where the bed rises at `slope` (rise over run), seen at `e` m a pixel: its
// lines (`FOAM_LINES`) where they show, else its rim; fading out as its band grows narrower than a pixel (`FOAM_PX`),
// where it would only show the pixels. (Where that fades it to none nothing more is read; and more than `FOAM_PAST` of
// the lines' spacing past the last line, the lines are none: a line's wander and width reach 0.68 of it at most.)
const FOAM_PAST = 1.0;
fn foam(depth: f32, slope: f32, e: f32, at: vec2f) -> f32 {
  let rise = max(slope, 1e-3);
  let shows = ease(FOAM_PX.x, FOAM_PX.y, FOAM_M / rise / max(e, 1e-3));
  if (shows <= 0.0) { return 0.0; }
  var white = FOAM_RIM * (1.0 - ease(0.0, FOAM_M, depth));
  // (The lines' breaks cannot show in a program made for draws from far, draw.wgsl `LEAST_E`: there the foam is its
  // rim, and the lines are not compiled.)
  if (!unseen(FOAM_BREAK_M)) {
    let lines = ease(FOAM_LINE_PX.x, FOAM_LINE_PX.y, FOAM_LINE_M / rise / max(e, 1e-3));
    // The lines, counted from the edge of the water drawn (`FILM_M`): the nearest one's share at this depth.
    let x = max(depth - FILM_M, 0.0) / FOAM_LINE_M;
    var line = 0.0;
    if (lines > 0.0 && x <= FOAM_LINES + FOAM_PAST) {
      let k = min(floor(x + 0.5), FOAM_LINES);
      let n = noise_turned(at / FOAM_BREAK_M + 7.3 * k, 110);
      if (k == 0.0) {
        line = (1.0 - smoothstep(0.0, FOAM_LINE_W, x)) * mix(FOAM_EDGE_KEPT, 1.0, ease(-0.2, 0.3, n));
      } else {
        let wander = noise_turned(at / FOAM_WANDER_M + 3.1 * k, 115);
        let across = abs(x - k + FOAM_WANDER * wander) / (FOAM_LINE_W * (1.0 + 0.5 * noise_turned(at / FOAM_WANDER_M + 5.7, 116)));
        let breaks = smoothstep(-0.1, 0.45, n + 0.5 * noise_turned(at * (3.0 / FOAM_BREAK_M) + 1.9 * k, 117));
        line = (1.0 - smoothstep(0.3, 1.0, across)) * breaks * pow(FOAM_LINE_FALL, k);
      }
    }
    white = mix(white, line, lines);
  }
  return FOAM * white * shows;
}

// The marks (cover.wgsl): each mark differs a little from its kind in hue and tone. A mark's hue turns from its kind's
// by up to this many radians either way, and its tone by up to this share.
const HUE_JITTER = 0.25; const TONE_JITTER = 0.2;

// Colour `c` (linear) of a mark turned in hue and tone by its own random numbers `r` (0..1 each): a turn about the grey
// axis keeps its lightness.
fn vary(c: vec3f, r: vec2f) -> vec3f {
  let turn = HUE_JITTER * (2.0 * r.x - 1.0);
  let k = (1.0 - cos(turn)) / 3.0;
  let q = sin(turn) * 0.57735;
  let m = mat3x3f(cos(turn) + k, k + q, k - q, k - q, cos(turn) + k, k + q, k + q, k - q, cos(turn) + k);
  return clamp(m * c, vec3f(0.0), vec3f(1.0)) * (1.0 + TONE_JITTER * (2.0 * r.y - 1.0));
}
