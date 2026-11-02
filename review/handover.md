# Handover for the next orchestrator (2 November 2026)

For the agent who leads the next round. Simple English. Read in this order: `CLAUDE.md` (one page of rules),
`review/big-look.md` (why we changed direction), `review/top-30.md` (the 30 changes and the budget), this file, then
`docs/laws.md` (how the laws work now). The spec is `docs/handover.md`; where it and the code differ, the code and
`docs/laws.md` are the truth, and the spec gets fixed.

## 1. Where the code stands

- **Branch `claude/ecstatic-hopper-6v12t0`** holds all the work. Its head builds, the normal suite passes (162 tests,
  11 slow ones ignored, about 22 s), clippy is clean, the wasm build works. `main` is the same tree (merged at the
  end of this round).
- **The owner's goal:** the spec's 128 x 128 island at 0.15 s a simulated year or less on 4 cores; a planet of about
  2,000 air nodes in the same time. Measured on the branch head, quiet machine, default life:

| World | Per year (mean) | Flow / Change / Bodies / Parts (ms a year, last printed line) |
|---|---|---|
| 64 x 32 globe, 2,048 cells, 60 years | 629 ms (820 over the last 6) | 317 / 40 / 388 / 57 at year 60, 2,808 bodies |
| 128 x 128 island, 16,384 cells, 30 years | 2,730 ms (4,830 over the last 3) | 1,649 / 257 / 1,435 / 75 at year 30, 17,026 bodies |

  Before this round a settled 128 x 128 year cost about 8 s. So the island is about 2 to 3 times faster, not 50. The
  reason is in section 4: Flow's per-cell overhead, and the number of cells itself. The planet of 2,048 cells runs
  1,000 years in about 12 minutes today; the old code did 1,800 cells in 13.

- **Sizes:** `core/src` 11,319 code lines (there is no line ceiling any more), `worldgen` 1,419, `tape` 1,310, `cli`
  619; docs 39,000 words (were 241,000).

## 2. What was done this round (each package built, adversarially reviewed, fixed)

| Package | What | Review state |
|---|---|---|
| Step 1 (WP1) | Plants are a cover field per cell (`core/src/cover/`): slots per kind, genes, seeds, selection between cells. Animals have a short body (fuel, matter, liquid, two minerals, heat; burn, build, digest). Bodies update once a month. Frost as a ramp, dormancy floor, grazing bulk, one node per kind and stage per cell, caps `MAX_NODES` 8 and `MAX_LINEAGES` 8 | Reviewed, fixed, on main |
| WP2 | The air as an explicit daily loop over the air level; cells exchange once per 10-day sub-step with the step's mean air, each cell one balance with analytic slopes; rain shared bilinearly; blocks even their warmth; each day solved exactly; the long-step test (10-day against 1-day) within 0.3 K | Reviewed three times, fixed, on main |
| WP3 | Bodies stored per cell; one parallel pass over chunks of cells (`Config::chunks`, `ew --chunks`) and one serial pass for cross-cell effects; the outbox and domains gone; the world is the same for any thread count and chunk count (`core/tests/threads.rs`) | Reviewed, fixed |
| WP4 | Rain reaches the ground at the air touching the cell; one world step is one month (three 10-day air sub-steps inside); books at 1e-9 (carries, the potential node and `loosen` gone; one potential number `Store::lifted`); erosion only where liquid runs off, creep on land | Reviewed, fixed, on main |
| WP4b | The potential energy is one function of what the nodes hold (`flow/potential.rs`), booked around each part of Flow; this found and fixed a real leak (falls booked from a false pile of passing runoff) and a lake bug | Reviewed; its three fixes are in (`WP4d fix:` commits) |
| WP4c | The connected sea ("the low basin", `flow/basin.rs`) stands at one level; no pores, sink or sweeps under the sea; the sea's heat one number per air block; one surface balance per air node for its sea cells | Reviewed; fixes in |
| WP4d | Flow speed with the same law and hash: buffers kept, plain loops, the potential energy cheaper, the air touching each cell found once per state. **Partly done**; the last cut (buffers in the cells' exchange) is unfinished: `review/wp4d-exchange-buffers-unfinished.patch` (71 lines, uncommitted work of the cut-off agent; apply it or delete it) | Not reviewed |
| WP5 | Body stores and genes as fixed arrays; brains in a per-cell array (packed when half freed); parts per cell with acts on the pool; the individuals mode deleted; two parents only for a count-1 body of a lineage with 8 or more hidden units; the carbon row re-anchored (258 ppm, with an Earth bound 180 to 420) | Reviewed; the fixes (memory, the cap binding, movers as amounts, born rows kept) are in but **the fixes themselves are not reviewed** |
| WP6 | Signals and the memory of known individuals parked; perception from a per-cell vector of 18 features computed monthly, three smoothed levels for sight 2 to 4, a body sees only its own cell's bodies; brains batched by lineage (one matrix product per lineage per chunk); planning: one step of look-ahead with the critic (`plan_weight`), 193 of 200 seeds move toward food against 30 without | **Not reviewed** (the review was cut off by the weekly rate limit) |
| WP9 | Tape defaults (series yearly, frames every 10 years, one snapshot at the end; `--snapshots-every`, `--pictures-every`, `--months`); one speed line at the end of a run; the suite from 39 to 22 s; the spec cleaned of deleted mechanisms | Light; not reviewed (docs and defaults) |

Dropped after measuring, on purpose (do not redo): the chemistry mask of item 6 (no ground node is ever idle; the
sleep is cheaper); a 30-day air step (it smooths the seasons by 0.5 K; the 10-day sub-step stays); "rain within the
step" for the long-step gap (the cause was the rain's temperature); the eaters' reaction-27 rate (no eater has the
enzyme); "upkeep by dry matter" (it already uses the wet body); moves as a flux, item 14 (the movers as amounts took
its main cost).

