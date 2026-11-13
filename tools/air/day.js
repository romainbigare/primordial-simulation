// Load the page (seed 1, size 22, Wide), run at the slowest speed (3 months a second) to two years before day DAY of year YEAR, then at the
// slowest speed in shares of the gap, pausing to read the date, until it is within a few days; shoot the fitted map, then
// zoom at square X, Y until the scale bar reads 200 m and shoot, then until it reads 10 m and shoot (FIT_ONLY 1: the
// fitted map alone). node day.js PORT OUT YEAR DAY X Y desk|phone FIT_ONLY
const { chromium, devices } = require('/opt/node22/lib/node_modules/playwright');
const [port, out, year, day, x, y, mode, fitOnly] = process.argv.slice(2);
(async () => {
  const b = await chromium.launch();
  const ctx = mode === 'phone' ? await b.newContext({ ...devices['iPhone 13'], hasTouch: false, isMobile: false }) : await b.newContext({ viewport: { width: 1200, height: 900 }, deviceScaleFactor: 2 });
  const p = await ctx.newPage();
  const errs = [];
  p.on('console', m => { if (m.type() === 'error') errs.push(m.text()); });
  p.on('pageerror', e => errs.push('pageerror: ' + e.message));
  await p.goto(`http://localhost:${port}/index.html?seed=1&size=22`);
  await p.waitForSelector('#speeds button');
  const date = async () => { const t = (await p.textContent('.stats')) || ''; const m = t.match(/year (\d+), day (\d+)/); return m ? [+m[1], +m[2]] : [0, 0]; };
  const at = async () => { const [yy, dd] = await date(); return yy * 360 + dd; };
  const playing = async () => p.$eval('#play', e => e.classList.contains('playing'));
  const setPlay = async on => { if ((await playing()) !== on) await p.click('#play'); };
  // The shown date lags the world while it plays; paused, the page catches up and shows the world's date.
  const settle = async () => { let same = 0, c = await at(); while (same < 2) { await p.waitForTimeout(4000); const d = await at(); same = d === c ? same + 1 : 0; c = d; } return c; };
  const target = +year * 360 + +day;
  const speeds = await p.$$('#speeds button');
  await speeds[0].click();
  const t0 = Date.now();
  let now = await at();
  // The world may run ahead of the shown date by a varying lag: play in bursts of at most 20 s and read the date paused.
  while (target - now > 720 && Date.now() - t0 < 2400000) {
    await setPlay(true);
    await p.waitForTimeout(Math.min(10000, ((target - now) / 90) * 1000 * 0.3));
    await setPlay(false);
    now = await settle();
  }
  // At the slowest speed (3 months, 90 days, a second) play a share of the gap at a time, and pause to read the date.
  while (target - now > 6 && Date.now() - t0 < 3000000) {
    const gap = target - now;
    await setPlay(true);
    await p.waitForTimeout(Math.max(150, (gap / 90) * 1000 * (gap > 60 ? 0.3 : 0.5)));
    await setPlay(false);
    now = await settle();
  }
  const shown = [Math.floor(now / 360), now % 360];
  await p.waitForTimeout(8000);
  const shot = await date();
  await (await p.$('#map')).screenshot({ path: out + '-fit.png' });
  if (fitOnly !== '1') {
    const box = await (await p.$('#cv')).boundingBox();
    // The live page shows no cell size; its fitted map's 10 km bar is 128 css px at 25.5 px a cell: a cell is 2 km.
    const cellM = 2000;
    const dpr = await p.evaluate(() => window.devicePixelRatio);
    const s0 = Math.min(box.width / 22, box.height / 22) * 0.94, ox = (box.width - 22 * s0) / 2, oy = (box.height - 22 * s0) / 2;
    const px = box.x + ox + +x * s0, py = box.y + oy + +y * s0;
    await p.mouse.move(px, py);
    let s = s0;
    for (const bar of [200, 10]) {
      // The bar shows the round length nearest 96 px: at s px a cell, bar m is 96 px when s = 96 cellM / bar.
      const want = 96 * cellM / bar;
      // The page zooms by e^(0.0015 deltaY), but a 600 wheel step of Playwright moves it by about e^(1 / the device
      // pixel ratio) (measured: e^0.5 at 2, about e^0.33 at 3).
      let delta = -Math.log(want / s) / (1 / dpr / 600);
      while (Math.abs(delta) > 1) { const d = Math.max(-600, Math.min(600, delta)); await p.mouse.wheel(0, d); delta -= d; await p.waitForTimeout(250); }
      s = want;
      await p.mouse.move(2, 2);
      await p.waitForTimeout(25000);
      await (await p.$('#map')).screenshot({ path: `${out}-${bar}m.png` });
      await p.mouse.move(px, py);
    }
  }
  console.log(out, 'paused at', shown, 'shot at', shot, 'errors:', errs.filter(e => !/GPU stall|adapter/i.test(e)).length, errs.slice(0, 2).join(' | '));
  await b.close();
})();
