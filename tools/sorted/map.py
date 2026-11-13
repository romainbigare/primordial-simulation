"""map.py OUT.png YEAR DIR...: per run a panel of cover kg/m2 (log, top) and the leading kind (colour per root, bottom)."""
import csv, sys, math, colorsys
from PIL import Image, ImageDraw
out, year, dirs = sys.argv[1], int(sys.argv[2]), sys.argv[3:]
U, N, Z = 4194304, 22, 14
img = Image.new('RGB', (len(dirs) * (N * Z + 10), 2 * N * Z + 30), 'white')
d = ImageDraw.Draw(img)
for k, dd in enumerate(dirs):
    ox = k * (N * Z + 10)
    d.text((ox + 2, 2 * N * Z + 12), dd.split('/')[-1], fill='black')
    for r in csv.DictReader(open(f'{dd}/cells_{year:03d}.csv')):
        x, y = int(r['x']) // U, int(r['y']) // U
        kg = float(r['kgm2'])
        v = 0 if kg <= 0.001 else min(1, (math.log10(kg) + 3) / 5)
        g = colorsys.hsv_to_rgb(0.3, 0.8, 0.2 + 0.8 * v) if kg > 0.001 else (0.8, 0.7, 0.6)
        d.rectangle([ox + x * Z, y * Z, ox + x * Z + Z - 1, y * Z + Z - 1], fill=tuple(int(255 * c) for c in g))
        root = int(r['root'])
        c = (0.8, 0.7, 0.6) if root < 0 else colorsys.hsv_to_rgb((root * 0.618) % 1, 0.7, 0.9)
        d.rectangle([ox + x * Z, N * Z + 10 + y * Z, ox + x * Z + Z - 1, N * Z + 10 + y * Z + Z - 1], fill=tuple(int(255 * q) for q in c))
img.save(out)
