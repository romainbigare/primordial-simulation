import { World, gunzip, parse, plane, fmt, fmtYear, unitText, latOf, lonOf } from './data';
import { CLASSES, PATH_LAYER, mapFields, rampFor, rampCss } from './earth';
import { MapView, View } from './map';
import { MapLook } from './field-window';
import { Charts } from './charts';
import { Live, Stats } from './live';
import { GroundTiles } from './ground';
import { Land } from './land';
import { Avatars, THINGS_PX } from './avatars';
import { Vegetation } from './vegetation';
import { Style } from './sprites';

const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;
const LABELS: Record<string, string> = {
  t_ground: 'Ground temperature', t_air: 'Air temperature', rain: 'Rain', vapor: 'Vapor', absorber: 'Absorber',
  depth: 'Liquid depth', flux: 'River flux', catch: 'Catchment', loose: 'Loose sediment', wet: 'Ground wetness', ice: 'Ice', z: 'Elevation',
};
const ORDER = Object.keys(LABELS);
const label = (n: string) => LABELS[n] ?? (n[0].toUpperCase() + n.slice(1).replace(/_/g, ' '));
const SPEEDS = [1, 2, 4, 8, 16, 32];
/** The live world's speeds, months of the world per second of real time. */
const LIVE_SPEEDS = [3, 12, 36, 120, Infinity];
const LIVE_SIZE = 22;
/** Time the view rests before the worker is asked for the things of the window in view, ms. */
const ASK_MS = 150;

let world: World | null = null;
let live: Live | null = null;
let frame = 0, speed = 4, playing = false, view = 'earth', layer: string | null = null, paths = false;
let cls: Uint8Array = new Uint8Array(0);

const map = new MapView($<HTMLCanvasElement>('cv'), () => inspect(), () => inspect(), v => viewSoon(v));
/** The live map: the ground's tiles and the cells' table (live world only). */
let land: Land | null = null;
/** The core's WebAssembly, for the worker that makes the ground's tiles of each new world. */
let coreWasm: ArrayBuffer | null = null;
let viewTimer = 0;
/** The things of the window in view, drawn one by one when the map is close (live world only), and the window asked for. */
const avatars = new Avatars();
let thingsKey = '';
/** The cover of the live map (earth view only), under the avatars. */
const vegetation = new Vegetation();
/** Where the player keeps the style life is drawn in (one per viewer). */
const LIFE_KEY = 'ew.life';
/** How life is drawn: from above or angled, as the player last chose (from above if the browser keeps nothing). */
let life: Style = 'above';
try { if (localStorage.getItem(LIFE_KEY) === 'angled') life = 'angled'; } catch { /* no storage: the default */ }
avatars.style = vegetation.style = life;
map.layer = vegetation;
map.overlay = (ctx, ox, oy, s) => avatars.draw(ctx, ox, oy, s, px => map.under(px));
/** Which GPU draws the map (none: the page cannot draw it). */
let drawnWith = '';
map.ready.then(g => { drawnWith = g ? g.kind : ''; if (!g) say('This browser has neither WebGPU nor WebGL2, so the map cannot be drawn.'); });
avatars.listen($<HTMLCanvasElement>('cv'), () => { inspect(); map.redraw(); });
const charts = new Charts($('charts'), year => world && setFrame(nearestFrame(year)));

// ---- loading
async function load(buf: ArrayBuffer): Promise<void> {
  let next: World;
  try {
    next = parse(await gunzip(buf));
  } catch (e) { return say(String(e instanceof Error ? e.message : e)); }
  live?.stop(); live = null;
  land = vegetation.land = null; map.setLand(null);
  world = next;
  setMode(false);
  setup(world);
}
const say = (t: string) => { $('msg').textContent = t; $('msg').hidden = !t; };

/** The live world or a tape: what only one of them has is shown, and the speeds are its own. */
function setMode(isLive: boolean): void {
  $('live').hidden = !isLive;
  document.querySelectorAll<HTMLElement>('.tape-only').forEach(el => (el.hidden = isLive));
  speed = isLive ? LIVE_SPEEDS[1] : 4;
  $('speeds').replaceChildren(...(isLive ? LIVE_SPEEDS : SPEEDS).map(s => {
    const b = button(isLive ? (s === Infinity ? 'max' : `${s} mo/s`) : `×${s}`, () => { speed = s; live?.rate(s); syncUi(); });
    b.dataset.speed = String(s);
    return b;
  }));
  live?.rate(speed);
}

