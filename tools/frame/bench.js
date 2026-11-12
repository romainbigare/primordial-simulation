// The map's frame bench. A page built with hook.py (--full: always drawn at the screen's own pixels), paused world
// stepped an exact number of months, views by scale bar at places in squares.
// node bench.js PORT PROFILE SEED MONTHS MODE OUT VIEWS
//   PROFILE: phone (iPhone 13) | desk (1200 x 800 css at ratio 2)
//   MODE: time (full-frame ms over a fixed pan path, FRAMES each) | shot (a picture of each view) | page (the page's
//         own fullMs while panning, build without --full)
//   VIEWS: name:metres:x:y[:look] separated by commas (metres 0: fitted; x, y in squares; look earth|true|layer name)
const { chromium, devices } = require('/opt/node22/lib/node_modules/playwright');
const fs = require('fs');
const [port, profile, seed, months, mode, out, views] = process.argv.slice(2);
// The GPU process of the browser `pid` (its child with --type=gpu-process), and a process's CPU time, ms.
const kids = pid => fs.readdirSync('/proc').filter(d => /^\d+$/.test(d)).filter(d => { try { return fs.readFileSync(`/proc/${d}/stat`, 'utf8').split(') ')[1].split(' ')[1] === String(pid); } catch { return false; } });
const gpuProcess = pid => { const all = [String(pid)]; for (let i = 0; i < all.length; i++) all.push(...kids(all[i])); return all.find(d => { try { return fs.readFileSync(`/proc/${d}/cmdline`, 'utf8').includes('--type=gpu-process'); } catch { return false; } }); };
const cpuOf = pid => { const f = fs.readFileSync(`/proc/${pid}/stat`, 'utf8').split(') ')[1].split(' '); return (+f[11] + +f[12]) * 10; };
const FRAMES = +(process.env.FRAMES || 3), PAGE = process.env.PAGE || 'index.html';
(async () => {
  const b = await chromium.launch();
  const ctxOpts = profile === 'phone' ? { ...devices['iPhone 13'] } : { viewport: { width: 1200, height: 800 }, deviceScaleFactor: 2 };
  const ctx = await b.newContext(ctxOpts);
  const p = await ctx.newPage();
  await p.addInitScript(() => {
    const W = window.Worker; window.__workers = [];
    // The world never plays by itself: the bench steps it an exact number of months.
    window.Worker = class extends W {
      constructor(...a) { super(...a); window.__workers.push(this); }
      postMessage(m, t) { if (m && m.cmd === 'play' && m.on) return; return super.postMessage(m, t); }
    };
  });
  const errs = [];
  p.on('console', m => { if (m.type() === 'error') errs.push(m.text()); });
  p.on('pageerror', e => errs.push('pageerror: ' + e.message));
  await p.goto(`http://localhost:${port}/${PAGE}?seed=${seed}&size=22`);
  await p.addStyleTag({ content: '.over, #msg, .fit, .stats { visibility: hidden !important }' });
  await p.waitForFunction(() => window.__map && window.__map.land && /date/.test(document.querySelector('.stats')?.textContent || ''), null, { timeout: 300000 });
  const date = () => p.evaluate(() => document.querySelector('.stats').textContent.match(/year \d+, day \d+/)?.[0]);
  const d0 = await date();
  if (+months > 0) {
    await p.evaluate(n => window.__workers[0].postMessage({ cmd: 'rate', rate: -n }), +months);
    await p.waitForFunction(d => (document.querySelector('.stats').textContent.match(/year \d+, day \d+/)?.[0]) !== d, d0, { timeout: 600000 });
  }
  const when = await date();
  const settle = async () => {
    // The tiles in view made and faded in, the GPU idle, for a second.
    let calm = 0;
    for (let i = 0; i < 600 && calm < 4; i++) {
      const ok = await p.evaluate(() => { const t = window.__map.land.tiles; return t.rock !== null && t.toCome() === 0 && !t.fading(performance.now()) && !window.__map.timing && !window.__map.raf; });
      calm = ok ? calm + 1 : 0;
      await p.waitForTimeout(250);
    }
    await p.evaluate(() => window.__map.gpu.done());
  };
  const results = [];
  for (const v of views.split(',')) {
    const [name, metres, x, y, look] = v.split(':');
    if (look) {
      if (look === 'earth' || look === 'true') await p.click(`#views button[data-view="${look}"]`);
      else if (look === 'angled' || look === 'above') await p.click(`#life button[data-life="${look}"]`);
      else await p.evaluate(l => { const b = [...document.querySelectorAll('#layers button')].find(b => b.textContent.trim().toLowerCase() === l.toLowerCase()); b.click(); }, look.replace(/_/g, ' '));
    }
    await p.evaluate(() => { if (window.__view) window.__map.land.tiles.view = window.__view; });
    await p.evaluate(([m, x, y]) => {
      const mv = window.__map, cv = document.getElementById('cv'), W = cv.clientWidth, H = cv.clientHeight;
      if (+m === 0) { mv.fit(); return; }
      const s = (96 * mv.cellM) / +m;
      mv.cam = { s, ox: W / 2 - +x * s, oy: H / 2 - +y * s };
      mv.fitted = false; mv.invalidate();
    }, [metres, x, y]);
    await settle();
    if (mode === 'shot') {
      // The picture at the screen's own pixels.
      await p.evaluate(async () => { const mv = window.__map; mv.scale = 1; mv.invalidate(); await new Promise(r => requestAnimationFrame(() => r())); await mv.gpu.done(); });
      await p.waitForTimeout(300);
      await (await p.$('#cv')).screenshot({ path: `${out}-${name}.png` });
      results.push({ view: name });
      continue;
    }
    // A fixed pan path: FRAMES frames, each 3 css px east and 2 south of the last.
    // While timing, the view asks for no tiles and the frames drawn are counted.
    await p.evaluate(() => {
      const mv = window.__map, t = mv.land.tiles;
      window.__view = window.__view || t.view; t.view = () => {};
      if (!mv.__counted) { const d = mv.drawGpu; mv.__frames = 0; mv.drawGpu = function (...a) { mv.__frames++; return d.apply(this, a); }; mv.__counted = true; }
    });
    const ms = [];
    const gpuPid = gpuProcess(process.pid);
    for (let k = 0; k < FRAMES; k++) {
      const c0 = cpuOf(gpuPid);
      const t = await p.evaluate(async () => {
        const mv = window.__map;
        mv.cam.ox += 3; mv.cam.oy += 2; mv.fitted = false;
        const before = mv.fullMs, f0 = mv.__frames;
        mv.scale = 1;
        mv.invalidate();
        await new Promise(r => requestAnimationFrame(() => r()));
        for (let i = 0; i < 2000 && (mv.timing || mv.fullMs === before); i++) await new Promise(r => setTimeout(r, 20));
        await mv.gpu.done();
        return { full: mv.fullMs, scale: mv.scale, frames: mv.__frames - f0 };
      });
      t.cpu = cpuOf(gpuPid) - c0;
      ms.push(t);
      if (mode === 'page') await p.waitForTimeout(50);
    }
    const tiles = await p.evaluate(() => [window.__map.land.tiles.depth, window.__map.land.tiles.count()]);
    results.push({ view: name, ms: ms.map(t => Math.round(t.full)), cpu: ms.map(t => t.cpu), frames: ms.map(t => t.frames), scale: ms.map(t => +t.scale.toFixed(2)), depth: tiles[0], kept: tiles[1] });
  }
  const load = fs.readFileSync('/proc/loadavg', 'utf8').split(' ').slice(0, 3).join(' ');
  console.log(JSON.stringify({ port, profile, seed, months, when, mode, load, results, errors: errs.filter(e => !/GPU stall|adapter/i.test(e)).slice(0, 3) }));
  await b.close();
})();
