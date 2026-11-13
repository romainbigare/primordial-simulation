// Load the page (seed 1, size 22, Wide), play at full speed until the year YEAR, pause, shoot the fitted map, then zoom
// at the square X, Y by STEPS1 wheel steps and shoot, then by STEPS2 more and shoot.
// node at.js PORT OUT YEAR X Y STEPS1 STEPS2 desk|phone
const { chromium, devices } = require('/opt/node22/lib/node_modules/playwright');
const [port, out, year, x, y, steps1, steps2, mode] = process.argv.slice(2);
(async () => {
  const b = await chromium.launch();
  const ctx = mode === 'phone' ? await b.newContext({ ...devices['iPhone 13'], hasTouch: false, isMobile: false }) : await b.newContext({ viewport: { width: 1200, height: 900 }, deviceScaleFactor: 2 });
  const p = await ctx.newPage();
  const errs = [];
  p.on('console', m => { if (m.type() === 'error') errs.push(m.text()); });
  p.on('pageerror', e => errs.push('pageerror: ' + e.message));
  await p.goto(`http://localhost:${port}/index.html?seed=1&size=22`);
  await p.waitForSelector('#speeds button');
  const yearNow = async () => { const t = (await p.textContent('.stats')) || ''; const m = t.match(/year (\d+)/); return m ? +m[1] : 0; };
  const speeds = await p.$$('#speeds button');
  if (+year > 3) await speeds[speeds.length - 1].click();
  const t0 = Date.now();
  while ((await yearNow()) < +year && Date.now() - t0 < 900000) await p.waitForTimeout(50);
  await p.click('#play');
  await p.waitForTimeout(8000);
  await (await p.$('#map')).screenshot({ path: out + '-fit.png' });
  const box = await (await p.$('#cv')).boundingBox();
  const s = Math.min(box.width / 22, box.height / 22) * 0.94, ox = (box.width - 22 * s) / 2, oy = (box.height - 22 * s) / 2;
  await p.mouse.move(box.x + ox + +x * s, box.y + oy + +y * s);
  for (let i = 0; i < +steps1; i++) { await p.mouse.wheel(0, -600); await p.waitForTimeout(300); }
  await p.waitForTimeout(15000);
  await (await p.$('#map')).screenshot({ path: out + '-mid.png' });
  for (let i = 0; i < +steps2; i++) { await p.mouse.wheel(0, -600); await p.waitForTimeout(300); }
  await p.waitForTimeout(15000);
  await p.mouse.move(2, 2);
  await (await p.$('#map')).screenshot({ path: out + '-near.png' });
  console.log('errors:', errs.filter(e => !/GPU stall|adapter/i.test(e)).length, errs.slice(0, 2).join(' | '));
  console.log(((await p.textContent('.stats')) || '').replace(/\s+/g, ' ').slice(0, 200));
  await b.close();
})();
