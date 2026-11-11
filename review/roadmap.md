# The next stage: one fixed world, a map on tiles, a ground with a history

For the owner and the next orchestrator. Simple English.

This is a review of the code only. Nothing was built and nothing was run. Numbers marked "est." come from reading the
constants, not from a measurement. The first job of the next stage is to measure them.

## 1. In short

The six problems the owner sees have four roots. All four are strategies, not small bugs.

1. **The cells of the simulation follow the camera.** This makes the world slow, makes it flicker, and gives the
   picture nothing: the map already draws every zoom from a law inside the cells, not from the fine cells.
2. **The fine picture is made on the CPU, as one big window, in the same worker that steps the world.** Every pan or
   zoom makes all of it again. While it is made, the world waits.
3. **The ground is noise, with every lake basin filled, and with the same random bumps laid on it everywhere.** So
   there is no valley, no lake, no river bed. And the ground drawn never changes, so erosion can never show.
4. **The viewer sorts plants into seven fixed classes and draws each as a disc.** The genes do not reach the picture.

The new shape, in one line each:

- **A.** The simulation runs on fixed cells. Zoom is a matter of drawing only.
- **B.** The map is drawn from tiles of the ground (they never change, so they are made once and kept) and from small
  tables of what the cells hold. The laws inside a cell run on the graphics card, per pixel.
- **C.** The ground is made once, at the world's start, by running the erosion law for a long time. Lakes stay lakes.
- **D.** One drainage network, on the ground's own grid, for the simulation and for the drawing.
- **E.** A plant's look is a function of its numbers: its kind, its wood, its leaves, its state.

## 2. Where each problem comes from

### 2.1 The simulation is a lot slower

| Cause | Where | Size |
|---|---|---|
| The page runs on one core: the wasm build has no threads | `wasm/Cargo.toml`, `core/src/par.rs` | all of it |
| The fine picture is made inside the step loop, once a second while the world plays | `web/src/worker.ts` (`FINE_MS`, `postFrame`) | 0.35 to 1.1 s each: a quarter to a half of the time (est.) |
| Each tick builds a full frame: 25 layers for all cells | `tape/src/layers.rs` (`frame_now`, `Ctx::new`) | small, but every 50 ms |
| The camera splits and joins cells: up to 32 changes a month | `core/src/refine/mod.rs` (`MOST_CHANGES`) | a year goes from 1 s to 3 to 7 s (`review/plan.md`) |
| Each change rebuilds the whole mesh and every array of every cell | `core/src/remap/split.rs`, `remap/mod.rs` | 4.6 ms a change, measured |
| After a change, all of Flow's working memory is dropped and built again for the whole world | `core/src/flow/mod.rs` (`Scratch::remap`, `with`) | a full build each month |
| One fine cell makes the deep store's flow take sub-steps, and each sub-step works over all cells | `core/src/solve/sweep.rs` (`by_size`), `flow/deep.rs` | about 1,500 sub-steps a month at 7.8 m (est.) |
| A cell with land and liquid ("parts") is solved 6 to 14 times a month and never counts as at rest | `core/src/flow/parts/mod.rs`, `parts/node.rs` | Flow costs twice what it did |
| The state of all ground is built in full each month, with two sums of the potential energy of the world | `core/src/survey.rs`, `flow/mod.rs` | about a tenth of Flow (est.) |
| Work for animals runs with no animals | `choose/features.rs`, `survey/steps.rs` | small |

The first six rows exist only because the cells follow the camera or because the picture is made in the wrong place.

### 2.2 Panning and zooming are slow, tiles load slowly, trees cover all and then go

- The page keeps **one** fine window: the view and a fifth more on each side, at most 360,000 pixels
  (`web/src/main.ts`: `FINE_MARGIN`, `FINE_MOST`). There is no store of tiles.
- Any change of the view asks for a whole new window. It comes 150 ms after the view rests, plus 0.35 to 1.1 s, plus
  the wait for the step loop.
- For each window the core makes **25 layers**, each through a pyramid of filters (`tape/src/fine/pyramid.rs`), and
  about 30 smoothing passes (`tape/src/fine/keep.rs`). The earth view reads about 8 of the layers.
- Past the edge of the window the page shows the map of squares: one texel for 2 km. The plant marks there come from
  the square's totals. They know nothing of water, rock or wet ground inside the square. So **trees stand on
  everything**, and they go when the fine window comes (`web/src/vegetation.ts`, `draw`). The two fields also do not
  mean the same thing: one counts stored fuel in the mass, the other does not.
- Each frame, each pixel runs a heavy program: 7 grids of marks, 9 places each, up to 4 texture reads each
  (`web/shaders/cover.wgsl`), and a 16-tap curve for the ground. When a frame takes more than 16 ms the page draws at
  a quarter of the pixels and sharpens later (`web/src/map.ts`). While the world plays, every world frame draws again,
  so soft and sharp pictures follow each other.

