// node page_time.js PORT QUERY: seconds until the page's world shows a date, and console errors.
const { chromium } = require('/opt/node22/lib/node_modules/playwright');
const [port, query] = process.argv.slice(2);
(async () => {
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: 1200, height: 900 } });
  const errs = [];
  p.on('console', m => { if (m.type() === 'error') errs.push(m.text()); });
  p.on('pageerror', e => errs.push('pageerror: ' + e.message));
  const t = Date.now();
  await p.goto(`http://localhost:${port}/index.html?${query}`);
  await p.waitForFunction(() => /year \d+/.test(document.querySelector('.stats')?.textContent ?? ''), null, { timeout: 300000 });
  console.log(query, 'world shown after', ((Date.now() - t) / 1000).toFixed(1), 's; errors', errs.filter(e => !/GPU stall|adapter/i.test(e)).length, errs.slice(0, 2).join(' | '));
  await b.close();
})();
