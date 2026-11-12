s = open('drive.js').read()
a = s[s.index("    const t = performance.now();\n    if (b.classList.contains('playing')) b.click();"):s.index("    return date();\n  }, until);")]
b = """    // Pause the moment the date shown reaches `target`, as the page shows it (a watcher of the stats' text).
    const stopAt = target => new Promise(res => {
      if (at() >= target) return res();
      const ob = new MutationObserver(() => { if (at() >= target) { ob.disconnect(); if (b.classList.contains('playing')) b.click(); res(); } });
      ob.observe(st, { childList: true, subtree: true, characterData: true });
      if (!b.classList.contains('playing')) b.click();
    });
    if (b.classList.contains('playing')) b.click();
    await wait(1500);
    await stopAt(U - 45 / 365);
    await wait(2500);
    await stopAt(U);
    await wait(2500);
"""
s = s.replace(a, b)
open('drive.js', 'w').write(s)
