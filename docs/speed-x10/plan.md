# Speed x10: the plan

Status: plan, after the second adversarial review, restated after phase P (section 5.0). Branch `speed/x10`. **This
file is the single source of the steps:** each kept step has here its description, check level, done-when, kill line,
what it needs and its agent. The designs in `design/` (`month.md`, `plants.md`, `animals.md`, `engine.md`, `air.md`,
`minds.md`, `gap.md`) hold the detail; `month.md` replaces parts of `plants.md` and `animals.md` (its note says which);
each starts with a note of which of its steps are kept, dropped or renamed. Decisions: `decisions.md` (X1 and on).
Measures: `measures.md` (made in R2). History: `docs/history.md`, section 6.

## 1. The goal, and what to expect

**The goal:** a settled year on 4 cores 10 times faster than at the start of the last round: island22 0.910 s ->
0.091 s, globe36 1.490 s -> 0.149 s (`docs/speed-post-foundation/measures.md` Z1). Today 0.614 s and 0.738 s, so
6.7 times (island) and 5.0 times (globe) are still to find.

**The owner's rules for this round:** bad ideas are dropped from the start; every dropped strategy brings an
alternative (section 3); laws and rules may change for speed; the three rules for smart bodies hold (individuals stay
real; the step length per place; kinds of body code).

**What the plan can give, after phase P** (section 5.0; the designs' estimates are in 5.1 and are not repeated here).
Times against the baseline; "likely" counts G5's fallback, half of G12 and no MI3a or MI4. **Until PR-A2 and P1b's
30-day check land, "likely" is the span of both cases of G2:** a 30-day floor for every node, or for young nodes only.

| Set of steps | island22 | globe36 |
|---|---|---|
| today | 1.5 times | 2.0 times |
| **likely, everything with G14 (-15 to -35% of the year; globe from -13%)** | **5.8 to 8.7** | **7.6 to 12.4** |
| of it, G2 at 30 days for every node | 6.7 to 8.7 | 9.3 to 12.4 |
| of it, G2 for young nodes only (X48) | 5.8 to 7.6 | 7.6 to 10.2 |
| without G14 | 4.9 to 5.7 | 6.6 to 8.0 |
| without G14 and without AIR4 | 4.2 to 4.8 | 5.7 to 6.8 |
| without G2 and G3 (P1b keeps neither the eaters nor the mineral at 30 days), with G14 | 4.8 to 6.3 | 6.1 to 8.1 |

**Plainly:** **10 times on the island is no longer likely**: about 6 to 9 times. G14-P1 saved 15% (island) and 13%
(globe) of today's year, not the 25 to 39% its bound gave; a stand update costs 10 to 10.5 µs, not 6 (PR-M, with the bare
reactions the prototype left out); MI3a's gate fails (R2). **The globe reaches 10 times only with G2 at 30 days and
G14 past its low end:** about 7.6 to 12.4 times. Two steps carry the risk: **G2** needs P1b's month to pass PR-A's
30-day check (the eaten mineral and the adults' cycle, answered by the sub-steps of `month.md`; PR-A2 looks for the
rest); **G14**: G14-P2 restates it after G4, G5, AIR2, AIR3. No claim is made for 16 cores: this machine gives at most
about 3 times on 4 cores (`gap.md` 1.3).

## 2. Rules for this round

All of `CLAUDE.md` holds; the rules that change land with the step that makes them true (section 7). The working rules
of the last round hold (`docs/speed-post-foundation/plan.md` section 2): one step per commit, a review after each
lane, the three rules for smart bodies.

**Check levels.** Level 1: bit for bit (golden unchanged with and without threads). Level 2: the same to rounding
(golden re-pinned; a few series columns within rounding where life does not amplify it). Level 3: the same world:
the key numbers (`tape/tests/ranges.rs`) in their ranges, the pace tests, and the nodes check **only for steps that
touch nodes or minds**. Any change of `Config` (a field or a default) re-pins golden. A level-3 step re-pins golden in
its commit and runs the key-number test once.

**Key-number references.** R2 runs the key-number test once at the branch's start and keeps its values in
`measures.md`. Every level-3 step is judged against these, and each lane review runs the test again. S50 (stored share)
and S84 (sleep bias on `air_3`) are known failures: a step must not move them past their seeds' noise unless it says
why (X5).

**How a gain is judged.** One thread: callgrind instructions of a year (S45). Four threads: wall time, 5 or more
interleaved runs on a quiet machine, medians. **One lock for wall time:** every wall-time run takes `flock
/tmp/ew-bench.lock`, so two lanes never time at once; a run is thrown away if another cargo, rustc, valgrind or ew
started during it.

**Steps that bump `SNAPSHOT_VERSION`** (`Config` is saved in `World`, so any `Config` field does) cannot run the
other build's save. **They are judged on one world (X50, PR-S's second method):** the base's save is turned to the
step's snapshot (the default bytes of a new field inserted), the step's build runs one year from it, and its
instructions per update and per stand-year (or per node-year) are set against the base's on the original save. A step
that changes no law counts above 0.1% of the year; a law step changes the world inside its year (one more year moves
`run_for` by +0.15% on the island and -1.3% on the globe), so its whole-year gain counts only above 2%. Remade benches and same-seed runs from worldgen (X24) are a check only: across different worlds a
gain counts only above 19% (island22) and 11% (globe36) on the mean of three seeds. Parallel lanes each bump the
version; at each merge it is set to the highest on either side plus one.

**Agents.** Opus for steps that change a law or the structure of the update; Sonnet for normal code, tests and docs;
Haiku for measures. Steps on different files run in parallel worktrees and merge one at a time; a step that re-pins
golden rebases and re-pins on the branch it merges into.

**Words.** No Earth words in `core/` (`cli/tests/word_ban.rs`): code names use physical words, never "herd" (a
group of movers) or "sea" (open liquid cells).

## 3. The owner's ideas: what is built, what is dropped, and the alternatives

| # | Owner's idea | What we build | Dropped, and its alternative |
|---|---|---|---|
| 1 | Plants as a field | Stands: the plant node of a cell, stepped by the month law (P1.2), then per cell in the survey, out of the ring (P3) | A new field type replacing plant bodies (X7): stands stay `Body` values |
| 2 | A growth table per genome | A card: the constants of a genome's month law, made in `prepare` (P1.2) | A grid table (X8): alternative the card; discrete strains (P5) only on a branch if the owner asks |
| 3 | Upkeep and light together | The month law for stands (P1.2) and for every body (P1b), `design/month.md`; `D` is 36 days for stands and the gene interval for bodies with a brain (X44) | Today's law at a longer pace (C2): replaced by the stock equation |
| 4 | A different history with threads | Not needed: the history is fixed by the data with any number of threads; E1 shares out the crossing writes | An order-free pipeline (X3): alternative E1's fixed-order pipeline |
| 5 | 32-bit books | Books stay 64-bit; 32-bit only for numbers that hold nothing (G11 uses it) | A 32-bit store (X4): alternative G11 (f32 rates in the air's rows) |
| 6 | Fewer air steps; a coarser air grid | AIR1 to AIR3; calm months AIR4 (= G10); G11 | A coarser air grid (X10): alternative AIR1 to AIR3 and AIR4; AIR5 only for worlds of 2,000+ cells. Longer Euler air steps: alternative AIR4. AIR6 (calm cells share a row): alternative G11 (X27) |
| 7 | Raise the code ceiling | 12,000 code lines in `core` (X22) | 11,500 (too tight for the full set) |
| 8 | Think only on change | Habits (MI3a), paid per thought (MI3b), both after G2 and only if MI3a's gate still holds then (X36) | MI6 (calm animals update less often): alternative G2 (X16). If MI3a's gate fails after G2: alternative G2 itself plus G16 |
| 9 | Think in groups | A node's movers leave as one group (MI5); shared place senses (MI4) after G2, if its gate still holds (X36) | One decision for unlike nodes (a merge of unlike nodes, S66): alternative MI5 and MI4 (or G16 if MI4's gate fails) |
| 10 | Batched neural networks | Not built (X12) | Alternative: habits (MI3a) cut the thoughts; GM measures a thought for later brains |
| 11 | Full thinking for smart bodies | `--full-minds`, the control run of habits (MI3a; if MI3a is not built, `--full-minds` waits for the first smart kind); the mark of a smart lineage comes with the first smart kind (MI7) | `Config::full_hidden` now (MI8, X25): dead code, and a node that crossed it would open into about 10^4 individuals at once |

