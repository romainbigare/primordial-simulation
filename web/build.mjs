// Inlines the compiled modules (dist/.js) and style.css into one self-contained dist/index.html.
import { readFileSync, writeFileSync, rmSync } from 'node:fs';

const order = ['data', 'earth', 'map', 'charts', 'main'];
const js = order
  .map(n => readFileSync(`dist/.js/${n}.js`, 'utf8').replace(/^import .*$/gm, '').replace(/^export \{\};$/gm, '').replace(/^export /gm, ''))
  .join('\n');
const html = readFileSync('index.html', 'utf8')
  .replace('<!--CSS-->', () => `<style>\n${readFileSync('style.css', 'utf8')}</style>`)
  .replace('<!--JS-->', () => `<script>\n(() => {\n${js}\n})();\n</script>`);
writeFileSync('dist/index.html', html);
// `node build.mjs world.ewf.gz` also writes dist/standalone.html with the world embedded.
const embed = process.argv[2];
if (embed) writeFileSync('dist/standalone.html', html.replace('</body>', () =>
  `<script type="application/octet-stream" id="world-data">${readFileSync(embed).toString('base64')}</script>\n</body>`));
rmSync('dist/.js', { recursive: true });
console.log(`dist/index.html ${(html.length / 1024).toFixed(1)} KB`);
