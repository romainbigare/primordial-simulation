s = open('drive.js').read()
a = s[s.index("    // Pause the moment the date shown reaches `target`"):s.index("    if (b.classList.contains('playing')) b.click();\n    await wait(1500);")]
b = """    // Play until the year shown reaches `target`, then pause.
    const stopAt = async target => {
      if (at() >= target) return;
      if (!b.classList.contains('playing')) b.click();
      const t = performance.now();
      while (at() < target && performance.now() - t < 900000) await wait(5);
      if (b.classList.contains('playing')) b.click();
    };
"""
s = s.replace(a, b)
open('drive.js', 'w').write(s)
