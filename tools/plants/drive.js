// Pictures of the live page by a plan: play the world to a year, then in each season (in calendar order) pause and
// shoot the plan's shots for it. node drive.js PORT OUTDIR SEED YEAR plan.json
// plan: { seasons: {name: [dayFrom, dayTo]}, shots: [{season, name, x, y, bars: [..], angled: [..], view}],
//         sweeps: [{season, name, x, y, steps}] }
const { chromium } = require('/opt/node22/lib/node_modules/playwright');
const fs = require('fs');
const [port, outDir, seed, yearArg, planFile] = process.argv.slice(2);
const plan = JSON.parse(fs.readFileSync(planFile, 'utf8'));
const SIZE = 22, DSF = process.env.PHONE ? 3 : 2, CELL_M = 2000;
fs.mkdirSync(outDir, { recursive: true });
(async () => {
  const b = await chromium.launch();
  const { devices } = require('/opt/node22/lib/node_modules/playwright');
  const p = await (process.env.PHONE ? (await b.newContext({ ...devices['iPhone 13'] })).newPage() : b.newPage({ viewport: { width: 1200, height: 900 }, deviceScaleFactor: DSF }));
  const errors = [];
  p.on('console', m => { if (m.type() === 'error') { errors.push(m.text()); console.log('console:', m.text()); } });
  p.on('pageerror', e => { errors.push(e.message); console.log('pageerror:', e.message); });
  await p.goto(`http://localhost:${port}/index.html?seed=${seed}&size=${SIZE}`);
  // The live world starts playing at once: pause it as soon as it does, so the target years lie ahead.
  await p.waitForFunction(() => document.getElementById('play')?.classList.contains('playing'), null, { polling: 5, timeout: 60000 });
  await p.evaluate(() => document.getElementById('play').click());
  await p.waitForTimeout(5000);
  const speeds = await p.$$('#speeds button');
  const playing = async () => p.$eval('#play', e => e.classList.contains('playing'));
  const yr = async () => parseFloat(((await p.textContent('#year')) || '').replace(/[^0-9.]/g, '')) || 0;
  const day = async () => +(((await p.textContent('.stats')) || '').match(/day (\d+)/) || [0, 0])[1];
  const stats = async () => ((await p.textContent('.stats')) || '').replace(/\s+/g, ' ');
  const t0 = Date.now();
  const W0 = process.env.PHONE ? 390 : 1200, H0 = process.env.PHONE ? 664 : 900, small = () => p.setViewportSize({ width: 240, height: 180 }), full = () => p.setViewportSize({ width: W0, height: H0 });
  // Play the world forward in the page itself until the date shown reaches `until` (years), the page drawn small so the
  // date shown keeps up with the world; then nudge (play for a moment, pause, let the date settle) up to it.
  const runTo = until => p.evaluate(async U => {
    const b = document.getElementById('play'), st = document.querySelector('.stats');
    const date = () => { const m = (st.textContent || '').match(/year (\d+), day (\d+)/); return m ? [+m[1], +m[2]] : [0, 0]; };
    // The year shown over the map (its fraction a month's), which follows the world more closely than the stats' date.
    const yearEl = document.getElementById('year');
    const at = () => parseFloat((yearEl.textContent || '').replace(/[^0-9.]/g, '')) || 0;
    const wait = ms => new Promise(r => setTimeout(r, ms));
    // Play until the year shown reaches `target`, then pause.
    const stopAt = async target => {
      if (at() >= target) return;
      if (!b.classList.contains('playing')) b.click();
      const t = performance.now();
      while (at() < target && performance.now() - t < 900000) await wait(5);
      if (b.classList.contains('playing')) b.click();
    };
    if (b.classList.contains('playing')) b.click();
    await wait(1500);
    await stopAt(U - 6 / 12);
    await wait(2500);
    // Nudge: play for a moment and pause, until the year shown reaches the target.
    for (let k = 0; k < 80 && at() < U; k++) {
      b.click(); await wait(40); if (b.classList.contains('playing')) b.click();
      await wait(1500);
    }
    return date();
  }, until);
  await small(); await p.waitForTimeout(1000);
  await p.waitForSelector("#speeds button[data-speed=\"36\"]"); await p.click("#speeds button[data-speed=\"36\"]"); await p.waitForTimeout(500);
  await runTo(+yearArg - 1.5);
  await p.click("#speeds button[data-speed=\"3\"]"); await p.waitForTimeout(1000);
  console.log('speed on:', await p.$eval('#speeds button.on', b => b.textContent));
  await full(); await p.waitForTimeout(2000);
  console.log('reached year', await yr(), 'in', ((Date.now() - t0) / 1000).toFixed(0), 's');
  const settle = async () => { for (let k = 0; k < 300; k++) { const m = (await stats()).match(/(\d+) to come/); if (!m || +m[1] === 0) break; await p.waitForTimeout(100); } await p.waitForTimeout(1500); };
  const style = async s => { await p.click(`#life button[data-life="${s}"]`); await p.waitForTimeout(300); };
  const view = async v => { await p.click(`#views button[data-view="${v}"]`); await p.waitForTimeout(300); };
  const box = await (await p.$('#cv')).boundingBox();
  const W = box.width, H = box.height, fit = Math.min(W / SIZE, H / SIZE) * 0.94, ox = (W - SIZE * fit) / 2, oy = (H - SIZE * fit) / 2;
  // Centre square (tx, ty) and zoom so the scale bar reads `bar` m (0: fitted).
  const go = async (tx, ty, bar) => {
    await p.dblclick('#cv'); await p.waitForTimeout(400);
    if (!bar) { await p.mouse.move(2, 2); return; }
    const x = box.x + ox + tx * fit, y = box.y + oy + ty * fit;
    await p.mouse.move(x, y); await p.mouse.down();
    for (let i = 1; i <= 10; i++) { await p.mouse.move(x + (box.x + W / 2 - x) * i / 10, y + (box.y + H / 2 - y) * i / 10); await p.waitForTimeout(20); }
    await p.mouse.up();
    const total = DSF * Math.log(96 * CELL_M / bar / fit) / 0.0015, n = Math.max(1, Math.ceil(Math.abs(total) / 1000));
    await p.mouse.move(box.x + W / 2, box.y + H / 2);
    for (let k = 0; k < n; k++) { await p.mouse.wheel(0, -total / n); await p.waitForTimeout(120); }
    await p.mouse.move(2, 2);
  };
  // Shoot once the picture holds still: the map is drawn at fewer pixels while frames are slow, then sharpened tile by
  // tile once the view rests.
  const shoot = async name => {
    await settle();
    const el = await p.$('#map');
    // The sharp picture replaces the soft one at once when its last tile is drawn: wait for that change (if it comes).
    let last = await el.screenshot();
    for (let k = 0; k < +(process.env.SHARP_S || 60) / 3; k++) { await p.waitForTimeout(3000); const now = await el.screenshot(); if (!now.equals(last)) { await p.waitForTimeout(1500); last = await el.screenshot(); break; } }
    require('fs').writeFileSync(`${outDir}/${name}.png`, last);
  };
  const order = Object.entries(plan.seasons).sort((a, b) => a[1][0] - b[1][0]);
  for (const [season, [lo, hi]] of order) {
    await small(); await p.waitForTimeout(1000);
    const got = await runTo(+yearArg + Math.round((lo + 13) / 30.4) / 12 - 0.006);
    await full(); await p.dblclick('#cv'); await p.mouse.move(2, 2); await p.waitForTimeout(2000);
    console.log(season, 'stopped at', got);
    await p.waitForTimeout(1500);
    console.log(season, 'paused at', await stats());
    await view('earth'); await style('above');
    for (const s of plan.shots.filter(s => s.season === season)) {
      for (const bar of s.bars) {
        await go(s.x, s.y, bar);
        await shoot(`${s.name}-${season}-${bar || 'fit'}`);
        if ((s.angled || []).includes(bar)) { await style('angled'); await shoot(`${s.name}-${season}-${bar}-angled`); await style('above'); }
        for (const v of s.views || []) { await view(v); await shoot(`${s.name}-${season}-${bar || 'fit'}-${v}`); await view('earth'); }
      }
    }
    for (const sw of (plan.sweeps || []).filter(s => s.season === season)) {
      const far = 96 * CELL_M / fit, near = 5;
      for (let k = 0; k < sw.steps; k++) {
        const bar = far * (near / far) ** (k / (sw.steps - 1));
        await go(sw.x, sw.y, k === 0 ? 0 : bar);
        await shoot(`${sw.name}-${season}-step${String(k).padStart(2, '0')}`);
      }
    }
  }
  console.log('errors', errors.length);
  await b.close();
})();