function setup(w: World): void {
  const m = w.meta;
  say('');
  cls = new Uint8Array(m.w * m.h);
  map.setWorld(m.w, m.h, m.cell_m, m.globe ? { north: m.lat_north, south: m.lat_south } : null);
  $('meta').innerHTML = [['seed', m.seed], ...(m.globe ? [['world', 'globe']] : []), ['size', `${m.w}×${m.h}`], ...(m.globe ? [] : [['band', `${fmt(m.lat_south)}° to ${fmt(m.lat_north)}°`]]), ['cell', m.cell_m >= 1000 ? `${fmt(m.cell_m / 1000)} km` : `${fmt(m.cell_m)} m`]]
    .map(([k, v]) => `<span>${k} <b>${v}</b></span>`).join('') + '<span>year <b id="hy"></b></span>';
  const names = m.layers.map(l => l.name).filter(n => !/^(true|ground)_[rgb]$/.test(n));
  const sorted = [...names.filter(n => ORDER.includes(n)).sort((a, b) => ORDER.indexOf(a) - ORDER.indexOf(b)), ...names.filter(n => !ORDER.includes(n))];
  $('layers').replaceChildren(...sorted.map(n => button(label(n), () => setLayer(layer === n ? null : n), n)));
  const years = m.frames.map(f => f.year), last = years.length - 1;
  if (!live) charts.set(m.series, m.series_units, years);
  const tl = $<HTMLInputElement>('tl');
  tl.max = String(last); tl.disabled = last < 1;
  $('y0').textContent = `year ${fmtYear(years[0])}`; $('y1').textContent = `year ${fmtYear(years[last])}`;
  $('marks').replaceChildren(...m.events.map(ev => {
    const i = nearestFrame(ev.year), el = document.createElement('i');
    el.style.setProperty('--p', String(last ? i / last : 0));
    const text = `Year ${fmtYear(ev.year)}: ${ev.text}`;
    el.onmouseenter = () => tip(el, text); el.onmouseleave = () => ($('tip').hidden = true);
    el.onclick = () => { setFrame(i); tip(el, text); };
    return el;
  }));
  $('paths').hidden = !w.index.has(PATH_LAYER);
  playing = false; layer = null; paths = false;
  setFrame(0); setLayer(null); syncUi();
  if (live) togglePlay();
}

function nearestFrame(year: number): number {
  const ys = world!.meta.frames.map(f => f.year);
  return ys.reduce((best, y, i) => (Math.abs(y - year) < Math.abs(ys[best] - year) ? i : best), 0);
}

// ---- state changes
function setFrame(f: number): void {
  if (!world) return;
  frame = Math.min(Math.max(f, 0), world.meta.frames.length - 1);
  const y = world.meta.frames[frame].year;
  $<HTMLInputElement>('tl').value = String(frame);
  $('year').textContent = `Year ${fmtYear(y)}`;
  document.body.dataset.year = String(y);
  const hy = document.getElementById('hy'); if (hy) hy.textContent = fmtYear(y);
  if (!live) charts.setYear(y);
  repaint();
}

/** Draw frame `frame` in the view shown; the map's classes come back from the GPU, and with them the cover. */
function repaint(): void {
  if (!world) return;
  const w = world, f = frame, squares = mapFields(w, f), look = mapLook(w);
  map.setMap(squares.fields);
  if (land && live) land.set(w, live.table, live.takeDischarge(), squares.wet, look);
  vegetation.set(view === 'earth' && !layer && live ? w : null, live?.table ?? null);
  map.setLook(look);
  map.mapClasses().then(c => {
    if (world !== w) return;
    cls = c;
    map.invalidate(); inspect();
  });
  inspect();
}

/** What the map draws: the view or the layer shown, and the worn paths if they are. */
function mapLook(w: World): MapLook {
  return {
    mode: layer ? 'layer' : view === 'true' ? 'true' : 'earth', layer: layer ?? '', ramp: layer ? rampFor(layer) : [],
    paths: paths && w.index.has(PATH_LAYER),
  };
}

