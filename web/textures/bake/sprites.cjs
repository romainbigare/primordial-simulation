// node sprites.cjs sprites.json outdir [model...]: renders the sprites that sprites.json names (sprite.html) and writes
// the strips, as web/textures/ holds them: outdir/models.png and outdir/trees.png, each split into more files of whole
// layers (models2.png, trees2.png, ...) where it would be over `STRIP_BYTES`, outdir/strips.json naming them in their
// layers' order (the page loads them so, detail.ts), and outdir/sprites.wgsl (the table the page reads:
// web/shaders/lib/sprites.wgsl); with models named, each of their sprites alone to outdir/<model>_<k>.png (and its
// bare twin to outdir/<model>_<k>_bare.png, its heights to outdir/<model>_<k>_height.png), and their table's rows in
// order to outdir/rows.json. Whole, and with `--heights` (the sprites with heights alone), it writes the heights:
// outdir/heights.png (more files of whole layers as the strips), named under `heights` in outdir/strips.json (with what
// that file already names), and outdir/heights.wgsl (web/shaders/lib/heights.wgsl).
//
// sprites.json: for each strip (`trees`, `models`) the side of its layers, its cells a layer (`per`: 1 or 4) and its
// sprites, each entry a model (Poly Haven's, or with `file`, a glTF or FBX file under the assets folder) with: `parts`,
// lists of its nodes, each part a sprite (default: the whole model); `turns`, degrees about the vertical, each a sprite
// of each part; `cut`, the share of its height cut off at its foot; `clump`, { n, r }: each part stood n times about
// one foot, turned, shrunk and set off by up to r of its height (sprite.html `clumped`: reeds from one stem); `rest`,
// set down on its resting face (sprite.html `restingTurn`) and `sink`, the share of its height sunk into the ground, cut
// off at the ground; `bare`, the materials (a pattern) hidden for its bare twin (the trees' twins go into
// the models' strip after its own sprites; sprites.wgsl `bare_of`); `leaf`, the mean lightness (linear) its leaves are
// scaled to (its other texels by the square root of that scale), so the models' leaves are about as light as each
// other's; `set`, the name of the set it starts (sprites.wgsl `<set>_FROM` and `<set>_N`, the set running to the next
// entry with a set); `height`, its sprites stand in water and get their heights: each texel's height over the ground as
// a share of its sprite's height (sprite.html `sprite`), in a strip of their own, `HEIGHT_PER` to a layer.
const { chromium } = require('playwright');
const fs = require('fs');
const specs = JSON.parse(fs.readFileSync(process.argv[2])), OUT = process.argv[3];
const heightsOnly = process.argv[4] === '--heights', only = heightsOnly ? [] : process.argv.slice(4);
// The heights' strip: its layers' side and its cells a layer (half a tree's side: a texel's height is read only against
// the water's surface).
const HEIGHT_SIDE = 512, HEIGHT_PER = 4;
// A strip's file holds at most this many bytes, so it stays well under what the owner's page host takes (15 MB).
const STRIP_BYTES = 12e6;
// The sprites of an entry, part by part and turn by turn.
const expand = e => (e.parts || [null]).flatMap(nodes => (e.turns || [0]).map(turn => ({ ...e, nodes, turn })));
// A table as one array of the program's (made once, not again at each call: a copy in each function that reads it is
// more than a software renderer compiles) and its reader.
const wgslTable = (name, table, type, values) => `var<private> ${table}: array<${type}, ${values.length}> = array<${type}, ${values.length}>(${values.join(', ')});\nfn ${name}(s: i32) -> ${type} { return ${table}[s]; }\n`;
const wgslArray = (name, rows, doc, digits = 3) => `${doc ? doc + '\n' : ''}${wgslTable(name, `${name.toUpperCase()}_TABLE`, 'f32', rows.map(v => v.toFixed(digits)))}`;

