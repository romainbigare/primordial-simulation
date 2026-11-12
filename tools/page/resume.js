// Play, pause, play again several times at max speed: the world must go on each time (no frame left unanswered).
// node resume.js PORT
const { chromium } = require('/opt/node22/lib/node_modules/playwright');
const port = process.argv[2];
const months = t => { const m = /year (\d+), day (\d+)/.exec(t || ''); return m ? (+m[1] * 360 + +m[2] - 1) / 30 : NaN; };
(async () => {
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: 1200, height: 900 } });
  const errs = [];
  p.on('pageerror', e => errs.push(e.message));
  p.on('console', m => { if (m.type() === 'error') errs.push(m.text()); });
  await p.goto(`http://localhost:${port}/index.html?seed=1&size=22`);
  await p.waitForFunction(() => /date/.test(document.querySelector('.stats')?.textContent || ''), null, { timeout: 120000 });
  const speeds = await p.$$('#speeds button');
  await speeds[speeds.length - 1].click();
  const out = [];
  for (let k = 0; k < 6; k++) {
    await p.waitForTimeout(1500 + 300 * k);
    await p.click('#play');
    await p.waitForTimeout(800);
    const a = months(await p.textContent('.stats'));
    await p.click('#play');
    await p.waitForTimeout(1500);
    const c = months(await p.textContent('.stats'));
    out.push([a, c]);
  }
  console.log(JSON.stringify(out), 'errors', errs.length, errs.slice(0, 2));
  await b.close();
})();