function setLayer(n: string | null): void {
  layer = n;
  const r = $('ramp'); r.hidden = !n;
  if (n && world) {
    const l = world.meta.layers[world.index.get(n)!], lut = world.lut[world.index.get(n)!];
    $('rampbar').style.background = rampCss(rampFor(n));
    [0, 128, 255].forEach((b, k) => ($('ramp').querySelectorAll('.lbl span')[k].textContent = fmt(lut[b])));
    $('rampunit').textContent = `${label(n)}${unitText(l.unit) ? ` (${unitText(l.unit)})` : ''}${l.scale === 'log' ? ', log scale' : ''}`;
  }
  syncUi(); repaint();
}

function syncUi(): void {
  document.querySelectorAll<HTMLElement>('#views button').forEach(b => b.classList.toggle('on', !layer && b.dataset.view === view));
  document.querySelectorAll<HTMLElement>('#life button').forEach(b => b.classList.toggle('on', b.dataset.life === life));
  document.querySelectorAll<HTMLElement>('#layers button').forEach(b => b.classList.toggle('on', b.dataset.layer === layer));
  document.querySelectorAll<HTMLElement>('#speeds button').forEach(b => b.classList.toggle('on', b.dataset.speed === String(speed)));
  $('paths').classList.toggle('on', paths);
  $('play').classList.toggle('playing', playing);
}

function setView(v: string): void { view = v; layer = null; setLayer(null); }
function setLife(style: Style): void {
  life = avatars.style = vegetation.style = style;
  try { localStorage.setItem(LIFE_KEY, style); } catch { /* no storage: kept for this page only */ }
  syncUi(); map.invalidate();
}

function button(text: string, fn: () => void, layerName?: string): HTMLButtonElement {
  const b = document.createElement('button');
  b.textContent = text; b.onclick = fn;
  if (layerName) b.dataset.layer = layerName;
  return b;
}

// ---- inspect
function inspect(): void {
  const el = $('inspect'), i = map.hover >= 0 ? map.hover : map.pick;
  const thing = avatars.inspect(map.hover >= 0);
  if (thing) { el.innerHTML = thing; return; }
  if (!world || i < 0) { el.innerHTML = '<p class="hint">Hover or tap the map.</p>'; return; }
  const { meta } = world, [name, color] = CLASSES[cls[i]];
  let rows = '';
  meta.layers.forEach((l, k) => {
    if (/^(true|ground)_[rgb]$/.test(l.name)) return;
    rows += `<tr><td>${label(l.name)}</td><td>${fmt(world!.lut[k][plane(world!, frame, l.name)[i]])} <u>${unitText(l.unit)}</u></td></tr>`;
  });
  const t = ['true_r', 'true_g', 'true_b'].map(n => plane(world!, frame, n)[i]);
  rows += `<tr><td>True colour</td><td><s style="background:rgb(${t})"></s></td></tr>`;
  el.innerHTML = `<div class="cell"><s style="background:${color}"></s><b>${name}</b><span>x ${i % meta.w}, y ${(i / meta.w) | 0}</span></div>${where(i)}<table>${rows}</table>`;
}

/** A globe cell's latitude and longitude, as a line above the table (nothing on an island). */
function where(i: number): string {
  const m = world!.meta;
  if (!m.globe) return '';
  const lat = latOf(m, (i / m.w) | 0), east = lonOf(m, i % m.w), lon = east > 180 ? east - 360 : east;
  const deg = (v: number, pos: string, neg: string) => `${Math.abs(v).toFixed(1)}° ${v < 0 ? neg : pos}`;
  return `<p class="where">${deg(lat, 'N', 'S')}, ${deg(lon, 'E', 'W')}</p>`;
}

function tip(at: HTMLElement, text: string): void {
  const t = $('tip'), r = at.getBoundingClientRect();
  t.textContent = text; t.hidden = false;
  t.style.left = `${Math.min(Math.max(r.left + r.width / 2 - t.offsetWidth / 2, 8), innerWidth - t.offsetWidth - 8)}px`;
  t.style.top = `${r.top - t.offsetHeight - 8}px`;
}

