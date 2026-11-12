// Frames the map draws: while the world plays (per world frame) and while the tiles of a new view come (world paused).
// node frames.js PAGE
const { chromium, devices } = require('/opt/node22/lib/node_modules/playwright');
const [page] = process.argv.slice(2);
(async () => {
  const b = await chromium.launch();
  const ctx = await b.newContext({ ...devices['iPhone 13'] });
  const p = await ctx.newPage();
  await p.addInitScript(() => {
    const W = window.Worker; window.__workers = []; window.__posted = 0;
    window.Worker = class extends W {
      constructor(...a) { super(...a); window.__workers.push(this); this.addEventListener('message', e => { if (e.data && e.data.kind === 'frame') window.__posted++; }); }
    };
  });
  await p.goto(`http://localhost:8810/${page}?seed=1&size=22`);
  await p.waitForFunction(() => window.__map && window.__map.land && /date/.test(document.querySelector('.stats')?.textContent || ''), null, { timeout: 300000 });
  await p.evaluate(() => { const mv = window.__map, d = mv.drawGpu; mv.__frames = 0; mv.drawGpu = function (...a) { mv.__frames++; return d.apply(this, a); }; });
  await p.waitForTimeout(20000);
  // Playing (the page starts the world playing at 12 months a second).
  const a = await p.evaluate(() => [window.__map.__frames, window.__posted]);
  await p.waitForTimeout(30000);
  const b2 = await p.evaluate(() => [window.__map.__frames, window.__posted]);
  console.log('playing: map frames', b2[0] - a[0], 'world frames', b2[1] - a[1]);
  // Paused: a zoom to the 1 km bar, the frames while its tiles come.
  await p.click('#play');
  await p.waitForTimeout(5000);
  const c = await p.evaluate(() => { const mv = window.__map, cv = document.getElementById('cv'), s = (96 * mv.cellM) / 1000; mv.cam = { s, ox: cv.clientWidth / 2 - 11.5 * s, oy: cv.clientHeight / 2 - 9 * s }; mv.fitted = false; mv.invalidate(); return mv.__frames; });
  for (let i = 0; i < 240; i++) { await p.waitForTimeout(500); if (await p.evaluate(() => window.__map.land.tiles.toCome() === 0 && window.__map.land.tiles.count() >= 40)) break; }
  await p.waitForTimeout(15000);
  const d = await p.evaluate(() => [window.__map.__frames, window.__map.land.tiles.count()]);
  console.log('zoom, paused: map frames', d[0] - c, 'tiles kept', d[1]);
  await b.close();
})();
