# Year rows of a run's series.csv: cover mass, deaths by cause, budget flows (PJ/yr), climate.
import csv, sys
rows = list(csv.DictReader(open(sys.argv[1] + '/series.csv')))
cols = sys.argv[2].split(',') if len(sys.argv) > 2 else None
yk = [k for k in rows[0] if k in ('year', 'years', 't_years')]
def f(r, k):
    try: return float(r[k])
    except: return float('nan')
default = ['cover_kg','cover_died_spent','cover_died_phase','cover_died_age','cover_died_taken','cover_died_lack',
 'cover_budget_stored','cover_budget_building','cover_budget_shed','cover_budget_died','cover_budget_exuded','cover_budget_seeds',
 't_land','t_land_winter','rain_land','evap_land','root_0_cover_frost','root_0_cover_moist','root_0_cover_light','root_1_kg']
cols = cols or default
print('row ' + ' '.join(f'{c[-11:]:>11}' for c in cols))
for n, r in enumerate(rows):
    print(f'{r.get(yk[0], n) if yk else n:>4} ' + ' '.join(f'{f(r,c):11.4g}' for c in cols))