### 2.3 Edges of cells show, things flicker

| What is seen | Cause | Where |
|---|---|---|
| Water ends at cell edges | Each cell is its own bath: it holds its own liquid at its own level over its own ground | `core/src/flow/curve.rs` |
| Lakes step from cell to cell | Only the sea is brought to one level. Lakes above it are not | `core/src/flow/basin.rs` |
| Forest ends at cell edges | The drawing writes a kind only into the pixels of cells that hold it | `tape/src/fine/inside.rs` (`cover`) |
| The look of trees jumps at edges | Plane `k` is "slot `k` of this cell": another kind in the next cell. Size and tone come from a table per cell | `tape/src/fine/inside.rs` (`extend`), `web/src/canopy.ts` |
| A kind cannot cross into a full cell | A cell holds 4 slots. Seeds of a new kind die unless they outweigh a whole slot | `core/src/cover/seed.rs` (`lodge`) |
| Snow in blocks | A temperature field gets each cell's own remainder at the end: a step at its edge | `tape/src/fine/keep.rs` (`keep`, `Mean::Shifted`) |
| Wet ground and plants change when the view moves | The wetness index counts only the ground inside the window, and uses the window's mean. The pixel size follows the zoom | `tape/src/fine/inside.rs` (`wetness_index`, `soil`) |
| Detail differs after zoom out and in | A join sums the cells and loses kinds past 4 slots. A split shares out again | `core/src/remap/`, `cover/tree.rs` |
| Seasons flash | One picture a second at 12 or more months a second | `web/src/worker.ts` |
| The view is a patchwork of cell sizes | The page asks for cells of 6 pixels. A screen needs about 27,000 (est.). The budget is 1,500, filled depth first | `web/src/main.ts`, `wasm/src/lib.rs`, `refine/mod.rs` |
| After a pan, new detail waits about 20 months (est.) | Joins of the old window run first and use up the month's 32 changes | `core/src/refine/join.rs` |

### 2.4 The water is not real

- **No normal lakes.** At the world's start a flood fills every closed basin of the ground with ground, at a slope of
  1 m in 1 km (`core/src/space/drain.rs`, `samples.rs`). Every basin of 125 m or more becomes a flat. No lake can
  form there.
- **A pond in every hollow.** Below 125 m the ground gets random bumps that are never drained
  (`core/src/space/below.rs`). On a filled flat the bumps are far larger than the slope. A fine cell that is low is
  its own pond. The drawing also wets every pixel under its cell's level, with no test that the water joins up
  (`tape/src/fine/mod.rs`, `standing`).
- **Rivers are straight.** The path drawn is the steepest way between the centres of the 125 m samples, in 8
  directions, with round corners (`core/src/space/network.rs`, `tape/src/fine/channels.rs`). On a filled flat that is
  a straight line. The path knows nothing of the ground drawn below 125 m.
- **Two networks.** The simulation moves water over the cells of the moment by another rule
  (`core/src/flow/runoff.rs`, `solve/downhill.rs`). The drawn river is not the simulated one. There are three rules
  for the way water runs and three floods in the code.
- **No erosion to see.** The law exists (`flow/runoff.rs`: `load`, `wear`). But it lowers the mean of a whole 2 km
  cell, by about 2 mm a year under the largest river (est.). No valley can form. And the ground drawn is a function
  of the seed alone (`tape/src/fine/heights.rs`), so it would not show anyway. The world's start has no erosion.

### 2.5 Plants are green balls

- The viewer sorts each stand into grass, tundra, steppe, shrub, woodland, forest or jungle by fixed limits of mass,
  wood, temperature and rain (`web/src/canopy.ts`, `standOf`). These are symbols, not matter.
- Every tree is one soft disc with a shadow (`web/shaders/cover.wgsl`, `crown`). Seven fixed colours
  (`web/src/vegetation.ts`, `FOLIAGE`).
- Genes reach the page only very close (64 pixels a square), as **one** tint for a whole 2 km square, from its
  heaviest slot. Grass and trees there get the same tint.
- The model itself has little to show: the three parts of every plant have the same colour in chemistry 0, and a
  plant has no height, no age and no count. The live world starts with 3 kinds, and mutation is too small to found a
  new one (from reading; a tape should check it).

### 2.6 The ground looks like bad noise

- The base is value noise in 6 octaves, with ridges and a warp (`core/src/space/terrain.rs`). All octaves sit on one
  grid with the same axes. There is no geology: no valley cut by water, no fan, no plain.
- The noise is sized to the map, not in metres: the live island of 22 squares has features 3 times smaller and
  steeper than the island of 64.
- Below 125 m each square of the tree gets four random draws, scaled to **the same** roughness everywhere: on a
  plain, on a peak, under the sea (`below.rs`, `terrain.rs`: `real_slope`). The draws sit on the squares of the tree.
- The map shades slopes 6 times steeper than they are (`web/shaders/lib/earth.wgsl`, `EXAGGERATE`).

