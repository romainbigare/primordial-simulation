// The GPU's time for a full frame of the map while panning: node perf.js PORT SEED DEVICE x:y:mpc ...
// The page served must expose its map as window.__map (a copy patched for measuring only).
const { chromium, devices } = require('/opt/node22/lib/node_modules/playwright');
const [port, seed, dev, ...views] = process.argv.slice(2);
const SIZE = 22, CELL_M = 2000;
(async () => {
  const b = await chromium.launch();
  const opts = dev === 'phone' ? { ...devices['iPhone 13'] } : { viewport: { width: 1200, height: 900 }, deviceScaleFactor: 2 };
  const p = await (await b.newContext(opts)).newPage();
  await p.goto(`http://localhost:${port}/index.html?seed=${seed}&size=${SIZE}`);
  await p.waitForFunction(() => { const y = document.getElementById('year'); return y && y.textContent && y.textContent.length > 0; }, null, { timeout: 120000 });
  await p.waitForTimeout(3000);
  if (await p.$eval('#play', e => e.classList.contains('playing'))) await p.click('#play');
  const box = await (await p.$('#cv')).boundingBox();
  const fitS = Math.min(box.width / SIZE, box.height / SIZE) * 0.94;
  for (const v of views) {
    const [xk, yk, mpc] = v.split(':').map(Number);
    await p.click('#fit');
    await p.waitForTimeout(500);
    const ox = (box.width - SIZE * fitS) / 2, oy = (box.height - SIZE * fitS) / 2;
    const px = box.x + ox + (xk * 1000 / CELL_M) * fitS, py = box.y + oy + (yk * 1000 / CELL_M) * fitS;
    await p.evaluate(([x, y, d]) => document.getElementById('cv').dispatchEvent(new WheelEvent('wheel', { deltaY: d, clientX: x, clientY: y, bubbles: true, cancelable: true })), [px, py, -Math.log((CELL_M / mpc) / fitS) / 0.0015]);
    await p.waitForTimeout(8000);
    const cx = box.x + box.width / 2, cy = box.y + box.height / 2, ms = [];
    await p.mouse.move(cx, cy);
    await p.mouse.down();
    for (let i = 0; i < 40; i++) {
      await p.mouse.move(cx + 40 * Math.sin(i / 3), cy + 40 * Math.cos(i / 4));
      await p.waitForTimeout(150);
      ms.push(await p.evaluate(() => window.__map.fullMs));
    }
    await p.mouse.up();
    ms.sort((a, b) => a - b);
    console.log(v, 'median full frame', ms[ms.length >> 1].toFixed(1), 'ms');
  }
  await b.close();
})();
