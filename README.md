# Emergent World

A world that runs on five physical laws; everything else must emerge.

- `docs/handover.md`: the plan, the decisions and the rules of work
- `docs/laws.md`: how each law works now, with the code that does it
- `docs/numbers.md`: every free number, its unit and its source
- `docs/tape.md`: what a run records

```
cargo run --release -p cli --bin ew -- run --years 300 --frames-every 5 --out out/run       # simulate: the tape (series.csv, viewer frames) and a snapshot at the end
cargo run --release -p cli --bin ew -- run --years 300 --snapshots-every 100 --pictures-every 50 --months --out out/run   # also snapshots, PNG pictures and months.csv
cargo run --release -p cli --bin ew -- resume --from out/run/snap_00300.bin.gz --years 100 --out out/run   # go on from a snapshot
cargo run --release -p cli --bin ew -- view --from out/run/snap_00300.bin.gz --window 40,10,8,8 --px 800 --layer map --edges --out look.png   # a window of a snapshot's map as a PNG (fractions of a square allowed; cells finer than a pixel are seen)
cargo run --release -p cli --bin ew -- chem                                                   # chemistry 0 as a table
cargo run --release -p cli --bin ew -- numbers out/a out/b                                    # the numbers two runs differ in
cargo run --release -p cli --bin ew                                                           # every command and flag
cargo run --release -p cli --no-default-features --bin ew -- run ...                         # flow and change without threads (same hash)
cd web && npm run build && npm run wasm && python3 -m http.server --directory dist           # live viewer: open http://localhost:8000 (the core runs in a worker; seed and size on the page)
cd web && npm run build && cp ../out/run/world.ewf.gz dist/                                   # tape viewer: open http://localhost:8000/?f=world.ewf.gz, or drop a file on the page
```

Speed: `ew run --size 128 --years 1000` on 4 cores: 2.1 s per year, measured over 20 years (`--years 20 --every 1000 --frames-every 1000`; 2.2 s per year over the last 2 of them, 16,384 cells, no bodies yet), on 2026-11-01 with a load average of 1.9 (`uptime`). Every run prints its own speed line at the end.

A world's cells stay those as built for its whole life, whatever the zoom. Zoom is a matter of
drawing only: the map draws every zoom from the cells (below). The page shows the date, the months per second the world
reaches and the cells. While the world plays, the worker posts its state to the page at most four times a second
(`web/src/worker.ts` `FRAME_MS`): each frame costs the page a repaint of the map. The map is drawn anew only when what
it shows changes: the camera, the view drawn, a new date of the world (a frame of the date shown, as after a pause or
with the things of another window, brings its things and no repaint), a tile of the ground that shows in the view (one
ahead of the view, or under finer tiles kept, waits for a pan).

