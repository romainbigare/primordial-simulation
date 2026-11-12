const { chromium } = require('/opt/node22/lib/node_modules/playwright');
(async () => {
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: 1200, height: 900 } });
  await p.goto(`http://localhost:8763/index.html?seed=1&size=32`);
  await p.waitForTimeout(8000);
  const speeds = await p.$$('#speeds button');
  const log = async tag => console.log(tag, ((await p.textContent('.stats')) || '').replace(/\s+/g, ' ').slice(0, 80));
  await log('start');
  await speeds[1].click();
  for (let i = 0; i < 8; i++) { await p.waitForTimeout(1000); await log('12mo ' + i); }
  await speeds[0].click();
  for (let i = 0; i < 4; i++) { await p.waitForTimeout(1000); await log('3mo ' + i); }
  await p.click('#play');
  for (let i = 0; i < 3; i++) { await p.waitForTimeout(1000); await log('paused? ' + i); }
  await b.close();
})();
