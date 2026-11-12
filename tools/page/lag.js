// The lag of the date shown behind the world, over time, at max speed at a view (metres for ~96 px; 0: fitted): every
// second the date the page shows and the world's date in the worker (read there), in months; then a pause, and where
// both stop. node lag.js PORT METRES SECONDS
const { chromium } = require('/opt/node22/lib/node_modules/playwright');
const [port, metres, secs] = process.argv.slice(2);
const months = t => { const m = /year (\d+), day (\d+)/.exec(t || ''); return m ? (+m[1] * 360 + +m[2] - 1) / 30 : NaN; };
(async () => {
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: 1200, height: 900 } });
  const errs = [];
  p.on('console', m => { if (m.type() === 'error') errs.push(m.text()); });
  p.on('pageerror', e => errs.push('pageerror: ' + e.message));
  await p.goto(`http://localhost:${port}/index.html?seed=1&size=22`);
  await p.waitForFunction(() => /date/.test(document.querySelector('.stats')?.textContent || ''), null, { timeout: 120000 });
  await p.waitForTimeout(3000);
  // The world's worker: the one whose global scope holds the core.
  let world = null;
  for (const w of p.workers()) { if (await w.evaluate(() => typeof postFrame === 'function' && core !== null).catch(() => false)) world = w; }
  const worldMonths = async () => world.evaluate(() => core.year() * 12);
  const shown = async () => months(await p.textContent('.stats'));
  const box = await (await p.$('#cv')).boundingBox();
  const cellM = await p.evaluate(() => { const t = document.getElementById('meta').textContent; const m = /cell ([\d.]+) (k?m)/.exec(t); return m ? +m[1] * (m[2] === 'km' ? 1000 : 1) : NaN; });
  const fit = Math.min(box.width / 22, box.height / 22) * 0.94;
  if (+metres) {
    const want = (96 * cellM / +metres) / fit;
    await p.mouse.move(box.x + box.width * 0.45, box.y + box.height * 0.5);
    let left = Math.log(want);
    while (Math.abs(left) > 1e-6) { const step = Math.max(-2, Math.min(2, left)); await p.mouse.wheel(0, -step / 0.0015); left -= step; await p.waitForTimeout(100); }
    await p.mouse.move(2, 2);
  }
  const speeds = await p.$$('#speeds button');
  await speeds[speeds.length - 1].click();
  const t0 = Date.now(), w0 = await worldMonths(), rows = [];
  while (Date.now() - t0 < 1000 * +secs) {
    await p.waitForTimeout(1000);
    const s = await shown(), w = await worldMonths();
    rows.push([((Date.now() - t0) / 1000).toFixed(1), s.toFixed(0), w.toFixed(0), (w - s).toFixed(0)]);
  }
  const t1 = Date.now(), w1 = await worldMonths();
  const pace = (w1 - w0) / ((t1 - t0) / 1000);
  // Pause: the world stops; the page then shows where it stopped.
  // The date shown and the click in one task of the page, as a person clicks at the date they see.
  const tp = Date.now();
  // With STALL ms of the page's main thread busy first (a slow draw), and the click at its end, as a person's click waits
  // for it.
  const stall = +(process.env.STALL || 0);
  const atClick = months(await p.evaluate(ms => { const s = performance.now(); while (performance.now() - s < ms); const t = document.querySelector('.stats').textContent; document.getElementById('play').click(); return t; }, stall));
  const clicked = Date.now() - tp;
  await p.waitForTimeout(3000);
  const ws = await worldMonths(), ss = await shown();
  console.log('t s, shown mo, world mo, lag mo');
  rows.forEach(r => console.log(r.join('  ')));
  const lags = rows.map(r => +r[3]);
  console.log(JSON.stringify({ view: +metres ? `${metres} m` : 'fitted', worldMonthsPerSec: +pace.toFixed(1), lagMonthsMean: +(lags.reduce((a, x) => a + x, 0) / lags.length).toFixed(1), lagMonthsLast: lags[lags.length - 1], lagSecondsMean: +((lags.reduce((a, x) => a + x, 0) / lags.length) / pace).toFixed(2), clickMs: clicked, shownAtClick: +atClick.toFixed(1), pausedPastShown: +(ws - atClick).toFixed(1), pausedWorld: +ws.toFixed(1), pausedShown: +ss.toFixed(1) }));
  console.log('errors:', errs.filter(e => !/GPU stall|adapter/i.test(e)).length, errs.slice(0, 3).join(' | '));
  await b.close();
})();
