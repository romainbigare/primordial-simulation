// Reader for the EWF1 tape: gzip( "EWF1" | u32 json_len | json | frames of u8 layers ).

export interface Layer { name: string; unit: string; scale: 'lin' | 'log' | 'raw'; min: number; max: number }
export interface Meta {
  w: number; h: number; cell_m: number; seed: number; days_per_year: number;
  layers: Layer[];
  frames: { day: number; year: number }[];
  series: Record<string, number[]>;
  series_units: Record<string, string>;
  events: { day: number; year: number; text: string }[];
}
export interface World { meta: Meta; data: Uint8Array; base: number; frameBytes: number; lut: Float32Array[]; index: Map<string, number> }

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
  if (String.fromCharCode(...data.subarray(0, 4)) !== 'EWF1') throw new Error('Not an EWF1 world file.');
  const jsonLen = new DataView(data.buffer, data.byteOffset).getUint32(4, true);
  const meta = JSON.parse(new TextDecoder().decode(data.subarray(8, 8 + jsonLen))) as Meta;
  meta.series ??= {}; meta.series_units ??= {}; meta.events ??= [];
  const frameBytes = meta.w * meta.h * meta.layers.length;
  const base = 8 + jsonLen;
  const have = Math.floor((data.length - base) / frameBytes);
  if (have < 1) throw new Error('The file holds no complete frame.');
  meta.frames = meta.frames.slice(0, have); // tolerate a tape still being written
  return {
    meta, data, base, frameBytes,
    lut: meta.layers.map(l => Float32Array.from({ length: 256 }, (_, b) => decodeByte(l, b))),
    index: new Map(meta.layers.map((l, k) => [l.name, k])),
  };
}

/** Raw bytes of one layer in one frame, or null if the file lacks it. */
export function plane(w: World, f: number, name: string): Uint8Array | null {
  const k = w.index.get(name);
  if (k === undefined) return null;
  const n = w.meta.w * w.meta.h, o = w.base + f * w.frameBytes + k * n;
  return w.data.subarray(o, o + n);
}

/** Decoded values of one layer in one frame, or null. */
export function decode(w: World, f: number, name: string): Float32Array | null {
  const p = plane(w, f, name);
  if (!p) return null;
  const lut = w.lut[w.index.get(name)!], out = new Float32Array(p.length);
  for (let i = 0; i < p.length; i++) out[i] = lut[p[i]];
  return out;
}

export function fmt(v: number): string {
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
