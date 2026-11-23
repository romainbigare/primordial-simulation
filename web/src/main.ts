import { World, gunzip, parse, plane, fmt, fmtYear, unitText, latOf, lonOf } from './data';
import { CLASSES, GROUND_LAYERS, mapFields, rampFor, rampCss } from './earth';
import { MapView, View } from './map';
import type { HeldGpu } from './gpu';
import { spent } from './cost';
import { detailFailure, detailSent } from './detail';
import { MapLook } from './field-window';
import { Charts } from './charts';
import { Live, Stats } from './live';
import { GroundTiles, GroundOf } from './ground';
import { Land } from './land';
import { CoverSlots, THINGS_PX } from './slots';
import { Vegetation } from './vegetation';
import { Fire } from './fire';
import { God } from './god';
import { textureClass, textureLegend } from './soil';

const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;
const LABELS: Record<string, string> = {
  t_ground: 'Ground temperature', t_air: 'Air temperature', rain: 'Rain', vapor: 'Vapor', absorber: 'Absorber',
  depth: 'Liquid depth', flux: 'River flux', catch: 'Catchment', loose: 'Loose sediment', wet: 'Ground wetness', ice: 'Ice', z: 'Elevation',
  coarse: 'Sand (coarse grain)', fine: 'Clay (fine grain)', organic: 'Organic matter', conduct: 'Soil conductivity',
  avail: 'Available water', filled: 'Soil water (pores filled)', roots: 'Water for roots (share)',
  rises: 'Evaporation', sheds: 'Runoff and drainage (rain less evaporation)',
  litter: 'Fresh litter', slow: 'Humus (slow organic pool)', mineral_free: 'Nitrogen, dissolved', mineral_held: 'Nitrogen, held on the soil',
  rot: 'Rot speed',
  seeds: 'Seeds on the ground', grown: 'Growth last year', takers: 'Takers',
};
const ORDER = Object.keys(LABELS);
/** The layers of the soil's grain, shown with the texture classes' legend (soil.ts). */
const GRAIN_LAYERS = ['coarse', 'fine'];
const label = (n: string) => LABELS[n] ?? (n[0].toUpperCase() + n.slice(1).replace(/_/g, ' '));
const SPEEDS = [1, 2, 4, 8, 16, 32];
/** The live world's speeds, months of the world per second of real time. */
const LIVE_SPEEDS = [3, 12, 36, 120, Infinity];
/** The live world's first size and its least and most, squares a side: the first shows a land with its valleys, a
 * lake and an outlier; the most is the largest island the laws are run on, or on a screen touched with a finger (a
 * phone's, whose browser gives a page a few hundred MB) the largest whose workers fit there (each of the ground's
 * workers holds about 0.4 kB a sample: 110 MB at 32, 250 MB at 48). */
const LIVE_SIZE = 32, SIZE_LEAST = 8, SIZE_MOST = matchMedia('(pointer: coarse)').matches ? 48 : 128;
/** The live world's kinds of cover a cell may hold, a setting of the world (the core's `Config::variety`), and its
 * most (the core's `cover::SLOTS`). */
const LIVE_VARIETY = 8, VARIETY_MOST = 12;
/** The climates a live island may have: a name, and the band of latitudes it stands for (south, north), its air the
 * planet's air over that band. The first is the page's. */
const BANDS: string[][] = [['Wide: 20° to 65°', '20,65'], ['Mild: 30° to 55°', '30,55'], ['Warm: 5° to 50°', '5,50']];
/** The large shapes a live island may have: a name, and the URL's word for it. The first is the page's. */
const SHAPES: string[][] = [['Drawn from the seed', 'drawn'], ['One island', 'one']];
/** The weathers a live island may have: a name, and the URL's word for it (`ew run --weather`). The first is the
 * page's: strong, the eddies' measured speed; soft, the speed their diffusion alone gives (the core's `soft_weather`). */
const WEATHERS: string[][] = [['Strong', 'strong'], ['Soft', 'soft']];
/** Time the view rests before the worker is asked for the things of the window in view, ms. */
const ASK_MS = 150;

let world: World | null = null;
let live: Live | null = null;
let frame = 0, speed = 4, playing = false, view = 'earth', layer: string | null = null;
let cls: Uint8Array = new Uint8Array(0);
/** The frames drawn, and the one whose classes `cls` were last asked for: they are read back from the GPU only when a
 * square is inspected (`inspect`), not at every frame. */
let paints = 0, classesOf = -1;