## 3. What the next orchestrator should do first

1. **Review WP6 and the WP5 fixes** (commits `021890d` to `ea3b61a` with first lines `WP5 fix:` and `WP6`). A fresh
   Opus reviewer; the brief I sent is in section 7. The open questions: the cap merging across brain sizes (what a
   16-hidden body's rows mean in a 12-hidden node), born rows kept by a node (do the joiners' born rows vanish, and
   does selection on rows still act), perception's 18 features (any outcome label in disguise), batched brains and the
   view a body takes after it perceived, planning's "my state there", memory packing against entries that hold
   offsets, resume equals an unbroken run.
2. **Finish or drop WP4d.** Apply the patch, finish the exchange buffers, keep the hash, measure. Or delete the patch.
3. **Review WP9 lightly** (defaults and the spec edits).
4. Then **push `main`** to the reviewed head. The rule used this round: nothing goes to `main` before its review's
   blocking findings are fixed; `main` was pushed after each reviewed package.

## 4. Where the time goes, and what is left of the 30

Flow is the wall. At 128 x 128 with no life, Flow is about 1,250 ms a year on one thread: the air step about 800
(the air's days 255, the cells' responses 185, book 99, wet 94, dissolving 92), booking the lifts 122, state 89, the
liquid 269. That is about 3 µs per cell per 10-day sub-step against a budget of under 1; the balance itself is a small
part, the rest is per-cell overhead (allocation, loops over all substances, strided gathers). Bodies cost about
115,000 instructions per update (`body::live` 36,500, `copy::copy` 18,900, `choose::act` 4,200 after WP6).

Of the 30, these remain:

- **Item 21, the planet as the air level with refined regions (WP7).** The biggest lever, and the spec's own design
  (7.2). Today a planet of 2,048 cells costs 0.63 s a year. The design: the base level of a planet is the air-node
  grid, each node with its own land, liquid, cover and animal nodes; a region is refined into cells only where brains
  live or the player looks (`Square::refine` exists for the start; make it dynamic, exact in the books). Not started.
