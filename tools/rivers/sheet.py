# A sheet of pictures of folder DIR, two across: python3 sheet.py DIR OUT A B C D  (names without folder and .png)
from PIL import Image
import sys
d = sys.argv[1]
ims = [Image.open(f'{d}/{p}.png').convert('RGB') for p in sys.argv[3:]]
w, h = ims[0].size
s = Image.new('RGB', (w * 2 + 10, (h + 10) * ((len(ims) + 1) // 2)), 'white')
for i, im in enumerate(ims):
    s.paste(im, ((i % 2) * (w + 10), (i // 2) * (h + 10)))
s.save(f'{d}/{sys.argv[2]}.png')
