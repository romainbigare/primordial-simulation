const { chromium } = require('/opt/node22/lib/node_modules/playwright');
const [port] = process.argv.slice(2);
(async () => {
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: 240, height: 180 }, deviceScaleFactor: 2 });
  p.on('console', m => console.log('page:', m.text()));
  await p.goto(`http://localhost:${port}/index.html?seed=1&size=22`);
  await p.waitForTimeout(5000);
  await p.click('#speeds button[data-speed="36"]');
  const r = await p.evaluate(async U => {
    const b = document.getElementById('play'), yearEl = document.getElementById('year');
    const at = () => parseFloat((yearEl.textContent || '').replace(/[^0-9.]/g, '')) || 0;
    const wait = ms => new Promise(r => setTimeout(r, ms));
    console.log('start', at(), b.classList.contains('playing'));
    if (b.classList.contains('playing')) b.click();
    await wait(1500);
    console.log('paused?', at(), b.classList.contains('playing'));
    b.click();
    const t = performance.now();
    while (at() < U && performance.now() - t < 60000) { await wait(500); console.log('at', at(), b.classList.contains('playing'), yearEl.textContent); }
    b.click();
    return at();
  }, 20);
  console.log('result', r);
  await b.close();
})();
