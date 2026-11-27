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
const RELIEF = 1.6; const AMBIENT = 0.62; const LIT_TOP = 1.35;
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
// Inland water by what it holds: one law for every stream and lake (Kirk 1994). Light is attenuated by the water itself
// (`INLAND_CLEAR`, stretched as the clear sea's is), by the organic matter dissolved from soils and litter, which takes
// the blue most and the red least (its absorption falls as exp(-0.014 (λ - 440 nm)): Bricaud and others 1981, red,
// green and blue `DOM_SPECTRUM`), and by the grains it carries. What comes back from water too deep to show its bed is
// what its particles scatter back over all that takes the light, R = 0.33 b_b / (a + b_b) (Gordon and others 1975),
// so water that holds nothing to scatter is dark, and water that holds fine pale grains glows. Three kinds of grains
// scatter: the load it carries (`load` g/m³; `BB_LOAD` a g/m³, brown: a muddy river); rock flour, fresh fine rock
// ground by ice and snow, carried by meltwater, so as much as the ground it runs on lies at melting (`MELT_K`): milky
// blue-green; and the fine calcite that warm water lays down, as the solubility of calcium carbonate falls as water
// warms (whitings, Thompson and others 1997), from `WHITING_K.x` to `WHITING_K.y`: turquoise. Pure water scatters
// `BB_WATER` (Morel 1974, stretched), and inland water always carries some fine clay and organic grains that hardly
// settle (its wash load), `BB_FINES` (a river's backscatter is some 0.01 to 0.05 per m), so deep water keeps a hue and
// is never black. So cold clear water reads dark blue-green, warm clear water turquoise, meltwater
// milky, the water of organic soils and forest tea-brown to black, a loaded river muddy.
// The organic matter dissolved, as its absorption at 440 nm, 1/m (stretched): `DOM_MOST` where the soil is darkest
// with it (style.wgsl `humus`) or woody crowns close over the ground (their litter, `DOM_LITTER` as much as the
// soil's), none where the ground is frozen, all of it from `DOM_THAW.y` (microbes make it in thawed soil).
// The flour scatters back up to `FLOUR` per m (pale: `FLOUR_TINT`) and takes `FLOUR_ABSORB` of that, as the ground lies
// from `MELT_K.x` to `MELT_K.y` K and no more from `MELT_K.z` to `MELT_K.w` (below, the water is frozen; above, its
// snow has melted); the calcite up to `WHITING` per m, its grains finer than the light's wavelength scattering the blue
// most (`WHITING_TINT`).
const INLAND_CLEAR = vec3f(0.32, 0.11, 0.09);
const DOM_SPECTRUM = vec3f(0.06, 0.25, 0.9); const DOM_MOST = 2.5; const DOM_LITTER = 0.6; const DOM_THAW = vec2f(273.0, 283.0);
const BB_WATER = vec3f(0.0006, 0.003, 0.006); const BB_FINES = vec3f(0.02, 0.026, 0.022); const BACKSCATTER = 0.33;
const BB_LOAD = vec3f(0.014, 0.010, 0.005);
const FLOUR = 0.4; const FLOUR_TINT = vec3f(0.9, 1.0, 1.08); const FLOUR_ABSORB = 0.15; const MELT_K = vec4f(271.0, 273.5, 275.5, 279.0);
const WHITING = 0.3; const WHITING_TINT = vec3f(0.4, 1.0, 1.3); const WHITING_K = vec2f(285.0, 300.0);
// What inland water takes from light, 1/m, and sends back where it is too deep to show its bed (linear).
struct Optics { k: vec3f, deep: vec3f }
// The optics of inland water on ground at `t` K whose soil's organic share is `organic` under woody crowns closing
// `canopy` of it, carrying `load` g/m³.
fn inland_optics(t: f32, organic: f32, canopy: f32, load: f32) -> Optics {
  let dom = DOM_MOST * min(humus(organic) + DOM_LITTER * min(canopy, 1.0), 1.0) * ease(DOM_THAW.x, DOM_THAW.y, t);
  let flour = FLOUR * ease(MELT_K.x, MELT_K.y, t) * (1.0 - ease(MELT_K.z, MELT_K.w, t));
  let whiting = WHITING * ease(WHITING_K.x, WHITING_K.y, t);
  let bb = BB_WATER + BB_FINES + flour * FLOUR_TINT + whiting * WHITING_TINT + load * BB_LOAD;
  let a = INLAND_CLEAR + dom * DOM_SPECTRUM + load * LOAD_ATTENUATION + flour * FLOUR_ABSORB;
  return Optics(a + bb, BACKSCATTER * bb / (a + bb));
}
// Clear inland water at no particular place: no organic matter, no load, cool (the water about a plant's stem,
// `seen_in_liquid`).
fn clear_inland() -> Optics { return inland_optics(WHITING_K.x, 0.0, 0.0, 0.0); }
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
const RIVER_SEEN_M = 5.0; const RIVER_SEEN_POW = 1.6; const STANDING_SEEN_M = 8.0;
// The depth a channel's water `depth` m deep is coloured as, `rel` of the depth of its middle by the channel law.
fn river_seen(depth: f32, rel: f32) -> f32 { return LAKE_SCALE * depth + RIVER_SEEN_M * pow(max(rel, 0.0), RIVER_SEEN_POW); }
// The depth standing liquid `depth` m deep (as a lake's) is coloured as, `rel` of the way through its shore's band.
fn standing_seen(depth: f32, rel: f32) -> f32 { return LAKE_SCALE * depth + STANDING_SEEN_M * pow(max(rel, 0.0), RIVER_SEEN_POW); }