const map = new MapView($<HTMLCanvasElement>('cv'), () => inspect(), () => inspect(), v => viewSoon(v));
/** The live map: the ground's tiles and the cells' table (live world only). */
let land: Land | null = null;
/** The core's WebAssembly, for the worker that makes the ground's tiles of each new world. */
let coreWasm: ArrayBuffer | null = null;
/** The saved world the live world stands on (the gzip of a snapshot of `ew run`), kept for the ground's tiles; null for
 * a new world. */
let saved: ArrayBuffer | null = null;
let viewTimer = 0;
/** The slots of cover of the window in view, asked for when the map is close (live world only), and the window asked for. */
const slots = new CoverSlots();
let thingsKey = '';
/** The cover of the live map (earth view only). */
const vegetation = new Vegetation();
map.layer = vegetation;
/** Fire on the live map: its scars, flames and smoke. */
const fire = new Fire();
map.fire = fire;
vegetation.fire = fire;
/** God mode: the tools that edit the live world (live world only). */
const god = new God($('god'), { probe: (x, y) => live?.probe(x, y), edit: (a, x, y, s, r) => live?.edit(a, x, y, s, r) }, () => map.redraw());
god.listen($<HTMLCanvasElement>('cv'));
map.overlay = (ctx, ox, oy, s) => { slots.seen(ox, oy, s); if (live) god.draw(ctx, ox, oy, s); };
map.tap = (x, y) => live !== null && god.tap(x, y);
/** The GPU that draws the map (null: none yet, or the page cannot draw it). */
let drawing: HeldGpu | null = null;
map.onScale = () => { if (live) showStats(live.stats); };
map.ready.then(g => { drawing = g; if (!g) say('This browser has neither WebGPU nor WebGL2, so the map cannot be drawn.'); });
slots.listen($<HTMLCanvasElement>('cv'), () => inspect());
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
/** The page's own errors, each with the time it came, shown on it (a phone has no console to read them in) until
 * tapped away: the first `ERRORS_FIRST`, which say what went wrong first, and the last `ERRORS_LAST`, with how many
 * came between. Those
 * without an error object are the browser's notes (a ResizeObserver's loop), not failures. */
const ERRORS_FIRST = 3, ERRORS_LAST = 3;
const errors: string[] = [];
function failed(text: string): void {
  errors.push(`${new Date().toLocaleTimeString()} ${text}`);
  const shown = errors.length <= ERRORS_FIRST + ERRORS_LAST ? errors
    : [...errors.slice(0, ERRORS_FIRST), `(${errors.length - ERRORS_FIRST - ERRORS_LAST} more)`, ...errors.slice(-ERRORS_LAST)];
  $('errors').textContent = `${shown.join('\n')}\n(tap to hide)`;
  $('errors').hidden = false;
}
$('errors').onclick = () => { $('errors').hidden = true; };
addEventListener('error', e => { if (e.error) failed(e.error instanceof Error ? e.error.message : String(e.error)); });
addEventListener('unhandledrejection', e => failed(e.reason instanceof Error ? e.reason.message : String(e.reason)));

