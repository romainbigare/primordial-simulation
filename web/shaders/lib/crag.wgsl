// The Earth view's style of the ground between the plants and of its rock (colour.wgsl `colour`; the rest of the style
// is style.wgsl). Each look means what the model holds: soil has clods, the forest floor its litter; where the loose
// ground is thin the bed shows (rock.wgsl), drawn as slabs with cracks where it is gentle and as a cliff's face where
// it is steep (slope alone never bares the ground); stones lie where the loose ground is thin on a steep slope (scree)
// or bare, and in a channel as big as its power moves (rock.wgsl `stone_of`), each drawn as a rendered stone
// (stone.wgsl). Fixed in the world: nothing moves as the view pans or zooms, and what is smaller than a pixel fades
// into its mean.

// Bare soil's tone varies by up to this share over clods `CLOD_M` across.
const CLOD = 0.06; const CLOD_M = 0.5;
// Open soil (no canopy over it) is darker in the low ground of its relief and paler on its rises, as wash gathers the
// damp fines in the hollows and leaves the crests dry and coarse (from memory): by up to `SOIL_LOW` either way by how
// low it lies among its basins and `SOIL_HOLLOW` among its hollows (relief.wgsl `Relief`, so its tone follows the
// light's shapes); it varies in tone by up to `SOIL_MOTTLE` over patches `SOIL_PATCH_M` across, and in grit `GRIT_M` across by up to `GRIT`, `SAND_GRIT` times that more where all its grain is coarse (bare
// sand glints); where it is dry (wetter than `CRUST_WET` none) its crust cracks in lines `CRUST_M` and a third of that
// apart, `CRUST_W` of their spacing wide, darker by up to `CRUST_DARK`, as far as it shrinks when it dries: by its fine
// grain and `SILT_SHRINK` of its middle grain, all of it from a share `SHRINK_FULL` (clay shrinks and swells, sand
// does not; Grossman and others 1968).
const SOIL_LOW = 0.12; const SOIL_HOLLOW = 0.12; const SOIL_MOTTLE = 0.07; const SOIL_PATCH_M = 40.0; const GRIT = 0.1; const GRIT_M = 0.035; const SAND_GRIT = 1.5;
const CRUST_WET = 0.35; const CRUST_M = 1.6; const CRUST_W = 0.035; const CRUST_DARK = 0.45;
const SILT_SHRINK = 0.3; const SHRINK_FULL = 0.35;
// Under a closed canopy the bare ground is litter: its colour times `LITTER_TINT`, up to `LITTER` of it, and in the
// crowns' shade, darker by up to `UNDER_DARK`; moss grows on it in patches `MOSS_M` across over this share of it, the
// more the wetter the ground.
const LITTER = 0.8; const LITTER_TINT = vec3f(0.7, 0.78, 0.5); const UNDER_DARK = 0.6;
// In the crowns' shade the floor's own colour is this far towards its grey: no bright soil under a canopy.
const FLOOR_GREY = 0.8;
const MOSS = 0.45; const MOSS_M = 1.5; const MOSS_TINT = vec3f(0.6, 0.95, 0.45);

