// Load the page, play at full speed for a while, pause, then take pictures: the whole map and a closer view of a place.
// node paused.js PORT OUT_PREFIX SECONDS X Y STEPS
const { chromium } = require('/opt/node22/lib/node_modules/playwright');
const [port, out, seconds, x, y, steps] = process.argv.slice(2);
(async () => {
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: 1200, height: 900 }, deviceScaleFactor: 2 });
  const errs = [];
  p.on('console', m => { if (m.type() === 'error') errs.push(m.text()); });
  p.on('pageerror', e => errs.push('pageerror: ' + e.message));
  await p.goto(`http://localhost:${port}/index.html?seed=1&size=22`);
  await p.waitForTimeout(6000);
  const speeds = await p.$$('#speeds button');
  await speeds[speeds.length - 1].click();
  await p.waitForTimeout(1000 * +seconds);
  await p.click('#play');
  await p.waitForTimeout(6000);
  await (await p.$('#map')).screenshot({ path: out + '-fit.png' });
  const box = await (await p.$('#cv')).boundingBox();
  const s = Math.min(box.width / 22, box.height / 22) * 0.94, ox = (box.width - 22 * s) / 2, oy = (box.height - 22 * s) / 2;
  await p.mouse.move(box.x + ox + +x * s, box.y + oy + +y * s);
  for (let i = 0; i < +steps; i++) { await p.mouse.wheel(0, -600); await p.waitForTimeout(300); }
  await p.waitForTimeout(12000);
  await p.mouse.move(2, 2);
  await (await p.$('#map')).screenshot({ path: out + '-near.png' });
  console.log('errors:', errs.filter(e => !/GPU stall|adapter/i.test(e)).length, errs.slice(0, 2).join(' | '));
  console.log(((await p.textContent('.stats')) || '').replace(/\s+/g, ' ').slice(0, 300));
  await b.close();
})();
