# Several pictures in one sheet, each scaled to W px wide, COLS a row: python3 grid.py OUT W COLS A B ...
from PIL import Image
import sys
out, w, cols = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
ims = []
for p in sys.argv[4:]:
    i = Image.open(p).convert('RGB')
    ims.append(i.resize((w, round(i.height * w / i.width)), Image.LANCZOS))
h = max(i.height for i in ims)
rows = (len(ims) + cols - 1) // cols
sheet = Image.new('RGB', (cols * (w + 6) - 6, rows * (h + 6) - 6), 'white')
for k, i in enumerate(ims):
    sheet.paste(i, ((k % cols) * (w + 6), (k // cols) * (h + 6)))
sheet.save(out)
