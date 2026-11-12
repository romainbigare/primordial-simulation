// The page on a phone (iPhone 13): time to the first banded world, a second seed (the planet's air reused), and
// pictures of the page with its climate control. node phone.js PORT OUT_PREFIX
const pw = require('/opt/node22/lib/node_modules/playwright');
const [port, out] = process.argv.slice(2);
(async () => {
  const b = await pw.chromium.launch();
  const ctx = await b.newContext({ ...pw.devices['iPhone 13'] });
  const p = await ctx.newPage();
  const errs = [];
  p.on('console', m => { if (m.type() === 'error') errs.push(m.text()); });
  p.on('pageerror', e => errs.push('pageerror: ' + e.message));
  const shown = () => p.waitForFunction(() => /year \d+/.test(document.querySelector('.stats')?.textContent ?? ''), null, { timeout: 300000 });
  let t = Date.now();
  await p.goto(`http://localhost:${port}/index.html`);
  await p.waitForTimeout(300);
  console.log('message:', await p.textContent('#msg'));
  await shown();
  console.log('first world (wide band) after', ((Date.now() - t) / 1000).toFixed(1), 's; band choice', await p.$eval('#band', e => e.value));
  await p.waitForTimeout(15000);
  await p.screenshot({ path: `${out}-wide.png` });
  // A new seed with the same band: the planet's air is reused.
  await p.fill('#seed', '2');
  t = Date.now();
  await p.click('#live button[type=submit]');
  await p.waitForTimeout(200);
  console.log('message:', await p.textContent('#msg'));
  await p.waitForFunction(() => /seed\s*2/.test(document.querySelector('#meta')?.textContent ?? ''), null, { timeout: 300000 });
  console.log('second world (seed 2) after', ((Date.now() - t) / 1000).toFixed(1), 's');
  // Today's island.
  await p.selectOption('#band', '');
  t = Date.now();
  await p.click('#live button[type=submit]');
  await p.waitForFunction(() => /42\.3/.test(document.querySelector('#meta')?.textContent ?? ''), null, { timeout: 300000 });
  console.log('today world after', ((Date.now() - t) / 1000).toFixed(1), 's');
  await p.waitForTimeout(15000);
  await p.screenshot({ path: `${out}-today.png` });
  console.log('options:', await p.$$eval('#band option', o => o.map(x => x.textContent).join(' | ')));
  console.log('errors:', errs.filter(e => !/GPU stall|adapter/i.test(e)).length, errs.slice(0, 3).join(' | '));
  await b.close();
})();
