"""sheets.py BEFORE AFTER OUT: contact sheets, BEFORE above AFTER, from the shots of shootall.sh."""
import sys, os
from PIL import Image, ImageDraw
b, a, out = sys.argv[1:4]
D = os.path.dirname(os.path.abspath(__file__))

def sheet(names, path, scale=0.5):
    ims = []
    for tag in (b, a):
        row = []
        for n in names:
            f = f"{D}/{tag}/{n}"
            row.append(Image.open(f).convert("RGB") if os.path.exists(f) else Image.new("RGB", (650, 600), "grey"))
        ims.append(row)
    w, h = int(ims[0][0].width * scale), int(ims[0][0].height * scale)
    s = Image.new("RGB", (w * len(names), h * 2 + 20), "white")
    d = ImageDraw.Draw(s)
    for r, row in enumerate(ims):
        for c, im in enumerate(row):
            s.paste(im.resize((w, h)), (c * w, r * (h + 20)))
    d.text((4, h + 4), f"above: {b}   below: {a}", fill=(0, 0, 0))
    s.save(path)

sheet([f"s{s}-22-whole.png" for s in range(1, 7)], f"{out}-22.png")
sheet([f"s{s}-32-whole.png" for s in range(1, 4)], f"{out}-32.png")
sheet([f"true-s{s}-22-whole.png" for s in (1, 2)], f"{out}-true.png")
for s, places in ((1, ["9-10", "15-13", "17-6"]), (2, ["13-13", "7-5", "3-8"])):
    sheet([f"s{s}-22-{p}-{bar}.png" for p in places for bar in (2000, 500)], f"{out}-near-s{s}.png", 0.6)
