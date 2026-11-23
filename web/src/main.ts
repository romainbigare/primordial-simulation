import { World, gunzip, parse, fmt, fmtYear, unitText, fetchBytes } from './data';
import { CLASSES, GROUND_LAYERS, mapFields, rampFor, rampCss } from './earth';
import { MapView, View } from './map';
import type { HeldGpu } from './gpu';
import { MapLook } from './field-window';
import { Charts } from './charts';
import { Live, Stats } from './live';
import { GroundTiles, GroundOf } from './ground';
import { Land } from './land';
import { CoverSlots, THINGS_PX } from './slots';
import { Vegetation } from './vegetation';
import { Fire } from './fire';
import { God } from './god';
import { godTools } from './god-tools';
import { Panels, watchCovered } from './panels';
import { fillIcons, iconSvg } from './icons';
import { textureLegend } from './soil';
import { Inspector, LAYER_ORDER, layerLabel } from './inspect';
import { Numbers } from './numbers';
import { fillForm, readForm } from './world-form';
import { showErrors } from './errors';

const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;
/** The layers of the soil's grain, shown with the texture classes' legend (soil.ts). */
const GRAIN_LAYERS = ['coarse', 'fine'];
const SPEEDS = [1, 2, 4, 8, 16, 32];
/** The live world's speeds, months of the world per second of real time. */
const LIVE_SPEEDS = [3, 12, 36, 120, Infinity];
/** Months in a year, for the date on the bar. */
const MONTHS = 12;
/** Time the view rests before the worker is asked for the things of the window in view, ms. */
const ASK_MS = 150;

let world: World | null = null;
let live: Live | null = null;
let frame = 0, speed = 4, playing = false, view = 'earth', layer: string | null = null;

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
/** God mode: the tools that edit the live world (live world only), shown while the god button is pressed. */
const editor = { probe: (x: number, y: number) => live?.probe(x, y), edit: (a: number, x: number, y: number, s: number, r: number) => live?.edit(a, x, y, s, r) };
const god = new God($('god-strip'), $('god'), $('toast'), editor, godTools(editor, () => live?.kinds ?? []), () => map.redraw());
god.listen($<HTMLCanvasElement>('cv'));
map.overlay = (ctx, ox, oy, s) => { slots.seen(ox, oy, s); if (live && godOn()) god.draw(ctx, ox, oy, s); };
map.tap = (x, y) => live !== null && godOn() && god.tap(x, y);
/** The GPU that draws the map (null: none yet, or the page cannot draw it). */
let drawing: HeldGpu | null = null;
map.onScale = () => { if (live) showStats(live.stats); };
map.ready.then(g => { drawing = g; if (!g) say('This browser has no WebGL2, so the map cannot be drawn.'); });
slots.listen($<HTMLCanvasElement>('cv'), () => inspect());
const inspector = new Inspector($('inspect-card'), $('inspect'), $('texture'), map, slots);
function inspect(): void { inspector.show(world, frame); }
const numbers = new Numbers($('stats'), $('cost'), () => { if (live) showStats(live.stats); });
/** The live world's numbers in the World panel. */
function showStats(s: Stats): void {
  const m = world?.meta;
  numbers.show(s, live && m ? { meta: m, playing, speed, land, drawing, drawnAt: map.drawnAt } : null);
}
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
showErrors($('errors'));

/** The live world or a tape: what only one of them has is shown, and the speeds are its own. */
function setMode(isLive: boolean): void {
  document.querySelectorAll<HTMLElement>('.live-only').forEach(el => (el.hidden = !isLive));
  document.querySelectorAll<HTMLElement>('.tape-only').forEach(el => (el.hidden = isLive));
  if (!isLive) setGod(false);
  speed = isLive ? LIVE_SPEEDS[1] : 4;
  $('speeds').innerHTML = (isLive ? LIVE_SPEEDS : SPEEDS).map(s => `<option value="${s}">${isLive ? (s === Infinity ? 'max' : `${s} mo/s`) : `×${s}`}</option>`).join('');
  live?.rate(speed);
}