## 3. The strategies that did not pay

**3.1 "The cells of the simulation follow the camera" (work packages 5 to 12).** The idea was: detail where we look.
For land, water, air and plants it gives nothing:

- The fine picture comes from the law inside a cell (`inside`), drawn smooth across the cells. It is drawn the same
  way over coarse cells. The page "draws every zoom from the fine raster" (`review/orchestrator.md`).
- The project's own report says that nothing new appears at fine grain (`docs/reports/fine.md`).
- It costs the six rows of 2.1, more than 2,000 lines of code, and the rule that the world depends on where one
  looks.
- It cannot even fill the view: 1,500 cells against 27,000.

**3.2 "The drawing must keep each cell's mean, and is made per window on the CPU."** This gave about 430 lines of hard
mathematics (`pyramid.rs`, `keep.rs`), run for 25 layers at each window. The books are kept by the core. A picture may
be a few percent off the mean. The rule that it may not is what puts the step at cell edges (the remainder in `keep`).

**3.3 "The ground is one function of the seed, at any depth, and never changes."** It is cheap and neat. But it means:
basins must be filled (noise has basins no water cut), detail must be random, and erosion can never show.

**3.4 "Speed up what runs."** Whole stages went into making splits, sub-steps and the raster faster. None of the three
needs to run. The same holds for the next ideas in the plan (a smooth mean-keeping field, a faster raster, cheaper
sub-steps): they tune parts that the new shape removes.

**3.5 "One more rule for each thing seen."** Channels drawn over the ground, a nudge of 1e-6, a region of 4 x 4
samples for the flux, three limits for a jungle. Each was added to make a picture right, and each is a place where
the picture and the world can differ.

## 4. The new shape

### A. Fixed cells for the simulation

The cells stay as built: squares on land, large cells in the open sea. The camera changes nothing in the world. The
same seed gives the same world with no camera path. Detail for animals and people comes back later in another form:
a group splits into single bodies, and each body reads the ground at its own place by the same law the map draws by.
The ground's grid does not split.

When the core is faster, the cells can be made smaller everywhere (1 km, 500 m). That is a number of the run, not a
thing that changes while it runs.

### B. The map from tiles

Split what is drawn in two:

- **What does not change** (the ground): height, slope, drained area, distance to a channel, which lake a place
  belongs to. It is a function of the place alone. It is made in tiles of 256 x 256, level by level, each level twice
  as fine. A tile is made once, in a second worker, and kept on the graphics card. Nine levels go from the whole
  island to under 1 m.
- **What changes** (the state): one small table with a row for each cell: liquid in the soil, snow, temperature, the
  mass and the numbers of each kind of plant. Plus the level of each lake and the flux of each river. A few kilobytes
  a frame. It is sent with each world frame.

The graphics card joins them per pixel. It blends the cells' values between their centres, then applies the law inside
a cell at the pixel: soil liquid by the wetness index of the place, snow by the height of the place, each plant kind by
how wet the place is. Nothing depends on the window, so nothing flickers on a pan. A tile not made yet shows its
parent, stretched: the picture is soft for a moment, never wrong. The world never waits for a picture.

### C. A ground with a history

At the world's start the noise is no longer the height. It is the **uplift**: where the ground rises. Then the build
runs the erosion law for a long time on the grid of samples: rivers cut by their discharge and slope (the stream
power law, Howard 1994), slopes creep (the law the core already has), what is cut settles in lakes and at the coast.
This gives valleys that branch, ridges between them, plains at river mouths. The basins the rivers did not yet cut
through stay: they are the lakes.

The ground is then **state**: a grid of heights kept in the world, which the slow erosion of the run can change.

### D. One drainage network

The network on the ground's grid is the only one. The simulation routes each cell's runoff down it. The drawing reads
the same paths and the same flux. A lake is a body with one level, as the sea is today; the sea is the lowest lake.
The monthly flood over the cells, the rules for rims, and two of the three rules for the way water runs go away.

### E. Plants from their numbers

The core sends, for each cell, each kind it holds with its mass and its numbers (wood share, leaf area per kg, light
capacity per leaf area, frost limit, wetness need, seed share, stored fuel, whether it is active). A kind has one
name over the whole map, not a slot number in a cell. The page blends the mix of kinds between cells, picks the kind
of each mark from that mix, and builds its shape and colour from the numbers. No classes.

## 5. The roadmap

Each stage ends with the gate of `CLAUDE.md` 6.3 and with pictures at the owner's views. Tracks A, B and D can run
side by side; they touch different folders.

### Stage 0: switch off and measure (small, first)

No law changes. The core stays bit for bit.

1. The live page runs with no cell budget (`wasm/src/lib.rs`): no splits, no joins.
2. The fine window is made only for the layers the view reads, and not inside the step loop each second.
3. A frame holds only the layers shown.
4. Skip the brain features and the body sub-steps when no body lives.
5. Write down the pace (months a second) before and after each of these, on the same seed.

