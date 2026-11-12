const { chromium } = require('/opt/node22/lib/node_modules/playwright');
(async () => {
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: 1100, height: 800 }, deviceScaleFactor: 1 });
  const errs = [];
  p.on('console', m => { if (m.type() === 'error') errs.push(m.text()); });
  p.on('pageerror', e => errs.push('pageerror: ' + e.message));
  await p.goto('http://localhost:8111/index.html?seed=1&size=22');
  await p.waitForTimeout(25000);
  const box = await (await p.$('#cv')).boundingBox();
  await p.mouse.move(box.x + box.width * 0.45, box.y + box.height * 0.6);
  for (let i = 0; i < 9; i++) { await p.mouse.wheel(0, -300); await p.waitForTimeout(250); }
  await p.waitForTimeout(9000);
  await p.screenshot({ path: process.argv[2] });
  console.log('errors:', errs.length, errs.slice(0, 3).join(' | '));
  console.log(((await p.textContent('.stats')) || '').replace(/\s+/g, ' ').slice(0, 300));
  await b.close();
})();
