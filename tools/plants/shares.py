import sys, numpy as np
names = ['round', 'cone', 'column', 'umbrella', 'tiered', 'lobed']
W = np.array([[0.7, 0, 0, 0, 0, 0, 0], [0, -1, -1, -1, 0, 0, 0], [-0.6, 0, 0, 0, 0, 1, 1], [-0.6, 0, 0, 0, 2, -1, 0], [0, -1, 1, -1, -1, 0, 0], [-0.8, 1, 0, 1, 0, 0, 0]])
sm = lambda a, b, v: (lambda t: t * t * (3 - 2 * t))(np.clip((v - a) / (b - a), 0, 1))
cols = 'm i k t root kg stem perM2 height width cover crown wood leaf pigment moist frost fuel act frozen seed'.split()
C = {c: j for j, c in enumerate(cols)}
for f in sys.argv[1:]:
    d = np.loadtxt(f)
    x = d[d[:, 0] == 6]
    g = lambda c: x[:, C[c]]
    x = x[(g('height') >= 3) & (g('kg') > 0)]
    leaf, frost, moist, hw, seed = g('leaf'), g('frost'), g('moist'), g('height') / g('width'), g('seed')
    X = np.stack([np.ones_like(leaf), (leaf - 0.1) / 0.1, (frost - 269) / 2, np.maximum((moist + 0.2) / 0.15, 0), np.maximum(-(moist + 0.3) / 0.2, 0), (hw - 4.1) / 0.2, (seed - 0.05) / 0.012])
    S = W @ X
    a = S.argmax(0)
    srt = np.sort(S, 0)
    sure = sm(0, 0.5, srt[-1] - srt[-2])
    w = g('perM2') * 22 * 0 + g('perM2')
    shed = sm(0.05, 0.2, leaf)
    pig = g('pigment')
    reach = sm(-5.32, -5.12, pig)
    print(f, 'trees (plants a m2 summed over cells):', round(w.sum(), 1))
    print('  forms:', ' '.join('%s %.0f%%' % (n, 100 * w[a == k].sum() / w.sum()) for k, n in enumerate(names)), '| drawn surely (>0.5): %.0f%%' % (100 * w[sure > 0.5].sum() / w.sum()))
    print('  shed (weighted mean) %.0f%%, shed > 0.5: %.0f%%' % (100 * (w * shed).sum() / w.sum(), 100 * w[shed > 0.5].sum() / w.sum()))
    m = shed > 0.5
    print('  of shedders, turn toward: yellow (reach<0.33) %.0f%%, orange %.0f%%, red (>0.67) %.0f%%' % tuple(100 * w[m & c].sum() / w[m].sum() for c in [reach < 0.33, (reach >= 0.33) & (reach <= 0.67), reach > 0.67]))
