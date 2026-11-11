// Inlines the compiled modules (dist/.js), the shaders (dist/.shaders.json, written by web/shade from web/shaders) and
// style.css into one self-contained dist/index.html.
import { readFileSync, writeFileSync, rmSync } from 'node:fs';

const order = ['data', 'fields', 'earth', 'style', 'form', 'canopy', 'gpu', 'gpu-webgpu', 'gpu-webgl2', 'field-window', 'ground', 'land', 'map', 'charts', 'live', 'sprites', 'blob', 'textures', 'vegetation', 'avatars', 'main'];
// A module as plain script: its imports and exports taken out (the modules share one scope).
const plain = n => readFileSync(`dist/.js/${n}.js`, 'utf8').replace(/^import .*$/gm, '').replace(/^export \{\};$/gm, '').replace(/^export /gm, '');
// The live world's two workers (the world's and the ground's tiles') run on their own: the page holds their scripts as
// text and starts them from Blobs.
const worker = plain('worker'), tiles = plain('tiles-worker');
// The shaders as `SHADERS` (gpu.ts), ahead of the modules.
const js = [`const SHADERS = ${readFileSync('dist/.shaders.json', 'utf8')};`, ...order.map(plain)].join('\n');
// The page with `before` ahead of the modules' script (which reads what comes before it as it runs).
const page = before => readFileSync('index.html', 'utf8')
  .replace('<!--CSS-->', () => `<style>\n${readFileSync('style.css', 'utf8')}</style>`)
  .replace('<!--JS-->', () => `${before}<script type="text/plain" id="worker-src">\n${worker}<\/script>\n<script type="text/plain" id="tiles-src">\n${tiles}<\/script>\n<script>\n(() => {\n${js}\n})();\n</script>`);
const html = page('');
writeFileSync('dist/index.html', html);
// `node build.mjs world.ewf.gz` also writes dist/standalone.html with the world embedded.
const embed = process.argv[2];
if (embed) writeFileSync('dist/standalone.html',
  page(`<script type="application/octet-stream" id="world-data">${readFileSync(embed).toString('base64')}</script>\n`));
rmSync('dist/.js', { recursive: true });
rmSync('dist/.shaders.json');
console.log(`dist/index.html ${(html.length / 1024).toFixed(1)} KB`);
