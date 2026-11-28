# The picture tool (`web/look/`)

Pictures and numbers of the page, taken in a headless browser on saved worlds, so a builder never grows a world in
the page. One entry, run in `web/` after `npm run wasm` and `npm run build`:

    npm run look -- <command> ...

It needs Playwright (found the normal way, else set `PLAYWRIGHT` to its folder, here
`/opt/node22/lib/node_modules/playwright`) and the system's EGL (`libegl1`, `libegl-mesa0`): the browser draws WebGL2
with Mesa's llvmpipe on every core (`page.mjs` `launch`), and prints `renderer: ...` at start. Without EGL it falls
back to SwiftShader, twice as slow. What it makes goes to `out/look/` (not in git): the page with its hooks
(`site/`), the crowns' mask build (`site-mask/`), the counted builds (`site-cost-*`), and the saved worlds (`worlds/`).

## Commands

| Command | What it does | Time here (4 cores, llvmpipe) |
|---|---|---|
| `set OUT [--scenes NAME] [--only A,B] [--px 400] [--crowns no]` | every scene and view of a scene set: `OUT/<scene>_<mpp>.png`, the mask pictures in `OUT/mask/`, `OUT/sheet.png`, `OUT/numbers.json`; prints each scene's kinds of cover (from the cells' table) | 10 min for `seed1` (28 + 28 pictures), 4.5 min with `--crowns no`; about 25 s more when the world is made first |
| `compare OUT_A OUT_B` | the numbers of two `set`s side by side with their differences (printed, and `OUT_B/compare_<A>.txt`), and `OUT_B/compare_<A>.png`: each picture of A beside B | a few s |
| `shot OUT.png --at X,Y --mpp M [--world FILE]` | one picture: of the set's world, or of a snapshot you give | about 1.5 min (the page makes its programs first) |
| `sheet OUT.png COLS FILE...` | a contact sheet of pictures, each labelled with its file name | a few s |
| `cost OUT [--scenes NAME] [--only A,B]` | the cost of a pixel as a count: each scene drawn once at each of the set's `cost_views` by two counted builds, the ground's and the marks' (below): `OUT/cost.json`, the count pictures `OUT/<scene>_<mpp>_<build>.png`, `OUT/sheet.png`, the counts as drawn in `OUT/raw/` | not timed with llvmpipe |
| `years OUT [--scenes NAME]` | the set's `years` scenes at its `years_view` in each of 10 years at the same date, from 10 saved worlds a year apart: the pictures, `OUT/years.png`, `OUT/years.json` | not timed with llvmpipe; 30 s to make its 10 worlds |

The map is `--px` css pixels a side (400 by default), taken at `--scale` screen pixels a css pixel (1 by default). The
first picture of a page takes 1 to 3 min: the browser makes the programs' machine code, on one thread a browser. Each
picture after it in the same page takes 1 to 10 s. So take many pictures in one command, not many `shot`s. Each build
(the page, the mask, the counted builds) runs in a browser of its own, so their programs are made side by side. A
picture that is not whole within 5 min is never taken: the command stops with an error. Two `set`s of one build give
the same numbers, but for pictures with flowing liquid, which may differ by a few hundredths (the flow's signs move with
the clock).

## Scenes and worlds (`scenes.json`)

A world is the flags of an `ew run` and a date (`at`, years with whole months: 42.75 is late summer of year 42), and for
`years` how many years in a row; or the test card (`"card": true`, made by `ew card`, at its one date). The tool makes
it with `cargo run --profile fast -p cli -- run <flags> --snapshots-at <dates>` (about 25 s for 43 years, 35 s for 52) into
`out/look/worlds/<name>-v<snapshot version>-<hash>/`, and makes it again when the snapshot's format version or the code
of `core`, `worldgen`, `tape` or `cli` changes (committed or not).