Done when: the numbers are in the stage's report, and the owner has looked at the page. **Check:** is anything worse
without fine cells? If yes, name it. It becomes a need for stage A2, not a reason to split cells again.

### Track A: the map on tiles

| Step | Goal | Goes away | Done when |
|---|---|---|---|
| A1 | Tiles of the ground, made in a second worker, kept by level; a parent shows until its child comes | the one fine window for the ground | a pan and a zoom never show the squares' picture; the world's pace does not drop while tiles load |
| A2 | The cells' table and the law inside a cell on the graphics card | `tape/src/fine/` (pyramid, keep, inside), `fineFields`, the second cover field | no cell edge at the 200 m, 50 m, 20 m and 10 m views; the same pixel has the same colour after any pan |
| A3 | Seasons blend between two world frames; far away, plants are a field, not marks | the soft and sharp pictures in turn | a pan stays smooth while the world plays, on the owner's phone |

### Track B: the ground and the water

| Step | Goal | Goes away | Done when |
|---|---|---|---|
| B1 | The ground made by the erosion law at the world's start; kept as a grid; noise in metres | filling basins to flats (`space/drain.rs`), the random bumps (`space/below.rs`) | pictures: valleys that branch, a few lakes of 0.5 to 10 km, no field of ponds |
| B2 | One network; lakes as bodies with one level; runoff routed down the network | the monthly flood, the rims, `basin.rs` as a special case, `remap/through.rs` | a river followed from spring to sea lies in its valley at every zoom; the books close to 1e-9 |
| B3 | Mud and sand (spec 19): what a river carries, and where it settles; the slow cut of the run changes the grid | erosion spread over a whole cell | a delta grows at a river mouth in a long run; a tile is made again where the ground changed |

### Track C: plants (after A2)

| Step | Goal | Done when |
|---|---|---|
| C1 | The table of kinds per cell; the mix blended between cells; a mark picks its kind | forests cross cell edges; no step of size or tone at an edge |
| C2 | Shape and colour from the numbers; shapes baked once per kind | two kinds with different numbers are told apart at a glance, far and near |
| C3 | Law changes, one commit each: a new kind can enter a full cell; the parts of a plant differ in colour; more starting kinds or a mutation that can found a kind | a tape shows how many kinds live at year 300, and a long run keeps the books |

### Track D: the core's own speed (any time)

1. One solve of a cell's parts at the end of the liquid step; a parts cell at rest is skipped.
2. The full build of the ground's state only when a world is loaded.
3. The level curves: sums kept, no sort at each neighbour.
4. No new memory in the daily loops.
5. The wasm build with SIMD and `wasm-opt`.

Not on the list: cheaper sub-steps, cheaper splits, a faster raster. Stage 0 and track A remove them.

### Stage E: the owner decides on the tree of cells

With animals back in view: keep the code for splitting cells, or delete it (section 7). Until then it is off in the
page and stays in the core.

## 6. The hardest parts

### 6.1 The erosion at the world's start (B1)

- **The law.** Height changes by uplift, less K x (drained area)^m x slope^n, plus creep. With n = 1 one step is
  solved exactly, cell by cell, from the sea up the rivers (Braun and Willett 2013):
  `h = (h_old + uplift x dt + f x h_downstream) / (1 + f)`, with `f = K x dt x area^m / distance`. It is stable for any
  step, so 50 to 200 steps make a ground.
- **Take m = 1/2.** Then the law needs only a square root. `powf` gives different last bits on different machines; a
  square root does not. So the page and the command line make the same world from the same seed.
- **Basins.** The flood today raises the ground. The new one only finds, for each basin, its lowest pass, and sends
  the water over it (Cordonnier, Bovy and Braun 2019). Inside a basin the river drops its load; it does not cut. A
  basin that the load fills becomes a plain. One that is left is a lake.
- **Hard and soft rock.** K follows the hardness of the bed, which the build already draws as patches. Hard rock then
  stands as ridges. This is where the look of each world comes from.
- **Cost.** The live island has 352 x 352 samples: a second or two (est.). The island of 64 has a million: run most
  steps on a grid 4 times coarser, then a few on the fine one.
- **Risk.** The numbers (K, the time, the uplift) decide if the land looks young, old or flat. Plan a few days of
  pictures. Each number gets its row in `docs/numbers.md`.
- **The loose ground** is no longer 1 m everywhere: it is what the build left, thin on ridges, deep in valleys. The
  map's rock and soil (work package 19) then come from matter.

### 6.2 The ground below a sample (A1, B1)

It is for drawing only. It must have no pits, no pattern of squares, and no seams between tiles. So each term is a
function of the place and of the world's grid, never of a tile or a window:

- a smooth surface through the samples (the quartic the code has), that makes no new pit;
- the river's bed cut into it: as wide and deep as its flux says (the channel law the core has), along a path that
  bends more where the slope is low (a bend about every 11 widths, Leopold and Wolman 1960). The path is kept as
  "distance to the channel" in the tile, so a river is sharp at 1 m;
