import os
os.chdir('/home/user/primordial-simulation/.claude/worktrees/agent-a2f9648fe0863f025')
def edit(p, pairs):
    s = open(p).read()
    for a, b in pairs:
        assert s.count(a) == 1, (p, a)
        s = s.replace(a, b)
    open(p, 'w').write(s)
edit('web/shaders/land.wgsl', [("""// (land.wgsl), in the view's colours. Made once more for draws whose pixels take at least 1 m and 3 m (web/shade,
// draw.wgsl `LEAST_E`): seen from afar, the detail of the ground and the plants that cannot show is compiled out.""",
"""// (land.wgsl), in the view's colours. Made once more for draws whose pixels take at least 0.5 m, 1 m and 3 m
// (web/shade, draw.wgsl `LEAST_E`): seen from afar, the detail of the ground and the plants that cannot show is
// compiled out.""")])
edit('README.md', [("""reaches and the cells. While the world plays, the worker posts its state to the page at most four times a second
(`web/src/worker.ts` `FRAME_MS`): each frame costs the page a repaint of the map.""",
"""reaches and the cells. While the world plays, the worker posts its state to the page at most four times a second
(`web/src/worker.ts` `FRAME_MS`): each frame costs the page a repaint of the map. The map is drawn anew only when what
it shows changes: the camera, the view drawn, a new date of the world (a frame of the date shown, as after a pause or
with the things of another window, brings its things and no repaint), a tile of the ground that shows in the view (one
ahead of the view, or under finer tiles kept, waits for a pan).""" ),
("""pixels, a tile a frame, and shown when whole. The page never waits for the card: what it reads back (the classes, the
colours under the avatars) comes a frame or so later.""",
"""pixels, a tile a frame, and shown when whole. The page never waits for the card: what it reads back (the classes, the
colours under the avatars) comes a frame or so later. Seen from afar, most of the drawing's detail is smaller than a
pixel and shows nothing: the build makes the live map's program once more for draws whose pixels take at least 0.5 m,
1 m and 3 m, with that detail compiled out (`web/shade`, `LEAST_E` in `web/shaders/lib/draw.wgsl`), and the page draws
with the one made for the largest no more than its pixels take (`web/src/gpu.ts` `seenFrom`). The picture is the same
to the bit; the card runs and holds less code (a full frame of the fitted island in headless Chromium's software
renderer: about 0.55 of the time, at the 200 m scale bar on a phone about 0.9).""")])
