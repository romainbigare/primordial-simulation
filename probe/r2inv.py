import os, sys
import numpy as np, pandas as pd
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
D = os.path.dirname(os.path.abspath(__file__))
os.chdir(D)
plt.rcParams.update({'axes.spines.top': False, 'axes.spines.right': False, 'axes.grid': True, 'grid.color': '#e6e6e3', 'font.size': 10})
C = ['#2a78d6', '#eb6834', '#1baf7a', '#eda100']


def patches(run):
    d = pd.read_csv(f'{run}/slots.csv')
    d = d[(d.phase == 'post') & (d.slot >= 0)]
    g = d.groupby(['year', 'cell', 'patch']).agg(kg=('kg', 'sum'), ps=('pshare', 'first'), gr=('ground', 'first')).reset_index()
    g['kgm2'] = g.kg / (g.gr * g.ps)
    return g


ctrl = patches('r2N')
ctrl = ctrl.groupby(['year', 'cell']).kgm2.sum().reset_index()
runs = sys.argv[1:]  # pairs name:label
fig, axs = plt.subplots(1, len(runs), figsize=(6.5 * len(runs), 4.8), sharey=True)
axs = np.atleast_1d(axs)
for ax, rl in zip(axs, runs):
    run, label = rl.split(':')
    g = patches(run)
    # ratio to the control, cell by cell, then the median over cells
    for k, (lab, col) in enumerate([('the 90% patch', C[0]), ('the 10% patch', C[1])]):
        x = g[g.patch == k].merge(ctrl, on=['year', 'cell'], suffixes=('', '_c'))
        x['r'] = x.kgm2 / x.kgm2_c
        q = x.groupby('year').r.quantile([.25, .5, .75]).unstack()
        ax.plot(q.index, q[0.5], color=col, lw=2, label=lab)
        ax.fill_between(q.index, q[0.25], q[0.75], color=col, alpha=0.15, lw=0)
        print(run, lab, q[0.5].loc[[61, 70, 90, 120]].round(3).to_dict())
    ax.axhline(1, color='#8a8984', ls='--', lw=1.5, label='no split (control)')
    ax.set_title(label)
    ax.set_xlabel('year (split at the end of year 60)'); ax.set_xlim(60, 120)
axs[0].set_ylabel('cover per m², over the same cell without split\n(median of 20 cells, band: middle half)')
axs[0].legend(frameon=False, loc='lower left')
fig.suptitle('The invariant: split a cell into two alike patches (90% and 10%), kill nothing. Both should stay at 1.', fontsize=11)
fig.tight_layout(); fig.savefig('r2_invariant.png', dpi=130)
