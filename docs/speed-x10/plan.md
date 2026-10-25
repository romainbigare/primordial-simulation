# Speed x10: the plan

Status: plan, after the second adversarial review. Branch `speed/x10`. **This file is the single source of the
steps:** each kept step has here its description, check level, done-when, kill line, what it needs and its agent.
The designs in `design/` (`plants.md`, `animals.md`, `engine.md`, `air.md`, `minds.md`, `gap.md`) hold the detail;
each starts with a note of which of its steps are kept, dropped or renamed. Decisions: `decisions.md` (X1 and on).
Measures: `measures.md` (made in R2). History: `docs/history.md`, section 6.

## 1. The goal, and what to expect

**The goal:** a settled year on 4 cores 10 times faster than at the start of the last round: island22 0.910 s ->
0.091 s, globe36 1.490 s -> 0.149 s (`docs/speed-post-foundation/measures.md` Z1). Today 0.614 s and 0.738 s, so
6.7 times (island) and 5.0 times (globe) are still to find.

**The owner's rules for this round:** bad ideas are dropped from the start; every dropped strategy brings an
alternative (section 3); laws and rules may change for speed; the three rules for smart bodies hold (individuals stay
real; the step length per place; kinds of body code).

**What the plan can give** (section 5 has the table; all numbers are estimates from the designs until R2 measures
them again on one set of benches):

| Set of steps | island22 | globe36 |
|---|---|---|
| today | 1.5 times | 2.0 times |
| phase 1 and P3 (plants, engine, air, MI1 and MI5) | 2.4 | 3.1 |
| + the steps that change no law (G1, G6, G7, G8, G13, G15) and G4 | 3.2 | 4.2 |
| + all law and clock steps, without G14 | 7.4 | 9.9 |
| everything, with G14 (its bound counts the serial parts) | 9.8 to 12.2 | 13.1 to 16.3 |
| **likely: everything, with G5's fallback, half of G12, G14's bound** | **8.6 to 10.5** | **11.8 to 14.7** |
| everything but AIR4, with G14 | 8.0 to 10.0 | 10.9 to 13.6 |
| without G14 and without AIR4 | 5.9 | 8.1 |
| everything but G2 and G3, with G14 | 5.5 to 6.9 | 6.7 to 8.3 |

