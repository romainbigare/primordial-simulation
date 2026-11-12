const { chromium } = require('/opt/node22/lib/node_modules/playwright');
const [port] = process.argv.slice(2);
(async () => {
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: 1200, height: 900 }, deviceScaleFactor: 1 });
  await p.goto(`http://localhost:${port}/index.html?seed=1&size=22`);
  await p.waitForTimeout(5000);
  console.log('stats:', JSON.stringify(await p.textContent('.stats')));
  console.log('year:', JSON.stringify(await p.textContent('#year')));
  await b.close();
})();
