"""Per row (south to north) land means of a probe run's squares.csv; or zonal means of a globe run at given latitudes.
usage: rows.py RUN_DIR [--globe S N STEP]"""
import csv, sys, collections

NAMES = {0: "herb", 2: "smallfast", 3: "tree", 10: "wettree", 11: "drywoody", 12: "tallfast"}


def load(d):
    return list(csv.DictReader(open(f"{d}/squares.csv")))


def rows(d):
    by = collections.defaultdict(list)
    for r in load(d):
        if r["land"] == "1":
            by[int(r["y"])].append(r)
    out = []
    for y in sorted(by, reverse=True):
        rs = by[y]
        n = len(rs)
        m = lambda k: sum(float(r[k]) for r in rs) / n
        leads = collections.Counter(NAMES.get(int(r["lead"]), "-") for r in rs)
        kinds = collections.defaultdict(float)
        for r in rs:
            for kv in r["kinds"].split():
                k, v = kv.split(":")
                kinds[NAMES.get(int(k), k)] += float(v) / n
        frost = [float(r["frost"]) for r in rs if r["frost"] != "NaN"]
        moist = [float(r["moist"]) for r in rs if r["moist"] != "NaN"]
        out.append(dict(y=y, lat=float(rs[0]["lat"]), n=n, t=m("t_mean"), ts=m("t_summer"), tw=m("t_winter"), rain=m("rain_m"),
                        snow=m("snow_months"), cover=m("cover_kg"), z=m("z"),
                        lead=",".join(f"{k}{v}" for k, v in leads.most_common(2)),
                        frost=sum(frost) / len(frost) if frost else float("nan"),
                        moist=sum(moist) / len(moist) if moist else float("nan"),
                        kinds=" ".join(f"{k}={v:.2f}" for k, v in sorted(kinds.items(), key=lambda e: -e[1]) if v > 0.005)))
    return out


def show(d):
    print(f"# {d}  (land rows, south to north)")
    print(f"{'lat':>6} {'n':>3} {'z':>5} {'Tmean':>6} {'Tsum':>6} {'Twin':>6} {'rain':>5} {'snow':>4} {'cover':>6} {'frost':>6} {'moist':>6}  lead / kinds kg/m2")
    for r in rows(d):
        print(f"{r['lat']:6.2f} {r['n']:3d} {r['z']:5.0f} {r['t']:6.1f} {r['ts']:6.1f} {r['tw']:6.1f} {r['rain']:5.2f} {r['snow']:4.1f} {r['cover']:6.2f} {r['frost']:6.1f} {r['moist']:6.2f}  {r['lead']}  {r['kinds']}")


def globe(d, lats):
    data = load(d)
    print(f"# globe {d}: zonal means over land (and all) at latitudes")
    print(f"{'lat':>6} {'nland':>5} {'Tland':>6} {'Tsum':>6} {'Twin':>6} {'rain':>5} {'Tall':>6} {'rainall':>7}")
    for lo, hi in lats:
        sel = [r for r in data if lo <= float(r["lat"]) < hi]
        land = [r for r in sel if r["land"] == "1"]
        m = lambda rs, k: sum(float(r[k]) for r in rs) / max(len(rs), 1)
        print(f"{(lo+hi)/2:6.1f} {len(land):5d} {m(land,'t_mean'):6.1f} {m(land,'t_summer'):6.1f} {m(land,'t_winter'):6.1f} {m(land,'rain_m'):5.2f} {m(sel,'t_mean'):6.1f} {m(sel,'rain_m'):7.2f}")


if __name__ == "__main__":
    if "--globe" in sys.argv:
        i = sys.argv.index("--globe")
        s, n, st = map(float, sys.argv[i + 1:i + 4])
        lats = []
        x = s
        while x < n - 1e-9:
            lats.append((x, x + st))
            x += st
        globe(sys.argv[1], lats)
    else:
        show(sys.argv[1])
