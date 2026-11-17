// node sprites.cjs sprites.json outdir [model...]: renders the sprites that sprites.json names (sprite.html) and writes
// outdir/trees.png and outdir/models.png (the strips, as web/textures/ holds them) and outdir/sprites.wgsl (the table
// the page reads: web/shaders/lib/sprites.wgsl); with models named, each of their sprites alone to outdir/<model>_<k>.png
// (and its bare twin to outdir/<model>_<k>_bare.png).
//
// sprites.json: for each strip (`trees`, `models`) the side of its layers, its cells a layer (`per`: 1 or 4) and its
// sprites, each entry a model with: `parts`, lists of its nodes, each part a sprite (default: the whole model); `turns`,
// degrees about the vertical, each a sprite of each part; `cut`, the share of its height cut off at its foot; `rest`,
// set down on its resting face (sprite.html `restingTurn`) and `sink`, the share of its height sunk into the ground,
// cut off at the ground; `bare`, the materials (a pattern) hidden for its bare twin (the trees' twins go into the models'
// strip after its own sprites; sprites.wgsl `bare_of`); `stem`, a stem it is set on (sprite.html `meshesOf`); `leaf`, the
// mean lightness (linear) its leaves are scaled to (its other texels by the square root of that scale), so the models'
// leaves are about as light as each other's; `set`, the name of the set it starts (sprites.wgsl `<set>_FROM` and
// `<set>_N`, the set running to the next entry with a set).
const { chromium } = require('playwright');
const fs = require('fs');
const specs = JSON.parse(fs.readFileSync(process.argv[2])), OUT = process.argv[3], only = process.argv.slice(4);
// The sprites of an entry, part by part and turn by turn.
const expand = e => (e.parts || [null]).flatMap(nodes => (e.turns || [0]).map(turn => ({ ...e, nodes, turn })));
const list = a => a.map(v => v.toFixed(3)).join(', ');
const wgslArray = (name, rows, doc) => `${doc ? doc + '\n' : ''}fn ${name}(s: i32) -> f32 {\n  var v = array<f32, ${rows.length}>(${list(rows)});\n  return v[s];\n}\n`;

(async () => {
  const b = await chromium.launch({ args: ['--use-gl=swiftshader', '--enable-unsafe-swiftshader'] });
  const p = await b.newPage({ viewport: { width: 1100, height: 1100 } });
  p.on('console', m => { if (m.type() === 'error') console.log('ERR', m.text()); });
  p.on('pageerror', e => console.log('PAGEERR', e.message));
  await p.goto('http://localhost:8835/render/sprite.html');
  await p.waitForFunction(() => window.ready === true);
  const save = (name, url) => fs.writeFileSync(`${OUT}/${name}`, Buffer.from(url.split(',')[1], 'base64'));
  const strips = {};
  for (const [strip, spec] of Object.entries(specs)) {
    const cell = spec.per === 4 ? spec.side / 2 : spec.side, done = [];
    let set = null;
    for (const e of spec.sprites) {
      if (e.set) set = { name: e.set, from: done.length, n: 0 };
      if (only.length && !only.includes(e.model)) continue;
      for (const [k, s] of expand(e).entries()) {
        const t = Date.now(), i = await p.evaluate(([s, c]) => window.sprite(s, c), [s, cell]);
        done.push({ i, s, set });
        if (set) set.n++;
        console.log('rendered', s.model, s.nodes || '', s.turn, 'leaves', (await p.evaluate(i => window.rowOf(i, false), i)).leaf.toFixed(4), (Date.now() - t) / 1000, 's');
        if (only.length) {
          save(`${s.model}_${k}.png`, await p.evaluate(i => window.cellOf(i, false), i));
          if (s.bare) save(`${s.model}_${k}_bare.png`, await p.evaluate(i => window.cellOf(i, true), i));
        }
      }
    }
    strips[strip] = { spec, cell, done };
  }
  if (only.length) { await b.close(); return; }
  // The trees' bare twins after the models' own sprites.
  const bares = strips.trees.done.filter(d => d.s.bare);
  const rows = [], sets = new Map();
  for (const d of [...strips.trees.done, ...strips.models.done]) rows.push(await p.evaluate(i => window.rowOf(i, false), d.i));
  const bareFrom = rows.length;
  for (const d of bares) rows.push(await p.evaluate(i => window.rowOf(i, true), d.i));
  const bareOf = strips.trees.done.map(d => (d.s.bare ? bareFrom + bares.indexOf(d) : -1));
  const trees = strips.trees.done.length;
  strips.trees.done.forEach(d => d.set && sets.set(d.set.name, [d.set.from, d.set.n]));
  strips.models.done.forEach(d => d.set && sets.set(d.set.name, [trees + d.set.from, d.set.n]));
  const models = [...strips.models.done.map(d => [d.i, false]), ...bares.map(d => [d.i, true])];
  save('trees.png', await p.evaluate(([l, side]) => window.strip(l, side, 1), [strips.trees.done.map(d => [d.i, false]), specs.trees.side]));
  save('models.png', await p.evaluate(([l, side]) => window.strip(l, side, 4), [models, specs.models.side]));
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
    '// Each tree\'s bare twin (its leaves hidden, in its own frame at half the side), -1 where it has none.',
    `fn bare_of(s: i32) -> i32 {\n  var v = array<i32, ${trees}>(${bareOf.join(', ')});\n  return v[s];\n}\n`,
  ].join('\n'));
  console.log('trees', trees, 'model layers', layers);
  await b.close();
})();
