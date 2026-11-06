// Inlines the compiled modules (dist/.js) and style.css into one self-contained dist/index.html.
import { readFileSync, writeFileSync, rmSync } from 'node:fs';

const order = ['data', 'earth', 'fine', 'map', 'charts', 'live', 'sprites', 'avatars', 'main'];
// A module as plain script: its imports and exports taken out (the modules share one scope).
const plain = n => readFileSync(`dist/.js/${n}.js`, 'utf8').replace(/^import .*$/gm, '').replace(/^export \{\};$/gm, '').replace(/^export /gm, '');
// The live world's worker runs on its own, with the modules it paints with: the page holds its script as text and
// starts it from a Blob.
const worker = ['data', 'earth', 'fine', 'worker'].map(plain).join('\n');
const js = order.map(plain).join('\n');
// The page with `before` ahead of the modules' script (which reads what comes before it as it runs).
const page = before => readFileSync('index.html', 'utf8')
  .replace('<!--CSS-->', () => `<style>\n${readFileSync('style.css', 'utf8')}</style>`)
  .replace('<!--JS-->', () => `${before}<script type="text/plain" id="worker-src">\n${worker}<\/script>\n<script>\n(() => {\n${js}\n})();\n</script>`);
const html = page('');
writeFileSync('dist/index.html', html);
// `node build.mjs world.ewf.gz` also writes dist/standalone.html with the world embedded.
const embed = process.argv[2];
if (embed) writeFileSync('dist/standalone.html',
  page(`<script type="application/octet-stream" id="world-data">${readFileSync(embed).toString('base64')}</script>\n`));
rmSync('dist/.js', { recursive: true });
console.log(`dist/index.html ${(html.length / 1024).toFixed(1)} KB`);