// Bare bed: its own colour greyed by `ROCK_GREY` and times `ROCK_TONE`, mottled by up to `MOTTLE` over `MOTTLE_M`
// (so it reads as rock from far, never as a smooth blot), its light's departure from flat ground's `ROCK_RELIEF` times
// as strong (a face shows how it is turned).
const ROCK_GREY = 0.55; const ROCK_TONE = 0.95; const MOTTLE = 0.25; const MOTTLE_M = 60.0; const ROCK_RELIEF = 1.8;
// One make of bed is not one colour. Its layers show as bands along the contours, `BAND_M` of height apart, each its own
// tone (up to `BANDS` either way), blended smoothly into the next (bedding seen across a slope), wobbling by up to
// `BAND_WOBBLE_M` along the mottle's noise; they fade out where a band takes under `BAND_PX` pixels. A stable gentle
// surface weathers darker and warmer (`VARNISH`: rock varnish and crusts of lichen take years to centuries to darken
// it), a steep face is broken fresh; lichens tint it grey-green or ochre by up to `LICHEN_HUE` along the mottle's
// noise (from memory).
const BAND_M = 18.0; const BANDS = 0.22; const BAND_WOBBLE_M = 6.0; const BAND_PX = vec2f(1.5, 4.0);
const VARNISH = vec3f(0.8, 0.74, 0.66); const LICHEN_HUE = vec3f(0.06, 0.03, -0.08);
// Gentle bare bed is slabs: a network of cracks `CRACK_W` of their spacing wide, every `SLAB_M`, darker by
// `CRACK_DARK`; each slab's tone differs by up to `SLAB_TONES`.
const SLAB_M = 4.0; const CRACK_W = 0.06; const CRACK_DARK = 0.5; const SLAB_TONES = 0.15;
// Bare bed steeper than `CLIFF_FROM` (rise over run) is a cliff's face by `CLIFF_FULL`: its strata along the contours
// every `STRATA_M` of height and `STRATA_FINE` of that, each bed of rock its own tone (up to `STRATA_TONES` either way)
// with a dark ledge at its foot (`STRATA_LINE` of its thickness, `STRATA_DARK`), wobbling by up to `STRATA_WOBBLE` m
// over `STRATA_WAVE_M` and broken where the noise over `BREAK_M` is under `BREAKS`; cracked down its face every
// `FACE_CRACK_M`.
const CLIFF_FROM = 0.9; const CLIFF_FULL = 1.4;
const STRATA_M = 8.0; const STRATA_FINE = 0.25; const STRATA_TONES = 0.35; const STRATA_LINE = 0.15; const STRATA_DARK = 0.4;
const STRATA_WOBBLE = 3.0; const STRATA_WAVE_M = 30.0; const BREAK_M = 20.0; const BREAKS = -0.25; const FACE_CRACK_M = 6.0;
// Stones (stone.wgsl): bare bed sheds blocks, which lie thick at its foot and thin out down the slope. So they lie
// where the loose ground is thinner than `SCREE_LOOSE_M` (the thinner the more of them: the square of how thin), on
// a slope from `SCREE_FROM` (all of them from `SCREE_FULL`), and below the bed that sheds them (where the loose
// ground deepens down the slope; `BELOW_LEAST` of them where it deepens up it): angular blocks up to `SCREE_STONE_M`
// across in at most `SCREE` of the places they could lie; on bare bed, `STONES_ON_BED` of them; none on a beach. A
// stone's foot is sunk by up to
// `SUNK_MOST` of its height where the loose ground about it is `SUNK_LOOSE_M` deep or more. In a channel's bed (where
// the spot holds its stones, `Spot.stony`), the largest are `STONE_SEEN` times the largest it moves (a map draws its
// gravel a size up), no smaller than `COBBLE_M`, sunk `BED_SUNK` in the bed's gravel.
const SCREE_FROM = 0.25; const SCREE_FULL = 0.7; const SCREE_LOOSE_M = 1.0; const SCREE_STONE_M = 3.0; const SCREE = 0.8;
const BELOW_LEAST = 0.2;
// A scree's blocks are `SCREE_LIFT` times as light as their bed's face: freshly broken, not yet weathered (from memory).
const SCREE_LIFT = 1.25;
const STONES_ON_BED = 0.25; const SUNK_MOST = 0.55; const SUNK_LOOSE_M = 0.5;
const STONE_SEEN = 2.5; const COBBLE_M = 0.65; const BED_SUNK = 0.3;

