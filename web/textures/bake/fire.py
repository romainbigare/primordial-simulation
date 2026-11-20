"""fire.py KENNEY_PNG_DIR BURNED_GROUND_JPG OUT_JPG: the fire's strip (textures/fire.jpg, SOURCES.md), two 512 x 512 layers.

Layer 0: four sprites, each in a quarter of 256 x 256, its foot on the quarter's bottom middle two texels over its edge
(as models.png): two flame tongues (Kenney's flame_05, flame_06) and two puffs of smoke (smoke_07, smoke_08), each its
density alone (the sprite's grey times its cover) in all three channels, on black. Layer 1: Poly Haven's
burned_ground_01 (diffuse, 2k) shrunk to 512 x 512, its light divided by itself blurred (a Gaussian of 40 texels,
wrapping round) and scaled back to its mean, as ground.jpg's photographs.

Run with `python3 -I` (numpy and Pillow); the downloads are data only.
"""
import sys

import numpy as np
from PIL import Image, ImageFilter

SIDE = 512
QUARTER = SIDE // 2
# A sprite fills its quarter but for this share of its side; its foot stands this many texels over the edge.
MARGIN = 1 / 64
FOOT = 2
BLUR = 40
SPRITES = ["flame_05.png", "flame_06.png", "smoke_07.png", "smoke_08.png"]


def density(path):
    """The sprite's density (grey times cover, 0..1), cut to what it covers."""
    im = Image.open(path).convert("RGBA")
    a = np.asarray(im, dtype=np.float64) / 255.0
    d = a[..., :3].mean(axis=2) * a[..., 3]
    ys, xs = np.nonzero(d > 1 / 255)
    return d[ys.min() : ys.max() + 1, xs.min() : xs.max() + 1]


def quarter(d):
    """Sprite density `d` fitted into a quarter, its foot on the bottom middle; and its width and height as shares."""
    room = QUARTER * (1 - 2 * MARGIN)
    k = room / max(d.shape)
    w, h = max(1, round(d.shape[1] * k)), max(1, round(d.shape[0] * k))
    im = Image.fromarray((d * 255).astype(np.uint8)).resize((w, h), Image.LANCZOS)
    out = Image.new("L", (QUARTER, QUARTER))
    out.paste(im, ((QUARTER - w) // 2, QUARTER - FOOT - h))
    return out, w / QUARTER, h / QUARTER


def photo(path):
    """The photograph shrunk, its large-scale light divided out (wrapping), scaled back to its mean."""
    im = Image.open(path).convert("RGB").resize((SIDE, SIDE), Image.LANCZOS)
    a = np.asarray(im, dtype=np.float64)
    tiled = Image.fromarray(np.tile(a, (3, 3, 1)).astype(np.uint8)).filter(ImageFilter.GaussianBlur(BLUR))
    light = np.asarray(tiled, dtype=np.float64)[SIDE : 2 * SIDE, SIDE : 2 * SIDE].mean(axis=2, keepdims=True)
    flat = a / np.maximum(light, 1.0) * a.mean(axis=(0, 1), keepdims=True).mean()
    return Image.fromarray(np.clip(flat, 0, 255).astype(np.uint8))


def main():
    kenney, ground, out = sys.argv[1:4]
    sheet = Image.new("RGB", (SIDE, 2 * SIDE))
    for k, name in enumerate(SPRITES):
        q, w, h = quarter(density(f"{kenney}/{name}"))
        sheet.paste(Image.merge("RGB", (q, q, q)), ((k % 2) * QUARTER, (k // 2) * QUARTER))
        print(f"{name}: width {w:.4f}, height {h:.4f}")
    sheet.paste(photo(ground), (0, SIDE))
    sheet.save(out, quality=90)


if __name__ == "__main__":
    main()