The map is drawn on the graphics card at the screen's own pixels: with WebGPU where the browser has it, else with
WebGL2. The shaders are written once in WGSL (`web/shaders/`); the build (`npm run build`) checks them and writes them
as GLSL ES 3.00 for WebGL2 too with naga, the shader translator of the wgpu project (`web/shade`, so the build needs
cargo), and puts both in the page. The page hands the card fields, not pictures (a tape's `web/src/fields.ts`, the live
map's tiles and table): of each square or texel the ground's height and slope, the liquid's surface over the ground, the ice, the ground's temperature, the
share that is sea and that is a river and the river's depth, how wet the ground is, the plant cover's byte, the true
colour and every layer. The card turns them into colours at each pixel of the screen (`web/shaders/lib/colour.wgsl`:
the Earth view, the true colour and the layer views, one rule for a tape's map of squares and for the live map),
with the light on the slopes. Water is water (`web/shaders/lib/water.wgsl`): a flat surface, its bed seen through it
and fading with depth as water absorbs red light most and blue least (Beer–Lambert), into the dark blue of deep water;
a little of the sky reflects from it and the sun glints on small ripples fixed in the world, which fade out as they
shrink below a few pixels. A sea, a lake and a river are the same water, a river as deep as its discharge makes it and turbid with what it
carries.
The ground goes from dry to wet continuously: soaked bare ground is darker (to half as bright), wet by how wet the core
says it is and by the capillary fringe over the liquid's surface (wet to e^-1 half a metre over it), so a flat shore
is a wide wet band and a steep one a thin line; near soaked, a film of liquid shines on it as mud, and at the
waterline it is the bed under no water, so the shore has no drawn line. It also names each square's class (sea, lake, forest...)
for the inspect panel. Avatars, the lines of latitude, the outlines and the scale bar are drawn on a 2D canvas over it.
While a frame takes the card longer than 16 ms, the map is drawn at fewer pixels (down to a quarter along each side)
and stretched onto the screen, the same picture, softer; once the view rests it is drawn again at the screen's own
pixels, a tile a frame, and shown when whole. The page never waits for the card: what it reads back (the classes, the
colours under the avatars) comes a frame or so later. Seen from afar, most of the drawing's detail is smaller than a
pixel and shows nothing: the build makes the live map's program once more for draws whose pixels take at least 0.5 m,
1 m and 3 m, with that detail compiled out (`web/shade`, `LEAST_E` in `web/shaders/lib/draw.wgsl`), and the page draws
with the one made for the largest no more than its pixels take (`web/src/gpu.ts` `seenFrom`). The picture is the same
to the bit; the card runs and holds less code.

The live viewer draws every zoom from two things (`web/src/land.ts`, `web/shaders/land.wgsl`). **The ground's tiles**
(`tape::Ground`) never change: each holds 256 x 256 texels of a depth, from the depth at which one tile holds the
island down to texels of 1 m (closer, the finest is magnified), with the ground's height (the terrain itself, smooth
below a metre), the wetness index of the place and the channel nearest it, as 16-bit floats (16 bytes a texel, the
height to a millimetre). Workers of their own, each with its own copy of the core, make them (`web/src/tiles-worker.ts`;
two where the browser has more than four cores, else one), so the world never waits for a picture; the page keeps at
most 56 on the graphics card (61 MB) and no copy of its own (after a lost context it asks for them again), and asks for
those about the view, at the depth whose texels take no more than a pixel of the screen (a depth coarser while that
would touch more than 30 tiles), at the depth above and at the coarsest, coarse before fine and nearest its middle first (`web/src/ground.ts`); a request that left the view is
dropped, and the tiles drawn least lately go first. A tile not made yet is drawn from its nearest ancestor, stretched: the picture is soft for
a moment, never the squares; a new tile fades in over it in a quarter of a second. **The cells' table**
(`tape::Table`) comes with each world frame: a row a square of what the laws inside a cell read. At each pixel the card
blends the table between the squares' centres and applies the laws inside a cell there: the ground's temperature is
its cell's less the air's fall with height times the pixel's height over the cell's mean; the frozen cover covers the
ground as its mean depth does (tanh of the depth over 8 mm of ice, about 3 cm of snow, Yang and others 1997), less
where the pixel is warmer than its melting point, and that share of its cells' ground is placed where the ground holds
snow longest (high ground, slopes turned from the sun, hollows: `snow_bares`, `snow_at` in `web/shaders/lib/land.wgsl`),
so a snow line is a sharp line of the ground, not a blur between the cells; ice on standing liquid lies first over
shallow liquid, its edge broken at every scale, its margin floes of two sizes with leads and brash, the closed pack
grey-white and flat with refrozen cracks and ridges, a tide crack along the shore (`web/shaders/lib/ice.wgsl`); the open
sea's ice and temperature are blurred over its 8 km cells first, and the land a coastal square of one holds takes the
land's own height, temperature and ice (`fromLand` in `web/src/land.ts`), so no edge follows a cell's border; liquid stands at the level of the ponds of the texels about the pixel where the ground drawn lies under it, the ground drawn near a shore leaning to a smooth line through the samples (a contour of their blurred heights, `tape/src/ground/shore.rs`), so a coast has no scallops a sample wide and its shallows are one band, and the water's edge covers its share of the pixel's footprint, so a shore is a smooth line at any zoom; the soil's liquid departs from its cell's by `soil_m` per unit of the wetness index over
the cell's level, within none and the pores full, as the core shares it; a channel shows where the pixel lies nearer
its line than half the width the channel law gives the discharge of its own sample (the core's `World::discharge`,
which comes with a frame when it has changed enough to show), its share of the pixel what the pixel's footprint
covers. Its line winds between the samples' centres by a law: a bend every 11 widths (Leopold and Wolman 1960), where
the valley's slope is low and within the valley's floor, none where it is steep (Montgomery and Buffington 1997). Its
bed is a parabola cut into the ground, as deep at its middle as its mean depth makes it (its banks catch the light),
and where it runs into a lake or the sea it runs on through their band of shallows, widening, its bars going under
and its water melting into the open water past the band (a rill ends at the water's edge); over it runs water that carries
what it wears and washes, turbid as inland water is (1 to 4 per m, Kirk 1994), over a bed of washed sand and gravel
under a film of algae: a brook shows its stones, a river a metre deep is dark water. The ground beside it is wet by
the capillary fringe over its water. From afar no channel whose discharge matters at that zoom is thinner than a line
of about a css pixel: one shows as a line once it passes 6.4e-3 m³/s for each metre a css pixel takes (the drained
area that shows grows with the zoom, so the whole island shows its main rivers, a closer view their larger branches,
and none a mesh of rills), a little wider as its discharge grows (to the power 0.5, up to 2.5 times); a channel is drawn as wide as its bed
wherever that is wider than its line, so its width changes evenly with the zoom.
Close up a channel's bed is as wide as the channel law makes five times this month's discharge (the bankfull flood,
Leopold and Maddock 1953; Dunne and Leopold 1978), bare of plants; its line wanders about its valley's and on gentle ground swings in a meander belt
(bends of many sizes, some reaches nearly straight, here and there a loop; on a steep reach nearly straight stretches
between short sharp turns: `tape/src/ground/belt.rs`). This month's
water fills most of it (at one place the water's width grows as its discharge to the power 0.26: Leopold and Maddock
1953): a pool against the outer bank of a bend over a bar of washed gravel on its inside, a wider riffle between
bends; what runs through it grows evenly from sample to sample, so it widens downstream and below each tributary
without a step. In the Earth view the water is coloured by its bed's shape, light shallows over the bed's stones and a
dark thread where it is deepest; a bar is pale gravel, wet at the water's edge, with the drift lines the falling
water leaves and tufts of plants on its top; a bank is a low scarp the sun lights or shades. The water shows which
way it runs (`web/shaders/lib/flow.wgsl`): streaks along the flow, many over a riffle and few on a pool, and on a
steep reach white water that starts at a step and fades down the flow. These signs slide down the channel in six
steps a second, faster where the reach is steeper, and come round with no jump; the page draws those frames only
while a channel in view shows them and it keeps up at the screen's own pixels, else none until something changes. The ground has relief finer than its tiles (`web/shaders/lib/relief.wgsl`): octaves of gradient noise from
48 m down to the pixel, fixed in the world, each fading in as it grows past a few pixels, smooth swells on deep loose
ground and ridged crags on bare bed, the rougher the steeper; it moves the shading and the line of every shore. A slope turned away from the sun is
lit by the sky alone, the less the steeper (an even sky, Liu and Jordan 1963), so shaded ground and snow keep their
shape. Nothing depends on the view or a tile, so a
place has the same colour after any pan or zoom. The stats line gives the tiles kept, their size and how many in view
are still to come, and which of WebGPU and WebGL2 draws. A tape holds no ground: its map is drawn from its squares.

On the live map the Earth view draws the plant cover plant by plant from its numbers, with no classes
(`web/src/vegetation.ts`, `web/src/canopy.ts`, `web/shaders/cover.wgsl`). With each frame the core sends, for each
square, up to 4 kinds of cover (the slots of its cell) and of each its plants as the cover's law holds them
(`tape/src/stands.rs`, laws.md 4.5): as many a m² as the slot counts, as tall as the buckling limit lets their stems
stand (McMahon 1973), each crown as wide as the ground it holds, in patches that cover as much of the ground as its
leaves close; the loose ground their roots need and the share of the square that holds them; and its genes and state
this month. A plant is drawn on one of three lattices fixed
in the world (cells of 0.1, 0.5 and 2.5 m), where its marks take a few pixels; where they take less, the same plants
are a field of their colours, drawn as crowns no smaller than the eye reads (each then a clump of plants) and far away
as their mean, so the far and the near pictures agree. A mark picks
its kind at its own foot from the four squares about it (blended as the map is) by the kinds' chances there, fewer
where the ground is drier than its cell's mean as each kind's activity law says: kinds mix mark by mark across a
square's edge, and a kind that needs wet ground gathers in the hollows. What a mark is depends on its cell alone, so
before the map is drawn each mark in view is decided once, a texel a cell in textures of four bytes a texel, every
number kept to the bit (`web/shaders/marks.wgsl`, `web/shaders/lib/mark.wgsl`), and each pixel reads the marks of the
cells about it. Trees grow in stands of their kinds, patches of
about 60 m fixed in the world that the far field draws alike. A kind's roots need the loose ground its slot says
(`Cover::need`): where the loose ground at a point is thinner it thins, by the cover's own rule and number
(`rooted_from`, sent with the laws' numbers), so bare bed holds only the smallest kinds in its cracks, thin ground herbs
and low bushes, and trees stand where it is deep; what a square's thin part does not hold stands on the share that holds
it (`Cover::rooted`), so the square keeps its cover (near and far alike). None stands in water (liquid deeper than a film of
5 cm over the ground drawn; under that film the ground is soaked and keeps its plants). Snow buries a plant by its depth (three times the depth of the cell's frozen cover,
settled snow against ice) against the plant's height: a herb is gone under a deep snow, a tree stands in it, its
crown snowy on top if it keeps its leaves, bare if it sheds them, its stem and shadow kept; from afar a forest in snow
is dark trees over white ground. A tape's map has no heights of plants: snow hides its foliage as far as it covers the
ground. A tree's crown takes one of six forms by its
kind's numbers (`web/src/form.ts`, one rule for near and far): round; a cone (thick long-lived leaves on a kind hardy to
frost); a slim column (a slender kind that puts much into seeds); a flat umbrella (dry ground, a crown wide for its
height); tiered (thick leaves of a tender kind); or lobed, in loose knots (thin leaves on wet ground). A form drawn less
surely lies nearer the round crown, so a kind that evolves never jumps. Its edge is in lobes as broad as its leaves,
open where its leaf area is low, over its stem and a soft shadow to the south-east; each tree leans a little its own
way; angled, it stands on its stem in its form's outline (a cone's stepped sides, a tier's layers, an umbrella's flat
underside). A low woody kind is drawn as a bush, a low knot of lobes with dots of flowers or fruit. Herbs have no
marks: they are the ground's sward (below). Its
physical colour is a reflectance: a green leaf's as measured (Gates and others 1965; Knipling 1970), moved by its
numbers (darker with more light capacity per capture area, darker and duller with thicker leaves, greyer for dry
ground, bluer for hardiness to frost, its walls showing through an open crown, yellowed short of fuel). Far away a
tall, open canopy is darker by the shadows its plants lay between them.

The Earth view takes one style step over the physical colours, a hand-painted map's, with all of its numbers in one
place (`web/shaders/lib/style.wgsl` for each pixel, `web/src/style.ts` for the plants' colours); the true view and the
layer views keep the physical colours. Every look still means what the model holds. The colours are cleaner, with
more contrast; slopes have a clear lit side and a soft cool shade. Water is coloured by its depth through the clear
water of the clearest coasts, drawn stretched, so the bed shows to tens of metres: sand, pale shallows, turquoise,
blue, then the deep, with the sea's foam along its edge (close up a line at the water's edge and a few soft broken lines that follow
the shore), which stops at a river's mouth; a lake and a river read the same way at their own
scale (as if one and a half times as deep and 2 m more, so the first tens of metres of depth read light, lighter over their shallow rim and banks), and a lake's shore shallows to
none across its last texel, so its outline is soft. Under any water the bed is drawn lighter. A beach lies on gentle loose ground a few
metres over the sea (lower by a lake), none on a steep coast. Each kind of plant has its own green from its
numbers (thin leaves yellow-green, thick ones blue-green, the dry ground's kinds grey-green, tender kinds warmer,
woody kinds darker), pushed far apart and lighter than real leaves; each plant differs a little in hue, tone and
size by its own fixed random numbers. A woody kind with thin leaves sheds them (thin leaves are short-lived:
Kikuzawa 1991): as its ground cools within 25 K of its frost gene it turns through a pale form of its deepest colour
to it, yellow, orange or red as its pigment per leaf area reaches, plant by plant at its own time and hue, then stands
brown and thin while dormant, and greens again as the ground warms. An evergreen kind (thick leaves) and the herbs turn
only where the cold shuts them down (frozen ground, their frozen share); a kind shut down by drought goes straw, then
brown. Far away the plants melt into a field of the same colours, with soft patches
fixed in the world and no rows; the trees' canopy in their stands' colours, lighter and darker over groups of crowns,
its stands fading into their mean from a 500 m scale bar on; the herbs and low plants as one layer from a metre a
pixel; thin loose ground is poorer, its herbs sparse over the ground's own colour.
The herbs are a sward over the ground at every scale, covering it far more than their share (a fifth of herb cover
reads as two thirds green: their leaves lean and shade), in patches of their kinds, drawn about as light as the trees
and less saturated so what stands on it reads by shape and shadow, with tufts, swathes and blades where each shows
(none past a metre a pixel), darker in the trees' shade, as a band of dense reeds on soaked ground and within 5 m of
the edge of standing or running water, and with their flowers as clusters of light dots (a soft tint from afar); the bare ground has clods, and under closed crowns their shade, litter and moss. Slope alone never bares the ground: where the bed shows (below) it is drawn, grey and mottled,
as slabs with cracks where it is gentle and as a cliff's face where it is steeper than about 0.9, with strata as broken
dark ledges along the contours, cracks and a strong light and shade; scree lies on steep thin ground, stones on bare
bed and in a channel as big as its power moves (`web/shaders/lib/crag.wgsl`). One maker draws every stone
(`web/shaders/lib/stone.wgsl`): a block cut by five planes, so no stone is a disc, longer one way than the other, each
face lit as it is turned to the sun, with a shadow on the ground, in its bed's colour with the bands of its layers;
angular on scree and bare bed, the rounder the more a channel carries (the travel wears it), in three sizes with more
of the smaller, its foot sunk in loose ground, under the snow unless it is large, then with snow on its top. In a
channel a stone taller than the water is deep stands out of it, and fast water breaks white on it and trails behind
it down the flow. About 3% of seed 1's land and 2% of seed
2's is bare and that steep.

In every view the bed shows where the loose ground thins (`web/shaders/lib/rock.wgsl`): each terrain sample's loose
depth, the largest stone its channel moves and the reflectance of its bed's make (its substances' by volume) come with
the ground (`Ground::rock`), so bare rock shows the bands of the folded layers at the samples' own place; the loose depth departs from the samples' in patches from 40 m down to the pixel (`loose_seen`), so outcrops have ragged edges; the bed shows fully at no loose ground and is hidden under 0.1 m, and plants hold on it only in its
cracks. A body that holds no liquid is drawn as dry ground. Every colour of the map is worked in linear light (a
reflectance times the light on the surface) and turned into the screen's bytes once, by the sRGB curve (IEC 61966-2-1).
Under the plants the ground has its own colour (its loose solids, else its bonded ones, under the litter of the
cover's remains: the `ground_r`, `ground_g`, `ground_b` layers), darker where wet, under liquid and snow; the green comes only from the plants. A tape
holds only totals per square and no kinds: its map draws the plants as one foliage as closed as their capture area
closes the ground, without marks.

From 64 pixels a square, the live viewer draws the bodies and parts of each cell one by one at their real size, from
the numbers the core hands out (`tape::things`): a member is as long as 1.5 times the cube root of its volume (a body
as dense as water), each member at its own seeded place across its cell's real area, so it holds while the camera
moves. A member of 3 pixels or more is drawn as its shape (from 8 pixels with a rim that stands out from the ground),
smaller as a soft dot. One or two doublings of the zoom before a member takes a pixel, its node shows as a light, soft
blob over its cell, a little stronger the more members a m² holds, lighter than dark ground and darker than light
ground; it fades in as the zoom comes near. Farther out no herd is drawn. Parts are drawn at their own sizes, hidden below a pixel. Two
styles draw the shapes (the "Life" buttons, kept per browser): from above (a body with its head, wider the stronger) or
angled (a block on legs, as a small axonometric sprite). Hover or click one to see its numbers; the inspect panel
gives a herd's count. The map zooms to 80 pixels a metre, so one animal shows its shape and can be picked.

By default the animals start with hand-set brain biases (`--neutral-eaters` for neutral brains) and the
light-eaters with no brain. When life starts (year 2) the cover is sorted by its own law on the world's own year before
it is placed (the nursery, `docs/laws.md` 8): the land falls in classes of like climate; in each a garden races
kinds over the genes' whole range, grows the best with the hand-made kinds into a stand and lets newcomers try to
enter it; each cell then takes the classes' stands mixed by how near its climate is to each. The page says "Sorting
the plants..." while it runs: about a second here, a few in the browser.
