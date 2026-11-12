// Load the page, play to about year 30, then take pictures: the whole map, a lake, and a close view of its shore.
const { chromium } = require('/opt/node22/lib/node_modules/playwright');
(async () => {
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: 1100, height: 800 }, deviceScaleFactor: 1 });
  const errs = [];
  p.on('console', m => { if (m.type() === 'error') errs.push(m.text()); });
  p.on('pageerror', e => errs.push('pageerror: ' + e.message));
  await p.goto('http://localhost:8111/index.html?seed=1&size=22');
  await p.waitForTimeout(6000);
  const speeds = await p.$$('#speeds button');
  await speeds[speeds.length - 1].click();
  await p.waitForTimeout(45000);
  await speeds[1].click();
  await p.waitForTimeout(3000);
  await p.screenshot({ path: process.argv[2] + '-fit.png' });
  const box = await (await p.$('#cv')).boundingBox();
  const s = Math.min(box.width / 22, box.height / 22) * 0.94, ox = (box.width - 22 * s) / 2, oy = (box.height - 22 * s) / 2;
  await p.mouse.move(box.x + ox + 14.6 * s, box.y + oy + 11.6 * s);
  for (let i = 0; i < 6; i++) { await p.mouse.wheel(0, -300); await p.waitForTimeout(250); }
  await p.waitForTimeout(9000);
  await p.screenshot({ path: process.argv[2] + '-lake.png' });
  for (let i = 0; i < 5; i++) { await p.mouse.wheel(0, -300); await p.waitForTimeout(250); }
  await p.waitForTimeout(9000);
  await p.screenshot({ path: process.argv[2] + '-near.png' });
  console.log('errors:', errs.length, errs.slice(0, 3).join(' | '));
  console.log(((await p.textContent('.stats')) || '').replace(/\s+/g, ' ').slice(0, 300));
  await b.close();
})();
