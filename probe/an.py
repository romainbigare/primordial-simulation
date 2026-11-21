import pandas as pd, sys, os
os.chdir(os.path.dirname(os.path.abspath(__file__)))
def tab(m):
    d = pd.read_csv(f'run{m}/slots.csv')
    s = d[d.slot >= 0]
    g = s.groupby(['year', 'phase', 'cell', 'patch']).agg(kg=('kg', 'sum'), ps=('pshare', 'first'), gr=('ground', 'first'), n=('slot', 'size'), top=('height', 'max')).reset_index()
    g['kgm2'] = g.kg / (g.gr * g.ps)
    return d, s, g
if __name__ == '__main__':
    for m in sys.argv[1:]:
        d, s, g = tab(m)
        print(m)
        print(g.groupby(['year', 'phase', 'patch']).agg(kgm2=('kgm2', 'mean'), n=('n', 'mean'), top=('top', 'mean'), cells=('cell', 'size')).reset_index().query('year in [60,61,62,65,70,80,90,100,110,120]').to_string())
        p = g[g.phase == 'post']
        cm = p.groupby(['year', 'cell']).apply(lambda x: x.kg.sum() / x.gr.iloc[0]).groupby('year').mean()
        print('cell mean', cm.loc[[60, 61, 70, 90, 120]].round(2).to_dict())
