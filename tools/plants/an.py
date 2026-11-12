import sys, numpy as np
d = np.loadtxt(sys.argv[1])
cols = 'm i k t root kg stem perM2 height width cover crown wood leaf pigment moist frost fuel act frozen seed'.split()
C = {c: j for j, c in enumerate(cols)}
g = lambda c, x=d: x[:, C[c]]
print('months', np.unique(g('m')))
for m in range(12):
    x = d[g('m') == m]
    print('month', m, 'tground min/med/max %.1f %.1f %.1f' % (np.min(g('t', x)), np.median(g('t', x)), np.max(g('t', x))))
x = d[g('m') == 6]
for r in np.unique(g('root', x)):
    y = x[g('root', x) == r]
    q = lambda c: ' '.join('%.2f' % v for v in np.percentile(g(c, y), [10, 50, 90]))
    tree = (g('height', y) >= 3).mean()
    print('root %d n %d kg %s | h %s | w %s | h/w %s | wood %s | leaf %s | pig %s | moist %s | frost %s | seed %s | crown %s | tree %.2f | t-frost %s' % (r, len(y), q('kg'), q('height'), q('width'), ' '.join('%.2f' % v for v in np.percentile(g('height', y) / g('width', y), [10, 50, 90])), q('wood'), q('leaf'), q('pigment'), q('moist'), q('frost'), q('seed'), q('crown'), tree, ' '.join('%.1f' % v for v in np.percentile(g('t', y) - g('frost', y), [10, 50, 90]))))