**Plainly:** **the island's 10 times needs every risky step to land near its good end**: G2 at 30 days, AIR4, G5
without its fallback, G14 near the low end of its bound, and a stand update of 6 µs (PR-M). The likely island is
about 8.5 to 10.5 times. **The globe is likely:** about 12 to 15 times; it reaches 10 times with G2 and either AIR4 or
G14. Two steps carry the risk: **G2** (every population node at least every 30 days) needs P1b first, and PR-A must
find what starves animal nodes at a long pace; **G14** (physics beside the bodies) is the hardest step of the round,
and G14-P decides early whether it can pay. If G14 fails, expect about **7.4 times** on the island and **9.9 times**
on the globe at best; if G14 and AIR4 both fail, about **6 and 8 times**. No claim is made for 16 cores: this machine
gives at most about 3 times on 4 cores (`gap.md` 1.3).

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
other build's save. They are judged (X24, method set by PR-S): (a) per update or per stand (µs, instructions) on each
build's own remade bench (S65); and (b) runs from worldgen with the same seed and both builds (`ew run --seed 1`,
the cost of years 150 to 200). A gain counts when both agree beyond PR-S's noise band. Parallel lanes each bump the
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
| 3 | Upkeep and light together | The month law for stands (P1.2) and for every body (P1b); D is the body's own gene interval (X14) | Today's law at a longer pace (C2): replaced by the stock equation |
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
| PR-M | Month prototype on a branch: the plant month (stages 3 to 6); µs per stand; its error against `body::live` on a grid (light, temperature, wetness, fuel fill) at 2 and 6 days; **and the stands' `D`**: runs with `D` = the gene interval against `D` = 36 days, and how far the key numbers move (X14) | none | a table in `measures.md`; the architect's choice of `D` for stands written in X14 | a stand over 4 µs, or over 5% error at 6 days: P1.2's design is reviewed before it starts. `D` = gene moves a key number out of its range, or S50 or S84 past its seeds' noise: stands take `D` = 36 (a `numbers.md` row) | R2 | Opus | plants 2.4, animals 3.1 |
| PR-B | Brain loss: a counter (on a branch) of resizes to no hidden unit, with members and parts held, in 1,000 years of island22 and globe36 | none | rows in `measures.md` | none | none | Sonnet (counter), Haiku (runs) | this plan, P1.2 |
| PR-A | What starves animal nodes at a 30-day pace with today's law, cause by cause | none | its table | none | R2 | Opus | animals 2 |
| PR-S | The judging method: a dummy `Config` field on a branch; the noise of per-update cost on own remade benches and of same-seed runs from worldgen | none | noise bands in `measures.md` | none | R2 | Sonnet (branch), Haiku (runs) | section 2 |
| G14-P1 | First overlap probe on today's code (branch): the air step and the batch joined (`rayon::join`, physics rows not booked; timing only). **Month windows** (Change, the state, the liquid) **and other windows are timed apart**, and the serial part outside the two lanes (survey, Bond's month, tape, draws) is reported | none | 4T year of both together against one after the other; the serial part in ms | under 15% saved: G14 is marked at risk in section 5 and the owner is told; G14-P2 decides | R2 | Sonnet | gap G14 |
| AIR4-R1 | Why 30-day Euler steps warm the winters: switch the explicit parts one at a time | none | a table of each part's share of the +1.2 K | none | R3 | Opus | air 2.4 |

**Review after phase P.** The architect restates section 5 with R2 and the probes. If PR-A and G14-P1 both fail,
the owner is told before phase 1 that the likely end is about 6 times (island) and 8 times (globe).

### Phase 1: five lanes in parallel

**Lane A, plants and animals** (`body/`, `copy/`, `params.rs`)

| ID | What | Level | Done when | Kill line | Needs | Agent | Detail |
|---|---|---|---|---|---|---|---|
| P1.1 | `body::apply_extent`: one helper books a reaction's extent on a body, cut out of `pay::pay_at` | 1 | golden unchanged with and without threads | not bit for bit: fix, never re-pin | R1 | Sonnet | plants 2.3 stage 5 |
| P1.2 | The month law for stands (stages 1 to 5 and 7; the heat still by the day law), the card, the stock's phases with the release limit (animals 3.1), booking order X29, `D` as PR-M chose it (X14), `node_days` -> `field_days` **30** (or 60; `Config::validate` wants a multiple of `batch_days`, and G4, G5 and P3 need 30, 60 or 90, X17); **the brain-loss switch** (below); tests 1 to 3 of plants 5 and a switch test; `laws.md` 6.10, `numbers.md`, handover 7.1, 7.3; snapshot bumped. **A law change, announced:** with `D` = gene, most stands keep a reserve 1.8 to 3.6 times smaller than today's 36 days (genes 20 and 10 days; 60 for the big slow kind), and their `INTERVAL` gene starts to matter to selection; the floor moves from 36 to 30 days | 3 | tests pass; books 1e-12 in the century; key numbers within the references (or new ones accepted by the owner); pace test 10, 30, 60; island 1T -10% | under 7% fewer island instructions, or key numbers out and not accepted | P1.1, PR-M, PR-B | Opus, adversarial review | plants 2.1 to 2.4, 3, 4 |
| P1.3 | The stand at its ground's temperature (stage 6): no own heat network, keep-warm or cooling for stands; the switch books its heat step | 3 | stand tests; R2's `|T_plant - T_ground|` in `measures.md`; key numbers; island 1T -2% | a climate test or a key number fails: stands keep the day law's heat (`relax_heat`) | P1.2 | Opus | plants 2.3 stage 6 |
| P1.4 | The sample by the month at every update; `sample_every` goes | 3 | `selection_inside_a_node_moves_its_mean_as_its_members_would`; `genes_do_not_drift_without_selection` (slow); island 1T not up over 1% | the selection test fails: `sample_every` stays 2 | P1.2 | Sonnet | plants 2.6 |
| P2 | `field_days` 60 | 3 | `the_pace_of_the_field_keeps_life` at 10 against 30, 60, 120; key numbers | the pace test fails at 60: 30 stays | P1.4 | Sonnet | plants 9 |
| P1b | The month law for every body: food as a pulse at the interval's start, the release limit, starving by a rate, the reserve by the gene, booking X29; acts stay after the update. **Keep-warm is a steady drain inside the upkeep `U`**: its power (the heat the surface sheds at the `KEEP_T` gene) is taken at the update's start, and the on-demand `keep_warm` pass after the balance goes, so a cold body neither grows from nor dies by a stock that keep-warm drains later (and then dies of frost in `check_stores`). **A law change, announced:** today the unpaid share `u` of an update dies at once; P1b kills `1 - e^-u` over a gene interval, so a deep famine kills fewer | 3 | the splitting test of the balance (60 days against 6 x 10, 2%; also at 260 K); key numbers at today's pace in their ranges, the shift from the famine rule reported as a law effect; PR-A again with P1b at a 30-day floor within the seed noise | key numbers out and not accepted: not merged. The 30-day check fails: P1b merges as the law (X6), G2 waits for the cause PR-A names or takes 20 days | P2, PR-A | Opus, adversarial review | animals 3 to 5 |

