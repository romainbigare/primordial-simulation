// How many frames the page draws a second while the flow's signs slide, and what a frame costs: at views with and
// without a river. node anim.js PAGE PROFILE VIEWS   (VIEWS: name:metres:x:y,...)
const { chromium, devices } = require('/opt/node22/lib/node_modules/playwright');
const [page, profile, views] = process.argv.slice(2);
(async () => {
  const b = await chromium.launch();
  const ctx = await b.newContext(profile === 'phone' ? { ...devices['iPhone 13'] } : { viewport: { width: 1200, height: 800 }, deviceScaleFactor: 2 });
  const p = await ctx.newPage();
  await p.addInitScript(() => {
    const W = window.Worker; window.__workers = [];
    window.Worker = class extends W {
      constructor(...a) { super(...a); window.__workers.push(this); }
      postMessage(m, t) { if (m && m.cmd === 'play' && m.on) return; return super.postMessage(m, t); }
    };
  });
  const errs = [];
  p.on('console', m => { if (m.type() === 'error') errs.push(m.text()); });
  p.on('pageerror', e => errs.push('pageerror: ' + e.message));
  await p.goto(`http://localhost:8761/${page}/index.html?seed=1&size=22`);
  await p.waitForFunction(() => window.__map && window.__map.land && /date/.test(document.querySelector('.stats')?.textContent || ''), null, { timeout: 300000 });
  const date = () => p.evaluate(() => document.querySelector('.stats').textContent.match(/year \d+, day \d+/)?.[0]);
  const d0 = await date();
  await p.evaluate(() => window.__workers[0].postMessage({ cmd: 'rate', rate: -150 }));
  await p.waitForFunction(d => (document.querySelector('.stats').textContent.match(/year \d+, day \d+/)?.[0]) !== d, d0, { timeout: 600000 });
  await p.evaluate(() => { const mv = window.__map, d = mv.drawGpu; mv.__frames = 0; mv.drawGpu = function (...a) { mv.__frames++; return d.apply(this, a); }; });
  const out = [];
  for (const v of views.split(',')) {
    const [name, metres, x, y] = v.split(':');
    await p.evaluate(([m, x, y]) => {
      const mv = window.__map, cv = document.getElementById('cv'), s = (96 * mv.cellM) / +m;
      mv.cam = { s, ox: cv.clientWidth / 2 - +x * s, oy: cv.clientHeight / 2 - +y * s }; mv.fitted = false; mv.invalidate();
    }, [metres, x, y]);
    // The tiles in view made and faded in.
    for (let i = 0, calm = 0; i < 400 && calm < 4; i++) {
      const ok = await p.evaluate(() => { const t = window.__map.land.tiles; return t.rock !== null && t.toCome() === 0 && !t.fading(performance.now()); });
      calm = ok ? calm + 1 : 0;
      await p.waitForTimeout(500);
    }
    const count = async ms => { const f0 = await p.evaluate(() => window.__map.__frames); await p.waitForTimeout(ms); return (await p.evaluate(() => window.__map.__frames)) - f0; };
    const shown = await count(20000);
    const r = await p.evaluate(() => ({ fullMs: Math.round(window.__map.fullMs), scale: window.__map.scale }));
    // The same view with the page hidden.
    await p.evaluate(() => { Object.defineProperty(document, 'hidden', { value: true, configurable: true }); });
    await p.waitForTimeout(1500);
    const hidden = await count(10000);
    await p.evaluate(() => { Object.defineProperty(document, 'hidden', { value: false, configurable: true }); window.__map.invalidate(); });
    out.push({ view: name, frames_in_20s: shown, frames_in_10s_hidden: hidden, ...r });
  }
  console.log(JSON.stringify({ page, profile, out, errors: errs.filter(e => !/GPU stall|adapter/i.test(e)).slice(0, 3) }));
  await b.close();
})();
