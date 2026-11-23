"""split.py DIR GROUP FILE...: the layers of a strip of 512 x 512 layers (one or more files, top first) written again
as files of at most `STRIP_LAYERS` layers each, DIR/<GROUP>.<ext>, DIR/<GROUP>2.<ext>, ..., named under GROUP in
DIR/strips.json (the page loads the groups in their layers' order, web/src/detail.ts). A PNG is written as it is; a
JPEG with its own quantization tables and sampling, so its pixels change by a level or so at most.

Then bake/sizes.py writes each file again at the smaller sides the page loads where its view needs no finer detail.

A phone's browser decodes a picture whole: a file of a few layers keeps what it holds at once small (a strip of 32
layers, 512 x 16384 pixels, is 32 MB decoded), and lets the page load the layers it needs.
"""
import json
import os
import sys

from PIL import Image, JpegImagePlugin

SIDE = 512
STRIP_LAYERS = 4


def main():
    out, group, files = sys.argv[1], sys.argv[2], sys.argv[3:]
    images = [Image.open(f) for f in files]
    for im in images:
        im.load()
    ext = os.path.splitext(files[0])[1]
    jpeg = images[0].format == 'JPEG'
    keep = dict(qtables=images[0].quantization, subsampling=JpegImagePlugin.get_sampling(images[0])) if jpeg else {}
    layers = [im.crop((0, y, SIDE, y + SIDE)) for im in images for y in range(0, im.height, SIDE)]
    names = []
    # A strip that fits one file already is that file, as it is (a JPEG is not encoded again).
    if len(files) == 1 and len(layers) <= STRIP_LAYERS:
        names = [os.path.basename(files[0])]
    for k in range(0, len(layers) if not names else 0, STRIP_LAYERS):
        part = layers[k:k + STRIP_LAYERS]
        strip = Image.new(images[0].mode, (SIDE, SIDE * len(part)))
        for i, layer in enumerate(part):
            strip.paste(layer, (0, SIDE * i))
        name = f'{group}{len(names) + 1 if names else ""}{ext}'
        names.append(name)
        strip.save(os.path.join(out, name), **keep) if jpeg else strip.save(os.path.join(out, name), optimize=True)
    path = os.path.join(out, 'strips.json')
    named = json.load(open(path)) if os.path.exists(path) else {}
    named[group] = names
    with open(path, 'w') as f:
        f.write(json.dumps(named, indent=1) + '\n')
    print(group, len(layers), 'layers in', len(names), 'files')


main()
