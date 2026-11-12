// The page on a phone profile: play, pause, shoot the fitted map and a pinch-like zoom (wheel) at a place.
// node phone.js PORT OUT_PREFIX SECONDS FX FY STEPS   (FX, FY: shares of the map's box)
const { chromium, devices } = require('/opt/node22/lib/node_modules/playwright');
const [port, out, seconds, fx, fy, steps] = process.argv.slice(2);
(async () => {
  const b = await chromium.launch();
  const ctx = await b.newContext({ ...devices['iPhone 13'], hasTouch: false, isMobile: false });
  const p = await ctx.newPage();
  const errs = [];
  p.on('console', m => { if (m.type() === 'error') errs.push(m.text()); });
  p.on('pageerror', e => errs.push('pageerror: ' + e.message));
  await p.goto(`http://localhost:${port}/index.html?seed=1&size=22`);
  await p.waitForTimeout(8000);
  const speeds = await p.$$('#speeds button');
  await speeds[speeds.length - 1].click();
  await p.waitForTimeout(1000 * +seconds);
  await p.click('#play');
  await p.waitForTimeout(8000);
  await (await p.$('#map')).screenshot({ path: out + '-fit.png' });
  const box = await (await p.$('#cv')).boundingBox();
  await p.mouse.move(box.x + box.width * +fx, box.y + box.height * +fy);
  for (let i = 0; i < +steps; i++) { await p.mouse.wheel(0, -600); await p.waitForTimeout(400); }
  await p.waitForTimeout(25000);
  await p.mouse.move(2, 2);
  await (await p.$('#map')).screenshot({ path: out + '-near.png' });
  console.log('errors:', errs.filter(e => !/GPU stall|adapter/i.test(e)).length, errs.slice(0, 2).join(' | '));
  console.log(((await p.textContent('.stats')) || '').replace(/\s+/g, ' ').slice(0, 300));
  await b.close();
})();
