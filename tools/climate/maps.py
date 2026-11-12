"""Maps per square of probe runs: mean ground temperature, summer and winter, rain per year, the leading kind, and the
frost gene, each on one colour scale for all runs, with a legend. Writes an HTML page (screenshot it to PNG).
usage: maps.py OUT.html TITLE RUN_DIR [RUN_DIR ...]"""
import csv, sys, html

NAMES = {0: "herb", 2: "small fast", 3: "tree", 10: "wet tree", 11: "dry woody", 12: "tall fast"}
KIND_COL = {0: "#7fbf3f", 2: "#e6c84a", 3: "#1f6b2f", 10: "#2a9d8f", 11: "#b5651d", 12: "#c94fb0", -1: "#d8d0c0"}
SEA = "#9db8d8"


def ramp(stops, v):
    if v != v:
        return "#888"
    if v <= stops[0][0]:
        c = stops[0][1]
    elif v >= stops[-1][0]:
        c = stops[-1][1]
    else:
        for (a, ca), (b, cb) in zip(stops, stops[1:]):
            if a <= v <= b:
                t = (v - a) / (b - a)
                c = tuple(round(x + (y - x) * t) for x, y in zip(ca, cb))
                break
    return "#%02x%02x%02x" % c


T_STOPS = [(265, (40, 40, 150)), (273, (120, 170, 240)), (280, (200, 230, 230)), (286, (250, 230, 140)), (293, (240, 140, 60)), (300, (170, 30, 30))]
R_STOPS = [(0.0, (240, 230, 200)), (0.4, (220, 200, 120)), (0.8, (150, 200, 110)), (1.5, (60, 150, 160)), (2.5, (30, 70, 150)), (4.0, (20, 20, 90))]
F_STOPS = [(264, (60, 60, 160)), (268, (150, 190, 230)), (270, (240, 240, 200)), (272, (240, 160, 80)), (275, (170, 40, 30))]
PANELS = [("t_mean", "ground T, year mean (K)", T_STOPS), ("t_summer", "warmest month (K)", T_STOPS), ("t_winter", "coldest month (K)", T_STOPS),
          ("rain_m", "rain (m/year)", R_STOPS), ("lead", "kind with most cover", None), ("frost", "frost gene (K)", F_STOPS)]


def panel(rows, key, stops, px):
    w = max(int(r["x"]) for r in rows) + 1
    h = max(int(r["y"]) for r in rows) + 1
    out = [f'<svg width="{w*px}" height="{h*px}" viewBox="0 0 {w} {h}" shape-rendering="crispEdges">']
    for r in rows:
        land = r["land"] == "1"
        if key == "lead":
            col = KIND_COL.get(int(r["lead"]), "#000") if land else SEA
            if land and float(r["cover_kg"]) < 0.05:
                col = KIND_COL[-1]
        elif key in ("rain_m",):
            col = ramp(stops, float(r[key]))
            if not land:
                col = "#ffffff" if False else col
        else:
            col = ramp(stops, float(r[key])) if land else SEA
            if key == "frost" and not land:
                col = SEA
        out.append(f'<rect x="{r["x"]}" y="{r["y"]}" width="1" height="1" fill="{col}"/>')
    out.append("</svg>")
    return "".join(out)


def legend(stops, label):
    if stops is None:
        items = "".join(f'<span class="sw" style="background:{KIND_COL[k]}"></span>{html.escape(n)} ' for k, n in NAMES.items())
        return f'<div class="leg"><b>{label}</b>: {items}<span class="sw" style="background:{KIND_COL[-1]}"></span>bare (&lt;0.05 kg/m²) <span class="sw" style="background:{SEA}"></span>sea/lake</div>'
    lo, hi = stops[0][0], stops[-1][0]
    n = 40
    cells = "".join(f'<span style="display:inline-block;width:6px;height:12px;background:{ramp(stops, lo + (hi-lo)*i/(n-1))}"></span>' for i in range(n))
    return f'<div class="leg"><b>{label}</b>: {lo} {cells} {hi}</div>'


def main():
    out, title, runs = sys.argv[1], sys.argv[2], sys.argv[3:]
    parts = [f"<html><head><style>body{{font:12px sans-serif;background:#fff;margin:8px}} td{{padding:3px;vertical-align:top}} .sw{{display:inline-block;width:12px;height:12px;margin:0 3px 0 8px;vertical-align:middle}} .leg{{margin:2px 0}}</style></head><body><h3>{html.escape(title)}</h3>"]
    parts.append("<table><tr><td></td>" + "".join(f"<td><b>{p[1]}</b></td>" for p in PANELS) + "</tr>")
    for d in runs:
        rows = list(csv.DictReader(open(f"{d}/squares.csv")))
        px = max(2, 200 // (max(int(r["x"]) for r in rows) + 1))
        lat = [float(r["lat"]) for r in rows]
        name = d.rstrip("/").split("/")[-1]
        parts.append(f"<tr><td><b>{html.escape(name)}</b><br>N {max(lat):.1f}°<br>S {min(lat):.1f}°<br>(north up)</td>" + "".join(f"<td>{panel(rows, k, s, px)}</td>" for k, _, s in PANELS) + "</tr>")
    parts.append("</table>")
    for k, label, s in PANELS:
        if k in ("t_summer", "t_winter"):
            continue
        parts.append(legend(s, "temperature (K), all three temperature panels" if k == "t_mean" else label))
    parts.append("</body></html>")
    open(out, "w").write("".join(parts))


main()
