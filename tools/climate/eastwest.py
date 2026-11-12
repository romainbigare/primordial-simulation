"""Rain on land by thirds of the map west to east, and on slopes that face east against those that face west (by the
height difference to the squares beside), for runs given.
usage: eastwest.py RUN_DIR ..."""
import csv, sys

for d in sys.argv[1:]:
    rows = list(csv.DictReader(open(f"{d}/squares.csv")))
    w = max(int(r["x"]) for r in rows) + 1
    at = {(int(r["x"]), int(r["y"])): r for r in rows}
    land = [r for r in rows if r["land"] == "1"]
    thirds = [[], [], []]
    for r in land:
        thirds[min(2, int(r["x"]) * 3 // w)].append(float(r["rain_m"]))
    m = lambda v: sum(v) / len(v) if v else float("nan")
    east, west = [], []
    for r in land:
        x, y = int(r["x"]), int(r["y"])
        a, b = at.get((x - 1, y)), at.get((x + 1, y))
        if not a or not b:
            continue
        # The ground rises toward the west: the slope faces east.
        slope = float(a["z"]) - float(b["z"])
        if slope > 100:
            east.append(float(r["rain_m"]))
        elif slope < -100:
            west.append(float(r["rain_m"]))
    s = open(f"{d}/summary.txt").read().split("edgesW/m")[-1].split("drift")[0].strip() if "edgesW/m" in open(f"{d}/summary.txt").read() else ""
    print(f"{d.split('/')[-1]:22s} rain west/mid/east {m(thirds[0]):.2f} {m(thirds[1]):.2f} {m(thirds[2]):.2f}  slopes facing east {m(east):.2f} (n {len(east)}) west {m(west):.2f} (n {len(west)})  edges W/m {s}")
