# Undo an edit script of exact replacements (one that defines `edit(p, pairs)` and calls it): python3 revedit.py SCRIPT
import re, sys
src = open(sys.argv[1]).read()
i = src.index('def edit(p, pairs):')
j = src.index("open(p, 'w').write(t)", i) + len("open(p, 'w').write(t)")
def edit(p, pairs):
    t = open(p).read()
    for a, b in reversed(pairs):
        assert t.count(b) == 1, (p, b[:70], t.count(b))
        t = t.replace(b, a)
    open(p, 'w').write(t)
exec(src[:i] + src[j:], {'edit': edit})
print('undone', sys.argv[1])