(async () => {
  const b = await chromium.launch({ args: ['--use-gl=swiftshader', '--enable-unsafe-swiftshader'] });
  const p = await b.newPage({ viewport: { width: 1100, height: 1100 } });
  p.on('console', m => { if (m.type() === 'error') console.log('ERR', m.text()); });
  p.on('pageerror', e => console.log('PAGEERR', e.message));
  await p.goto('http://localhost:8835/render/sprite.html');
  await p.waitForFunction(() => window.ready === true);
  const save = (name, url) => fs.writeFileSync(`${OUT}/${name}`, Buffer.from(url.split(',')[1], 'base64'));
  const strips = {}, heights = [];
  // Each sprite's number in the page's table (the trees' first), counted whether it is rendered or not.
  let number = 0;
  for (const [strip, spec] of Object.entries(specs)) {
    const cell = spec.per === 4 ? spec.side / 2 : spec.side, done = [];
    let set = null;
    for (const e of spec.sprites) {
      if (e.set) set = { name: e.set, from: done.length, n: 0 };
      const skip = (only.length && !only.includes(e.model)) || (heightsOnly && !e.height);
      for (const [k, s] of expand(e).entries()) {
        const own = number++;
        if (skip) continue;
        const t = Date.now(), i = await p.evaluate(([s, c, h]) => window.sprite(s, c, h), [s, cell, e.height ? HEIGHT_SIDE / 2 : 0]);
        done.push({ i, s, set });
        if (e.height) heights.push({ i, own });
        if (set) set.n++;
        console.log('rendered', s.model, s.nodes || '', s.turn, 'leaves', (await p.evaluate(i => window.rowOf(i, 'leafy'), i)).leaf.toFixed(4), (Date.now() - t) / 1000, 's');
        if (only.length) {
          save(`${s.model}_${k}.png`, await p.evaluate(i => window.cellOf(i, 'leafy'), i));
          if (s.bare) save(`${s.model}_${k}_bare.png`, await p.evaluate(i => window.cellOf(i, 'bare'), i));
          if (e.height) save(`${s.model}_${k}_height.png`, await p.evaluate(i => window.cellOf(i, 'height'), i));
        }
      }
    }
    strips[strip] = { spec, cell, done };
  }
  if (only.length) {
    const rows = [];
    for (const st of Object.values(strips)) for (const d of st.done) rows.push({ model: d.s.model, set: d.set?.name, ...(await p.evaluate(i => window.rowOf(i, 'leafy'), d.i)) });
    fs.writeFileSync(`${OUT}/rows.json`, JSON.stringify(rows, null, 1));
    await b.close(); return;
  }
  // The trees' bare twins after the models' own sprites.
  const bares = strips.trees.done.filter(d => d.s.bare);
  const rows = [], sets = new Map();
  for (const d of [...strips.trees.done, ...strips.models.done]) rows.push(await p.evaluate(i => window.rowOf(i, 'leafy'), d.i));
  const bareFrom = rows.length;
  for (const d of bares) rows.push(await p.evaluate(i => window.rowOf(i, 'bare'), d.i));
  const bareOf = strips.trees.done.map(d => (d.s.bare ? bareFrom + bares.indexOf(d) : -1));
  const trees = strips.trees.done.length;
  strips.trees.done.forEach(d => d.set && sets.set(d.set.name, [d.set.from, d.set.n]));
  strips.models.done.forEach(d => d.set && sets.set(d.set.name, [trees + d.set.from, d.set.n]));
  const models = [...strips.models.done.map(d => [d.i, 'leafy']), ...bares.map(d => [d.i, 'bare'])];
  // A strip in as few files of at most `STRIP_BYTES` as it fits, each of whole layers: `<name>.png`, `<name>2.png`, ...
  const files = async (name, list, side, per) => {
    const layers = Math.ceil(list.length / per);
    for (let parts = 1; parts <= layers; parts++) {
      const each = Math.ceil(layers / parts) * per, pngs = [];
      for (let at = 0; at < list.length; at += each) {
        pngs.push(Buffer.from((await p.evaluate(([l, side, per]) => window.strip(l, side, per), [list.slice(at, at + each), side, per])).split(',')[1], 'base64'));
      }
      if (pngs.every(b => b.length <= STRIP_BYTES)) {
        const names = pngs.map((b, k) => `${name}${k ? k + 1 : ''}.png`);
        names.forEach((n, k) => fs.writeFileSync(`${OUT}/${n}`, pngs[k]));
        return names;
      }
    }
    throw new Error(`${name}: a layer's file is over ${STRIP_BYTES} bytes`);
  };
  // The heights: their strip, named in strips.json, and their table (the runs of sprites with heights, so the page holds
  // no row for each sprite).
  const named = fs.existsSync(`${OUT}/strips.json`) ? JSON.parse(fs.readFileSync(`${OUT}/strips.json`)) : {};
  named.heights = await files('heights', heights.map(h => [h.i, 'height']), HEIGHT_SIDE, HEIGHT_PER);
  const runs = [];
  heights.forEach(({ own }, k) => {
    const last = runs[runs.length - 1];
    if (last && last[0] + last[1] === own) last[1]++; else runs.push([own, 1, k]);
  });
  const feet = [];
  for (const h of heights) feet.push((await p.evaluate(i => window.rowOf(i, 'height'), h.i)).foot);
  fs.writeFileSync(`${OUT}/heights.wgsl`, [
    '// The heights of the sprites that stand in water, written by web/textures/bake/sprites.cjs from sprites.json',
    `// (web/textures/SOURCES.md): heights.png, ${HEIGHT_PER} to a layer, each texel's height over the ground as a share of its`,
    "// sprite's height, its grey.",
    `const HEIGHT_LAYERS = ${Math.ceil(heights.length / HEIGHT_PER)};`,
    '// The runs of sprites with heights: the first sprite, how many, and the first of their heights.',
    `const HEIGHT_RUNS_N = ${runs.length};`,
    `var<private> HEIGHT_RUNS: array<vec3i, ${runs.length}> = array<vec3i, ${runs.length}>(${runs.map(r => `vec3i(${r.join(', ')})`).join(', ')});`,
    '// The heights of sprite s (their number among the heights), -1 where it has none.',
    'fn heights_of(s: i32) -> i32 {',
    '  for (var r = 0; r < HEIGHT_RUNS_N; r++) { let run = HEIGHT_RUNS[r]; if (s >= run.x && s < run.x + run.y) { return run.z + s - run.x; } }',
    '  return -1;',
    '}',
    wgslArray('foot_of', feet, "// Of each of the heights, its sprite's stem's foot (the middle of its model's box on the ground) over its lowest point,\n// as a share of its height."),
  ].join('\n'));
  if (heightsOnly) {
    fs.writeFileSync(`${OUT}/strips.json`, JSON.stringify(named, null, 1) + '\n');
    await b.close(); return;
  }
  fs.writeFileSync(`${OUT}/strips.json`, JSON.stringify({
    models: await files('models', models, specs.models.side, 4),
    trees: await files('trees', strips.trees.done.map(d => [d.i, 'leafy']), specs.trees.side, 1),
    heights: named.heights,
  }, null, 1) + '\n');
  const layers = Math.ceil(models.length / 4);
  fs.writeFileSync(`${OUT}/sprites.wgsl`, [
    '// The rendered models\' table, written by web/textures/bake/sprites.cjs from sprites.json (web/textures/SOURCES.md):',
    '// the trees (trees.png) one to a layer, then the rest (models.png) four to a layer, the trees\' bare twins last.',
    `const TREES = ${trees}; const MODEL_LAYERS = ${layers};`,
    '// Each set of sprites: its first and how many.',
    ...[...sets].map(([n, [from, k]]) => `const ${n}_FROM = ${from}; const ${n}_N = ${k};`),
    wgslArray('model_w', rows.map(r => r.w), '// Each sprite\'s content, as a share of its layer (a tree) or its quarter: its width and its height.'),
    wgslArray('model_h', rows.map(r => r.h), ''),
    wgslArray('model_fill', rows.map(r => r.fill), '// Each sprite\'s mean cover over its content\'s box.'),
    wgslArray('model_leaf', rows.map(r => r.leaf), '// Each sprite\'s mean leaf lightness (linear; a bare twin its tree\'s), 0 where it has no leaves.', 4),
    '// Each tree\'s bare twin (its leaves hidden, in its own frame at half the side), -1 where it has none.',
    wgslTable('bare_of', 'BARE_TABLE', 'i32', bareOf),
  ].join('\n'));
  console.log('trees', trees, 'model layers', layers);
  await b.close();
})();