/** The live world or a tape: what only one of them has is shown, and the speeds are its own. */
function setMode(isLive: boolean): void {
  $('live').hidden = !isLive;
  $('god-group').hidden = !isLive;
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
  // A new live world plays at once, before the page is set up for it and draws it, so a failure there does not hold it;
  // a saved one opens paused at its date, to be looked at.
  playing = false; layer = null;
  if (live && !saved) togglePlay();
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
  setFrame(0); setLayer(null); syncUi();
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

/** Draw frame `frame` in the view shown; the map's classes come back from the GPU for the inspector. */
function repaint(): void {
  if (!world) return;
  const w = world, f = frame, squares = mapFields(w, f), look = mapLook();
  map.setMap(squares.fields);
  if (land && live) land.set(w, live.table, live.takeDischarge(), squares.wet, look);
  if (land && live) land.plumesIn(live.plumes);
  if (land && live) land.tiles.rebuild(live.takeRelief(), !playing);
  vegetation.set(view === 'earth' && !layer && live ? w : null, live?.table ?? null);
  fire.set(land && live ? w : null);
  map.setLook(look);
  paints++;
  inspect();
}


/** What the map draws: the view or the layer shown. */
function mapLook(): MapLook {
  return { mode: layer ? 'layer' : view === 'true' ? 'true' : 'earth', layer: layer ?? '', ramp: layer ? rampFor(layer) : [], ground: GROUND_LAYERS.get(layer ?? '') ?? 0 };
}

function setLayer(n: string | null): void {
  layer = n;
  const r = $('ramp'); r.hidden = !n;
  $('texture').hidden = !GRAIN_LAYERS.includes(n ?? '');
  $('texture').innerHTML = GRAIN_LAYERS.includes(n ?? '') ? textureLegend(null) : '';
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
  document.querySelectorAll<HTMLElement>('#layers button').forEach(b => b.classList.toggle('on', b.dataset.layer === layer));
  document.querySelectorAll<HTMLElement>('#speeds button').forEach(b => b.classList.toggle('on', b.dataset.speed === String(speed)));
  $('play').classList.toggle('playing', playing);
  map.playing = playing && live !== null;
}

function setView(v: string): void { view = v; layer = null; setLayer(null); }

function button(text: string, fn: () => void, layerName?: string): HTMLButtonElement {
  const b = document.createElement('button');
  b.textContent = text; b.onclick = fn;
  if (layerName) b.dataset.layer = layerName;
  return b;
}

// ---- inspect
function inspect(): void {
  const el = $('inspect'), i = map.hover >= 0 ? map.hover : map.pick;
  const cover = slots.inspect();
  if (cover) { el.innerHTML = cover; return; }
  if (!world || i < 0) { el.innerHTML = '<p class="hint">Hover or tap the map.</p>'; return; }
  if (classesOf !== paints) {
    const w = world;
    classesOf = paints;
    map.mapClasses().then(c => { if (world === w) { cls = c; inspect(); } });
  }
  const { meta } = world, [name, color] = CLASSES[cls[i]];
  const share = (n: string) => world!.lut[world!.index.get(n)!][plane(world!, frame, n)[i]];
  const grain: [number, number] | null = world.index.has('coarse') ? [share('coarse'), share('fine')] : null;
  if (!$('texture').hidden) $('texture').innerHTML = textureLegend(grain);
  let rows = grain ? `<tr><td>Soil texture</td><td>${textureClass(...grain)}</td></tr>` : '';
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
  // (A paused world's ground is made again at once from its last build, ground.ts `rebuild`.)
  if (live) { playing = !playing; live.play(playing); if (!playing) land?.tiles.rebuild(null, true); return syncUi(); }
  if (!playing && frame >= world.meta.frames.length - 1) setFrame(0);
  playing = !playing; acc = 0; syncUi();
}

// ---- wiring
$('play').onclick = togglePlay;
$('fit').onclick = () => map.fit();
$<HTMLInputElement>('chartfind').oninput = e => charts.filter((e.target as HTMLInputElement).value);
document.querySelectorAll<HTMLElement>('#views button').forEach(b => (b.onclick = () => setView(b.dataset.view!)));
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
});
let depth = 0;
addEventListener('dragenter', e => { e.preventDefault(); depth++; $('drop').hidden = false; });
addEventListener('dragleave', () => { if (--depth <= 0) $('drop').hidden = true; });
addEventListener('dragover', e => e.preventDefault());
addEventListener('drop', e => { e.preventDefault(); depth = 0; $('drop').hidden = true; e.dataTransfer?.files[0]?.arrayBuffer().then(load); });
matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => { charts.draw(); map.invalidate(); });
$('classes').innerHTML = CLASSES.map(([n, c]) => `<li><s style="background:${c}"></s>${n}</li>`).join('');

// ---- start
/** The live world: the core's WebAssembly (ew.wasm, beside the page) in a worker; the saved world `from`, or a new one
 * of the seed, size and variety from the page. */