**The brain-loss switch (P1.2, P1.3, P3).** Plants are not a closed set: `lineage::resize` (`core/src/lineage.rs:69-85`)
moves a body whose `HIDDEN` gene falls below 0.5 (its range starts at -1, `body/genes.rs:105`) to a lineage with
no hidden unit, and `resize` already clears its trace, memory and signal (`lineage.rs:81-84`). From then on it is a
stand. The law changes at the resize: (1) from its next update it runs the month law (P1.2); (2) it lets go of any
part it holds or is inside (the `bond` helper a dying holder uses), at the resize; (3) it is scheduled at
`field_days` from that day (P1.2); (4) its heat steps to its ground's temperature, the difference booked with its
bodies' node and the ground as stage 6 books it (P1.3); (5) it leaves the ring (P3). The other way cannot happen
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
| AIR1 | Calm cells (`CALM_K`, `CALM_SHARE`) and the calm solver (`STILL_SHARE` 1e-4); `CLAUDE.md` 6.5 (air tolerances are speed numbers) | 3 (with the long-step and globe climate tests) | globe rounds at most 3.3; cell rows -25%; both climate tests; golden re-pinned; key numbers | Flow 1T under -8% on globe36 | R3 | Sonnet | air 2.1 |
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
| G9a | `field_days` 90 | 3 | the field's pace test at 10, 45, 90, 180; key numbers | fails at 90: 60 | P3 | Sonnet | gap G9 |
| G9b | Compact stands (P4). Gate: the handling of `Body` values (moves, clones) at least 5% of a year in P3's profile | 1 | stands 1T -10% | under -3% of a year on 4T | P3, G9a | Opus | plants P4, gap G9 |
| G4 | `batch_days` 10 (replaces S64). `field_days` must then be 30, 60 or 90 and `node_days` a multiple of 10 (`Config::validate`, X17). A small law effect: gene intervals from 5 to 15 days all become 10 days | 3 | the batch test (`the_batches_of_bodies_keep_life`) changes from 1 against 2 days to 2 against 10; key numbers; bodies' stage 4T -10% | under -6%: back to 2 | E1c | Sonnet, Opus review | gap G4 |
| G5 | `phys_days` 30; the air stays at 10 days (`field_days` 30, 60 or 90). **Expect the fallback:** 75% of Change's rests are under 10 days, and a 30-day span for every node grows S84's bias, so Change likely keeps 10 days and the rest takes 30; section 5's "likely" column counts only that part | 3 | long-step and globe climate tests at `phys_days` 30; the sleep test within noise; key numbers; year 4T -4% | a climate test fails or under -3%: 10, or Change at 10 and the rest at 30 | G4, G1 | Opus | gap G5 |
| G2 | Every population node updates at least every `node_days` 30; the movers (`min(1, p x paces)`), learning's discount (`discount^paces`, carried here, so MI3a is not needed), `away` (gene intervals) and an act's heat follow the pace; MI6 is never written. **The nodes check (30-day nodes against individuals at their gene) is a strong test of G2** | 3 | pace test for animals at 10, 30, 60; key numbers; nodes check; learning scene; bodies' stage 4T -25% on both | fails at 30: 20; fails at 20, or under -15%: revert | P1b, MI5 | Opus, adversarial review | gap G2, animals 5 |
| G3 | The animal month as a cost path for population nodes with a brain: the heat by `relax_heat`'s closed form (keep-warm is already a drain in P1b's balance; G3 keeps only the closed form of the heat); individuals and full minds keep the day law's heat | 3 | the splitting test for an animal node, also at 260 K; animal update 1T -30% | under -15%: keep P1b's law only | P1b, G2 | Opus | gap G3 |
| MI3a | Habits, still paying one decision per update; `--full-minds` (the control run); counter `thoughts`. **Gated after G2** (X36): on the merged branch after G2, the sleep share at least 40% on island and globe; likely gain 5 to 8 ms a year for about +145 net lines. Not built: `--full-minds` waits for the first smart kind, and G2 plus G16 stand in its place. `CLAUDE.md` 2.6 adds `--full-minds`; handover 6.5 habits bullet, 7.5; `laws.md` 5.3, 5.4, 5.6 | 3 | all tests; the learning scene; `habits_give_the_patterns_of_full_minds` (the full-minds check, slow) and the nodes check on this commit; key numbers; island and globe 1T -1% (after G2) | the gate fails after G2: not built. Under -0.7% on both, or the full-minds check fails beyond the noise: revert | G2, MI1, R2 (MI0) | Opus | minds 2.1, 2.5 |
| MI4 | Shared place senses per domain and survey. **Gated after G2** (X36): sharing at least 1.5, measured again on the merged branch; likely gain 1 to 2 ms | 2 | golden re-pinned; series within noise; island 1T -0.5% | the gate fails after G2: not built. Under -0.3% island and -0.2% globe: revert | G2 (MI3a if built) | Sonnet | minds 2.2 |
| MI3b | Pay per thought, its own law commit, only if MI3a is built: a thought's cost is a drain in P1b's balance over the interval; the commit gives its effect on the eaters (mass, members, births less deaths) | 3 | key numbers; nodes check; eaters within noise of MI3a, or the change reported as a law effect | the eaters leave their range and the owner does not accept | P1b, MI3a | Opus | minds 2.1 "Paying" |
| G12 | `flow_days` 60. Its gain is a guess until R3's clock fix lands (the probe's liquid ran monthly); section 5's "likely" column counts half | 3 | rivers, land rain, worn land in range; liquid -35% | a key number out: 30 | AIR2, R3 | Sonnet | gap G12 |
| AIR4 | Calm months (= G10). If AIR4-R1 names an explicit part, it is fixed first; **then a calm-month variant is always built and measured** (TR-BDF2 with its error estimate, against Euler with the fix) | 3 | long-step test within 0.3 K, globe test, key numbers, sleep test; at least 6 of 12 months calm on both | Flow 4T under x1.4 after AIR3, or a climate test fails | AIR4-R1, AIR1 to AIR3 | Opus | air 2.4 |
| G11 | f32 and SIMD for the air's cell rows (replaces AIR6) | 2 | climate tests; `pass` -35% | under -20%: revert, then AIR6 is tried as the alternative (gate: R2's share of cells that would group at least 40%) | AIR2 (after AIR4 if kept) | Opus | gap G11, air 2.6 |
| G16 | Well-being once per decision (with G2, the alternative to MI3a and MI4 if their gates fail, X36) | 3 | learning scene; nodes check | under 1% of the bodies' stage | G2, MI5 | Sonnet | gap G16 |
| E4 | `store::apply` per array for what is left on one core (the liquid's fluxes). Gate: `apply` at least 3% of a year on the main thread at 4T after AIR3 and G1 | 1 | golden unchanged with and without threads | year 4T under -2% | AIR3, G1 | Sonnet | engine 2.8 |
| E5 | Parts per tile. Gate, measured on the merged branch after G7 and G2: serial-body updates at least 10% on a bench, or S2 at least 15% of the bodies' stage | 3 | order test with parts; golden; key numbers | bodies' stage 4T under -8% on globe36 | E2, G7, G2 | Opus | engine 2.5 |
| G14-P2 | The overlap probe again, on the branch after G4, G5, AIR2, AIR3; month windows and other windows timed apart; the serial part reported | none | as G14-P1 | under 15% of a year saved: G14 is not built; the island's goal then rests on AIR4 and is reported at risk | G4, G5, AIR2, AIR3 | Sonnet | gap G14 |
| G14 | Physics beside the bodies: the physics works out its moves from the window's start and books them after the bodies. **Each reaction is cut as a whole:** one share per reaction, from its least held input against what the node then holds, applied to all its inputs, outputs and heat (a move of matter with its heat, as `hold_air`); never substance by substance. G1's rows are booked here, after the bodies (not redone). The survey's per-domain part (G6) runs in the bodies' lane; the tape reads a frozen copy beside the next window (G8). `CLAUDE.md` 3.4 sentence, handover 7.3, `laws.md` 0 | 3 | order test with the two lanes in both orders; books each month; a debug check: no row books on a node more than it holds, and every reaction keeps its proportions; the cut counter under 1e-3; climate tests; key numbers; year 4T -25% | under -15% on either bench, or the cut counter over 1e-3 | G14-P2, E1c, AIR2, AIR3, G1, G4, G5 | Opus, adversarial review | gap G14 |
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

### 5.1 Four threads, ms of a year (the goal's measure)

"Likely" is the same as "year" down to G4; from G5 on it counts G5's expected fallback (Change keeps 10 days: about
half of G5's gain), half of G12 (a guess until R3's clock fix lands and it is measured) and G14's bound with the serial
parts. Both columns take a stand update of 6 µs after P1.2 to P3, which rests on PR-M: if PR-M finds more, both fall.

