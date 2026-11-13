// Load the page (seed 1, size 22, Wide), play at 36 months a second until a year before YEAR, then slowly to day DAY of YEAR,
// pause, shoot the fitted map, then zoom at the square X, Y by STEPS1 wheel steps and shoot, then by STEPS2 more and
// shoot. node at2.js PORT OUT YEAR DAY X Y STEPS1 STEPS2 desk|phone
const { chromium, devices } = require('/opt/node22/lib/node_modules/playwright');
const [port, out, year, day, x, y, steps1, steps2, mode] = process.argv.slice(2);
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
  const speeds = await p.$$('#speeds button');
  await speeds[0].click();
  const t0 = Date.now();
  while ((await date())[0] < +year - 1 && Date.now() - t0 < 1500000) await p.waitForTimeout(100);
  await speeds[0].click();
  await p.waitForFunction(([yr, dy]) => { const t = document.querySelector('.stats')?.textContent || ''; const m = t.match(/year (\d+), day (\d+)/); return m && (+m[1] > yr || (+m[1] === yr && +m[2] >= dy)); }, [+year, +day], { polling: 10, timeout: 1800000 });
  const at = (await date()).join(', ');
  await p.click('#play');
  await p.waitForTimeout(8000);
  console.log('pause clicked at year, day', at);
  console.log('shot at year, day', (await date()).join(', '));
  await (await p.$('#map')).screenshot({ path: out + '-fit.png' });
  const box = await (await p.$('#cv')).boundingBox();
  const s = Math.min(box.width / 22, box.height / 22) * 0.94, ox = (box.width - 22 * s) / 2, oy = (box.height - 22 * s) / 2;
  await p.mouse.move(box.x + ox + +x * s, box.y + oy + +y * s);
  for (let i = 0; i < +steps1; i++) { await p.mouse.wheel(0, -600); await p.waitForTimeout(300); }
  await p.waitForTimeout(15000);
  await (await p.$('#map')).screenshot({ path: out + '-200m.png' });
  for (let i = 0; i < +steps2; i++) { await p.mouse.wheel(0, -600); await p.waitForTimeout(300); }
  await p.waitForTimeout(15000);
  await p.mouse.move(2, 2);
  await (await p.$('#map')).screenshot({ path: out + '-10m.png' });
  console.log('errors:', errs.filter(e => !/GPU stall|adapter/i.test(e)).length, errs.slice(0, 2).join(' | '));
  console.log(((await p.textContent('.stats')) || '').replace(/\s+/g, ' ').slice(0, 200));
  await b.close();
})();