async function startLive(from: ArrayBuffer | null): Promise<void> {
  // (Asked of the server each time, never from the browser's cache: a page of one build on the core of another reads
  // layers the core does not make, and draws nothing.)
  const wasm = await fetch('ew.wasm', { cache: 'no-cache' }).then(r => (r.ok ? r.arrayBuffer() : Promise.reject(new Error(`ew.wasm: ${r.status}`))));
  coreWasm = wasm.slice(0);
  const q = new URLSearchParams(location.search);
  $<HTMLInputElement>('seed').value = q.get('seed') ?? '1';
  Object.assign($<HTMLInputElement>('size'), { min: String(SIZE_LEAST), max: String(SIZE_MOST), value: q.get('size') ?? String(LIVE_SIZE) });
  Object.assign($<HTMLInputElement>('variety'), { min: '1', max: String(VARIETY_MOST), value: q.get('variety') ?? String(LIVE_VARIETY) });
  // The climate: a band of latitudes the island stands for, from the URL's `band=S,N` or the first preset (`BANDS`).
  const bands = $<HTMLSelectElement>('band'), asked = q.get('band');
  const presets = asked && !BANDS.some(b => b[1] === asked) ? [...BANDS, [`${asked.replace(',', '° to ')}°`, asked]] : BANDS;
  bands.innerHTML = presets.map(([name, v]) => `<option value="${v}">${name}</option>`).join('');
  bands.value = asked ?? BANDS[0][1];
  // The large shape: from the URL's `shape=one|drawn` or the first (`SHAPES`).
  const shapes = $<HTMLSelectElement>('shape');
  shapes.innerHTML = SHAPES.map(([name, v]) => `<option value="${v}">${name}</option>`).join('');
  shapes.value = SHAPES.some(s => s[1] === q.get('shape')) ? (q.get('shape') as string) : SHAPES[0][1];
  // The weather: from the URL's `weather=strong|soft` or the first (`WEATHERS`).
  const weathers = $<HTMLSelectElement>('weather');
  weathers.innerHTML = WEATHERS.map(([name, v]) => `<option value="${v}">${name}</option>`).join('');
  weathers.value = WEATHERS.some(s => s[1] === q.get('weather')) ? (q.get('weather') as string) : WEATHERS[0][1];
  live?.stop();
  world = null;
  const l = live = new Live($('worker-src').textContent ?? '', wasm, {
    ready: () => (from ? openSaved(from) : newWorld()),
    world: w => { world = w; land = vegetation.land = landOf(w); map.setLand(land); thingsKey = ''; slots.set(null); god.setKinds(l.kinds); setup(w); land?.tiles.view(map.view()); showStats(l.stats); },
    things: data => { if (live === l && world) slots.set(data); },
    frame: (stats, changed) => {
      if (live !== l) return;
      if (changed) setFrame(0);
      showStats(stats);
    },
    probe: p => { if (live === l) god.probed(p); },
    edited: r => { if (live === l) god.edited(r); },
    error: text => { playing = false; syncUi(); say(text); },
  });
  l.drawn = () => map.shown();
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
/** The live map of world `w`: its ground's tiles, made by a worker of their own from the world's seed and size, or from
 * the saved world it stands on. */
function landOf(w: World): Land | null {
  const m = w.meta;
  if (!coreWasm || !m.tiles || !m.laws || !m.samples || m.square_depth === undefined) return null;
  const ground: GroundOf = saved ? { saved } : { seed: m.seed, size: m.w, band: [m.lat_south, m.lat_north], drawn: m.shape === 'Drawn' };
  const tiles = new GroundTiles($('tiles-src').textContent ?? '', coreWasm.slice(0), ground, { w: m.w, h: m.h, cell_m: m.cell_m, square_depth: m.square_depth }, m.tiles, shown => { if (shown) map.invalidate(); if (live) showStats(live.stats); }, say);
  return new Land(tiles, m.laws, m.samples);
}
/** From `THINGS_PX` pixels a square on, the core is asked for the slots of cover of the window in view. */
function thingsOf(v: View): void {
  const at: [number, number, number, number] | null = v.s >= THINGS_PX && v.w > 0 && v.h > 0 ? [v.x, v.y, v.w, v.h] : null;
  const key = JSON.stringify(at);
  if (!live || key === thingsKey) return;
  thingsKey = key;
  if (!at) slots.set(null);
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
    ...(drawing ? [['map', `drawn with ${drawing.kind} at ${fmt(map.drawnAt)}`]] : [])].map(([k, v]) => `<span>${k} <b>${v}</b></span>`).join('')
    + `<button class="costs" id="costs">${costShown ? 'hide cost' : 'cost'}</button>`;
  showCost(s);
}
/** Whether the cost line shows (`showCost`). */
let costShown = false;
$('stats').onclick = e => { if ((e.target as HTMLElement).id === 'costs') { costShown = !costShown; if (live) showStats(live.stats); } };
/** The cost line: where the time and the memory go, over the last whole second, so a screenshot of it tells where a
 * slow device spends: the world's worker (its months, the time a month takes, the share of its time it steps), the
 * page's thread (the world's frames taken, the map's draws), the GPU's time a frame, the ground's tiles (made in their
 * worker, taken on the page), the detail sent to the GPU, and the memory of the GPU's textures by what they hold, of
 * each worker's core and of the page's script where the browser says it. */
