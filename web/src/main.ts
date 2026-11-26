import { World, gunzip, parse, fmt, fmtYear, unitText, fetchBytes } from './data';
import { CLASSES, GROUND_LAYERS, mapFields, rampFor, rampCss, wetOf } from './earth';
import { MapView, RING_AS_VIEW_SCALE, View } from './map';
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
import { FireFront } from './front';
import { nearestFrame, setTimeline } from './timeline';
import { godTools } from './god-tools';
import { Panels, watchCovered } from './panels';
import { fillIcons, iconSvg } from './icons';
import { textureLegend } from './soil';
import { Inspector, LAYER_GROUPS, layerLabel } from './inspect';
import { Numbers } from './numbers';
import { fillForm, readForm, listenForm } from './world-form';
import { showErrors } from './errors';
import { Boot, BootStep, plantShare } from './boot';

const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;
/** The layers of the soil's grain, shown with the texture classes' legend (soil.ts). */
const GRAIN_LAYERS = ['coarse', 'fine'];
const SPEEDS = [1, 2, 4, 8, 16, 32];
/** The live world's speeds, months of the world per second of real time. */
const LIVE_SPEEDS = [3, 12, 36, 120, Infinity];
/** Months in a year, for the date on the bar. */
const MONTHS = 12;
/** A phone's screen (as style.css): the map is drawn out to the screen's edges and a fit frames the land. */
const PHONE = matchMedia('(max-width: 640px)');
/** Time the view rests before the worker is asked for the things of the window in view, ms. */
const ASK_MS = 150;
/** The steps of the loading screen (boot.ts): their words and their shares of its bar. */
const BOOT_STEPS: Record<string, BootStep> = {
  file: { label: 'Download the world', weight: 15 }, core: { label: 'Download the core', weight: 15 },
  start: { label: 'Start the simulation', weight: 5 }, land: { label: 'Shape the land', weight: 20 },
  open: { label: 'Open the saved world', weight: 15 }, read: { label: 'Read the world', weight: 10 },
  ground: { label: 'Build the ground', weight: 20 }, grow: { label: 'Grow the first plants', weight: 25 },
  picture: { label: 'Draw the first picture', weight: 10 },
};
/** The time the steps that say nothing of their progress are expected to take, ms: the core downloaded where its
 * length is not known, the core started, a world made (and the first time, the air of the planet around it), a saved
 * world opened, the ground's workers ready, and the map's picture drawn whole. */
const BOOT_MS = { core: 3000, start: 1500, land: 2000, air: 5000, open: 3000, rock: 8000, picture: 2000 };
/** Of the step that builds the ground, the share its workers take to be ready (the rest: the tiles in view). */
const ROCK_SHARE = 0.3;
/** The loading screen lifts once the picture stays whole this long, ms; or this long after the map began to draw it,
 * whatever is left (a card too slow to draw it whole at the screen's pixels never keeps the page hidden). */
const SETTLED_MS = 400, PICTURE_MOST_MS = 20000;
/** A new world runs at full speed behind the loading screen till the plants show on this share of its dry squares
 * (boot.ts `plantShare`: a green season), or for this many years at most (a world too dry or too cold for them). */
const PLANTS_SHOW = 0.45, GROW_MOST_YEARS = 20;
/** Its speed meanwhile, months of the world per second: fast, yet slow enough that it pauses within a month or two of
 * that date (at full speed it steps years between two frames). */
const GROW_RATE = 60;
/** The plants on the loading screen's slab are drawn again at most this often, ms. */
const BOOT_PLANTS_MS = 500;

