const { chromium } = require('/opt/node22/lib/node_modules/playwright');
const [port] = process.argv.slice(2);
(async () => {
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: 240, height: 180 }, deviceScaleFactor: 2 });
  await p.goto(`http://localhost:${port}/index.html?seed=1&size=22`);
  const st = async () => ((await p.textContent('.stats')) || '').replace(/\s+/g, ' ').slice(0, 70) + ' | ' + (await p.textContent('#year')) + ' | on=' + (await p.$eval('#speeds button.on', x => x.dataset.speed).catch(() => '?')) + ' playing=' + (await p.$eval('#play', e => e.classList.contains('playing')));
  for (let k = 0; k < 6; k++) { console.log(k, await st()); await p.waitForTimeout(1000); }
  await p.click('#speeds button[data-speed="36"]');
  for (let k = 0; k < 6; k++) { console.log('36:', k, await st()); await p.waitForTimeout(1000); }
  await b.close();
})();
