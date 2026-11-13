# How near a belt's limbs come to one another: python3 necks.py key=value ...
import sys, math, importlib.util
sys.argv = ['belt.py', '/dev/null'] + sys.argv[1:] + ['rows=0']
spec = importlib.util.spec_from_file_location('belt', '/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/rv/belt.py')
b = importlib.util.module_from_spec(spec)
try:
    spec.loader.exec_module(b)
except Exception:
    pass
width = 5.0; close = [0, 0, 0]; sin = 0
for seed in range(40):
    pts = b.walk(1000 + seed, width, 140)
    n = len(pts); S = b.STEPS
    length = sum(math.hypot(pts[i + 1][0] - pts[i][0], pts[i + 1][1] - pts[i][1]) for i in range(n - 1)); sin += length / pts[-1][0] / 40
    grid = {}
    for i, p in enumerate(pts): grid.setdefault((int(p[0] // 10), int(p[1] // 10)), []).append(i)
    best = {}
    for i, p in enumerate(pts):
        cx, cy = int(p[0] // 10), int(p[1] // 10)
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                for j in grid.get((cx + dx, cy + dy), []):
                    if j > i + S:
                        d = math.hypot(p[0] - pts[j][0], p[1] - pts[j][1]) / width
                        k = i // S
                        best[k] = min(best.get(k, 9), d)
    for d in best.values():
        close[0] += d < 0.5; close[1] += d < 1.2; close[2] += d < 2.0
print(' '.join(sys.argv[2:-1]), '-> sinuosity %.2f; half bends (of 5600) whose limb comes within 0.5 w: %d, 1.2 w: %d, 2 w: %d' % (sin, *close))
