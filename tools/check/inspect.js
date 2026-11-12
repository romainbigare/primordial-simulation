// Load the page, play, pause, click a place on the fitted map and print what the page says about it.
// node inspect.js PORT SECONDS FX FY   (FX, FY: the click as shares of the map's box, 0..1)
const { chromium } = require('/opt/node22/lib/node_modules/playwright');
const [port, seconds, fx, fy] = process.argv.slice(2);
(async () => {
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: 1200, height: 900 }, deviceScaleFactor: 1 });
  await p.goto(`http://localhost:${port}/index.html?seed=1&size=22`);
  await p.waitForTimeout(6000);
  const speeds = await p.$$('#speeds button');
  await speeds[speeds.length - 1].click();
  await p.waitForTimeout(1000 * +seconds);
  await p.click('#play');
  await p.waitForTimeout(4000);
  const before = await p.evaluate(() => document.body.innerText);
  const box = await (await p.$('#cv')).boundingBox();
  await p.mouse.click(box.x + box.width * +fx, box.y + box.height * +fy);
  await p.waitForTimeout(3000);
  const after = await p.evaluate(() => document.body.innerText);
  const a = new Set(before.split('\n'));
  console.log(after.split('\n').filter(l => !a.has(l) && l.trim()).slice(0, 60).join('\n'));
  await b.close();
})();