let world: World | null = null;
let live: Live | null = null;
let frame = 0, speed = 4, playing = false, layer: string | null = null;

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
/** Fire on the live map: its scars, flames and smoke. */
const fire = new Fire();
map.fire = fire;
// (While a burn of god mode plays the plants stand as they were till its front has passed: they are let go once it
// played out, `repaint`; the pictures about the view show it as it was.)
fire.played = begun => { if (!begun) repaint(); map.forgetAround(); };
/** The cover of the live map (with no layer shown), its plants scorched on fresh scars. */
const vegetation = new Vegetation(fire);
map.layer = vegetation;
/** God mode: the tools that edit the live world (live world only), shown while the god button is pressed. */
const front = new FireFront(fire);
const god = new God({
  strip: $('god-strip'), card: $('god'), redraw: () => map.redraw(), squareM: () => world?.meta.cell_m ?? 1,
  picked: () => { if (playing) togglePlay(); }, close: () => setGod(false), burnt: (b, strike, lit) => front.burnt(b, strike, lit),
  aim: () => (PHONE.matches && world ? map.sightAt() : null),
  ground: () => [live?.version ?? 0, land?.told ?? 0],
}, () => live, godTools(() => live?.kinds ?? []));
god.listen($<HTMLCanvasElement>('cv'));
map.overlay = (ctx, ox, oy, s) => {
  slots.seen(ox, oy, s);
  if (live && godOn()) god.draw(ctx, ox, oy, s);
};
map.paint = { on: () => godOn() && god.toolOn(), direct: () => !god.aiming(), start: (x, y) => live !== null && godOn() && !god.aiming() && god.start(x, y), move: (x, y) => god.move(x, y), end: () => god.end() };
/** The GPU that draws the map (null: none yet, or the page cannot draw it). */
let drawing: HeldGpu | null = null;
map.onScale = () => { if (live) showStats(live.stats); };
map.ready.then(g => { drawing = g; if (!g) fail('This browser has no WebGL2, so the map cannot be drawn.'); });
slots.listen($<HTMLCanvasElement>('cv'), () => inspect());
const inspector = new Inspector($('inspect-card'), $('inspect'), $('texture'), map, slots);
function inspect(): void { inspector.show(world, frame); }
const numbers = new Numbers($('stats'), $('cost'), () => { if (live) showStats(live.stats); });
/** The live world's numbers in the World panel. */
function showStats(s: Stats): void {
  const m = world?.meta;
  numbers.show(s, live && m ? { meta: m, playing, speed, land, drawing, drawnAt: map.drawnAt } : null);
}
const charts = new Charts($('charts'), year => world && setFrame(nearestFrame(world.meta, year)));
/** The loading screen, over the page till the map first stands whole, and when the map began to draw the picture and
 * since when it is whole, ms (NaN: not yet). */
const boot = new Boot($('boot'), watchBoot);
let pictureFrom = NaN, settledFrom = NaN;
/** Whether the new world runs behind the loading screen till its first plants show, the share of its dry squares they
 * show on (`plantShare`), and when the slab last drew them, ms. */
let growing = false, grown = 0, grownAt = -Infinity;
/** Plan the loading screen's steps. */
const bootPlan = (...keys: string[]) => boot.plan(keys.map(k => [k, BOOT_STEPS[k]]));
/** Tell step `key` of the loading screen the bytes of a download come. */
const bootBytes = (key: string) => (bytes: number, all: number) => {
  const mb = (b: number) => (b / 1e6).toFixed(1);
  if (all > 0) boot.at(key, bytes / all, `${mb(bytes)} of ${mb(all)} MB`);
  else boot.wait(key, BOOT_MS.core, `${mb(bytes)} MB`);
};
/** A frame of the new world growing behind the loading screen: it pauses as soon as its plants show (its ground may
 * still be being made). */
function growFrame(w: World): void {
  grown = plantShare(w);
  if (grown >= PLANTS_SHOW || w.meta.frames[0].year >= GROW_MOST_YEARS) { growing = false; togglePlay(); }
}
/** The loading screen's steps once the world came, read at each of its frames: the ground's tiles in view made; a new
 * world's first plants grown (then it pauses, and its ground is made again of that date); then the picture drawn with
 * all its programs, of the ground of that date, whole for `SETTLED_MS`. */
