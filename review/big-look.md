# A big look at Emergent World (31 October 2026)

For the owner. Simple English. This is my view after reading the whole spec (`docs/handover.md`), the law doc, the
reports of the five rounds after M3, the history, and the code of `core`, and after timing the program myself on this
machine (4 cores). Numbers from the repo say where they come from.

## 1. In short

You are right on both points. **The project is over-built for where it stands, and the speed is far from the goal.**
But the reason is not "too much code". The code is small (10,473 lines in `core`) and clean. The reason is **what the
code computes, and what the work since M3 has been about**:

1. **The detail is at the wrong level.** The world is coarse (cells of 2 km on the island, 670 km on the biggest globe,
   a 10-day air step). But every plant stand and every animal group runs a full organism model: 26 substances, 28
   reactions solved exactly, melting of each substance, pores (stomata), a leaf's boundary layer, sweating by the Lewis
   relation, roots, a self-buckling column for height, a brood, a sample member for selection, young and adult nodes
   joined by a size flux. That is 60% of a year's time, and it is "realism over emergence" (lesson 3 of version 1) in a
   new coat: not field data this time, but textbook biophysics.
2. **Three speed rounds in a row (14 days of 20) gave x3.8 in total, not x10 twice.** Each round found the same thing:
   the bodies are the cost, and every design that made them much cheaper "changed the world by a few per cent" and was
   dropped. The test that kills a speed idea (the key numbers of plants within noise) is a test of sameness, not of
   emergence. With that rule nothing big can ever change.
3. **Nobody has watched the world since M3.** Principle 4 of the spec is "watch first, measure second, polish last".
   Since 17 October every report is about instructions, drift and milliseconds. The last three reports have no picture.
   The repo does not say whether trails, imitation, signals or shelters (the M2 and M3 watch-fors) ever appeared. The
   M3 check-point report "was never in the repository" (`docs/history.md` 3). The parts that exist (about 20 to 60 in
   a world) are "nearly all flesh or fuel" and end by a reactor passing a million kelvin.
