"""sizes.py DIR: of each strip file DIR/strips.json names (layers of 512 x 512), the same strip at smaller sides,
DIR/<name>@<side>.<ext> for each side of `SIDES`, which the page loads where its view needs no finer detail
(web/src/detail.ts `sideFor`). Each layer at a side is the mip level of that side the page itself makes from the layer
at 512 (web/src/detail-worker.ts `photoMips`): the colour of four texels the mean by their cover, the cover their mean,
and of the trees' layers (cut-outs) the cover scaled so as many texels show as at 512 (`keep_shown`); so a smaller
file gives the page the levels it made before. A PNG is written as PNG; a JPEG as JPEG of quality `JPEG_QUALITY`.
"""
import json
import os
import sys

import numpy as np
from PIL import Image

SIDE = 512
SIDES = (256, 128, 64)
# A cut-out texel shows where its cover is at least this (web/src/detail.ts `CUT`).
CUT = 128
JPEG_QUALITY = 92


def showing(level):
    """The share of a layer's texels whose cover reaches `CUT`."""
    return float((level[:, :, 3] >= CUT).mean())


def keep_shown(level, shown):
    """The cover of `level` scaled so the share `shown` of its texels reach `CUT` (detail-worker.ts `keepShown`)."""
    n = level.shape[0] * level.shape[1]
    counts = np.bincount(level[:, :, 3].ravel(), minlength=256)
    above, v = 0, 255
    while v > 1 and above + counts[v] <= shown * n:
        above += counts[v]
        v -= 1
    scale = CUT / (v + 1)
    level[:, :, 3] = np.minimum(255, np.floor(level[:, :, 3].astype(np.float64) * scale + 0.5)).astype(np.uint8)


def halve(level):
    """The next mip level of `level` (detail-worker.ts `photoMips`)."""
    a = [level[dy::2, dx::2].astype(np.int64) for dy in (0, 1) for dx in (0, 1)]
    cover = sum(x[:, :, 3] for x in a)
    out = np.zeros((level.shape[0] // 2, level.shape[1] // 2, 4), np.uint8)
    for c in range(3):
        weighted = sum(x[:, :, 3] * x[:, :, c] for x in a)
        mean = np.floor(weighted / np.maximum(cover, 1) + 0.5)
        out[:, :, c] = np.where(cover > 0, mean, 0).astype(np.uint8)
    out[:, :, 3] = ((cover + 2) >> 2).astype(np.uint8)
    return out


def main():
    root = sys.argv[1]
    strips = json.load(open(os.path.join(root, 'strips.json')))
    for group, files in strips.items():
        cut_out = group == 'trees'
        for name in files:
            im = Image.open(os.path.join(root, name))
            jpeg = im.format == 'JPEG'
            rgba = np.asarray(im.convert('RGBA'))
            layers = [rgba[y:y + SIDE] for y in range(0, rgba.shape[0], SIDE)]
            out = {side: [] for side in SIDES}
            for layer in layers:
                shown = showing(layer) if cut_out else 0.0
                level = layer.copy()
                while level.shape[0] > SIDES[-1]:
                    level = halve(level)
                    if cut_out and shown > 0:
                        keep_shown(level, shown)
                    if level.shape[0] in out:
                        out[level.shape[0]].append(level.copy())
            stem, ext = os.path.splitext(name)
            for side, made in out.items():
                strip = Image.fromarray(np.concatenate(made, axis=0), 'RGBA')
                path = os.path.join(root, f'{stem}@{side}{ext}')
                if jpeg:
                    strip.convert('RGB').save(path, quality=JPEG_QUALITY)
                else:
                    strip.save(path, optimize=True)
        print(group, len(files), 'files at', ', '.join(map(str, SIDES)))


main()
