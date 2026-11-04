# Handover for the next orchestrator

Simple English. Read `CLAUDE.md`, then this file, then `docs/laws.md`. `review/top-30.md` is the list of cuts this work
follows; `review/big-look.md` says why. The spec is `docs/handover.md`; where it and the code differ, the code and
`docs/laws.md` are the truth.

## 1. Where the code stands

- `main` = `claude/handover-implementation-yw7dgd` = `3711eb44`. It builds; the fast suite, the slow tests, clippy
  and the wasm build pass. Every package on it had an adversarial review and its blocking findings fixed.
- Speed, 128 x 128 island (9,163 cells), quiet machine, 4 cores, 45 years: 1,698 ms a year on average, **2,533 ms a
  year over years 42 to 45** (settled, about 16,000 bodies): bodies 1,495, Flow 734, tape 136, Change 105, parts 93.
  Years before 22 have no animals, so a 30-year mean hides the settled cost. The goal is 150 ms: about 17 times off.
  The globe of 64 x 32 air nodes runs in about the island's time (that goal is met).
- Costs (callgrind, one thread): Flow about 86,000 instructions per cell-month; the bodies' batch about 100,000 per
  node update. The bodies are bound by memory traffic (about 500 last-level misses a decision), not instructions:
  Part B cut 12% of their instructions and none of their time.
- `core/src` about 11,600 code lines.

## 2. What this round did

- **WP6 review and fixes.** The world no longer depends on the chunk count (a chunk updates cell by cell); the threads
  test checks it on a crowded roster and on a small globe. Brains learn by the normalised least-mean-squares rule
  (Nagumo and Noda 1967), so the count of inputs never slows learning; the learning tests pass with a wide margin.
  Planning adds γ(V(s′) − V(s)). Mass classes are fixed decades.
- **Parts and verbs.** Parts run no reactor; their walls rot at Change's own rate (`change::rate`). Dig, anchor, lever,
  drifting structures and ground bonds are deleted.
- **WP8.** Chemistry 0 has 14 substances (was 26), one limiting mineral, life of two materials (body matter, frame).
- **Flow.** Ground nodes solved again only when their state can move; the low basin under one air node answers the air
  as one patch; the liquid step leaves the sea out; gases dissolve once a month.
- **WP7a.** One mesh of cells of 1, 2 or 4 squares a side (`Square::mesh`, `Square::size`); open water away from coasts
  in whole air blocks; a globe in whole air blocks with one fine rectangle (`--fine X,Y,W,H`).
- **WP7b.** The air's wind does not depend on the grid: the overturning follows the warmth gradient, the eddies and the
  overturning are one backward-Euler day solved by lines (`solve/line.rs`). Both globes hold about 288 to 291 K, ice
  0.09 to 0.13, poleward transport 4.2 / 5.4 / 2.8 PW at 22.5° / 45° / 67.5°.

- **After the owner's two yeses.** The cells exchange with the air every 15 days (one step a month warms winters
  0.8 K, past the owner's 0.5 K; the jump appears only between 15 and 30 days, cause not found: untested are the
  month-start ground state, Change, the cover's month, the soil-water sink). Slow bodies every two months was built
  and reverted: a body burns fuel at the same rate in time, so half the decisions halved its learning.
- **Body heat.** Bodies reached 365 to 14,000 K (an unbounded coat, the pace from the month's start, dead members'
  heat through survivors, no heat-capacity limit, bodies bigger than their room counted inside it). Now the coat is a
  share up to a mammal's least conductance (Herreid and Kessel 1967), the temperature moves by 1 − e^(−g·dt/C), a
  body only sheds heat after its decision, and none passes its liquid's boiling point. On the roster only 2 to 4
  kinds live at year 40 (were 7): the founders were set under the runaway heat. A life round should look at it.

- **Parts A and B (design, review, build, review, fix).** Flow: fixed-size arrays in the ground's response, the pool
  only where its work pays (`par::pays`), the ground law once a day, the month's Flow booked as one, the vapour search
  from yesterday's place: Flow 101k → 86k per cell-month. Bodies: blocks written once, unread rows skipped, children
  who stay keep their parent's born rows. Measured and rejected: rows per lineage as a delta (the delta is dense),
  one scorer for the part verbs (35% of learning), one node per lineage with a mass spread (today's node-mean rule
  already matches the McKendrick-von Foerster steady state: 22% of births reach adulthood), lumpier litters (the
  founders die on the 32 x 32 island; diff in git history of the session, not committed).
- **Two book bugs found by 100-year runs:** a pit could send its liquid back upstream, so it circled for a century and
  fell 5 km at once (fixed: a pit spills by its way out); a cell far over a boiling point could give more heat than it
  held (fixed in `hold()`).

## 3. Next

Cuts inside this design are used up (each now gives 5 to 15%, and the bodies are bound by memory). The island's
goal needs a new design of a body update: today it streams about 32 KB of brain rows and stores per decision. Ideas:
rows in f16 (born rows are read-only), one decision per node that does not copy rows into the view, fewer nodes per
cell. Flow's rest is spread: the cells' surface balance 10k, the liquid 20k, the air book, `wet`, state.

## 4. Known problems

- `mix` is tuned (1.3e6, anchor 1e6): the air also carries what the sea carries on Earth. The coarse globe ends about
  3 K warmer than the fine one. Rotation does not turn the middle-latitude cell (laws.md 1.6, known limits).
- The fine region of a planet is the globe's squares (about 156 km), not 2 km: a real 2 km region needs the cell tree
  stored per cell. Refine and collapse during a run, the trigger and `relief` are not built (planner's design: an
  exact remap of every per-cell array, `relief` for the potential energy of flattened ground, ids by cell serial).
- On coarse globes there are few, very large land cells (23 bodies on the 32-row globe); a move of one link is 625 km.
- The eaters are heavy (about 1 kg per m² of land); the carbon settles near 260 ppm with life (no carbon source).
- The sight gene now costs nothing (it adds no work); selection on it is only by what it shows.
- A cell of a sea patch pays its share of the patch's heat by heat capacity; a cell far off its patch's temperature
  (9,000 K in a probe) can overdraw another cell of the patch. Patches are evened monthly, so no run reaches it.
- In its own cell a node's born rows no longer change (children who stay keep them); they vary only through litters
  that start bodies in other cells (selection between cells, laws.md).

## 5. How to work (keep)

- One Opus agent per package in its own worktree; reuse the agent for its review's fixes and the next package in the
  same code (up to about 700,000 tokens). A fresh adversarial reviewer after each package; every review this round
  found a real problem (a chunk-dependent world, a second rate law, uneven rain at coarse edges, a false anchor).
- At most two heavy agents at once (4 cores, 15 GB). The container restarts often: agents commit small and push only
  finished commits; worktrees survive a restart and an agent can be resumed.
- Gate to `main`: blocking findings fixed; fast suite, clippy, wasm in the main checkout; the slow tests before a large
  merge.
