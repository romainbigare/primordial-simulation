import sys, numpy as np
from collections import defaultdict
seed = sys.argv[1]
d = np.loadtxt(f'kinds{seed}.txt')
sq = [l.split() for l in open(f'sq{seed}.txt') if l.startswith('S')]
cell = {(int(a[1]), int(a[2])): int(a[3]) for a in sq}
tree = defaultdict(float); herb = defaultdict(float); t0 = {}
for r in d:
    m, i = int(r[0]), int(r[1])
    if m == 0: t0[i] = r[3]
    if m != 6: continue
    if r[8] >= 3: tree[i] += r[5]
    else: herb[i] += r[5]
G = np.zeros((22, 22)); H = np.zeros((22, 22)); T = np.full((22, 22), np.nan)
for (x, y), i in cell.items():
    G[y, x] = tree[i]; H[y, x] = herb[i]; T[y, x] = t0.get(i, np.nan)
np.set_printoptions(linewidth=250)
print('tree kg'); print(np.round(G).astype(int))
print('herb kg x10'); print(np.round(H * 10).astype(int))
print('winter t - 270'); print(np.nan_to_num(np.round(T - 270), nan=-9).astype(int))
