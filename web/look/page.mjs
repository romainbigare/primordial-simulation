// The page under test: the built page (web/dist) copied with the tool's hooks, served with the saved worlds, and driven
// in a headless browser (Playwright's Chromium; with no graphics card it draws WebGL2 in software).
import { createRequire } from 'node:module';
import { createServer } from 'node:http';
import { readFileSync, writeFileSync, mkdirSync, cpSync, existsSync, createReadStream } from 'node:fs';
import { join, extname, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

/** The repository's root, and the tool's folder for what it makes (`out/look`, not in git). */
export const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
export const OUT = join(ROOT, 'out/look');

/** Playwright, found the normal way, else at `$PLAYWRIGHT` (a folder that holds its package). */
function playwright() {
  const require = createRequire(import.meta.url);
  try { return require('playwright'); } catch { /* not on the module path */ }
  if (process.env.PLAYWRIGHT) return require(process.env.PLAYWRIGHT);
  throw new Error('Playwright not found: install it, or set PLAYWRIGHT to its folder (e.g. /opt/node22/lib/node_modules/playwright).');
}

/** The hooks the tool gives the page (`window.__ew`), put at the end of the page's script, inside its scope: the map,
 * the cover, the world, its worker and its ground, play and pause, the speed, the view and the layer, opening a saved world by URL, and the numbers the marks' textures are laid out by (textures.ts `prepare`, read.mjs `marks`).
 * The shipped page holds none of this. */
const HOOKS = `
window.__ew = { map, vegetation, fire,
  get world() { return world; }, get live() { return live; }, get land() { return land; }, get playing() { return playing; },
  setView, setLayer, togglePlay, setSpeed(s) { speed = s; live && live.rate(s); syncUi(); },
  open: u => fetch(u).then(r => r.arrayBuffer()).then(openSaved),
  lattices: { LATTICES, REACH, STAND, ROWS_MOST, MARK_MARGIN, MARK_SPARE, PLANES_DOWN, GROUPS } };
`;
/** Where the hooks go: the end of the page's script (web/build.mjs). */
const SCRIPT_END = '})();\n</script>';

/** Copy the built page into `out/look/<name>` with the hooks, its programs replaced by `shaders` (the JSON of
 * web/shade) where given. Returns the folder. */
export function site(name, shaders = null) {
  const dist = join(ROOT, 'web/dist'), dir = join(OUT, name);
  if (!existsSync(join(dist, 'index.html'))) throw new Error('No built page: in web/, run npm run wasm and npm run build.');
  mkdirSync(dir, { recursive: true });
  cpSync(dist, dir, { recursive: true });
  let html = readFileSync(join(dist, 'index.html'), 'utf8');
  const at = html.lastIndexOf(SCRIPT_END);
  if (at < 0) throw new Error('The page has no script end to put the hooks at.');
  html = html.slice(0, at) + HOOKS + html.slice(at);
  if (shaders) {
    const i = html.indexOf('const SHADERS = ') + 16, j = html.indexOf(';\n', i);
    html = html.slice(0, i) + shaders + html.slice(j);
  }
  writeFileSync(join(dir, 'index.html'), html);
  return dir;
}

const TYPES = { '.html': 'text/html', '.wasm': 'application/wasm', '.png': 'image/png', '.jpg': 'image/jpeg', '.gz': 'application/gzip', '.js': 'text/javascript' };

/** Serve `out/look` on a free port: the sites and the saved worlds (as files, never re-encoded). */
export function serve() {
  return new Promise(ok => {
    const server = createServer((req, res) => {
      const f = join(OUT, decodeURIComponent(new URL(req.url, 'http://x').pathname));
      if (!f.startsWith(OUT) || !existsSync(f)) { res.writeHead(404); return res.end(); }
      res.writeHead(200, { 'content-type': TYPES[extname(f)] || 'application/octet-stream' });
      createReadStream(f).pipe(res);
    });
    server.listen(0, '127.0.0.1', () => ok({ server, url: `http://127.0.0.1:${server.address().port}` }));
  });
}

const sleep = ms => new Promise(r => setTimeout(r, ms));

/** A headless browser. */
export async function launch() {
  return playwright().chromium.launch();
}
/** In `browser`, the page of `site` on the saved world `world` (both paths under `out/look`, served at `base`), its map
 * `px` css pixels a side, once it stands on the world. Errors of the page are kept in `page.errors`. */
export async function open(browser, base, site, world, { px = 400 } = {}) {
  const page = await browser.newPage({ viewport: { width: px + 340, height: px + 200 } });
  page.errors = [];
  page.on('console', m => { if (m.type() === 'error' && !/GPU stall|adapter/i.test(m.text())) page.errors.push(m.text()); });
  page.on('pageerror', e => page.errors.push('pageerror: ' + e.message));
  await page.goto(`${base}/${site}/index.html?world=${base}/${world}`);
  await page.evaluate(px => { const m = document.getElementById('map'); Object.assign(m.style, { width: `${px}px`, height: `${px}px`, flex: 'none', aspectRatio: 'auto' }); }, px);
  await opened(page);
  return page;
}

/** Wait until the page stands on a world and its ground is made. */
export async function opened(page, ms = 300000) {
  await page.waitForFunction(() => { const e = window.__ew; return e && e.world && e.land && e.land.tiles.rock; }, null, { timeout: ms, polling: 250 });
}

/** Open the saved world at `url` in the running page (`__ew.open`) and wait for it. */
export async function reopen(page, url) {
  await page.evaluate(u => { window.__ewWas = window.__ew.world; return window.__ew.open(u); }, url);
  await page.waitForFunction(() => { const e = window.__ew; return e.world && e.world !== window.__ewWas && e.land && e.land.tiles.rock; }, null, { timeout: 300000, polling: 250 });
}

/** Wait until the picture is whole: no tile to come, no frame due, the sharp picture drawn; the ms waited, or -1. */
export async function settle(page, most = 120000) {
  const t0 = Date.now(); let calm = 0;
  while (Date.now() - t0 < most) {
    const busy = await page.evaluate(() => { const e = window.__ew, m = e.map;
      return (e.land ? e.land.tiles.toCome() : 0) > 0 || m.gpuDue || !!m.sharpening || !!m.raf || m.timing || (e.land && e.land.tiles.fading(performance.now())); });
    calm = busy ? 0 : calm + 1;
    if (calm >= 4) return Date.now() - t0;
    await sleep(250);
  }
  return -1;
}

/** Put world point (x, y in squares) at the middle of the map, a css pixel `mpp` metres. */
export async function look(page, { x, y, mpp }) {
  await page.evaluate(({ x, y, mpp }) => { const e = window.__ew, m = e.map, cv = document.getElementById('cv'), s = e.world.meta.cell_m / mpp;
    m.cam = { s, ox: cv.clientWidth / 2 - x * s, oy: cv.clientHeight / 2 - y * s }; m.fitted = false; m.invalidate(); }, { x, y, mpp });
  await sleep(400);
}

/** The picture of the map now, whole (`settle`), as PNG bytes; written to `out` where given. */
export async function shot(page, out = null) {
  const waited = await settle(page);
  await page.mouse.move(2, 2);
  const png = await (await page.$('#map')).screenshot();
  if (out) { mkdirSync(dirname(out), { recursive: true }); writeFileSync(out, png); }
  return { png, waited };
}

/** The year of the world the page stands on. */
export const year = page => page.evaluate(() => window.__ew.world.meta.frames[0].year);
