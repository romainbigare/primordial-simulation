// node shoot.js PORT SEED SIZE YEAR OUT VIEW [x,y,barMetres ...]
// Load the page, play at full speed until the date shown reaches YEAR, pause, shoot the whole island in VIEW
// (earth or true), then for each place zoom so the scale bar reads barMetres at x,y (squares) and shoot.
const { chromium } = require('/opt/node22/lib/node_modules/playwright');
const [port, seed, size, year, out, view, ...places] = process.argv.slice(2);
(async () => {
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: 1000, height: 900 }, deviceScaleFactor: 1 });
  const errs = [];
  p.on('console', m => { if (m.type() === 'error') errs.push(m.text()); });
  p.on('pageerror', e => errs.push('pageerror: ' + e.message));
  await p.goto(`http://localhost:${port}/index.html?seed=${seed}&size=${size}`);
  await p.waitForFunction(() => /Year/.test(document.getElementById('year').textContent || ''), null, { timeout: 300000 });
  await p.waitForTimeout(2000);
  const speeds = await p.$$('#speeds button');
  await speeds[2].click();
  const reach = y => p.waitForFunction(y => { const m = (document.getElementById('year').textContent || '').match(/Year ([\d.,]+)/); return m && parseFloat(m[1].replace(/,/g, '')) >= y; }, y, { timeout: 900000, polling: 50 });
  await reach(+year);
  await p.click('#play');
  await p.click(`#views button[data-view="${view}"]`);
  await p.waitForTimeout(8000);
  const shown = await p.textContent('#year');
  await (await p.$('#map')).screenshot({ path: `${out}-whole.png` });
  const box = await (await p.$('#cv')).boundingBox();
  const n = +size, s0 = Math.min(box.width / n, box.height / n) * 0.94, ox = (box.width - n * s0) / 2, oy = (box.height - n * s0) / 2;
  for (const pl of places) {
    const [x, y, bar] = pl.split(',').map(Number);
    await p.dblclick('#cv', { position: { x: 5, y: 5 } }).catch(() => {});
    await p.waitForTimeout(1500);
    const want = 96 / (bar / 2000); // px per cell where the bar reads `bar`
    await p.mouse.move(box.x + ox + x * s0, box.y + oy + y * s0);
    await p.mouse.wheel(0, -Math.log(want / s0) / 0.0015);
    await p.waitForTimeout(15000);
    await p.mouse.move(box.x + 2, box.y + 2);
    await (await p.$('#map')).screenshot({ path: `${out}-${x}-${y}-${bar}.png` });
  }
  console.log(out, shown, 'errors:', errs.filter(e => !/GPU stall|adapter/i.test(e)).length, errs.slice(0, 2).join(' | '));
  await b.close();
})();
