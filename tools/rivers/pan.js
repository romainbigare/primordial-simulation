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
// The CPU time of the whole tree of processes under this one (the browser and all its children), ms.
const treeCpu = () => { const all = [String(process.pid)]; for (let i = 0; i < all.length; i++) all.push(...kids(all[i])); return all.slice(1).reduce((s, d) => { try { return s + cpuOf(d); } catch { return s; } }, 0); };
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
  // The signs of the flow stand still (their slide at 0) unless FLOW is set: pictures and timings repeat.
  if (!process.env.FLOW) await p.evaluate(() => { const m = window.__map; if (m.flowSoon) m.flowSoon = () => {}; if (m.land.slid) m.land.slid = () => 0; });
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

  // A pan while tiles load: at each view, jump half a screen east four times; a picture as soon as each jump is drawn
  // (coarse tiles standing in, the map drawn at whatever scale it keeps), and one once all has settled.
  for (const v of views.split(',')) {
    const [name, metres, x, y] = v.split(':');
    await p.evaluate(([m, x, y]) => {
      const mv = window.__map, cv = document.getElementById('cv'), W = cv.clientWidth, H = cv.clientHeight;
      const s = (96 * mv.cellM) / +m;
      mv.cam = { s, ox: W / 2 - +x * s, oy: H / 2 - +y * s };
      mv.fitted = false; mv.invalidate();
    }, [metres, x, y]);
    await settle();
    for (let k = 1; k <= 4; k++) {
      const st = await p.evaluate(async () => {
        const mv = window.__map, cv = document.getElementById('cv');
        mv.cam.ox -= cv.clientWidth * 0.5; mv.cam.oy -= cv.clientHeight * 0.15; mv.fitted = false; mv.invalidate();
        await new Promise(r => requestAnimationFrame(() => requestAnimationFrame(() => r())));
        return { scale: +mv.scale.toFixed(2), toCome: mv.land.tiles.toCome(), depth: mv.land.tiles.depth };
      });
      await (await p.$('#cv')).screenshot({ path: `${out}-${name}-pan${k}.png` });
      console.log(name, 'pan', k, JSON.stringify(st));
    }
    await settle();
    await p.evaluate(async () => { const mv = window.__map; mv.scale = 1; mv.invalidate(); await new Promise(r => requestAnimationFrame(() => r())); await mv.gpu.done(); });
    await p.waitForTimeout(300);
    await (await p.$('#cv')).screenshot({ path: `${out}-${name}-rest.png` });
  }
  console.log(JSON.stringify({ errors: errs.filter(e => !/GPU stall|adapter/i.test(e)).slice(0, 3) }));
  await b.close();
})();