- **Items 8, 23 to 26, the cuts (WP8):** one limiting mineral; chemistry 0 to the 14 substances the world uses; two
  materials in life; parts without a reactor (the parts' reactor still passes 1e6 K: the branch `objects` had O2 and
  O3 for it, on the owner's machine if anywhere); park afloat, lever, dig, anchor. Not started.
- **Item 14, moves as a flux:** dropped (see above); revisit only if the serial pass grows.
- **Item 30, the live viewer (WP10):** the core as wasm in a worker, the viewer reading state arrays. Independent of
  everything else; a good package for an agent while reviews run. Not started.
- **Flow speed (WP4d):** the cell pass as flat arrays, the daily loop's three passes to one plus block slopes, the
  downhill pass in parallel by drainage trees. Partly done.
- **Body speed:** `body::live` 36,500 and `copy::copy` 18,900 per update; children who stay should join their young
  node without a `shake` of rows (the reviewer's cut b); Bond's month 24,900 per update equivalent (skip cells with
  no parts).

## 5. Known problems and open choices for the owner

- **The eaters are heavy:** about 1 kg per m² of land (real herbivores 0.005 to 0.02). Cause: the cold-blooded pace at
  290 K (upkeep 4.8 times below a mammal's) and a cover that is mostly digestible. Accepted for now; the ranges row is
  a guard at the current value, not a target. The fix belongs to a life round (warm blood paid for, or plants with
  more frame).
- **The air's carbon settles at about 260 ppm**, not 390: the cover fixes more than the old plants and the world has
  no carbon source (a volcanic input is the same physics as a god action). The ranges row has an Earth bound.
- **The roster's body count** is about 15,800 at year 100 on 64 x 64 with the cap binding (one young and one adult
  node per kind per crowded cell); that is the floor of the cap.
- **An island at 55° or colder freezes over** (no heat from lower latitudes in a box world). Expected physics.
- **The sea has no slope under wind** (one level per connected sea); the eddies even whole blocks of about 300 km a
  day, so an island smaller than a block has no wind at all. Both are written in laws.md as known limits.
- **Change between the layers of a cell books no height** (about 5e11 J a year; the potential function has it now,
  check it stayed).
- **`core` is 11,319 lines** and grew this round (the owner removed the ceiling). Each package added structure
  (chunks, per-cell parts, brains array, the cover). WP7 and WP8 should delete more than they add.
- **The parts' reactor at 1e6 K** still ends walls (an old open item).
- Decisions I took for the owner without asking, as the owner said to: the island references re-anchored on the new
  air (land 289.0 K, swing 12.6 K, rain 1.7 m a year), the carbon row re-anchored, the cap as a speed tool, signals and
  memory parked, `MATE_HIDDEN` 8, planning weight 1, the sea at one level. All are in commit messages and laws.md.

## 6. How the round worked (keep this)

- **Agents.** One Opus agent per package in its own git worktree (`isolation: worktree`), pushing to the branch after
  each step with `git fetch && git rebase origin/<branch>` first; a Sonnet agent for light work (docs, tape, tests).
  Two heavy agents at a time at most on 4 cores; a third makes every timing useless (load 5 to 12). Each agent lasts
  one package plus one or two fix rounds (about 500,000 to 900,000 tokens); then it asks to stop: start a fresh one
  with the last report as notes. Finished agents' worktrees hold 1 to 3 GB of `target/`: remove them
  (`git worktree remove --force`) or the disk fills (it did, twice).
- **Reviews.** After each package, an adversarial Opus reviewer with the commit list, the claims, and a numbered list
  of what to check; it reverts its probes, commits nothing, and reports findings most severe first with file, line,
  evidence and a one-sentence fix, then a verdict. Every review found at least one real bug (an energy leak, a
  guard that made mass, a world that depended on the chunk count, a memory leak, a cap that never bound, a sea mixing
  that never ran). Keep the reviewer across packages in one area: it knows the code; it cost about 100,000 tokens a
  pass.
- **Gate to `main`:** the review's blocking findings fixed, the suite, clippy and wasm green, verified by the
  orchestrator in the main checkout, then `git push origin <head>:main`. Do not cherry-pick around unreviewed
  commits again: it made `main` diverge and needed a merge at the end.
- **Measuring.** `ew run` prints `ms/yr flow, change, bodies, parts, tape` every `--every` years and one speed line at
  the end. Callgrind on a 2-year resume of a 22 x 22 or 64 x 64 snapshot is the steady measure of instructions per
  update; wall time only on a quiet machine (`uptime` load under 2.5), two runs each, before and after.
- **Rules relaxed by the owner this round:** no line ceiling; no golden hash (liveness and range tests instead); the
  world must be the same for a fixed thread count (it is also the same for any thread count and chunk count by
  construction, and the threads test checks it); the books at 1e-9; no history anywhere; re-anchoring a reference
  when a law changes the world is allowed, with the reason in the commit message, never in code comments.
- **What a brief needs:** the goal in one paragraph; where things stand with numbers; the steps in order, each with
  its acceptance numbers; the files the agent owns and the files another agent owns; the rules kept and the rules
  suspended; how to commit, rebase and push; "do not stop to ask: choose the simplest honest physics, write it in
  laws.md, list it in your report"; the report's shape. The briefs of this round are in this session's transcript;
  the shape above is enough to write new ones.

## 7. The review brief that was cut off (for step 1 of section 3)

Review the WP5 fixes and WP6 on the branch head. Check: (1) the cap merging across brain sizes keeps members, stores,
heat, brood and genes, and what the rows mean after a blend of different shapes; the merge at the end of the batch
sees bodies after their moves landed; (2) movers as amounts and born rows kept: do the joiners' born rows vanish, can
born rows still evolve, is a null run's "rows frozen" true; (3) the 18 features: any outcome label in disguise
("edible cover": edible for whom), the cost of sight 2 to 4 now that it is smoothed levels, a predator finding prey in
a neighbour cell only through a mass feature; (4) batched brains: the matrix product's sum order against the threads
test, and the view a body takes after perceiving (no overdraw, no double take); (5) planning: which own-state features
are kept when the critic reads a neighbour's vector, and the anchor of `plan_weight` 1; (6) memory packing against
entries that hold offsets; resume at year 10 of a 20-year 32 x 32 roster equals an unbroken run (same hash); (7)
speed: where the 115,000 instructions per update go and the three cheapest cuts; (8) dead code (`sig`, `known`,
`Faces`, `seen`, `cellf`, `note`, `recall`, `mi_`), laws.md 0, 4, 5 and tape.md current, numbers.md rows, the suite
time. Rules: probes reverted, no commits, no pushes, one run at a time. Report: findings most severe first with file,
line, evidence and a one-sentence fix; what is fine; a verdict.
