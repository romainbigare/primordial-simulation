// Textures of the plant cover (vegetation.ts), drawn per pixel on the GPU (WebGL2). Each kind of mark (a tuft of grass,
// of tundra or of steppe, a shrub, a tree's crown of 4, 8 or 14 m) lies on its own grid fixed in the world, one mark a
// grid cell at a place, size, tone and rank hashed from the cell (Worley 1996; the hash is pcg3d, Jarzynski and Olano
// 2020): the marks hold while the camera moves, never repeat and stand in no rows. A mark shows where its rank is below
// the density of its kind at its centre, read from the field of the cover (blended between the field's texels), so a
// thinner cover shows fewer marks and two kinds mix mark by mark where one gives way to the other. A kind's marks fade
// in as they grow from under a pixel. From above a crown is a soft round blob over its shadow, lit from the north-west;
// angled, it is lifted over its shadow. A tuft is a small turned blob, angled a few blades standing up.
import { Rgb, Style } from './sprites';

/** A kind of mark: its size across, m, and the side of its grid cell over its size. */
interface Lattice { mark: number; spacing: number }
/** A tree's crown is drawn at these sizes across, m, blended between the two nearest. */
export const CROWN_SIZES = [4, 8, 14];
/** The kinds of mark in the order they are drawn (the low under the tall): the tufts of grass, tundra and steppe (the
 * first `TUFTS`), a shrub's crown, and the trees' crowns (from `TREES`). The spacings keep a mark within its own cell
 * and the eight about it. */
const LATTICES: Lattice[] = [
  { mark: 0.25, spacing: 0.9 }, { mark: 0.35, spacing: 1.1 }, { mark: 0.5, spacing: 1.6 }, { mark: 1.5, spacing: 1.1 },
  ...CROWN_SIZES.map(mark => ({ mark, spacing: 0.8 })),
];
const TUFTS = 3, TREES = 4;
/** The largest mark, m: nothing shows while it takes less than the pixels a mark starts to show at. */
export const LARGEST_MARK = Math.max(...LATTICES.map(l => l.mark));
/** The channels of a field's texel, each 0..1: the density of the marks of each lattice (in `LATTICES` order: the
 * share of its cells that hold a mark), how much darker the foliage is for the wood it holds, and the shares of the
 * crowns that are woodland and jungle (the rest are forest). */
export const CH = { grass: 0, tundra: 1, steppe: 2, shrubs: 3, crowns: 4, dark: 7, woodland: 8, jungle: 9 } as const;
/** Channels a texel holds: three layers of four. */
const LAYERS = 3;
/** The colours of the marks (display, 0..255): the foliage of grass, tundra, steppe, shrubs, woodland, forest and jungle,
 * the colour a woody cover darkens towards, and how strongly the genes of a place tint them. */
export interface Palette { foliage: Rgb[]; dark: Rgb; tint: number }

/** A field of the cover: `w` x `h` texels over a window of the map, its channels (`CH`) as bytes, layer by layer. */
export interface CoverField { w: number; h: number; data: Uint8Array }
/** An empty field of `w` x `h` texels. */
export const coverField = (w: number, h: number): CoverField => ({ w, h, data: new Uint8Array(LAYERS * 4 * w * h) });
/** Set channel `ch` of texel `i` of field `f` to `v` (0..1). */
export function putChannel(f: CoverField, i: number, ch: number, v: number): void {
  f.data[((ch >> 2) * f.w * f.h + i) * 4 + (ch & 3)] = Math.round(255 * Math.min(Math.max(v, 0), 1));
}

/** A picture to paint: its size, px, the metres a pixel, the world's point at its corner, m, the field's window and the
 * window of the genes' tint (x, y, w, h in m), the pixels a mark takes from which its kind starts to show and from which
 * it shows fully, how strongly it covers the map, and the style. */
export interface Paint { w: number; h: number; mpp: number; at: [number, number]; field: number[]; tint: number[]; fade: number[]; alpha: number; style: Style }

const VERTEX = `#version 300 es
void main() { gl_Position = vec4(float((gl_VertexID & 1) << 2) - 1.0, float((gl_VertexID & 2) << 1) - 1.0, 0.0, 1.0); }`;

