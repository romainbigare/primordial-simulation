const { chromium, devices } = require('/opt/node22/lib/node_modules/playwright');
(async () => {
  const b = await chromium.launch();
  const ctx = await b.newContext({ ...devices['iPhone 13'] });
  const p = await ctx.newPage();
  await p.addInitScript(() => { const W = window.Worker; window.__workers = []; window.Worker = class extends W { constructor(...a) { super(...a); window.__workers.push(this); } }; });
  p.on('console', m => console.log('console', m.type(), m.text().slice(0, 200)));
  p.on('pageerror', e => console.log('pageerror: ' + e.message));
  const t0 = Date.now();
  await p.goto(`http://localhost:${process.argv[3] || 8811}/${process.argv[2]}?seed=1&size=22`);
  for (let i = 0; i < 40; i++) {
    await p.waitForTimeout(3000);
    const s = await p.evaluate(() => ({ map: !!window.__map, land: !!(window.__map && window.__map.land), stats: document.querySelector('.stats')?.textContent?.slice(0, 120), workers: window.__workers.length, msg: document.getElementById('msg').textContent }));
    console.log((Date.now() - t0) / 1000, JSON.stringify(s));
    if (s.land && /date/.test(s.stats || '')) break;
  }
  await b.close();
})();