| Step | island saves | island year | x | island likely (x) | globe saves | globe year | x | globe likely (x) |
|---|---|---|---|---|---|---|---|---|
| today | | 646 | 1.49 | | | 818 | 2.02 | |
| E1a to E1c (the serial phase) | -50 | 596 | 1.61 | | -76 | 742 | 2.22 | |
| MI1 (the cover of a face once a batch) | -3 | 593 | 1.62 | | -4 | 738 | 2.24 | |
| MI5 (acting; its landings after E1) | -4 | 589 | 1.63 | | -26 | 712 | 2.32 | |
| P1.1 to P1.4 (the month law) | -73 | 516 | 1.86 | | -40 | 672 | 2.46 | |
| P2 (`field_days` 60; its booking after E1) | -32 | 484 | 1.98 | | -22 | 650 | 2.54 | |
| AIR1, AIR3, AIR2 | -51 | 433 | 2.22 | | -83 | 567 | 2.91 | |
| E2 (tiles) | -18 | 415 | 2.31 | | -19 | 548 | 3.01 | |
| P3 (field step; faces and booking after E1) | -13 | 402 | 2.39 | | -16 | 532 | 3.10 | |
| **after phase 1 and P3** | | **402** | **2.39** | | | **532** | **3.10** | |
| G1 Change per domain | -32 | 370 | 2.59 | | -32 | 500 | 3.30 | |
| G7 Bond per domain | -4 | 366 | 2.62 | | -16 | 484 | 3.41 | |
| G8 tape on the pool | -8 | 358 | 2.68 | | -8 | 476 | 3.47 | |
| G6 survey per domain | -13 | 345 | 2.78 | | -20 | 456 | 3.62 | |
| G15 sharing between threads | -13 | 332 | 2.89 | | -17 | 439 | 3.76 | |
| G13 one draw per cell | -3 | 329 | 2.92 | | -3 | 436 | 3.78 | |
| G4 `batch_days` 10 | -26 | 303 | 3.17 | 303 (3.17) | -39 | 397 | 4.16 | 397 (4.16) |
| G5 `phys_days` 30 (likely: its fallback, -12) | -24 | 279 | 3.44 | 291 (3.30) | -23 | 374 | 4.41 | 385 (4.29) |
| **G2 nodes at least every 30 days** (with the decisions MI3a and MI4 would have cut) | **-73** | **206** | **4.66** | 218 (4.40) | **-121** | **253** | **6.52** | 264 (6.25) |
| MI3a habits (gated after G2) | -5 | 201 | 4.78 | 213 (4.51) | -6 | 247 | 6.68 | 258 (6.40) |
| MI4 place senses (gated after G2) | -1 | 200 | 4.80 | 212 (4.53) | -2 | 245 | 6.73 | 256 (6.45) |
| G3 the animal month (cost) | -9 | 191 | 5.03 | 203 (4.73) | -12 | 233 | 7.08 | 244 (6.76) |
| G9a, G9b stands at a season, compact | -10 | 181 | 5.30 | 193 (4.97) | -6 | 227 | 7.27 | 238 (6.93) |
| G12 liquid every 60 days (likely: half) | -13 | 168 | 5.71 | 187 (5.13) | -14 | 213 | 7.75 | 231 (7.14) |
| AIR4 calm months (research) | -32 | 136 | 7.06 | 155 (6.19) | -38 | 175 | 9.43 | 193 (8.55) |
| G11 f32 cell rows | -6 | 130 | 7.38 | 149 (6.44) | -9 | 166 | 9.94 | 184 (8.97) |
| **G14 physics beside the bodies** | -32 to -51 | **79 to 98** | **9.8 to 12.2** | **91 to 112 (8.6 to 10.5)** | -40 to -65 | **101 to 126** | **13.1 to 16.3** | **112 to 140 (11.8 to 14.7)** |

