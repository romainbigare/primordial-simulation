// Load the live page for an island (with or without a band), Earth view, play, pause at a winter and a summer date of
// the same year, and shoot the map each time.
// node page_shots.js PORT OUT_PREFIX QUERY YEAR WINTER_DAY SUMMER_DAY
const { chromium } = require('/opt/node22/lib/node_modules/playwright');
const [port, out, query, year, winterDay, summerDay] = process.argv.slice(2);
(async () => {
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: 1200, height: 900 } });
  const errs = [];
  p.on('console', m => { if (m.type() === 'error') errs.push(m.text()); });
  p.on('pageerror', e => errs.push('pageerror: ' + e.message));
  await p.goto(`http://localhost:${port}/index.html?${query}`);
  await p.waitForFunction(() => /year \d+/.test(document.querySelector('.stats')?.textContent ?? ''), null, { timeout: 120000 });
  await (await p.$$('#speeds button'))[0].click();
  const earth = await p.$('#views button[data-view="earth"]');
  if (earth) await earth.click();
  const speeds = await p.$$('#speeds button');
  console.log('speeds:', (await Promise.all(speeds.map(s => s.textContent()))).join(' '));
  const date = async () => {
    const t = ((await p.textContent('.stats')) || '').replace(/\s+/g, ' ');
    const m = t.match(/year (\d+), day (\d+)/);
    return m ? (+m[1]) * 360 + (+m[2]) : 0;
  };
  const advancing = async () => { const a = await date(); await p.waitForTimeout(1500); return (await date()) !== a; };
  let on = null;
  const setPlaying = async want => {
    if (on === null) on = await advancing();
    if (on !== want) { await p.click('#play'); on = want; }
  };
  for (const [name, day] of [['winter', +winterDay], ['summer', +summerDay]]) {
    const target = +year * 360 + day;
    // Fast to two months before, then the slowest speed to the day.
    await speeds[0].click();
    await setPlaying(true);
    // Pause from inside the page as soon as the date shown reaches the target.
    await p.evaluate(t => new Promise(res => {
      const tick = () => {
        const m = (document.querySelector('.stats')?.textContent ?? '').match(/year (\d+), day (\d+)/);
        if (m && +m[1] * 360 + +m[2] >= t) { document.querySelector('#play').click(); res(); } else setTimeout(tick, 5);
      };
      tick();
    }), target);
    on = false;
    await p.waitForTimeout(6000);
    const d = await date();
    await (await p.$('#map')).screenshot({ path: `${out}-${name}.png` });
    console.log(name, 'year', Math.floor(d / 360), 'day', d % 360);
  }
  console.log('meta:', ((await p.textContent('#meta')) || '').replace(/\s+/g, ' '));
  console.log('errors:', errs.filter(e => !/GPU stall|adapter/i.test(e)).length, errs.slice(0, 3).join(' | '));
  await b.close();
})();
