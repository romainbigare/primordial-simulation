import sys, numpy as np
names = ['round', 'cone', 'column', 'umbrella', 'tiered', 'lobed']
def forms(leaf, frost, moist, hw, seed, pig):
    thin = (leaf - 0.1) / 0.1
    tender = (frost - 270) / 2
    wet = (moist + 0.2) / 0.15
    slender = (hw - 4.1) / 0.2
    seedy = (seed - 0.05) / 0.012
    pg = (pig + 5.2) / 0.1
    dry = np.maximum(-(moist + 0.3) / 0.2, 0); wet = np.maximum(wet, 0)
    tender = (frost - 269) / 2
    s = np.stack([np.full_like(leaf, 0.7), -thin - tender - wet, slender + seedy - 0.6, 2 * dry - slender - 0.6, -thin + tender - wet - dry, wet + thin - 0.8])
    return s
for f in sys.argv[1:]:
    d = np.loadtxt(f)
    cols = 'm i k t root kg stem perM2 height width cover crown wood leaf pigment moist frost fuel act frozen seed'.split()
    C = {c: j for j, c in enumerate(cols)}
    x = d[(d[:, 0] == 6)]
    g = lambda c: x[:, C[c]]
    x = x[(g('height') >= 3) & (g('kg') > 0.05)]
    s = forms(g('leaf'), g('frost'), g('moist'), g('height') / g('width'), g('seed'), g('pigment'))
    a = s.argmax(0)
    srt = np.sort(s, 0); margin = srt[-1] - srt[-2]
    w = g('perM2')
    print(f, ' '.join('%s %.0f%%' % (n, 100 * w[a == k].sum() / w.sum()) for k, n in enumerate(names)), 'margin>0.5 %.0f%%' % (100*(margin > 0.5).mean()))
    for r in np.unique(g('root')):
        m = g('root') == r
        print('  root', int(r), ' '.join('%s %d' % (n, ((a == k) & m).sum()) for k, n in enumerate(names)))