P1b, MI3b, G16, E4, E5 are counted as 0 here (P1b and MI3b are law steps; G16, E4 and E5 are small or gated).
Landing in another order moves milliseconds between rows, not the total. If MI3a and MI4 are not built (X36), their
6 / 8 ms stay; G16 takes 1 to 2 ms of them.

**How G14 is counted.** G14 cuts no work: it fills idle cores. The first bound (`gap.md` 2.2: all the parts' thread
time over 4 cores, times an imbalance of 1.1 to 1.3) gave 75 to 88 ms (island) and 95 to 112 (globe). It left out
two things, now counted: (1) **the serial parts outside the two lanes**: the survey's "before" (about 10 ms island,
18 globe), Bond's month and the tape at month lines (about 11 and 14 ms), and the draws; (2) **lumpy windows**: every
third window carries Change, the state, the liquid and AIR4's 30-day step, so the lanes balance worse there. So:
**island 79 to 98 ms (9.8 to 12.2 times), globe 101 to 126 ms (13.1 to 16.3 times).** Two moves lower the serial
part, and the good end needs them: G6's per-domain survey runs in the bodies' lane, and the tape reads a frozen copy
beside the next window (G8). G14-P1 and G14-P2 time month windows and other windows apart and report the serial part.

### 5.2 One thread, instructions of a year (how most steps are judged)

