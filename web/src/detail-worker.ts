// The ground's detail's strips (detail.ts) made into the levels of one texture array, off the page's thread: a Web
// Worker of its own, built into the page as text (build.mjs) and started from a Blob by detail.ts. It takes the strips'
// images, one after another, how many of the last layers are the trees', the side of their layers and the cover a
// cut-out texel shows from, and answers layer by layer with its number and its levels (`photoMips`): one layer is read
// from its strip at a time, so neither a canvas nor what is kept grows with the strips (a phone's browser draws no
// canvas past a few million pixels).

const host = self as unknown as { postMessage(m: unknown, transfer: Transferable[]): void; onmessage: ((e: MessageEvent<{ strips: ImageBitmap[]; trees: number; side: number; cut: number }>) => void) | null };

/** The strips' levels (layers `side` texels a side), each of its `layers` after the last: level 0 as the image holds
 * it, each next one the mean of four texels (their colour weighted by their cover); of the layers from `cutFrom` on
 * (cut-outs drawn by their cover's edge at cover `cut`, tex.wgsl `model_texel`), each level's cover scaled so as many
 * of its texels show as of level 0 (Castaño 2010), so a crown of fine leaves keeps its cover far away and does not
 * fade to a haze. */
function photoMips(base: Uint8Array, layers: number, cutFrom: number, side: number, cut: number): Uint8Array[] {
  const out = [base], area = side * side, mips = Math.log2(side) + 1;
  const showing = (level: Uint8Array, l: number, n: number) => { let k = 0; for (let i = 0; i < n; i++) k += level[4 * (l * n + i) + 3] >= cut ? 1 : 0; return k / n; };
  const shown = Array.from({ length: layers }, (_, l) => (l >= cutFrom ? showing(base, l, area) : 0));
  for (let k = 1; k < mips; k++) {
    const s = side >> k, from = out[k - 1], next = new Uint8Array(4 * s * s * layers);
    for (let l = 0; l < layers; l++) {
      for (let y = 0; y < s; y++) for (let x = 0; x < s; x++) {
        const at = (dx: number, dy: number, c: number) => from[4 * ((l * 2 * s + 2 * y + dy) * 2 * s + 2 * x + dx) + c];
        const o = 4 * ((l * s + y) * s + x);
        // The colour is the mean of the texels by their cover, so a cut-out's mips do not take the black about it.
        const a = [at(0, 0, 3), at(1, 0, 3), at(0, 1, 3), at(1, 1, 3)], sum = a[0] + a[1] + a[2] + a[3];
        for (let c = 0; c < 3; c++) {
          next[o + c] = sum > 0 ? Math.round((a[0] * at(0, 0, c) + a[1] * at(1, 0, c) + a[2] * at(0, 1, c) + a[3] * at(1, 1, c)) / sum) : 0;
        }
        next[o + 3] = (sum + 2) >> 2;
      }
      if (l >= cutFrom && shown[l] > 0) keepShown(next, l, s * s, shown[l], cut);
    }
    out.push(next);
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
  const { strips, trees, side, cut } = e.data;
  const layers = strips.reduce((n, s) => n + s.height / side, 0);
  const c = new OffscreenCanvas(side, side), g = c.getContext('2d')!;
  let layer = 0;
  for (const s of strips) {
    for (let y = 0; y < s.height; y += side, layer++) {
      g.clearRect(0, 0, side, side);
      g.drawImage(s, 0, -y);
      const levels = photoMips(new Uint8Array(g.getImageData(0, 0, side, side).data.buffer), 1, layer >= layers - trees ? 0 : 1, side, cut);
      host.postMessage({ layer, levels }, levels.map(l => l.buffer));
    }
    s.close();
  }
};

export {};
