"""Temperature and rain by latitude bin over land: today's band, the wide band alone (A), the wide band with the stretch
(C), and the globe (G144). Means over seeds 1 and 2 where both runs exist.
usage: compare.py S N BIN SIZE"""
import csv, sys

S, N, BIN, SIZE = float(sys.argv[1]), float(sys.argv[2]), float(sys.argv[3]), sys.argv[4]
R = "runs"


def bins(path, lo, hi):
    try:
        rows = [r for r in csv.DictReader(open(f"{path}/squares.csv")) if r["land"] == "1" and lo <= float(r["lat"]) < hi]
    except FileNotFoundError:
        return None
    if not rows:
        return None
    m = lambda k: sum(float(r[k]) for r in rows) / len(rows)
    return m("t_mean"), m("t_summer"), m("t_winter"), m("rain_m"), len(rows)


def both(tag, lo, hi):
    vals = [v for s in (1, 2) if (v := bins(f"{R}/{tag}_s{s}_n{SIZE}_b{S:g}-{N:g}", lo, hi))]
    if not vals:
        return None
    return tuple(sum(v[i] for v in vals) / len(vals) for i in range(4))


def f(v):
    return " " * 27 if v is None else f"{v[0]:6.1f} {v[1]:6.1f} {v[2]:6.1f} {v[3]:5.2f}"


print(f"band {S:g} to {N:g}, size {SIZE}, land squares, years 26-30; each block: T year, T warmest month, T coldest month (K), rain (m/yr)")
print(f"{'lat':>9} | {'wide band alone (A)':^27} | {'band + stretch (C)':^27} | {'all four edges open (W)':^27} | {'globe (seed 1)':^27}")
lo = S
while lo < N - 1e-9:
    hi = min(lo + BIN, N)
    print(f"{lo:4.1f}-{hi:4.1f} | {f(both('A', lo, hi))} | {f(both('C', lo, hi))} | {f(both('W', lo, hi))} | {f(bins(f'{R}/G144_s1', lo, hi))}")
    lo = hi
