# Handover to the next orchestrator

For the next agent who leads the work. Simple English, as the owner asks. Read this first, then `CLAUDE.md`, then
`review/roadmap.md` (the owner's decisions in its section 9, the one list of priorities in 7.1, the contract in
4.9, the clean-up in 8, the open items of the natural world in 12), then `docs/laws.md`. The
spec is `docs/handover.md`; where it and the code differ, the code and `docs/laws.md` are the truth. `review/` holds
two files: this one and the roadmap.

## 1. The owner and how they work

- The owner writes short messages and reads very simple English only: short sentences, common words.
- They give a direction and expect it done with helpers: you are the orchestrator, architect and reviewer; helpers
  (subagents) write the code; you check, merge, run the gate and push. They asked for at most two helpers at once, Opus
  and Sonnet where each fits (Sonnet did one pure clean-up well; check its claims, one was wrong).
- They judge by pictures. Ask every helper for before | after pictures (same seed, date and place) and send the best
  to the owner. Look at them yourself first: twice a helper's work passed every test and looked worse.
- **Their two rules, above all others:**
  1. **Things emerge from general physics and chemistry laws.** No rule made for one thing.
  2. **Do not compute what is not needed.**
- The "Earth" view is the pretty one: vivid, a little cartoony, tricks allowed if they follow the world's numbers. The
  "True" view stays physically true.
- Tests: few, and always without animals for now (`ew run --roster --plants`). Never loosen a tolerance.
- **Run tests sparingly** (their word: "it feels like we're running and re-running slow tests every few minutes"):
  one full gate for a merge; the slow tests once, and only when a law changed (a change of the page or of the tiles
  needs none); a helper builds and checks only what it changed while it works.
- **The animals do not count for now** (their word: their code will be reworked later): an eaters' number that
  moves, or eaters that fare badly, holds nothing back; do not study it.
- They said: push to main often.

## 2. How to run helpers

- Give each helper a full brief: the goal, the owner's own words, what is known, the files, what not to touch (the
  other helper's area), what to measure before and after, the gate, the report's form (simple English, bad news
  first), and: commit, never push, no pull request. Each works in its own worktree (`isolation: worktree`). A new
  worktree starts from `origin/main`, not from your newest commit: tell every helper to merge the work branch first.
- Measure first. The briefs that worked asked for a table of numbers before any change, then one change per commit.
- A helper stops and asks when a key number leaves its range. The decision is yours; tell the owner plainly.
- A helper cannot give you permission. If it says an action was denied to it, do not do that action for it.
- Before merging: every commit signed (`git cat-file -p <commit> | grep -c '^gpgsig'` gives 1), the pictures looked
  at, the law commits' messages read. Merge with `git merge --no-ff`. On a conflict of `SNAPSHOT_VERSION`
  (`tape/src/snapshot.rs`) take a number above both sides.
- For a "bit for bit" claim run your own pair: the `ew` binary before and after, same run, equal `hash` and equal
  `series.csv` but the `ms_*` columns.
- Work on the session's branch and push it freely. Push to `main` only as a fast-forward of a commit whose gate
  passed: `git fetch -q origin main && git merge-base --is-ancestor origin/main HEAD && git push origin HEAD:main`.
  Then `git branch -f main origin/main`: the push does not move the local branch `main`, and a helper told to
  start from or merge `main` takes the local one.
- Worktrees and scratch files die with the container, and the container restarts without warning (three times in
  one day): tell every helper to commit its work in progress early, and push its branch to a `wip/` name as soon as
  it holds something. Anything worth keeping is merged or pushed before the session ends.
- When the helpers of the session's own model meet their usage limit, start them with `model: "opus"`.
- Never delete scratch folders a running helper may read (its scripts live there).

## 3. The gate

Before every push to main (CLAUDE.md 6.3). Builds take 10 to 25 minutes when helpers build too: run it in the
background.

```
cargo build --release --workspace --all-targets
cargo test --release --workspace --no-fail-fast 2>&1 | grep "^test result" | awk '{p+=$4; f+=$6} END {print "passed",p,"failed",f}'
cargo clippy --release --workspace --all-targets      # no warning
cargo build -p pcore --target wasm32-unknown-unknown
cd web && npm run build && npm run wasm
```

The fast suite has 213 tests. Then open the page (section 6) and look. For a change of the laws, also the slow tests
(`-- --ignored`): `the_key_numbers_stay_in_their_ranges`, `a_globe_without_life_keeps_an_earth_like_climate`,
`the_long_step_keeps_the_climate`.

**The key numbers' references** (`tape/tests/ranges.rs`) are the means of three runs of the world itself, a guard that
the world stays near itself; the Earth anchors in that test are separate and fixed. After the new ground, the bodies
and the rain, all nine references were measured again (ranges unchanged). Measure them again only with a change of the
laws that moves the world on purpose, in a commit of its own, and tell the owner each time.

**One slow test is red on main and stays so by the owner's decision**:
`the_laws_do_not_depend_on_the_size_of_the_cells` (section 5, nature track). It does not hold a merge. It is not to
be loosened, skipped or hunted without the owner's word.

## 4. Where things stand

**Main** (and the live page) holds, all gated: fixed cells; the worn ground with bare bed on steep ground
(`space/mantle.rs`); water on one network, bodies full at the start; rain that drifts; plants from their numbers; the
Earth view's first style; the core's first speed pass; the page's world waiting for its frame.

**Also on main since** (the work branch and main are the same commit; fast suite 213 passed, clippy clean, the slow
tests of `tape/tests/ranges` pass):
- **The new ground** (landscapes on a small world; what is open is in `review/roadmap.md` section 12): a range, hills and a plain on one island (`terrain::rise`), a coast drowned
  after a low stand of 100 m, five makes of bed in folded layers (`space/strata.rs`); `SNAPSHOT_VERSION` 122. With it:
  Flow's books for liquid that only passes through a node (`potential::relayed`), and test scenes that ask the kit
  for their ground by a condition (`testkit::island_with`, `mild_with`).
- **Plants close up**: six crown forms from the kinds' numbers (`web/src/form.ts`, `lib/crown.wgsl`), stands, autumn by
  leaf life, herbs as a sward, snow buries a plant by its depth against its height, roots need loose ground.
