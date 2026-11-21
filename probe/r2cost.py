import os
import pandas as pd, numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
D = os.path.dirname(os.path.abspath(__file__))
os.chdir(D)
plt.rcParams.update({'axes.spines.top': False, 'axes.spines.right': False, 'axes.grid': True, 'grid.color': '#e6e6e3', 'font.size': 10})
runs = [('base', 'baseline\n(main code)'), ('N', 'probe code\n(16 slots), no patch'), ('B20', 'patches in\n20 cells'), ('ALL', 'a 1% patch in\nevery land cell')]
rows = []
for r, lab in runs:
    s = pd.read_csv(f'c_{r}/series.csv')
    yc = [c for c in s.columns if c in ('year', 'years', 't_years')]
    s = s.iloc[-60:]  # years 61-120
    ms = {c: s[c].mean() for c in s.columns if c.startswith('ms_')}
    rows.append((lab, ms))
    print(r, {k: round(v, 1) for k, v in ms.items()}, round(sum(ms.values()), 1))
fig, ax = plt.subplots(1, 2, figsize=(12, 4.5))
tot = [sum(m.values()) for _, m in rows]
cov = [m['ms_cover'] for _, m in rows]
x = np.arange(len(rows))
ax[0].bar(x, tot, color='#2a78d6', width=0.6)
ax[1].bar(x, cov, color='#1baf7a', width=0.6)
for a, v, t in [(ax[0], tot, 'All laws, ms per simulated year'), (ax[1], cov, 'The cover alone, ms per simulated year')]:
    a.set_xticks(x); a.set_xticklabels([l for l, _ in rows], fontsize=9)
    a.set_title(t + '\n(mean of years 61-120, 2 threads, runs back to back)')
    for i, val in enumerate(v):
        a.text(i, val, f'{val:.0f}', ha='center', va='bottom', fontsize=9)
fig.suptitle('Cost of patches (noise between runs is about 5%)', fontsize=10)
fig.tight_layout(); fig.savefig('r2_cost.png', dpi=130)