const N = LATTICES.length;
const FRAGMENT = `#version 300 es
precision highp float; precision highp int; precision highp sampler2DArray;
uniform sampler2DArray uField; uniform sampler2D uTint;
uniform float uH, uMpp, uAngled, uAlpha, uTintMix;
uniform vec2 uFieldUv0, uFieldUvPerM, uTintUv0, uTintUvPerM;
uniform ivec2 uBase[${N}]; uniform vec2 uFrac[${N}]; uniform float uStep[${N}], uMark[${N}], uFade[${N}], uNearLod[${N}];
uniform vec3 uFoliage[7], uDark;
out vec4 outColour;
// The foliage of the trees' kinds, after those of the lattices before them.
const int WOODLAND = ${TREES}, FOREST = ${TREES + 1}, JUNGLE = ${TREES + 2};
// A crown is drawn 0.75 to 1.25 times its kind's size; a jungle's emergent crowns (this share) are wider by this.
const float EMERGENT = 0.15, EMERGENT_WIDE = 1.2;
// An angled crown is lifted over its shadow by this share of its radius; a shadow darkens the ground this much.
const float ANGLED_LIFT = 0.7, SHADOW = 0.35;
// A crown's soft rim takes this share of its radius; its lit side is this much lighter (and as much again by its
// tone), its far rim this much darker from above and angled.
const float RIM = 0.25, LIGHT = 0.14, DARK_ABOVE = 0.2, DARK_ANGLED = 0.3;
// A tuft covers this much, is this much narrower than long, and lighter or darker by up to this; angled, its blades
// stand this many radii tall.
const float TUFT_COVER = 0.9, TUFT_NARROW = 0.55, TUFT_TONE = 0.25, BLADE_TALL = 1.4;
// No part of a crown's shadow lies farther from its foot than this many radii.
const float SHADOW_REACH = 1.4;

// pcg3d (Jarzynski and Olano 2020): three random words from three.
uvec3 pcg3d(uvec3 v) {
  v = v * 1664525u + 1013904223u;
  v.x += v.y * v.z; v.y += v.z * v.x; v.z += v.x * v.y;
  v ^= v >> 16u;
  v.x += v.y * v.z; v.y += v.z * v.x; v.z += v.x * v.y;
  return v;
}
vec3 rand3(ivec2 c, int k) { return vec3(pcg3d(uvec3(uvec2(c), uint(k)))) * (1.0 / 4294967296.0); }
// A cheaper hash for the first look at a cell (lowbias32, Wellons 2018): one word, read as three of 11, 11 and 10 bits.
vec3 place(ivec2 c, int k) {
  uint x = uint(c.x) * 0x9E3779B1u ^ uint(c.y) * 0x85EBCA77u ^ uint(k) * 0xC2B2AE3Du;
  x ^= x >> 16u; x *= 0x7feb352du; x ^= x >> 15u; x *= 0x846ca68bu; x ^= x >> 16u;
  return vec3(float(x & 2047u) / 2048.0, float((x >> 11u) & 2047u) / 2048.0, float(x >> 22u) / 1024.0);
}
// The field's layer at m metres from the picture's corner, at mip level lod (the mean of 2^lod texels square);
// nothing outside the field.
vec4 field(vec2 m, int layer, float lod) {
  vec2 uv = uFieldUv0 + m * uFieldUvPerM;
  if (any(lessThan(uv, vec2(0.0))) || any(greaterThan(uv, vec2(1.0)))) return vec4(0.0);
  return textureLod(uField, vec3(uv, float(layer)), lod);
}
// Paint colour rgb with coverage a over dst (premultiplied).
void over(inout vec4 dst, vec3 rgb, float a) { dst = vec4(rgb * a, a) + dst * (1.0 - a); }
// 1 inside a disc of radius r at distance d, 0 outside, soft over w.
float disc(float d, float r, float w) { return 1.0 - smoothstep(r - w, r, d); }

void crown(inout vec4 dst, vec2 p, vec2 c, float r, float tone, vec3 col, float e, float fade) {
  float lift = uAngled * r * ANGLED_LIFT;
  vec2 s = c + r * mix(vec2(0.25, 0.25), vec2(0.35, 0.15), uAngled), sr = r * vec2(1.0, mix(0.95, 0.6, uAngled));
  float ds = length((p - s) / sr);
  over(dst, vec3(0.0), SHADOW * fade * (1.0 - smoothstep(1.0 - RIM, 1.0, ds)));
  vec2 b = c - vec2(0.0, lift);
  float d = length(p - b), a = disc(d, r, max(RIM * r, e));
  if (a <= 0.0) return;
  float t = clamp(length(p - b + r * vec2(0.3, 0.35)) / (1.2 * r), 0.0, 1.0);
  vec3 lit = t < 0.5 ? mix(col, vec3(1.0), (LIGHT + 0.1 * tone) * (1.0 - 2.0 * t)) : mix(col, vec3(0.0), mix(DARK_ABOVE, DARK_ANGLED, uAngled) * (2.0 * t - 1.0));
  over(dst, lit, a * fade);
}

// A blade from its foot a to its tip b, as wide as w at its foot: the distance from p to its edge (negative inside).
float blade(vec2 p, vec2 a, vec2 b, float w) {
  vec2 ab = b - a; float h = clamp(dot(p - a, ab) / dot(ab, ab), 0.0, 1.0);
  return length(p - a - h * ab) - w * (1.0 - h);
}

void tuft(inout vec4 dst, vec2 p, vec2 c, float r, float tone, float turn, vec3 col, float e, float fade) {
  float a;
  if (uAngled > 0.5) {
    float d = 1e9;
    for (int k = -1; k <= 1; k++) d = min(d, blade(p, c, c + r * vec2(0.5 * float(k), -BLADE_TALL), 0.25 * r));
    a = 1.0 - smoothstep(-0.5 * e, 0.5 * e, d);
  } else {
    vec2 q = mat2(cos(turn), -sin(turn), sin(turn), cos(turn)) * (p - c);
    float d = length(q / vec2(r, TUFT_NARROW * r)), w = min(e / (TUFT_NARROW * r), 1.0);
    a = 1.0 - smoothstep(1.0 - w, 1.0, d);
  }
  if (a <= 0.0) return;
  vec3 shaded = mix(col, vec3(tone < 0.5 ? 0.0 : 1.0), abs(tone - 0.5) * 2.0 * TUFT_TONE);
  over(dst, shaded, TUFT_COVER * a * fade);
}

// The marks of lattice l over dst at p (m from the picture's corner), e m a pixel, of foliage darkened by dark and
// tinted by tint. Called with l a constant, so each lattice's code is its own.
void lattice(inout vec4 dst, vec2 p, int l, float e, float dark, vec4 tint) {
  // Skip a kind that is drawn faint by none of the texels this pixel's marks could stand on.
  if (uFade[l] <= 0.0 || field(p, l >> 2, uNearLod[l])[l & 3] <= 0.0) return;
  // How far a mark reaches from its foot, in radii: a tuft from above its own, an angled one its blades, a crown its
  // shadow (and an angled one its lifted crown), and an emergent crown more.
  float S = uStep[l], reach = l < ${TUFTS} ? mix(1.0, BLADE_TALL, uAngled) : mix(SHADOW_REACH, 1.0 + ANGLED_LIFT, uAngled) * (l >= ${TREES} ? EMERGENT_WIDE : 1.0);
  vec2 cell = floor(uFrac[l] + p / S);
  // An angled mark stands up over its foot: also the row south of the pixel's.
  int south = uAngled > 0.5 ? 2 : 1;
  for (int j = -1; j <= south; j++) for (int i = -1; i <= 1; i++) {
    vec2 k = cell + vec2(i, j);
    ivec2 id = uBase[l] + ivec2(k);
    vec3 h = place(id, l);
    vec2 c = (k + h.xy - uFrac[l]) * S;
    float r = 0.5 * uMark[l] * (0.75 + 0.5 * h.z);
    if (length(p - c) > reach * r + e) continue;
    vec3 g = rand3(id, l);
    vec4 f = field(c, l >> 2, 0.0);
    if (g.x >= f[l & 3]) continue;
    int kind = l;
    bool emergent = false;
    if (l >= ${TREES}) {
      vec4 share = field(c, 2, 0.0);
      kind = g.y < share.r ? WOODLAND : g.y < share.r + share.g ? JUNGLE : FOREST;
      // The rank of a mark that shows is even below its density, so its lowest ranks are a fixed share of its crowns.
      emergent = kind == JUNGLE && g.x < EMERGENT * f[l & 3];
    }
    vec3 col = mix(mix(uFoliage[kind], uDark, dark), tint.rgb / max(tint.a, 1e-3), uTintMix * tint.a);
    if (emergent) r *= EMERGENT_WIDE;
    if (l >= ${TUFTS}) crown(dst, p, c, r, g.z, col, e, uFade[l]);
    else tuft(dst, p, c, r, g.z, g.y * 3.14159265, col, e, uFade[l]);
  }
}

void main() {
  vec2 p = vec2(gl_FragCoord.x, uH - gl_FragCoord.y) * uMpp;
  vec4 tint = texture(uTint, uTintUv0 + p * uTintUvPerM), dst = vec4(0.0);
  float dark = field(p, 1, 0.0).a;
  ${LATTICES.map((_, l) => `lattice(dst, p, ${l}, uMpp, dark, tint);`).join(' ')}
  outColour = dst * uAlpha;
}`;