function setup(w: World): void {
  const m = w.meta;
  say('');
  // A new live world plays at once, before the page is set up for it and draws it, so a failure there does not hold it;
  // a saved one opens paused at its date, to be looked at.
  playing = false; layer = null;
  if (live && !saved) togglePlay();
  inspector.newWorld(m.w * m.h);
  map.setWorld(m.w, m.h, m.cell_m, m.globe ? { north: m.lat_north, south: m.lat_south } : null);
  $('meta').innerHTML = [['seed', m.seed], ...(m.globe ? [['world', 'globe']] : []), ['size', `${m.w}×${m.h}`], ...(m.globe ? [] : [['band', `${fmt(m.lat_south)}° to ${fmt(m.lat_north)}°`]]), ['cell', m.cell_m >= 1000 ? `${fmt(m.cell_m / 1000)} km` : `${fmt(m.cell_m)} m`]]
    .map(([k, v]) => `<span>${k} <b>${v}</b></span>`).join('') + '<span>year <b id="hy"></b></span>';
  const names = m.layers.map(l => l.name).filter(n => !/^(true|ground)_[rgb]$/.test(n));
  const sorted = [...names.filter(n => LAYER_ORDER.includes(n)).sort((a, b) => LAYER_ORDER.indexOf(a) - LAYER_ORDER.indexOf(b)), ...names.filter(n => !LAYER_ORDER.includes(n))];
  $('layers').replaceChildren(...sorted.map(n => button(layerLabel(n), () => setLayer(layer === n ? null : n), n)));
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
  $('year').innerHTML = `<b>Year ${fmtYear(Math.floor(y))}</b><small>month ${Math.floor((y - Math.floor(y)) * MONTHS) + 1}</small>`;
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
  inspector.painted();
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
    $('rampunit').textContent = `${layerLabel(n)}${unitText(l.unit) ? ` (${unitText(l.unit)})` : ''}${l.scale === 'log' ? ', log scale' : ''}`;
  }
  syncUi(); repaint();
}

function syncUi(): void {
  document.querySelectorAll<HTMLElement>('#views button').forEach(b => b.classList.toggle('on', !layer && b.dataset.view === view));
  document.querySelectorAll<HTMLElement>('#layers button').forEach(b => b.classList.toggle('on', b.dataset.layer === layer));
  $<HTMLSelectElement>('speeds').value = String(speed);
  $('play').classList.toggle('playing', playing);
  $('play').setAttribute('aria-label', playing ? 'Pause' : 'Play');
  map.playing = playing && live !== null;
}

/** Whether god mode is on: its strip and card shown. */
function godOn(): boolean { return !$('god-strip').hidden; }
function setGod(on: boolean): void {
  $('god-strip').hidden = $('god').hidden = !on;
  $('god-btn').setAttribute('aria-pressed', String(on));
  if (!on) god.reset();
}

function setView(v: string): void { view = v; layer = null; setLayer(null); }

function button(text: string, fn: () => void, layerName?: string): HTMLButtonElement {
  const b = document.createElement('button');
  b.textContent = text; b.onclick = fn;
  if (layerName) b.dataset.layer = layerName;
  return b;
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
$('play').innerHTML = iconSvg('play', 'go') + iconSvg('pause', 'pause');
fillIcons(document);
$('play').onclick = togglePlay;
$<HTMLSelectElement>('speeds').onchange = e => { speed = +(e.target as HTMLSelectElement).value; live?.rate(speed); syncUi(); };
const panels = new Panels($('bar'), $('cv'));
watchCovered([$('bar'), $('god-strip'), $('god'), $('inspect-card'), $('timebar')], edges => map.setCovered(edges));
/** Inspect's card, shown while its button is pressed. */
$('inspect-btn').onclick = () => { const on = $('inspect-card').hidden; $('inspect-card').hidden = !on; $('inspect-btn').setAttribute('aria-pressed', String(on)); inspect(); };
$('god-btn').onclick = () => setGod(!godOn());
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
  else if (e.key === 'Escape' && !panels.close() && !god.putAway()) { setLayer(null); map.pick = -1; inspect(); map.invalidate(); }
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
  const wasm = await fetchBytes('ew.wasm', { cache: 'no-cache' });
  coreWasm = wasm.slice(0);
  const q = new URLSearchParams(location.search);
  fillForm(q);
  live?.stop();
  world = null;
  const l = live = new Live($('worker-src').textContent ?? '', wasm, {
    ready: () => (from ? openSaved(from) : newWorld()),
    world: w => { world = w; land = vegetation.land = landOf(w); map.setLand(land); thingsKey = ''; slots.set(null); god.reset(); setup(w); land?.tiles.view(map.view()); showStats(l.stats); },
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

/** Whether a banded world was made in this session: the first one makes the planet's air too, which takes a while. */
let banded = false;
function newWorld(): void {
  const { seed, size, band, drawn, variety, soft } = readForm();
  playing = false; saved = null;
  say(band[0] < band[1] && !banded ? 'Making the world, and once the air of the planet around it (a few seconds)…' : 'Making the world…');
  banded ||= band[0] < band[1];
  live?.newWorld(seed, size, band, drawn, variety, soft);
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
if (embedded) load(Uint8Array.from(atob(embedded), c => c.charCodeAt(0)).buffer);
else if (url) fetchBytes(url).then(load, waiting);
else if (savedUrl) fetchBytes(savedUrl).then(startLive).catch(e => say(String(e instanceof Error ? e.message : e)));
else startLive(null).catch(e => say(`${String(e instanceof Error ? e.message : e)}. Drop a .ewf.gz file here, or use “Open file”.`));
