"""snags.py RENDERED_DIR OUT_PNG OUT_WGSL: the snags' strip (textures/snags.png, SOURCES.md) and its table
(shaders/lib/snag_sprites.wgsl).

The strip is 512 x 512 layers, from the sprites `sprites.cjs snags.json RENDERED_DIR <models>` renders one by one (each
fitted to its cell as models.png's, its foot on the cell's bottom middle), in the order `KINDS` names: charred trunks
broken off, a whole layer each (they are thin and tall: drawn 250 px tall up close), then charred stumps four to a layer;
each fitted to its cell, cut to its one whole piece (`whole`: the chips and the scraps of the ground the scan holds
apart from it are left out), sharpened once (`SHARPEN`, after the shrinking), its light scaled so its
mean (linear, over the texels it covers) is `LIGHT_MEAN` (burnt bark in the sun reads charcoal grey, about as light as
the bare trees' bark of models.png: 0.054), its darkest lifted by `LIFT` (charred wood is no black hole). The table holds
each sprite's width (twice its farthest texel from the cell's middle, so its middle is the cell's), its height
over its foot and its foot (the lowest row covered in the middle `FOOT_MIDDLE` of its width, where its trunk meets the
ground: chips and roots of the scan lie lower), as shares of its cell, its layer and its quarter (-1: the whole layer),
and each kind's first sprite and count.

Run with `python3 -I` (numpy and Pillow).
"""
import sys

import numpy as np
from PIL import Image, ImageFilter

SIDE = 512
LIGHT_MEAN = 0.06
LIFT = 0.006
LUMA = np.array([0.299, 0.587, 0.114])
FOOT_MIDDLE = 1 / 3
COVERED = 8
# An unsharp mask after the shrinking: its radius, texels, and strength, %.
SHARPEN = (1.0, 60)
# Each kind: its name in the table, its sprites a layer (1 or 4) and its rendered sprites (model, turns).
KINDS = [
    ("TRUNK", 1, [("burned_tree_photogrammetry", 6)]),
    ("STUMP", 4, [("burned_trunk_lassen", 6), ("old_burned_tree", 4)]),
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


# A texel joins the sprite's piece where its cover is over `COVERED`; the texels within `EDGE` of the piece keep their
# cover (its soft rim).
EDGE = 2


def whole(im):
    """Sprite `im` (RGBA) with its largest piece alone: of the parts its covered texels make (each texel joined to its
    eight neighbours), the one with most texels, and its rim (`EDGE`); the rest cleared."""
    a = np.asarray(im).copy()
    covered = a[..., 3] > COVERED
    label = np.where(covered, np.arange(covered.size).reshape(covered.shape) + 1, 0)
    while True:
        grown = label.copy()
        for dy in (-1, 0, 1):
            for dx in (-1, 0, 1):
                grown = np.maximum(grown, np.roll(np.roll(label, dy, 0), dx, 1))
        grown = np.where(covered, grown, 0)
        if np.array_equal(grown, label):
            break
        label = grown
    ids, counts = np.unique(label[covered], return_counts=True)
    keep = label == ids[counts.argmax()]
    for _ in range(EDGE):
        keep = keep | np.roll(keep, 1, 0) | np.roll(keep, -1, 0) | np.roll(keep, 1, 1) | np.roll(keep, -1, 1)
    a[..., 3] = np.where(keep, a[..., 3], 0)
    return Image.fromarray(a)


def measured(im, cell):
    """Of sprite `im` in its cell of `cell` texels: its width, its height over its foot and its foot, as shares."""
    covered = np.asarray(im)[..., 3] > COVERED
    ys, xs = np.nonzero(covered)
    half = max(cell / 2 - xs.min(), xs.max() + 1 - cell / 2)
    middle = covered[:, int(cell / 2 - FOOT_MIDDLE * half) : int(cell / 2 + FOOT_MIDDLE * half) + 1]
    base = np.nonzero(middle.any(axis=1))[0].max() + 1
    return 2 * half / cell, (base - ys.min()) / cell, (cell - base) / cell


def main():
    src, out_png, out_wgsl = sys.argv[1:4]
    cells = []
    for _, per, models in KINDS:
        cells += [(per, f"{model}_{k}.png") for model, n in models for k in range(n)]
    layer, quarter, placed = 0, 0, []
    for per, _ in cells:
        if per == 1:
            if quarter:
                layer, quarter = layer + 1, 0
            placed.append((layer, -1))
            layer += 1
        else:
            placed.append((layer, quarter))
            layer, quarter = (layer + 1, 0) if quarter == 3 else (layer, quarter + 1)
    layers = layer + (1 if quarter else 0)
    sheet = Image.new("RGBA", (SIDE, SIDE * layers))
    w, h, foot = [], [], []
    for (per, name), (at, q) in zip(cells, placed):
        cell = SIDE if per == 1 else SIDE // 2
        im = whole(Image.open(f"{src}/{name}").convert("RGBA").resize((cell, cell), Image.LANCZOS))
        im = lit(im.filter(ImageFilter.UnsharpMask(radius=SHARPEN[0], percent=SHARPEN[1], threshold=0)))
        x = measured(im, cell)
        w.append(x[0]); h.append(x[1]); foot.append(x[2])
        off = (0, 0) if q < 0 else ((q % 2) * cell, (q // 2) * cell)
        sheet.paste(im, (off[0], at * SIDE + off[1]))
    sheet.save(out_png, optimize=True)
    rows = lambda v: ", ".join(f"{x:.3f}" for x in v)
    n = len(cells)
    lines = [
        "// The snags' sprites (web/textures/snags.png), written by web/textures/bake/snags.py: their count and their layers,",
        "// each one's width, its height over its foot and its foot over its cell's bottom as shares of its cell, its layer",
        "// and its quarter (-1: the whole layer), and each kind's first sprite and count.",
        f"const SNAG_SPRITES = {n}; const SNAG_LAYERS = {layers};",
        f"const SNAG_W: array<f32, {n}> = array<f32, {n}>({rows(w)});",
        f"const SNAG_H: array<f32, {n}> = array<f32, {n}>({rows(h)});",
        f"const SNAG_F: array<f32, {n}> = array<f32, {n}>({rows(foot)});",
        f"const SNAG_LAYER: array<i32, {n}> = array<i32, {n}>({', '.join(str(a) for a, _ in placed)});",
        f"const SNAG_QUARTER: array<i32, {n}> = array<i32, {n}>({', '.join(str(q) for _, q in placed)});",
    ]
    first = 0
    for kind, _, models in KINDS:
        count = sum(c for _, c in models)
        lines.append(f"const {kind}_FROM = {first}; const {kind}_N = {count};")
        first += count
    open(out_wgsl, "w").write("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
