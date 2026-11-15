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
- **The disk fills** (it did, twice): each helper's worktree holds a build folder of 2 to 3 GB. When a helper is
  done and its branch is merged or saved, delete its `target` (`rm -rf .claude/worktrees/agent-*/target` of the
  finished ones); tell helpers to delete scratch runs and old binaries once their numbers are in a table.
- **Ports are shared too**: helpers serve their own page builds on port 8765. The orchestrator's smoke tests
  serve `web/dist` on a port of their own (8791) and stop only that one; stopping 8765 stops a helper's server.
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
- **A band of bright flat green beside rock or brown land when zoomed in** (the owner's fault; their picture:
  seed 1, 32 squares, Wide, year 15.92, the 100 m bar): it is the herbs' layer (the near view's sward pass in
  `lib/cover_field.wgsl`) where the trees' roots do not hold (thin loose ground by the bare bed); the low layer is
  real. Mended on main: each slot's roots fade over the change of loose depth across one of the ground's 125 m
  samples (no straight line across a sample's square); herbs cover the share their leaves close by the cover's
  light law (the setting that drew a thin sward five times too wide is gone); a young stand of a woody kind (60%
  of its grown dry mass in walls, from its walls gene: `KIND_ROW` is 20) is drawn as bushes of its own colour, not
  as meadow. Left: the herbs' tone and fine pattern are as they were (calmer only because they cover less); young
  trees are bushes, not small crowns of their form; no place with dark rock like the owner's was found to show
  it there; the herb slot's activity on that winter day was not read (the page uses one rule of dormancy for
  herbs and trees; that cell's herbs are hardier, 265 K against 270 K, so they turn later).
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
- **The variety is a setting of a world, on main** (merge 3dea286, `SNAPSHOT_VERSION` 144): `Config::variety`,
  4 unless asked, 1 to `cover::SLOTS` (now the bound, 12); `ew run --variety N`; a field on the page. At 4 the
  world is the same to the last digit. Each extra slot of the bound costs 85 bytes a cell; at 8 and 12 the
  cover's stage costs about 25 to 30% more.
- **Found with it, a fault of the cover's laws: the world depends on how a stand is cut into slots.** The 32
  drawn world in the band at year 60 holds 0.88e10 kg of cover at variety 2, 4.10e10 at 4, 6.69e10 at 8, 6.97e10
  at 12, with the same six starting kinds (variants of one kind found slots of their own). The builder's guess,
  not measured: each slot is thinned on its own, and is its own layer in the light and in the rooted share, so
  one stand cut into more slots holds more plants and more leaf. The law that should hold: a slot cut into two
  alike halves gives the same world.
- **The mend of the slots is built, not on main** (on the remote as `wip/slots`, merged with main 6f7dfe1b;
  notes and tables in `scratchpad/sorted/spike/`: `notes.md`, `contact_round3.png`, `pics/`). Five laws broke
  "a slot cut into alike pieces gives the same world" and each has its commit: thinning by the crowns' layer
  (`Cover::crown`), seeds keyed by the cell and the genes (rounded to 1/1024 of a step) and waking for their kin
  (`kin::kins`), light and liquid on the rings of nested rooted shares (`math::rings`, `cover/canopy.rs`,
  `grow::drawn`), minerals shared by what each slot would take (`build::wants`); the seeds lying keep all 12
  lots at any variety. The test `core/tests/stand.rs`: a stand cut in two and in four gives the uncut cell's
  year within 1e-6 (the genes are 32-bit, and seeds joining one packet at a time round the mean genes in the
  last bit, about 1e-8 a year; a law that acts on a slot moves it by 1e-4 or more). Where it stands:
  - **The pictures are the same world** (the page and the what-stands maps, 32 and 64, both bands, years 60 and
    100: forest a few points up in the band 20 to 65, the rest within two points). The mass moves: the key
    world's cover 9.831 → 10.119 log10 kg, the gas 330.7 → 287.0 ppm, land worn 0.1875 → 0.1333 mm/yr; the 32
    drawn world's cover falls by 40% (the light passes down through each crown, so the trees that hold the mass
    grow lighter: the builder's reading, not measured).
  - **Cost**: the cover's stage +12% for a cell of one slot, +48% per slot-month with four kinds a cell (counted
    instructions); its share of a 30-year run 16.5% → 20.5%. The cuts that keep the results to the last digit
    are in.
  - **The variety is not neutral, and that is the limit itself, on main too**: cover at 2, 4, 8, 12 is 0.55e10,
    1.52e10, 2.67e10, 2.61e10 kg on the branch (main: 0.59, 2.55, 3.81, 4.10): from about 8 the setting no
    longer matters. Full cells of stands that pay their way shut better kinds out. Tried and dropped: a full
    cell joining only kin and keeping the newcomer as seeds lying (worse: 1.06e10 at 4). Almost all forced
    joins are of unlike stands (118,000 at 4 in 60 years, 2.9 steps apart on average). Open: a default of 8
    (the cover's stage then costs about twice), or a better rule at the limit.
  - **Three fast tests of the takers are red on the branch** since it merged the drought law
    (`newborns_grow_by_the_law_of_growth_and_the_books_are_kept`, `the_young_leave_and_the_books_are_kept`,
    `the_soft_part_stands_in_the_crown`): in a dry month the test cell's herb stand has a tiny gain, does not
    pay, and `grow::shed` drops all its living matter. The same fragile spot the fire builder's work replaces
    (a leaf's gain by the liquid in a straight line; shedding by what it costs to keep).
  - The long step's test passes on main and on the branch now (`runoff_sea` +0.24% against 0.35%).
  **In work**: the leaf's year is on main, and a fresh builder (branch `slots`, scratch
  `scratchpad/sorted/merge/`) merges main into `wip/slots`, holds every law of the leaf's year to the cut (a
  dry season added to `core/tests/stand.rs`), sees the three tests, puts the variety's bound back to 1, resets
  the references, measures the cost again, and hands a branch to merge. Fire v1 merges after it.
