// Inlines the compiled modules (dist/.js), the shaders (dist/.shaders.json, written by web/shade from web/shaders) and
// style.css into one self-contained dist/index.html.
import { readFileSync, writeFileSync, rmSync, copyFileSync, readdirSync, mkdirSync } from 'node:fs';

const order = ['data', 'errors', 'cost', 'fields', 'earth', 'style', 'form', 'kind_row', 'lattices', 'paint', 'canopy', 'gpu', 'gpu-webgl2', 'detail', 'field-window', 'ground', 'land', 'burns', 'fire', 'gesture', 'map', 'charts', 'live', 'textures', 'vegetation', 'slots', 'soil', 'icons', 'panels', 'god-types', 'god-draw', 'god-controls', 'god-list', 'god-menu', 'god', 'god-tools', 'front', 'timeline', 'world-form', 'inspect', 'numbers', 'boot', 'main'];
// A module as plain script: its imports and exports taken out (the modules share one scope).
const plain = n => readFileSync(`dist/.js/${n}.js`, 'utf8').replace(/^import .*$/gm, '').replace(/^export \{\};$/gm, '').replace(/^export /gm, '');
// The live world's two workers (the world's and the ground's tiles') and the detail's (its strips' levels) run on
// their own: the page holds their scripts as text and starts them from Blobs.
const worker = plain('worker'), tiles = plain('tiles-worker'), detail = plain('detail-worker');
// The strips' files of the ground's detail, by group (textures/strips.json, written by the bake tools).
const strips = JSON.parse(readFileSync('textures/strips.json', 'utf8'));
// The shaders as `SHADERS` (gpu.ts) and the strips' files as `STRIPS` (detail.ts), ahead of the modules.
// The modules share one scope, so no two may declare the same name (one would take the other's place unseen).
const declared = new Map();
for (const n of order) for (const [, name] of readFileSync(`dist/.js/${n}.js`, 'utf8').matchAll(/^(?:export )?(?:async )?(?:function\*?|const|let|var|class) ([\w$]+)/gm)) {
  if (declared.has(name)) throw new Error(`${name} is declared in both ${declared.get(name)}.ts and ${n}.ts: the page's modules share one scope`);
  declared.set(name, n);
}
const js = [`const SHADERS = ${readFileSync('dist/.shaders.json', 'utf8')};`, `const STRIPS = ${JSON.stringify(strips)};`, ...order.map(plain)].join('\n');
// The page with `before` ahead of the modules' script (which reads what comes before it as it runs).
const page = before => readFileSync('index.html', 'utf8')
  .replace('<!--CSS-->', () => `<style>\n${readFileSync('style.css', 'utf8')}</style>`)
  .replace('<!--JS-->', () => `${before}<script type="text/plain" id="worker-src">\n${worker}<\/script>\n<script type="text/plain" id="tiles-src">\n${tiles}<\/script>\n<script type="text/plain" id="detail-src">\n${detail}<\/script>\n<script>\n(() => {\n${js}\n})();\n</script>`);
const html = page('');
writeFileSync('dist/index.html', html);
// `node build.mjs world.ewf.gz` also writes dist/standalone.html with the world embedded.
const embed = process.argv[2];
if (embed) writeFileSync('dist/standalone.html',
  page(`<script type="application/octet-stream" id="world-data">${readFileSync(embed).toString('base64')}</script>\n`));
// The ground's photographs and plants, beside the page, with their smaller sides (`name@side.ext`, detail.ts).
const sized = readdirSync('textures');
for (const f of Object.values(strips).flat()) {
  const [stem, ext] = [f.replace(/\.[a-z]+$/, ''), f.slice(f.lastIndexOf('.'))];
  for (const g of [f, ...sized.filter(n => n.startsWith(`${stem}@`) && n.endsWith(ext))]) copyFileSync(`textures/${g}`, `dist/${g}`);
}
// God mode's pictures (god-list.ts, god-controls.ts), beside the page in `god/`: each tool's and each starting kind's.
mkdirSync('dist/god', { recursive: true });
for (const f of readdirSync('textures/god').filter(n => n.endsWith('.jpg'))) copyFileSync(`textures/god/${f}`, `dist/god/${f}`);
rmSync('dist/.js', { recursive: true });
rmSync('dist/.shaders.json');
console.log(`dist/index.html ${(html.length / 1024).toFixed(1)} KB`);