function watchBoot(): void {
  if (!world || !drawing) return;
  const now = performance.now(), tiles = land?.tiles;
  const year = world.meta.frames[0].year;
  if (now - grownAt > BOOT_PLANTS_MS) { grownAt = now; boot.plantsGrow(world); }
  if (tiles && !tiles.rock) return boot.wait('ground', BOOT_MS.rock, '', ROCK_SHARE);
  if (tiles && tiles.toCome() > 0) {
    const n = tiles.inViewCount(), made = n - tiles.toCome();
    return boot.at('ground', ROCK_SHARE + ((1 - ROCK_SHARE) * made) / n, `${made} of ${n} tiles`);
  }
  if (boot.has('grow')) boot.at('grow', growing ? Math.max(grown / PLANTS_SHOW, year / GROW_MOST_YEARS) : 1, `year ${Math.floor(year)}`);
  if (growing) return;
  if (Number.isNaN(pictureFrom)) pictureFrom = now;
  if (now - pictureFrom > PICTURE_MOST_MS) return boot.end();
  if (!map.programsMade() || tiles?.shownVersion() === null || !map.settled()) { settledFrom = NaN; return boot.wait('picture', BOOT_MS.picture); }
  if (Number.isNaN(settledFrom)) settledFrom = now;
  if (now - settledFrom >= SETTLED_MS) boot.end();
}

// ---- loading
async function load(buf: ArrayBuffer): Promise<void> {
  let next: World;
  try {
    next = parse(await gunzip(buf));
  } catch (e) { return fail(String(e instanceof Error ? e.message : e)); }
  boot.at('read', 1);
  live?.stop(); live = null;
  land = vegetation.land = null; map.setLand(null);
  world = next;
  setMode(false);
  setup(world);
}
const say = (t: string) => { $('msg').textContent = t; $('msg').hidden = !t; };
/** Say what went wrong, with the loading screen lifted at once so it is read. */
const fail = (t: string) => { say(t); boot.end(true); };
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
  // A new live world plays at once: behind the loading screen at `GROW_RATE` till its first plants show (`watchBoot`),
  // then at the speed shown once the screen lifts (`PICTURE_MOST_MS` at most after, and at once on a failure, `fail`).
  // A saved one opens paused at its date, to be looked at.
  playing = false; layer = null;
  if (live && !saved && boot.up) {
    const l = live;
    growing = true;
    l.rate(GROW_RATE);
    boot.lifted.then(() => { if (live !== l) return; growing = false; l.rate(speed); if (!playing) togglePlay(); });
  }
  if (live && !saved) togglePlay();
  if (boot.up) boot.world(w);
  inspector.newWorld(m.w * m.h);
  map.setWorld(m.w, m.h, m.cell_m, m.globe ? { north: m.lat_north, south: m.lat_south } : null);
  $('meta').innerHTML = [['seed', m.seed], ...(m.globe ? [['world', 'globe']] : []), ['size', `${m.w}×${m.h}`], ...(m.globe ? [] : [['band', `${fmt(m.lat_south)}° to ${fmt(m.lat_north)}°`]]), ['cell', m.cell_m >= 1000 ? `${fmt(m.cell_m / 1000)} km` : `${fmt(m.cell_m)} m`]]
    .map(([k, v]) => `<span>${k} <b>${v}</b></span>`).join('') + '<span>year <b id="hy"></b></span>';
  const names = m.layers.map(l => l.name).filter(n => !/^ground_[rgb]$/.test(n));
  const grouped = LAYER_GROUPS.map(([g, labels]): [string, string[]] => [g, Object.keys(labels).filter(n => names.includes(n))]);
  const others = names.filter(n => !grouped.some(([, ns]) => ns.includes(n)));
  $('layers').replaceChildren(...[...grouped, ['Other', others] as [string, string[]]].filter(([, ns]) => ns.length).map(([g, ns]) => {
    const group = document.createElement('div'), chips = document.createElement('div');
    group.className = 'layergroup'; chips.className = 'chips';
    group.innerHTML = `<h3>${g}</h3>`;
    chips.replaceChildren(...ns.map(n => button(layerLabel(n), () => setLayer(layer === n ? null : n), n)));
    group.append(chips);
    return group;
  }));
  findLayers();
  map.setFill(PHONE.matches, landBox(w));
  const years = m.frames.map(f => f.year);
  if (!live) charts.set(m.series, m.series_units, years);
  setTimeline(m, setFrame);
  setFrame(0); setLayer(null); syncUi();
}

