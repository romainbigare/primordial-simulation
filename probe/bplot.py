import os, sys
import numpy as np, pandas as pd
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.patches import Patch
D = os.path.dirname(os.path.abspath(__file__))
os.chdir(D)
C = ['#2a78d6', '#eb6834', '#1baf7a', '#eda100', '#e87ba4', '#008300', '#6250d6', '#e34948']
plt.rcParams.update({'axes.spines.top': False, 'axes.spines.right': False, 'axes.grid': True, 'grid.color': '#e6e6e3', 'font.size': 10})
KIND = {0: 'small fast', 1: 'light-eaters', 2: 'big slow (trees)', 3: 'big, wet ground', 4: 'woody, dry ground', 5: 'tall fast'}
B = sys.argv[1] if len(sys.argv) > 1 else 'B12'


def load(m):
    d = pd.read_csv(f'run{m}/slots.csv')
    d = d[d.phase == 'post']
    s = d[d.slot >= 0].copy()
    s['area'] = s.ground * s.pshare
    return d, s


def per_patch(s):
    g = s.groupby(['year', 'cell', 'patch']).agg(kg=('kg', 'sum'), area=('area', 'first'), n=('slot', 'size')).reset_index()
    g['kgm2'] = g.kg / g.area
    return g


# Which patch is the clearing: the patch of share < 0.5 (its index can change only by fusion).
def clearing(s):
    return s[s.pshare < 0.5]


dA, sA = load('A')
dB, sB = load(B)
dN, sN = load('N')
gA = sA.groupby(['year', 'cell']).apply(lambda x: x.kg.sum() / x.ground.iloc[0]).rename('kgm2').reset_index()
gN = sN.groupby(['year', 'cell']).apply(lambda x: x.kg.sum() / x.ground.iloc[0]).rename('kgm2').reset_index()
pB = per_patch(sB)
pB['clear'] = pB.area / sB.groupby(['year', 'cell']).ground.first().reindex(list(zip(pB.year, pB.cell))).values < 0.5
cells = sorted(sB.cell.unique())
years = np.arange(60, 121)

# Picture 1: regrowth curves.
def band(df, col='kgm2'):
    q = df.groupby('year')[col].quantile([.25, .5, .75]).unstack()
    return q.reindex(years)

fig, ax = plt.subplots(figsize=(9, 5))
cl = pB[pB.clear]
# A clearing that fused into the rest counts as the rest from then.
fusedB = set(cells) - set(cl[cl.year == 120].cell)
for df, lab, col, ls in [(gN, 'no clearing (control), whole cell', '#8a8984', '--'), (gA, 'A, averaged: the cleared 10% = the cell mean', C[1], '-'),
                         (pB[~pB.clear], 'B, patches: the other 90%', C[0], ':'), (cl, 'B, patches: the cleared 10% patch', C[0], '-')]:
    q = band(df)
    ax.plot(q.index, q[0.5], color=col, ls=ls, lw=2, label=lab)
    ax.fill_between(q.index, q[0.25], q[0.75], color=col, alpha=0.15, lw=0)
ax.set_xlabel('year (the clearing is made at the end of year 60)')
ax.set_ylabel('cover, kg per m² (median of 20 cells, band: middle half)')
ax.set_title('Regrowth after clearing 10% of 20 forest cells\nA: the clearing is spread over the cell.  B: the clearing is its own patch.')
ax.legend(frameon=False, loc='center right')
ax.set_xlim(60, 120)
fig.tight_layout(); fig.savefig('p1_regrowth.png', dpi=130)

# Picture 2: kinds in the clearing patch (B) and in the same share of A, mean kg/m² over the 20 cells.
def by_kind(s, area_of):
    k = s.groupby(['year', 'cell', 'root']).kg.sum().reset_index()
    k['kgm2'] = k.kg / area_of(k)
    t = k.groupby(['year', 'root']).kgm2.sum().unstack(fill_value=0) / len(cells)
    return t.reindex(years, fill_value=0)

