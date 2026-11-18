// The picture tool: `npm run look -- <command>` (README.md).
import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { join, resolve, basename } from 'node:path';
import * as P from './page.mjs';
import { lab, numbers, same, sheet, MIDDLE } from './lab.mjs';
import { worlds, dates } from './worlds.mjs';
import { maskShaders } from './mask.mjs';
import { PASSES, costShaders, counts } from './cost.mjs';
import { kinds, marks, persist } from './read.mjs';

const SCENES = JSON.parse(readFileSync(new URL('./scenes.json', import.meta.url), 'utf8'));
/** The map's side by default, css pixels: small, as a picture is drawn in software here. */
const PX = 400;

const [cmd, ...rest] = process.argv.slice(2);
/** The words after the command: those not of a flag, and the flags' values (`--name value`). */
const flags = {}, words = [];
for (let i = 0; i < rest.length; i++) if (rest[i].startsWith('--')) flags[rest[i].slice(2)] = rest[++i]; else words.push(rest[i]);
const px = +(flags.px ?? PX);
const say = (...a) => console.log(...a);
const secs = t0 => `${((Date.now() - t0) / 1000).toFixed(0)} s`;

/** The scene set named by `--scenes` (the first of scenes.json by default), its world's spec and the scenes kept by
 * `--only A,B`. */
function setOf() {
  const name = flags.scenes ?? Object.keys(SCENES.sets)[0], set = SCENES.sets[name];
  if (!set) throw new Error(`no scene set ${name} (scenes.json: ${Object.keys(SCENES.sets).join(', ')})`);
  const only = flags.only?.split(',');
  return { name, set, world: SCENES.worlds[set.world], scenes: set.scenes.filter(s => !only || only.includes(s.name)) };
}

/** A browser on the tool's server with the page of the current build, and the builds `variants` ({ site: the JSON of
 * its programs }, `page.site`). */
async function start(variants = {}) {
  P.site('site');
  for (const [name, shaders] of Object.entries(variants)) P.site(name, shaders);
  const { server, url } = await P.serve(), browser = await P.launch();
  return { server, url, browser, lab: await lab(browser), stop: async () => { await browser.close(); server.close(); } };
}

/** The file name of a view's picture. */
const fileOf = (scene, v) => `${scene.name}_${v.mpp}_${v.style}.png`;

/** `set OUT`: every scene and view of the set on the current build: the pictures, a contact sheet, numbers.json. */
async function set(out) {
  const t0 = Date.now(), { name, set, world, scenes } = setOf(), crowns = flags.crowns !== 'no';
  const at = worlds(set.world, world, dates(world))[world.at], s = await start(crowns ? { 'site-mask': maskShaders() } : {});
  mkdirSync(out, { recursive: true });
  const page = await P.open(s.browser, s.url, 'site', at, { px }), maskPage = crowns ? await P.open(s.browser, s.url, 'site-mask', at, { px }) : null;
  const result = { set: name, world: at, year: await P.year(page), px, scenes: {} }, items = [];
  for (const scene of scenes) {
    const r = result.scenes[scene.name] = { about: scene.about, at: scene.at, kinds: await kinds(page, scene.at), views: {} };
    say(`${scene.name} (${scene.about}) at ${scene.at}: ${r.kinds}`);
    const shots = [];
    for (const v of scene.views ?? set.views) {
      const t = Date.now(), view = { x: scene.at[0], y: scene.at[1], mpp: v.mpp, style: v.style };
      await P.look(page, view);
      const { png, waited } = await P.shot(page, join(out, fileOf(scene, v)));
      let mask = null;
      if (maskPage) { await P.look(maskPage, view); mask = (await P.shot(maskPage, join(out, 'mask', fileOf(scene, v)))).png; }
      r.views[`${v.mpp}_${v.style}`] = { ...(await numbers(s.lab, png, { mask })), waited };
      shots.push({ v, png, mask });
      items.push({ label: `${scene.name} ${v.mpp} m/px ${v.style}`, png });
      say(`  ${v.mpp} ${v.style}: ${secs(t)} ${JSON.stringify(r.views[`${v.mpp}_${v.style}`])}`);
    }
    // Far against near on the same ground: the farther picture cut to the nearer one's ground.
    for (const [far, near] of shots.map((f, k) => [f, shots[k + 1]]).slice(0, -1)) {
      if (far.v.style !== near.v.style) continue;
      const keep = MIDDLE * near.v.mpp / far.v.mpp, a = await numbers(s.lab, far.png, { keep, mask: far.mask });
      const b = r.views[`${near.v.mpp}_${near.v.style}`];
      r.views[`${far.v.mpp}_on_${near.v.mpp}_${far.v.style}`] = { ...a, lightness_change: +(100 * (b.lightness - a.lightness) / a.lightness).toFixed(1), ...(a.crowns !== undefined ? { crowns_change: +(b.crowns - a.crowns).toFixed(1) } : {}) };
    }
  }
  result.errors = page.errors;
  writeFileSync(join(out, 'numbers.json'), JSON.stringify(result, null, 1));
  writeFileSync(join(out, 'sheet.png'), await sheet(s.lab, items, set.views.length * 2));
  await s.stop();
  say(`set ${name}: ${items.length} pictures in ${secs(t0)}, ${result.errors.length} page errors; ${join(out, 'sheet.png')}, ${join(out, 'numbers.json')}`);
}