- **On main: a world's variety is from 2** (`Config::validate`, the page's field): at 1 a full cell looked for
  two slots to join and the run stopped. The mend that lets 1 stand is on `wip/slots` (a cell of one slot held
  by a stand that pays its way refuses the seeds, `seed::Landed::Refused`); the bound goes back to 1 with it.
- Then, by a fresh builder and only after the look of the habitats (the owner's order): the spike of the nursery on its branch behind `--sorted` (no race by mass, newcomers near
  known kinds, the garden check, bands' tables, a contact sheet): numbers and pictures, not for main as it is.
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

**The owner's next order after the variety of habitat (their word): the look of the habitats, and everything else
paused until it is done.** When the habitat work is complete (the air and the rain: the rain on rising ground, the
dry belt; the orchestrator counts fire in, as what makes grassland), launch one page builder (the owner asks for a
Sonnet agent: `model: sonnet`) to make the new variety of habitat look far better, and pause the sorted start and
all other helpers until it is done. The owner's goal: the map as full of detail and information as possible and
as real as possible, with features dramatised for the game: textures, details, relief, objects.
- Faults the owner sees on the page (their pictures, on the remote as `wip/look-refs`, folder
  `review/looks/faults/`; a copy in `scratchpad/looks/`): near views of the green cover show grid-like marks and
  odd patterns (at the 20 m bar the crowns are a dark maze of straight strokes; at 50 m green and brown stand in
  square blocks; at 2 m dark smears made of fine horizontal and vertical lines, and thin straight lines at tile
  borders; at 10 m green over rock in straight lines and a grid); the new dry land is empty of detail; less and
  less to look at; bushes look like small trees; trees are too simple; no flowers, no grass; no plant drawn
  that fits the new habitats.
- What the owner wants it to look like (`review/looks/want/` on that branch): a painted dry grassland with
  flat-topped trees, tall straw grass, a dry sandy river bed, paths, mounds and herds; a forest glade with a
  stony stream, fallen logs, mossy rocks, many tree forms; a swamp with dark standing liquid, trees with wide
  feet, floating leaves, mud banks; high mountains with ice, scree, a milky lake; a braided river with gravel
  bars and green islands.
- For the brief, **the owner's rule for the drawing (their word)**: procedural detail with artistic liberty is
  welcome to make the world look alive (a boulder, a fallen log, a tuft, a flower that no number tracks), as
  long as the numbers still guide the general outline and the behaviour of what is drawn: where forest, grass,
  bare ground, liquid, snow and rock are, how tall and dense the plants, how dry, the season. Detail is keyed to
  the place so it does not move between frames or zoom levels, and nothing stands on a grid; the look differs by
  habitat because the numbers differ. The animals' drawing stays paused (see the life track).

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
  - **Paused by the owner** ("we'll continue that when we have sorted the habitat variety for them to live in"):
    the takers' three open faults. No work on the animals' numbers before the habitats and the sorted start. A
    third builder measured them and was stopped; nothing is committed (its notes: `scratchpad/f1/b3/notes.md`;
    its uncommitted trial as a patch beside them). What it found, to start from:
    - **The 30 g kind**: the bite is not its limit (1 to 5% of its feeding time is search). Its chewing is: the
      `chew` row stands for a whole day's eating but the code spends it over the working half of the day
      (`busy`); it finds nothing for three months under snow; seeds are 70% walls and use up its chewing. With
      the chewing spread over the feeding hours (tried, not committed, unchecked) the 30 g kind lives (2.5
      million adults after 15 years at 32 squares), the cold-bodied kind booms and the 60 and 400 kg kinds fall.
    - **Large grazers**: the "70 kg of dry matter per hectare" holds only without the bite's floor; with it a 60
      kg grazer pays its way at 385 to 854 kg per hectare and a 400 kg one at 666 to 1,550, near the measured
      1,000 to 1,500. "A bite takes the top of what stands" changes nothing there. But the soft part breaks so
      easily that the most a grazer eats is 10 to 13 times its need; walls inside the leaf (one anchored row:
      30 to 60% of a leaf's dry mass) would bring that to 1.1 to 1.9 times. Not built.
    - **Births**: Hamilton and others 2011 count young at weaning, not at birth. Against real newborn mass our
      births are 12 to 29 times too many (1.7 to 5 against weaned mass). Our young feed themselves at 3 to 6% of
      the adult's mass; real young become independent at 17 to 40%. The step that is too easy is likely that a
      newborn feeds itself at once. What the takers achieve in a world was not measured (the run was stopped).
  - **Paused by the owner until the takers' numbers are final; none of the three looks is to be merged.** The
    owner's words on them: all look terrible, grid-like, far too many animals, and too big. A next try starts
    from the final numbers and must not place members on a grid nor draw them larger than they are.
    Built: the first drawing of animals on the page (the design's decision 10).
    Three looks from one base (from main `be1d3d2`; on the remote as `wip/animals-look-dots`, the base and look
    A; `wip/animals-look-bodies`, a small body seen from above with a shadow; `wip/animals-look-herds`, herds of
    up to 12 and a soft mark far away). The road: `tape/src/takers.rs` gives each square 8 slots × 8 numbers,
    `web/src/herds.ts` and `web/shaders/herds.wgsl` place each member at a fixed spot, the count rounded down,
    hidden in standing liquid and under deep snow, nothing drawn where a member is under a quarter pixel.
    Pictures in `scratchpad/animals/` (`looks.png`, `sheet_*.png`, `notes.md`). Faults of the looks: members
    stand almost on a grid; the herds' far mark fills a cell's square to a hard edge; the angled view draws them
    from above; the shader's cost is counted, not timed. **What the drawing shows of the takers' numbers** (for
    the paused work): up to 75,000 per km² of the 3 kg kind and 8,000 per km² of the 60 kg kind at year 100, and
    adults far lighter than their kind's adult mass (the 400 kg kind's adults 74 kg on average in a cell, young
    sometimes heavier than adults). Then F2 (movement by what is sensed) and F3 (eaters of takers). The second builder's working room is used up: a fresh builder
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
0. **The variety of habitat** (the owner's first priority). A study with measures is done (its report is in this
   session's record; tables, probe and pictures in `scratchpad/habitats/`: `why.png`, `trials.png`, `notes.md`).
   What it found on today's main (64 drawn world, band 20 to 65, six kinds, year 100: 85% forest, 7% open low
   cover, 3% bare):
   - **Nothing kills a tree but frost.** A plant is fully active until its ground holds about 1 cm of liquid
     (`wet_depth`, read by `cover::wetness`): in the driest warm month a quarter of the land is under 46% of field
     capacity, yet a herb is under half activity on 15 of 1,470 cells. So the wet-ground tree (`MOIST` -0.05) is
     the commonest tree everywhere. No fire; every year the same; no root drowns (26% of land cells have full
     pores six months or more).
   - **Rain has no direction and almost no season**: east and west slopes get the same rain; the driest month
     gets 0.62 of the wettest's (savanna and dry-summer lands: under 0.1). The planet's dry belt is weak and has
     no season (1,165 to 1,230 mm at 18 to 28 degrees against 2,300 at the equator): the strip is all liquid and
     the overturning cell's edge is set by the year's mean sun.
   - **A band nearer the dry belt gives dry open land with today's laws** (the band 5 to 50, measured: at 10 to 20
     degrees 20% forest, 23% open, 58% bare at year 100, 540 mm of rain; too harsh for that rain, and the dry
     woody kind has not spread yet). On the page as the climate "Warm".
   - A cold open belt is there (ground frozen 6 to 7 months: 40 to 56% open, a low mat of the dry woody kind);
     its cause is the cost of frost hardiness, where a real tree line comes from a short warm season.
   - The meadow on thin ground is real early (58% of the land holds no tall slot's roots at year 20) and is
     closed later by woody plants of 4 to 6 m, not by shade from trees on deep ground: no fault of area in the
     light law.
   - Rivers stay small at every size (largest catchment 330 km² at 32 squares, 1,240 at 64, 2,600 at 128, 6,400
     at 256: drawn worlds split into more islands as the map grows; the 64 world's largest river is 13 m³/s, 11 m
     wide). A river of 1,000 m³/s needs about 100,000 km²: a world of 512 with one large land.
   - No cliffs or canyons: the makes of bed wear at rates 1.5 times apart (real hard and soft layers: 10 to 100
     times), the layers are folded, a 125 m sample holds no cliff. The ground's water numbers are one per world.
   - The stretch already covers both directions for the air's sideways laws (to the air the middle of a 128 km
     island is about 1,200 km from its shore): no more stretch.
   **The orchestrator's order from it** (each a package with its measure and its picture):
   1. Done: the page's climate "Warm" (5 to 50).
   2. **Plants and the ground's liquid** (a builder; branch in its worktree, scratch `scratchpad/water/`).
      - Law 1, on main and on the page (`SNAPSHOT_VERSION` 145): the share a plant can draw is (how full the
        pores are − `wilt`) / (`field` − `wilt`), `wilt` 0.25 anchored (Saxton and Rawls 2006). Kinds sort by the
        liquid (the moisture gene by mass: −0.8 on the driest ground, −0.4 to −0.3 on wet ground; before −0.2
        everywhere). 64 drawn world, year 100: forest 87 → 78% (band 20 to 65), 71 → 62% (5 to 50); open 10 → 18%
        and 16 → 27%; cover 33 → 22 and 28 → 19 kg/m². On the page the thin open cover of the warm band reads as
        bare ground (the 32 island's south and north shore).
      - Mended on main with it: a cell read three things from its own air node alone (the share of the sunlight
        its air and cloud let through, the heat its air sends down and takes up, the height its vapour fills),
        so the ground's liquid in the driest warm month stepped at the nodes' borders and the page drew green
        squares. All three are shared between the nodes' centres as the temperature is (the mean jump across a
        border over that inside a node: 1.14 → 1.00).
      - Law 2, built on the branch `soaked`, in review: roots under the depth where the pores are full get no gas
        and split their fuel by the chemistry's own reaction (13 times the fuel for the same work); a gene for
        open channels (`CHANNEL`, cost 1/(1−ε) of the fine roots and conducting walls) carries gas down by
        diffusion (Armstrong 1979); the burning fine roots lie near the top whatever the height (share above z:
        1 − e^(−z / `fine_root_m`), 0.3 m, Jackson, Mooney and Schulze 1997), the frame's reach by height as
        before; a starting kind of soaked ground. Year 100, law 1 → both: forest 78 → 69% and 62 → 54%, open
        18 → 24% and 27 → 34%; the wet-ground tree halves on ground full six months or more. The soaked kind
        holds no more of the cover on full ground than off it: channels are cheap and pay wherever the free
        liquid nears the top for some months (seasonally soaked ground). Judged honest; a weaker stem for
        channels is not (wood carries none). **Not merged: it gives no new habitat** (on the remote as
        `wip/soaked`, gate green on main 6f7dfe1b; contact sheets `scratchpad/water/contact_soaked.png`,
        `contact_herb_trial.png`). Of the tall cover the soaked kind leads 36% on full ground and 41% off it;
        the new open land lies on dry ground and on the rim with standing liquid, not on soaked ground (open
        share of full ground 50 → 55%); a starting herb of soaked ground makes no marsh; the 10th gene costs 4
        bytes a slot and no time that can be measured. **Shelved until the sorted start can test kinds on soaked
        ground.** The branch's last commit (360a94e8) holds the stronger reading, also tried: a root without
        gas does 37/484 of the work on the same fuel, so the slot's uptake falls (a 20 m tree on ground soaked
        to the top draws 0.34, with channels 0.80). It moves the soaked kind to 45% of the tall cover on full
        ground against 41% off it and the other trees to 36%, the open share of full ground to 56%, the low
        plants not at all: still no marsh (`contact_uptake.png`). The branch's references belong to the first
        reading. What a swamp may need beyond these laws, not tried: no seedling under standing liquid, ground
        that differs by its grain.
      - Open, for the sorted start's garden check: **the moisture gene costs nothing real** (its cost is capacity
        × 10^MOIST, and one step of the light gene gives the same capacity at the same upkeep).
      - On main, a mend of the books in Flow (`runoff::book_all`, test
        `what_runs_off_takes_no_more_of_a_liquid_than_a_cell_holds`): a surface pass books all its moves
        together, those between cells and those with the boundary at the open edge, so an edge cell never gives
        of a second liquid what has come in before that is booked. A world of one liquid is the same to the last
        digit.
   3. **Rain with a season and a direction** (the air's helper).
      - On main: the overturning cell follows the month's sun (`air::cell`, Lindzen and Hou 1988). On the strip the
        rain belt swings to 12 to 17 degrees; the driest month at 10 to 20 degrees gets 0.55 to 0.61 of the wettest
        (land of the band 5 to 50: 0.48; a savanna: under 0.1). The belt peaks three and a half months after the
        solstice (the measured one: about two).
      - Open: **the dry belt stays wet** (1,120 to 1,170 mm at 22 to 35 degrees; the measured ocean belts: 600 to
        900). Measured cause: the belt sends away 260 of its 1,430 mm of evaporation (the cell carries a fifth of
        the measured flux, `wind` "anchored, open", and its share falls to none where the air sinks), and no term
        of the rain law reads whether the air rises or sinks. The halves already carry their own vapour; the
        vapour's profile is tall (about 4 km against the measured 2).
      - Tried and not kept: rain following the rise and sinking of the overturning's low flow. No rain law dries
        the belt alone: over a year a belt's rain is what rises from it less what the air carries out (the belt
        22 to 33 degrees: 1,167 → 1,101 mm, while the rising branch went to 2,854 mm).
      - **On main and on the page: the wind lifts the air where the ground rises** (`flow/air/rise.rs`; Smith and
        Barstad 2004); the wind is the overturning's own low flow turned by the spin, an easterly at every
        latitude, 0.4 to 1.6 m/s. Windward over lee at one height: 1.5 to 2.6 on the 64 island of the band 5 to
        50, 3 to 5 on the 32 island of the band 20 to 65 (the real: 2 to 5). Closed islands are unchanged.
        Known limits: no eddy-driven westerlies of the middle latitudes (the wet side there is the east); the
        wind is a fifth of the measured trades, the same fault as the weak cell; rain falls on the sea beside a
        windward coast (the rise is taken per air node).
      - **On main with it: the air takes the ground's liquid by the share it holds for roots**
        (`Physics::available`, one helper for the ground and the plants; Manabe 1969): the pores dry only to
        the wilting point. The cells never wet fall from 212 to 19 on the 64 world of the band 5 to 50.
      - **The page is barer with the two** (year 60, bare / forest: 64 band 5 to 50 0.33/0.41 → 0.37/0.35; 64
        band 20 to 65 0.29/0.46 → 0.44/0.33; 32 band 5 to 50 0.22/0.44 → 0.24/0.40; 32 band 20 to 65 0.21/0.48 →
        0.49/0.29): the lee sides are dry land now, a wet east and a dry west on every island
        (`scratchpad/air2/c2r/page_sheet.png`). A pale square shows on the 32 island's dry west on the page
        (seed 1, band 20 to 65, year 67): not looked at yet.
      - **On main: the overturning answers to the warmth's fitted gradient** (the mean of the two ends' fits
        along the link), not to one link's own difference: a wave two nodes long is the mesh's, and it fed
        itself by the rising branch (a checkerboard of rain at 5 to 15 degrees). Closed islands and the key
        numbers are unchanged to the last digit. What it shows: **part of the rain belt's swing was that wave**
        (the belt swings 0.6 degrees on the strip, not 2.2), the equator's rain 2,041 → 1,597 mm, the islands'
        dry belt wetter again (64 band 5 to 50, 15 to 25 degrees: about 1,030 mm, the driest month 0.55 of the
        wettest; bare 0.37 → 0.17, forest 0.35 → 0.45 at year 60); the cell's low flux on the lifeless globe up
        40% (3.8 to 4.4e10 kg/s at 22 degrees; the `wind` row's measure is to be set again).
      - **The pale square on the 32 island's dry west** (seed 1, band 20 to 65): four air nodes of the lee never
        rain, and rain reaches a cell only from the nodes it reads, drift landing inside its own node, so the
        cells between those nodes' centres get none. In work: the rain a node makes drifts with the wind across
        the nodes' borders.
      - Then for the air's helper: the two very wet southern corner nodes of the 64 world of the band 5 to 50;
        the measure of `wind` at 2, 3 and 5 times, which now decides the dry belt and its season; the long
        step's error on the land's runoff.
   4. Fire, as a fast event inside a month: the chemistry's own burning running away in dry fine fuel, what
      keeps grassland open (Bond, Woodward and Midgley 2005). A fresh builder (branch `fire`, scratch
      `scratchpad/fire/`: `design.md`, `fire_today.png`, tables).
      - Measured on main (64 drawn, year 60): **open land cannot carry fire: herbs never cure.** Living tissue
        keeps its liquid in every season and `grow::shed` sheds nothing in a dry month, so open land holds 0.19
        (band 20 to 65) and 0.01 kg/m² (5 to 50) of dead fuel (a real dry grassland: 0.2 to 0.6, cured). Forest
        holds about 5 kg/m² but stays wet. Fire could run today in a thin ring of forest edge by the dry
        coasts, about 4 months a year. The chemistry's burning rates are those of rot (the frame's bare
        reaction reaches 1e-6/s only at 796 K): what burns into what and the heat come from the chemistry, the
        speed from the heat balance (Rothermel 1972).
      - In work, step 1, mergeable alone, two law commits. Measured first: with today's costs no leaf is ever
        worth shedding, because the living upkeep is multiplied by activity and activity falls to about zero on
        dry or frozen ground (keeping a kg through a dry season costs 0.005 to 0.007 of building it anew). So
        (a) **upkeep follows temperature, not the ground's dryness** (a leaf on dry ground still breathes; the
        gain keeps following the liquid; seeds lying and wear keep Harrington's rule): keeping then costs 1.1 to
        2.4 times rebuilding through a dry season and 0.17 through a cold one; (b) **what costs more to keep
        through the months a slot expects without gain than to build anew is shed** (Kikuzawa 1991), one rule
        for every season in `grow::shed`, the expected months from the slot's own last year. About half the
        dry-season cover would become litter; in the cold season leaves are kept (leaves falling in the cold
        would need frost to act on leaves, not on whole plants).
      - Built on `fire` (three commits, gate green, `Cover::idle` one number per cell, `SNAPSHOT_VERSION` 146;
        `scratchpad/fire/step1_page.png`, `step1_tables/`), **not merged: the dry lands get far too bare.**
        Year 100, 64 drawn, main → a → a+b: forest 0.78 → 0.44 → 0.47 (band 20 to 65), 0.62 → 0.38 → 0.37
        (5 to 50); bare 0.05 → 0.29 → 0.26 and 0.12 → 0.36 → 0.35; land at 10 to 20 degrees half bare on 1,100
        mm of rain; no wood that stands leafless through the dry season. The good half: herbs on open land cure,
        dead fuel there 0.3 to 0.7 kg/m². Causes found: **a leaf's gain follows the liquid by the seeds' power
        (activity to the 5.7, Harrington), so a half-dry month gains almost nothing** (a fault of main's drought
        law; FAO-56 gives a straight line to the wilting point); and a plant paid a full month's upkeep before
        it shed.
      - Second round on `fire` (c the straight line, a, b with shedding before the month's upkeep; gate green,
        `scratchpad/fire/step1c_page.png`, `step1c_table.txt`): main → c → c+a → c+a+b, forest / open / bare,
        band 20 to 65: 0.78/0.18/0.04 → 0.88/0.09/0.02 → 0.51/0.25/0.24 → 0.54/0.26/0.20; band 5 to 50:
        0.62/0.26/0.12 → 0.76/0.18/0.06 → 0.41/0.27/0.32 → 0.46/0.34/0.20. Woods stand leafless in the dry
        month on 6 to 7% of forest cells; dead fuel on open land in the driest month 0.28 to 0.54 kg/m² (real:
        0.2 to 0.6). A leafless plant pays nothing (known limit); a kept leaf pays 2.7% of its dry mass a day at
        295 K (real: 1 to 3%); the herb's store pays a quarter of a month of that, the tree's three months
        (real stores: grasses 10 to 30% of dry mass, trees 5 to 10%): no change to the start's rows.
        The three commits go to main together (c alone makes the page 88% forest); the slots' mend merges
        after them.
      - Third round (a fourth commit: out of season a slot keeps only the leaves its store can pay for and
        drops the rest as leaves, so it stands leafless with its store; `step1m_table.txt`,
        `step1m_bare_class.txt`, `trace_*.txt`): bare 0.18 in both bands (main 0.04 and 0.12). **The cells that
        went bare get 530 to 680 mm, and main's cover there was never alive**: it sat through dry or frozen
        months at no cost, and on ground under the wilting point all year it stood unchanged from the year it
        was placed. Three classes: cold (7 months frozen, summer ground at 273 to 280 K, near 55 degrees: 174
        cells of the band 20 to 65), a dry season of 6 to 7 months (79 and 130 cells), ground never wet (14 and
        70). So the branch is the more honest world; it merges after two small mends, in work: the budget rule
        asks the store only to pay the keeping; **a slot is in season only while its gain pays for all that gain
        needs, the fine roots that draw its liquid too** (in the dry cells' last half-dry month the roots'
        upkeep, 42% of the whole, emptied the store and the herbs died as whole plants): a plant whose pores
        would lose more than they bring keeps them shut (Cowan and Farquhar 1977).
      - Fourth round (`wip/fire`, six commits, gate green; `step1r_table.txt`, `step1r_page.png`,
        `trace_roots_2508_1108.txt`): both mends are in (`upkeep::root_share`: from a roots' share of 1 of the
        gain the pores stay shut). Forest / open / bare: 0.59/0.23/0.17 (band 20 to 65), 0.51/0.32/0.17 (5 to
        50); no plant dies whole in the half-dry month any more; a tree keeps its walls and store through five
        dry months. **Not merged: in hot air plants sit shut through wet months** (cell 2508, 1,510 mm: the
        fine roots for the month's liquid cost 1.47, 0.92 and 1.07 of the gain at 300 to 303 K; a real plant
        spends about a third on its roots). The season rule is right; the roots' number is off: `root_uptake`
        is anchored on what a temperate forest's roots carry in its peak month, a typical flow, and is used as
        what a root can carry. In work: re-anchored by a method fixed before any run (the highest biome value of
        peak-month transpiration over live fine-root mass). Left as they are: the cold class (near-tundra, a
        summer too short to pass the bare line), the never-wet cells (the bucket).
      - **Step 1 is on main** (seven commits, `SNAPSHOT_VERSION` 146, `Cover::idle` one number per cell): a
        leaf's gain falls in a straight line to the wilting point (FAO-56); its upkeep follows temperature; it
        is in season while its gain pays its keep and the fine roots for its liquid, and keeps its pores shut
        otherwise (`upkeep::root_share`); out of season it is shed where keeping it through the expected months
        costs more than building it anew or more than the store can pay (Kikuzawa 1991); `root_uptake` is the
        highest flow roots are seen to carry (1.6e-3, a transition forest's wet season). Year 100, 64 drawn,
        main before → now, forest / open / bare: 0.78/0.18/0.04 → 0.63/0.21/0.16 (band 20 to 65),
        0.62/0.26/0.12 → 0.54/0.30/0.16 (5 to 50); land at 10 to 20 degrees bare 0.23 → 0.17; woods leafless in
        the dry month on 8 to 9% of forest cells; dead fuel on open land 0.29 and 0.09 kg/m² in the dry month.
        Known limits, in `docs/laws.md` or here: a leafless plant pays nothing; at 303 K the roots still cost
        all of the gain and the first hot wet months stay shut (the roots' breathing goes by the bare rate
        law, real roots acclimate); the cold class stays bare (near-tundra: real ground like it holds 0.5 to 1
        kg/m² of low cover); coastal forest 0.71 → 0.48 and 0.44 → 0.32 of coastal land cells; the takers'
        tests place their cover (`FED_KG`).
      - **Fire v1 is built, not on main** (`wip/fire-v1`, the law in `core/src/flame/`, test
        `core/tests/flame.rs`, gate green, books exact, `SNAPSHOT_VERSION` 147, a map layer `burnt`;
        `scratchpad/fire/v1/contact_sheet.png`). It costs 1.2% of a year, 0.4% where nothing burns. **It does
        not behave as fire**: 0.42 and 0.71 of the land burns a year (the real planet: about 0.03), the same
        cell 2 to 4 months of one dry season, forest every 2 years, flames of 8 to 14 cm that kill only
        seedlings, no open belt, and the burnt share depends on the world's size. It burns in the right
        places (the forest's dry edge; none on bare or the wettest ground). Three pieces of physics are
        missing, in work: (1) **reach**: a fire covers what it spreads over while it lives (spread rate ×
        the life of a fire, one anchored number, Andela et al. 2019), where the design let it take its whole
        joined region; (2) **a least load**: the balance has no losses, so 0.03 kg/m² of dead leaf carries a
        flame and burns again next month; a thin bed radiates its heat away; (3) **the bed**: `litter_rho`
        (40 kg/m³) is packed litter, a grass fire's bed is the cured grass still standing (1 to 5 kg/m³): a
        proposal first (the walls of leafless low slots as standing fuel, and a low plant burnt to the ground
        growing again from its store). Slips of the design note mended in the build: the burning time per
        foot, Van Wagner's plain scorch fit (no `t_kill`), `flux_ahead` 0.05 kept though Rothermel's own
        formula gives 0.025 for 1.5 mm particles.
      - Found with it and mended on main (see the rain's item): the ground's bucket dried by evaporation below
        the wilting point, so on semi-dry land rain never became liquid a root can draw.
      - Then step 2, fire v1, the design accepted: the fuel is the cover's remains; the fuel bed's air is pulled
        to the ground's wetness; the moisture of extinction falls out of the heat balance (0.17 to 0.18); a
        cell carries fire where fuel covers more than the percolation threshold of the ground; burnable cells
        that touch join into regions, each burning the share its expected strikes give (no random draws, no
        front solver); leaves under the scorch height die (Van Wagner 1973), young trees must outgrow the flame;
        gases and heat to the air, ash to the ground, seeds lying survive. Five numbers and two of a sorption
        curve, all anchored. Not in v1: a bark gene, the loss of the fixed element to the air, a reach limit.
   5. Then: the ground's water numbers by grain (heath on coarse ground, wet meadow on fine); a wider spread of
      the bed's wear rates and flat layers on a high land in some worlds (scarps); one large land in a world of
      512 for a large river and a delta (with the package of the mouths).
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
