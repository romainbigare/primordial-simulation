// node bake.js specs.json outdir [name...]: bakes each named tile of the specs to outdir/<name>.png.
const { chromium } = require('playwright');
const fs = require('fs');
const specs = JSON.parse(fs.readFileSync(process.argv[2])), OUT = process.argv[3], only = process.argv.slice(4);
(async () => {
  const b = await chromium.launch({ args: ['--use-gl=swiftshader', '--enable-unsafe-swiftshader'] });
  const p = await b.newPage({ viewport: { width: 1100, height: 1100 } });
  p.on('console', m => { if (m.type() === 'error') console.log('ERR', m.text()); });
  p.on('pageerror', e => console.log('PAGEERR', e.message));
  await p.goto('http://localhost:8835/render/bake.html');
  await p.waitForFunction(() => window.ready === true);
  for (const [name, spec] of Object.entries(specs)) {
    if (only.length && !only.includes(name)) continue;
    const t = Date.now();
    const url = await p.evaluate(s => window.bake(s), spec);
    fs.writeFileSync(`${OUT}/${name}.png`, Buffer.from(url.split(',')[1], 'base64'));
    console.log('baked', name, (Date.now() - t) / 1000, 's');
  }
  await b.close();
})();