function showCost(s: Stats): void {
  const el = $('cost');
  el.hidden = !costShown;
  if (!costShown) return;
  const ms = (v: number) => v.toFixed(v < 10 ? 1 : 0), mb = (bytes: number) => (bytes / 2 ** 20).toFixed(bytes < 2 ** 23 ? 1 : 0);
  const per = (key: string) => `${ms(spent(key))} ms on ${spent(`${key} n`).toFixed(1)}`;
  const [sent, layers] = detailSent(), gpuN = spent('gpu n');
  const heap = (performance as Performance & { memory?: { usedJSHeapSize: number } }).memory?.usedJSHeapSize;
  const cards = drawing?.memory() ?? [], cardAll = cards.reduce((n, [, b]) => n + b, 0);
  el.innerHTML = [
    ['world', `${fmt(s.pace)} months/s, ${ms(s.monthMs)} ms a month, worker busy ${Math.round(100 * s.busy)}%`],
    ['page /s', `world frames ${per('world frames')}, draws ${per('draw')}`],
    ['gpu', `${gpuN > 0 ? ms(spent('gpu') / gpuN) : '-'} ms a frame; programs: ${drawing?.making() ?? '-'}`],
    ['tiles /s', `${ms(spent('tile ms'))} ms in the worker, ${per('tiles')} on the page`],
    ['detail', `${sent} of ${layers} layers sent, ${per('detail')} frames /s${detailFailure() ? `; failed: ${detailFailure()}` : ''}`],
    ['gpu MB', `${mb(cardAll)}: ${cards.map(([label, b]) => `${label} ${mb(b)}`).join(', ')}`],
    ['wasm MB', `world ${mb(s.memory)}${land ? `, tiles ${land.tiles.memory().map(mb).join(' + ')}` : ''}${heap ? `; js ${mb(heap)}` : ''}`],
  ].map(([k, v]) => `<span>${k} <b>${v}</b></span>`).join('');
}

/** The band of latitudes `S,N`, degrees, south first; [NaN, NaN] (the core's default band, its edges closed) for none,
 * or where S is not below N. */
function bandOf(text: string | null): [number, number] {
  const v = (text ?? '').split(',').map(Number);
  return v.length === 2 && v[0] < v[1] ? [v[0], v[1]] : [NaN, NaN];
}
/** Whether a banded world was made in this session: the first one makes the planet's air too, which takes a while. */
let banded = false;
function newWorld(): void {
  const seed = Math.max(0, Math.floor(+$<HTMLInputElement>('seed').value) || 0);
  const size = Math.min(SIZE_MOST, Math.max(SIZE_LEAST, Math.floor(+$<HTMLInputElement>('size').value) || LIVE_SIZE));
  const variety = Math.min(VARIETY_MOST, Math.max(1, Math.floor(+$<HTMLInputElement>('variety').value) || LIVE_VARIETY));
  const band = bandOf($<HTMLSelectElement>('band').value);
  playing = false; saved = null;
  say(band[0] < band[1] && !banded ? 'Making the world, and once the air of the planet around it (a few seconds)…' : 'Making the world…');
  banded ||= band[0] < band[1];
  live?.newWorld(seed, size, band, $<HTMLSelectElement>('shape').value === 'drawn', variety, $<HTMLSelectElement>('weather').value === 'soft');
}
/** Stand the live world on the saved world `gz` (the gzip of a snapshot of `ew run`). */
function openSaved(gz: ArrayBuffer): void {
  playing = false; saved = gz;
  say('Opening the saved world…');
  live?.open(gz.slice(0));
}
$<HTMLInputElement>('saved').onchange = e => { const f = (e.target as HTMLInputElement).files?.[0]; if (f) f.arrayBuffer().then(openSaved); };
$('live').onsubmit = e => { e.preventDefault(); newWorld(); };
$('band').onchange = () => newWorld();
$('shape').onchange = () => newWorld();
$('weather').onchange = () => newWorld();

requestAnimationFrame(loop);
const embedded = document.getElementById('world-data')?.textContent?.trim();
// A tape (`?f=URL`), or the live world: on a saved world (`?world=URL`, a snapshot of `ew run`) or a new one.
const asked = new URLSearchParams(location.search), url = asked.get('f'), savedUrl = asked.get('world');
const waiting = () => say('Drop a .ewf.gz file here, or use “Open file”.');
const fetched = (u: string) => fetch(u).then(r => (r.ok ? r.arrayBuffer() : Promise.reject(new Error(`${u}: ${r.status}`))));
if (embedded) load(Uint8Array.from(atob(embedded), c => c.charCodeAt(0)).buffer);
else if (url) fetched(url).then(load, waiting);
else if (savedUrl) fetched(savedUrl).then(startLive).catch(e => say(String(e instanceof Error ? e.message : e)));
else startLive(null).catch(e => say(`${String(e instanceof Error ? e.message : e)}. Drop a .ewf.gz file here, or use “Open file”.`));