/** The squares that hold land in the first frame of world `w` (x0, y0, x1, y1), what a fit frames on a phone; null:
 * none. */
function landBox(w: World): [number, number, number, number] | null {
  const { sea } = wetOf(w, 0), W = w.meta.w;
  let x0 = W, y0 = w.meta.h, x1 = 0, y1 = 0;
  sea.forEach((s, i) => {
    if (s) return;
    const x = i % W, y = (i / W) | 0;
    x0 = Math.min(x0, x); y0 = Math.min(y0, y); x1 = Math.max(x1, x + 1); y1 = Math.max(y1, y + 1);
  });
  return x1 > x0 ? [x0, y0, x1, y1] : null;
}

/** Show the layers whose names hold the text searched for, and the groups that hold one of them. */
function findLayers(): void {
  const q = $<HTMLInputElement>('layerfind').value.trim().toLowerCase();
  document.querySelectorAll<HTMLElement>('#layers .layergroup').forEach(g => {
    let any = false;
    g.querySelectorAll<HTMLElement>('button').forEach(b => { const on = b.textContent!.toLowerCase().includes(q); b.hidden = !on; any ||= on; });
    g.hidden = !any;
  });
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
  if (layer || !live || !fire.playing()) vegetation.set(!layer && live ? w : null, live?.table ?? null);
  fire.set(land && live ? w : null);
  map.setLook(look);
  inspector.painted();
  inspect();
}


/** What the map draws: the Earth view, or a layer over it. */
function mapLook(): MapLook {
  return { mode: layer ? 'layer' : 'earth', layer: layer ?? '', ramp: layer ? rampFor(layer) : [], ground: GROUND_LAYERS.get(layer ?? '') ?? 0 };
}

function setLayer(n: string | null): void {
  layer = n;
  const r = $('ramp'); r.hidden = !n;
  $('texture').hidden = !GRAIN_LAYERS.includes(n ?? '');
  $('texture').innerHTML = GRAIN_LAYERS.includes(n ?? '') ? textureLegend(null) : '';
  if (n && world) {
    const l = world.meta.layers[world.index.get(n)!], lut = world.lut[world.index.get(n)!];
    $('rampname').textContent = layerLabel(n);
    $('rampbar').style.background = rampCss(rampFor(n));
    [0, 128, 255].forEach((b, k) => ($('ramp').querySelectorAll('.lbl span')[k].textContent = fmt(lut[b])));
    $('rampunit').textContent = [unitText(l.unit), l.scale === 'log' ? 'log scale' : ''].filter(t => t).join(', ');
  }
  syncUi(); repaint();
}

function syncUi(): void {
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
  // (On a phone the strip takes the dock's place, style.css.)
  document.body.classList.toggle('god-on', on);
  $('god-btn').setAttribute('aria-pressed', String(on));
  if (!on) god.reset();
}

function button(text: string, fn: () => void, layerName?: string): HTMLButtonElement {
  const b = document.createElement('button');
  b.textContent = text; b.onclick = fn;
  if (layerName) b.dataset.layer = layerName;
  return b;
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
  if (fire.step(now)) map.invalidate();
  requestAnimationFrame(loop);
}
function togglePlay(): void {
  if (!world || god.busy()) return; // (a stroke of god mode settles first)
  // (A paused world's ground is made again at once from its last build, ground.ts `rebuild`.)
  if (live) {
    playing = !playing; live.play(playing);
    // (The core forgets the last stroke as the world plays on.)
    if (playing) god.forgetUndo(); else land?.tiles.rebuild(null, true);
    return syncUi();
  }
  if (!playing && frame >= world.meta.frames.length - 1) setFrame(0);
  playing = !playing; acc = 0; syncUi();
}

