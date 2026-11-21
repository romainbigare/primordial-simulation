import os
import numpy as np, pandas as pd
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
D = os.path.dirname(os.path.abspath(__file__))
os.chdir(D)
plt.rcParams.update({'axes.spines.top': False, 'axes.spines.right': False, 'axes.grid': True, 'grid.color': '#e6e6e3', 'font.size': 10})
C = ['#2a78d6', '#eb6834', '#1baf7a', '#eda100']

d = pd.read_csv('r2F/slots.csv')
d = d[(d.phase == 'post') & (d.slot >= 0)]
# the field: the patch of the smallest share in each cell
f = d[d.pshare < 0.05].copy()
f['area'] = f.ground * f.pshare
k0 = f[f.root == 0].groupby(['year', 'cell']).agg(seeds=('seeds', 'sum'), kg=('kg', 'sum'), area=('area', 'first')).reset_index()
cells = d.cell.unique()
allf = f.groupby(['year', 'cell']).agg(area=('area', 'first')).reset_index()
k0 = allf.merge(k0.drop(columns='area'), on=['year', 'cell'], how='left').fillna(0)
k0['seeds_m2'] = k0.seeds / k0.area
k0['kg_m2'] = k0.kg / k0.area
other = f[f.root != 0].groupby('year').size()
print('field cells per year', allf.groupby('year').size().loc[[61, 70, 90, 120]].to_dict())
print('field seeds kg/m2/yr', k0.groupby('year').seeds_m2.median().loc[[61, 62, 65, 70, 90, 120]].round(4).to_dict())
print('field cover kg/m2', k0.groupby('year').kg_m2.median().loc[[61, 65, 70, 90, 120]].round(3).to_dict())

w = pd.read_csv('r2F/wild.csv')
w = w[(w.patches == 1) & (w.year > 60)]
w['seeds_m2'] = w.seeds / w.ground
w['kg_m2'] = w.kg / w.ground
# also per m2 of the ground its plants hold
w['seeds_held'] = w.seeds / (w.ground * w.on.clip(lower=1e-6))
print('wild cells', w.groupby('year').size().median())
qw = w.groupby('year').seeds_m2.quantile([.5, .9]).unstack()
print('wild seeds kg/m2 cell median/p90', qw.loc[[61, 90, 120]].round(5).to_dict())
print('wild kg/m2', w.groupby('year').kg_m2.median().loc[[61, 90, 120]].round(3).to_dict())

fig, ax = plt.subplots(1, 2, figsize=(13, 4.8))
qf = k0.groupby('year').seeds_m2.quantile([.25, .5, .75]).unstack()
ax[0].plot(qf.index, qf[0.5], color=C[1], lw=2, label='field: 1% patch, sown, weeded each year (20 cells)')
ax[0].fill_between(qf.index, qf[0.25], qf[0.75], color=C[1], alpha=0.15, lw=0)
qw3 = w.groupby('year').seeds_m2.quantile([.25, .5, .75, .9]).unstack()
ax[0].plot(qw3.index, qw3[0.5], color=C[0], lw=2, label=f'wild: the same kind in cells with no patch (median of ~{int(w.groupby("year").size().median())} cells)')
ax[0].fill_between(qw3.index, qw3[0.25], qw3[0.75], color=C[0], alpha=0.15, lw=0)
ax[0].plot(qw3.index, qw3[0.9], color=C[0], lw=1.5, ls=':', label='wild: the best tenth of cells')
ax[0].set_yscale('log')
ax[0].set_ylabel('seeds made, kg (fuel + body matter)\nper m² per year')
ax[0].set_xlabel('year (field made and sown at the end of year 60)')
ax[0].set_title('Seed yield per m²: field vs wild')
ax[0].legend(frameon=False, fontsize=8, loc='lower right')
ax[0].set_xlim(61, 120)
qk = k0.groupby('year').kg_m2.quantile([.25, .5, .75]).unstack()
ax[1].plot(qk.index, qk[0.5], color=C[1], lw=2, label='field')
ax[1].fill_between(qk.index, qk[0.25], qk[0.75], color=C[1], alpha=0.15, lw=0)
wk = w.groupby('year').kg_m2.quantile([.25, .5, .75]).unstack()
ax[1].plot(wk.index, wk[0.5], color=C[0], lw=2, label='wild (per m² of its cell)')
ax[1].fill_between(wk.index, wk[0.25], wk[0.75], color=C[0], alpha=0.15, lw=0)
ax[1].set_yscale('log'); ax[1].set_xlim(61, 120)
ax[1].set_title('Standing cover of the sown kind, kg per m²')
ax[1].set_xlabel('year'); ax[1].legend(frameon=False)
fig.suptitle('A weeded field of the small fast kind', fontsize=11)
fig.tight_layout(); fig.savefig('r2_field.png', dpi=130)
