// Frames a second the page draws while the view moves (world paused, all tiles come), at a place and scale bars:
// node cost.js PORT YEAR X,Y "2000,200,50,10"
const { chromium } = require('/opt/node22/lib/node_modules/playwright');
const [port, yearArg, xy, scalesArg] = process.argv.slice(2);
const SIZE = 22, DSF = 2, CELL_M = 2000;
(async () => {
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: +(process.env.VW || 1200), height: +(process.env.VH || 900) }, deviceScaleFactor: DSF });
  await p.goto(`http://localhost:${port}/index.html?seed=1&size=${SIZE}`);
  await p.waitForTimeout(5000);
  const speeds = await p.$$('#speeds button');
  await speeds[2].click();
  await p.evaluate(() => { const b = document.getElementById('play'); if (!b.classList.contains('playing')) b.click(); });
  const yr = async () => parseFloat(((await p.textContent('#year')) || '').replace(/[^0-9.]/g, '')) || 0;
  while ((await yr()) < +yearArg) await p.waitForTimeout(500);
  // The world's pace at the top speed with the island fitted: the stats' months a second, sampled over 12 s.
  await p.click('#speeds button[data-speed="Infinity"]');
  await p.waitForTimeout(3000);
  const paces = [];
  for (let k = 0; k < 12; k++) { const m = ((await p.textContent('.stats')) || '').match(/pace ([0-9.]+)/); if (m) paces.push(+m[1]); await p.waitForTimeout(1000); }
  console.log(port, 'pace', 'world', 'months/s:', (paces.reduce((a, b) => a + b, 0) / Math.max(paces.length, 1)).toFixed(2));
  await p.evaluate(() => document.getElementById('play').click());
  const stats = async () => ((await p.textContent('.stats')) || '').replace(/\s+/g, ' ');
  const settle = async () => { for (let k = 0; k < 300; k++) { const m = (await stats()).match(/(\d+) to come/); if (!m || +m[1] === 0) break; await p.waitForTimeout(100); } await p.waitForTimeout(1500); };
  const box = await (await p.$('#cv')).boundingBox();
  const W = box.width, H = box.height, s = Math.min(W / SIZE, H / SIZE) * 0.94, ox = (W - SIZE * s) / 2, oy = (H - SIZE * s) / 2;
  const [tx, ty] = xy.split(',').map(Number);
  for (const style of (process.env.ORDER || 'above,angled').split(',')) {
    await p.click(`#life button[data-life="${style}"]`);
    for (const M of scalesArg.split(',').map(Number)) {
      await p.dblclick('#cv'); await p.waitForTimeout(400);
      if (M > 0) {
      const x = box.x + ox + tx * s, y = box.y + oy + ty * s;
      await p.mouse.move(x, y); await p.mouse.down();
      for (let i = 1; i <= 10; i++) { await p.mouse.move(x + (box.x + W / 2 - x) * i / 10, y + (box.y + H / 2 - y) * i / 10); await p.waitForTimeout(20); }
      await p.mouse.up();
      const total = DSF * Math.log(96 * CELL_M / M / s) / 0.0015, n = Math.ceil(total / 1000);
      await p.mouse.move(box.x + W / 2, box.y + H / 2);
      for (let k = 0; k < n; k++) { await p.mouse.wheel(0, -total / n); await p.waitForTimeout(120); }
      }
      await settle();
      // Count animation frames while the view pans a little each frame for 4 s.
      const fps = await p.evaluate(async () => {
        const cv = document.getElementById('cv'); const r = cv.getBoundingClientRect();
        let frames = 0; const t0 = performance.now(); let k = 0;
        await new Promise(done => { const f = () => { frames++; k++; cv.dispatchEvent(new WheelEvent('wheel', { deltaY: k % 2 ? 1 : -1, clientX: r.x + r.width / 2, clientY: r.y + r.height / 2, bubbles: true, cancelable: true })); if (performance.now() - t0 < 15000) requestAnimationFrame(f); else done(); }; requestAnimationFrame(f); });
        return frames / ((performance.now() - t0) / 1000);
      });
      console.log(port, style, M, 'm:', fps.toFixed(2), 'frames/s');
    }
  }
  await b.close();
})();