grA = sA.groupby(['year', 'cell']).ground.first()
clB = sB[sB.pshare < 0.5]
arB = clB.groupby(['year', 'cell']).area.first()
tB = by_kind(clB, lambda k: arB.reindex(list(zip(k.year, k.cell))).values)
tA = by_kind(sA, lambda k: grA.reindex(list(zip(k.year, k.cell))).values)
roots = sorted(set(tA.columns) | set(tB.columns))
fig, ax = plt.subplots(1, 2, figsize=(13, 5), sharey=True)
for a, t, title in [(ax[0], tB, 'B: the cleared patch'), (ax[1], tA, 'A: the same 10%, averaged = the whole cell')]:
    t = t.reindex(columns=roots, fill_value=0)
    a.stackplot(t.index, [t[r] for r in roots], colors=[C[r] for r in roots], labels=[f'kind {r}: {KIND.get(r, "?")}' for r in roots], edgecolor='white', linewidth=0.5)
    a.set_title(title); a.set_xlabel('year'); a.set_xlim(60, 120)
ax[0].set_ylabel('cover, kg per m² of that ground (mean of 20 cells)')
ax[1].legend(frameon=False, loc='upper left', fontsize=9)
fig.suptitle('Which kinds hold the clearing (succession)', fontsize=12)
fig.tight_layout(); fig.savefig('p2_kinds.png', dpi=130)

# Genes and heights of the kinds in the clearing at a few years (printed).
for y in [61, 65, 70, 80, 90, 120]:
    x = clB[clB.year == y]
    w = x.groupby('root').agg(kg=('kg', 'sum'), light=('light', 'mean'), leaf=('leaf', 'mean'), walls=('walls', 'mean'), h=('height', 'median'))
    w['share'] = w.kg / w.kg.sum()
    print(y, w.round(2).to_string())

# Picture 3: one cell drawn at years 61, 70, 90, 120.
t = pd.read_csv('t1/terrain.csv')
def top(x):
    # the canopy height of a patch: the tallest slot that holds at least a tenth of the patch's cover
    x = x[x.kg >= 0.1 * x.kg.sum()]
    return x.height.max() if len(x) else 0.0
cand = [c for c in cells if c not in fusedB and (t.cell == c).sum() == 256]
cell = cand[0]
e = t[t.cell == cell]
rows, cols = e.row.values, e.col.values
cr, cc = 7.5, 5.5
order = np.argsort((rows - cr) ** 2 + (cols - cc) ** 2, kind='stable')
fig, ax = plt.subplots(1, 4, figsize=(15, 4.2))
vmax = sB[sB.cell == cell].height.max()
for a, y in zip(ax, [61, 70, 90, 120]):
    x = sB[(sB.cell == cell) & (sB.year == y)]
    grid = np.zeros((16, 16))
    share = x[x.pshare < 0.5].pshare.max() if (x.pshare < 0.5).any() else 0.0
    nin = int(round(share * 256))
    inside = np.zeros(256, bool); inside[order[:nin]] = True
    hr = top(x[x.pshare >= 0.5]); hc = top(x[x.pshare < 0.5]) if nin else hr
    grid[rows, cols] = np.where(inside, hc, hr)
    im = a.imshow(grid, cmap='Greens', vmin=0, vmax=max(vmax, 1), origin='upper')
    if nin:
        m = np.zeros((16, 16)); m[rows[inside], cols[inside]] = 1
        a.contour(m, levels=[0.5], colors=C[1], linewidths=2)
    a.set_title(f'year {y}\nclearing {hc:.1f} m, rest {hr:.1f} m', fontsize=10)
    a.set_xticks([]); a.set_yticks([]); a.grid(False)
fig.colorbar(im, ax=ax, label='height of the canopy, m', shrink=0.8)
fig.suptitle(f'One cleared cell ({cell}, 2 km x 2 km) as its 16 x 16 samples, B. The patch has no place in the state:\nthe drawing puts it as a blob (orange line) of its share (10%) around one sample.', fontsize=10)
fig.savefig('p3_cell.png', dpi=130, bbox_inches='tight')
print('cell', cell, 'fused', fusedB)