4. **The process is heavy again (lesson 6).** 241,000 words of docs for 10,500 lines of code; about 290 numbered
   decisions (S1-S84, X1-X88, L1-L47, O1-O36, the foundation's); measure files of 1,000 to 3,300 lines; a "kill line"
   of 1% of a year for 20 lines of code; callgrind instruction counts as the main measure. Version 1 had 255,000 lines
   of documents. This is the same road.
5. **The thread machinery is a cost, not a gain.** About 2,500 lines of `core` exist so that the world is bit for bit
   the same with 1 or 4 threads (domains, outbox, two-phase batches, views, booking by array, carried rounding). The
   spec asks only for "same seed, same machine, same result" and bans proof machinery. The result: 4 cores give x1.6 to
   x1.8 on a year, and the code of a body update is hard to change.

**What I would do:** stop speed work; do the watch that was skipped (one week); then one decision round with you on the
level of detail of each layer (plants as a field, animals as simple population nodes, brains only where there are
brains); then build Choose (planning, memory) because that is the only road to settlements. Details in section 5.

## 2. Where things stand: the facts

### 2.1 Size

| Part | Lines (code, no comments) | Note |
|---|---|---|
| `core/src` | 10,473 | ceiling 10,673 (L4) |
| `worldgen/src` | 1,392 | |
| `tape/src` | 1,316 | |
| `cli/src` | 584 | |
| `web/src` (TypeScript) | 736 | a tape player: map layers, charts, inspect |
| tests | about 9,000 lines, 212 tests | normal suite 16 s warm (199 pass, 17 slow ones ignored); the slow suite 16 min |
| docs | 241,000 words in 30 files | `measures.md` x3: 5,400 lines; `history.md` 960 lines; plans 2,200 lines |

Where `core` goes: Flow 2,135 (the air alone about 1,300), bodies 1,371, Bond 749, Choose 718, Change 635, Copy 553,
chemistry 333, brain 211, nodes 228. The base and the thread machinery: store 734, life 567, survey 336, space 344,
solve 284, clock 108, outbox 72, par 73: about 2,500.

### 2.2 Speed, measured today on this machine (4 cores, release build)

Runs I made today (`ew run`, default flags, seed 1; "flow" is the air, liquid and land; bodies are plants and animals):

| World | Cells | Years run | ms per year | of which flow / change / bodies | Bodies at the end |
|---|---|---|---|---|---|
| island 22 x 22 | 484 | 50 | 128 | 60 / 16 / 57 | 877 (741 plant nodes, 130 animal nodes) |
| island 22 x 22, no life | 484 | 30 | 67 | 54 / 11 / 3 | 0 |
| island 22 x 22, one thread | 484 | 30 | 160 | 71 / 22 / 95 | 747 |
| globe 36 x 18 | 648 | 50 | 144 | 80 / 17 / 54 | 713 |
| island 64 x 64 | 4,096 | 20 | 798 | 472 / 98 / 284 | 5,937 (plants only: animals come at year 22) |
| island 128 x 128 (the spec's island) | 16,384 | 6 | 3,062 | 2,105 / 322 / 933 | 20,473 (plants only) |

A young world is cheap; a settled one costs about twice as much (the lighter report: 250 ms a year on the 22 x 22
island at year 300, where animals are 39% of the time). Four cores against one: x1.25 here (young world), x1.6 to
x1.8 settled (the reports). The books hold in every run (element drift 3e-16, energy 1e-14). The thread count did
not change the world (the same hash with 1 and 4 threads).

Two things the table says plainly:

- **The physics alone misses the spec's target at the spec's size.** On the 128 x 128 island, Flow and Change with no
  animal cost 2.4 s a year: 1,000 years take 40 minutes before a single animal exists. The target was 10 minutes
  for the whole world.
- **The cost per cell is what it is.** 0.13 ms per cell-year for the physics, about 0.4 ms per cell-year for the bodies
  of a settled world (lighter report). At that price the 15-minute planet is 1,800 cells, each about 670 km across:
  three cells for all of France. No town can be seen on that map.

What the reports say about where a year goes (one thread, island 22 x 22, `docs/reports/lighter.md`): animal updates
39%, plant stand updates 22%, the air 16%, Change 8%, survey 5%, landing 4%, liquid 3%, tape 2%. One animal update
costs about 130,000 instructions, one plant stand update about 81,000. A plant stand updates every 40 days; one update
in two also runs a second time for a "sample member" (`sample_every` 2), so selection inside a node costs about half
a stand's time again.

The speed history (4 cores, a settled year of the 22 x 22 island): before the speed work 0.94 s; after round 1 0.66 s;
after round 2 0.26 s; after round 3 0.25 s. Two rounds aimed at x10 and wrote "we did not reach 10 times". Over three
rounds: x3.8.

The spec's own target (12.6): the M1-M3 island of 128 x 128 cells, 1,000 years in 10 minutes, so 0.6 s a year. The
project changed the target to "the planet has as many cells as fit the time" (5.4). That is honest, but it hides the
gap: a settled 128 x 128 island would cost about 8 s a year on this machine (2.4 s of physics, measured today, plus
about 0.4 ms per cell-year for bodies, from the lighter report), so 1,000 years take about 2.3 hours against the target
of 10 minutes. The gap is about x14, after three speed rounds.

### 2.3 Time steps

Air, radiation, rain, wind, reactions of ground and air: every 10 days. Liquid: every 60 days. Plant stands: every 40
days. Animals: their gene, 10 to 360 days, in batches of 10 days; a node thinks every 20 days. Parts: loads monthly,
reactions every 6 months. Ground heat sideways and creep: yearly. **These steps are already as long as the physics
allows.** Speed cannot come from longer steps any more. It can only come from fewer nodes, or a cheaper node, or
dropping the bit-for-bit thread rule.

### 2.4 What has emerged (what the repo can say)

- Climate: land at 291 to 295 K for 1,000 years, seasons, rain, rivers in about 700 cells of a 64 x 64 island, erosion
  at real rates. The carbon gas falls in some seeds (no source). Good.
- Life: light-eaters and eaters live for 1,000 years, populations steady. Eaters live "almost only on dead plant
  matter". Light-eaters are nodes without a brain.
- M2 and M3 watch-fors (trails, seasonal movement, imitation that beats the null run, signals, tools, shelters,
  containers, floating things): **no report in the repo says yes or no.** The tape has the columns (`gave`, `watched`,
  `worn_*`, `structures`, `enclosed`), so the answer can be read, but nobody has written it down since the M3 check
  point, and that report is not in the repo.
- Parts: about 22 on the globe, 61 on the 64 x 64 island, nearly all made of flesh or fuel, ended by a reactor bug
  (1e6 K). So Bond v1 exists in code and does not yet do anything in the world.
- Nothing people-like, no planning (6.5, "not built yet"), no tool, no shelter with an effect. The spec says this is
  expected before the "intelligence step" (option 2 of 13.1). That step has not started.

### 2.5 Things that are broken or risky in the repo itself

- **The git history was cut.** `docs/history.md` points to about 100 commits (`154554a`, `3b98e84`, patches "kept in
  git") that do not exist in this clone: the oldest commit here is from 26 October. If your machine has the old
  history, keep it safe; if not, "git keeps them" (CLAUDE.md 2.4) is no longer true, and the M0-M3 prototypes and
  studies are gone.
- **Known law problems left open** (`docs/reports/lighter.md`, "Open items"): the parts' reactor at 1e6 K (walls end by
  a bug), plants stiffen with lime because chemistry 0 prices lime under frame, S50 (plants store too much), small
  nodes weaker than individuals, living bodies that rot, the carbon cycle with no source, the methane-like gas's life.
  Each has a branch. None is decided.
- **The threads give little.** 4 cores: x1.6 to x1.8 on a year (`docs/handover.md` 7.4). The air does not parallelise;
  the serial phases of a batch, the landing order and the parts stay on one thread.

## 3. What I think is wrong, point by point

### 3.1 The level of detail does not match the scale (the main point)

The spec's picture (7, "rough far away, detailed where it matters") is right. The build is the opposite: it is
detailed everywhere, in the bodies, and the only speed tool is "merge alike bodies into a node". A node still runs the
whole organism model. On the globe of 60 x 30 a cell is 670 km across; the "plant stand" in it runs stomata, a 5 cm
leaf boundary layer and Greenhill's buckling column. None of that can change whether a town appears.

The spec itself says what matters for emergence of the big things (6.6, 13, 15): Choose (planning, memory,
imitation), Bond (things you can hold, join, enter), seasons and places that differ. Plant physiology is not on that
list. Today Choose is the smallest law folder (718 lines) and has no planning; bodies and Flow are 3,500.

### 3.2 The sameness test blocks every big saving

Each speed round judged a design by "the key numbers (plant mass, eater mass, lives) stay within noise, the golden hash
is re-pinned, the nodes still match individuals". That is the right test for a pure clean-up. It is the wrong test for
a change of level. "Plants as a cover" was dropped because plants came out 1.0 decade lighter in copy mass and kinds
spread slower; the "coarser air" because rain on land rose 6 to 12%. These are not failures for a game about
emergence; they are a different world, as valid as the first. With this rule the project can only shave per cents.
The three rounds show it: x1.5 to x1.9 each from compiler flags, hot spots and threads, never from a new level.

### 3.3 The bit-for-bit thread rule is proof machinery in all but name

The spec (12.5) wants same seed, same machine, same result, and says do not build more. Decision X3 added "the number
of threads does not change the world". To get it, the code has domains, an outbox with landing in data order, two
phases per batch, views of the store per domain, booking by array in domain order, provisional ids, carried rounding
(Neumaier). It is about a quarter of `core`, it is the hardest code to read, and it is why 4 cores give x1.7. A rule
of "same result for a fixed thread count" would keep every debugging use and let the parallel code be plain.

### 3.4 The process is heavier than the product

Numbers: docs 241,000 words against 10,500 code lines (23 words per line of code); about 290 numbered decisions in
three weeks; plans of 300 to 1,000 lines per round; measure logs of thousands of lines in the repo (CLAUDE.md 4.5 says
"no study files in the repository"); a kill line that needs callgrind runs for a 20-line change; every step reviewed
twice "adversarially". A new agent must read about 60,000 words before touching code (CLAUDE.md says so). Version 1
died of exactly this (2, lesson 6).

The docs that matter and are good: `handover.md` (the spec), `laws.md`, `numbers.md`, `tape.md`, the one-line
`history.md`. The rest (measures, decisions, plans of finished rounds, 4 of 7 reports) are history and could live on
a branch or in an archive folder that no one must read.

### 3.5 The line budget shapes the code, not the design

`core` is held under a ceiling (10,673) counted by a test. The effect: dense lines (607 lines over 120 characters, 14
over 200), functions that return 5-tuples, and the words "within the line budget" in design notes. A budget on lines
pushes toward clever code, not toward less code. The real lever on size is 3.1 (fewer things computed), not line
width. The spec meant the budget as a warning sign ("3x over means something is wrong"), not as a daily constraint.

### 3.6 Things that are computed that do not need to be (concrete list)

Each of these is good physics. Each costs time at a level where it cannot change the outcome we want:

- The exact integral of 28 reactions with product inhibition in every body at every update (then skipped by
  `TOO_SLOW` for about half).
- The melting plateau of every one of 26 substances in every body (`Body::thermal` at several points of one update).
- Evaporation through pores with the Lewis relation, a leaf boundary layer and a dew-point stop, per body.
- A second full update of a sample member in half of all plant updates (selection by Price's equation inside nodes).
- Young and adult nodes per kind per cell, joined by a McKendrick-von Foerster flux.
- The partner search for two parents in every mature individual's update.
- Newton rounds with a banded solve of the whole air level every 10 days for 36 air nodes (island 22), with 4 grey
  layers and clouds; good climate, but 16 to 24% of a year.
- Memory of 4 known individuals, signals and watching for animals whose brains have 0 to 16 hidden units and no
  planning: these are paid for and cannot yet produce anything (the null run would say).
- The parts' reactions every 6 months at a million kelvin.

## 4. What is good (keep it)

- The spec. Sections 1 to 6 and 13 to 15 of `handover.md` are a clear, honest design with the right lessons.
- The discipline of the code: no magic numbers, one row per number with its anchor, one place per idea, the word ban,
  the books that close, the one clock, tests that are few and fast. These are rare and worth keeping.
- The climate and the land: a cheap, stable, Earth-like climate with seasons, rain, rivers and erosion at real rates,
  for 1,000 years, that runs in minutes. This is M0 done well.
- The tape and the viewer: a run leaves a readable record and a map you can scrub.
- The honesty of the reports: "we did not reach 10 times" is written at the top, twice.

## 5. What I would do next

### Step 0 (one week): the watch that was skipped

Run the 64 x 64 island with the roster for 300 years, with and without `--null`. Read the tape and the frames and
write the M3 check-point report the spec asks for (13.1): which M0-M3 watch-fors appeared, with pictures; the three
biggest things that did not emerge and the best guess why. Do not change code. This report decides everything below,
and it should exist before any more engineering.

### Step 1 (one decision round with you): the level of each layer

Decide, per layer, the smallest model that keeps what emergence needs. My proposal:

| Layer | Today | Proposal |
|---|---|---|
| Air, radiation, rain, wind | implicit step, 4 grey layers, clouds, Newton on the air level | keep; it is 20% of a year and works |
| Liquid, erosion | downhill, Darcy, spill levels | keep |
| Plants | a body node per stand, full organism model, selection by sample members | **a cover field per cell**: a few kinds, each with mass per m² and 5 to 8 genes (light use, water need, frost limit, height, hardness, seed spread). Growth from light, water and the minerals of the cell; death by frost, drought and grazing; genes drift by local selection on the field. A plant cell costs hundreds of operations, not 81,000 per stand. Chemistry 0 still gives what plants are made of (sugar, frame, lime) so animals can eat it and fire can burn it |
| Animals without a brain | nodes with the full organism model | one population node per kind per cell, with a short body model: fuel, body matter, water, temperature; 3 reactions (burn fuel, build body, digest); Kleiber's upkeep; birth from surplus; the same genes as now but far fewer stages |
| Animals with a brain | same as above plus the brain | keep the brain and perception, cut the body model as above; add planning (6.5) and a longer memory. This is where the detail should go |
| Parts and Bond | reactor at 1e6 K, 20 to 60 parts per world | finish the `objects` round's O2 and O3 only (walls end by their strength), then stop until a kind exists that wants parts |

The sameness test for this change is **not** the key numbers of today's plants. It is the spec's own test: does the
new world still show the M0-M3 watch-fors (from step 0), and is a year now cheap enough for the 128 x 128 island
(0.6 s)? A different plant mass is fine.

### Step 2: relax the two rules that cost the most

- **Threads:** keep "same seed, same machine, same thread count, same result". Drop "any thread count". Then the
  outbox, the two phases and the booking by array can become one parallel pass with per-thread accumulators. Expect
  the body code to lose 1,000 to 2,000 lines and the cores to give nearer x3.
- **The books:** keep conservation and the yearly check, at 1e-9 relative instead of 1e-12. Drop the carried rounding
  and the potential node if they then do nothing. 1e-9 still catches every real leak (a leak is 1e-3 or worse).

### Step 3: the docs diet

- Move `docs/speed-post-foundation/`, `docs/speed-x10/`, `docs/lighter/`, `docs/foundation/` and the four
  foundation/speed reports to `docs/archive/` (or a branch), with one page that says what each round did and found.
  Keep the spec, `laws.md`, `numbers.md`, `tape.md`, `history.md`, the last report.
- Cut CLAUDE.md's reading list to: the spec's sections 3, 6, 14; `laws.md`; the last report. One page of rules.
- Replace the kill lines, the callgrind rule and the two adversarial reviews per step by one rule: a change is measured
  by a 1,000-year run on the reference world (time, drift, and the watch-for list), once.
- Fix `history.md` so it does not point at commits that are not there, or restore the old history.

### Step 4: build toward the game

Only after 0 to 2. In the spec's order: planning and longer memory in Choose (the research risk, 15), a kind with a
brain that is paid for by something (a season to store for, a shelter that saves heat), Bond that does something, the
pattern finder for the few patterns that exist, the Earth view. The viewer that exists is enough for watching until
then.

## 6. The honest bottom line

- The code is good craft on a plan that drifted. The plan drifted at the M3 check point: the spec says "stop, watch,
  decide", the project did "foundation, then speed, then speed, then lighter", with no watch report in the repo.
- Speed is not a code problem; it is a level-of-detail problem. Three rounds of code-level speed work proved it: x3.8,
  and every change of level was blocked by a sameness test.
- The game is not nearer than it was on 17 October. The climate and land are done. Life is alive. Everything after
  "animals" is still ahead, and the one law it needs (Choose with planning) has had the least work.
- None of this is lost work: the climate, the land, the chemistry, the tape, the viewer, the test kit and the spec
  carry over whole. The body model can be made small without touching them.
