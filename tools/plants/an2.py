import sys, numpy as np
d = np.loadtxt(sys.argv[1])
cols = 'm i k t root kg stem perM2 height width cover crown wood leaf pigment moist frost fuel act frozen seed'.split()
C = {c: j for j, c in enumerate(cols)}
x = d[(d[:, 0] == 6)]
g = lambda c: x[:, C[c]]
tree = (g('height') >= 3) & (g('kg') > 0.05)
x = x[tree]
w = g('perM2') * g('cover') * 0 + g('perM2')  # trees a m2 (cells ~equal area)
print('trees n', len(x), 'weight', w.sum())
for c in ['leaf', 'frost', 'moist', 'pigment', 'seed', 'wood', 'height', 'width', 'crown']:
    print(c, ' '.join('%.3f' % v for v in np.percentile(g(c), [5, 25, 50, 75, 95])))
print('h/w', ' '.join('%.3f' % v for v in np.percentile(g('height') / g('width'), [5, 25, 50, 75, 95])))
