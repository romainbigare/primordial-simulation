# Lighter world: the plan

Status: **restated after phase P** (the probes R0, PR1, PR3, PR6, PR7, PR8, C0, PA; `measures.md`). Branch
`speed/lighter`: lane F and G1 merged (`feceae7`); B1 (the skip) under its review's fixes on `lighter/b-skip`. This file
is the one source of the steps: each row has its files, check level, done-when and kill line. The designs are in
`design/`; decisions in `decisions.md` (L1 and on; this restatement is L9, L12 to L14, L20, L35 to L39); measures in
`measures.md`; history in `docs/history.md`, section 7.

## 1. The goal, and where the year goes

**The goal:** compute less. The owner said yes to all seven ideas, the law changes too (L3); several plant kinds per
cell that do not mix are a must (L17); idea 2 waits for the cover (L18). What must stay is the goal of the game
(handover 1, 6, 13): animals with brains, trails, later people, settlements and trade; a climate with seasons, rain and
rivers; plants that are food, cover and fuel and that adapt to their places.

**Where the year goes** (R0 and PA on `benchZ`, one thread, share of `run_for`; island22 / globe36 / region24):

| Part | island22 | globe36 | region24 |
|---|---|---|---|
| **animal updates** | **40.2%** | **32.1%** | **42.7%** |
| of it: the body's day (physics, upkeep, growth) | 16.6% | 12.8% | 17.3% |
| of it: children (`copy::copy`) / maturing (`copy::young::mature`) | 7.3% / 4.0% | 4.8% / 2.7% | 7.8% / 4.1% |
| of it: the decision / the habit | 9.0% / 2.3% | 9.5% / 2.0% | 9.7% / 2.8% |
| landing of the animals' moves and litters | 4.9% | 1.3% | 4.2% |
| **stand updates** | **21.7%** | **18.5%** | **24.1%** |
| the air | 13.4% | 20.6% | 11.0% |
| Change / the liquid / the survey | 7.8 / 2.6 / 5.3% | 7.8 / 3.5 / 5.0% | 6.1 / 2.0 / 5.5% |
| parts (Bond's month; before G1) | 0 | 5.4% | 0.2% |

**Animals are now the largest part** (L37), and inside them the bookkeeping of nodes (children, maturing, the
landing, much of it `nodes::add` at about 21 thousand instructions a call) is 16% / 9% / 16% of the year, more than the
decision and the habit together. Stands are 19% to 24%, all included about 27% / 24% / 30% (C0). Every animal updates
every batch (its `INTERVAL` gene at or under 10 days, PA).

**What the round can give** (one thread, instructions of a settled year; estimates from the probes; "x" is faster by):

| Step | island22 | globe36 | region24 | Status |
|---|---|---|---|---|
| F the saves (long runs) | 0 | 0 | 0 | done: long runs about -6% (F3, estimate) and the main thread's saves 2.4% to 0.4% (F2, counted) |
| B1 the skip | -2.0% | -0.9% | -2.1% | building (`lighter/b-skip`, the review's fixes) |
| G1 the parts' search start | 0 | -1.2% | -0.1% | done |
| G2 the parts react as a ground node | 0 | about -3.5% | about 0 | design, review, step |
| H1 children in lumps | about -5.8% | about -3.8% | about -6.5% | design (`design/animals.md`), then the step |
| H2 maturing in lumps | about -2.6% | about -1.8% | about -2.7% | after H1 |
| H4 a cheaper join | up to -2 to -3% | up to -1 to -2% | up to -2 to -3% | probe first |
| C the cover: C1a and C1 / with C2 | -7.5% / -11.5% | -6.5% / -9% | -8% / -13% | C0 done; `design/cover.md` restated (counted estimates); C2 built only if C1 cuts at least 6.5% (island22) and 6% (region24), kept at -10% (L35) |

| Case | island22 | globe36 | region24 | long runs |
|---|---|---|---|---|
| without the cover (B1, G, H1, H2) | about x1.11 to x1.12 | about x1.12 to x1.13 | about x1.12 to x1.13 | about 6 to 8% more |
| with C1a and C1 | about x1.20 to x1.22 | about x1.19 to x1.21 | about x1.22 to x1.24 | the same |
| **with C1 and C2** | **about x1.26 to x1.28** | **about x1.23 to x1.25** | **about x1.30 to x1.32** | the same |

(The cases add the steps' shares of today's year; each step's own measure restates them.) **Plainly:** the cover is no
longer the whole round: on `benchZ` the stands are a fifth to a quarter of the year, and the
animals two fifths. The round's largest sure gain is in the animals' bookkeeping (lane H), the parts' reactor on the
globe (G) and the long runs' saves (F, done). Built and not built so far: section 3.

## 2. Rules for this round

All of `CLAUDE.md` and of the round's brief (kept outside the repository, beside the scratch files) holds.

- **Check levels:** 1 bit for bit; 2 the same to rounding; 3 the same world (key numbers, pace and batch tests, the
  nodes check when a step touches nodes or minds); **4 a new law for a part of the world**: each level-4 row says what
  must still hold and which tests change. Never loosen a tolerance; a test of a deleted mechanism is deleted; a test of
  a changed law is rewritten to state the new law.
- **Kill lines and worth** (L2): a kill line asks for at most 10% of the year (for lane F, of a long run's year). A
  level-1 or level-2 step of 20 core lines or less is kept at 1% of the year or more, of 21 to 40 lines at 1.5%; a law
  step (level 3 or 4) needs about 3% on its target worlds. A step under its kill line is reverted and kept on a branch
  `lighter/<id>-not-kept`, and its alternative (section 3) starts.
- **References** (L10): the start's key numbers are those of `docs/speed-x10/measures.md` "Z: the final checks" 1.
  After each level-4 law lands, the key-number test runs once more and its values become the reference. Every level-3
  or level-4 step rebased on a moved `speed/lighter` runs the key numbers again before it merges. S50 and S84 stay the
  known failures. The long-step test judges the mean of seeds 1 to 6, its 0.3 K limit unchanged (L38).
- **How a gain is judged** (L11): 1T callgrind instructions of a year; 4T wall time under `flock $L/timing.lock`, 5 or
  more interleaved runs, medians, the load said; a step that changes the saved world is judged on one world (a turned
  save) or by counted work on remade benches; across remade worlds only large cuts count (PR-S). **Benches:** the step
  that bumps `SNAPSHOT_VERSION` remakes the three benches with its own build (`make_benches.sh`) at its merge and names
  the folder in `measures.md`; later steps use the newest set (`benchZ` until the first bump). Long runs count wall
  time (L8).
- **Agents.** Law steps and every adversarial review go to the strongest agents; normal code to normal agents;
  measures to measuring agents. Each step in its own worktree and branch; the architect merges one at a time;
  `SNAPSHOT_VERSION` goes to the highest plus one at each merge.
- **The three rules for smart bodies hold:** individuals stay real; the step per place; two kinds of body code (code
  paths, not two physics). **No calendar rule:** a step may do its work at most once in a window of days (the world's
  clock step), never at a season or a date shared by every body (L39).
- **Core lines** (L4, L31): the ceiling is 10,643 (10,444 at the start, R0). The step that would pass it raises
  `CEILING` in `cli/tests/core_size.rs`, `CLAUDE.md` 2.11 and handover 14.3 in the same commit, by the lines it needs,
  with a decision logged; phase Z sets the round's ceiling.

## 3. The owner's seven ideas: what is built, what is not, and the alternatives

| # | Idea | Status | Alternative taken |
|---|---|---|---|
| 1 | Cheaper saves | **done** (F1, F3, F2; L13): the snapshot and the pictures timed, the tape's files written cheaper, the saves on a writer thread | |
| 2 | Fewer stands a cell | not built before the cover (L18) | the cover's bound on kinds (C1); A3 only if the cover fails |
| 3 | Living bodies do not rot | **the law is not built this round** (PR3: every form makes the world heavier, eaters x35, updates +38%): an open item for the owner (L9) | **B1, the skip**: a bare reaction in a body too small to matter over a year is skipped (-2.0 / -0.9 / -2.1%) |
| 4 | A cheaper sample member | not built before the cover (L15) | A4 only if C2 is not built and its gate holds |
| 5 | Plants as a cover | **restated from C0** (L35, L40): C1a (`crowd` counts bodies with a brain only), C1 (the bound on kinds and the newcomer's clock) kept at 3% of the year; C2 built only if C1 cuts at least 6.5% (island22) and 6% (region24), kept only if C1 + C2 reach -10% on both with the animals unchanged | A4 (the sample's stages) or nothing |
| 6 | Animals look around less | **D1, D2 not built** (PR6: perception is 0.6% to 0.9% of the year, the take's payer 0.6%) (L14) | **lane H**: the animal update by stage (PA): children and maturing in lumps, a cheaper join |
| 7 | A coarser air on globes | **E1 not built** (PR7: -4.7% of globe36 but the land 6% to 12% wetter on every seed) (L12) | **lane G**: G1 (done) and G2, the parts' reactor |

## 4. The steps

"1T": callgrind instructions of a year on one thread; "4T": wall time on 4 threads. "Counted": 1T on remade benches,
seeds 1 to 3, the parts the step names.

### Phase P (done)

R0, PR1, PR3, PR6, PR7, PR8, C0, PA: the key tables and verdicts are in `measures.md`; the full tables stay on the
probe branches (`probe/r0`, `probe/pr3`, `probe/pr7`, `probe/c0`, `probe/pa`). PR2 and PR4 were not run (L22).

### Lane F: the saves (done)

| ID | What | Level | Status |
|---|---|---|---|
| F1 | `save_snapshot` and `render::save` timed in the tape's column (the next year's row) | 1 | done |
| F3 | `series.csv` and `months.csv` appended, the viewer's file at gzip `fast` | 1 | done: a 1,000-year island run's writes -19.3 s, about -5.8% (estimate) |
| F2 | The compression and the write on a writer thread | 1 | done: judged on the count, the main thread's saves 2.4% to 0.4% of a 300-year run (L13); a quiet wall pair in phase Z |

### Lane B: the skip (`core/src/change/kernel.rs`; `design/no-rot.md`)

| ID | What | Level | Done when | Kill line | Status |
|---|---|---|---|---|---|
| B1 | **The skip:** in a body's reactor, a bare reaction (factor at most 1) whose rate over a year would use under `TINY` of its scarcest input (a named speed const, no `numbers.md` row) is skipped; counted over the world's year (`kap * year`), so it does not depend on the call's length (L9, L34) | 2 or 3 | golden re-pinned; the key numbers within their seeds' noise; the long-step test on the mean of seeds 1 to 3 (L38); the year 1T -2% on island22 and region24 | the year 1T under -1% on island22 | building (`lighter/b-skip`, the review's fixes) |

**The law "living bodies do not rot" is an open item for the owner** (L9): its best form is (b0) (a decomposer reaction
runs in a living body only by its own catalyst; about +15 lines; no data change); it changes the eaters x35 and the
updates +38%, so it belongs to a round about life, with new eater references.

### Lane G: the parts' reactor (`core/src/bond/month.rs`, `core/src/change/kernel.rs`; `design/parts.md`)

| ID | What | Level | Done when | Kill line | Status |
|---|---|---|---|---|---|
| G1 | The halving search starts at `ceil(log2(heat / bound))` | 1 | golden unchanged; globe36 1T -1.18% | under 1% (L2) | **done** (`feceae7`) |
| G2 | **A law fix:** a part reacts as Change runs a ground node: its own share of the air, with the exchange of heat with its cell, so its temperature stays physical (today it passes 1e6 K inside every call, PR8). Its own commit with the physics reason; `laws.md` 3 | 3 | a short design and its review first; globe36 1T -3% (counted); the parts' walls' life (the share left after a year, by cell moisture) within its seeds' noise; the gas turnover a call; key numbers; the books | globe36 1T under -2% | design, review, step |

### Lane H: animals, the largest part (`core/src/copy/`, `core/src/nodes.rs`, `core/src/body/`; `design/animals.md`)

| ID | What | Files | Level | Done when | Kill line |
|---|---|---|---|---|---|
| H1 | **Children in lumps:** a node's brood fills at every update as today; it makes children at most once in a window of `Config::lump_days` 30 of its own phase (from its id, so no world-wide pulse): at the update where the window of its day differs from that of the update before it (no new state). The snapshot is bumped (a `Config` field). `vary`, the children's brain rows and the join then run once per lump. Individuals and stands as today. `laws.md` 4.2; `numbers.md` `lump_days` (a speed number) | `copy/mod.rs`, `params.rs`, `tape/src/snapshot.rs`, tests, docs | 4 | what must hold, below; counted: the year 1T -3% on island22 and region24 (estimate -5.8 / -6.5%) | under -3% on island22 (L2) |
| H2 | **Maturing in lumps:** a young node's McKendrick flux runs at most once in its window, over the growth since its last flux (the log10 growth summed over the updates since: one `Body` field; snapshot bumped; benches remade) | `copy/young.rs`, `body/mod.rs`, `survey.rs`, `tape/src/snapshot.rs`, tests, docs | 4 | what must hold, below; counted: the year 1T -2% on top of H1 (estimate -2.6 / -2.7%) | under -1.5% on island22 (a step of 21 to 40 lines, L2) |
| H4 | **Probe first:** a cheaper `nodes::add` for joiners of the same lineage (the group that moved, the part that matured): which share of the joins have genes equal or within rounding of the node's, and what the per-gene pooling, `Body::prepare`, the rows' blend and `see` cost of a join's 21 thousand; then a step at level 1 or 2 if it is worth L2's line | `nodes.rs` | probe, then 1 or 2 | the probe's table; a step only at 1% of the year (20 lines) or 1.5% (40) | as L2 |

**H1 and H2 must still hold:** the key numbers, the eaters within their range (5.354 ± 0.5; life 10.47 yr x/ 2); the
nodes check (nodes against individuals; individuals keep making children at every update); the pace and batch tests
(seeds 1 to 3); `the_eaters_live_and_stop_growing`; `a_century_keeps_its_books` and the books of every lump (the brood
leaves with dead members, `Body::lose`, as today); births a year and the young node's members within their seeds'
noise; watch-for M1 and M2 (populations rise, fall and cycle; trails; imitation). **Tests that change:** the copy tests
that make children in one update (`a_mature_nodes_children_gather_in_its_young_node`,
`reproducing_updates_keep_elements_and_energy`) state the window; `young_members_mature_at_the_rate_their_growth_gives`
and `any_spread_of_sizes_matures_a_share_of_the_young` state the flux over the window (H2); a new test: the births of a
node over 90 days are the same in lumps as at every update, within the brood held.

**H3 (`think_days` 40) is not built** (L39): X72 measured that 30 days cost the young world's eaters -0.32 decades,
and the animals' choices are the story. Its alternative is H4.

### Lane C: plants as a cover (`design/cover.md` 7; L17, L23 to L28, L35, L36, L40)

C0 is done (`measures.md` C0). "Counted" = callgrind on one thread, one year from the remade benches, seeds 1 to 3, with
`--toggle-collect` on the update of a body without a brain: the year, the plant work per planted cell-year, the planted
cells, the animal updates a year, bodies per planted cell, `outbox::deliver`. A number in brackets is a section of
`cover.md`; tests 1 to 3 are its 6.3. The order: C1a, C1, then C2 only at its entry condition, C3, G-C; after lane H in
`copy/` (L35).

| ID | What | Files | Level | Done when | Kill line |
|---|---|---|---|---|---|
| C1a | **`crowd` counts bodies with a brain only, set to 1** (3): `nodes::crowd` and `nodes::split` count only bodies with a brain; `numbers.md`'s `crowd` row (its value and anchor: what planted cells do today); `laws.md` 4.4; the nodes check's noise settings (handover 7.1: `--crowd 2` and `4`); snapshot (the meaning of a saved field changes) | `nodes.rs`, `params.rs`, `tape/src/snapshot.rs`, `tape/tests/nodes.rs`, docs | 4 (animals) | animal nodes per planted cell, animal updates a year and animal merges a year within their seeds' noise of today (counted, 3 seeds); the eaters' key numbers; the nodes check holds its rules | those move past their seeds' noise: option (a) stays and C1 and C2 are gated on the animals as they are |
| C1 | **The bound on kinds and the newcomer's clock** on today's young-adult pairs (3): `Config::cell_kinds` = 3 (row in `numbers.md`, speed); a new kind starts in `emit` and `land_litter` only while the newcomer place is free, else the litter joins the most alike; `nodes::split` of a light-eater node needs two free places; at a month's start `nodes::crowd` merges the most alike pair of a cell holding more than `cell_kinds` kinds with adults (no `alike` limit for bodies without a brain), and a newcomer older than its genes' life span joins its most alike kind (`Body::life_span`, from the genes); snapshot; test 1 (two invasions) and the rewritten `crowd` and `dispersing` tests; `laws.md` 4.2, 4.4; `history.md` | `params.rs`, `nodes.rs`, `copy/mod.rs`, `body/tissue.rs`, `tape/src/snapshot.rs`, tests, docs | 4 | at every month's start, after `nodes::crowd`, at most `2 (cell_kinds + 1)` = 8 bodies without a brain in every cell (the mean per planted cell measured apart); the counted year at least -3% on island22 and region24 (estimate -7.5% and -8%); test 1; animals within their seeds' noise; key numbers in range (S50 within noise); the climate and life tests of 6; kinds by climate kept (the spread of the form genes' cell means across globe36's land at least half of C0's) | **kept only if** the counted year falls at least 3% on island22 and region24 (L2, decision 37) with all its law checks; else C stops (alternative: A4 or nothing) |
| C2 | **The young inside** (2), **built only if C1's counted cut is at least 6.5% on island22 and 6% on region24** (5): `Body::young` and `Young`; `adult`, `size_moments`, `set_size`, `height`, `equiv` by stage; two shade entries; growth by each stage's balance; maturing in `live` before children; children join the cover; seeds as young; `starved` per stage, the brood with the adults; `nodes::add`, `rejoin`, `Lot`, `part`, `kin`, `kin_key` without stage; founders and bodies that lose their brain all adult (`held`); the clock reads `Young::held`; snapshot; tape `root_<K>_adults`; tests 2 and 3 and the rewritten ones; `laws.md` 1.4, 3.6, 4.2, 4.4, 6.6; `tape.md`; `history.md` | `body/mod.rs`, `body/tissue.rs`, `body/live.rs`, `copy/grow.rs`, `copy/mod.rs`, `copy/young.rs`, `nodes.rs`, `survey.rs`, `life/domain.rs`, `lineage.rs`, `worldgen/src/life/make.rs`, `tape/src/snapshot.rs`, `tape/src/rec/cols/`, tests, docs | 4 | at every month's start at most `cell_kinds` + 1 = 4 bodies without a brain in every cell; **C1 + C2 at least -10% of the year counted on island22 and region24** (decision 37; estimate -11.5% and -13%) with animal nodes and animal updates within their seeds' noise of today; light-eater members per planted cell within a factor of 2 of C1's; test 2; key numbers, kinds by climate and tests as C1; the land within 0.3 K of C1 | C1 + C2 under -10% on island22 or region24, or the animal, key-number, climate or kinds lines fail: C2 is not merged, C1 stays |
| C3 | **The checks** (no code): counted runs of the three benches, plants all included; wall on 4 threads (5 interleaved runs under the lock); key numbers; the slow suite; 1,000-year runs of the island, the globe and the region, one at a time, with the watch list of 6 | `docs/lighter/measures.md` | — | the measure and a verdict per bench | — |
| **G-C** | **The cover's gate** (decision 37): C1 at least -3% of the year counted on island22 and region24 with its law checks; with C2, C1 + C2 at least -10% on both, globe36 not worse; animal nodes per planted cell and animal updates a year within their seeds' noise of today; every key number in range (S50 within noise), the climate tests and the canopy check pass; the 1,000-year runs keep their books and live; tests 1 to 3 pass | | | | **fails:** what failed is not merged (C1 stays if it passed its own lines); A4 or nothing |

### Lanes not built

| Lane | Why | Alternative |
|---|---|---|
| D (D1, D2) | PR6: perception 0.55 to 0.91% of the year, the take's payer 0.6%, the per-substance repeats in the take 0.6% (L14) | lane H |
| E (E1) | PR7: -4.7% of globe36, -3.8 to -4.2% of the size-28 globe, but rain on land +6% to +12% on every seed and world, 13 to 20 times its year-to-year spread (L12); code on `probe/pr7` | lane G (G2) |
| A (A3, A4) | phase 2 only (L6, L18) | A4 if C2 is not built and it is worth L2's line |

### Phase 2 (after the gate)

| ID | What | Level | Done when | Kill line |
|---|---|---|---|---|
| A4 | Only if C2 is not built: the sample runs only the stages its gene reads, the node's other results reused | 2 | a stand update 1T -6%; the year 1T -1.5% | under -1.5% of the year (L2) |
| C4 | After G-C if C2 is built: a probe of the cost of making a cover's children (L30) | probe | its table | none |

### Phase Z: the end (unchanged)

1. The year at the round's end against its start: `$L/ew-base` on `benchZ` and the end's build on benches made with
   its own code (`make_benches.sh`), 6 interleaved sets, 4 threads and 1 thread, under the lock; medians. And 1T
   instructions of a year on both.
2. 1,000-year runs (seed 1; island 22, globe 18, region 24 with `--region 8,8,8,8`), one at a time, wall time, the
   default pictures and saves, for both `$L/ew-base` and the end's build (L21); the quiet wall pair of F2 (L13).
3. The largest globe that runs 1,000 years in 15 minutes of wall time (start at size 28, then up).
4. The slow suite, the golden test without threads, the debug checks.
5. The report `docs/reports/lighter.md`, with the owner's open items (the law of living bodies, L9).
6. The docs at the round's end: the `CLAUDE.md` row "Lighter world (done)"; handover 7.1, 7.3, 13.2, 14.3 (the core
   ceiling: the lines then plus 200, L4); every row of this plan says kept, not kept or not built.
7. The final adversarial review, then fixes, until accepted.

## 5. Order and lanes

```
Phase P   done
Phase 1   B: B1 (the skip; review fixes) ──────────────────────────┐
          G: G1 (done) ─ G2 (design, review, step) ────────────────┤
          H: H1 ─ H2 ;  H4 (probe, then maybe a step) ─────────────┼─► lane reviews
          C: C1a ─ C1 ─ (C2 at its entry) ─ C3 ─ G-C ── after H in copy/ ┤
          F: done ─────────────────────────────────────────────────┘
Phase 2   C2 not built: A4 ;  C2 built: C4
Phase Z   the end ─► final review
```

**Where steps collide, this order holds:**
- **H before C in `copy/`** (L35, L39): H1 and H2 change `copy/mod.rs` and `copy/young.rs`, as C1 and C2 do; H has the
  larger gain and is simpler; C rebases on H and restates its counted gain on H's world.
- `change/kernel.rs` (L34): B1 (the skip) rebases on G1 (merged); G2 (in `bond/month.rs`, and the kernel only through
  the call) after B1; each re-pins golden. G2's walls' check uses the world after B1.
- H1, H2, C1, C2 and G2 are level 3 or 4: they merge one at a time; each re-runs the key numbers against the reference
  the one before set (L10).
- H1, H2, C1, C2 bump the snapshot: the number is the highest plus one at each merge, and the benches are remade (L11).

## 6. Reviews

The cycle is: plan, adversarial review, implement, adversarial review, fix, until accepted. Done: the plan (two rounds),
`cover.md` (two rounds), lane F, lane B's first round. To come: this restatement; G2's and H's designs before their
steps; lane B after its fixes; each lane after its last step (B, G, H, C), each level-3 or level-4 lane with the
key-number test against the reference; the cover's restated design and G-C; phase Z.

## 7. Rule and doc changes, and the step that writes each

| Rule or doc | Lands with |
|---|---|
| `docs/tape.md` (a run's year holds its saves and pictures; how the tape is written) | F (done) |
| `laws.md` 2.1 or 2.4 (a tiny bare reaction in a body is skipped, over a year) | B1 |
| `laws.md` 3 (a part reacts its own share of the air, with its cell's heat) | G2 |
| `laws.md` 4.2 (children in lumps, at most once in a body's window), 4.4 (maturing in lumps); `numbers.md` the window | H1, H2 |
| handover 7.1 (the nodes check's noise settings `--crowd 2` and `4`); `laws.md` 4.4, `numbers.md` `crowd` (bodies with a brain only, 1) | C1a |
| handover 7.1, 7.3; `laws.md` 4.2, 4.4, 6 (what a plant is); `numbers.md` `cell_kinds` | C1, C2 (from `design/cover.md`) |
| `CLAUDE.md` row "Lighter world (done)"; handover 13.2 (the law of living bodies left to the owner), 14.3 and `CLAUDE.md` 2.11 (the ceiling) | Z |

## 8. Risks

| Risk | Answer |
|---|---|
| H1 changes the timing of births (up to a window later) and the brood's exposure to deaths | the brood fills as today and leaves with dead members (`Body::lose`); births a year and the young node within their seeds' noise; the nodes check |
| H1 or H2 makes a world-wide pulse (every node at the same batch) | each body's window has its own phase (its id); no date shared by every body (L39) |
| H2's new field and the young node's counts differ by up to a window of flux | the maturing tests restated over the window; the eaters' key numbers |
| G2 changes the parts' walls and their gas turnover | its walls' life check; the books |
| The cover's gain on the restated world is small | C1 judged at 3%, C2 at -10% with the animals unchanged; the alternative A4 or nothing |
| B1's skip moves the long-step test by chance (one seed spans 0.10 to 0.40 K) | the test judges the mean of seeds 1 to 6 (L38); the base's 3-seed value said in the report; the cause of the small room (the 60-day liquid, X80) stays an open item for the owner |
| Lanes collide in `copy/`, golden and the snapshot | section 5's order; one merge at a time; key numbers again after a rebase |

## 9. Lines

Core at the round's start 10,444 (R0); after F and G1 10,450; ceiling 10,643. Budgets: B1 about +8; G2 about +20;
H1 about +20; H2 about +30; H4 by its probe (at most +20); the cover by `design/cover.md` 7 (C1a about +2, C1 about
+29 net, C2 about +115 net). Without C2 about +130 (to about 10,580), within the ceiling; with C2 about +245 (to about
10,695): C2 (or the step that crosses) raises the ceiling in its commit, with a decision (L31).