// ---- wiring
$('play').innerHTML = iconSvg('play', 'go') + iconSvg('pause', 'pause');
fillIcons(document);
$('play').onclick = togglePlay;
$<HTMLSelectElement>('speeds').onchange = e => { speed = +(e.target as HTMLSelectElement).value; live?.rate(speed); syncUi(); };
/** A test setting: the share of the screen's pixels along each axis the near ring is drawn at (map.ts `setRingScale`),
 * kept in this browser. */
const RING_SCALES = [1, 0.5, 0.35, 0.25, 0.125], RING_SCALE_KEY = 'ew.ringScale';
{
  const pick = $<HTMLSelectElement>('ringscale');
  pick.innerHTML = RING_SCALES.map(r => `<option value="${r}">${r} a side: 1/${Math.round(1 / (r * r))} of the pixels</option>`).join('');
  let kept = NaN;
  try { kept = Number(localStorage.getItem(RING_SCALE_KEY)); } catch { /* storage blocked: the default */ }
  const r = RING_SCALES.includes(kept) ? kept : RING_AS_VIEW_SCALE;
  pick.value = String(r);
  map.setRingScale(r);
  pick.onchange = () => {
    const v = +pick.value;
    try { localStorage.setItem(RING_SCALE_KEY, String(v)); } catch { /* storage blocked: this visit only */ }
    map.setRingScale(v);
  };
}
const panels = new Panels($('bar'), $('cv'));
watchCovered([$('bar'), $('year'), $('god-strip'), $('god'), $('inspect-card'), $('timebar')], edges => map.setCovered(edges));
PHONE.addEventListener('change', () => map.setFill(PHONE.matches, world && landBox(world)));
$<HTMLInputElement>('layerfind').oninput = findLayers;
$('clearlayer').onclick = () => setLayer(null);
/** Inspect's card, shown while its button is pressed. */
$('inspect-btn').onclick = () => { const on = $('inspect-card').hidden; $('inspect-card').hidden = !on; $('inspect-btn').setAttribute('aria-pressed', String(on)); inspect(); };
$('god-btn').onclick = () => setGod(!godOn());
$('fit').onclick = () => map.fit();
$<HTMLInputElement>('chartfind').oninput = e => charts.filter((e.target as HTMLInputElement).value);
$<HTMLInputElement>('tl').oninput = e => setFrame(+(e.target as HTMLInputElement).value);
addEventListener('keydown', e => {
  if (boot.up) return; // (the page under the loading screen is not yet to be used)
  const tag = (e.target as HTMLElement).tagName;
  if ((e.target as HTMLInputElement).type === 'search') return; // typing in a search (charts, layers)
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'z' && tag !== 'INPUT') { e.preventDefault(); god.undo(); }
  else if (e.key === ' ' && tag !== 'BUTTON' && tag !== 'INPUT') { e.preventDefault(); togglePlay(); }
  else if (e.key === 'ArrowLeft' && tag !== 'INPUT') setFrame(frame - (e.shiftKey ? 10 : 1));
  else if (e.key === 'ArrowRight' && tag !== 'INPUT') setFrame(frame + (e.shiftKey ? 10 : 1));
  else if (e.key === 'Escape' && !panels.close() && !god.putAway()) { setLayer(null); map.pick = -1; inspect(); map.invalidate(); }
  else if (e.key === 'f') map.fit();
});
$('classes').innerHTML = CLASSES.map(([n, c]) => `<li><s style="background:${c}"></s>${n}</li>`).join('');

// ---- start

/** The live world: the core's WebAssembly (ew.wasm, beside the page) in a worker; the saved world `from`, or a new one
 * of the seed, size and variety from the page. */