// A dry wash's floor (gullies.wgsl `Gullies.wash`) is the ground's colour `WASH_GREY` of the way to its grey and
// `WASH_LIFT` times as light: its floods carry the dark fines and the varnished stones' coats away and leave pale
// sand and fresh gravel (from memory).
const WASH_GREY = 0.3; const WASH_LIFT = 1.4;
fn washed(c: vec3f, wash: f32) -> vec3f { return mix(c, mix(c, vec3f(dot(c, LUMA)), WASH_GREY) * WASH_LIFT, wash); }

// How far ground of grain `grain` (`Spot.grain`) shrinks and cracks as it dries, 0..1 (`SHRINK_FULL`).
fn shrinks(grain: vec4f) -> f32 { return ease(0.0, SHRINK_FULL, grain.y + SILT_SHRINK * max(1.0 - grain.x - grain.y, 0.0)); }

// The bare ground's colour `c` (linear, unlit) at the pixel, seen at `e` m a pixel, under `canopy` of closed crowns, `wet`,
// of grain `grain` (`Spot.grain`), as low as `low` lies among its basins and its hollows (`Spot.basin`, `Spot.hollow`,
// 0..1, 0.5 where none shows): its clods, and under the crowns their shade, litter and moss.
fn floor_of(c: vec3f, e: f32, canopy: f32, wet: f32, grain: vec4f, low: vec2f) -> vec3f {
  let litter = LITTER * STYLE * canopy;
  let soft = mix(c, vec3f(dot(c, vec3f(1.0 / 3.0))), FLOOR_GREY * STYLE * canopy);
  var out = soft * mix(vec3f(1.0), LITTER_TINT, litter) * (1.0 - UNDER_DARK * STYLE * canopy);
  // Each grain only where it shows: its noise costs a few reads of the hash.
  let clods = shows_at(CLOD_M, e);
  if (clods > 0.0) { out *= 1.0 + CLOD * STYLE * noise_l(lat_axes(CLOD_M, vec2f(0.0)), 51).x * clods; }
  // Open soil: its patches, its grit and, where dry, its cracked crust.
  let open = 1.0 - min(canopy, 1.0);
  var m = SOIL_LOW * (2.0 * low.x - 1.0) + SOIL_HOLLOW * (2.0 * low.y - 1.0);
  let patches = shows_at(SOIL_PATCH_M, e);
  if (patches > 0.0) { m += SOIL_MOTTLE * patches * noise_l(lat_turned(SOIL_PATCH_M, vec2f(0.0)), 56).x; }
  out *= 1.0 + open * m;
  let grit = shows_at(2.0 * GRIT_M, e);
  if (grit > 0.0) { out *= 1.0 + GRIT * (1.0 + SAND_GRIT * grain.x) * open * grit * noise_l(lat_turned(GRIT_M, vec2f(1.0)), 58).x; }
  let dry = open * ease(CRUST_WET, 0.0, wet) * shrinks(grain) * shows_at(CRUST_W * CRUST_M, e);
  if (dry > 0.0) {
    let a = noise_l(lat_turned(CRUST_M, vec2f(7.0)), 59).x;
    let b = noise_l(lat_turned(CRUST_M / 3.0, vec2f(11.0)), 60).x;
    let line = max(1.0 - smoothstep(0.0, CRUST_W * 2.0, abs(a)), 0.7 * (1.0 - smoothstep(0.0, CRUST_W * 3.0, abs(b))));
    out *= 1.0 - CRUST_DARK * dry * line;
  }
  let moss = MOSS * litter * wet * shows_at(MOSS_M, e);
  if (moss > 0.0) { out *= mix(vec3f(1.0), MOSS_TINT, moss * ease(0.0, 0.4, noise_l(lat_axes(MOSS_M, vec2f(0.0)), 52).x)); }
  return out;
}

// The ground of a point of drawn slope `steep` (rise over run) that is a cliff.
fn cliff_of(steep: f32) -> f32 { return ease(CLIFF_FROM, CLIFF_FULL, steep); }

