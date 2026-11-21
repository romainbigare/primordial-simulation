import os
import numpy as np, pandas as pd
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
D = os.path.dirname(os.path.abspath(__file__))
os.chdir(D)
C = ['#2a78d6', '#eb6834', '#1baf7a', '#eda100', '#e87ba4', '#008300', '#6250d6', '#e34948']
plt.rcParams.update({'axes.spines.top': False, 'axes.spines.right': False, 'axes.grid': True, 'grid.color': '#e6e6e3', 'font.size': 10})

land = pd.read_csv('runA/land.csv')
landc = set(land[land.land > 0].cell)
nland = len(landc)
f = pd.read_csv('t1/fire.csv')
per_year = f.groupby('year').cell.nunique().reindex(range(1, 101), fill_value=0) / nland
fig, ax = plt.subplots(1, 2, figsize=(11, 4))
ax[0].bar(per_year.index, per_year.values * 100, color=C[1], width=0.8)
ax[0].set_title(f'Share of land cells that burn each year\n(mean {per_year.mean()*100:.2f}% a year, {nland} land cells)')
ax[0].set_xlabel('year'); ax[0].set_ylabel('% of land cells burnt')
ax[1].hist(f.share, bins=np.linspace(0, 1, 41), color=C[0])
ax[1].set_title(f'Burnt share f of a cell, each burn ({len(f)} burns)\nmedian f = {f.share.median():.2f}')
ax[1].set_xlabel('share of the cell burnt in one burn (f)'); ax[1].set_ylabel('burns')
fig.tight_layout(); fig.savefig('t1_fire.png', dpi=130)
print('fire', per_year.mean(), f.share.median(), f.share.quantile(.1), f.cell.nunique(), nland)

t = pd.read_csv('t1/terrain.csv')
t = t[t.cell.isin(landc)]
n = t.groupby('cell').size()
t = t[t.cell.isin(n[n == 256].index)]
g = t.groupby('cell')
st = pd.DataFrame({'twi_sd': g.twi.std(), 'twi_rng': g.twi.quantile(.9) - g.twi.quantile(.1),
                   'loose_lo': g.loose.quantile(.1), 'loose_hi': g.loose.quantile(.9), 'loose_med': g.loose.median(),
                   'h_rng': g.h.max() - g.h.min()})
st['loose_ratio'] = st.loose_hi / st.loose_lo.clip(lower=0.005)
print(st.describe().T[['25%', '50%', '75%']])
fig, ax = plt.subplots(1, 3, figsize=(14, 4.2))
ax[0].hist(st.twi_rng, bins=30, color=C[0])
ax[0].set_title(f'Wetness index inside a cell:\n90th minus 10th percentile of its 256 samples\n(median {st.twi_rng.median():.1f}; the whole map spans {t.twi.quantile(.9)-t.twi.quantile(.1):.1f})')
ax[0].set_xlabel('spread of wetness index (ln m)'); ax[0].set_ylabel('land cells')
ax[1].hist(np.log10(st.loose_ratio), bins=30, color=C[2])
ax[1].set_title(f'Loose ground depth inside a cell:\n90th over 10th percentile of its samples\n(median x{st.loose_ratio.median():.1f})')
ax[1].set_xlabel('log10 of (deep end / shallow end)'); ax[1].set_ylabel('land cells')
# one example cell: the median spread
ex = (st.twi_rng - st.twi_rng.median()).abs().idxmin()
e = t[t.cell == ex]
grid = np.full((16, 16), np.nan)
grid[e.row.values, e.col.values] = e.twi.values
im = ax[2].imshow(grid, cmap='Blues', origin='upper')
ax[2].set_title(f'One cell ({ex}), its 16 x 16 samples:\nwetness index (a cell is 2 km wide)')
ax[2].set_xticks([]); ax[2].set_yticks([]); ax[2].grid(False)
plt.colorbar(im, ax=ax[2], label='wetness index')
fig.tight_layout(); fig.savefig('t1_terrain.png', dpi=130)
st.to_csv('t1_terrain_stats.csv')
