s = open('drive.js').read()
a = s[s.index("    // Play at the slowest speed to about two months before the range"):s.index("    console.log(season, 'stopped at', got);")]
b = '''    // Play at the slowest speed to about three months before the range, pausing in the page itself; then nudge (play
    // for 150 ms, pause, wait for the date shown to settle) until the date shown is in range.
    const got = await p.evaluate(([lo, hi, Y]) => new Promise(done => {
      const b = document.getElementById('play'), st = document.querySelector('.stats');
      const date = () => { const m = (st.textContent || '').match(/year (\\d+), day (\\d+)/); return m ? [+m[1], +m[2]] : [0, 0]; };
      const at = () => { const [y, d] = date(); return y + d / 365; };
      const lo365 = Y + lo / 365, t0 = performance.now();
      const wait = ms => new Promise(r => setTimeout(r, ms));
      (async () => {
        if (at() < lo365 - 100 / 365) {
          b.click();
          while (at() < lo365 - 100 / 365 && performance.now() - t0 < 600000) await wait(3);
          b.click();
        }
        await wait(2500);
        for (let k = 0; k < 40 && at() < lo365; k++) {
          const was = date().join();
          b.click(); await wait(150); b.click();
          for (let w = 0; w < 40 && date().join() === was; w++) await wait(100);
          await wait(1500);
        }
        done(date());
      })();
    }), [lo, hi, +yearArg]);
'''
s = s.replace(a, b)
old = "  await speeds[0].click(); await p.waitForTimeout(3000);"
assert old in s
s = s.replace(old, old + "\n  if (!/of 3 months/.test(await stats())) { await (await p.$$('#speeds button'))[0].click(); await p.waitForTimeout(2000); }")
open('drive.js', 'w').write(s)
