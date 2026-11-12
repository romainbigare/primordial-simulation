# Patch a built page (dist/index.html) for the bench: expose the map view, let the world's worker step an exact number
# of months on command ('rate' with a negative rate), and optionally hold the map at the screen's own pixels.
# python3 hook.py IN OUT [--full]
import sys
src, out = sys.argv[1], sys.argv[2]
full = '--full' in sys.argv
h = open(src).read()
def sub(a, b, n=1):
    global h
    c = h.count(a)
    assert c == n, (a, c)
    h = h.replace(a, b)
sub("this.over.className = 'over';", "window.__map = this; this.over.className = 'over';")
import re
h, k = re.subn(r"else if \(m\.cmd === 'rate'\)\s*rate = m\.rate;",
    "else if (m.cmd === 'rate') { if (m.rate < 0) { for (let i = 0; i < -m.rate; i++) core.step_month(); postFrame('frame'); } else rate = m.rate; }", h)
assert k == 1, k
if full:
    sub("FRAME_BUDGET_MS = 16", "FRAME_BUDGET_MS = 1e9")
open(out, 'w').write(h)
