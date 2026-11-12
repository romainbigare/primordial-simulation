// usage: node shot.js IN.html OUT.png
const { chromium } = require('/opt/node22/lib/node_modules/playwright');
(async () => {
  const [inp, out] = process.argv.slice(2);
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: 1400, height: 800 } });
  await p.goto('file://' + inp);
  await p.screenshot({ path: out, fullPage: true });
  await b.close();
})();