Kept apart from 5.1: one-thread gains do not add up to the 4T year (the parallel steps gain nothing on one thread).

| Step | island22 1T | globe36 1T | Note |
|---|---|---|---|
| P1.1 to P1.4 | -12% | -8% | plants are 38% and 21% of a 1T year |
| P2 | -7% | -3% | |
| P3 | -5% | -3% | |
| P1b | about 0 | about 0 | a law step |
| E1, E2, AIR2, G1, G6, G7, G8, G14, G15 | 0 to +3% | 0 to +3% | they share work out; they cost a little on one thread |
| AIR1 | about 0 | -2% (Flow -8% or more) | the island already needs 2.26 rounds |
| AIR3 | -1% | -1% | |
| MI1, MI5 | -1 to -2% | -2 to -3% | `choose::act` is 11.6% and 17.8% of a 1T year (Z1) |
| MI3a, MI4 (after G2, gated) | -1 to -1.5% | -1 to -2% | after G2 a node's decisions are 0.28 to 0.31 of today |
| G2 | -25 to -35% | -30 to -40% | animal node updates x0.28 and x0.31 |
| G3 | -5% | -7% | an animal update -30% |
| G4, G5 | -5% | -5% | faces x0.22; Change x0.4 |
| G9a, G9b, G12, AIR4, G11 | -3%, -2%, -2%, -6%, -2% | -2%, -1%, -2%, -10%, -3% | |

