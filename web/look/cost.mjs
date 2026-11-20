// The cost of a pixel as a count: the page's programs built again (variant.mjs) so that each pixel counts what it reads,
// every texture read (each call of textureSample* and textureLoad), each gradient noise (draw.wgsl `gradient_noise`)
// and each read of a photograph (tex.wgsl `photo`), and draws the counts as its colour (red, green, blue: a level of
// 255 two texture reads, a noise, a photograph). One build counts the ground's programs (land.wgsl, and burnt.wgsl over
// burn scars: lib/live_map.wgsl) and draws no marks; the other counts the marks' (cover.wgsl) over a black ground. The
// count is the same on every machine and from run to run.
import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { programs, patch } from './variant.mjs';

/** The counters, ahead of every program (each starts with draw.wgsl), and the step a texture read takes. */
const COUNTERS = `var<private> n_reads: f32 = 0.0; var<private> n_noise: f32 = 0.0; var<private> n_photo: f32 = 0.0;
fn counted(v: vec4f) -> vec4f { n_reads += 1.0; return v; }
`;
/** Texture reads a level of red: a pixel of the marks reads more than 255 textures (noise and photographs a level each). */
const READ_STEP = 2;
const COLOUR = `vec4f(vec3f(n_reads / ${READ_STEP}.0, n_noise, n_photo) / 255.0, 1.0)`;
/** A texture read of WGSL (every texture of the page holds floats: each read is a vec4f). */
const READ = /\btexture(?:Sample\w*|Load)\(/g;
const LAND_OUT = '  return vec4f(clamp(rgb / 255.0, vec3f(0.0), vec3f(1.0)) * draw.alpha, draw.alpha);';
const FLAMES_OUT = '  return vec4f(tone * a + lit, a + shine * (1.0 - a)) * draw.alpha;';
const SNAGS_OUT = '  return mix(vec4f(0.0, 0.0, 0.0, min(mean, 1.0)), out, shows) * draw.alpha;';
const MARKS_OUT = '@fragment fn fs(v: Varying) -> @location(0) vec4f { return marks_over(v); }';

/** The counted programs: `ground`, the ground's, or `marks`, the marks'. */
export const PASSES = {
  ground: { land: `  return ${COLOUR};`, marks: '@fragment fn fs(v: Varying) -> @location(0) vec4f { return vec4f(0.0); }' },
  marks: { land: '  return vec4f(0.0, 0.0, 0.0, 1.0);', marks: `@fragment fn fs(v: Varying) -> @location(0) vec4f { _ = marks_over(v); return ${COLOUR}; }` },
};

/** WGSL text `s` with each texture read outside a comment counted (`counted(...)`), those inside its arguments too. */
function countReads(s) {
  let out = '', from = 0, m;
  const re = new RegExp(READ.source, 'g');
  while ((m = re.exec(s))) {
    if (s.slice(s.lastIndexOf('\n', m.index) + 1, m.index).includes('//')) continue;
    let depth = 0, end = m.index + m[0].length - 1;
    for (; end < s.length; end++) if (s[end] === '(') depth++; else if (s[end] === ')' && --depth === 0) break;
    const open = m.index + m[0].length;
    out += `${s.slice(from, m.index)}counted(${s.slice(m.index, open)}${countReads(s.slice(open, end))}))`;
    from = re.lastIndex = end + 1;
  }
  return out + s.slice(from);
}

/** The programs of the counted build of `pass` (`PASSES`), as the JSON web/shade writes (for `page.site`). */
export function costShaders(pass) {
  return programs(`cost-${pass}`, dir => {
    for (const sub of ['', 'lib']) for (const f of readdirSync(join(dir, sub)).filter(f => f.endsWith('.wgsl'))) {
      const p = join(dir, sub, f);
      writeFileSync(p, countReads(readFileSync(p, 'utf8')));
    }
    const ed = (file, text, by) => patch(dir, file, text, by, 'cost.mjs');
    ed('lib/draw.wgsl', 'fn gradient_noise(l: Lat, k: i32) -> vec3f {\n', 'fn gradient_noise(l: Lat, k: i32) -> vec3f {\n  n_noise += 1.0;\n');
    ed('lib/tex.wgsl', 'fn photo(t: i32, uv: vec2f, gx: vec2f, gy: vec2f) -> vec3f {\n', 'fn photo(t: i32, uv: vec2f, gx: vec2f, gy: vec2f) -> vec3f {\n  n_photo += 1.0;\n');
    const draw = join(dir, 'lib/draw.wgsl');
    writeFileSync(draw, COUNTERS + readFileSync(draw, 'utf8'));
    ed('lib/live_map.wgsl', LAND_OUT, PASSES[pass].land);
    ed('cover.wgsl', MARKS_OUT, PASSES[pass].marks);
    // The snags, flames and smoke over the map are not counted: they draw nothing over the counts.
    ed('flames.wgsl', FLAMES_OUT, '  return vec4f(0.0);');
    ed('snags.wgsl', SNAGS_OUT, '  return vec4f(0.0);');
  });
}

/** Of a count picture (PNG bytes, `costShaders`), over its middle `keep` share: of each count (reads, noise, photos)
 * the mean, the 95th percentile and the most a pixel, and the % of pixels at 255 (a count the colour cannot hold); and
 * the picture to look at, each colour's level times its `show` (red, green, blue). */
export async function counts(page, png, keep, show) {
  const r = await page.evaluate(async ({ png, keep, show, readStep }) => {
    const img = new Image(); img.src = `data:image/png;base64,${png}`; await img.decode();
    const w = Math.round(img.width * keep), h = Math.round(img.height * keep), c = new OffscreenCanvas(w, h), g = c.getContext('2d');
    g.drawImage(img, (w - img.width) / 2, (h - img.height) / 2);
    const d = g.getImageData(0, 0, w, h).data, n = w * h, out = {};
    ['reads', 'noise', 'photos'].forEach((name, ch) => {
      const v = new Uint8Array(n), step = ch === 0 ? readStep : 1;
      for (let i = 0; i < n; i++) v[i] = d[4 * i + ch];
      let sum = 0, full = 0;
      for (const x of v) { sum += x; full += x === 255; }
      v.sort();
      out[name] = { mean: +(step * sum / n).toFixed(2), p95: step * v[Math.floor(0.95 * (n - 1))], most: step * v[n - 1], clipped: +(100 * full / n).toFixed(2) };
    });
    const s = new OffscreenCanvas(img.width, img.height), sg = s.getContext('2d');
    sg.drawImage(img, 0, 0);
    const all = sg.getImageData(0, 0, img.width, img.height);
    for (let i = 0; i < all.data.length; i++) if (i % 4 !== 3) all.data[i] = Math.min(255, all.data[i] * show[i % 4]);
    sg.putImageData(all, 0, 0);
    const blob = await s.convertToBlob({ type: 'image/png' }), bytes = new Uint8Array(await blob.arrayBuffer());
    let bin = '';
    for (const b of bytes) bin += String.fromCharCode(b);
    return { out, seen: btoa(bin) };
  }, { png: png.toString('base64'), keep, show, readStep: READ_STEP });
  return { numbers: r.out, seen: Buffer.from(r.seen, 'base64') };
}
