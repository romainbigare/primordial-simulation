// The page in every view: the live world (Earth, True, a layer, both styles) and a tape. node gate.js PORT OUTDIR
const { chromium } = require('/opt/node22/lib/node_modules/playwright');
const [port, out] = process.argv.slice(2);
require('fs').mkdirSync(out, { recursive: true });
(async () => {
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: 1200, height: 900 }, deviceScaleFactor: 1 });
  const errors = [];
  p.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
  p.on('pageerror', e => errors.push(e.message));
  await p.goto(`http://localhost:${port}/index.html?seed=1&size=22`);
  await p.waitForTimeout(7000);
  const box = await (await p.$('#cv')).boundingBox();
  await p.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  for (let k = 0; k < 6; k++) { await p.mouse.wheel(0, -900); await p.waitForTimeout(300); }
  const shot = async n => { await p.waitForTimeout(3000); await p.screenshot({ path: `${out}/${n}.png`, timeout: 120000 }); };
  for (const style of ['above', 'angled']) {
    await p.click(`#life button[data-life="${style}"]`);
    await p.click('#views button[data-view="earth"]'); await shot(`live-earth-${style}`);
    await p.click('#views button[data-view="true"]'); await shot(`live-true-${style}`);
    await p.click('#views button[data-view="earth"]');
    await (await p.$$('#layers button'))[0].click(); await shot(`live-layer-${style}`);
    await (await p.$$('#layers button'))[0].click();
  }
  await p.goto(`http://localhost:${port}/index.html?f=world.ewf.gz`);
  await p.waitForTimeout(7000);
  await shot('tape-fit');
  await p.mouse.move(box.x + box.width * 0.45, box.y + box.height * 0.45);
  for (let k = 0; k < 6; k++) { await p.mouse.wheel(0, -600); await p.waitForTimeout(200); }
  await shot('tape-zoom');
  await p.click('#life button[data-life="angled"]'); await shot('tape-zoom-angled');
  console.log('errors', errors.length, errors.slice(0, 5));
  await b.close();
})();