- **A crisp map**: shores as smooth lines (`tape/src/ground/shore.rs`), no water ending at a sample's edge, fine
  relief fixed in the world (`lib/relief.wgsl`), bare bed with ragged edges (`loose_seen`), rivers' beds at their
  flood's width with bars, lakes with shallows.
- **The frame's cost**: one place for a pixel's colours, the land program made three times with the detail that cannot
  show compiled out (`LEAST_E`), no frame when nothing changed. Far views cost about 90% of what they did before both
  drawing packages; the 200 m, 50 m and 10 m bars on a phone still 140 to 165% (measured in software only).

- **An island in the planet's air** (`flow::Planet`, `worldgen/src/planet.rs`, `flow/air/{edge,stretch,planet}.rs`): the
  page's three climates (Wide 20° to 65°, the default; Mild 30° to 55°; None), `ew run --band S,N`; a world made
  without a band is bit for bit what it was. `SNAPSHOT_VERSION` 124.
- **Snow, ice and rock on the page**: snow placed where the ground keeps it, its line in patches; the frozen low ground
  off its 8 km grid, floes at its margin, a textured pack (`lib/ice.wgsl`); bare bed in its sample's make colour.
- **The plants' marks decided once a frame per cell** (`marks.wgsl`, `lib/mark.wgsl`), kept in rgba8 textures.

**Known faults on the page:**
- **A band of bright flat green beside rock or brown land when zoomed in** (the owner's fault as they meant it;
  their picture: seed 1, 32 squares, Wide, year 15.92, the 100 m bar): 150 to 300 m wide, no crowns, soft toward
  the stand, ending against the bare bed in a hard ragged straight line; it appears only close up; "not sure it
  relates to anything real in the numbers". In work (branch `green-band`): which term draws it, what the cells
  under it hold, whether the cover can stand on bare bed at all; a report before a mend.
