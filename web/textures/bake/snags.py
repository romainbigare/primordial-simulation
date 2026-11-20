"""snags.py RENDERED_DIR OUT_PNG: the snags' strip (textures/snags.png, SOURCES.md), 512 x 512 layers of four
quarters, from the sprites `sprites.cjs snags.json RENDERED_DIR <models>` renders one by one (each fitted to its cell
as models.png's, its foot on the cell's bottom middle): three charred standing trunks, a stand of dead stems, and three
charred stumps, in the order `SPRITES` names, each shrunk to its quarter; and the sprites' width and height as shares of
their quarter, printed (the page's table).

Run with `python3 -I` (numpy and Pillow).
"""
import sys

import numpy as np
from PIL import Image

SIDE = 512
QUARTER = SIDE // 2
SPRITES = [
    "burned_tree_photogrammetry_0.png",
    "burned_tree_photogrammetry_1.png",
    "burned_tree_photogrammetry_2.png",
    "dead_trees_0.png",
    "burned_trunk_lassen_0.png",
    "burned_trunk_lassen_1.png",
    "burned_trunk_lassen_2.png",
]


def main():
    src, out = sys.argv[1:3]
    layers = (len(SPRITES) + 3) // 4
    sheet = Image.new("RGBA", (SIDE, SIDE * layers))
    for k, name in enumerate(SPRITES):
        im = Image.open(f"{src}/{name}").convert("RGBA").resize((QUARTER, QUARTER), Image.LANCZOS)
        a = np.asarray(im)[..., 3]
        ys, xs = np.nonzero(a > 8)
        print(f"{name}: width {(xs.max() - xs.min() + 1) / QUARTER:.3f}, height {(QUARTER - ys.min()) / QUARTER:.3f}")
        layer, q = divmod(k, 4)
        sheet.paste(im, ((q % 2) * QUARTER, layer * SIDE + (q // 2) * QUARTER))
    sheet.save(out, optimize=True)


if __name__ == "__main__":
    main()
