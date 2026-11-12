s = open('drive.js').read()
a = s[s.index("    const stopAt = target => new Promise(res => {"):s.index("    if (b.classList.contains('playing')) b.click();\n    await wait(1500);")]
b = """    const stopAt = target => new Promise(res => {
      if (at() >= target) return res();
      let done = false;
      const check = () => { if (!done && at() >= target) { done = true; ob.disconnect(); clearInterval(iv); if (b.classList.contains('playing')) b.click(); res(); } };
      const ob = new MutationObserver(check);
      ob.observe(document.body, { childList: true, subtree: true, characterData: true });
      const iv = setInterval(check, 2);
      if (!b.classList.contains('playing')) b.click();
    });
"""
s = s.replace(a, b)
open('drive.js', 'w').write(s)