- roughness only where rock is bare or the slope is steep, from gradient noise in metres, none on deep soil.

The hard point is small streams: under which flux a stream is not drawn. Start with the streams the grid holds.

### 6.3 Lakes with exact books (B2)

- A lake is the set of samples of one basin under its pass. Its curve of volume by level is made once.
- Its liquid stays in the ground nodes of its cells, as today. Once a month the cells of a lake are brought to one
  level by the code that does it for the sea (`basin::share_out`, `curve::level_over`). The sea is not special.
- When a lake rises over the pass to the next basin the two are one lake. The basins form a tree (Barnes, Callaghan
  and Wickert 2020); the curve of each joined lake is made once too.
- A lake in a dry place shrinks and leaves its salt. This comes free: salt already moves with the liquid.
- Risk: the potential energy of each move must be booked as for the sea. Reuse that path; write no second one.

### 6.4 Tiles and workers (A1)

- Threads in wasm need a shared memory that most hosts of a single page do not give. So: two workers, each with its
  own copy of the wasm. The second one only makes ground tiles. It needs the seed and the ground grid, not the world.
  The grid goes to it once (half a megabyte for the live island).
- A queue of tiles: what is in view first, coarse before fine; a request that left the view is dropped.
- A cap on the tiles kept (about 150), the oldest out first.
- Each tile has two texels more on each side, so slopes and blends are right at its edge.
- A parent tile must be the mean of its children. The ground function already gives means over squares of the tree.
- When erosion changes the grid, the tiles over that place are made again. This is rare.

### 6.5 The law inside a cell, in one place (A2)

- Today it lives twice and differs: the core shares soil liquid by a straight line with limits
  (`core/src/inside/shares.rs`), the drawing by an exponential (`tape/src/fine/inside.rs`).
- New: the formula is written once for the core and once for the shader, with the same numbers sent from the core,
  and one test gives both the same inputs and compares them.
- The core needs it only as a table per cell (how much of the cell is how wet), made from the ground once.
- The mean of a cell: the shader divides by the cell's mean of the law, kept in the cell's row. Close to exact, with
  no rounds and no step at an edge.
- Snow on sunny and shady slopes with coarse cells: the pixel's temperature moves from the cell's by its own
  sunlight, times how much the cell's surface warms per watt. The core already has that number from its surface
  balance.

### 6.6 Plants from numbers (C1, C2)

- **The mix.** Per cell, up to 4 kinds with their weights, in two texels. A mark reads the 4 cells about it, weighs
  them by distance, and picks one kind by its own random rank. So kinds mix mark by mark across an edge.
- **How many and how big.** The model holds kg per m², not plants. The count and the size of the plants of a stand
  follow from its mass by the self-thinning law (Yoda and others 1963: mean plant mass goes as density to the power
  -3/2). Height follows from wood by the buckling limit (McMahon 1973). No limit made for "a forest".
- **Shape.** Baked once per kind into a small picture, from its numbers: broad or narrow crown from height against
  leaf area, open or dense from leaf area per kg, a trunk that shows from the wood share. The marks then stamp these
  pictures. This is cheap, and far richer than a disc.
- **Colour.** From light capacity per leaf area (more pigment per area is darker), and from the state: stored fuel,
  activity, frost, dry stress. Real colour by kind needs C3.
- **Honest limit.** With 3 starting kinds and small mutation the picture will show 3 kinds with small differences.
  The rest is C3, a change of the model.
- **Cost.** Three sizes of mark, not 7 grids. Far away, no marks: a field with grain.

## 7. What can be deleted

With stage 0, A and B done, and if the owner decides for fixed cells:

- `tape/src/fine/` (about 900 lines) and `web/src/fine.ts`, `canopy.ts` classes, the square tint.
- `core/src/space/below.rs`, the fill in `space/drain.rs`, `Samples::leaving`.
- `core/src/solve/downhill.rs`, most of `flow/runoff.rs` (rims, ways), the special case in `flow/basin.rs`.
- If the tree goes: `core/src/refine/`, `core/src/remap/`, `core/src/inside/` as a split rule, the flux register in
  `solve/sweep.rs`, the steps by size, the relief book, the camera as an input of the run. More than 2,000 lines.

What stays and is good: the books, the air, the cover's growth law, the seed kernel, the channel law, the map on the
graphics card with its two back ends, the water's colour by depth, the marks fixed in the world.

## 8. Found on the way (from reading, not run)

1. Two laws for soil liquid inside a cell (6.5): against `CLAUDE.md` 3.3.
2. The solid surface of a cell is computed in three places (`ground.rs`, `flow/land.rs`, `flow/stack.rs`).
3. "Sea" means three things: under 0 m, in the basin, in its body. 0 m is written in, not the basin's level.
4. Seeds mix by XOR of small numbers: the ridge noise of seed S is the base noise of seed S xor 7. Near seeds are alike.
5. On a globe the ground below the samples holds its edge value where it should wrap: a kink at one longitude.
6. A cell split before the first refinement never joins (`watch` stays empty).
7. The mass of plants means two things in the two cover fields (with and without stored fuel).
8. Water from upstream enters a split cell at "the lowest sample of the highest side", not where the network says.

