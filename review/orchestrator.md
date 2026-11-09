# Handover to the next orchestrator

For the next agent who leads the work. Simple English, as the owner asks. Read this first, then `CLAUDE.md`, then
`review/plan.md` (the work packages and where each stands), then `docs/handover.md` (the spec).

## 1. The owner and how they work

- The owner writes short messages, often from a phone, and reads simple English only: short sentences, common words.
- They give a direction and expect it done "with agents": you are the orchestrator, architect and reviewer; helpers
  (subagents) write the code; you check, merge, run the gate and push.
- They test the live world themselves (section 6) and send screenshots. Treat each screenshot as a bug report: find
  the cause with numbers before fixing.
- **Their two rules, above all others:**
  1. **Things emerge from general physics and chemistry laws.** No rule made for one thing (no "snow melts less on
     sunny slopes" rule; a general sunlight law instead). A share inside a cell comes only from a general law
     evaluated at its samples.
  2. **Do not compute what is not needed.** Compute once what does not change (the terrain), skip what no one sees.
- They accept honest physics and do not want tests loosened. When a test fails, find whether the scene, the law or the
  grain is at fault; never widen a bound, never pick a scene that happens to pass.

## 2. How to run helpers

- Give each helper a full brief: the goal, the owner's words, what is known, the files, what not to touch (another
  helper's area), the gate, and the git rules below. Run at most about 3 at once: the machine is shared and the
  container restarts sometimes (helpers then lose their running commands; resume them with SendMessage; their
  worktrees survive).
- Each helper works in its own worktree (`isolation: worktree`). Tell it to `git merge claude/three-levels` first if
  its worktree is behind.
- **Git rules for every helper** (put them in each brief):
  - commit with plain `git commit` (commits are signed automatically); NEVER `git commit-tree` (its commits are not
    signed and the stop hook rejects them);
  - end every commit message with exactly these two lines:
    `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` and
    `Claude-Session: <the session link the system gives you>`;
  - no model names anywhere else; do not push; no pull request.
- Before merging a helper's branch: check every commit is signed
  (`git cat-file -p <commit> | grep -c '^gpgsig'` gives 1), look at its pictures, read its law commit messages. Squash a
  branch of WIP commits into one clean commit with a full message (`git merge --squash`).
- On a merge conflict of `SNAPSHOT_VERSION` (`tape/src/snapshot.rs`), take a number above both sides.
- Push only to `claude/three-levels` with `git push -u origin claude/three-levels`, and only after the gate passes
  (and the slow tests, for a law change). The stop hook nags about unpushed commits while the gate runs: wait anyway.

## 3. The gate

Run it before every push (CLAUDE.md 6.3). A script that stops at the first failure:

```
set -eo pipefail
export CARGO_BUILD_JOBS=3
cargo build --release --workspace --all-targets
cargo test --release --workspace
cargo clippy --release --workspace --all-targets      # must print no warning
cargo build -p pcore --target wasm32-unknown-unknown
cargo build -p ew-wasm --profile wasm --target wasm32-unknown-unknown
cd web && npm run build && npm run wasm
```

For a law change also the slow tests: `cargo test --release --workspace --no-fail-fast -- --ignored` (about 15 to 25
minutes on a busy machine). Do not pipe a step into `tail` without `pipefail`: a failed build then looks green.

## 4. Where things stand (main = `claude/three-levels`)

Done in the last session (details in `review/plan.md` and in each commit message):
- WP0 to WP13 of the tree plan (one tree of cells, split and join with exact books, the step by the size, rates per
  second, refinement by an interest score under a budget, the camera as a recorded input).
- Objects as lots (O1 to O6, `review/objects.md`); the founders' low score for forming parts stays (owner's decision).
- The viewer: true-size life, herds as a soft blob only near the animals' own size, zoom to 80 px a metre, plant
  marks on world-fixed hashed grids, no grid seams, fades between pictures.
- WP15 inside a cell: the terrain's samples are drained once at the start (priority flood); every cell's height and
  relief are means over them; one rule shares a cell's means over its own ground (soil liquid by the wetness index,
  each plant kind by its own growth law); a split uses it; the raster hands the shares to the viewer.
- WP16 the map on the graphics card: WGSL shaders written once (`web/shaders/`), GLSL for WebGL2 made at build time by
  `web/shade` (naga); WebGPU first, WebGL2 where it is missing. WebGPU was seen running only on a software GPU here:
  **ask the owner how smooth it is on their phone.**
- WP21 sun on slopes: Lambert's law over the day on each cell's slope and facing; a square's sun equals the sum over
  its cells; a sunny slope loses its snow first.
- Grain fixes: seeds land evenly over the area they reach (`copy::dispersed`); the meal's heat limit (a units bug);
  the liquid reads the phases the air's step left (lakes on ridges blinking each month).

In progress when the session ended: **the water's level over each cell's own ground**, on the pushed branch
`claude/water-levels` (section 8). The owner's report: lakes appear and vanish, lakes on ridges, shores move by hundreds
of metres. What the branch holds (read its commit messages):
- Flow keeps each cell's liquid at a level over its own drained samples: a level-volume curve per cell,
  V(L) = Σ area·(L−h)+, in one place (`core/src/flow/curve.rs`), used by the runoff, the flood, the basin, the sea's
  level, the liquid's potential energy (at its centroid) and the split. `World::relief_scale` is its own field; the
  terrain keeps its own seed (`World::drained`); the samples are built without a lock held across rayon; a split
  builds Flow's state first; worldgen starts the sea at 0 m over each cell's own ground (`flow::volume_under`); the
  surface's reflectance and evaporation read the wet share of the cell's own ground.
