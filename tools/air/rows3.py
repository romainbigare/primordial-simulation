"""The page's world (Wide, seed 1, size 22) by row of latitude at years 30 and 100 for the runs given (TAG=DIR ...):
cover kg/m2 | dry matter kg/m2/yr | rain m/yr."""
import subprocess, sys
S = '/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/'
runs = dict(a.split('=') for a in sys.argv[1:])


def load(d, y):
    t = subprocess.run(['python3', S + 'rows.py', d, str(y)], capture_output=True, text=True).stdout.split('\n')[1:]
    out = {}
    for line in t:
        p = line.split()
        if len(p) < 15:
            continue
        out[float(p[0])] = dict(kg=float(p[8]), vap=float(p[12]), rain=float(p[13]), npp=float(p[14]), tmin=float(p[2]))
    return out


names = list(runs)
for y in (30, 100):
    d = {n: load(r, y) for n, r in runs.items()}
    print(f"year {y}: ({' / '.join(names)})  cover kg/m2 | dry matter kg/m2/yr | rain m/yr | coldest month K")
    for lat in sorted(d[names[0]], reverse=True):
        if all(lat in d[n] for n in d):
            f = lambda k, w=5, p=2: ' '.join(f"{d[n][lat][k]:{w}.{p}f}" for n in names)
            print(f"{lat:5.1f}  {f('kg')} | {f('npp')} | {f('rain')} | {f('tmin', 6, 1)}")
