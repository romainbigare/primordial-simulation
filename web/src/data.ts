// Reader for the EWF2 tape: gzip( "EWF2" | u32 json_len | json | frames of u8 layers ), as tape/src/rec/ewf.rs writes it.

export interface Layer { name: string; unit: string; scale: 'lin' | 'log' | 'raw'; min: number; max: number }
export interface Meta {
  w: number; h: number; cell_m: number; seed: number; days_per_year: number;
  /** A globe: rows run pole to pole, columns wrap east to west; rows are evenly spread from lat_north to lat_south (degrees). */
  globe: boolean; lat_north: number; lat_south: number;
  layers: Layer[];
  /** The live world only: the depth of a square below its air block. */
  square_depth?: number;
  frames: { day: number; year: number }[];
  /** One value a year per column; null where the tape wrote NaN (none that year). */
  series: Record<string, (number | null)[]>;
  series_units: Record<string, string>;
  events: { day: number; year: number; text: string }[];
  /** The rows of months.csv (a resumed run reads them; the viewer does not show them). */
  months: { year: number; month: number; values: number[]; births: number[]; deaths: number[] }[];
}
export interface World { meta: Meta; data: Uint8Array; base: number; frameBytes: number; lut: Float32Array[]; index: Map<string, number> }

/** Latitude of the centre of row y, degrees north. */
export const latOf = (m: Meta, y: number) => m.lat_north + ((m.lat_south - m.lat_north) * (y + 0.5)) / m.h;
/** Longitude of the centre of column x of a globe, degrees east of the map's west edge. */
export const lonOf = (m: Meta, x: number) => ((x + 0.5) * 360) / m.w;

export function decodeByte(l: Layer, b: number): number {
  if (l.scale === 'raw') return b / 255;
  if (l.scale === 'lin') return l.min + (b / 255) * (l.max - l.min);
  if (b === 0) return 0;
  const a = Math.log(l.min);
  return Math.exp(a + ((b - 1) / 254) * (Math.log(l.max) - a));
}

export async function gunzip(buf: ArrayBuffer): Promise<Uint8Array> {
  const b = new Uint8Array(buf);
  if (b[0] !== 0x1f || b[1] !== 0x8b) return b; // already inflated (e.g. by the web server)
  const stream = new Blob([b]).stream().pipeThrough(new DecompressionStream('gzip'));
  return new Uint8Array(await new Response(stream).arrayBuffer());
}

export function parse(data: Uint8Array): World {
  if (String.fromCharCode(...data.subarray(0, 4)) !== 'EWF2') throw new Error('Not an EWF2 world file.');
  const jsonLen = new DataView(data.buffer, data.byteOffset).getUint32(4, true);
  const meta = JSON.parse(new TextDecoder().decode(data.subarray(8, 8 + jsonLen))) as Meta;
  const base = 8 + jsonLen;
  const have = Math.floor((data.length - base) / (meta.w * meta.h * meta.layers.length));
  if (have < 1) throw new Error('The file holds no complete frame.');
  meta.frames = meta.frames.slice(0, have); // tolerate a tape still being written
  return makeWorld(meta, data, base);
}

/** A world of `meta` whose frames lie in `data` from byte `base` on. */
export function makeWorld(meta: Meta, data: Uint8Array, base: number): World {
  return {
    meta, data, base, frameBytes: meta.w * meta.h * meta.layers.length,
    lut: meta.layers.map(l => Float32Array.from({ length: 256 }, (_, b) => decodeByte(l, b))),
    index: new Map(meta.layers.map((l, k) => [l.name, k])),
  };
}

/** Raw bytes of one layer in one frame (the tape writes every layer of tape/src/layers.rs in every frame). */
export function plane(w: World, f: number, name: string): Uint8Array {
  const k = w.index.get(name);
  if (k === undefined) throw new Error(`The file has no layer ${name}.`);
  const n = w.meta.w * w.meta.h, o = w.base + f * w.frameBytes + k * n;
  return w.data.subarray(o, o + n);
}

/** Decoded values of one layer in one frame. */
export function decode(w: World, f: number, name: string): Float32Array {
  const p = plane(w, f, name);
  const lut = w.lut[w.index.get(name)!], out = new Float32Array(p.length);
  for (let i = 0; i < p.length; i++) out[i] = lut[p[i]];
  return out;
}

/** A missing value: null (a NaN the tape wrote) or NaN. */
export const missing = (v: number | null): v is null => v === null || Number.isNaN(v);

export function fmt(v: number | null): string {
  if (missing(v)) return '–';
  const a = Math.abs(v);
  if (a === 0) return '0';
  if (a >= 1e7 || a < 1e-3) return v.toExponential(2).replace('e+', 'e');
  return v.toLocaleString('en', { maximumSignificantDigits: 4 });
}

/** Unit text for display; "1" means dimensionless. */
export const unitText = (u?: string) => (!u || u === '1' ? '' : u);

export function fmtYear(y: number): string {
  return y.toLocaleString('en', { maximumFractionDigits: Math.abs(y) < 100 && !Number.isInteger(y) ? 2 : 1 });
}