// A dark line where noise `n` (-1..1) crosses 0, `width` of its spacing wide: a network of cracks.
fn crack(n: f32, width: f32) -> f32 { return 1.0 - smoothstep(0.0, width, abs(n)); }

// Bare bed `bed` (linear, unlit) at height `z`, m, and `at`, of slope `steep`, lit `shade` times flat ground, seen at
// `e` m a pixel, in the style: slabs where gentle, a cliff's face where steep (the constants above). Each detail fades
// where a pixel spans more than a quarter of it.
fn rock_face(bed: vec3f, z: f32, at: vec2f, steep: f32, e: f32, shade: f32) -> vec3f {
  let mottle = noise_turned(at / MOTTLE_M, 57);
  let cliff = cliff_of(steep);
  let weathered = mix(VARNISH, vec3f(1.0), cliff) * (1.0 + LICHEN_HUE * mottle);
  let grey = mix(bed, vec3f(dot(bed, LUMA)), ROCK_GREY) * ROCK_TONE * weathered;
  var tone = (1.0 + MOTTLE * mottle) * (1.0 + (ROCK_RELIEF - 1.0) * (shade - 1.0));
  // Its layers' bands, read only where they show (each layer's tone hashed, no texture read).
  let band_shows = 1.0 - ease(BAND_M / BAND_PX.y, BAND_M / BAND_PX.x, steep * e);
  if (band_shows > 0.0) {
    let layer = (z + BAND_WOBBLE_M * mottle) / BAND_M;
    let k = floor(layer);
    let own = vec2f(f32(lowbias(vec2i(i32(k), 7), 54) & 255u), f32(lowbias(vec2i(i32(k) + 1, 7), 54) & 255u)) / 255.0 - 0.5;
    tone *= 1.0 + 2.0 * BANDS * band_shows * mix(own.x, own.y, smoothstep(0.0, 1.0, layer - k));
  }
  // The slabs and their cracks, read only where they show.
  let slab_shows = shows_at(SLAB_M * CRACK_W * 4.0, e);
  var slabs = 1.0;
  if (slab_shows > 0.0) { slabs = 1.0 + SLAB_TONES * noise_turned(at / SLAB_M + 17.0, 59) - CRACK_DARK * crack(noise_turned(at / SLAB_M, 58), CRACK_W); }
  var face = 1.0;
  // A face is no less steep than `CLIFF_FROM`, so its pixels span at least that much height: its strata are gone where
  // that is 0.3 of their spacing, and its cracks where they are under a pixel (draw.wgsl `LEAST_E`).
  if (cliff > 0.0 && CLIFF_FROM * least_e() < 0.3 * STRATA_M) {
    let h = z + STRATA_WOBBLE * noise_turned(at / STRATA_WAVE_M, 53);
    let whole = ease(BREAKS - 0.1, BREAKS + 0.1, noise_turned(at / BREAK_M, 60));
    let span = steep * e;
    for (var k = 0; k < 2; k++) {
      let spacing = STRATA_M * select(1.0, STRATA_FINE, k == 1);
      let layer = h / spacing;
      let shown = 1.0 - ease(0.15 * spacing, 0.3 * spacing, span);
      let own = f32(lowbias(vec2i(i32(floor(layer)), k), 56) & 255u) / 255.0 - 0.5;
      let ledge = (1.0 - smoothstep(STRATA_LINE * spacing, STRATA_LINE * spacing + span, fract(layer) * spacing)) * whole;
      face *= 1.0 + shown * (STRATA_TONES * own - (1.0 - STRATA_DARK) * ledge);
    }
    face *= 1.0 - CRACK_DARK * crack(noise_turned(at / FACE_CRACK_M, 61), CRACK_W) * shows_at(FACE_CRACK_M * CRACK_W * 4.0, e);
  }
  tone *= mix(mix(1.0, slabs, slab_shows), face, cliff);
  return grey * max(tone, 0.0);
}
