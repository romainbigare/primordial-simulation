// The ground's detail's strips (detail.ts) made into the levels of one texture array, off the page's thread: a Web
// Worker of its own, built into the page as text (build.mjs) and started from a Blob by detail.ts. It takes one strip's
// image at a time with the number of its first layer, whether its layers are cut-outs (the trees'), the side of their
// layers and the cover a cut-out texel shows from, and answers layer by layer with its number and its levels
// (`photoMips`), then that the strip is done (so the page decodes the next only then): one layer is read from its
// strip at a time, so neither a canvas nor what is kept grows with the strips (a phone's browser draws no canvas past
// a few million pixels).

/** A strip handed in: its image, its first layer's number, whether its layers are cut-outs, how many of its first
 * layers hold numbers, not colours (detail.ts `NUMBER_LAYERS`), the side of its layers and the cut-outs' cover. */
interface Strip { strip: ImageBitmap; first: number; cutOut: boolean; numbers: number; side: number; cut: number }
const host = self as unknown as { postMessage(m: unknown, transfer: Transferable[]): void; onmessage: ((e: MessageEvent<Strip>) => void) | null };

/** The light of an sRGB level `c` (0..1, IEC 61966-2-1), which the GPU reads an sRGB texel as (detail.ts
 * `DETAIL_FORMAT`); of each byte; and half way between each byte and the next, a light under which is nearer the
 * byte. */
const unlit = (c: number) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
const LIGHT = Float32Array.from({ length: 256 }, (_, b) => unlit(b / 255));
const EDGES = Float32Array.from({ length: 255 }, (_, b) => unlit((b + 0.5) / 255));
/** The sRGB byte whose level lies nearest light `x` (0..1): found among the edges, with no power taken. */
function srgbByte(x: number): number {
  let lo = 0, hi = 255;
  while (lo < hi) { const m = (lo + hi) >> 1; if (EDGES[m] <= x) lo = m + 1; else hi = m; }
  return lo;
}

/** The strips' levels (layers `side` texels a side), each of its `layers` after the last: level 0 as the image holds
 * it, each next one the mean of four texels (their colour weighted by their cover), in light (the GPU reads the texels
 * as light, so a level is the light of the one before on the mean); of the layers from `cutFrom` on (cut-outs drawn by
 * their cover's edge at cover `cut`, tex.wgsl `model_texel`), each level's cover scaled so as many of its texels show
 * as of level 0 (Castaño 2010), so a crown of fine leaves keeps its cover far away and does not fade to a haze. With
 * `numbers`, the layers hold numbers, not colours: each level is their mean, and each is written as the sRGB byte whose
 * light is that number, so the GPU reads it back. */
function photoMips(base: Uint8Array, layers: number, cutFrom: number, side: number, cut: number, numbers: boolean): Uint8Array[] {
  const out = [base], area = side * side, mips = Math.log2(side) + 1;
  const showing = (level: Uint8Array, l: number, n: number) => { let k = 0; for (let i = 0; i < n; i++) k += level[4 * (l * n + i) + 3] >= cut ? 1 : 0; return k / n; };
  const shown = Array.from({ length: layers }, (_, l) => (l >= cutFrom ? showing(base, l, area) : 0));
  // The colour of each texel of the last level made: its light (of a colour), or its number over 255 (each fourth, its
  // cover, unused).
  let from = new Float32Array(base.length);
  for (let i = 0; i < base.length; i++) from[i] = numbers ? base[i] / 255 : LIGHT[base[i]];
  if (numbers) out[0] = base.map((v, i) => ((i & 3) === 3 ? v : srgbByte(v / 255)));
  for (let k = 1; k < mips; k++) {
    const s = side >> k, bytes = out[k - 1], next = new Uint8Array(4 * s * s * layers), made = new Float32Array(4 * s * s * layers);
    for (let l = 0; l < layers; l++) {
      for (let y = 0; y < s; y++) for (let x = 0; x < s; x++) {
        // The four texels' places, and the texel's own.
        const t0 = 4 * ((l * 2 * s + 2 * y) * 2 * s + 2 * x), t1 = t0 + 4, t2 = t0 + 8 * s, t3 = t2 + 4;
        const o = 4 * ((l * s + y) * s + x);
        // The colour is the mean of the texels by their cover, so a cut-out's mips do not take the black about it.
        const a0 = bytes[t0 + 3], a1 = bytes[t1 + 3], a2 = bytes[t2 + 3], a3 = bytes[t3 + 3], sum = a0 + a1 + a2 + a3;
        for (let c = 0; c < 3; c++) {
          made[o + c] = sum > 0 ? (a0 * from[t0 + c] + a1 * from[t1 + c] + a2 * from[t2 + c] + a3 * from[t3 + c]) / sum : 0;
          next[o + c] = srgbByte(made[o + c]);
        }
        next[o + 3] = (sum + 2) >> 2;
      }
      if (l >= cutFrom && shown[l] > 0) keepShown(next, l, s * s, shown[l], cut);
    }
    out.push(next);
    from = made;
  }
  return out;
}

/** Scale the cover of layer `l` (`n` texels) of `level` so the share `shown` of its texels reach `cut`: the least cover
 * of that share's most covered texels is lifted (or lowered) to `cut`. */
function keepShown(level: Uint8Array, l: number, n: number, shown: number, cut: number): void {
  const counts = new Array(256).fill(0);
  for (let i = 0; i < n; i++) counts[level[4 * (l * n + i) + 3]]++;
  let above = 0, v = 255;
  while (v > 1 && above + counts[v] <= shown * n) above += counts[v--];
  const scale = cut / (v + 1);
  for (let i = 0; i < n; i++) { const o = 4 * (l * n + i) + 3; level[o] = Math.min(255, Math.round(level[o] * scale)); }
}

host.onmessage = e => {
  const { strip, first, cutOut, numbers, side, cut } = e.data;
  const c = new OffscreenCanvas(side, side), g = c.getContext('2d')!;
  for (let y = 0, layer = first; y < strip.height; y += side, layer++) {
    g.clearRect(0, 0, side, side);
    g.drawImage(strip, 0, -y);
    const levels = photoMips(new Uint8Array(g.getImageData(0, 0, side, side).data.buffer), 1, cutOut ? 0 : 1, side, cut, layer - first < numbers);
    host.postMessage({ layer, levels }, levels.map(l => l.buffer));
  }
  strip.close();
  host.postMessage({ layer: -1, levels: [] }, []);
};

export {};
