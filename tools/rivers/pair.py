# Put pictures side by side: python3 pair.py OUT A B [C ...]
from PIL import Image
import sys
ims = [Image.open(p).convert('RGB') for p in sys.argv[2:]]
w = sum(i.width for i in ims) + 10 * (len(ims) - 1)
im = Image.new('RGB', (w, max(i.height for i in ims)), 'white')
x = 0
for i in ims:
    im.paste(i, (x, 0)); x += i.width + 10
im.save(sys.argv[1])
