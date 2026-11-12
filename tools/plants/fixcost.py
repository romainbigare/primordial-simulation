s = open('cost.js').read()
old = "  while ((await yr()) < +yearArg) await p.waitForTimeout(500);\n  await p.click('#play');"
new = """  while ((await yr()) < +yearArg) await p.waitForTimeout(500);
  // The world's pace at the top speed with the island fitted: the stats' months a second, sampled over 12 s.
  await p.click('#speeds button[data-speed="Infinity"]');
  await p.waitForTimeout(3000);
  const paces = [];
  for (let k = 0; k < 12; k++) { const m = ((await p.textContent('.stats')) || '').match(/pace ([0-9.]+)/); if (m) paces.push(+m[1]); await p.waitForTimeout(1000); }
  console.log(port, 'pace', (paces.reduce((a, b) => a + b, 0) / Math.max(paces.length, 1)).toFixed(2), 'months/s');
  await p.evaluate(() => document.getElementById('play').click());"""
assert old in s
s = s.replace(old, new)
open('cost.js', 'w').write(s)