async function startLive(from: ArrayBuffer | null): Promise<void> {
  // (Asked of the server each time, never from the browser's cache: a page of one build on the core of another reads
  // layers the core does not make, and draws nothing.)
  const wasm = await fetchBytes('ew.wasm', { cache: 'no-cache' }, bootBytes('core'));
  coreWasm = wasm.slice(0);
  const q = new URLSearchParams(location.search);
  fillForm(q);
  live?.stop();
  world = null;
  const l = live = new Live($('worker-src').textContent ?? '', wasm, {
    ready: () => (from ? openSaved(from) : newWorld()),
    world: w => { world = w; land = vegetation.land = landOf(w); map.setLand(land); thingsKey = ''; slots.set(null); god.reset(); god.forgetUndo(); setup(w); land?.tiles.view(map.view()); showStats(l.stats); },
    things: data => { if (live === l && world) slots.set(data); },
    frame: (stats, changed, edited) => {
      if (live !== l) return;
      if (changed && !edited) fire.still();
      if (changed) setFrame(0);
      if (changed && growing && world) growFrame(world);
      if (edited) map.showNow();
      if (world) front.frame(world);
      groundBusy();
      showStats(stats);
    },
    brushed: r => { if (live === l) god.brushed(r); },
    settled: r => { if (live === l) god.settled(r); },
    undone: () => { if (live === l) fire.still(); },
    preview: p => { if (live === l && land) { land.preview(p); map.showNow(); } },
    relief: () => { if (live === l) land?.tiles.rebuild(l.takeRelief(), true); },
    error: text => { playing = false; syncUi(); fail(text); },
  });
  boot.wait('start', BOOT_MS.start);
  // (While its first plants grow behind the loading screen, the world does not wait for the map to draw its frames.)
  l.drawn = () => (growing ? Promise.resolve() : map.shown());
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
  const tiles = new GroundTiles($('tiles-src').textContent ?? '', coreWasm.slice(0), ground, { w: m.w, h: m.h, cell_m: m.cell_m, square_depth: m.square_depth }, m.tiles, live?.version ?? 0, shown => {
    if (shown) map.invalidate();
    land?.tell(live);
    groundBusy();
    if (live) showStats(live.stats);
  }, fail);
  return new Land(tiles, m.laws, m.samples, live?.version ?? 0);
}
/** While the world stands still, its ground in view is being made again from samples built anew. */
function groundBusy(): void {
  $('busy').hidden = !(live && land && !playing && live.version !== land.told);
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
  const air = band[0] < band[1] && !banded;
  say(air ? 'Making the world, and once the air of the planet around it (a few seconds)…' : 'Making the world…');
  boot.wait('land', air ? BOOT_MS.air : BOOT_MS.land, air ? 'and the air of the planet' : '');
  banded ||= band[0] < band[1];
  live?.newWorld(seed, size, band, drawn, variety, soft);
}
/** Stand the live world on the saved world `gz` (the gzip of a snapshot of `ew run`). */
function openSaved(gz: ArrayBuffer): void {
  playing = false; saved = gz;
  say('Opening the saved world…');
  boot.wait('open', BOOT_MS.open);
  live?.open(gz.slice(0));
}
listenForm(newWorld, openSaved, load);

requestAnimationFrame(loop);
const embedded = document.getElementById('world-data')?.textContent?.trim();
// A tape (`?f=URL`), or the live world: on a saved world (`?world=URL`, a snapshot of `ew run`) or a new one.
const asked = new URLSearchParams(location.search), url = asked.get('f'), savedUrl = asked.get('world');
const waiting = () => fail('Drop a .ewf.gz file here, or use “Open file”.');
if (embedded) { bootPlan('read', 'picture'); load(Uint8Array.from(atob(embedded), c => c.charCodeAt(0)).buffer); }
else if (url) { bootPlan('file', 'read', 'picture'); fetchBytes(url, undefined, bootBytes('file')).then(load, waiting); }
else if (savedUrl) {
  bootPlan('file', 'core', 'start', 'open', 'ground', 'picture');
  fetchBytes(savedUrl, undefined, bootBytes('file')).then(startLive).catch(e => fail(String(e instanceof Error ? e.message : e)));
} else {
  bootPlan('core', 'start', 'land', 'ground', 'grow', 'picture');
  startLive(null).catch(e => fail(`${String(e instanceof Error ? e.message : e)}. Drop a .ewf.gz file here, or use “Open file”.`));
}
