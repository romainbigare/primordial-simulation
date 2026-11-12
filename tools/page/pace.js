// Load the live page (seed 1 size 22), play at max speed, and for the fitted, 200 m and 50 m views measure the world's
// pace (months a second, from the date shown over wall time and from the page's own pace) and the lag of the date shown
// behind the world (the date shown just before a pause against the date the pause settles on).
// node pace.js PORT OUT_PREFIX SECONDS_PER_VIEW
const { chromium } = require('/opt/node22/lib/node_modules/playwright');
const [port, out, secs] = process.argv.slice(2);
const months = t => { const m = /year (\d+), day (\d+)/.exec(t || ''); return m ? (+m[1] * 360 + +m[2]) / 30 : NaN; };
(async () => {
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: 1200, height: 900 } });
  const errs = [];
  p.on('console', m => { if (m.type() === 'error') errs.push(m.text()); });
  p.on('pageerror', e => errs.push('pageerror: ' + e.message));
  await p.goto(`http://localhost:${port}/index.html?seed=1&size=22`);
  await p.waitForFunction(() => /date/.test(document.querySelector('.stats')?.textContent || ''), null, { timeout: 120000 });
  await p.waitForTimeout(3000);
  const speeds = await p.$$('#speeds button');
  await speeds[speeds.length - 1].click();
  const box = await (await p.$('#cv')).boundingBox();
  const cellM = await p.evaluate(() => { const t = document.getElementById('meta').textContent; const m = /cell ([\d.]+) (k?m)/.exec(t); return m ? +m[1] * (m[2] === 'km' ? 1000 : 1) : NaN; });
  const fit = Math.min(box.width / 22, box.height / 22) * 0.94;
  const stats = async () => ((await p.textContent('.stats')) || '').replace(/\s+/g, ' ');
  const date = async () => months(await stats());
  const results = [];
  let zoomNow = 1;
  for (const [name, metres] of [['fitted', 0], ['200 m', 200], ['50 m', 50]]) {
    if (metres) {
      // The scale bar shows about 96 px for its length: px per square = 96 * cell / metres.
      const want = (96 * cellM / metres) / fit, k = want / zoomNow;
      await p.mouse.move(box.x + box.width * 0.45, box.y + box.height * 0.5);
      let left = Math.log(k);
      while (Math.abs(left) > 1e-6) { const step = Math.max(-2, Math.min(2, left)); await p.mouse.wheel(0, -step / 0.0015); left -= step; await p.waitForTimeout(100); }
      zoomNow = want;
      await p.mouse.move(2, 2);
    }
    await p.waitForTimeout(4000);
    // The date shown and its changes over the measuring time.
    const t0 = Date.now(), d0 = await date();
    let changes = 0, last = d0;
    while (Date.now() - t0 < 1000 * +secs) {
      await p.waitForTimeout(50);
      const d = await date();
      if (d !== last) { changes++; last = d; }
    }
    const t1 = Date.now(), d1 = await date(), shown = await stats();
    // Pause: the date shown before, and the date the page settles on after the worker's last frame.
    const before = await date(), tp = Date.now();
    await p.click('#play');
    let after = before, settled = 0;
    for (let i = 0; i < 100; i++) { await p.waitForTimeout(100); const d = await date(); if (d !== after) { after = d; settled = Date.now() - tp; } }
    const scale = await p.evaluate(() => 0);
    results.push({ view: name, monthsPerSecShown: +((d1 - d0) / ((t1 - t0) / 1000)).toFixed(1), dateUpdatesPerSec: +(changes / ((t1 - t0) / 1000)).toFixed(1), paceText: (/pace ([^c]*)/.exec(shown) || [])[1], lagMonthsAtPause: +(after - before).toFixed(1), settleMs: settled });
    await p.click('#play');
  }
  await (await p.$('#map')).screenshot({ path: out + '-50m.png' });
  console.log(JSON.stringify(results, null, 1));
  console.log('errors:', errs.filter(e => !/GPU stall|adapter/i.test(e)).length, errs.slice(0, 3).join(' | '));
  await b.close();
})();