// ---- playback
let last = 0, acc = 0;
function loop(now: number): void {
  const dt = Math.min((now - last) / 1000, 0.25); last = now;
  if (playing && world && !live) {
    acc += dt * speed;
    if (acc >= 1) {
      const n = Math.floor(acc); acc -= n;
      if (frame + n >= world.meta.frames.length - 1) { playing = false; syncUi(); }
      setFrame(frame + n);
    }
  }
  requestAnimationFrame(loop);
}
function togglePlay(): void {
  if (!world) return;
  if (live) { playing = !playing; live.play(playing); return syncUi(); }
  if (!playing && frame >= world.meta.frames.length - 1) setFrame(0);
  playing = !playing; acc = 0; syncUi();
}

// ---- wiring
$('play').onclick = togglePlay;
$('fit').onclick = () => map.fit();
$('paths').onclick = () => { paths = !paths; syncUi(); repaint(); };
$<HTMLInputElement>('chartfind').oninput = e => charts.filter((e.target as HTMLInputElement).value);
document.querySelectorAll<HTMLElement>('#views button').forEach(b => (b.onclick = () => setView(b.dataset.view!)));
document.querySelectorAll<HTMLElement>('#life button').forEach(b => (b.onclick = () => setLife(b.dataset.life === 'angled' ? 'angled' : 'above')));
$<HTMLInputElement>('tl').oninput = e => setFrame(+(e.target as HTMLInputElement).value);
$<HTMLInputElement>('file').onchange = e => { const f = (e.target as HTMLInputElement).files?.[0]; if (f) f.arrayBuffer().then(load); };
addEventListener('keydown', e => {
  const tag = (e.target as HTMLElement).tagName;
  if ((e.target as HTMLInputElement).type === 'search') return; // typing in the chart search
  if (e.key === 't' || e.key === 'T') setView(view === 'earth' ? 'true' : 'earth');
  else if (e.key === ' ' && tag !== 'BUTTON' && tag !== 'INPUT') { e.preventDefault(); togglePlay(); }
  else if (e.key === 'ArrowLeft' && tag !== 'INPUT') setFrame(frame - (e.shiftKey ? 10 : 1));
  else if (e.key === 'ArrowRight' && tag !== 'INPUT') setFrame(frame + (e.shiftKey ? 10 : 1));
  else if (e.key === 'Escape') { setLayer(null); map.pick = -1; inspect(); map.invalidate(); }
  else if (e.key === 'f') map.fit();
  else if (e.key === 'p' && tag !== 'INPUT') $('paths').click();
});
let depth = 0;
addEventListener('dragenter', e => { e.preventDefault(); depth++; $('drop').hidden = false; });
addEventListener('dragleave', () => { if (--depth <= 0) $('drop').hidden = true; });
addEventListener('dragover', e => e.preventDefault());
addEventListener('drop', e => { e.preventDefault(); depth = 0; $('drop').hidden = true; e.dataTransfer?.files[0]?.arrayBuffer().then(load); });
matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => { charts.draw(); map.invalidate(); });
$('classes').innerHTML = CLASSES.map(([n, c]) => `<li><s style="background:${c}"></s>${n}</li>`).join('');