A scene set names its world, its views (metres a pixel; a scene may name its own), its cost views
(`cost`) and its scenes (a place in squares, x to the east, y to the south). `seed1` is the page's own world (seed 1,
size 32, band 20,65, the six starting kinds, the takers in year 52) at 42.75, with the builders' places. The law moves
what grows where: `set` prints each scene's kinds, so you see what stands there.

Each scene set and world in `scenes.json` says what it shows in its `about`. Two keys change how a world is opened:
`step` plays it that many months in the page, then pauses it (`page.mjs` `step`: what the core counts only as it steps,
a month's load and its plumes, is then drawn); `plumes` ([x, y, kg/s], x and y in squares) puts test plumes in place of
the world's (`page.mjs` `plumes`).

The scene set `plants` stands on `seed1`'s world where its plant cover is drawn from far to near: green hills
cut by gullies (12, 4 and 2 m a pixel), a forest where its trees pass from the field to single crowns (6 to 0.3 m),
tall forest and mountains from far (40 and 12 m), the island's middle (85 m), a river valley, an open woodland and
sparse low plants on dry ground; its cost views are 40, 4 and 1 m a pixel.

The scene set `ground` stands on `seed1`'s world on bare and thin-soil ground from 12 m to 0.4 m a pixel: steep bare
rock, steep hills of thin soil, the edge of the snow on them, a dry plain and a river valley's side, with the island's
middle (85 m) and the snowy hills from far (40 m); its cost views are 40, 4 and 1 m a pixel.

The scene set `water` stands on `seed1`'s world where it is wet, from far to near: the island's middle (85 m a pixel), the big lake from far (40 m), a stream's inlet into the big lake (12 to 0.3 m), a bay of the sea under a steep coast, the east river's mouth at the sea (12 to 1 m) and a river valley (12 to 0.3 m); its cost views are 40, 4 and 1 m a pixel.

The scene set `card` stands on the test card (`ew card`, `worldgen/src/testkit/card.rs`): an island of 12 x 10
squares whose block of 6 x 4 squares from square 3, 3 is set by hand, each square a kind of stand or ground the map
draws (a tall closed stand of four size groups, an open stand over a sward, a young dense stand near 4 m, tall soft
plants, bushes, swards, bare loose ground, trees of wet ground, snow on a sward, a woody stand in the cold season, and
more), so a picture before and after a change of the page shows the same scene whatever the laws do. It is not stepped:
the page opens it paused, in late summer of year 0. Not set by hand: a river (what runs in a channel comes from Flow's
steps), stony ground with thin loose cover (the ground's tiles come from the terrain's samples, not from the cells), and
the season (one for the whole world: a square is warm or cold by its ground's temperature, which the page's leaves and
snow follow; a warm square's soil is wetted, as the card has had no rain). The shore is the island's own (scene
`shore`).

## The numbers (`numbers.json`, per picture, over its middle 80 %: no button, no scale bar)

| Number | Meaning |
|---|---|
| `mean` | mean colour, R G B (0..255) |
| `lightness` | mean of R, G and B |
| `dark` | % of pixels darker than 50 (mean of R, G, B) |
| `black` | % of pixels darker than 16 |
| `specks` | % of pixels darker by more than 40 than the mean of their 8 neighbours: single dark pixels, salt and pepper |
| `green`, `white`, `blue` | % of pixels a colour tells: green (G over R and B by more than 4), white (all channels over 200: snow), blue (B over R by more than 8 and over G: water) |
| `crowns` | % of the ground under the trees' crowns: the same view drawn by the mask build (the marks and the far field of the lattice of stands, the trees', magenta: `mask.mjs`), per pixel m = (R+B)/2 - G, the share (m_mask - m) / (243 - m), 0..1 |
| `waited` | ms the picture took to settle (tiles made, frames drawn): not a measure of the page's speed |
| `<far>_on_<near>` | far against near on the same ground: the farther picture cut to the nearer one's ground, its numbers, and `lightness_change` (% the nearer is lighter) and `crowns_change` (points the nearer has more) |

`years.json`: for each scene, from each year to the next (`by_year`) and their means:

| Number | Meaning |
|---|---|
| `stay` | % of the first year's marks whose cell holds, the next year, a mark of the same square, kind (its slot, a label a kind keeps while it lives) and size group |
| `other` | % of the first year's marks whose cell holds another mark the next year (another kind or group) |
| `go` | % of the first year's marks whose cell holds none the next year |
| `appear` | % of the next year's marks whose cell held none |
| `marks` | the marks of each year (all lattices, the view's cells and those about it) |
| `pixels` | % of the picture whose pixels stay (each channel within 24 of 255) |

The marks are read back from the GPU after the picture is drawn (`read.mjs` `marks`: the first of the marks' textures,
laid out as `textures.ts` `prepare` lays them). A mark's size inside its group is not compared. The worlds are opened one
after another in one page.

## The cost of a pixel (`cost`)

The count is the measure of cost: the same on every machine and from run to run, so one run tells whether a change
makes a pixel do more work. A wall-clock time here is information only (it swings by about 30 % and need not match a
graphics card).

The tool builds the page's programs again from a copy of `web/shaders` (`cost.mjs`, never in the page) in which each
pixel counts what it does and draws the counts as its colour: every texture read (each `textureSample*` and
`textureLoad`, red, two reads a level), each gradient noise (`draw.wgsl` `gradient_noise`, green) and each read of a
photograph (`tex.wgsl` `photo`, blue). Two builds: `ground` counts the ground's
program (`land.wgsl`: `lib/live_map.wgsl`, the far field of the plants in it) and
draws no marks; `marks` counts the marks' program (`cover.wgsl`) over a black ground. The marks decided once a frame
for each cell (`marks.wgsl`) are not counted: they are not a pixel's work, nor are the flames and smoke over the map
(`flames.wgsl`, drawn only where a fire burns); the dead trees are the marks' (`lib/snag.wgsl`).

`cost.json`: for each scene and build, of `reads`, `noise` and `photos` a pixel over the middle 80 % of the picture: the
mean, the 95th percentile, the most, and the % of pixels at the top of the colour (a count it cannot hold: 510 reads,
255 noises or photographs). A texture read is known to one read (two to a level). In the pictures red is full at 510
texture reads, green at 64 noises, blue at 32 photographs. `compare` shows the counts of two `cost` folders.

## The hooks

The tool copies the built page (`web/dist`) and puts one object, `window.__ew`, at the end of the page's script
(`page.mjs` `HOOKS`): the map, the cover, the world, play and pause, the speed, the view and the layer shown, opening a saved
world by URL, and the numbers the marks' textures are laid out by. The shipped page holds none of it: the hooks cost
it no code. They name the page's own variables (`main.ts`, `lattices.ts`, `textures.ts`, `canopy.ts`, `kind_row.ts`): when one is renamed, the
page fails to start in the tool (an error at once), and `HOOKS` is changed with it. `read.mjs` `marks` repeats the
layout of the marks' textures (`textures.ts` `prepare`) and their code (`lib/mark.wgsl` `decide`): change it with them.

The crowns' mask (`mask.mjs`) is the page's programs built again from a copy of `web/shaders` in which the marks of the
lattice of stands (the trees') and the far field of stands are pure magenta. A change to those lines of the shaders
stops the tool with the text it no longer finds; then `PATCHES` is changed with them.

## A saved world in the page

`index.html?world=<URL of a snapshot>` opens it, paused at its date, and the page draws it as it draws a world it
played into. It is the world the page grows from the same seed and flags: both start Flow's state from the ground's
heat (wasm/src/lib.rs `new_world`, cli `ew run`), so their worlds have the same fingerprint at each month
(`tape::fingerprint`). A saved world's cells' table is not the grown one's to the last bit: a snapshot holds no cache,
and Flow's state found again on opening it (`open_world`) differs a little: in the air's fall with height of every
square, the ground's temperature of most, and in fewer, their ice, water and share of land.
