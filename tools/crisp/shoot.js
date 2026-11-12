// Shoot views of the live map. node shoot.js PORT SEED DEVICE(phone|desk) OUTDIR VIEW(earth|true) shot...
// shot = name:xkm:ykm:mpercss   (place in km from the map's north-west corner; metres a css pixel)
// The world plays for a moment at its start and is paused, then each shot zooms and pans to its place.
const { chromium, devices } = require('/opt/node22/lib/node_modules/playwright');
const [port, seed, dev, out, viewName, ...shots] = process.argv.slice(2);
const SIZE = 22, CELL_M = 2000;
(async () => {
  const b = await chromium.launch();
  const opts = dev === 'phone' ? { ...devices['iPhone 13'] } : { viewport: { width: 1200, height: 900 }, deviceScaleFactor: 2 };
  const ctx = await b.newContext(opts);
  const p = await ctx.newPage();
  const errs = [];
  p.on('console', m => { if (m.type() === 'error') errs.push(m.text()); });
  p.on('pageerror', e => errs.push('pageerror: ' + e.message));
  await p.goto(`http://localhost:${port}/index.html?seed=${seed}&size=${SIZE}`);
  await p.waitForFunction(() => { const y = document.getElementById('year'); return y && y.textContent && y.textContent.length > 0; }, null, { timeout: 120000 });
  await p.waitForTimeout(3000);
  if (await p.$eval('#play', e => e.classList.contains('playing'))) await p.click('#play');
  if (viewName === 'true') await p.click('#views button[data-view="true"]');
  await p.waitForTimeout(3000);
  const box = await (await p.$('#cv')).boundingBox();
  const fitS = Math.min(box.width / SIZE, box.height / SIZE) * 0.94;
  for (const shot of shots) {
    const [name, xk, yk, mpc] = shot.split(':');
    // Back to fitted (double-click the fit button), then zoom at the place and drag it to the middle.
    await p.click('#fit');
    await p.waitForTimeout(500);
    const ox = (box.width - SIZE * fitS) / 2, oy = (box.height - SIZE * fitS) / 2;
    const px = box.x + ox + (+xk * 1000 / CELL_M) * fitS, py = box.y + oy + (+yk * 1000 / CELL_M) * fitS;
    const k = (CELL_M / +mpc) / fitS;
    await p.mouse.move(px, py);
    // One wheel event of the whole step, sent to the canvas itself (the browser's own wheel is capped and coalesced).
    await p.evaluate(([x, y, d]) => document.getElementById('cv').dispatchEvent(new WheelEvent('wheel', { deltaY: d, clientX: x, clientY: y, bubbles: true, cancelable: true })), [px, py, -Math.log(k) / 0.0015]);
    await p.waitForTimeout(300);
    const cx = box.x + box.width / 2, cy = box.y + box.height / 2;
    await p.mouse.move(px, py);
    await p.mouse.down();
    for (let i = 1; i <= 10; i++) await p.mouse.move(px + (cx - px) * i / 10 + 12 * Math.sin(i / 10 * Math.PI), py + (cy - py) * i / 10 + 12 * Math.sin(i / 10 * Math.PI));
    await p.mouse.up();
    await p.mouse.move(box.x + box.width / 2, box.y + box.height + 40);
    // Wait for the tiles: the stats line says how many are still to come.
    for (let t = 0; t < 120; t++) {
      await p.waitForTimeout(1000);
      const s = (await p.textContent('#stats').catch(() => '')) || '';
      if (t > 4 && !/(\d+) to come/.test(s)) break;
      const m = s.match(/(\d+) to come/);
      if (t > 4 && m && +m[1] === 0) break;
    }
    await p.waitForTimeout(2500);
    await (await p.$('#map')).screenshot({ path: `${out}/${name}.png` });
    const stats = ((await p.textContent('#stats').catch(() => '')) || '').replace(/\s+/g, ' ');
    console.log(name, '|', stats.slice(0, 200));
  }
  console.log('errors:', errs.length, errs.slice(0, 3).join(' | '));
  await b.close();
})();
