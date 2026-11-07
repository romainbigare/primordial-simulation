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

In progress when the session ended (branches kept, see section 8):
- **The water's level over each cell's own ground** (the owner's report: lakes appear and vanish, lakes on ridges,
  shores move by hundreds of metres). Findings: Flow treats each cell's ground as flat at its mean, so a 2 km lake can
  exist only in the cell means and drains in one month when the camera splits it; the sea's edge moves with
  refinement. The fix being built: a level-volume curve per cell from its own drained samples, V(L) = Σ area·(L−h)+,
  used everywhere a head is read (runoff, the flood, the sea's level, the liquid's potential energy, drain slopes,
  split and join). Then the drawing on top of WP17: ground = the drained terrain, wet where below the cell's level.
- **WP17 water as water** (flat surface, colour by depth, the shore's wetness, plants by slot): built, but NOT merged.
  Drawn with each cell's flat level, coasts became square steps along 2 km cells. It waits for the level-volume law.

## 5. What to do next, in order

1. Finish the level-volume law and WP17 on top of it (section 4). Check with the owner's view: a coast and a lake
   over several months, zooming in and out: no blinking lakes, no shore jumps, no square coasts.
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

## 7. Open choices for the owner

- **The shapes of animals:** "From above" (the default) or "Angled". The switch stays until they choose.
- **The globe's land is warm** (289 K land, 291 K surface, against Earth's 282 K and 288 K; the test's bound is 292 K
  for the surface). Cause measured: no land near the poles at 22.5° cells, and low land (123 m mean against Earth's
  840 m). Geography, not the radiation. Ask before changing the globe's terrain.
- **A law for mud and sand** (spec section 19): left open on purpose; deltas, flats and beaches need it.

## 8. Branches kept for the next session

Each is pushed under its own name if it was unfinished at the end; check `git branch -r`. Merge only after the gate.
- The level-volume law (water) and its drawing.
- WP17 water as water (to rebuild on top of the level-volume law, not to merge as is).
