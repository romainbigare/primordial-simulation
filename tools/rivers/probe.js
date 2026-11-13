// The discharge of the samples about places of a page at a month: node probe.js PAGE MONTHS name:x:y,...
const { chromium } = require('/opt/node22/lib/node_modules/playwright');
const [page, months, places] = process.argv.slice(2);
(async () => {
  const b = await chromium.launch();
  const p = await (await b.newContext({ viewport: { width: 800, height: 600 } })).newPage();
  await p.addInitScript(() => {
    const W = window.Worker; window.__workers = [];
    window.Worker = class extends W {
      constructor(...a) { super(...a); window.__workers.push(this); }
      postMessage(m, t) { if (m && m.cmd === 'play' && m.on) return; return super.postMessage(m, t); }
    };
  });
  await p.goto(`http://localhost:8761/${page}/index.html?seed=1&size=22`);
  await p.waitForFunction(() => window.__map && window.__map.land && /date/.test(document.querySelector('.stats')?.textContent || ''), null, { timeout: 300000 });
  const date = () => p.evaluate(() => document.querySelector('.stats').textContent.match(/year \d+, day \d+/)?.[0]);
  const d0 = await date();
  await p.evaluate(n => window.__workers[0].postMessage({ cmd: 'rate', rate: -n }), +months);
  await p.waitForFunction(d => (document.querySelector('.stats').textContent.match(/year \d+, day \d+/)?.[0]) !== d, d0, { timeout: 600000 });
  await p.waitForTimeout(3000);
  const out = await p.evaluate(places => {
    const land = window.__map.land, s = land.samples, d = land.discharge, m = land.tiles.map;
    const unhalf = h => { const e = (h >> 10) & 31, f = h & 1023, sg = h & 0x8000 ? -1 : 1; return sg * (e === 0 ? f * 2 ** -24 : e === 31 ? Infinity : (1 + f / 1024) * 2 ** (e - 15)); };
    const per = s.across / m.w, res = { across: s.across, down: s.down, per, cell_m: m.cell_m };
    const all = Array.from(d, unhalf).filter(v => v > 0).sort((a, b) => a - b);
    res.q = { n: all.length, p50: all[all.length >> 1], p90: all[Math.floor(all.length * 0.9)], p99: all[Math.floor(all.length * 0.99)], max: all[all.length - 1] };
    const rock = land.tiles.rock;
    for (const pl of places.split(',')) {
      const [name, x, y] = pl.split(':'); const c = Math.floor(+x * per), r = Math.floor(+y * per); const near = [];
      for (let dr = -3; dr <= 3; dr++) for (let dc = -3; dc <= 3; dc++) near.push([unhalf(d[(r + dr) * s.across + c + dc]), c + dc, r + dr, rock ? unhalf(rock[4 * ((r + dr) * s.across + c + dc) + 1]) : -1]);
      near.sort((a, b) => b[0] - a[0]); res[name] = near.slice(0, 6).map(v => [+v[0].toFixed(3), v[1], v[2], +v[3].toFixed(3)]);
    }
    return res;
  }, places);
  console.log(JSON.stringify(out));
  await b.close();
})();
