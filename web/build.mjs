// Inlines the compiled modules (dist/.js), the shaders (dist/.shaders.json, written by web/shade from web/shaders) and
// style.css into one self-contained dist/index.html.
import { readFileSync, writeFileSync, rmSync, copyFileSync } from 'node:fs';
import { Script } from 'node:vm';

const order = ['data', 'fields', 'earth', 'style', 'form', 'kind_row', 'lattices', 'canopy', 'gpu', 'gpu-webgpu', 'gpu-webgl2', 'detail', 'field-window', 'ground', 'land', 'map', 'charts', 'live', 'textures', 'vegetation', 'slots', 'main'];
// A module as plain script: its imports and exports taken out (the modules share one scope).
const plain = n => readFileSync(`dist/.js/${n}.js`, 'utf8').replace(/^import .*$/gm, '').replace(/^export \{\};$/gm, '').replace(/^export /gm, '');
// The live world's two workers (the world's and the ground's tiles') run on their own: the page holds their scripts as
// text and starts them from Blobs.
const worker = plain('worker'), tiles = plain('tiles-worker');
// The shaders as `SHADERS` (gpu.ts), ahead of the modules.
const parts = [{ name: 'the shaders', code: `const SHADERS = ${readFileSync('dist/.shaders.json', 'utf8')};` },
  ...order.map(n => ({ name: `src/${n}.ts`, code: plain(n) }))];
// The modules as one block in strict mode: a top-level name declared in two modules is then an error, where in a
// function's body the later function would silently replace the earlier one.
const block = ps => `'use strict';\n{\n${ps.map(p => p.code).join('\n')}\n}`;
// The error of compiling these parts as one block, or null.
const error = ps => { try { new Script(block(ps)); return null; } catch (e) { if (e instanceof SyntaxError) return e; throw e; } };
// When the block does not compile, the first module that breaks it and, unless it breaks on its own, the earlier
// module it clashes with.
if (error(parts)) {
  const k = parts.findIndex((_, i) => error(parts.slice(0, i + 1)));
  const first = error([parts[k]]) ? undefined : parts.slice(0, k).find(p => error([p, parts[k]]));
  const broken = first ? [first, parts[k]] : [parts[k]];
  console.error(`build: ${broken.map(p => p.name).join(' and ')}: ${error(broken).message}`);
  process.exit(1);
}
const js = block(parts);
// The page with `before` ahead of the modules' script (which reads what comes before it as it runs).
const page = before => readFileSync('index.html', 'utf8')
  .replace('<!--CSS-->', () => `<style>\n${readFileSync('style.css', 'utf8')}</style>`)
  .replace('<!--JS-->', () => `${before}<script type="text/plain" id="worker-src">\n${worker}<\/script>\n<script type="text/plain" id="tiles-src">\n${tiles}<\/script>\n<script>\n${js}\n</script>`);
const html = page('');
writeFileSync('dist/index.html', html);
// `node build.mjs world.ewf.gz` also writes dist/standalone.html with the world embedded.
const embed = process.argv[2];
if (embed) writeFileSync('dist/standalone.html',
  page(`<script type="application/octet-stream" id="world-data">${readFileSync(embed).toString('base64')}</script>\n`));
// The ground's photographs and plants, beside the page (detail.ts).
copyFileSync('textures/ground.jpg', 'dist/ground.jpg');
copyFileSync('textures/carpets.jpg', 'dist/carpets.jpg');
copyFileSync('textures/models.png', 'dist/models.png');
copyFileSync('textures/trees.png', 'dist/trees.png');
rmSync('dist/.js', { recursive: true });
rmSync('dist/.shaders.json');
console.log(`dist/index.html ${(html.length / 1024).toFixed(1)} KB`);
