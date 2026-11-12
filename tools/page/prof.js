// Profile the page's main thread while the world plays at max speed at a given view (metres for ~96 px; 0: fitted).
// node prof.js PORT METRES SECONDS
const { chromium } = require('/opt/node22/lib/node_modules/playwright');
const [port, metres, secs] = process.argv.slice(2);
(async () => {
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: 1200, height: 900 } });
  await p.goto(`http://localhost:${port}/index.html?seed=1&size=22`);
  await p.waitForFunction(() => /date/.test(document.querySelector('.stats')?.textContent || ''), null, { timeout: 120000 });
  await p.waitForTimeout(3000);
  const speeds = await p.$$('#speeds button');
  await speeds[speeds.length - 1].click();
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
  await p.waitForTimeout(4000);
  // Long tasks of the main thread.
  await p.evaluate(() => { window.__long = []; new PerformanceObserver(l => l.getEntries().forEach(e => window.__long.push(e.duration))).observe({ type: 'longtask', buffered: false }); });
  const c = await p.context().newCDPSession(p);
  await c.send('Profiler.enable');
  await c.send('Profiler.setSamplingInterval', { interval: 1000 });
  await c.send('Profiler.start');
  await p.waitForTimeout(1000 * +secs);
  const { profile } = await c.send('Profiler.stop');
  const long = await p.evaluate(() => window.__long);
  // Self time by function, and total time by function (a node's self plus its children's).
  const byId = new Map(profile.nodes.map(n => [n.id, n]));
  const self = new Map();
  const dt = profile.timeDeltas; const total = profile.endTime - profile.startTime;
  const count = new Map();
  profile.samples.forEach((id, k) => count.set(id, (count.get(id) || 0) + (dt[k + 1] || 0)));
  const parent = new Map(); profile.nodes.forEach(n => (n.children || []).forEach(ch => parent.set(ch, n.id)));
  const incl = new Map();
  for (const [id, t] of count) {
    const n = byId.get(id); const key = `${n.callFrame.functionName || '(anon)'}:${n.callFrame.lineNumber}`;
    self.set(key, (self.get(key) || 0) + t);
    const seen = new Set();
    for (let x = id; x !== undefined; x = parent.get(x)) {
      const m = byId.get(x); const k2 = `${m.callFrame.functionName || '(anon)'}:${m.callFrame.lineNumber}`;
      if (!seen.has(k2)) { seen.add(k2); incl.set(k2, (incl.get(k2) || 0) + t); }
    }
  }
  const top = m => [...m].sort((a, b) => b[1] - a[1]).slice(0, 25).map(([k, t]) => `${(t / 1000).toFixed(0).padStart(7)} ms ${(100 * t / total).toFixed(1).padStart(5)}%  ${k}`).join('\n');
  console.log(`total ${(total / 1000).toFixed(0)} ms; long tasks: ${long.length}, sum ${long.reduce((a, x) => a + x, 0).toFixed(0)} ms, max ${Math.max(0, ...long).toFixed(0)} ms`);
  console.log('SELF\n' + top(self));
  console.log('INCLUSIVE\n' + top(incl));
  await b.close();
})();
