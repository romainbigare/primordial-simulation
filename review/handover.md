# Handover for the next orchestrator

Simple English. Read `CLAUDE.md`, then this file, then `docs/laws.md`. `review/top-30.md` is the list of cuts this work
follows; `review/big-look.md` says why. The spec is `docs/handover.md`; where it and the code differ, the code and
`docs/laws.md` are the truth.

## 1. Where the code stands

- `main` = `claude/handover-implementation-yw7dgd` = `fe6a543d`. It builds; the fast suite, the 11 slow tests, clippy
  and the wasm build pass. Every package on it had an adversarial review and its blocking findings fixed.
- Speed, with life, 30-year means (the machine was not always quiet):

| World | Cells | ms a year |
|---|---|---|
| 128 x 128 island | 9,163 (open water in 8 km cells) | about 1,390 (2,160 over the last 3 years) |
| Globe of 64 x 32 air nodes | 6,080 | about 1,080 (1,220 over the last 3) |

  The goal is 0.15 s a year for the island on 4 cores, and the planet in the island's time. The planet part is met.
  The island is about 10 times off.
- Costs: Flow about 16,500 instructions per cell per 10-day step (before the coarse open water); a body update about
  113,000. Both are spread wide; no part is above about 15%.
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

## 3. Open choices for the owner (asked, not answered)

1. Cells exchange with the air once a month, not three times: about 3 times cheaper for the cells' part of Flow; the
   last round measured the seasons 0.5 K softer.
2. Slow bodies (cold-blooded kinds) update every two months.

Without these, or a simpler body, the island will not reach 0.15 s: small cuts give 5 to 10% each now.

## 4. Known problems

- `mix` is tuned (1.3e6, anchor 1e6): the air also carries what the sea carries on Earth. The coarse globe ends about
  3 K warmer than the fine one. Rotation does not turn the middle-latitude cell (laws.md 1.6, known limits).
- The fine region of a planet is the globe's squares (about 156 km), not 2 km: a real 2 km region needs the cell tree
  stored per cell. Refine and collapse during a run, the trigger and `relief` are not built (planner's design: an
  exact remap of every per-cell array, `relief` for the potential energy of flattened ground, ids by cell serial).
- On coarse globes there are few, very large land cells (23 bodies on the 32-row globe); a move of one link is 625 km.
- The eaters are heavy (about 1 kg per m² of land); the carbon settles near 260 ppm with life (no carbon source).
- The sight gene now costs nothing (it adds no work); selection on it is only by what it shows.

## 5. How to work (keep)

- One Opus agent per package in its own worktree; reuse the agent for its review's fixes and the next package in the
  same code (up to about 700,000 tokens). A fresh adversarial reviewer after each package; every review this round
  found a real problem (a chunk-dependent world, a second rate law, uneven rain at coarse edges, a false anchor).
- At most two heavy agents at once (4 cores, 15 GB). The container restarts often: agents commit small and push only
  finished commits; worktrees survive a restart and an agent can be resumed.
- Gate to `main`: blocking findings fixed; fast suite, clippy, wasm in the main checkout; the slow tests before a large
  merge.