- The green grain over thin cover when zoomed in (found on the way, not what the owner meant) is mended on main: the near view drew
  each crown at least 1.5 pixels wide whatever the plants' size (`grain` in `lib/cover_field.wgsl`); a crown is
  now as wide as the plants' own marks and drawn only where those span 1.5 pixels, else the near view shows the
  mean tone as the far view does. Left: a cell with no cover still takes green from the cells about it through
  the smooth blend, at every zoom (bare land reads olive); faint arcs along the ground's contours in the owner's
  picture were not found again (a guess: the water's-edge drawing in `land.wgsl`).
- The frame's cost against before the drawing packages (software only; a real phone is not measured): far views 92 to
  97%, 50 m and 10 m 60 to 71%, the 200 m bar on a phone 176% (the near plant field at 0.69 m a pixel, snow and lakes).
- Snow and ice cost 25 to 45% of a far frame in the headless browser, also on an island with no ice (the software
  drawing seems to pay for the program's size, not for the work a pixel runs). The owner's word: the speed on phones
  is perfectly fine. So no more cuts for now; the headless numbers only guard against a change that makes them worse.
- The True view on water: a 2 km haze, no floes. The pack's ridges read as wiggly lines.
- The coast's squares in a Wide island's far north hold no plants (their cells are low ground): bare land there.
- A square notch where a channel joins two lakes (mended, not seen on a picture); steps on lake shores at the 2 km
  bar on a phone (texels of 31 m: `VIEW_TILES` bites); shallow lakes are a pale grey-blue (their true depth).

**The cover's law is on main** (`core/src/cover/`, `docs/laws.md`; `SNAPSHOT_VERSION` 127): seeds lodge where a slot
runs a deficit of light, a slot holds a count of plants that thins as they grow up (`cover/stand.rs`), roots need loose
ground, the leaves pass the key liquid to the air through Flow's daily surface step (the cover only sets its film),
conducting walls and fine roots pay, a full slot sheds the leaf layers that do not pay, hardiness is the kind's own
gene, and the cover keeps a yearly budget the tape records (`cover_budget_<flow>`). A Wide island sorts into zones and
is steady from about year 100 (run to year 300): bare far north, a low woody and herb belt, a tree line, old stands of
13 to 32 kg/m² in the warm south; the frost gene follows the coldest month (r 0.7). The lifts after Flow go between
the heads the potential energy found (`Scratch::lift_at`). The page draws a row a slot from the law's own numbers
(`tape/src/stands.rs`; no number of the law is copied into `web/`), crowns that share the canopy, a shaded floor.

**Known faults of the cover and its page:** a warm wet stand builds 2 to 4 kg of dry matter a year (the sun at the
ground is 1.6 times Earth's: item 4 below); a plant short of a mineral leaks half of what it stores to the ground
where a real one slows its light reaction (to mend with item 4); small dark-red specks in the forest at the 200 m
bar; blocky dark patches at the 2 km bar; a lake of the far north drawn with straight edges on the fitted view; old
stands whose crowns are wider than a mark reaches are filled by the field's canopy.

**Rivers and stones are on main** (`tape/src/ground/{belt,bends,wander,frame,lines,mend,channels,shore,stems}.rs`,
`web/shaders/lib/{river,stone,flow}.wgsl`): a meander belt built with the tiles, a line that wanders and grows with
its discharge, water by its depth, bars, one maker of a stone (corners and sides, four sizes, buried by snow), signs
of flow that slide six steps a second and ask for a frame only while a river is in view, a far line that fades into
the true bed, mouths that reach their lake or sea with a fan. The tiles' second texture holds four numbers a texel
(56 layers: 61 MB).

**One width, one water (on main):** a river's width on screen is the larger of its true bed and a least line of 1 to
2.5 css px by discharge, at every zoom and whatever tile is loaded (`web/src/land.ts` `drawn`); every channel wanders;
lakes, ponds and the sea take the rivers' water (`river_seen` in `lib/style.wgsl`); nothing of a channel is drawn
under standing water. The owner: "amazing, much better".

**Mouths (on main):** a channel is drawn on land and through the band of shallows (`river_spots` in
`lib/river.wgsl`, `through_band` / `standing` in `lib/land.wgsl`): it widens (`MOUTH_FLARE`), keeps its water and
flow signs, and melts into the open water at the band's end; a rill ends at the water's edge; the sea's foam is the
first metres only and stops at a mouth. A bend turns on at least two bed widths and a tile draws a sway only from
enough texels (`belt.rs`, `lines.rs`): no saw from afar. The shore's choice is blended between sample centres
(`shore.rs` `share`): no straight outlines. One number says where liquid is water for the picture and the plants
(`FILM_M`). The pixel runs two layers and colours the river's water once: against the page before the rivers, by CPU
time in software, the fitted view costs -10%, the 2 km bar -7%, the 200 m bar +3%.

**The shores' round (on main):** beside a pond the smooth shore lies lower on a channel's line, so the shore and its
band make a short funnel up a river (`shore.rs` `drowned`, `MOUTH_DROWN`); the sea's foam close up is one line at the
water's edge and two soft broken lines by the bed's depth (`style.wgsl` `foam`), compiled out of the far program; a
channel is drawn only where its bed or far line takes half a pixel or more (`FRINGE_PX`); the herbs' flowers lie on
two turned lattices (`cover_field.wgsl` `flowers_at`).

**Where a lake is (on main):** a texel is in a pond only where the smooth shore's offset is under `EDGE_M`
(`shore.rs`; `Shore::beside` gives the pond's id), a function of the place alone, so every tile at every depth agrees;
up to that edge the ground is kept above the pond's level, so the edge of what is named is dry ground
(`every_tile_agrees_on_where_a_pond_is`). It mends blocks of water beside a lake's outlet (the valley's blended
ground lay under the level in the samples next to it) and the cut of the water texel by texel on low ground (the
owner's shore in steps at the 5 m bar: the cause is gone, the same steps were not reproduced). Low ground beside a
pond's sill is raised to just over its level: a pale rounded band round a summit pond from afar.

**Not merged, on `wip/lakes` under the lakes' commit:** four commits on small plants (067873ae reeds by their cell's
herbs; 73cdb42d no crown under 1.5 css px, a thin stand's clumps as its tone; 842e39f6 a square without a layer's
plants borrows its neighbours' numbers; 0d13a9a7 a herb's tufts drawn as small plants). Their helper was stopped
before its report; in the pictures the trees read thinner at the 100 m bar and the ground gains pale balls and ochre
patches: review them with pairs before taking any.

**Known faults of the rivers and shores:** straight shores: a far-north lake of seed 1 follows a straight steep slope
of the made ground 2 to 4 km long (the cause is in how the terrain is made, not in `shore.rs`), and one lake's west
shore runs straight for 200 m (cause not found; look at the samples' heights along it first); the band's turn up a
mouth is slight, hardly seen at the sea; the foam's broken lines are short soft dashes, and sheltered coves get foam
(no number for shelter); thin tributaries seen from afar run nearly straight; bends come in between the 500 m and
200 m bars; the pan frame draws a river a little wide. Not mended: dark-red specks in closed forest at the 200 m
bar, blocky dark patches at the 2 km bar (two of the commits on `wip/lakes` are about them).

**The frame's cost in software says how much code a program holds, not what a pixel runs** (a branch never taken
costs the same as one that runs; code removed at build time is what saves). The largest share is the sea ice
(`lib/ice.wgsl`): without it a frame costs 28 to 45% less. The next cut: a program without ice when no cell holds
ice on water. A real phone is not measured.

**The air's package is on main** (section 5, nature track): sunlight is absorbed by the gases and reflected by
the clouds, the heat goes to the poles by the eddies and the overturning, the ice is bounded.

**The sorted start is parked on `wip/sorted-start`** (33c1e353, not on main; main is merged in, the fast gate
passes, `SNAPSHOT_VERSION` 128 there). It holds: every world's cover placed by the nursery
(`worldgen/src/life/nursery/`), the random placing deleted; a garden grows the race's champions and the hand-made
kinds sown together (`Class::grow`); a cell takes every class's stand weighted by 1/d² of its distance in climate
(Shepard 1968), so no 2 km squares; a status line on the page, 1.5 to 1.7 s of sorting; the two slow tests compare
runs from one placed world (`tape/tests/common`: `placed`, `go_from`); on the page a hardy kind's hue is spread
(`HUE_PER_FROST`) and flowers cover a share of a crown (`PETAL_SHARE`). **What holds it off main:**
- **The look** (the owner judges by pictures; pairs in `tools/sorted/shots/` on `tools/checks`): an open woodland of
  pale khaki and olive on a straw or grey-blue floor with flower dots, a mottled fitted view with a yellow north,
  where main shows a vivid green forest on a green sward.
- **The world moves**: the cover is 11.26 log10 kg (reference 9.505), it draws the heat-absorbing gas down to 223 ppm
  (326) and cools the land to 287.5 K (289.19); the far north is forest (15 to 24 kg/m² at year 100, bare on main)
  and the page's northern ice at year 30 is gone; southern stands hold 37 to 70 kg/m² at year 30 (main 7 to 32).
  `the_key_numbers_stay_in_their_ranges` fails on temperature, gas and cover; `the_long_step_keeps_the_climate`
  fails on `air_3` (-1.21e-5 against 9.18e-6: the gas falls fast and the long and short air steps differ more).
- **The kinds are extreme**: the races draw from the genes' whole range and their winners are kinds like a shrub
  hardy to 245 K, made for dry ground, with 89% of what it builds to seeds and walls 20 times its matter, on a fifth
  of the island. The gardens grow stands of about 200 years. The garden still reads the bare year's dry air.
- The orchestrator's reading, not measured: main's random start is steady from year 100 at 13 to 32 kg/m² in the
  south, the nursery places 37 to 70, so a garden is not the world (no neighbours' air, a class's mean ground). The
  first step is a run of main to year 300 beside a sorted one: if they do not meet, the garden is at fault, not the
  page's colours. Do not turn the page's palette to hide it.
- Probes and tables: `tools/sorted/` on `tools/checks` (`probe.rs` gives rows by latitude; `rows_seed1.txt`,
  `rows_seed2.txt`, `slow_ranges.txt`).
- **Read against today's main** (a reader's report, nothing run: `scratchpad/sorted/assessment.md`): the branch
  cannot be merged (it stands on the old animal world, 169 commits back); the nursery itself is four files of
  about 700 lines and is to be carried over by hand. Three things it reads are gone from main (`World::rained`,
  `World::canopy`, `rng::Purpose::Mutate`); its garden uses the cell's area and the whole cell's wetness where
  main's cover lives on the cell's land; its seeds do not lie on the ground. Faults of its design, read in the
  code: its mutants do nothing (five generations move a gene 0.14 steps, seeds join their kind's slot within 2
  steps, so all new variety comes from draws over each gene's whole range: its `numbers.md` says otherwise); its
  race scores a kind by its mass alone after 12 years (walls are cheap mass); its three champions may be one
  kind; it places stands about 200 garden-years old in year 2 (37 to 70 kg/m² where main places 0.05 to 0.3);
  it adds about nine tuned rows; it runs two whole world-years on a copy (about 15 s at 128 squares in a
  browser).
- **The orchestrator's decisions for the port**: no race by mass: the mixed stand's own competition says who
  stays; newcomers are draws near known kinds, far enough to found a slot by the slot rule, and no mutant rounds;
  the palette is not touched before the kinds are judged; the age of the placed stand and whether a garden's air
  gets its own vapour back are decided after the garden check; the roots' package lands before the nursery is
  judged (a garden selects under the cover's laws). Not to be carried over: the branch as a merge, anything of
  bodies, a renumbering of `rng::Purpose`, `World::rained` as saved state, a second wetness or area rule.
- **The owner's word: the amount of variety is a setting of a world**, like its seed, size and climate (a start's
  choice, not a number of a law): how many kinds a garden keeps and a cell may hold, 4 today (`cover::SLOTS`, a
  constant the arrays, the tape and the page are sized by), "what if we want 10". For the port: `SLOTS` becomes
  the most a cell may hold and a world's setting says how many it uses (a pure clean-up at 4: the same world to
  the last digit), the garden keeps that many and tries newcomers in proportion, the CLI and the page offer it.
  It gives room for kinds; how many live together in a cell stays with the laws. The cover's month costs about
  in proportion to the slots used.
- **In work**, by a builder in its own worktree (scratch `scratchpad/sorted/spike/`, `notes.md` there): part 1,
  the variety as a setting of a world (`cover::SLOTS` the bound, the world's setting the count; the same world at
  4; `ew run --variety N` and a field on the page), reported and merged on its own; part 2, the spike of the
  nursery on the same branch behind `--sorted` (no race by mass, newcomers near known kinds, the garden check,
  bands' tables, a contact sheet): numbers and pictures, not for main as it is.
- **Its steps**: (0) main alone to year 300 on the new air, the target: done (`scratchpad/sorted/target/`, seed
  1 at 64 squares). The cover is steady from year 100 in all three: in the band with the roster 1.33 to 1.40e11
  kg, nearly all the land covered, the gas held at 386 ppm, land 283.7 K, the low liquid down 1.2 m at year 300;
  with no band and the roster 4.9 to 5.1e10 kg, the shores bare, the gas drawn down to 212 ppm by year 50; with
  no band and one kind 1.1e10 kg, the gas 289 ppm and still falling. So a closed island holds about a third of
  the cover the same island holds in the planet's air: the gas, not the light; (1) a spike
  of the port on a branch; (2) the garden check: a garden grown on a cell of main's year-300 world with that
  cell's kinds against the world's own mass there; (3) side by side to year 300 with pictures; (4) the same after
  the roots' package; (5) the design's mends, a commit each; (6) to main with the references set once; (7) the
  look. About 3 to 5 builder-days.
- Found on main by that read: `Cover::sent()` has no caller; comments in `cover/around.rs` and `cover/mod.rs`
  speak of a nursery main does not have. To delete, or to use with the port.

## 5. What to do next, in order

**The owner's order of work** (their word): (1) the variety of habitat; (2) the sorted start, for plants and
for animals, so that life is fitted to that variety from the start; (3) the animals' numbers run again after
those. The world's size has a ceiling of 512 x 512 squares; they ask for honest or nearly honest and easy ways to
get the variety on a land of manageable size (asked of the habitats' explorer: the planet's dry belt inside the
band, a steady wind and its rain shadow, more relief, ground that differs by its grain, years that differ and
fire, ground that stays wet, stretching the air's distance inland as the band stretches the latitudes, a larger
world). No rule that says where a habitat goes.

**Where the two tracks stand** (the orchestrator keeps these lines true):
- On main: the docs' clean-up; the plants made free (`core/src/contract.rs`, the natural world's month on its own, a
  test that guards the sides, `cli/tests/sides.rs`, the gate and the key numbers on worlds of plants: the cover's
  reference is 9.881, the world of plants' own mean); worlds that differ (`core/src/space/rises.rs`, `ew run --shape
  drawn`, the page's choice of shape; one island stays the default until the owner has seen the drawn worlds). The
  live page is its Version 44.
- On main, life track: the old animal world is deleted (bodies, brains, parts, nodes, lanes, their tests, tape
  columns, start and drawing; the random chemistry with them). The last commit that holds it is `16a774d` (a tag
  cannot be pushed from this machine: the name `animals-v2` exists only locally). A world of plants gives the same
  series to the last digit and the page the same picture. `core/src` is 11,300 lines of code (16,900 before),
  `series.csv` 141 columns (323), `docs/laws.md` 16,600 words (28,700). The contract stays (`core/src/contract.rs`):
  Flow and the cover read `Stand` and `Place`; `take`, `give`, `press`, `shown`, `slots` are used by the plain taker
  of the test kit alone until the animal field comes. Every world is a world of plants: `--plants` and `--null` are
  gone, `--roster` is six kinds of cover against one.
- On main, nature track: **the air's package** (merge a75ccd0, `SNAPSHOT_VERSION` 140). Gases absorb sunlight
  and clouds reflect it; the eddies carry the near-ground energy (`mix`, Hwang and Frierson 2010); the
  overturning's cell ends where the spin sets it (Held and Hou 1980), and only its part that runs down the
  gradient carries the heat difference of its two halves; the rain of a step wets the ground's bucket day by day
  as it falls; a loaded world's first build keeps its saved parts; the round lobes of the shelf; life starts on
  every cell that holds land; the cover of a cell lives on the cell's land (`contract::land`).
  - What it gives: the lifeless globe keeps a bounded ice (surface 288.5 K, ice at most 0.12); a banded island of
    128 squares is 290 K in summer and 273 K in winter and green, where it was frozen all year (262 K); its rain
    is smooth (`scratchpad/air2/shots4/contact.png`); on a 32 drawn world in the band the low liquid falls 1.3 m
    in 150 years and 11% of the map is under ice, where it fell 23 m with 40% under ice
    (`scratchpad/air2/sea/table.txt`). Flow costs about 5 to 12% more than before it.
  - The key numbers' references are the package's: land 287.86 K, swing 11.82 K, rain 2.506e-3, gas 354.1 ppm,
    river cells 236.1, worn 0.1998 mm/yr, cover 9.492. The long step's test holds the land's runoff
    (`runoff_sea`), not the river cells' flux.
  - **One slow test is red on main, by the owner's decision** ("land the air package, forget the red test, move
    on"): `the_laws_do_not_depend_on_the_size_of_the_cells`, on `root_1_cover_area` (6.60e-2 against 5.25e-2,
    +3.6% of its mean) and `air_3` (4.93e-6 against 4.39e-6, +1.4%). Before the package the cover's area moved by
    +5.9% and passed on a wider year-to-year spread. Nobody hunts it now. Do not loosen it. What a hunt found
    before it was stopped (`scratchpad/air2/notes.md`): the difference is not in how it is measured; on land
    under half a cell the 1 km squares stand thinner and read a larger vapour gap.
  - **The low liquid of an island in the planet's air is the planet's** (on main, merge ed14b23,
    `SNAPSHOT_VERSION` 141; `flow/runoff/edge.rs`): the open low ground's body that touches the map's edge is held
    at the level the world is made with (`space::wear::STAND_M`, 0 m): what stands over it leaves, what is
    missing comes in at the edge cells with their own heat and dissolved matter, booked as a flux from the
    boundary and counted (`edge_liquid`, `edge_liquid_heat`). In the band 20 to 65 about 9 mm of level a year
    comes in; in 30 to 55 about 11 mm a year leaves (its air brings more vapour than it loses). A world with no
    band is the same to the last digit. Open: the tiles' channels end at the shore the world was made with; for
    levels that truly move (ice, a god's hand) the ground must be worn again where the level moved, at a slow
    cadence, with the package of the mouths. `flow/runoff/mod.rs` is 376 lines.
  - **The page makes islands in the planet's air only** (the owner's word: "remove None from the page"): the
    closed island stays for the tests and the CLI. Measured: a closed island's air is drawn down to 212 ppm of the
    gas and holds a third of the cover.
  - Open on the air: No cloud made by cooling from below; the `wind` row is "anchored, open"; the
    exact solve of the surface's balance by pieces is parked on the local branch `surface-exact` (31eb1d1: right
    for the temperature, it raises the long step's water cycle by 0.3 to 0.4%); one air step a month is refused
    (the warmest month moves by +0.90 K).
- On main: a world is drawn from its seed unless one island is asked for (`--shape one`; the page's choice). Rings
  are 3 rises in 100 and mostly breached; where two rises raise the land alike the crust sags, so lands stand apart;
  drawn worlds hold about a third of land. Every test builds `worldgen::testkit::one_rise`. The live page is its
  Version 46. Open on the drawn worlds (the next round of that work, not started): on maps of 128 the lands read as
  smooth long blocks each in its own moat, and some straits run straight (the sag follows a line; the noise's warp
  is not on it); the noise is sized to the map, so a 128 map is a 64 map scaled up: the proposal is the noise's unit
  by km (the 64 map's), which leaves worlds of 64 as they are and moves every other size, tests' worlds of 16, 24
  and 32 among them (picture: `scratchpad/islands/round2/km/`); flat shelves with straight edges show on some 128
  worlds; land reaches the map's edge in a few. Found on the way, not of that work: the rain over open liquid shows
  a checkerboard in the planet's air.
- On main, life track: seeds lie on the ground (lots of the cover that wake by the ground's state, wear at the
  dormant floor and freeze below their hardened limit; `core/src/cover/kin.rs` is the one joining rule of slots and
  lots), the soft solid on a cell hides what lies under it from a taker (`contract::Place` holds its depth), and two
  map layers show the seeds and the last year's growth. The live page is its Version 47. Open on the seeds: the
  ground holds 5 to 150 g of seeds per m² (measured banks: 0.1 to some tens) because a full slot sends all it builds
  as seeds; by `dormant` and `e_life` a lying seed would last about 700 years (the row says years to decades); seeds
  lie on low-ground cells near the shore and the two layers bleed over the liquid there; nothing hides a buried
  plant from the light (the air's step lets the canopy catch light over any snow). To measure again on the new air.
- Designed, not built: the first takers of the cover (F1), `review/f1-design.md`, with the orchestrator's
  decisions at its top (thirst is a law of F1: a body drinks open liquid only and pays the way to it). On main
  (merge 2081501), none placed unless asked for (`ew run --fauna-year N`; the page places none): the shared
  ground as a pure clean-up; the fauna's state, placing and books (`core/src/fauna/`, eight small files); the
  month's running liquid (`World::liquid_month`); the take (Holling's disk law in six steps at midpoint rates)
  and digestion; thirst that cuts work before it kills; the way to another cell's liquid by the law of walking
  (`core/src/contract/liquid.rs`); the map layer "Takers". Books with takers that eat: drift 1.7e-13 over 60
  years at 64 squares; about 2.7 millionths of a second per land cell and month. Found:
  - Placed at year 50 on a 64 drawn world with no births, most starve in the first year. The budget against
    measured figures (`scratchpad/f1/budget.md`): no anchored row is on a wrong reading (rest, the coat's
    conductance, the chewing power's slope agree with the measured; our warm takers burn 46 to 78% of a real
    field metabolic rate). The start's guts were 4 to 7 times too large (a start condition, mended). On good low
    cover every warm kind but the 30 g one pays 106 to 216% of its need. Two laws lack: a small body stands in
    the open all day (the 30 g kind's coat passes 1.7 times a real field metabolic rate in the cold month); a
    bite of a tree is 75 to 83% wood.
  - No gathering by the liquid: on a wet island nearly every cell sheds liquid every month. Honest; the gate's
    row needs a land with a dry belt.
  - The drink fetched by walking is booked from the walker's own cell: to go right with the outbox of the young's
    dispersal.
  - On main too (merge 02c5534), two laws the budget asked for. A resting body sits in a shell of what lies on
    the ground (the node's remains at `litter_rho`, conduction through a spherical shell at `litter_k`, in series
    with the coat): a 30 g body's loss falls from 1.17 to 0.50 W; the 400 kg kind's need does not move, the 60 kg
    kind's falls 15% in its coldest month; the soft solid is not in the shell (Flow holds no number for how it
    insulates). A mouth breaks what its jaw can rupture: a slot shows its soft part (matter and fuel) and its
    frame (walls) apart (`Layer::Soft`, `Layer::Frame`); the jaw's force from one fit of small mammals (log10 N =
    0.43 log10 g + 0.416, an extrapolation for the large); `shear` of the frame 60 MPa (from memory); a jaw cuts
    axes of 0.8 mm (30 g) to 6 mm (400 kg), 1 to 10% of a tree's frame and all of a herb's. With both, each kind
    alone pays its way on good low cover and on tree cover (98 to 557% of its need; the cold-bodied 15 to 20
    times).
  - Open: `contract::reached` spreads a slot's matter evenly over its height, so a mouth at the ground reaches a
    share of a tall slot's soft part though its leaves stand in its crown; and a slot's living matter holds its
    fine roots, which a mouth may be shown. Both asked of the next builder, with numbers, not mended.
  - On main too (merge fcd736d): growth and births. A slot is its adults and its young; the surplus builds the
    young at the cap of the rate of living (West, Brown and Enquist 2001, read: B0 m^(3/4) / E_m, no new number);
    maturing; whole counts by draws that keep the mean. A 64 drawn world, takers at year 50, 250 years
    (`scratchpad/f1/shots2/`, `b2/m64.txt`): books kept (energy drift 4.7e-13); three kinds alive at year 250
    (30 g, 3 kg, the cold-bodied 2 kg), slope -0.81 beside Damuth's -0.75; the 60 kg and 400 kg kinds go out
    cell by cell by year 225 (the young do not leave their cell yet); **the cover falls to 31 to 44% of its
    twin's**: the cold-bodied kind takes 10 to 20 times its need (its work runs at min(1, pace) over `t_cat`, its
    upkeep at a sixth of a warm body's) and reaches 13.6 t per km²; a mother at her cap bears about her own mass
    of young a month. About 12 millionths of a second per land cell and month.
  - Roots are not shown to a mouth (the cover does not store them). The even spread over height lets a 30 g
    mouth reach 0.05 kg/m² of a 23 m slot's soft part at the trunk's foot (none if it stood in the top third):
    kept for now, open.
  - On main too (merge c3ddf4a, `SNAPSHOT_VERSION` 142): a body's work runs at the pace of its own temperature
    over the pace at 310 K (`WORK_T`; the cold kind's surplus falls from 12 to 23 times its need to 2 to 4);
    a newborn grows from nothing by the growth law (26 days to bear at 30 g, 258 at 400 kg; a mother's cap bears
    about a third of what it builds); mutation and the slot rule (`fauna::RANGE`, `life::kin::mutate` shared with
    the cover; slots join only when alike in every gene); the young leave by the seeds' kernel through an outbox
    and the liquid fetched from a far cell is that cell's, carried as life carries matter; the takers live on
    the cell's land. The 64 drawn world on the new air, takers at year 50, to year 250 (`scratchpad/f1/shots3/`,
    `b2/me64.txt`): **all five kinds alive**, books at 4.6e-13; **the cover is eaten to 13% of its twin's** (1.12
    against 8.50 kg per m² of land; 11% in the band), 42 to 80% of its growth eaten, by the 3 kg kind (11.8 t per
    km²) and the cold-bodied 2 kg kind (4.8 t); births are still 3 to 10 times real ones (Hamilton and others
    2011); no mutant founds a slot (as the design said); the slope against Damuth's is -1.0 to -1.4.
  - On main too (merge 0979bd6, `SNAPSHOT_VERSION` 143): a member takes its food bite by bite (the ground clipped
    a day is bounded by the bite's area over the bite's time: 0.29 m² at 30 g, 7.8 at 3 kg, 67 at 60 kg, 264 at
    400 kg, where the sweep gave 380 to 42,800; `bite_time` from Shipley, `bite_breadth` and `bite_exp` from
    Illius and Gordon 1987, from memory and fitted on ruminants); a slot's soft part and the thin stems of its
    frame stand in its crown, from h - L/`crown_density` to h (1 m² per m³, from memory); a mother pays the whole
    burning of her unborn (births at full food 1.7 to 5 times Hamilton and others 2011).
  - **Where F1 stands** (64 drawn world, takers at year 50, to year 250; `scratchpad/f1/shots4/`, `b2/me64.txt`;
    the twin on the same head): the cover holds at 97% of its twin's (92% in the band), 5% of its growth eaten,
    takers about 800 kg per km² of land, books at 2.2e-13, the tall slots live and young ones come up; four kinds
    live 200 years. Not met: the 30 g kind dies out within ten years on both worlds (on a ground layer of 0.02 kg
    per m² its bite of 6 mm² clips less than it burns: the bite's rule is stretched far under its data, and the
    land has almost no open low cover: 0.06 kg per m² under 0.5 m); Damuth's slope is -0.6 to -1.0 on the
    island and -0.2 to -0.6 in the band; a grazer of 60 or 400 kg pays its way down to about 70 kg of dry matter
    per hectare where measured thresholds are near 1,000 (the soft part breaks at 3.6e3 J a kg, and a bite takes
    all the soft part within its band, not the sward's top); no mutant founds a slot. No eaters of takers yet.
  - **The page places the takers** 50 years after the first life (`wasm/src/lib.rs` `TAKERS_YEAR`); the CLI
    places none unless asked (`--fauna-year`). The layer "Takers" shows them; no single animal is drawn yet.
  - In work, the owner's three questions on the takers, by a third builder (scratch `scratchpad/f1/b3/`, `notes.md`
    there), measure first: why the 30 g kind dies (its budget on three kinds of cell; real bite sizes and rates of
    small bodies; a stretched row is mended, a lacking law is described); why large grazers pay their way on 70 kg
    of dry matter per hectare (a leaf's cells have walls; a bite takes the top of what stands: each built only
    from what the cover holds or one anchored row); what births our takers achieve in a world against wild ones
    (the figures so far are at full food).
  - In work: the first drawing of animals on the page (the design's decision 10), by a page builder in its own
    worktree: two or three looks on branches from one base, pictures in `scratchpad/animals/` (`looks.png`); the
    owner picks, then one is merged. Then F2 (movement by what is sensed) and F3 (eaters of takers). The second builder's working room is used up: a fresh builder
    reads `scratchpad/f1/b2/notes.md` first.
- On main, the drawn worlds' third round: a unit of an island's noise is 21.3 km on every map (`ANCHOR_M`), so
  lands of 128 squares have ragged coasts, bays and outliers; worlds of 64 squares are the same to the last digit;
  a world of 22 squares is one plain dome, so the page's first world is 32 squares, and its size field goes to 128
  (made in 38 s, 1.6 months a second in a headless browser). Pictures: `scratchpad/islands/round3/`. Open:
  - A cell that holds parts of two ponds, one of them a speck, gave 0/0 in the runoff's pass (`flow/runoff.rs`
    `stand`) and hung Change: mended on main (a cell keeps at least what its other parts hold), and a reaction heat
    that is not a number stops the run. Seeds 1 to 12 at 64 squares in the planet's air run 15 years clean. The
    slow tests were not run after this mend (the 64 one-rise world is the same to the last digit): the air
    package's gate runs them.
  - Land in the map's edge row in 14 of 60 worlds of 128 (2 of 60 at 64): the low ground does not ring the map
    there. To mend: the ground falls to the low ground within a margin in km of the edge.
  - The shelf's platforms with straight edges: mended on the local branch `round3-shelf-lobes` (4b488e0, straight
    distance in place of steps along the grid). It moves the worlds of 64 squares, so it lands with the air
    package's setting of the key numbers' references.
  - Each land sits in an even shallow ring (the low ground follows the rise's smooth fall); some small rises have
    flat tops. Taking the rises' fall at the warped point bends the straits but pushes land onto the edge: dropped
    (`scratchpad/islands/round3/B1_warp.diff`).
- **The bare coast** (the owner's question), measured on worlds of 32 squares, seeds 1 to 4
  (`scratchpad/coast/`, `notes.md` and `pics/`): most of the bare shore is bare in the core, not in the drawing.
  - Roots cost too much on warm low ground: **mended on main** (the roots' package): `root_uptake` is 1.1e-3 mol
    per kg a s, the flow of the growing season's peak month (3 mm a day through 1.76 kg of living fine roots per
    m²; Jackson, Mooney and Schulze 1997, Table 1, read), where the row was a year's mean. In the band, 0 to 1 km
    from the shore: the roots' share of the upkeep 0.49 to 0.27, living cover 1.25 to 1.87 kg per m² of land; the
    island's cover at year 100 2.96e10 to 4.58e10 kg, its climate unmoved. The shores are covered; in summer the
    page draws them straw and orange (dormant, dry cover), not green. The references are the world's own means
    again: land 287.71 K, swing 11.72 K, rain 2.509e-3, gas 315.6 ppm, river cells 234.5, worn 0.1847 mm/yr,
    cover 9.867. A closed island with the roster falls to 157 ppm of the gas (221 before), under the 180 of
    Earth's range: no test holds that world, and the page makes none. The 3 mm a day is from excerpts, not whole
    papers.
  - A cell whose mean ground is under the level gets no soil and no starting cover (`worldgen::life::open_land`
    asks `is_land`), though it holds land: 3.4 to 4.9% of the land's samples, 26% of the land in the first km.
    Accepted: soil and cover on the land part of such a cell.
  - The drawing thinned the first km: mended on main (ec8ff03): the cover's layers are per m² of a cell's land
    (`tape::layers::per_land`, `flow::land_shares`) and the page blends them over the cells that hold land (a land
    mask read once per pixel). Under 1% of the map's pixels change. Its cost could not be timed on the shared
    machine (runs differ by 15%): 5 more texture reads per pixel; to time on a quiet machine, and a cheaper form
    is known (a mark for squares whose 4 by 4 neighbours are all land).
  - Climate None: the island's closed air falls from 402 to 175 ppm of the gas by year 25 as the cover locks it
    in its walls; the water lost per mol of gas grows with it. Open: whether an island's air is ever closed.
  - Climate Wide (main's air): 45 to 50% of the land is bare at every distance from the shore, dry (wetness 0.4
    to 0.5) or cold; emptied in the first year after life starts, refilled slowly by few seeds. To measure again
    on the new air.
- Timed in turn on a quiet machine, a 32 island of plants, 30 years: 261 ms a year before the contract, 262 to 266
  after.

The owner's order (`review/roadmap.md` section 9): free the plants from the animals, delete the animals, build
them again as a field; beside it nature goes on. No globe as a world; a world is 128 x 128 squares, 256 or 512 at
the very most, and worlds should differ in shape (several islands). No speed target now. People after the animal
layer. The full list with its reasons is `review/roadmap.md` 7.1; two tracks side by side, one helper each.

**Life track**
1. **Free the plants** (`roadmap.md` 4.9 and 8, steps C1a and C1b): a thin contract between the land and the
   bodies (what a cell shows; take, give, press; what stands on a cell), the natural world's month stated on its own
   in today's order, the main guard and the key numbers without animals, a test taker. A plants-only world stays the
   same to the last digit. To `main` before the air's package, so that the references move once on a gate that no
   animal can fail.
2. Done: the old animals are deleted. Left of the clean-up: the tree's tools in `core/src/space/` (the most
   depth of 24, `Terrain::below`, a mesh of any number of levels), the cell serials in `life/` if nothing needs them,
   one helper per idea (roadmap 8, C3), and the spec's sections on the old animals (C4).
3. **What the land offers, first part**: seeds that lie on the ground; snow that hides food; the contract's table in
   the tape as the count of resources.
4. **Grazers** (F1), then movement (F2), eaters and the web (F3), animals on the page (F4); gates in
   `roadmap.md` 7.2. Then the small game (god tools, fire).

**Nature track**
0. **The variety of habitat** (the owner's question: why no grassland, swamp, delta, desert, deep gorges, large
   rivers; "I would love to have those environments emerging naturally through the laws we have"): a study with
   measures, in work by an explorer that changes nothing (scratch `scratchpad/habitats/`: `report.md`, `why.png`):
   per habitat what makes it on a real planet, what our laws hold, why it does not show, what would let it; a
   ranked list of five changes. The orchestrator decides from it and puts the steps here.
1. Nothing in work on the page. Next there: the first drawing of animals (life track).
2. **The sorted start**, carried over by hand in the steps of section 4 (its target, main alone to year 300, is
   measured), with the amount of variety as a setting of a world. It goes to main only when its pictures are at
   least as good as main's.
3. **Worlds that differ**: several islands and other shapes drawn from the seed (the land falls from the middle of
   the map by one rule today, `space::terrain` `fall`); the globe as a world leaves (`--globe`, `--fine`, its view
   on the page). The round mesh stays: the planet's strip is built on it, and the air's law is judged on the
   lifeless globe until it is on main.
4. **Fill-in, on the page**: the four commits on small plants (`wip/lakes`), each reviewed with pairs; the dark-red
   specks at the 200 m bar and the blocky dark patches at the 2 km bar; the straight shores of the made ground; the
   pale band round a summit pond if the owner minds it; a program without ice when no cell holds ice on water.
5. **Years that differ** (a study first: today rain on land changes by 0.02% from year to year), and wet and dry
   sides: a law for the mean wind's momentum. The strip of the planet with land and life (it is an all-liquid,
   lifeless planet: its air is warmer and its heat-absorbing gas higher than a globe's with life).
6. Channels that lay down their load (plains, marsh, deltas). The owner asked for wide deltas at some mouths, and
   agreed to wait if it is a big change: it is. A true delta's size is the load that reached the mouth over the depth
   of the water there (Gilbert 1885; Galloway 1975), so it shows where a river of much load meets shallow calm water
   and not elsewhere. Today the made ground holds a fan under water at a mouth (`space::shelf`), the running world's
   load lands on a whole cell, and the tiles draw one channel to the shore. The package: the load settles at the
   mouth below the cell's size, the fan's top reaches the water's level, the tiles split the channel over it
   (Edmonds and Slingerland 2007), plants take it. Also: ice that cuts valleys; the exact solve of the surface's
   step by pieces; the True view's water.

**The orchestrator, between packages**: the spec and the docs say what is true (`roadmap.md` 8, step C4).

The slow tests of bodies that were skipped once (`core/tests/choose.rs`, `lots.rs`) leave with the old animals;
`tape/tests/scale.rs` stays and is run with the deletion.

## 6. The live world for the owner

The owner tests an artifact: https://claude.ai/artifact/GKxSQCDfgBAwwL2B8RvEuC (private to them). It shows main
(the version label says what it shows). To update it after a push to main:
1. `cd web && npm run build && npm run wasm` (makes `web/dist/index.html` and `web/dist/ew.wasm`).
2. Copy `ew.wasm` to a scratch folder, and the page with these removed (regexes, case-insensitive, with `\b` after
   the tag names): the doctype, the `html`, `head` and `body` tags, the `meta charset` and `meta viewport` tags, the
   `link rel=icon`.
3. Publish with the Artifact tool to that URL, the page as `file_path` and
   `files: {"ew.wasm": {"from": <the copied wasm>, "contentType": "application/wasm"}}`. If it refuses because the
   published version was not read in this session, read it first (`action: read`), then publish.

To look at the page before publishing: serve it (`python3 -m http.server PORT --directory <the full path of
web/dist>`) and drive it with Playwright (`require('/opt/node22/lib/node_modules/playwright')`, `chromium.launch()`
with no options): open
`index.html?seed=1&size=22`, wait 6 s, click the last button of `#speeds`, wait, click `#play` to pause, wait 6 s,
screenshot `#map`; to zoom, move the mouse over `#cv` and send wheel steps of -600, then wait 12 s for the tiles. Count
console errors (0 expected; ignore "GPU stall" lines) and read `.stats`. Pause before every picture. The headless
browser draws in software, so its frames per second mean nothing; the world's pace in months per second does.

## 7. Choices the owner should know of

- **References moved this round**, told to the owner: river cells with the rain's drift, then all nine with the
  bodies' start. Each is one commit.
- **A world is alone or in the planet's air.** "None" is today's closed island and keeps every reference. Wide and
  Mild open the island to a strip of the planet run once: drier (0.5 m of rain in the middle of a Wide island, none in
  its far north) and its air's gases pulled to the lifeless strip's.
- **The plateau test** states the melting rule on the balance itself; the long step is backward Euler and is off
  against short steps (laws.md 1.1 says by how much).
- **The page's world waits for its frame**, so the frame's cost is the world's pace.
- Animals are out of every run and test for now, by the owner's word.
- **Asked of the owner, no answer yet:** do the summit lake of seed 1 at the 200 m bar and a lake's shore at the 5 m
  bar look right on the live page; does the pale rounded band round a summit lake bother them; does zooming and
  panning feel slower on their phone.

## 8. Branches kept for the next session

The owner's word: no work lives only on one machine. Push a helper's branch as soon as it holds something. A forced
push is refused here: push to a new branch name instead.
- `main` and the session's work branch are the same commit.
- `wip/air-light`, `wip/air-heat`, `wip/surface-step`: the air's package as it was built; all of it is on main.
- `wip/sorted-start`: the sorted start, parked (section 4). `wip/nursery-start`, `wip/cover-nursery-g2` and
  `experiment/sorted-start` are its older states.
- `wip/lakes`: four commits on small plants, not reviewed, under the lakes' mend that is on main.
- `wip/drawing`, `wip/cover-law`, `wip/cover-costs`, `wip/page-cover`, `wip/rivers`, `experiment/climate-bands`:
  merged; kept as they were pushed.
- `tools/checks`: the check scripts and probes of this work (`tools/check/paused.js`, `phone.js`, `inspect.js`; the
  frame's bench in `tools/frame/`; `tools/rivers/` with the page's cost by CPU time (`cpucost.sh`); `tools/air/`;
  `tools/sorted/`; the climate's and the ground's probes). Study files: never on main.