/** The marks drawn on the GPU: a WebGL2 canvas, the field and the genes' tint it reads, and the picture it paints. */
export class CoverPainter {
  readonly canvas = document.createElement('canvas');
  private gl: WebGL2RenderingContext | null;
  private program: WebGLProgram | null = null;
  private fieldTex: WebGLTexture | null = null;
  private tintTex: WebGLTexture | null = null;
  private loc = new Map<string, WebGLUniformLocation | null>();
  /** The field and the tint last handed in, uploaded again after the context is lost and restored. */
  private field: CoverField | null = null;
  /** Whether any texel of the field holds marks of each lattice: the GPU skips those that none holds. */
  private held: boolean[] = [];
  private tint: { w: number; h: number; data: Uint8Array } | null = null;

  constructor(private palette: Palette) {
    this.gl = this.canvas.getContext('webgl2', { premultipliedAlpha: true, preserveDrawingBuffer: true, antialias: false });
    this.canvas.addEventListener('webglcontextlost', e => e.preventDefault());
    this.canvas.addEventListener('webglcontextrestored', () => { this.program = null; this.loc.clear(); if (this.field) this.setField(this.field); if (this.tint) this.setTint(this.tint.w, this.tint.h, this.tint.data); });
  }

  /** Read the cover from field `f`. */
  setField(f: CoverField): void {
    this.field = f;
    const n = f.w * f.h;
    this.held = LATTICES.map((_, l) => { const from = (l >> 2) * n * 4 + (l & 3); for (let i = 0; i < n; i++) if (f.data[from + 4 * i]) return true; return false; });
    const gl = this.ready();
    if (!gl) return;
    this.fieldTex ??= gl.createTexture();
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D_ARRAY, this.fieldTex);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texImage3D(gl.TEXTURE_2D_ARRAY, 0, gl.RGBA8, f.w, f.h, LAYERS, 0, gl.RGBA, gl.UNSIGNED_BYTE, f.data);
    gl.generateMipmap(gl.TEXTURE_2D_ARRAY);
    smoothSampling(gl, gl.TEXTURE_2D_ARRAY, true);
  }

  /** Read the genes' tint, `w` x `h` texels over the map: per texel its colour times its weight, and its weight (RGBA
   * bytes). */
  setTint(w: number, h: number, data: Uint8Array): void {
    this.tint = { w, h, data };
    const gl = this.ready();
    if (!gl) return;
    this.tintTex ??= gl.createTexture();
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_2D, this.tintTex);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, data);
    smoothSampling(gl, gl.TEXTURE_2D);
  }

  /** Paint picture `p`; false when there is no GPU to paint with or nothing to read. */
  paint(p: Paint): boolean {
    const gl = this.ready();
    if (!gl || !this.field || !this.tint) return false;
    if (this.canvas.width !== p.w || this.canvas.height !== p.h) { this.canvas.width = p.w; this.canvas.height = p.h; }
    gl.viewport(0, 0, p.w, p.h);
    gl.clearColor(0, 0, 0, 0); gl.clear(gl.COLOR_BUFFER_BIT);
    gl.useProgram(this.program);
    const u = (name: string) => { if (!this.loc.has(name)) this.loc.set(name, gl.getUniformLocation(this.program!, name)); return this.loc.get(name)!; };
    gl.uniform1i(u('uField'), 0); gl.uniform1i(u('uTint'), 1);
    gl.uniform1f(u('uH'), p.h); gl.uniform1f(u('uMpp'), p.mpp); gl.uniform1f(u('uAngled'), p.style === 'angled' ? 1 : 0);
    gl.uniform1f(u('uAlpha'), p.alpha); gl.uniform1f(u('uTintMix'), this.palette.tint);
    // The windows as uv = uv0 + m × per, m the metres from the picture's corner.
    const window = (name: string, [x, y, w, h]: number[]) => {
      gl.uniform2f(u(`${name}Uv0`), (p.at[0] - x) / w, (p.at[1] - y) / h); gl.uniform2f(u(`${name}UvPerM`), 1 / w, 1 / h);
    };
    window('uField', p.field); window('uTint', p.tint);
    // Each grid's cell at the picture's corner, split into its whole number and its fraction in double precision, so
    // the GPU's single precision only counts the cells across the picture.
    const base = new Int32Array(2 * N), frac = new Float32Array(2 * N), fade = new Float32Array(N), near = new Float32Array(N);
    const f = this.field, texel = Math.min(p.field[2] / f.w, p.field[3] / f.h), top = Math.floor(Math.log2(Math.max(f.w, f.h)));
    LATTICES.forEach((l, k) => {
      const step = l.mark * l.spacing;
      // The mip level whose texels, read about a pixel, hold all the texels its marks' feet (at most two cells away)
      // read: half a texel of it reaches two cells and a texel.
      near[k] = Math.min(Math.max(Math.ceil(Math.log2(2 * ((2 * step) / texel + 1))), 0), top);
      for (const a of [0, 1]) { const g = p.at[a] / step; base[2 * k + a] = Math.floor(g); frac[2 * k + a] = g - Math.floor(g); }
      const px = l.mark / p.mpp, t = Math.min(Math.max((px - p.fade[0]) / (p.fade[1] - p.fade[0]), 0), 1);
      fade[k] = this.held[k] ? t * t * (3 - 2 * t) : 0;
    });
    gl.uniform2iv(u('uBase'), base); gl.uniform2fv(u('uFrac'), frac); gl.uniform1fv(u('uFade'), fade); gl.uniform1fv(u('uNearLod'), near);
    gl.uniform1fv(u('uStep'), LATTICES.map(l => l.mark * l.spacing)); gl.uniform1fv(u('uMark'), LATTICES.map(l => l.mark));
    gl.uniform3fv(u('uFoliage'), this.palette.foliage.flat().map(v => v / 255)); gl.uniform3fv(u('uDark'), this.palette.dark.map(v => v / 255));
    gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D_ARRAY, this.fieldTex);
    gl.activeTexture(gl.TEXTURE1); gl.bindTexture(gl.TEXTURE_2D, this.tintTex);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    return true;
  }

  /** The context with its program built; null without WebGL2 or while the context is lost. */
  private ready(): WebGL2RenderingContext | null {
    const gl = this.gl;
    if (!gl || gl.isContextLost()) return null;
    if (this.program) return gl;
    const shader = (type: number, src: string) => {
      const s = gl.createShader(type)!;
      gl.shaderSource(s, src); gl.compileShader(s);
      if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(`cover shader: ${gl.getShaderInfoLog(s)}`);
      return s;
    };
    const prog = gl.createProgram()!;
    gl.attachShader(prog, shader(gl.VERTEX_SHADER, VERTEX)); gl.attachShader(prog, shader(gl.FRAGMENT_SHADER, FRAGMENT));
    gl.linkProgram(prog);
    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) throw new Error(`cover program: ${gl.getProgramInfoLog(prog)}`);
    this.program = prog;
    this.fieldTex = this.tintTex = null;
    return gl;
  }
}

/** Sample the bound texture blended between its texels' centres (and its mip levels, if it has them), its edge texels
 * held beyond it. */
function smoothSampling(gl: WebGL2RenderingContext, target: number, mips = false): void {
  gl.texParameteri(target, gl.TEXTURE_MIN_FILTER, mips ? gl.LINEAR_MIPMAP_NEAREST : gl.LINEAR); gl.texParameteri(target, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
  gl.texParameteri(target, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(target, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
}