## 9. The owner's decisions

1. **Fixed cells for the natural world: yes.** Three layers, each at its own cost: the slow physics on fixed cells;
   the ground, the rivers and the lakes on the fixed grid of samples; the fine picture per pixel on the graphics card.
   The cells can be made smaller everywhere once the core is faster.
2. **The tree of cells is deleted.** The cells of a world stay as built for its whole life. Later, a group of
   animals splits into single bodies that read the ground at their own place; the ground does not split.
3. **The ground is state: yes.**
4. **Plants' variety:** a balance of speed and of an interesting picture. First draw what the model holds (C1, C2);
   change the model (C3) only as far as the picture still needs.
5. **Seasons at high speed:** blend from month to month; above 36 months a second show the year's mean.

How the work runs: at most two work streams side by side; the fewest tests that state a law; every run without
animals; pictures before and after each fix; `main` moves forward after each stage that passes the gate.

## 10. Where each stage stands

**Stage 0: done.** The live page keeps the cells as built; the fine picture makes only the layers the view reads and
takes at most a tenth of the worker's time; the page gets a world frame at most four times a second; no work for
animals while none lives. Seed 1, size 22, speed max, months a second:

| | fitted | 200 m view | 50 m view |
|---|---|---|---|
| before | 26 | 0.4 | 0.1 |
| after | 43 | 41 | 39 |

What stage 0 showed, for the tracks:
- Without fine cells a 2 km cell spreads its cover evenly over its ridges: bare rock on steep ground is a need of A2
  (the law inside a cell at each pixel), and of the ground's materials.
- The fine picture still takes 0.3 to 0.7 s to make and comes 1 to 3 s after a zoom; while the world plays it shows a
  state some years old. Making fewer layers saved a tenth only: the cost is the window itself. Track A removes it.
- Trees stand in the sea at the 50 m view, as before.

**Track A, the map from tiles (A1, A2): done.** The ground is made in tiles (height, wetness index, channels) by a
second worker and kept; the world's worker posts the cells' table and makes no picture; the card applies the laws
inside a cell per pixel. A tile not made yet shows its parent. The cover's marks are masked at their foot: none in
liquid, none under snow. The window finer than the cells is deleted. Months a second at the 200 m and 50 m views:
24 and 22 before, 36 and 37 after; while panning at the 50 m view a coarser picture showed 84% of the time before,
never after.

The tiles are 16-bit floats, at most 56 kept, 38 MB on the card, no copy kept by the page; a tile takes 27 to 36 ms
in wasm. A channel has a bed of washed sand cut into the ground; snow covers by its depth.

Open on track A: a tile's time (the goal is 15 ms; what is left is `Terrain::heights`, which builds its patch from
the samples down at each call); channels a metre or two wide and 0.2 m deep show their bed, not water (their colour
needs what they carry, B3); the fitted view uses texels of 125 m; tapes keep the map of squares; seasons blended
(A3). How long a zoomed view takes to be sharp is not known: the test browser draws on the CPU.

**B1, the worn ground: done.** The samples' heights come from an erosion law run at the world's start (the stream
power law, creep, the rebound of what was worn) on grids of 500 m, 250 m and 125 m; basins are kept, each with its
pass and its level; below a sample the ground is its smooth continuation. Seed 1, a 22 island: 20 bodies of standing
liquid, 15 of 0.5 km or more. Making the samples of a 32 island: 0.45 to 0.59 s (0.60 to 0.64 s on the noise ground).

Open on the ground:
- Bodies of standing liquid are drawn dry or per cell until B2.
- Close views (200 m, 50 m) show a smooth sheet: the ground has no detail below 125 m. It needs roughness by
  material and slope in the drawing (6.2), and the channel's bed cut into it.
- One erosion strength for the whole map: the samples cannot read the bed's hardness, which is drawn per cell.
- Tops are 5 to 13% lower on a 22 island than on the noise ground.
- The slow test `the_frost_gene_falls_where_winters_are_cold` fails: the cold band still freezes (268.7 K), but the
  warm band's mean gene is 273.4 K against the cold band's 270.0 K, and the test asks for more than 5 K between
  them. Its limit is not moved. It belongs to the plants' work (C3).

**The tree of cells: deleted.** The cells of a world are those it is built with. 5,340 lines out, 383 in; runs on
cells as built are bit for bit. The fast suite holds 199 tests.

**Plants drawn from their numbers (C1, C2): done.** Each square's kinds reach the page with the numbers a look needs;
a mark picks its kind at its own foot, so kinds mix across a cell's edge; the count and size of a stand's plants come
from its mass, their height from their wood, their crown from their leaf area (`tape/src/stands.rs`). The seven
classes and their colours are deleted.

