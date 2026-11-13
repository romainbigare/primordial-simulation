# The cover's budget flows (PJ/yr) of runs at chosen years, side by side.
import csv, sys
runs = sys.argv[1].split(','); years = [int(y) for y in sys.argv[2].split(',')]
D = sys.argv[3]
flows = ["stored","basal","capacity","firmness","leaves","sapwood","root_breath","root_renew","building","shed","died","exuded","seeds","bodies","placed","rounded"]
data = {r: {int(float(x['year'])): x for x in csv.DictReader(open(f'{D}/{r}/series.csv'))} for r in runs}
hdr = [f'{r}@{y}' for y in years for r in runs]
print(f'{"flow":>12} ' + ' '.join(f'{h:>12}' for h in hdr))
for fl in flows + ['SUM']:
    vals = []
    for y in years:
        for r in runs:
            row = data[r][y]
            v = sum(float(row['cover_budget_'+f]) for f in flows) if fl == 'SUM' else float(row['cover_budget_'+fl])
            vals.append(v / 1e15)
    print(f'{fl:>12} ' + ' '.join(f'{v:12.2f}' for v in vals))
for c in ['cover_kg','cover_died_spent','cover_died_age','cover_died_phase','cover_died_lack','cover_slots','cover_median','root_0_cover_leaf','root_0_cover_walls','root_0_cover_light','root_0_cover_area','root_0_cover_moist','root_0_cover_store','t_land','t_land_summer','t_land_winter','rain_land','evap_land','runoff_sea']:
    vals = [float(data[r][y].get(c, 'nan')) for y in years for r in runs]
    print(f'{c[-12:]:>12} ' + ' '.join(f'{v:12.4g}' for v in vals))
