s = open('drive.js').read()
a = s[s.index("  await speeds[2].click();\n  const t0 = Date.now();"):s.index("  console.log('reached year'")]
b = """  const t0 = Date.now();
  const W0 = 1200, H0 = 900, small = () => p.setViewportSize({ width: 240, height: 180 }), full = () => p.setViewportSize({ width: W0, height: H0 });
  // Play the world forward in the page itself until the date shown reaches `until` (years), the page drawn small so the
  // date shown keeps up with the world; then nudge (play for a moment, pause, let the date settle) up to it.
  const runTo = until => p.evaluate(async U => {
    const b = document.getElementById('play'), st = document.querySelector('.stats');
    const date = () => { const m = (st.textContent || '').match(/year (\\d+), day (\\d+)/); return m ? [+m[1], +m[2]] : [0, 0]; };
    const at = () => { const [y, d] = date(); return y + d / 365; };
    const wait = ms => new Promise(r => setTimeout(r, ms));
    const t = performance.now();
    if (b.classList.contains('playing')) b.click();
    await wait(1500);
    if (at() < U - 45 / 365) {
      b.click();
      while (at() < U - 45 / 365 && performance.now() - t < 900000) await wait(3);
      b.click();
      await wait(2000);
    }
    for (let k = 0; k < 12 && at() < U; k++) {
      const was = date().join();
      b.click();
      while (date().join() === was && performance.now() - t < 900000) await wait(3);
      b.click();
      await wait(1500);
    }
    return date();
  }, until);
  await small(); await p.waitForTimeout(1000);
  await speeds[2].click();
  await runTo(+yearArg - 1.5);
  await (await p.$$('#speeds button'))[0].click(); await p.waitForTimeout(1000);
  console.log('speed on:', await p.$eval('#speeds button.on', b => b.textContent));
  await full(); await p.waitForTimeout(2000);
"""
s = s.replace(a, b)
a = s[s.index("    // Step a month or so at a time (play, then pause a moment later in the page itself) until the day is in range."):s.index("    console.log(season, 'stopped at', got);")]
b = """    await small(); await p.waitForTimeout(1000);
    const got = await runTo(+yearArg + lo / 365);
    await full(); await p.dblclick('#cv'); await p.mouse.move(2, 2); await p.waitForTimeout(2000);
"""
s = s.replace(a, b)
open('drive.js', 'w').write(s)