**The plants' second and third rounds: done.** Six kinds of cover start a world; a kind new to a full cell makes
room by joining two variants of another; the ground under the plants is its own colour under the cover's litter;
every map colour is a reflectance in linear light, encoded once by the sRGB curve. No trait of a kind is free under
the laws as they are (the table is in the commit of the six kinds), so no cost was added. The frost test's scene
makes its premises true and asserts them: it passes, and no slow test is known to fail.

Open on the plants:
- From afar the kinds are one olive: leaves are alike, and chemistry 0 gives every part of a plant one reflectance.
- Kinds sort little in space on a 22 island: its ground never freezes hard and most of it is wet. The dry-ground
  woody kind lives on the heights; the wet-ground kind wins elsewhere. Sorting needs a world with frost and dry land.
- A few lowland cells hold no liquid in their pores among wet ones, so their cover is dormant and they show as pale
  squares (the water's follow-ups).
- A seedling of a tall kind is ranked for light by its slot's walls per m2, so it is slow to enter a low stand.
- Stems are pale: the walls' reflectance in chemistry 0.

**B2, the water on one network: done.** A body of standing liquid is a pond of the terrain's samples with one level;
the low basin is the lowest body, with no code of its own. Each month a cell's runoff goes down the samples' ways to
the body it reaches; the bodies are levelled in the order of their passes and what a body holds over its pass goes
on; one pass sums the discharge of every sample. The page draws each body flat at its level. The monthly flood over
the cells, the rims and the ways are deleted. 30 years, plants only, 32 x 32: 9 bodies above the low ground hold
1.8 km3; drift of energy 8.0e-12; Flow costs 167 -> 196 ms a year.

Open on the water:
- The page sizes a channel from one flux per cell, not from each sample's discharge; rivers do not show from afar
  (the rivers' package).
- A body's volume by level comes from its cells' curves (8 x 8 grounds), not from its own samples: levels read back
  are off by millimetres in cells of parts, a pond whose cells all have their lowest sample outside it holds no body,
  and its shore shows steps of a sample on plains level with it.
- Liquid goes straight from the cell it leaves to the body's cell; what it picks up lands with it.
- The passes over the samples cost about 20 ms a month on a 64 x 64 island.

**The rivers on the page: done.** The discharge of every sample reaches the page; a tile names the sample of the
channel each texel is nearest and its distance to that channel's line; the shader sizes the channel by the core's
channel law from that sample's own discharge. A channel is drawn as water at every zoom, as a line from afar once its
discharge passes a share of the pixel's size; its line bends on slopes under 0.01. Shores run smooth between texels.
A tile costs 9 to 16 ms in wasm.

Open on the rivers: the network runs in straight lines of 8 directions with a herringbone of tributaries on plains,
and a ladder of steps shows along some valleys (the ground's second pass); bends are an even sine and their spacing
uses a typical runoff; a channel starts at a body with a square end; notches where tributaries join.

**The owner's style for the map.** The "Earth" view is the pretty one: vivid, a little cartoony, tricks allowed
(wide shallow shores and beaches, lighter and more varied trees, leaves that turn in the cold for tender kinds, small
plants as tufts, flowers and bushes that melt into a field from afar). The "True" view stays the raw colours the
physics gives. Cliffs, bare rock, scree and boulders come from numbers (slope, the loose ground's depth, a channel's
power); the drawing places each one.

**The water's follow-ups: done.** A body of standing liquid is built from its own samples; each cell holds its parts
of it, and the passes over the samples keep their room. A cell's liquid covers the share of its area that is left
after the pores of the ground it comes to cover have filled, so every cell of a body ends a month at the body's level.

**The ground's second pass: done.** The wear's ways no longer follow the grid: coarse rows have their true width, a
sample picks one of its ways down by a fixed draw in proportion to the fall (Fairfield and Leymarie 1991), a way
crosses a corner only from draining ground, and a filled basin's surface leans by each sample's draw. The bed has hard
and soft patches from the chemistry's two makes of ground. The wear leaves a depth of loose ground per sample
(Heimsath et al. 1997) and builds a shelf under the shore from the load that reaches the low ground; taken = basins +
shelf is tested. Channel length in straight runs of 4 samples or more: 50% -> 30%.

A cell's ground node takes the top `skin_m` (2 m) of each sample's loose ground at most: the layer the year's heat and
the roots reach. What the wear left deeper is counted with the bed. `Samples::loose` keeps the true depth for the
picture. With it the key numbers and the globe without life are in their ranges on the new ground.

**The Earth view's style: done, on main.** One style step for the Earth view only: water coloured by depth through
clear coastal water with foam at its edge, beaches by the sea on gentle ground, stronger and warmer light, a colour
per kind from its own numbers, leaves that turn in the cold (each plant at its own time) and go to straw in drought,
three designs (tree, bush, herb tuft with flowers and reeds), and a far field whose grains sit anywhere in their
cells, so no grid shows at any zoom. The True view is unchanged.

Rock, shores and the ground's grain, in the same style: beaches by the sea and by lakes on gentle loose ground;
lakes and rivers in one water colour with a lighter band at the shore; a body that holds under 0.1 m is drawn as
ground; where the loose ground is under 0.1 m the bed's own colour shows (both views), as slabs or, where steep, a
face; scree between plants on steep thin ground; stones in channels by the power of their flow (`Ground::rock`).

Open on the look:
- Rock faces are rare (2 to 3% of the land is bare and steeper than 0.9) and read as soft grey patches: their ledges
  and cracks do not show at 200 to 500 m. More cliffs need a ground whose wear strips steep slopes.
- Bare ground is shaded in blocks of a sample's width (125 m): the drawn normal is the samples', not smoothed.
- At 10 m the ground between plants reads brown: herbs cover a fifth; flowers are too small to see.
- Rivers are pale beside lakes; thin streams are faint at 200 to 500 m.
- White wedges and small squares where a channel leaves a pond; pale blotches in seed 1's river-mouth bay.
- Autumn is mostly herbs going yellow; red shows only on the heights.

**The ground on the page: done.** A channel is drawn as one smooth line down its stem (the mean of the sample centres
3 up and 3 down, Chaikin 1974), with the bends laid on it; within 250 m the drawn ground leans to the channel's floor,
so a valley follows its river. Water against ground is decided per texel by the drawn height; the shelf lies 10 m
under 0 m with a rounded edge. A scene world builds in 0.06 s.

**The rain: done.** Of what condenses over a cell, the share 1 - e^(-width / 20 km) lands there; the rest drifts with
the wind and lands over all the cells of its air node (Smith and Barstad 2004), the node's total kept. So no cell
under raining air gets none, and the pale dry squares are gone. Rain on a 22 island's land cells (least / median /
most): 0.03 / 1.16 / 2.33 m a year; runoff is 0.65 of the rain on land. The wettest cells get less than under rain
that lands where it condenses, so fewer cells are rivers: that reference of the key numbers (`RIVERS`) is measured
with this law.

**The bodies' start: done.** Every body of standing liquid with a pass starts full to its pass, its pores full: in six
runs of 60 years, 86 of 92 bodies end at their pass, so that is the level their gains and losses hold. A body that
loses more falls to its own level by the laws.

**The core's speed, first pass (track D): done.** A month of the flow takes a fifth fewer instructions, results bit
for bit: the runoff finds once per cell whether it is land, a body's floors are sorted from last month's order, a
cell's shore is counted by a search of its sorted sides, one search gives a part's volume and area under a level, and
the rain's landing builds no list. Native, plants only, ms a year (size 22 / 32 / 64): 244 / 505 / 1915 -> 167 / 366 /
1498. On the page the world waits for the page to draw its frame, so the date and the picture shown are the world's
own within a quarter of a second, and a pause stops at the date shown.

A body's level is one exact search of its floors (`level::level_with`), in place of Newton's steps, a sort and the
halvings of the pores' share: 4% fewer instructions; the climate's numbers move in their fourth digit at most.

Open on the speed:
- The parts of a cell are solved after each of the liquid's bookings (7 a month) and after each air step. One solve at
  the end of the liquid step changes the law's timing, not only its rounding: the runoff reads the solve after the
  pore flow (the liquid share of a node on a melting plateau, a cell's share under its liquid, its land's pore room),
  the erosion reads the solve after the runoff (what is solid), and the mix through the thermocline reads the parts
  after the load. A cell at rest is already taken from its last solve; 1% of the cells the bookings pass are at rest.
- The ground's full state is built each month at the survey. To build only what changed: a full thaw where the refresh
  changes the substances present, the fusion enthalpy held found from the amounts (summed by moves its rounding
  grows), and Flow's state saved with a snapshot, so a resumed run stays the unbroken one. A first try that kept only
  the bodies and the gases monthly ran out of memory within 6 years at size 22 (not traced).
- The wasm build has no `wasm-opt`; SIMD gives 3 to 5%.

Open on the ground and the water:
- Loose ground that settles during a run is not bounded by the `skin_m` rule.
- The heat capacity of a node's amounts is one helper (`ground::amounts_cap`). Two orders of summing remain:
  `World::heat_cap_of` and `land::capacities` add the amounts onto the skin one by one, the parts and the reactor sum
  the amounts first. The same node can differ in the last bit between them; joining them changes results.
- A closed basin by the sea that loses more than it gains starts full and falls about 0.1 m a year to its own level
  (3 of 92 bodies in six runs).
- A body colder than its liquid's melting point at the start starts molten, as the sea does.
- A small square where a channel leaves a pond; a tile costs 7 to 10 ms, most of it the channels' field.
- Building the samples costs 0.5 to 0.6 s on a 32 island.