### 5.3 What reaches 10 times, and what is at risk

| | island22 | globe36 |
|---|---|---|
| needs | G2, G14 and AIR4, each near its good end | G2, and AIR4 or G14 |
| likely with everything | 8.6 to 10.5 times: 10 times only if every risky step lands near its good end | 11.8 to 14.7 times: likely |
| at risk | G2: needs P1b, and PR-A must find what starves nodes at 30 days (fallback 20 days: about 75% of G2's gain) | the same G2 |
| at risk | G14: the hardest step; its serial part; G14-P1 and G14-P2 decide early | AIR4: its climate test (0.3 K) |
| at risk | G5 (its fallback is expected), G12 (not measured), the stands' 6 µs (PR-M) | the same |
| without G14 | 7.4 times | 9.9 times |
| without G14 and AIR4 | 5.9 times | 8.1 times |

## 6. Order and lanes

```
Phase P   R1 ─ R2 ─ R3, probes: PR-M, PR-B, PR-A, PR-S, G14-P1, AIR4-R1 ─► review (section 5 restated)
Phase 1   A: P1.1 ─ P1.2 ─ P1.3 ─ P1.4 ─ P2 ─ P1b ─────────────────────────────┐
          B: E1a ─ (MI1) ─ E1b.1 ─ E1b.2 ─ E1b.3 ─ E1c ─ E2 ───────────────────┤
          C: AIR1 ─────────────────────────── (E1c) ─ AIR3 ─ AIR2 ─────────────┤─► lane reviews (merge main)
          D: MI1 ─ MI5 ────────────────────────────────────────────────────────┤
          E: G8 ──────────────────────────── (E1c) ─ G1 ─ G7 ─ (E2) ─ G15 ─────┘
Phase 2   P3 ─ G13, G6, G9a ─ G9b (gated)
          G4 ─ G5 ;  G2 ─ G3 ;  G2 ─ MI3a (gated) ─ MI4 (gated) ─ MI3b ;  G16 ;  G12 ;  AIR4 ─ G11 ;  E4, E5 (gated)
          G14-P2 ─ G14 (last) ;  GM, E7 (measures)            ─► review ─► merge main ─► Z
```

**Where steps collide, this order holds** (X23):
- MI1 lands before E1b.1 (both touch the faces).
- E1b.3 lands before P3 (P3 runs inside the new pipeline).
- E1c builds the one domain runner and the one `book_by_array` helper. AIR2, G1, G6, G7 use the runner; AIR3 and G1
  book through the helper. AIR2 comes after E1c and cuts its domains by air node. E4 never touches the air's book
  (one way for it: AIR3).
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
| handover 7.1 stands, 7.3 population nodes; `laws.md` 6.10; `numbers.md` `field_days` 30 (and the stands' `D` row if PR-M chose 36, X14) | P1.2 (P2, G9a move the value) |
| `laws.md` 6.2 one energy law for all bodies (keep-warm a drain in the upkeep; starving at a rate) | P1b |
| `laws.md` 0, 4.4, 5.4; handover 7.3; `numbers.md` `node_days` 30; plan 2.2 rule 2 (a batch is 10 days; individuals and smart bodies may run sub-batches) | G2, G4 |
| `numbers.md` `batch_days` 10 (S64 replaced), `phys_days` 30, `flow_days` 60 | G4, G5, G12 |
| handover 7.3 the air's method; `numbers.md` `calm_tol` | AIR4 |
| `CLAUDE.md` 3.4: "When the physics runs beside the bodies, it works out its moves from the store at the window's start and books them after the bodies; each node's outgoing moves are cut to what the node holds then, one share for each reaction or move as a whole, so no order of the two lanes can overdraw it."; handover 7.3 the step loop | G14 |

## 8. Risks

| Risk | Answer |
|---|---|
| The sum stays under 10 times | Said in section 1; each step is judged on its own kill line |
| P1.2 and P1b move the key numbers (S50, eaters) | Announced law changes: stands' reserve by the gene (PR-M chooses `D` = gene or 36, X14); a deep famine kills `1 - e^-u`, not `u`. Level 3 against R2's references; a law change is its own commit with its effect |
| PR-A finds a cause P1b does not fix | P1b still lands (X6); G2 takes 20 days or waits for that cause |
| C2's second cause: children and maturing members wait for their node's update (P2, G2, G9a) | PR-A (e) measures it; the pace tests read births less deaths; the fallbacks are 30, 20 and 60 days |
| Habits slow learning | The learning scene must pass; the answer is a wake on surprise, never a looser test |
| Tiles let a crowded tile run short of a scarce gas | E2 is level 3; the share per tile is `Mesh::share`; the E2 audit |
| G14's two lanes fight for memory, or its serial part stays large | G14-P1 and G14-P2 before the build, month windows timed apart, the serial part reported; G6 in the bodies' lane, the tape's frozen copy; the cut counter shows the lag |
| AIR4 fails its climate test | The island then needs G14 at its good end; the globe still passes with G14 |
| G5 grows S84's bias (expected: 75% of Change's rests are under 10 days) | Change keeps 10 days, the survey and state take 30; the likely column already counts this |
| Lanes collide in `survey.rs`, `spread.rs` and golden | Section 6's order; one merge at a time; `main` merged at each lane review and before Z |
| Snapshot bumps break the comparison of two builds | Section 2's judging method (PR-S) |

## 9. Lines

Core today: 9,602 code lines. Estimates from the designs (net): lane A +265 (P1.x +215, P1b +50), P3 +155, engine
+120, air +257 (AIR1 +12, AIR3 -10, AIR2 +35, AIR4 +140, G11 +80), minds +193, gap steps +490 (G14 +250, G3 +100, G1
+20, G2 +20, G4/G5/G12 +30, G6 +25, G7 +30, G13 +10, G15/G16 +5): about **+1,480, to 11,080**. The gated steps
(G9b +100, E4 +30, E5 +150) bring it to about 11,360; if the estimates run 20% over, about 11,800. So the ceiling is
**12,000** (X22, past handover 14.3's 20% with the owner's yes): each step names what it deletes; at the end of the
round the ceiling is set to the lines then plus 200. If MI3a is not built after G2 (X36), about 145 lines are not
written.