/** The numbers files `compare` reads, of `set` and `cost`. */
const NUMBERS = ['numbers.json', 'cost.json'];

/** `compare OUT_A OUT_B`: the numbers side by side with their differences (of `set` and of `cost`, where both folders
 * hold them), and a sheet of each picture before beside after. */
async function compare(a, b) {
  const lines = [`${'view'.padEnd(26)} ${'number'.padEnd(17)} ${basename(a).padStart(10)} ${basename(b).padStart(10)} ${'diff'.padStart(8)}`];
  const items = [];
  for (const file of NUMBERS.filter(f => existsSync(join(a, f)) && existsSync(join(b, f)))) {
    const na = JSON.parse(readFileSync(join(a, file), 'utf8')), nb = JSON.parse(readFileSync(join(b, file), 'utf8'));
    for (const [scene, r] of Object.entries(na.scenes)) for (const [view, x] of Object.entries(r.views)) {
      const y = nb.scenes[scene]?.views[view];
      if (!y) continue;
      for (const [k, v] of Object.entries(x)) {
        if (k === 'waited') continue;
        const w = y[k], fmt = q => Array.isArray(q) ? q.join(' ') : String(q);
        const diff = Array.isArray(v) ? v.map((q, i) => +(w[i] - q).toFixed(2)).join(' ') : (w - v).toFixed(2);
        lines.push(`${`${scene} ${view}`.padEnd(26)} ${k.padEnd(17)} ${fmt(v).padStart(10)} ${fmt(w).padStart(10)} ${diff.padStart(8)}`);
      }
      const f = join(a, `${scene}_${view}.png`), g = join(b, `${scene}_${view}.png`);
      if (existsSync(f) && existsSync(g)) items.push({ label: `${scene} ${view}: ${basename(a)}`, png: readFileSync(f) }, { label: basename(b), png: readFileSync(g) });
    }
  }
  const table = lines.join('\n');
  say(table);
  writeFileSync(join(b, `compare_${basename(a)}.txt`), table + '\n');
  if (items.length) {
    const s = await start();
    writeFileSync(join(b, `compare_${basename(a)}.png`), await sheet(s.lab, items, 4));
    await s.stop();
    say(join(b, `compare_${basename(a)}.png`));
  }
}

/** `shot OUT.png --at X,Y --mpp M [--style angled|above] [--world FILE]`: one picture, of the set's world or a saved world. */
async function shot(out) {
  const t0 = Date.now(), { set, world } = setOf(), [x, y] = flags.at.split(',').map(Number);
  const s = await start();
  let at = worlds(set.world, world, dates(world))[world.at];
  if (flags.world) { mkdirSync(join(P.OUT, 'given'), { recursive: true }); writeFileSync(join(P.OUT, 'given', basename(flags.world)), readFileSync(flags.world)); at = `given/${basename(flags.world)}`; }
  const page = await P.open(s.browser, s.url, 'site', at, { px });
  await P.look(page, { x, y, mpp: +flags.mpp, style: flags.style ?? 'angled' });
  await P.shot(page, resolve(out));
  say(`${resolve(out)}: year ${(await P.year(page)).toFixed(2)}, ${secs(t0)}, ${page.errors.length} page errors`);
  await s.stop();
}

/** `sheet OUT.png COLS FILE...`: a contact sheet of pictures, each labelled with its name. */
async function sheetCmd(out, cols, ...files) {
  const s = await start();
  writeFileSync(out, await sheet(s.lab, files.map(f => ({ label: basename(f), png: readFileSync(f) })), +cols));
  await s.stop();
  say(resolve(out));
}

/** `years OUT`: the set's `years` scenes at its years' view, in each of the world's years at the same date, from saved
 * worlds a year apart; from each year to the next, the marks that stay, go and appear (`read.persist`), and the share
 * of the picture that stays (`lab.same`). */
