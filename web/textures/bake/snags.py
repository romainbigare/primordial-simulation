"""snags.py RENDERED_DIR OUT_PNG OUT_WGSL: the snags' strip (textures/snags.png, SOURCES.md) and its table
(shaders/lib/snag_sprites.wgsl).

The strip is 512 x 512 layers of four quarters, from the sprites `sprites.cjs snags.json RENDERED_DIR <models>` renders
one by one (each fitted to its cell as models.png's, its foot on the cell's bottom middle), in the order `KINDS` names:
charred trunks broken off, stands of dead stems, charred stumps; each shrunk to its quarter, its light scaled so its
mean (linear, over the texels it covers) is `LIGHT_MEAN` (burnt bark in the sun reads charcoal grey, about as light as
the bare trees' bark of models.png: 0.054), its darkest lifted by `LIFT` (charred wood is no black hole). The table holds
each sprite's width and height as shares of its quarter, and each kind's first sprite and count.

Run with `python3 -I` (numpy and Pillow).
"""
import sys

import numpy as np
from PIL import Image

SIDE = 512
QUARTER = SIDE // 2
LIGHT_MEAN = 0.06
LIFT = 0.006
LUMA = np.array([0.299, 0.587, 0.114])
# Each kind: its name in the table and its rendered sprites (model, turns).
KINDS = [
    ("TRUNK", [("burned_tree_photogrammetry", 6)]),
    ("STEMS", [("dead_trees", 2)]),
    ("STUMP", [("burned_trunk_lassen", 6), ("old_burned_tree", 4), ("old_burnt_tree", 4)]),
]


def linear(x):
    return np.where(x <= 0.04045, x / 12.92, ((x + 0.055) / 1.055) ** 2.4)


def encoded(x):
    x = np.clip(x, 0.0, 1.0)
    return np.where(x <= 0.0031308, 12.92 * x, 1.055 * x ** (1 / 2.4) - 0.055)


def lit(im):
    """Sprite `im` (RGBA) with its light scaled to `LIGHT_MEAN` over its cover, its darkest lifted by `LIFT`."""
    a = np.asarray(im, dtype=np.float64) / 255.0
    rgb, cover = linear(a[..., :3]), a[..., 3]
    shown = cover > 0.5
    mean = (rgb[shown] @ LUMA).mean()
    rgb = LIFT + rgb * (LIGHT_MEAN - LIFT) / max(mean, 1e-6)
    out = np.concatenate([encoded(rgb), a[..., 3:]], axis=2)
    return Image.fromarray((out * 255 + 0.5).astype(np.uint8))


def main():
    src, out_png, out_wgsl = sys.argv[1:4]
    names = [(kind, f"{model}_{k}.png") for kind, models in KINDS for model, n in models for k in range(n)]
    layers = (len(names) + 3) // 4
    sheet = Image.new("RGBA", (SIDE, SIDE * layers))
    w, h = [], []
    for k, (_, name) in enumerate(names):
        im = lit(Image.open(f"{src}/{name}").convert("RGBA").resize((QUARTER, QUARTER), Image.LANCZOS))
        ys, xs = np.nonzero(np.asarray(im)[..., 3] > 8)
        w.append((xs.max() - xs.min() + 1) / QUARTER)
        h.append((QUARTER - ys.min()) / QUARTER)
        layer, q = divmod(k, 4)
        sheet.paste(im, ((q % 2) * QUARTER, layer * SIDE + (q // 2) * QUARTER))
    sheet.save(out_png, optimize=True)
    rows = lambda v: ", ".join(f"{x:.3f}" for x in v)
    n = len(names)
    lines = [
        "// The snags' sprites (web/textures/snags.png), written by web/textures/bake/snags.py: their count and their layers,",
        "// each one's width and height as shares of its quarter, and each kind's first sprite and count.",
        f"const SNAG_SPRITES = {n}; const SNAG_LAYERS = {layers};",
        f"var<private> SNAG_W: array<f32, {n}> = array<f32, {n}>({rows(w)});",
        f"var<private> SNAG_H: array<f32, {n}> = array<f32, {n}>({rows(h)});",
    ]
    first = 0
    for kind, models in KINDS:
        count = sum(c for _, c in models)
        lines.append(f"const {kind}_FROM = {first}; const {kind}_N = {count};")
        first += count
    open(out_wgsl, "w").write("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
