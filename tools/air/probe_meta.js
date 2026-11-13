const { chromium } = require('/opt/node22/lib/node_modules/playwright');
(async () => {
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: 1200, height: 900 } });
  await p.goto('http://localhost:8832/index.html?seed=1&size=22');
  await p.waitForSelector('#speeds button');
  await p.waitForTimeout(3000);
  console.log(JSON.stringify(await p.evaluate(() => document.getElementById('meta').textContent)));
  const box = await (await p.$('#cv')).boundingBox(); console.log(JSON.stringify(box));
  await b.close();
})();