**Other ideas checked and dropped** (`gap.md` 4, `engine.md` 2.6, `air.md` 1.5):

| Idea | Why not | Alternative |
|---|---|---|
| Climate "on repeat" (last year's fluxes) | a cache of past results, not physics; most cells would wake anyway | G5, AIR4 |
| Last year's air as Newton's first guess | the step before was already worse (2.4 rounds, not 2.1) | AIR1's calm solver |
| Change asleep by its net change | sleeps reaction pairs that undo themselves (S84's bias) | G1, G5 |
| A cap of bodies per cell | not a speed tool after P3 (a crowded cell costs about 7.5 ms a year); a world choice | P3, G9a |
| Fewer substances or reactions | Change's rows cost 2.3 µs a node; the cost is the booking and the clock | G1, G5 |
| A stage-by-stage update for the instruction cache | 3% repeated work (E3's gate); it reorders moves (level 3) | G3 and G9 shrink the update code |
| A linear-time air solver | the band solve is 6 to 8% of the air step | AIR5, gated on large worlds |
| Takes inside one crowded cell on several threads (history by timing) | not needed while E2's tiles balance | E2 |
| D1 (b)'s reserved shares (S81) | 170 to 280 lines, more law changes | G14 (the cut at booking) |
| The 16-core model as a road to the goal | this machine gives at most about 3 times on 4 cores | the 4-core budget of section 5; E7 measures only |

## 4. The steps

"4T": wall time on 4 threads; "1T": callgrind instructions of a year on one thread; "detail": where the design
holds the rest. A kill line reverts the step and keeps it on a branch.

### Phase P: rules, measures and probes (before phase 1)

| ID | What | Level | Done when | Kill line | Needs | Agent | Detail |
|---|---|---|---|---|---|---|---|
| R1 | The rules true at once: `CLAUDE.md` 2.11 (ceiling 12,000), 3.1 (books are 64-bit), 6.1 (the number of threads does not change the world; every order fixed by the data), its table row "the work now"; handover 12.1, 12.3, 12.5, 14.5, and 14.3 (the `core` row 12,000, and the "20% with a reason" sentence: this round goes past it with the owner's explicit yes, X22); `cli/tests/core_size.rs` `CEILING` 12,000 | 1 | tests pass; wording as X3, X4, X22 | none | none | Sonnet | engine 4 |
| R2 | One set of benches remade in one hour, and every gate measured on it: plants' and animals' shares (P0), E0 (a) to (f), AIR0, MI0 (sleep share; wake causes counted by cause, `others` keyed by lineage; decisions per (cell, lineage, batch); place sharing), G0 (the update split, Change's sleep, the clock settings, the four-worlds test), the minds breakdown (Z1: `choose::act` 11.6 / 17.8 / 3.5%, not 16%); the key-number references; section 5 restated | none | rows in `measures.md`; section 5 restated and read by the review | none | R1 | Haiku (Sonnet for timer builds on branches) | plants 9 (P0), engine 9 (E0), air 9 (AIR0), minds 9 (MI0), gap 9 (G0) |
| R3 | The long-step test runs `phys_days` 2 and calls `Config::validate` (AIR-T); the `flow_days` bug: the liquid's clock counts its own period (`clock::Every` within it, as creep does) and `validate` refuses a `flow_days` over a month that is not a whole number of months | 1 (golden unchanged at `flow_days` 30) | the long-step test runs a world `validate` accepts (it passes, or its failure is reported); a test that `flow_days` 60 runs the liquid every 60 days | none | none | Sonnet | air 5, gap 1.2 |
| PR-M | Month prototype on a branch: the plant month (stages 3 to 6); µs per stand; its error against `body::live` on a grid (light, temperature, wetness, fuel fill) at 2 and 6 days; **and a first look at the stands' `D`**: the prototype runs inside the update on the branch, with `D` = the gene interval against `D` = 36 days, and how far the key numbers move (X14) | none | a table in `measures.md`; the first look at `D` (the final choice is P1.2's, X14) | a stand over 4 µs, or over 5% error at 6 days: P1.2's design is reviewed before it starts. The first look shows `D` = gene moving a key number out of its range, or S50 or S84 past its seeds' noise: P1.2 starts with `D` = 36 (a `numbers.md` row) | R2 | Opus | plants 2.4, animals 3.1 |
| PR-B | Brain loss: a counter (on a branch) of resizes to no hidden unit, with members and parts held, in 1,000 years of island22 and globe36 | none | rows in `measures.md` | none | none | Sonnet (counter), Haiku (runs) | this plan, P1.2 |
| PR-A | What starves animal nodes at a 30-day pace with today's law, cause by cause | none | its table | none | R2 | Opus | animals 2 |
| PR-A2 | The rest of PR-A's loss after the eaten mineral: the adult nodes' root cause at a 30-day pace, and the smallest law change that removes it (branch) | none | its table in `measures.md`; `month.md` 5.2 replaced by its answer | none | PR-A | Opus | month 5.2 |
| PR-S | The judging method: a dummy `Config` field on a branch; the noise of per-update cost on own remade benches and of same-seed runs from worldgen | none | noise bands in `measures.md` | none | R2 | Sonnet (branch), Haiku (runs) | section 2 |
| G14-P1 | First overlap probe on today's code (branch): the air step and the batch joined (`rayon::join`, physics rows not booked; timing only). **Month windows** (Change, the state, the liquid) **and other windows are timed apart**, and the serial part outside the two lanes (survey, Bond's month, tape, draws) is reported | none | 4T year of both together against one after the other; the serial part in ms | under 10% saved (X38): G14 is marked at risk in section 5 and the owner is told; G14-P2 decides | R2 | Sonnet | gap G14 |
| AIR4-R1 | Why 30-day Euler steps warm the winters: switch the explicit parts one at a time | none | a table of each part's share of the +1.2 K | none | R3 | Opus | air 2.4 |

**Review after phase P.** Done: section 5.0 restates section 5 with R2 and the probes (X51). PR-A failed (its causes
(a) to (d) win nothing) and G14-P1 passed on the line of X38 (15% and 13%), so the "both fail" message (6 and 8 times)
does not apply; the owner is told the restated end: island about 5.8 to 8.7 times, globe about 7.6 to 12.4 (the span
of G2's two cases until PR-A2 and P1b's check land). The month law is redesigned after PR-M, PR-A and its review
(`design/month.md`, X41 to X53).

### Phase 1: five lanes in parallel

**Lane A, plants and animals** (`body/`, `copy/`, `params.rs`)

| ID | What | Level | Done when | Kill line | Needs | Agent | Detail |
|---|---|---|---|---|---|---|---|
| P1.1 | `body::apply_extent`: one helper books a reaction's extent on a body, cut out of `pay::pay_at` | 1 | golden unchanged with and without threads | not bit for bit: fix, never re-pin | R1 | Sonnet | plants 2.3 stage 5 |
| P1.2 | The month law for stands (`design/month.md`): the plan (stages 0 to 5, writing nothing; it returns the new light and rain marks) and the booking (X41); the fuel stock by X9's closed form with product inhibition on its product substance, the release limit, the moving reserve `U (D + days left)` and deaths by the rate; the body's other reactions (bare ones too, X43) and growth in sub-steps by the reactor's kernel with their own orders, ended at `MONTH_SHARE` of a scarcest input or a change of the fuel's phase (X42); starving judged per sub-step on the body's path (X53); every rate read over the window `D`, `D` = `Physics::reserve_days` 36 for stands (X44); the liquids first, no sweating, the stand at its ground's temperature as the last stage, the dead leaving at that state (P1.3 folded in, X45); the booking of the net change per substance in one pass (X52); the card; the sample in a scratch body; `node_days` -> `field_days` **30** (`Config::validate` wants a multiple of `batch_days`; G4, G5 and P3 need 30, 60 or 90, X17); **the brain-loss switch** (below); the kept tests of `month.md` 6 and a switch test; `laws.md` 6.10, 6.3 (stands), 2.4, `numbers.md` (`field_days`, `reserve_days`), handover 7.1, 7.3; snapshot bumped. **A law change, announced:** deaths by the rate (`1 - e^-u` for a deep famine); growth through the interval beside the body's reactions; the reserve with the rest of the interval; stands at their ground's temperature, without sweating; the floor from 36 to 30 days | 3 | the splitting test of `month.md` 6 (90th percentile 2%, worst 10%, with young stands and a seasonal ramp) and its day-law check; tests pass; books 1e-12 in the century; key numbers within the references, the land's temperature reported (a move over 0.3 K is looked into); the stand's law at most 5 µs (island22 callgrind); stand updates -30% per stand-year and the year -9% (X50: one year from the base save turned to the new snapshot) | the splitting test fails at 60 days after a smaller `MONTH_SHARE`; the stand's law over 6 µs; under 7% fewer island instructions; key numbers out and not accepted. A climate test fails: stage 7 takes `relax_heat`'s closed form and the rates the stand's own temperature (X45) | P1.1, PR-M, PR-B | Opus, adversarial review | month 2 to 7 |
| P1.3 | **Folded into P1.2** (X45): the month reads its rates at one temperature, and PR-M measured the month and the ground's temperature together. Its kill line is P1.2's fallback | | | | | | month 2 |
| P1.4 | `sample_every` goes: the sample (stages 2 to 5 of the month for its genes, in a scratch body, `month.md` 5) runs at every update | 3 | `selection_inside_a_node_moves_its_mean_as_its_members_would`; `genes_do_not_drift_without_selection` (slow); a sample run at most 3 µs; island 1T not up over 1% | the selection test fails: `sample_every` stays 2 | P1.2 | Sonnet | month 5, 7 |
| P2 | `field_days` 60 | 3 | the splitting test at 60 (1 x 60 against 6 x 10) on its grid; `the_pace_of_the_field_keeps_life` at 10 against 30, 60, 120; key numbers. These values are valid here because `phys_days` is still 10 and `batch_days` 2: the reference 10 is "every survey" only while `phys_days` is 10 (after G5 it is 30, see G9a) | the pace test or the splitting test fails at 60 (with the month's sub-steps): 30 stays | P1.4 | Sonnet | plants 9, month 6 |
| P1b | The month law for every body (`month.md` 5.1): the plan and the booking for bodies with a brain, `D` their `INTERVAL` gene; **keep-warm's power in `U`**, taken at the update's start (X14); growth in the sub-steps beside the body's own reactions, so the eaten mineral is used as it turns solid and the body is rebuilt as it is burnt (PR-A); starving per sub-step on the body's path and the reserve with the rest of the interval (X53); `settle_upkeep`, the body's call of the reactor, `settle_gas`, `store_light`, `keep_warm`, the body paths of `soak` and `roots`, `copy::grow`'s conversion and paying and `soak_again` go; the day law's heat network stays, fed the month's heat; the acts stay after the update. **PR-A2's answer (pending, `month.md` 5.2) may add a step.** **A law change, announced:** deaths by the rate (a deep famine kills `1 - e^-u`, not `u`); no dip of the body between updates; keep-warm inside `U` | 3 | the splitting test with young and adult animal nodes, also at 260 K, and a dip that recovers; key numbers at today's pace in their ranges, the shifts reported as law effects; **PR-A's script again with P1b at a 30-day floor: the eaters within the seed noise of the gene's pace, its counters read** (adults under their copy mass at `copy`, the mineral left at growth); an animal update 1T measured (an estimate: -10 to -20%) | key numbers out and not accepted: not merged. The 30-day check fails: P1b merges as the law (X6); if the mineral is kept, G2 takes the young-only floor; if not, G2 waits for the cause (X48) | P2, PR-A, PR-A2 | Opus, adversarial review | month 3, 5.1, 5.2; animals 1, 4 |

**The brain-loss switch (P1.2, P3).** Plants are not a closed set: `lineage::resize` (`core/src/lineage.rs:69-85`)
moves a body whose `HIDDEN` gene falls below 0.5 (its range starts at -1, `body/genes.rs:105`) to a lineage with
no hidden unit, and `resize` already clears its trace, memory and signal (`lineage.rs:81-84`). From then on it is a
stand. The law changes at the resize: (1) from its next update it runs the month law as a stand (P1.2: `D` becomes
`reserve_days`); (2) it lets go of any part it holds or is inside (the `bond` helper a dying holder uses), at the
resize; (3) it is scheduled at `field_days` from that day (P1.2); (4) at that update its heat steps to its ground's
temperature, the difference booked with its bodies' node and the ground as the month's last stage books it (P1.2);
(5) it leaves the ring (P3). The other way cannot happen
(decision 72: a body with no brain does not mutate its brain-size genes, `genes.rs:256-264`). Test:
`a_body_that_loses_its_brain_becomes_a_stand` (a node with `HIDDEN` at 0.4 resized: it holds no part, its next update
is a month, books kept). PR-B says how often it happens.

**Lane B, engine** (`outbox`, `life/`, `survey::batch`, `store/`, `par.rs`)

| ID | What | Level | Done when | Kill line | Needs | Agent | Detail |
|---|---|---|---|---|---|---|---|
| E1a | `pushed` and `crossings` per domain; booking and delivering only the domains touched | 1 (series bit for bit; fingerprint re-pinned for the snapshot format) | golden with and without threads; 2 years of each bench, same `series.csv` | the parts' or bodies' stage slower (1T) | R1 | Sonnet | engine 2.3 |
| E1b.1 | Faces per domain, made at the end of the domain's own phase; a domain changed later marks its faces stale | 1 | golden unchanged | bodies' stage 4T slower | E1a, MI1 | Sonnet | engine 2.2 |
| E1b.2 | Landing by destination: leaving bodies taken out at P1's end, outboxes in buckets, global ids in S1, landing per destination in P2 | 3 | the order test (widened) equal; golden re-pinned, the same without threads; key numbers once | bodies' stage 4T under -6% on two benches, or worse on the third | E1b.1 | Opus | engine 2.2, 2.3 |
| E1b.3 | The S2 reorder: part acts, watches, returns of litters, lane clears after all landings | 3 | as E1b.2 | bodies' stage 4T slower than after E1b.2 | E1b.2 | Opus | engine 2.2 |
| E1c | Booking by array in P2's call: **the one helper `store::book_by_array`** (AIR3, G1, E4 use it); **one domain runner** (LPT order, scratch per thread with `each_init`, `SERIAL_BELOW`) that AIR2, G1, G6, G7 use | 2 | golden re-pinned; the series to rounding | bodies' stage 4T under -2% on top of E1b on two benches | E1b.3 | Opus | engine 2.2 |
| E2 | Tiles of 4 cells with the air column. First, on the merged branch: the busiest share at 4 cells (gate: at most 0.6 x today's on two benches). Audit: no body code reads `Class::Air` straight from the store (only through the view); the shares of an air node's cells sum to 1 (a debug assert). `CLAUDE.md` 3.4 tile sentence, handover 7.4 | 3 | order test; golden; key numbers; per update 1T at most +3% | bodies' stage 4T under -5% on two benches and under -8% on region24 | E1c | Opus | engine 2.4 |

**Lane C, air** (`flow/`, `solve/`)

| ID | What | Level | Done when | Kill line | Needs | Agent | Detail |
|---|---|---|---|---|---|---|---|
| AIR1 | Calm cells (`CALM_K`, `CALM_SHARE`) and the calm solver (`STILL_SHARE` 2e-4, X39); `CLAUDE.md` 6.5 (air tolerances are speed numbers) | 3 (with the long-step and globe climate tests) | globe rounds at most 3.3; cell rows -25%; both climate tests; golden re-pinned; key numbers | Flow 1T under -8% on globe36 | R3 | Sonnet | air 2.1 |
| AIR3 | The air's book as column moves, written through E1c's `book_by_array` (no own pass); E4 does not touch the air | 1 or 2 | `book` -40% | under -25% | E1c, AIR1 | Sonnet | air 2.3 |
| AIR2 | The air's and the liquid's per-cell loops in phases on E1c's runner, the domains **cut by air node** (the tiles of one node in one task), partials per air node summed in node order | 2 | golden re-pinned; 5-year runs: climate columns within 0.05 K and 1%; Flow 4T x1.25 on both | under x1.15 on either | E1c, AIR3 | Opus | air 2.2 |

**Lane D, minds** (`choose/`, `brain/`). MI3a, MI4 and MI3b come in phase 2, after G2 (X36).

| ID | What | Level | Done when | Kill line | Needs | Agent | Detail |
|---|---|---|---|---|---|---|---|
| MI1 | The cover of a face once per batch (`hide` in `Faces::make`); lands before E1b.1 | 1 | golden unchanged; island 1T -0.8% | under -0.5% | R1 | Sonnet | minds 2.7 |
| MI5 | A node's movers leave as one group; `can_go` before a split | 3 | nodes check; globe 1T -1.5% | under -1.5% on globe, or the nodes check fails | MI1 | Sonnet, Opus review | minds 2.3 |

**Lane E, the steps that change no law** (`change/`, `bond/`, `tape`, `par.rs`)

| ID | What | Level | Done when | Kill line | Needs | Agent | Detail |
|---|---|---|---|---|---|---|---|
| G8 | The tape's recorder on the pool (chunks summed in order) | 1 for the world (golden unchanged); 2 for the tape (its files the same to rounding), or 1 if each sum keeps its order | golden unchanged; tape files as the level says; tape 4T -40% | under -25% | none | Sonnet | gap G8 |
| G1 | Change per domain on E1c's runner: the domain tasks make the rows (they book nothing); the rows are booked after the call by `book_by_array`, and the air nodes' sums in domain order. So G14 later books the same rows after the bodies and does not redo G1 | 2 | golden re-pinned; Change 4T -50% | under -30% | E1c | Sonnet | gap G1 |
| G7 | Bond's monthly turns per domain | 2 | parts 4T -40% on globe36 | under -25% | E1c | Sonnet | gap G7 |
| G15 | Find and remove the sharing between threads ((a) to (d) of G15, each measured first) | 1 | an update on 4T at most 1.2 times its 1T cost | nothing found above 3% of the bodies' stage: stop | E2 | Sonnet (measures), Opus (a fix) | gap G15 |

### Phase 2: the clock and law steps, then the research steps

| ID | What | Level | Done when | Kill line | Needs | Agent | Detail |
|---|---|---|---|---|---|---|---|
| P3 | The field step per cell in the survey; stands out of the ring. `field_days` must be 30, 60 or 90 (a multiple of `phys_days`, X17). Files: `survey::survey`; `Life::unsound` (stands are in no ring); `copy::place_child` and `Domain::add` (no ring entry for a stand); `Body::interval` (its `n() > 1` test: every stand by `field_days`); the snapshot reader (`life/mod.rs:118`); the switch leaves the ring | 3 | plants test 4; key numbers; island 1T -5% on top of P2 | under -5% | P2, E1b.3, E1c | Opus | plants 2.5 |
| G13 | One heat draw per cell for stands | 2 | `Draws` and the light marks loop over cells for stands | under 2 ms a year | P3 | Sonnet | gap G13 |
| G6 | The whole survey per domain on E1c's runner (in G14 it runs in the bodies' lane) | 2 | survey 4T -35% on both | under -20% | P3, E1c | Sonnet | gap G6 |
| G9a | `field_days` 90. **Comes after G4 and G5.** The test's settings are 30, 60, 90 and 180, all multiples of `phys_days` 30 and of `batch_days` 10 | 3 | the splitting test at 90 (1 x 90 against 3 x 30, `month.md` 6); the field's pace test at 30 (the reference: every survey, once `phys_days` is 30), 60, 90, 180; key numbers | fails at 90 (with the month's sub-steps): 60 | P3, G5 | Sonnet | gap G9, month 6 |
| G9b | Compact stands (P4). Gate: the handling of `Body` values (moves, clones) at least 5% of a year in P3's profile | 1 | stands 1T -10% | under -3% of a year on 4T | P3, G9a | Opus | plants P4, gap G9 |
| G4 | `batch_days` 10 (replaces S64). `field_days` must then be 30, 60 or 90 and `node_days` a multiple of 10 (`Config::validate`, X17). A small law effect: gene intervals from 5 to 15 days all become 10 days | 3 | the batch test (`the_batches_of_bodies_keep_life`) changes from 1 against 2 days to 2 against 10; key numbers; bodies' stage 4T -10% | under -6%: back to 2 | E1c | Sonnet, Opus review | gap G4 |
| G5 | `phys_days` 30; the air stays at 10 days (`field_days` 30, 60 or 90). **Expect the fallback:** 75% of Change's rests are under 10 days, and a 30-day span for every node grows S84's bias, so Change likely keeps 10 days and the rest takes 30; section 5's "likely" column counts only that part | 3 | long-step and globe climate tests at `phys_days` 30; the sleep test within noise; key numbers; year 4T -4% | a climate test fails or under -3%: 10, or Change at 10 and the rest at 30 | G4, G1 | Opus | gap G5 |
| G2 | Every population node updates at least every `node_days` 30; the movers (with MI5's one group: `x = count x min(1, Σp x paces)` members leave, `Σp` the sum of the chances to go in every direction, and one direction is drawn by `p[d]/Σp`), learning's discount (`discount^paces`, carried here, so MI3a is not needed), `away` (gene intervals) and an act's heat follow the pace; MI6 is never written. **The nodes check (30-day nodes against individuals at their gene) is a strong test of G2.** **The floor (X48):** 30 days for every population node; if P1b's 30-day check failed but its counter shows the eaten mineral kept, 30 days for young nodes only (PR-A: the eaters within the noise once the mineral is kept; about 0.6 of the node updates, about 57% of G2's gain); if the mineral is not kept, G2 is not built until its cause is fixed; 20 days is not a fallback (PR-A: it fails too) | 3 | pace test for animals at 10, 30, 60; key numbers; nodes check; learning scene; bodies' stage 4T -25% on both (young only: -12%) | fails at 30 for every node: young only; fails for young only, or under -15% (every node) / -8% (young only): revert | P1b, MI5 (PR-A2 if it names a cause) | Opus, adversarial review | gap G2, month 5.1 |
| G3 | The heat of population nodes with a brain by a closed form: the stand's lean evaporation with sweating as a rate, and `relax_heat`'s closed form without its second conductance; individuals and full minds keep the day law's heat. **Gated after P1b (X49):** P1b's month already takes the reactor, upkeep, keep-warm and growth out of an animal's day; G3 is built only if the heat network and the evaporation are at least 25% of an animal node update in P1b's profile | 3 | the splitting test for an animal node, also at 260 K; animal update 1T -15% | the gate fails: not built; under -10%: keep P1b's law only | P1b, G2 | Opus | gap G3, month 5.1 |
| MI3a | Habits, still paying one decision per update; `--full-minds` (the control run); counter `thoughts`. **Gated after G2** (X36): on the merged branch after G2, the sleep share at least 40% on island and globe; likely gain 5 to 8 ms a year for about +145 net lines. Not built: `--full-minds` waits for the first smart kind, and G2 plus G16 stand in its place. `CLAUDE.md` 2.6 adds `--full-minds`; handover 6.5 habits bullet, 7.5; `laws.md` 5.3, 5.4, 5.6 | 3 | all tests; the learning scene; `habits_give_the_patterns_of_full_minds` (the full-minds check, slow) and the nodes check on this commit; key numbers; island and globe 1T -1% (after G2) | the gate fails after G2: not built. Under -0.7% on both, or the full-minds check fails beyond the noise: revert | G2, MI1, R2 (MI0) | Opus | minds 2.1, 2.5 |
| MI4 | Shared place senses per domain and survey. **Gated after G2** (X36): sharing at least 1.5, measured again on the merged branch; likely gain 1 to 2 ms | 2 | golden re-pinned; series within noise; island 1T -0.5% | the gate fails after G2: not built. Under -0.3% island and -0.2% globe: revert | G2 (MI3a if built) | Sonnet | minds 2.2 |
| MI3b | Pay per thought, its own law commit, only if MI3a is built: a thought's cost is a drain in P1b's balance over the interval; the commit gives its effect on the eaters (mass, members, births less deaths) | 3 | key numbers; nodes check; eaters within noise of MI3a, or the change reported as a law effect | the eaters leave their range and the owner does not accept | P1b, MI3a | Opus | minds 2.1 "Paying" |
| G12 | `flow_days` 60. Its gain is a guess until R3's clock fix lands (the probe's liquid ran monthly); section 5's "likely" column counts half | 3 | rivers, land rain, worn land in range; liquid -35% | a key number out: 30 | AIR2, R3 | Sonnet | gap G12 |
| AIR4 | Calm months (= G10). If AIR4-R1 names an explicit part, it is fixed first; **then a calm-month variant is always built and measured** (TR-BDF2 with its error estimate, against Euler with the fix) | 3 | long-step test within 0.3 K, globe test, key numbers, sleep test; at least 6 of 12 months calm on both | Flow 4T under x1.4 after AIR3, or a climate test fails | AIR4-R1, AIR1 to AIR3 | Opus | air 2.4 |
| G11 | f32 and SIMD for the air's cell rows (replaces AIR6) | 2 | climate tests; `pass` -35% | under -20%: revert, then AIR6 is tried as the alternative (gate: R2's share of cells that would group at least 40%) | AIR2 (after AIR4 if kept) | Opus | gap G11, air 2.6 |
| G16 | Well-being once per decision (with G2, the alternative to MI3a and MI4 if their gates fail, X36) | 3 | learning scene; nodes check | under 1% of the bodies' stage | G2, MI5 | Sonnet | gap G16 |
| E4 | `store::apply` per array for what is left on one core (the liquid's fluxes). Gate: `apply` at least 3% of a year on the main thread at 4T after AIR3 and G1 | 1 | golden unchanged with and without threads | year 4T under -2% | AIR3, G1 | Sonnet | engine 2.8 |
| E5 | Parts per tile. Gate, measured on the merged branch after G7 and G2: serial-body updates at least 10% on a bench, or S2 at least 15% of the bodies' stage | 3 | order test with parts; golden; key numbers | bodies' stage 4T under -8% on globe36 | E2, G7, G2 | Opus | engine 2.5 |
| G14-P2 | The overlap probe again, on the branch after G4, G5, AIR2, AIR3; month windows and other windows timed apart; the serial part reported | none | as G14-P1 | under 10% of a year saved (X38): G14 is not built; the island's goal then rests on AIR4 and is reported at risk | G4, G5, AIR2, AIR3 | Sonnet | gap G14 |
| G14 | Physics beside the bodies: the physics works out its moves from the window's start and books them after the bodies. **Each reaction is cut as a whole, in a fixed order, each against what is left after the ones before it** (as X29): one share per reaction, from its least input against that rest, applied to all its inputs, outputs and heat (a move of matter with its heat, as `hold_air`); never substance by substance. G1's rows are booked here, after the bodies (not redone). The survey's per-domain part (G6) runs in the bodies' lane; the tape reads a frozen copy beside the next window (G8). `CLAUDE.md` 3.4 sentence, handover 7.3, `laws.md` 0 | 3 | order test with the two lanes in both orders; books each month; a debug check on the running total: no row books on a node more than is left after the rows before it, and every reaction keeps its proportions; the cut counter under 1e-3; climate tests; key numbers; year 4T -25% | under -10% on either bench (X38), or the cut counter over 1e-3 | G14-P2, E1c, AIR2, AIR3, G1, G4, G5 | Opus, adversarial review | gap G14 |
| GM | The cost of a thought at 12 and 64 hidden units, and of a `--full-minds` year (if MI3a is not built: of today's year, where every update thinks) | none | a row in `measures.md` | none | G2 (MI3a if built) | Haiku | gap G16 |
| E7 | Scaling at 1, 2, 4 threads (more if a machine allows): a measure, no claim against the goal | none | a table in `measures.md` | none | E2 | Haiku | engine 6.2 |

### Phase Z: the final check

First `main` is merged into `speed/x10` (golden re-pinned if `main` re-pinned it, X1). Then: the benches and
profiles; the slow suite (key numbers, nodes check, pace tests, debug checks); 1,000-year runs of the island, the
globe, the refined region and the largest planet that fits 15 minutes; a watch report `docs/reports/speed-x10.md`
saying plainly where the speed stands against 10 times; the docs describing the present; the ceiling set to the lines
then plus 200.

## 5. Expected gains

**These are estimates** from the designs' measures (`gap.md` 2, on its probe world: today 646 ms island, 818 ms
globe; 96 and 165 ms there stand for Z1's 91 and 149). R2 measures the shares again on one set of benches made in one
hour and restates this section before phase 1. "x" is against the baseline (Z1 scale: 960 and 1,650 ms in the probe
world). Each step is counted **in the order of the table, on what the steps above it left**, so a saving is counted
once: plants' share of the booking and faces is counted after E1, MI5's landings after E1, G2's after G4, MI3a's and
MI4's after G2 (X36).

### 5.0 The review after phase P

R2 and the probes are measured (`measures.md`). 5.1 to 5.3 below are the designs' estimates; this section says where
the measures move them, and its end is the plan's expected result (X51).

| Measure | What it says | What moves in section 5 |
|---|---|---|
| R2 | the shares hold: plant updates 30.0% / 14.7% of a one-thread year, a 30-day floor keeps 0.30 / 0.31 of the animal node updates; the machine runs 4 to 14% slower than Z1's hour; MI3a's gate fails (sleep 14% / 11%, the line 40%) | the table keeps Z1's scale; MI3a and MI4 are not counted in "likely" (their 6 / 8 ms stay; G16 takes 1 to 2) |
| E1a | neutral on one thread (+0.04% of the batch), as expected | nothing: the chain E1a to E1c is judged at E1c (X40) |
| AIR1 | globe: Flow -11.7%, the year -3.6% on one thread (5.2 said -2%); island and region about 0 | the air's row as planned, a little better on the globe |
| G14-P1 | the join saves 15% (island) and 13% (globe) of today's year; the lanes' imbalance is 1.6 to 2.0 (the bound guessed 1.1 to 1.3); the serial part 57 / 75 ms; the saving grows as the bodies' lane shrinks and the physics' share rises; over the line of X38 (10%), so G14-P2 decides | G14: -15% (island) and -13% (globe) at the low end, -35% at the high end, of the year it meets (the bound: -25 to -39%) |
| PR-M | the stand's law 7.0 µs without bare reactions; with them and lean stages about 4.5 to 5 µs; a whole stand update about 10 to 10.5 µs at 30 days, 9 after P3 (5.1 took 6); the month failed its own splitting test until the fixes of `month.md`; `D` = 36 for stands | the plants' rows, below |
| PR-A | (a) to (d) win nothing; the eaten mineral and the adults' cycle are the causes; 20 days fails too; a young-only floor keeps the eaters once the mineral is kept | G2 at 30 days rests on P1b's month passing PR-A's check; its fallback is the young-only floor (about 57% of G2's gain), only if P1b keeps the mineral (X48); "likely" spans both until PR-A2 and P1b land |
| PR-S | across different worlds a gain counts only above 19% (island) and 11% (globe); on one world above 0.1%, and a law step's own year moves the world by up to about 1.3% | law steps are judged per update and per stand-year on turned saves, whole years only above 2% (X50); no gain moves |
| AIR4-R1, AIR4.0 | the plateau fault is fixed; TR-BDF2 lands near the 0.3 K limit | AIR4 stays a research step at risk |

**The rows that move** (four threads, ms a year saved, Z1's scale, in the order of 5.1):

| Step | island, 5.1 | island, restated | globe, 5.1 | globe, restated | Why |
|---|---|---|---|---|---|
| P1.1 to P1.4 | -73 | -48 | -40 | -26 | a stand update of 10 to 10.5 µs at 30 days, not 7.5 (`month.md` 7) |
| P2 | -32 | -43 | -22 | -29 | a dearer update gains more for each update dropped |
| P3 | -13 | -13 | -16 | -16 | 9 µs, not 6; its gain is mostly faces and the ring |
| P1b | 0 | -15 | 0 | -20 | the body's reactor and today's upkeep, keep-warm and growth passes leave animal updates (`month.md` 5.1) |
| G2 at 30 days | -73 | -61 | -121 | -102 | counted on P1b's cheaper update |
| MI3a, MI4 | -6 | -2 | -8 | -2 | gates fail (R2); G16 instead |
| G3 | -9 | -4 | -12 | -6 | only the heat is left after P1b (X49) |
| G9a, G9b | -10 | -10 | -6 | -6 | |
| G14 | -32 to -51 | -15 to -35% | -40 to -65 | -13 to -35% | G14-P1 (its low end per bench) |

**The end, restated.** Before G14 the year is about 169 ms (island; 5.1's likely: 149) and 205 ms (globe; 187) with
G2 at 30 days for every node, and about 195 and 249 ms with the young-only floor. With G14 at -15 to -35% (island) and
-13 to -35% (globe):

| Case | island22 | globe36 |
|---|---|---|
| G2 at 30 days for every node | 110 to 144 ms, 6.7 to 8.7 times | 133 to 178 ms, 9.3 to 12.4 times |
| G2 for young nodes only | 127 to 166 ms, 5.8 to 7.6 times | 162 to 217 ms, 7.6 to 10.2 times |
| **likely, until PR-A2 and P1b's check land: the span** | **5.8 to 8.7 times** | **7.6 to 12.4 times** |
| without G14 (both cases) | 4.9 to 5.7 times | 6.6 to 8.0 times |
| without G2 and G3 (the mineral not kept), with G14 | 4.8 to 6.3 times | 6.1 to 8.1 times |

What would lift the island toward 10: G14 at the good end of its range (G14-P2 after G4, G5, AIR2, AIR3), AIR4 without
its climate risk, G2 at 30 days for every node, and stands near 8 µs. **The owner is told:** 10 times on the island is
no longer likely; the globe passes it only with G2 at 30 days and G14 past its low end.

### 5.1 Four threads, ms of a year (the goal's measure)

"Likely" is the same as "year" down to G4; from G5 on it counts G5's expected fallback (Change keeps 10 days: about
half of G5's gain), half of G12 (a guess until R3's clock fix lands and it is measured) and G14's bound with the serial
parts. Both columns take a stand update of 6 µs after P1.2 to P3; after PR-M it is 10 to 10.5 µs (9 after P3), and
5.0 restates the rows that this and the other probes move.

| Step | island saves | island year | x | island likely (x) | globe saves | globe year | x | globe likely (x) |
|---|---|---|---|---|---|---|---|---|
| today | | 646 | 1.49 | | | 818 | 2.02 | |
| E1a to E1c (the serial phase) | -50 | 596 | 1.61 | | -76 | 742 | 2.22 | |
| MI1 (the cover of a face once a batch) | -3 | 593 | 1.62 | | -1 | 741 | 2.23 | |
| MI5 (acting; its landings after E1) | -4 | 589 | 1.63 | | -26 | 715 | 2.31 | |
| P1.1 to P1.4 (the month law) | -73 | 516 | 1.86 | | -40 | 675 | 2.44 | |
| P2 (`field_days` 60; its booking after E1) | -32 | 484 | 1.98 | | -22 | 653 | 2.53 | |
| AIR1, AIR3, AIR2 | -51 | 433 | 2.22 | | -83 | 570 | 2.89 | |
| E2 (tiles) | -18 | 415 | 2.31 | | -19 | 551 | 2.99 | |
| P3 (field step; faces and booking after E1) | -13 | 402 | 2.39 | | -16 | 535 | 3.08 | |
| **after phase 1 and P3** | | **402** | **2.39** | | | **535** | **3.08** | |
| G1 Change per domain | -32 | 370 | 2.59 | | -32 | 503 | 3.28 | |
| G7 Bond per domain | -4 | 366 | 2.62 | | -16 | 487 | 3.39 | |
| G8 tape on the pool | -8 | 358 | 2.68 | | -8 | 479 | 3.44 | |
| G6 survey per domain | -13 | 345 | 2.78 | | -20 | 459 | 3.59 | |
| G15 sharing between threads | -13 | 332 | 2.89 | | -17 | 442 | 3.73 | |
| G13 one draw per cell | -3 | 329 | 2.92 | | -3 | 439 | 3.76 | |
| G4 `batch_days` 10 | -26 | 303 | 3.17 | 303 (3.17) | -39 | 400 | 4.13 | 400 (4.13) |
| G5 `phys_days` 30 (likely: its fallback, -12) | -24 | 279 | 3.44 | 291 (3.30) | -23 | 377 | 4.38 | 388 (4.25) |
| **G2 nodes at least every 30 days** (with the decisions MI3a and MI4 would have cut) | **-73** | **206** | **4.66** | 218 (4.40) | **-121** | **256** | **6.45** | 267 (6.18) |
| MI3a habits (gated after G2) | -5 | 201 | 4.78 | 213 (4.51) | -6 | 250 | 6.60 | 261 (6.32) |
| MI4 place senses (gated after G2) | -1 | 200 | 4.80 | 212 (4.53) | -2 | 248 | 6.65 | 259 (6.37) |
| G3 the animal month (cost) | -9 | 191 | 5.03 | 203 (4.73) | -12 | 236 | 6.99 | 247 (6.68) |
| G9a, G9b stands at a season, compact | -10 | 181 | 5.30 | 193 (4.97) | -6 | 230 | 7.17 | 241 (6.85) |
| G12 liquid every 60 days (likely: half) | -13 | 168 | 5.71 | 187 (5.13) | -14 | 216 | 7.64 | 234 (7.05) |
| AIR4 calm months (research) | -32 | 136 | 7.06 | 155 (6.19) | -38 | 178 | 9.27 | 196 (8.42) |
| G11 f32 cell rows | -6 | 130 | 7.38 | 149 (6.44) | -9 | 169 | 9.76 | 187 (8.82) |
| **G14 physics beside the bodies** | -32 to -51 | **79 to 98** | **9.8 to 12.2** | **91 to 112 (8.6 to 10.5)** | -40 to -65 | **104 to 129** | **12.8 to 15.9** | **115 to 143 (11.5 to 14.3)** |

P1b, MI3b, G16, E4, E5 are counted as 0 here (P1b and MI3b are law steps; G16, E4 and E5 are small or gated).
Landing in another order moves milliseconds between rows, not the total. If MI3a and MI4 are not built (X36), their
6 / 8 ms stay; G16 takes 1 to 2 ms of them.

**How G14 is counted.** G14 cuts no work: it fills idle cores. The first bound (`gap.md` 2.2: all the parts' thread
time over 4 cores, times an imbalance guessed at 1.1 to 1.3) gave 75 to 88 ms (island) and 95 to 112 (globe). It left out
two things, now counted: (1) **the serial parts outside the two lanes**: the survey's "before" (about 10 ms island,
18 globe), Bond's month and the tape at month lines (about 11 and 14 ms), and the draws; (2) **lumpy windows**: every
third window carries Change, the state, the liquid and AIR4's 30-day step, so the lanes balance worse there. So:
**island 79 to 98 ms (9.8 to 12.2 times), globe 104 to 129 ms (12.8 to 15.9 times).** Two moves lower the serial
part, and the good end needs them: G6's per-domain survey runs in the bodies' lane, and the tape reads a frozen copy
beside the next window (G8). G14-P1 and G14-P2 time month windows and other windows apart and report the serial part; **their timing of the month
windows replaces the 1.1 to 1.3 imbalance guess**, and the review restates the G14 row from it.

### 5.2 One thread, instructions of a year (how most steps are judged)

Kept apart from 5.1: one-thread gains do not add up to the 4T year (the parallel steps gain nothing on one thread).

| Step | island22 1T | globe36 1T | Note |
|---|---|---|---|
| P1.1 to P1.4 | -9 to -11% | -4.5 to -5.5% | plants are 38% and 21% of a 1T year; a stand update 10 to 10.5 µs at 30 days (`month.md` 7) |
| P2 | -9 to -10% | -4.5% | a dearer update gains more for each update dropped |
| P3 | -5% | -3% | |
| P1b | -3 to -6% | -4 to -7% | an estimate (an animal update -10 to -20%); a law step; the body's reactor and today's upkeep, keep-warm and growth passes leave animal updates (`month.md` 5.1) |
| E1, E2, AIR2, G1, G6, G7, G8, G14, G15 | 0 to +3% | 0 to +3% | they share work out; they cost a little on one thread |
| AIR1 | about 0 | -2% (Flow -8% or more) | the island already needs 2.26 rounds |
| AIR3 | -1% | -1% | |
| MI1, MI5 | -1 to -2% | -2 to -3% (MI1 -0.3%) | `choose::act` is 11.6% and 17.8% of a 1T year (Z1) |
| MI3a, MI4 (after G2, gated) | -1 to -1.5% | -1 to -2% | after G2 a node's decisions are 0.28 to 0.31 of today |
| G2 | -25 to -35% | -30 to -40% | animal node updates x0.28 and x0.31 |
| G3 | about -2% | about -3% | gated after P1b (X49): only the heat is left |
| G4, G5 | -5% | -5% | faces x0.22; Change x0.4 |
| G9a, G9b, G12, AIR4, G11 | -3%, -2%, -2%, -6%, -2% | -2%, -1%, -2%, -10%, -3% | |

### 5.3 What reaches 10 times, and what is at risk

| | island22 | globe36 |
|---|---|---|
| needs | G2 at 30 days, G14 and AIR4, each at its good end, and stands near 8 µs | G2 at 30 days, and G14 past its low end |
| likely with everything (5.0, the span of both G2 cases) | 5.8 to 8.7 times: 10 times no longer likely | 7.6 to 12.4 times |
| at risk | G2: needs P1b's month to pass PR-A's 30-day check (PR-A2 looks for the rest); fallback the young-only floor (about 57% of G2's gain), only if P1b keeps the mineral (X48): 5.8 to 7.6 times; without G2: 4.8 to 6.3 | the same G2: 7.6 to 10.2 times; without G2: 6.1 to 8.1 |
| at risk | G14: the hardest step; G14-P1 saved 15% (globe 13%) today, G14-P2 decides | AIR4: its climate test (0.3 K) |
| at risk | G5 (its fallback is expected), G12 (not measured), the month's splitting test at 60 and 90 days (`month.md` 6) | the same |
| without G14 | 4.9 to 5.7 times | 6.6 to 8.0 times |
| without G14 and AIR4 | 4.2 to 4.8 times | 5.7 to 6.8 times |

## 6. Order and lanes

```
Phase P   R1 ─ R2 ─ R3, probes: PR-M, PR-B, PR-A, PR-S, G14-P1, AIR4-R1 ─► review (section 5 restated)
Phase 1   A: P1.1 ─ P1.2 ─ P1.4 ─ P2 ─ P1b (after PR-A2) ──────────────────────┐
          B: E1a ─ (MI1) ─ E1b.1 ─ E1b.2 ─ E1b.3 ─ E1c ─ E2 ───────────────────┤
          C: AIR1 ─────────────────────────── (E1c) ─ AIR3 ─ AIR2 ─────────────┤─► lane reviews (merge main)
          D: MI1 ─ MI5 ────────────────────────────────────────────────────────┤
          E: G8 ──────────────────────────── (E1c) ─ G1 ─ G7 ─ (E2) ─ G15 ─────┘
Phase 2   P3 ─ G13, G6 ;  P3 ─ G9a (also after G5) ─ G9b (gated)
          G4 ─ G5 ─ G9a ;  G2 ─ G3 ;  G2 ─ MI3a (gated) ─ MI4 (gated) ─ MI3b ;  G16 ;  G12 ;  AIR4 ─ G11 ;  E4, E5 (gated)
          G14-P2 ─ G14 (last) ;  GM, E7 (measures)            ─► review ─► merge main ─► Z
```

**Where steps collide, this order holds** (X23):
- MI1 lands before E1b.1 (both touch the faces).
- E1b.3 lands before P3 (P3 runs inside the new pipeline).
- E1c builds the one domain runner and the one `book_by_array` helper. AIR2, G1, G6, G7 use the runner; AIR3 and G1
  book through the helper. AIR2 comes after E1c and cuts its domains by air node. E4 never touches the air's book
  (one way for it: AIR3).
- G9a lands after G5 (its settings are multiples of `phys_days` 30 and `batch_days` 10; the reference is every survey,
  30 days).
- P1b lands before MI3b (a thought's cost is a drain in P1b's balance) and before G2. MI5 lands before G2 (both
  change `spread`). G2 lands before MI3a and MI4 (X36): their gates are measured again after it, and each is built
  only if its gate still holds; MI3b only if MI3a is built.
- Stale gates are measured again on the merged branch just before their step: E2's busiest share, E5's serial share,
  MI3a's sleep share, MI4's sharing.
- **`main` is merged into `speed/x10` at each lane review and before phase Z** (X1). If `main` re-pinned golden, the
  merge re-pins it on `speed/x10` in the same merge commit.
- Merges into `speed/x10` go one at a time; a level-3 step rebases and re-pins golden; `SNAPSHOT_VERSION` goes to the
  highest plus one at each merge. A review (adversarial Opus, then a fixer, until approved) runs after each lane, after
  phase 2 and after Z; each lane review runs the key-number test against the references.

## 7. Rule changes, and the step that writes each

| Rule | Lands with |
|---|---|
| `CLAUDE.md` 2.11 ceiling 12,000; 3.1 books are 64-bit; 6.1 threads do not change the world; handover 12.1, 12.3, 12.5, 14.5; handover 14.3 (12,000, past its 20% with the owner's explicit yes, X22) | R1 |
| `CLAUDE.md` 3.6 "one writer for each number" | E1c |
| `CLAUDE.md` 3.4 a tile takes at most its cells' share of its air node; handover 7.4 domains are tiles | E2 |
| `CLAUDE.md` 6.5 air tolerances are speed numbers | AIR1 |
| `CLAUDE.md` 2.6 `--full-minds` among the control runs; handover 6.5 habits, 7.5 | MI3a (if built, X36) |
| handover 6.5 a brain is paid per thought | MI3b (if built) |
| handover 7.1 stands, 7.3 population nodes; `laws.md` 6.10 (the month), 6.3 (stands at their ground's temperature, no sweating), 2.4 (a stand's reactions as rates over its window); `numbers.md` `field_days` 30 and `reserve_days` 36 (X44) | P1.2 (P2, G9a move `field_days`) |
| `laws.md` 6.2 one energy law for all bodies (keep-warm a drain in the upkeep; starving at a rate), 2.4 and 6.6 (every body's reactions and growth as rates through its interval) | P1b |
| `laws.md` 0, 4.4, 5.4; handover 7.3; `numbers.md` `node_days` 30; plan 2.2 rule 2 (a batch is 10 days; individuals and smart bodies may run sub-batches) | G2, G4 |
| `numbers.md` `batch_days` 10 (S64 replaced), `phys_days` 30, `flow_days` 60 | G4, G5, G12 |
| handover 7.3 the air's method; `numbers.md` `calm_tol` | AIR4 |
| `CLAUDE.md` 3.4: "When the physics runs beside the bodies, it works out its moves from the store at the window's start and books them after the bodies; each node's outgoing moves are cut in a fixed order, each against what the node holds after the ones before it, one share for each reaction or move as a whole, so no order of the two lanes can overdraw it."; handover 7.3 the step loop | G14 |

## 8. Risks

| Risk | Answer |
|---|---|
| The sum stays under 10 times | Said in section 1; each step is judged on its own kill line |
| P1.2 and P1b move the key numbers (S50, eaters) | Announced law changes: a deep famine kills `1 - e^-u`, not `u`; stands at their ground's temperature; no dip of an animal's body between updates. Stands keep `D` = 36 days (PR-M: the gene moved S50 past its noise, X44). Level 3 against the references after AIR4.0; a law change is its own commit with its effect |
| P1b's month does not pass PR-A's 30-day check | P1b still lands (X6); G2 takes the young-only floor if P1b keeps the mineral, else waits for the cause PR-A2 names (`month.md` 5.2) (X48); 20 days is no fallback (PR-A) |
| The month fails its splitting test at 60 or 90 days (the living body held at its start, the ramp's temperature) | a smaller `MONTH_SHARE`, a running total of the ground's temperature (`month.md` 2, 8); else P2 or G9a keeps the shorter pace |
| C2's second cause: children and maturing members wait for their node's update (P2, G2, G9a) | PR-A (e): births about 13.5 days late cost under 1% of the members in 2 years; the pace tests read births less deaths; the fallbacks are 30 days (stands), the young-only floor (animals) and 60 days (G9a) |
| Habits slow learning | The learning scene must pass; the answer is a wake on surprise, never a looser test |
| Tiles let a crowded tile run short of a scarce gas | E2 is level 3; the share per tile is `Mesh::share`; the E2 audit |
| G14's two lanes fight for memory, or its serial part stays large | G14-P1 and G14-P2 before the build, month windows timed apart, the serial part reported; G6 in the bodies' lane, the tape's frozen copy; the cut counter shows the lag |
| AIR4 fails its climate test | The island then needs G14 at its good end; the globe still passes with G14 |
| G5 grows S84's bias (expected: 75% of Change's rests are under 10 days) | Change keeps 10 days, the survey and state take 30; the likely column already counts this |
| Lanes collide in `survey.rs`, `spread.rs` and golden | Section 6's order; one merge at a time; `main` merged at each lane review and before Z |
| Snapshot bumps break the comparison of two builds | Section 2's judging method: turned saves (PR-S, X50) |

## 9. Lines

Core today: 9,602 code lines. Estimates from the designs (net): lane A +290 (P1.x +460, P1b -170; `month.md` 9), P3 +155, engine
+120, air +257 (AIR1 +12, AIR3 -10, AIR2 +35, AIR4 +140, G11 +80), minds +193, gap steps +490 (G14 +250, G3 +100, G1
+20, G2 +20, G4/G5/G12 +30, G6 +25, G7 +30, G13 +10, G15/G16 +5): about **+1,505, to 11,110**. The gated steps
(G9b +100, E4 +30, E5 +150) bring it to about 11,360; if the estimates run 20% over, about 11,800. So the ceiling is
**12,000** (X22, past handover 14.3's 20% with the owner's yes): each step names what it deletes; at the end of the
round the ceiling is set to the lines then plus 200. If MI3a is not built after G2 (X36), about 145 lines are not
written.
