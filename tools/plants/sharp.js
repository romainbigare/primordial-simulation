// How long the picture takes to sharpen after the view rests: node sharp.js PORT
const { chromium } = require('/opt/node22/lib/node_modules/playwright');
const [port] = process.argv.slice(2);
const SIZE = 22, DSF = 2, CELL_M = 2000;
(async () => {
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: 1200, height: 900 }, deviceScaleFactor: DSF });
  await p.goto(`http://localhost:${port}/index.html?seed=1&size=22`);
  await p.waitForTimeout(5000);
  const speeds = await p.$$('#speeds button');
  await speeds[2].click();
  const yr = async () => parseFloat(((await p.textContent('#year')) || '').replace(/[^0-9.]/g, '')) || 0;
  while ((await yr()) < 20) await p.waitForTimeout(300);
  await p.click('#play');
  const box = await (await p.$('#cv')).boundingBox();
  const W = box.width, H = box.height, fit = Math.min(W / SIZE, H / SIZE) * 0.94, ox = (W - SIZE * fit) / 2, oy = (H - SIZE * fit) / 2;
  const x = box.x + ox + 4.5 * fit, y = box.y + oy + 6.6 * fit;
  await p.mouse.move(x, y);
  const total = DSF * Math.log(96 * CELL_M / 5 / fit) / 0.0015, n = Math.ceil(total / 1000);
  for (let k = 0; k < n; k++) { await p.mouse.wheel(0, -total / n); await p.waitForTimeout(120); }
  await p.mouse.move(2, 2);
  const el = await p.$('#map');
  let last = await el.screenshot();
  const t0 = Date.now();
  for (let k = 0; k < 24; k++) {
    await p.waitForTimeout(5000);
    const now = await el.screenshot();
    console.log(((Date.now() - t0) / 1000).toFixed(0), 's changed:', !now.equals(last), (await p.textContent('.stats')).replace(/\s+/g, ' ').slice(0, 120));
    last = now;
  }
  require('fs').writeFileSync('sharp-last.png', last);
  await b.close();
})();
