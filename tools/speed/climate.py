import csv, re, sys
# Means of years 21 to 30 of the climate numbers, and the drifts at year 30, per seed.
d = sys.argv[1]
cols = [('t_land', 't_land'), ('t_ground', 't_ground'), ('swing', None), ('rain_land', 'rain_land'), ('cover_kg', 'cover_kg'),
        ('lake_m3', 'lake_m3'), ('lakes', 'lakes'), ('runoff_sea', 'runoff_sea'), ('rivers', 'rivers')]
for seed in ('s1', 's2'):
    rows = [r for r in csv.DictReader(open(f'{d}/{seed}/series.csv')) if 21 <= float(r['year']) <= 30]
    out = []
    for name, c in cols:
        v = [float(r['t_land_summer']) - float(r['t_land_winter']) if c is None else float(r[c]) for r in rows]
        out.append(f"{name} {sum(v) / len(v):.5g}")
    log = open(f'{d}/{seed}.log').read()
    m = re.findall(r'year 30: element drift ([\d.e+-]+).*?energy drift ([\d.e+-]+)', log)
    h = re.findall(r'hash ([0-9a-f]+)', log)
    t = re.findall(r'\(([\d.]+) ms per year;', log)
    print(seed, ', '.join(out), f"| elem drift {m[-1][0]} energy drift {m[-1][1]} | {t[-1] if t else '?'} ms/yr | hash {h[-1] if h else '?'}")
