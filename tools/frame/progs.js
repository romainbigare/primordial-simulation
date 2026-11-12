// The programs a build draws with at each view: node progs.js PAGE PROFILE VIEWS
const { chromium, devices } = require('/opt/node22/lib/node_modules/playwright');
const [page, profile, views] = process.argv.slice(2);
(async () => {
  const b = await chromium.launch();
  const ctx = await b.newContext(profile === 'phone' ? { ...devices['iPhone 13'] } : { viewport: { width: 1200, height: 800 }, deviceScaleFactor: 2 });
  const p = await ctx.newPage();
  const errs = [];
  p.on('console', m => { if (m.type() === 'error') errs.push(m.text()); });
  p.on('pageerror', e => errs.push('pageerror: ' + e.message));
  await p.goto(`http://localhost:8810/${page}?seed=1&size=22`);
  await p.waitForFunction(() => window.__map && window.__map.land && window.__map.gpu, null, { timeout: 300000 });
  await p.evaluate(() => {
    const g = window.__map.gpu, pass = g.pass.bind(g);
    window.__names = new Set();
    g.pass = (...a) => { const q = pass(...a), d = q.draw.bind(q); q.draw = (n, ...r) => { window.__names.add(n); return d(n, ...r); }; return q; };
  });
  for (const v of views.split(',')) {
    const [name, m] = v.split(':');
    const out = await p.evaluate(async m => {
      const mv = window.__map, cv = document.getElementById('cv');
      window.__names.clear();
      if (+m === 0) mv.fit(); else { const s = (96 * mv.cellM) / +m; mv.cam = { s, ox: cv.clientWidth / 2 - 11 * s, oy: cv.clientHeight / 2 - 9 * s }; mv.fitted = false; mv.invalidate(); }
      await new Promise(r => setTimeout(r, 1500));
      const e = mv.cellM / (mv.cam.s * devicePixelRatio * mv.scale);
      return { e: +e.toFixed(3), scale: mv.scale, names: [...window.__names] };
    }, m);
    console.log(name, JSON.stringify(out));
  }
  console.log('errors', errs.filter(e => !/GPU stall|adapter/i.test(e)).length, errs.slice(0, 2).join(' | '));
  await b.close();
})();