// Bed `bed` (linear, lit) seen through `depth` m of water in the style of optics `o` (`inland_optics`), whose surface
// has slope g with waves of mean square slope `hidden` too small to show (water.wgsl `surface_seen`), seen from the
// map's view: the bed lightened (`BED_LIGHTEN`), over the water's deep colour, its surface's light `SHEEN` of what it
// reflects (a painted map shows the water's own colour first: the sky it mirrors would grey every water alike).
const SHEEN = 0.55;
fn painted_through(bed: vec3f, depth: f32, g: vec2f, hidden: f32, o: Optics) -> vec3f {
  let t = exp(-2.0 * o.k * depth);
  return mix(bed, BEACH_SAND * BED_TONE, BED_LIGHTEN) * t + o.deep * (1.0 - t) + SHEEN * surface_seen(g, MAP_VIEW, hidden);
}
// The open sea far from any shore, as the map paints it (the screen's share of full scale): what the page shows past
// the map's edge where it is drawn out to the screen's (picture.wgsl).
fn open_sea_painted() -> vec3f { return painted(srgb(standing_painted(vec3f(0.0), DEEP_M, vec2f(0.0), 0.0, 1.0, clear_inland(), 0.0))); }
// Standing liquid, `sea` of it the sea's (the rest a lake's, of optics `o`), carrying `load` g/m³: the sea's clear
// water over the open sea's colour, which darkens with depth, turbid with its load (water.wgsl `loaded_k`).
fn standing_painted(bed: vec3f, depth: f32, g: vec2f, hidden: f32, sea: f32, o: Optics, load: f32) -> vec3f {
  let open = mix(OPEN_WATER, DEEP_WATER_PAINT, ease(log(OPEN_M), log(DEEP_M), log(max(depth, 1.0))));
  let k = mix(o.k, loaded_k(CLEAR_ATTENUATION, load), sea);
  return painted_through(bed, depth, g, hidden, Optics(k, mix(o.deep, loaded_deep(open, load), sea)));
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
    seen = standing_painted(BED_SHOWN * c, column * min(below / depth, 1.0), vec2f(0.0), 0.0, 0.0, clear_inland(), 0.0);
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
