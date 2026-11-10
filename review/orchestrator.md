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
- Each helper works in its own worktree (`isolation: worktree`). Tell it to `git merge main` first if its worktree is
  behind.
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
- **`main` is the base branch (the owner's decision).** Work on the session's own branch, push it freely; push to
  `main` (a fast-forward) only after the gate and all the slow tests pass on the exact commit.
- **Look at the pictures yourself before publishing**, at the owner's views: a coast and a hill at the 200 m, 50 m,
  20 m and 10 m scale bars, after the world played a while at each zoom (the shoot script below). A test or a number is not enough: square
  edges have come back twice after green gates.
- For now (the owner's decision) long runs and the live page run without animals (`ew run --roster --plants`): the
  work is on the ground, the liquid, the cover and the drawing.

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

## 4. Where things stand (main = `a93a2e33` and later)

Done and on main, beyond the tree plan (WP0 to WP16, WP21; details in `review/plan.md` and the commit messages):
- **The liquid over each cell's own ground** (a level-volume curve per cell, `flow/curve.rs`): no blinking lakes, no
  lakes on ridges, the coast holds while the camera splits and joins.
- **A cell's parts** (`flow/parts/`): its land, the liquid's top (`h_mix`, 50 m) and its deep, each with its own heat;
  the deep trades with the top through the thermocline and overturns when the top is denser; the densest temperature
  falls with what is dissolved (Despretz). The world's sea is nearly fresh (0.19 g/kg). Flow costs about twice what it
  did.
- **Rivers (WP18, the physics):** one channel law from the discharge (Leopold and Maddock 1953, `flow/channel.rs`);
  the flux a cell passes on is a flow through a split and a join (`remap/through.rs`); the terrain's samples hold their
  channels (`space/network.rs`); the map draws channels on the drawn ground at their true width (`fine/channels.rs`).
- **The map without squares (close views):** one ground below the terrain's samples for the core and the map
  (`space/below.rs`), its slopes drawn in full at every depth; the liquid at the core's own levels, evened over half a
  cell; the drawn fields averaged over half a cell (`fine/keep.rs`).
- **Seeds and dispersing children land in lots** drawn from a kernel of two scales (40 m and 800 m, `copy/spread.rs`);
  the books by cell (`store::life_sent`). Cover fell by half: a full stand's surplus now falls at home as litter.
- **Frost:** dormancy lowers the lethal limit by `harden` (20 K), one helper for cover and bodies (`Physics::frozen`).
- Test scenes that span latitudes where a test speaks of winters (snow slope, frost gene); `ew run --plants`.

## 5. What to do next, in order

1. Edges still seen on the map: snow patches with blocky edges in the far view, forest that stops at cell borders,
   small ponds whose field ends along a cell border. (The live page already draws every zoom from the fine raster; the
   map of squares stays for tapes, which hold no fine ground.)
2. The owner's view of rivers (`review/plan.md`, after section 6): a winding channel cut into the ground, its width
   and depth along its course, coloured by what it carries.
3. Lakes above the sea level as the sea does; the drawn fields keep each cell's mean and stay smooth; plant kinds and
   their marks continuous across cell borders; the raster's time (0.6 to 0.8 s a 1 m window).
4. The open items in `review/plan.md` (frost minima, the parts' solves' cost, WP19, WP20, WP22, the bodies' overdraw).

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
   no console errors, the fine picture draws at 11 wheel steps.

The shoot script (drives the page, keeps it playing, takes a picture per zoom):
`node shoot2.js PORT OUTDIR TAG YEARS "x,y;x,y" "7,11,14"`. It lives on the branch `tools/shoot` (study files stay off
main): `git show origin/tools/shoot:shoot2.js > <a scratch folder>/shoot2.js`.

## 7. Open choices for the owner

- **The shapes of animals:** "From above" (the default) or "Angled". The switch stays until they choose.
- **The globe's land is warm** (289 K land, 291 K surface, against Earth's 282 K and 288 K; the test's bound is 292 K
  for the surface). Cause measured: no land near the poles at 22.5° cells, and low land (123 m mean against Earth's
  840 m). Geography, not the radiation. Ask before changing the globe's terrain.
- **A law for mud and sand** (spec section 19): left open on purpose; deltas, flats and beaches need it.

## 8. Branches kept for the next session

None: everything of this session is on main.
