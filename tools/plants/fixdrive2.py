s = open('drive.js').read()
old = "  if (!/of 3 months/.test(await stats())) { await (await p.$$('#speeds button'))[0].click(); await p.waitForTimeout(2000); }"
new = """  // The slowest speed on, by its button (the buttons are made anew when the world starts).
  for (let k = 0; k < 10 && !(await p.$eval('#speeds button', b => b.classList.contains('on'))); k++) { await p.click('#speeds button'); await p.waitForTimeout(1000); }
  console.log('speed on:', await p.$eval('#speeds button.on', b => b.textContent));"""
assert old in s
s = s.replace(old, new)
s = s.replace("if (at() < lo365 - 200 / 365) {", "if (at() < lo365 - 1) {")
s = s.replace("while (at() < lo365 - 200 / 365 && ", "while (at() < lo365 - 1 && ")
s = s.replace("b.click(); await wait(60); b.click();", "b.click(); await wait(40); b.click();")
s = s.replace("          await wait(1500);\n        }\n        done(date());", "          await wait(2500);\n        }\n        done(date());")
open('drive.js', 'w').write(s)
