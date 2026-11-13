// Load the page, play at full speed, and catch the message of the month the cover is sorted: when it shows, how long,
// and a picture of the page while it shows. node busy.js PORT OUT
const { chromium } = require('/opt/node22/lib/node_modules/playwright');
const [port, out] = process.argv.slice(2);
(async () => {
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: 1200, height: 900 } });
  const errs = [];
  p.on('console', m => { if (m.type() === 'error') errs.push(m.text()); });
  p.on('pageerror', e => errs.push('pageerror: ' + e.message));
  await p.goto(`http://localhost:${port}/index.html?seed=1&size=22`);
  const t0 = Date.now();
  let seen = null, gone = null, shot = false;
  while (Date.now() - t0 < 60000 && gone === null) {
    const t = await p.evaluate(() => { const m = document.getElementById('msg'); return m && !m.hidden ? m.textContent : ''; });
    if (t.includes('Sorting') && seen === null) seen = Date.now() - t0;
    if (seen !== null && !shot) { await p.screenshot({ path: out + '-busy.png' }); shot = true; }
    if (seen !== null && !t.includes('Sorting')) gone = Date.now() - t0;
    await p.waitForTimeout(100);
  }
  console.log('message shown at', seen, 'ms, gone at', gone, 'ms; errors:', errs.length, errs.slice(0, 2).join(' | '));
  await b.close();
})();
