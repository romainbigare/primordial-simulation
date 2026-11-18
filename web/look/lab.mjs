// The tool's own blank page in the browser, where pictures are read pixel by pixel (the numbers) and laid out (the
// contact sheets): no image library needed.

/** Of a picture, the middle share kept: its edges hold the fit button and the scale bar. */
export const MIDDLE = 0.8;

/** A blank page of `browser` for reading and laying out pictures. */
export async function lab(browser) {
  const page = await browser.newPage();
  await page.setContent('<canvas id="c"></canvas>');
  return page;
}

/** The numbers of a picture (PNG bytes) over the middle `keep` share of it (`MIDDLE`, or less for the far one of a pair),
 * with the crowns' mask picture of the same view where given (`mask`). See README.md for each. */
export async function numbers(page, png, { keep = MIDDLE, mask = null } = {}) {
  return page.evaluate(async ({ png, mask, keep }) => {
    const read = async b64 => {
      const img = new Image(); img.src = `data:image/png;base64,${b64}`; await img.decode();
      const w = img.width, h = img.height, cw = Math.round(w * keep), ch = Math.round(h * keep);
      const c = new OffscreenCanvas(cw, ch), g = c.getContext('2d');
      g.drawImage(img, (cw - w) / 2, (ch - h) / 2);
      return { w: cw, h: ch, d: g.getImageData(0, 0, cw, ch).data };
    };
    const { w, h, d } = await read(png), n = w * h, lum = new Float32Array(n);
    const DARK = 50, BLACK = 16, SPECK = 40, TELL = 4, WHITE = 200, MAGENTA = 243;
    let r = 0, gg = 0, b = 0, dark = 0, black = 0, green = 0, white = 0, blue = 0;
    for (let i = 0; i < n; i++) {
      const R = d[4 * i], G = d[4 * i + 1], B = d[4 * i + 2], l = (R + G + B) / 3;
      lum[i] = l; r += R; gg += G; b += B;
      dark += l < DARK; black += l < BLACK;
      green += G > R + TELL && G > B + TELL; white += Math.min(R, G, B) > WHITE; blue += B > R + 2 * TELL && B > G;
    }
    let specks = 0, inner = 0;
    for (let y = 1; y < h - 1; y++) for (let x = 1; x < w - 1; x++) {
      let s = 0;
      for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) if (dx || dy) s += lum[(y + dy) * w + x + dx];
      specks += s / 8 - lum[y * w + x] > SPECK; inner++;
    }
    const pc = v => +(100 * v).toFixed(2), out = {
      mean: [r, gg, b].map(v => +(v / n).toFixed(1)), lightness: +((r + gg + b) / (3 * n)).toFixed(1),
      dark: pc(dark / n), black: pc(black / n), specks: pc(specks / inner), green: pc(green / n), white: pc(white / n), blue: pc(blue / n) };
    if (mask) {
      const k = (await read(mask)).d, mag = (a, i) => (a[4 * i] + a[4 * i + 2]) / 2 - a[4 * i + 1];
      let crowns = 0;
      for (let i = 0; i < n; i++) { const m = mag(d, i); crowns += Math.min(Math.max((mag(k, i) - m) / Math.max(MAGENTA - m, 1), 0), 1); }
      out.crowns = pc(crowns / n);
    }
    return out;
  }, { png: png.toString('base64'), mask: mask && mask.toString('base64'), keep });
}

/** Of two pictures of the same view (PNG bytes), over the middle `MIDDLE` of them: the share of pixels whose largest
 * channel differs by no more than `STAY` of 255 (what stays), and the mean difference of the channels. */
export async function same(page, a, b) {
  return page.evaluate(async ({ a, b, keep }) => {
    const STAY = 24;
    const read = async b64 => {
      const img = new Image(); img.src = `data:image/png;base64,${b64}`; await img.decode();
      const cw = Math.round(img.width * keep), ch = Math.round(img.height * keep), c = new OffscreenCanvas(cw, ch), g = c.getContext('2d');
      g.drawImage(img, (cw - img.width) / 2, (ch - img.height) / 2);
      return g.getImageData(0, 0, cw, ch).data;
    };
    const p = await read(a), q = await read(b), n = p.length / 4;
    let stay = 0, sum = 0;
    for (let i = 0; i < n; i++) {
      let most = 0;
      for (let c = 0; c < 3; c++) { const d = Math.abs(p[4 * i + c] - q[4 * i + c]); most = Math.max(most, d); sum += d; }
      stay += most <= STAY;
    }
    return { stay: +(100 * stay / n).toFixed(1), mean_diff: +(sum / (3 * n)).toFixed(2) };
  }, { a: a.toString('base64'), b: b.toString('base64'), keep: MIDDLE });
}

/** A contact sheet (PNG bytes) of `items` ({ label, png }) in `cols` columns, each picture whole, its label above it. */
export async function sheet(page, items, cols) {
  const b64 = await page.evaluate(async ({ items, cols }) => {
    const LABEL = 22;
    const imgs = await Promise.all(items.map(async it => { const i = new Image(); i.src = `data:image/png;base64,${it.png}`; await i.decode(); return i; }));
    const w = Math.max(...imgs.map(i => i.width)), h = Math.max(...imgs.map(i => i.height)), rows = Math.ceil(imgs.length / cols);
    const c = document.getElementById('c'); c.width = cols * w; c.height = rows * (h + LABEL);
    const g = c.getContext('2d'); g.fillStyle = '#fff'; g.fillRect(0, 0, c.width, c.height);
    g.font = '13px sans-serif'; g.fillStyle = '#000';
    imgs.forEach((im, k) => { const x = (k % cols) * w, y = Math.floor(k / cols) * (h + LABEL); g.fillText(items[k].label, x + 4, y + 15); g.drawImage(im, x, y + LABEL); });
    return c.toDataURL('image/png').split(',')[1];
  }, { items: items.map(it => ({ label: it.label, png: it.png.toString('base64') })), cols });
  return Buffer.from(b64, 'base64');
}