- Measured against main: lake area that appears or vanishes in a month 2.3 → 0.4 km²; the largest month's change of a
  lake's depth 1.34 → 0.11 m; the coast's wet area as the camera splits and joins within 0.2% (main −10%).
- The drawing: WP17's look (flat water coloured by depth, sky light, the shore's wetness, plant marks by slot standing
  in shallow water) on the drawn ground = the drained terrain; a pixel is wet where that ground lies under its cell's
  level; the drawn volume matches each cell's within 1%. The coast holds still through a year while zooming.
- Test scenes chosen by what they state (`testkit::inland`, `testkit::mild_land`, `testkit::flat_round`), and the
  known limit of a node's one heat state written in `docs/laws.md` 6.3 (the owner chose to accept it and restate the
  two node tests of `core/tests/steps.rs` on scenes away from the heat turning point).
- **Two slow tests fail on the branch, so it is not merged:** `a_globe_without_life_keeps_an_earth_like_climate`
  (surface 293.6 K, bound 292) and `the_frost_gene_falls_where_winters_are_cold`. One cause, measured: a coast cell,
  now part land and part sea, is one heat node, so its heat capacity grows about 17 times (like 34 m of water over the
  whole cell); its land gets the sea's mild winter and loses its snow and ice (globe reflectance 0.181 → 0.137).

## 5. What to do next, in order

1. **A mixed cell's two parts, each with its own temperature** (a law change, then merge `claude/water-levels`). The
   design from the last helper: each cell stores one more number, the heat of its liquid part, J (the node's total heat
   stays in the store, so the books keep their form; moves by other laws take heat from each part by its share of the
   heat capacity); the parts' areas from the curve's wet share (`Curve::wet`); a cell with wet share 0 or 1 runs exactly
   as today (bit for bit); Flow finds a melting state, snow cover and reflectance for each part of a mixed cell; in the
   air step each part is its own surface, and rain lands on each by its area; the parts trade heat at the shore by one
   law (conduction through the ground and the liquid's mixing, one anchored number); life, the cover and bodies read
   the land part, a wading body the liquid part, Change and the tape the area mean; SNAPSHOT_VERSION bump; split and
   join share the parts' heat. About 1,000 lines in Flow (land state, air step, rain, booking, basin, drain, runoff,
   inside) plus remap, snapshot, worldgen and readers. Commit in stages; the law commit lands when the globe and frost
   tests pass as they are, with all slow tests and a 100-year run before and after. Then merge the branch and update
   the live world.
2. WP18 rivers as real water: no river vanishes when cells are fine (likely its flux splits over fine cells, each
   under the river limit: measure it); channels of true width and depth (Leopold and Maddock 1953); small channels from
   the flow on the drawn ground.
3. WP19 ground of materials; WP20 single plants with crowns of form (the raster already hands each pixel's cell and
   each slot's walls and leaf area per kg, for WP20).
4. WP22 no rule made for one thing: objects covered by the loose ground's own movement instead of a burial rate of
   5 mm a year; liquid from upstream enters a cell where the flow crosses its border; groups of objects join only when
   truly alike, the cap by cost, not a fixed 16.
5. Two overdraws found by the WP21 helper (CLAUDE.md 3.4, "a law never overdraws a node"): in a 100-year run one
   `World::take_chunks` booking took 3.9 MJ more heat than a bodies' node held (zeroed and counted by the store); a
   debug build stops at year 92 to 93 on −1e-10 J in Flow's lift booking. Find and fix both.

## 6. The live world for the owner

The owner tests an artifact: https://claude.ai/artifact/GKxSQCDfgBAwwL2B8RvEuC (private to them). To update it after
a merge:
1. `cd web && npm run build && npm run wasm` (makes `web/dist/index.html` and `web/dist/ew.wasm`).
2. Copy `ew.wasm` to a scratch folder, and the page with these removed (a regex): the doctype, the `html`, `head` and
   `body` tags, the `meta charset` and `meta viewport` tags, the `link rel=icon`.
3. Publish with the Artifact tool to that URL, the page as `file_path` and
   `files: {"ew.wasm": {"from": <the copied wasm>, "contentType": "application/wasm"}}`. If it refuses because the
   published `ew.wasm` was not read in this session, read it first (`action: read`, `path: ew.wasm`), then publish.
4. Check the page in a browser before publishing (serve `web/dist`, Playwright is installed; headless uses WebGL2):
   no console errors, a zoom to the finest cells draws.

A second artifact shows `claude/water-levels` for the owner to look at the new water: publish it the same way to its
own URL: https://claude.ai/artifact/XPs1YXbaJquG1n61ec6LE5 .

## 7. Open choices for the owner

- **The shapes of animals:** "From above" (the default) or "Angled". The switch stays until they choose.
- **The globe's land is warm** (289 K land, 291 K surface, against Earth's 282 K and 288 K; the test's bound is 292 K
  for the surface). Cause measured: no land near the poles at 22.5° cells, and low land (123 m mean against Earth's
  840 m). Geography, not the radiation. Ask before changing the globe's terrain.
- **A law for mud and sand** (spec section 19): left open on purpose; deltas, flats and beaches need it.

## 8. Branches kept for the next session

- `claude/water-levels`: the level-volume law, the water's drawing, the test scenes and the node limit (section 4).
  The normal gate passes on it; the globe and frost slow tests fail until the two-part law (section 5, item 1) lands.
  A preview of it for the owner is published as its own artifact (section 6), next to the main one.
