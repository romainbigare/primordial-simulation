"""Gene means (by built cover) of the land squares by side (west third, east third) and by latitude bands.
usage: genes.py RUN_DIR ..."""
import csv, sys

for d in sys.argv[1:]:
    rows = [r for r in csv.DictReader(open(d + "/squares.csv")) if r["land"] == "1" and r["frost"] != "NaN"]
    w = max(int(r["x"]) for r in rows) + 1
    groups = [("west", lambda r: int(r["x"]) < w / 3), ("east", lambda r: int(r["x"]) >= 2 * w / 3)]
    groups += [(f"{lo}-{lo + 10}", lambda r, lo=lo: lo <= float(r["lat"]) < lo + 10) for lo in (25, 35, 45, 55)]
    for name, sel in groups:
        rs = [r for r in rows if sel(r)]
        if not rs:
            continue
        m = lambda k: sum(float(r[k]) for r in rs) / len(rs)
        print(f"{d.split('/')[-1]:22s} {name:6s} n {len(rs):3d} frost {m('frost'):.2f} moist {m('moist'):.3f} light {m('light'):.3f} walls {m('walls'):.3f} rain {m('rain_m'):.2f} cover {m('cover_kg'):.2f}")