async function years(out) {
  const t0 = Date.now(), { set, world } = setOf(), when = dates(world, true), saved = worlds(set.world, world, when);
  const scenes = set.scenes.filter(sc => set.years.includes(sc.name)), v = set.years_view;
  const s = await start();
  mkdirSync(out, { recursive: true });
  const page = await P.open(s.browser, s.url, 'site', saved[when[0]], { px });
  const seen = Object.fromEntries(scenes.map(sc => [sc.name, []])), items = [];
  for (const [i, d] of when.entries()) {
    if (i > 0) await P.reopen(page, `${s.url}/${saved[d]}`);
    for (const sc of scenes) {
      await P.look(page, { x: sc.at[0], y: sc.at[1], mpp: v.mpp, style: v.style });
      const { png } = await P.shot(page, join(out, `${sc.name}_${d}.png`));
      seen[sc.name].push({ png, marks: await marks(page) });
      items.push({ label: `${sc.name} year ${d}`, png });
    }
    say(`year ${d}: ${secs(t0)}`);
  }
  const result = {}, mean = (rs, key) => +(rs.reduce((a, r) => a + r[key], 0) / rs.length).toFixed(1);
  for (const sc of scenes) {
    const ys = seen[sc.name], pairs = [];
    for (let k = 1; k < ys.length; k++) pairs.push({ ...persist(ys[k - 1].marks, ys[k].marks), pixels: (await same(s.lab, ys[k - 1].png, ys[k].png)).stay });
    result[sc.name] = { by_year: pairs, ...Object.fromEntries(['stay', 'other', 'go', 'appear', 'pixels'].map(key => [key, mean(pairs, key)])) };
  }
  writeFileSync(join(out, 'years.json'), JSON.stringify({ view: v, years: when, scenes: result, errors: page.errors }, null, 1));
  writeFileSync(join(out, 'years.png'), await sheet(s.lab, items, scenes.length));
  for (const [n, r] of Object.entries(result)) say(`${n}: marks stay ${r.stay} %, other ${r.other} %, go ${r.go} %, appear ${r.appear} %; pixels stay ${r.pixels} %`);
  await s.stop();
  say(`years: ${when.length} years in ${secs(t0)}, ${page.errors.length} page errors; ${join(out, 'years.png')}, ${join(out, 'years.json')}`);
}

/** A count picture is shown with each colour's level times these: red full at 510 texture reads, green at 64 noises,
 * blue at 32 photographs. */
const SHOW = [1, 4, 8];

/** `cost OUT`: each scene of the set drawn once at each of its cost views by the two counted builds (cost.mjs), the
 * ground's and the marks': per scene and build the mean, the 95th percentile and the most of each count a pixel
 * (texture reads, noise, photographs) and the % of pixels the colour cannot hold (`cost.json`); the count pictures
 * (each colour times `SHOW`), their sheet, and the counts as drawn in `OUT/raw/`. */
async function cost(out) {
  const t0 = Date.now(), { name, set, world, scenes } = setOf(), passes = Object.keys(PASSES);
  const at = worlds(set.world, world, dates(world))[world.at];
  const s = await start(Object.fromEntries(passes.map(pass => [`site-cost-${pass}`, costShaders(pass)])));
  mkdirSync(join(out, 'raw'), { recursive: true });
  const pages = {};
  for (const pass of passes) pages[pass] = await P.open(s.browser, s.url, `site-cost-${pass}`, at, { px });
  const result = { set: name, world: at, px, show: SHOW, scenes: {} }, items = [];
  for (const scene of scenes) {
    const r = result.scenes[scene.name] = { about: scene.about, at: scene.at, views: {} };
    for (const v of set.cost_views) for (const pass of passes) {
      const view = `${v.mpp}_${v.style}_${pass}`, file = `${scene.name}_${view}.png`;
      await P.look(pages[pass], { x: scene.at[0], y: scene.at[1], mpp: v.mpp, style: v.style });
      const { png } = await P.shot(pages[pass], join(out, 'raw', file)), c = await counts(s.lab, png, MIDDLE, SHOW);
      writeFileSync(join(out, file), c.seen);
      r.views[view] = Object.fromEntries(Object.entries(c.numbers).map(([k, q]) => [k, [q.mean, q.p95, q.most, q.clipped]]));
      items.push({ label: `${scene.name} ${v.mpp} m/px ${pass}`, png: c.seen });
      say(`${scene.name} ${view}: ${Object.entries(c.numbers).map(([k, q]) => `${k} ${q.mean} (p95 ${q.p95})`).join(', ')}`);
    }
  }
  result.errors = passes.flatMap(pass => pages[pass].errors);
  writeFileSync(join(out, 'cost.json'), JSON.stringify(result, null, 1));
  writeFileSync(join(out, 'sheet.png'), await sheet(s.lab, items, passes.length * set.cost_views.length));
  await s.stop();
  say(`cost ${name}: ${items.length} pictures in ${secs(t0)}, ${result.errors.length} page errors; ${join(out, 'sheet.png')}, ${join(out, 'cost.json')}`);
}

const COMMANDS = { set, compare, shot, sheet: sheetCmd, years, cost };
if (!COMMANDS[cmd]) { say('usage: npm run look -- set|compare|shot|sheet|years|cost ... (web/look/README.md)'); process.exit(2); }
// (npm runs the tool in web/: paths are read from where the command was typed, `INIT_CWD`.)
const here = process.env.INIT_CWD ?? process.cwd();
await COMMANDS[cmd](...words.map((w, i) => (cmd === 'sheet' && i === 1 ? w : resolve(here, w))));
