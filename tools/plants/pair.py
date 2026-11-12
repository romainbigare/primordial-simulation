# Before | after pictures side by side, half size, labelled: python3 pair.py BEFORE_DIR AFTER_DIR OUT_DIR
import os, sys
from PIL import Image, ImageDraw
b, a, out = sys.argv[1:4]
os.makedirs(out, exist_ok=True)
for f in sorted(os.listdir(a)):
    if not f.endswith('.png') or not os.path.exists(os.path.join(b, f)):
        continue
    x, y = Image.open(os.path.join(b, f)), Image.open(os.path.join(a, f))
    w, h = x.size[0] // 2, x.size[1] // 2
    im = Image.new('RGB', (2 * w + 8, h + 28), 'white')
    im.paste(x.resize((w, h)), (0, 28)); im.paste(y.resize((w, h)), (w + 8, 28))
    d = ImageDraw.Draw(im)
    d.text((6, 8), 'before: ' + f, fill='black'); d.text((w + 14, 8), 'after', fill='black')
    im.save(os.path.join(out, f))