// ---- start
/** The live world: the core's WebAssembly (ew.wasm, beside the page) in a worker; the seed and size from the page. */
async function startLive(): Promise<void> {
  const wasm = await fetch('ew.wasm').then(r => (r.ok ? r.arrayBuffer() : Promise.reject(new Error(`ew.wasm: ${r.status}`))));
  coreWasm = wasm.slice(0);
  const q = new URLSearchParams(location.search);
  $<HTMLInputElement>('seed').value = q.get('seed') ?? '1';
  $<HTMLInputElement>('size').value = q.get('size') ?? String(LIVE_SIZE);
  live?.stop();
  world = null;
  const l = live = new Live($('worker-src').textContent ?? '', wasm, {
    ready: () => newWorld(),
    world: w => { world = w; land = vegetation.land = landOf(w); map.setLand(land); thingsKey = ''; avatars.set(null, w.meta.cell_m); setup(w); land?.tiles.view(map.view()); showStats(l.stats); },
    things: data => { if (live === l && world) avatars.set(data, world.meta.cell_m); },
    frame: stats => {
      if (live !== l) return;
      setFrame(0);
      showStats(stats);
    },
    error: text => { playing = false; syncUi(); say(text); },
  });
  setMode(true);
  say('Making the world…');
}
/** The view's tiles of the ground are asked for at once; once the view rests, the worker is asked for its things. */
function viewSoon(v: View): void {
  if (!live || !world) return;
  land?.tiles.view(v);
  clearTimeout(viewTimer);
  viewTimer = window.setTimeout(() => thingsOf(v), ASK_MS);
}
/** The live map of world `w`: its ground's tiles, made by a worker of their own from the world's seed and size. */
function landOf(w: World): Land | null {
  const m = w.meta;
  if (!coreWasm || !m.tiles || !m.laws || !m.samples || m.square_depth === undefined) return null;
  const tiles = new GroundTiles($('tiles-src').textContent ?? '', coreWasm.slice(0), m.seed, m.w, [m.lat_south, m.lat_north], { w: m.w, h: m.h, cell_m: m.cell_m, square_depth: m.square_depth }, m.tiles, () => { map.invalidate(); if (live) showStats(live.stats); }, say);
  return new Land(tiles, m.laws, m.samples);
}
/** From `THINGS_PX` pixels a square on, the core is asked for the things of the window in view, to draw them one by one. */
function thingsOf(v: View): void {
  const at: [number, number, number, number] | null = v.s >= THINGS_PX && v.w > 0 && v.h > 0 ? [v.x, v.y, v.w, v.h] : null;
  const key = JSON.stringify(at);
  if (!live || key === thingsKey) return;
  thingsKey = key;
  if (!at) avatars.set(null, world?.meta.cell_m ?? 1);
  live.things(at);
}
/** The date, the pace, the cells and the ground's tiles kept and still to come in view. */
function showStats(s: Stats): void {
  const el = $('stats'), m = world?.meta;
  if (!live || !m) { el.hidden = true; return; }
  const y = m.frames[0].year, day = Math.floor((y - Math.floor(y)) * m.days_per_year) + 1;
  el.hidden = false;
  el.innerHTML = [['date', `year ${Math.floor(y)}, day ${day}`], ['pace', `${playing ? fmt(s.pace) : '0'} of ${speed === Infinity ? 'max' : speed} months/s`],
    ['cells', String(s.cells)],
    ...(land ? [['ground', `${land.tiles.count()} tiles of ${fmt(m.cell_m / 2 ** (land.tiles.depth - (m.square_depth ?? 0)))} m, ${land.tiles.toCome()} to come`]] : []),
    ...(drawnWith ? [['map', `drawn with ${drawnWith}`]] : [])].map(([k, v]) => `<span>${k} <b>${v}</b></span>`).join('');
}

/** The band of latitudes of the URL's `band=S,N`, degrees, south first; [NaN, NaN] (the core's default band) without
 * one, or where S is not below N. */
function bandOf(text: string | null): [number, number] {
  const v = (text ?? '').split(',').map(Number);
  return v.length === 2 && v[0] < v[1] ? [v[0], v[1]] : [NaN, NaN];
}
function newWorld(): void {
  const seed = Math.max(0, Math.floor(+$<HTMLInputElement>('seed').value) || 0);
  const size = Math.min(64, Math.max(8, Math.floor(+$<HTMLInputElement>('size').value) || LIVE_SIZE));
  playing = false;
  say('Making the world…');
  live?.newWorld(seed, size, bandOf(new URLSearchParams(location.search).get('band')));
}
$('live').onsubmit = e => { e.preventDefault(); newWorld(); };

requestAnimationFrame(loop);
const embedded = document.getElementById('world-data')?.textContent?.trim();
const url = new URLSearchParams(location.search).get('f');
const waiting = () => say('Drop a .ewf.gz file here, or use “Open file”.');
if (embedded) load(Uint8Array.from(atob(embedded), c => c.charCodeAt(0)).buffer);
else if (url) fetch(url).then(r => (r.ok ? r.arrayBuffer() : Promise.reject(new Error(`${url}: ${r.status}`))))
  .then(load, waiting);
else startLive().catch(waiting);
