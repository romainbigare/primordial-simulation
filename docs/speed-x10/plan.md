# Speed x10: the plan

Status: plan, after the second adversarial review, restated after phase P and after P1.2 (section 5.0). Branch
`speed/x10`. **This file is the single source of the steps:** each kept step has here its description, check level,
done-when, kill line, what it needs and its agent. The designs in `design/` (`lane-a.md`, `plants.md`, `animals.md`,
`engine.md`, `air.md`, `minds.md`, `gap.md`, and `month.md`, not kept) hold the detail; `lane-a.md` replaces
`month.md` and parts of `plants.md`, `animals.md` and `gap.md` (its note says which); each starts with a note of which
of its steps are kept, dropped or renamed. Decisions: `decisions.md` (X1 and on).
Measures: `measures.md` (made in R2). History: `docs/history.md`, section 6.

## 1. The goal, and what to expect

**The goal:** a settled year on 4 cores 10 times faster than at the start of the last round: island22 0.910 s ->
0.091 s, globe36 1.490 s -> 0.149 s (`docs/speed-post-foundation/measures.md` Z1). Today 0.614 s and 0.738 s, so
6.7 times (island) and 5.0 times (globe) are still to find.

**The owner's rules for this round:** bad ideas are dropped from the start; every dropped strategy brings an
alternative (section 3); laws and rules may change for speed; the three rules for smart bodies hold (individuals stay
real; the step length per place; kinds of body code).

**What the plan can give, after P1.2** (section 5.0; the designs' estimates are in 5.1 and are not repeated here).
Times against the baseline; "likely" counts G5's fallback, half of G12 and no MI3a (dropped, X67). The month law is not
kept (X62): plants run today's day law at a longer pace with C2's causes fixed inside it (PR-C2, P2.1, P2.2, G9a, X63),
and G2 floors the brain on today's day law (X66). **"Likely" is PR-C2's variant (b) (the stand's 36-day grid) at 60 and
90 days, with G2 at 30 days or its 20-day fallback.**

| Set of steps | island22 | globe36 |
|---|---|---|
| today | 1.5 times | 2.0 times |
| **likely, everything with G14 (-15 to -35% of the year; globe from -13%)** | **4.0 to 5.4** | **5.9 to 8.1** |
| of it, G2 at 30 days | 4.1 to 5.4 | 6.1 to 8.1 |
| of it, G2 at 20 days (X58) | 4.0 to 5.3 | 5.9 to 7.8 |
| PR-C2's variant (a) passes at 60 and 90 (no grid), G2 at 30 | 4.6 to 6.0 | 6.4 to 8.6 |
| stands at 60 only (no variant passes at 90) | 3.8 to 5.0 | 5.9 to 7.8 |
| no plant pace passes (P2, G9a, P3 not built) | 3.3 to 4.3 | 5.3 to 7.1 |
| without G2 | 3.8 to 5.0 | 5.2 to 7.0 |
| without G14 | 3.5 to 3.9 | 5.3 to 5.6 |

**Plainly:** **10 times is out of reach on both benches with the steps planned**: likely about 4.0 to 5.4 times on the
island and 5.9 to 8.1 on the globe. The month law cost more per stand than today's law (P1.2: about 26 µs against 18), so
the plants' gain must come from fewer updates of today's update, and the grid that keeps maturing and births on today's
cadence costs about 5 µs a grid point; G2 now saves the decision only (37% / 50% of an animal update, R2) on today's
body, x0.77 to 0.80 / x0.68 to 0.70 of the animal node updates' cost, not x0.39 / x0.42. G14-P1 saved 15% (island) and
13% (globe) of today's year, not the 25 to 39% its bound gave. The risks left: PR-C2 (C2 failed the pace test's x2 at 140
days); G14 (G14-P2 restates it after G4, G5, AIR2, AIR3); AIR4. No claim is made for 16 cores: this machine gives at
most about 3 times on 4 cores (`gap.md` 1.3).

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
| 1 | Plants as a field | Stands: the plant node of a cell, on today's day law at a pace of 60 days (P2.2), then 90 (G9a), then per cell in the survey, out of the ring (P3) | A new field type replacing plant bodies (X7): stands stay `Body` values. The month law (P1.2): not kept, it cost more per stand than today's law (X62); alternative: fewer updates of today's update (X63) |
| 2 | A growth table per genome | The card as it is (`Consts`, made in `prepare` at every change of the genes) | A grid table (X8): alternative the card; discrete strains (P5) only on a branch if the owner asks. The month's card: not built with the month (X62) |
| 3 | Upkeep and light together | Inside today's day law (P2.1, `design/lane-a.md` 2): the time's income pays the upkeep before members die, they die at a rate over 36 days, growth keeps 36 days of reserve; if needed, growth, maturing and births on the stand's 36-day grid; one law for every body | The month's stock equation (P1.2, P1b): not kept, its cost (X62, X65); alternative: C2's fixes inside the day law, probe PR-C2 first (X63, X64) |
| 4 | A different history with threads | Not needed: the history is fixed by the data with any number of threads; E1 shares out the crossing writes | An order-free pipeline (X3): alternative E1's fixed-order pipeline |
| 5 | 32-bit books | Books stay 64-bit; 32-bit only for numbers that hold nothing (G11 uses it) | A 32-bit store (X4): alternative G11 (f32 rates in the air's rows) |
| 6 | Fewer air steps; a coarser air grid | AIR1 to AIR3; calm months AIR4 (= G10); G11 | A coarser air grid (X10): alternative AIR1 to AIR3 and AIR4; AIR5 only for worlds of 2,000+ cells. Longer Euler air steps: alternative AIR4. AIR6 (calm cells share a row): alternative G11 (X27) |
| 7 | Raise the code ceiling | 12,000 code lines in `core` (X22) | 11,500 (too tight for the full set) |
| 8 | Think only on change | G2: a node decides about every 30 days and acts by its last decision's take between (a habit with the clock as its one wake rule, X66), plus G16 | MI6 (calm animals update less often): alternative G2 (X16). MI3a (habits with wake rules) and MI3b: dropped, MI3a's gate failed (R2) and G2 takes its place (X67) |
| 9 | Think in groups | A node's movers leave as one group (MI5); shared place senses (MI4) after G2, if its gate still holds (X36) | One decision for unlike nodes (a merge of unlike nodes, S66): alternative MI5 and MI4 (or G16 if MI4's gate fails) |
| 10 | Batched neural networks | Not built (X12) | Alternative: G2's habit cuts the nodes' thoughts (X66, X67); GM measures a thought for later brains |
| 11 | Full thinking for smart bodies | `--full-minds` waits for the first smart kind (MI3a is dropped, X67; G2's control is `think_days` 0); the mark of a smart lineage comes with the first smart kind (MI7) | `Config::full_hidden` now (MI8, X25): dead code, and a node that crossed it would open into about 10^4 individuals at once |

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
| PR-A2 | The rest of PR-A's loss after the eaten mineral: the adult nodes' root cause at a 30-day pace, and the smallest law change that removes it (branch `probe/pr-a2`). **Done** (`measures.md`): eating and maturing, not the body law; the segmented law (X55) | none | its table in `measures.md`; `month.md` 5.1 and 5.2 | none | PR-A | Opus | month 5.1, 5.2 |
| PR-S | The judging method: a dummy `Config` field on a branch; the noise of per-update cost on own remade benches and of same-seed runs from worldgen | none | noise bands in `measures.md` | none | R2 | Sonnet (branch), Haiku (runs) | section 2 |
| G14-P1 | First overlap probe on today's code (branch): the air step and the batch joined (`rayon::join`, physics rows not booked; timing only). **Month windows** (Change, the state, the liquid) **and other windows are timed apart**, and the serial part outside the two lanes (survey, Bond's month, tape, draws) is reported | none | 4T year of both together against one after the other; the serial part in ms | under 10% saved (X38): G14 is marked at risk in section 5 and the owner is told; G14-P2 decides | R2 | Sonnet | gap G14 |
| AIR4-R1 | Why 30-day Euler steps warm the winters: switch the explicit parts one at a time | none | a table of each part's share of the +1.2 K | none | R3 | Opus | air 2.4 |

**Review after phase P.** Done: section 5.0 restates section 5 with R2 and the probes (X51). PR-A failed (its causes
(a) to (d) win nothing) and G14-P1 passed on the line of X38 (15% and 13%), so the "both fail" message (6 and 8 times)
does not apply; the owner is told the restated end: island about 5.8 to 8.1 times, globe about 7.7 to 11.3 (G2 at 30
or 20 days on the segmented law, after PR-A2). The month law is redesigned after PR-M, PR-A, PR-A2 and its reviews
(`design/month.md`, X41 to X53, X55 to X59). **After P1.2** (not kept, X62) lane A is replanned (`design/lane-a.md`,
X62 to X69) and section 5.0 restated again: island about 4.0 to 5.4 times, globe about 5.9 to 8.1.

### Phase 1: five lanes in parallel

**Lane A, plants and animals** (`body/`, `copy/`, `choose/`, `survey::update`, `params.rs`; `design/lane-a.md`)

| ID | What | Level | Done when | Kill line | Needs | Agent | Detail |
|---|---|---|---|---|---|---|---|
| P1.1 | **Done (merged).** `body::apply_extent`: one helper books a reaction's extent on a body, cut out of `pay::pay_at`; it stays `pay_at`'s helper | 1 | golden unchanged with and without threads | not bit for bit: fix, never re-pin | R1 | Sonnet | plants 2.3 stage 5 |
| P1.2 | **Not kept** (X62; `measures.md` "P1.2"; branch `x10/p1.2-not-kept`): the month law for stands cost about 26 µs a stand update against 18 today, so a stand-year cost more than today's at 30, 60 and 90 days (about 316, 225, 200 µs against 182). **Alternative:** PR-C2, then P2.1 and P2.2 (today's day law at a longer pace, C2's causes fixed inside it, X63) | | | | | | lane-a 1 |
| P1.3 | **Not kept** with P1.2 (it was folded into it, X45). Stands keep their own temperature, as today | | | | | | |
| P1.4 | **Dropped** (X62): its base was the month's cheap sample. Today's sample stays (a clone run, `sample_every` 2); if the selection test fails at a longer pace, P2.2 sets `sample_every` 1 (about +2.3 µs an update) | | | | | | |
| PR-C2 | **Probe** (branch `probe/pr-c2`, switches off by default): today's day law with C2's causes fixed (P2.1's three changes), (a) alone, then (b) with growth, maturing and children on the stand's 36-day grid; stands at 60 and 90 days. The splitting test of the day law (1 x 60 against 6 x 10; 1 x 90 against 9 x 10 and 3 x 30; young and adult stands, constant forcing, a seasonal ramp, the summer's peak and the winter's low; members and body matter, fuel reported) beside today's law; the pace test for stands (gene, 60, 120, 30; then 90's); key numbers once a variant that passes; counters (deaths by unpaid upkeep, births and young that mature per member-day, stand updates where the light catalyst's limit binds); the plant nodes at year 30; instructions per stand update and stand-year at 36, 60, 90 on one save | none | a table in `measures.md`; the variant and pace chosen | no variant passes the pace test at 60 with its x2 and /2, or the splitting line (members and body matter: 90th percentile under 5%, and no worse than today's law at 1 x 36 against 6 x 6), or the key numbers: stands stay at 36 days on today's law; P2.1, P2.2, G9a and P3 are not built. A stand-year at 60 over x0.85 of today's: the same | none | Opus | lane-a 2.3 |
| P2.1 | **The law, at today's pace** (X63, one law for every body): upkeep the stock cannot pay is owed and paid after `physics` from what the time brought (C2's `pay_owed`); the unpaid share `u` kills `1 - e^(-u Δ / D)`, `D` = `Physics::reserve_days` 36 for a stand, the gene interval for a body with a brain; a stand's growth reserve and `buildable`'s horizon read `D`; with PR-C2's variant (b), growth in pieces cut at the stand's 36-day grid points (`d mod 36 = id mod 36`), each point maturing the young stand and making the adult's children. **A law change, announced:** a famine kills `1 - e^-u`, not `u`; a stand pays its upkeep from the time's light before members die; the effect on light-eaters, S50 and births less deaths reported. `laws.md` 6.2, 6.6, 4.4; `numbers.md` `reserve_days`; snapshot bumped | 3 | PR-C2's splitting test as a kept test (`the_day_law_splits_alike`, four points, the line of PR-C2); a starving stand dies at its own time in one update and in six; books 1e-12 in the century; key numbers within the references, S50 not past its noise unless explained | key numbers out and not accepted | PR-C2 | Opus, adversarial review | lane-a 2.2 |
| P2.2 | **`node_days` becomes `field_days` 60** (stands' pace; `Config::validate` wants a multiple of `batch_days`, and P3 of `phys_days`, X17). `numbers.md` `field_days`; handover 7.1, 7.3; snapshot bumped | 3 | the pace test for stands at gene, 60, 120, 30; `selection_inside_a_node_moves_its_mean_as_its_members_would`; key numbers; per stand-year at most x0.82 (variant (b)) or x0.65 (variant (a)) of today's on one world (X50), island22 1T about -6% ((b)) or -12% ((a)) | the pace test fails at 60: 36 stays (P3 then not built); island 1T under -4% | P2.1 | Sonnet | lane-a 2.3 |
| P1b | **Dropped** (X65): its base was the month law; G2 no longer needs it (the body keeps today's law at its gene's pace, X66); one energy law for every body holds with P2.1. **Alternative:** G2 on today's law | | | | | | lane-a 3.2 |
| G2 | **The brain floored, the body at its gene's pace, on today's day law** (X66, `lane-a.md` 3): a population node with a brain keeps its gene's update and runs today's body update each time; it decides at the update nearest to `Config::think_days` 30 after its last decision; at its other updates the share of its members that took from the ground at its last decision (`p[TAKE]`, saved) takes again as `spread` does for that one option, with no perception, network or learning; moves, body takes, gifts and bond acts come at decisions only, where the movers (`count x min(1, Σp x paces)`, MI5's one group), learning's discount (`discount^paces`) and `away` (gene intervals) follow the paces since the last decision; upkeep pays one decision per update, as today; MI6 is never written; snapshot bumped. **The floor (X58):** 30 days; else 20; else young nodes only; else revert. `laws.md` 0, 4.4, 5.4; handover 7.3; `numbers.md` `think_days` | 3 | PR-A2's script with G2 at 30 days ((g) off; 20 for the fallback): the eaters within the seed noise of `think_days` 0, PR-A2's counters near it, the globe's body takes reported; the animals' pace test (`think_days` 0, 15, 30, 60); nodes check; learning scene; key numbers; animal node updates x0.80 (island) and x0.70 (globe) or less, 1T year -6% and -10% (20 days: -5% and -8%; young only: -3.5% and -6%) on one world (X50) | under -5% island and -8% globe (30 days), -4% and -6.5% (20 days), -2.5% and -4.5% (young only), or the check fails at every floor: revert | MI5, PR-A2 | Opus, adversarial review | lane-a 3 |

**The brain-loss switch (X15, X62).** Plants are not a closed set: `lineage::resize` (`core/src/lineage.rs:69-85`) moves
a body whose `HIDDEN` gene falls below 0.5 to a lineage with no hidden unit, and clears its trace, memory and signal.
With the day law kept for every body, today's code already makes it a stand: `Body::interval` paces a brainless node by
the stands' setting, and P2.1 gives it the window `reserve_days`. P3 adds its ring exit and the test
`a_body_that_loses_its_brain_becomes_a_stand` (a node with `HIDDEN` at 0.4 resized: its next update is a stand's, it
leaves the ring, books kept). PR-B saw no brain loss in 1,000 years of island22 and globe36.

**Lane B, engine** (`outbox`, `life/`, `survey::batch`, `store/`, `par.rs`)

| ID | What | Level | Done when | Kill line | Needs | Agent | Detail |
|---|---|---|---|---|---|---|---|
| E1a | `pushed` and `crossings` per domain; booking and delivering only the domains touched | 1 (series bit for bit; fingerprint re-pinned for the snapshot format) | golden with and without threads; 2 years of each bench, same `series.csv` | the parts' or bodies' stage slower (1T) | R1 | Sonnet | engine 2.3 |
| E1b.1 | Faces per domain, made at the end of the domain's own phase; a domain changed later marks its faces stale | 1 | golden unchanged | bodies' stage 4T slower | E1a, MI1 | Sonnet | engine 2.2 |
| E1b.2 | Landing by destination: leaving bodies taken out at P1's end, outboxes in buckets, global ids in S1, landing per destination in P2 | 3 | the order test (widened) equal; golden re-pinned, the same without threads; key numbers once | bodies' stage 4T under -6% on two benches, or worse on the third | E1b.1 | Opus | engine 2.2, 2.3 |
| E1b.3 | **Folded into E1b.2** (X61): watches, part acts, returns of litters, lane clears and pushes on structures already run after all landings, one by one in a fixed order | | | | | | engine 2.2 |
| E1c | Booking by array in P2's call: **the one helper `store::book_by_array`** (AIR3, G1, E4 use it); **one domain runner** (LPT order, scratch per thread with `each_init`, `SERIAL_BELOW`) that AIR2, G1, G6, G7 use | 2 | golden re-pinned; the series to rounding | bodies' stage 4T under -2% on top of E1b on two benches | E1b.3 | Opus | engine 2.2 |
| E2 | Tiles of 4 cells with the air column. First, on the merged branch: the busiest share at 4 cells (gate: at most 0.6 x today's on two benches). Audit: no body code reads `Class::Air` straight from the store (only through the view); the shares of an air node's cells sum to 1 (a debug assert). `CLAUDE.md` 3.4 tile sentence, handover 7.4 | 3 | order test; golden; key numbers; per update 1T at most +3% | bodies' stage 4T under -5% on two benches and under -8% on region24 | E1c | Opus | engine 2.4 |

**Lane C, air** (`flow/`, `solve/`)

| ID | What | Level | Done when | Kill line | Needs | Agent | Detail |
|---|---|---|---|---|---|---|---|
| AIR1 | Calm cells (`CALM_K`, `CALM_SHARE`) and the calm solver (`STILL_SHARE` 2e-4, X39); `CLAUDE.md` 6.5 (air tolerances are speed numbers) | 3 (with the long-step and globe climate tests) | globe rounds at most 3.3; cell rows -25%; both climate tests; golden re-pinned; key numbers | Flow 1T under -8% on globe36 | R3 | Sonnet | air 2.1 |
| AIR3 | The air's book as column moves, written through E1c's `book_by_array` (no own pass); E4 does not touch the air | 1 or 2 | `book` -40% | under -25% | E1c, AIR1 | Sonnet | air 2.3 |
| AIR2 | The air's and the liquid's per-cell loops in phases on E1c's runner, the domains **cut by air node** (the tiles of one node in one task), partials per air node summed in node order | 2 | golden re-pinned; 5-year runs: climate columns within 0.05 K and 1%; Flow 4T x1.25 on both | under x1.15 on either | E1c, AIR3 | Opus | air 2.2 |

**Lane D, minds** (`choose/`, `brain/`). MI4 comes in phase 2, after G2 (X36); MI3a and MI3b are dropped (X67).

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
| P3 | The field step per cell in the survey; stands out of the ring. `field_days` must be 30, 60 or 90 (a multiple of `phys_days`, X17). Files: `survey::survey`; `Life::unsound` (stands are in no ring); `copy::place_child` and `Domain::add` (no ring entry for a stand); `Body::interval` (its `n() > 1` test: every stand by `field_days`); the snapshot reader (`life/mod.rs:118`); the brain-loss switch leaves the ring, and its test | 3 | plants test 4; key numbers; island 1T -5% on top of P2.2 | under -5% | P2.2, E1b.3, E1c | Opus | plants 2.5 |
| G13 | One heat draw per cell for stands | 2 | `Draws` and the light marks loop over cells for stands | under 2 ms a year | P3 | Sonnet | gap G13 |
| G6 | The whole survey per domain on E1c's runner (in G14 it runs in the bodies' lane) | 2 | survey 4T -35% on both | under -20% | P3, E1c | Sonnet | gap G6 |
| G9a | `field_days` 90, on the day law and PR-C2's variant that passed at 90. **Comes after G4 and G5.** The test's settings are 30, 60, 90 and 180, all multiples of `phys_days` 30 and of `batch_days` 10 | 3 | PR-C2's splitting test at 90 (1 x 90 against 3 x 30); the field's pace test at 30 (the reference: every survey, once `phys_days` is 30), 60, 90, 180; key numbers; per stand-year at most x0.65 ((b)) or x0.42 ((a)) of today's on one world, island 1T -4.5% ((b)) or -6% ((a)) | PR-C2 had no variant pass at 90, or the pace test fails at 90, or island 1T under -3%: 60 | P3, G5 | Sonnet | lane-a 2.3 |
| G9b | Compact stands (P4). Gate: the handling of `Body` values (moves, clones) at least 5% of a year in P3's profile (if P3 is not built: in today's profile) | 1 | stands 1T -10% | under -3% of a year on 4T | P3 (or the gate on today's code), G9a | Opus | plants P4, gap G9 |
| G4 | `batch_days` 10 (replaces S64). `field_days` must then be 30, 60 or 90, a multiple of 10 (`Config::validate`, X17). A small law effect: gene intervals from 5 to 15 days all become 10 days | 3 | the batch test (`the_batches_of_bodies_keep_life`) changes from 1 against 2 days to 2 against 10; key numbers; bodies' stage 4T -10% | under -6%: back to 2 | E1c | Sonnet, Opus review | gap G4 |
| G5 | `phys_days` 30; the air stays at 10 days (`field_days` 30, 60 or 90). **Expect the fallback:** 75% of Change's rests are under 10 days, and a 30-day span for every node grows S84's bias, so Change likely keeps 10 days and the rest takes 30; section 5's "likely" column counts only that part | 3 | long-step and globe climate tests at `phys_days` 30; the sleep test within noise; key numbers; year 4T -4% | a climate test fails or under -3%: 10, or Change at 10 and the rest at 30 | G4, G1 | Opus | gap G5 |
| G2 | **Moved to lane A** (X66): it now runs on today's day law and needs only MI5 and PR-A2 | | | | | | lane-a 3 |
| G3 | **Dropped** (X68): it was the closed form of the month's heat for animals; its base (P1b) is gone. **Alternative:** the measure GB below, and a step only if it names a stage | | | | | | lane-a 3.2 |
| MI3a | **Dropped** (X67): its gate failed (R2: sleep 14% / 11% against 40%), and G2 is its clock-only form for nodes; after G2 it could only add habits for individuals (2% of the decisions) and wake nodes earlier. `--full-minds` waits for the first smart kind (X25). **Alternative:** G2 itself, plus G16 | | | | | | lane-a 3.2 |
| MI4 | Shared place senses per domain and survey. **Gated after G2** (X36): sharing at least 1.5, measured again on the merged branch; likely gain 1 to 2 ms | 2 | golden re-pinned; series within noise; island 1T -0.5% | the gate fails after G2: not built. Under -0.3% island and -0.2% globe: revert | G2 | Sonnet | minds 2.2 |
| MI3b | **Dropped** (X67): it needed MI3a; a brain stays paid once per update, as today | | | | | | |
| G12 | `flow_days` 60. Its gain is a guess until R3's clock fix lands (the probe's liquid ran monthly); section 5's "likely" column counts half | 3 | rivers, land rain, worn land in range; liquid -35% | a key number out: 30 | AIR2, R3 | Sonnet | gap G12 |
| AIR4 | Calm months (= G10). If AIR4-R1 names an explicit part, it is fixed first; **then a calm-month variant is always built and measured** (TR-BDF2 with its error estimate, against Euler with the fix) | 3 | long-step test within 0.3 K, globe test, key numbers, sleep test; at least 6 of 12 months calm on both | Flow 4T under x1.4 after AIR3, or a climate test fails | AIR4-R1, AIR1 to AIR3 | Opus | air 2.4 |
| G11 | f32 and SIMD for the air's cell rows (replaces AIR6) | 2 | climate tests; `pass` -35% | under -20%: revert, then AIR6 is tried as the alternative (gate: R2's share of cells that would group at least 40%) | AIR2 (after AIR4 if kept) | Opus | gap G11, air 2.6 |
| G16 | Well-being once per decision (with G2, the alternative to MI3a, X67, and to MI4 if its gate fails, X36) | 3 | learning scene; nodes check | under 1% of the bodies' stage | G2, MI5 | Sonnet | gap G16 |
| E4 | `store::apply` per array for what is left on one core (the liquid's fluxes). Gate: `apply` at least 3% of a year on the main thread at 4T after AIR3 and G1 | 1 | golden unchanged with and without threads | year 4T under -2% | AIR3, G1 | Sonnet | engine 2.8 |
| E5 | Parts per tile. Gate, measured on the merged branch after G7 and G2: serial-body updates at least 10% on a bench, or S2 at least 15% of the bodies' stage | 3 | order test with parts; golden; key numbers | bodies' stage 4T under -8% on globe36 | E2, G7, G2 | Opus | engine 2.5 |
| G14-P2 | The overlap probe again, on the branch after G4, G5, AIR2, AIR3; month windows and other windows timed apart; the serial part reported | none | as G14-P1 | under 10% of a year saved (X38): G14 is not built; the island's goal then rests on AIR4 and is reported at risk | G4, G5, AIR2, AIR3 | Sonnet | gap G14 |
| G14 | Physics beside the bodies: the physics works out its moves from the window's start and books them after the bodies. **Each reaction is cut as a whole, in a fixed order, each against what is left after the ones before it** (as X29): one share per reaction, from its least input against that rest, applied to all its inputs, outputs and heat (a move of matter with its heat, as `hold_air`); never substance by substance. G1's rows are booked here, after the bodies (not redone). The survey's per-domain part (G6) runs in the bodies' lane; the tape reads a frozen copy beside the next window (G8). `CLAUDE.md` 3.4 sentence, handover 7.3, `laws.md` 0 | 3 | order test with the two lanes in both orders; books each month; a debug check on the running total: no row books on a node more than is left after the rows before it, and every reaction keeps its proportions; the cut counter under 1e-3; climate tests; key numbers; year 4T -25% | under -10% on either bench (X38), or the cut counter over 1e-3 | G14-P2, E1c, AIR2, AIR3, G1, G4, G5 | Opus, adversarial review | gap G14 |
| GM | The cost of a thought at 12 and 64 hidden units, and of a year at `think_days` 0 (every update thinks) | none | a row in `measures.md` | none | G2 | Haiku | gap G16 |
| GB | The animal node updates' profile by stage after G2 (G3's alternative, X68): the body's day by stage, the habit's take, the decision | none | a row in `measures.md`; a step is designed only if one stage of the body's day is at least 25% of an animal node update | none | G2 | Haiku | lane-a 3.2 |
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

### 5.0 The review after phase P, and after P1.2

R2 and the probes are measured (`measures.md`), and P1.2 was built and not kept. 5.1 to 5.3 below are the designs'
estimates; this section says where the measures move them, and its end is the plan's expected result (X51, X59, X69).

| Measure | What it says | What moves in section 5 |
|---|---|---|
| R2 | the shares hold: plant updates 30.0% / 14.7% of a one-thread year, a 30-day floor keeps 0.30 / 0.31 of the animal node updates; the machine runs 4 to 14% slower than Z1's hour; MI3a's gate fails (sleep 14% / 11%, the line 40%) | the table keeps Z1's scale; MI3a and MI4 are not counted in "likely" (their 6 / 8 ms stay; G16 takes 1 to 2) |
| E1a | neutral on one thread (+0.04% of the batch), as expected | nothing: the chain E1a to E1c is judged at E1c (X40) |
| AIR1 | globe: Flow -11.7%, the year -3.6% on one thread (5.2 said -2%); island and region about 0 | the air's row as planned, a little better on the globe |
| G14-P1 | the join saves 15% (island) and 13% (globe) of today's year; the lanes' imbalance is 1.6 to 2.0 (the bound guessed 1.1 to 1.3); the serial part 57 / 75 ms; the saving grows as the bodies' lane shrinks and the physics' share rises; over the line of X38 (10%), so G14-P2 decides | G14: -15% (island) and -13% (globe) at the low end, -35% at the high end, of the year it meets (the bound: -25 to -39%) |
| PR-M | the stand's law 7.0 µs without bare reactions; with them and lean stages about 4.5 to 5 µs; a whole stand update about 10 to 10.5 µs at 30 days, 9 after P3 (5.1 took 6); the month failed its own splitting test until the fixes of `month.md`; `D` = 36 for stands | the plants' rows, below |
| PR-A | (a) to (d) win nothing; the eaten mineral turns solid before growth; 20 days fails too under today's law | P1b keeps the mineral (the sub-steps) |
| PR-A2 | the loss is not in the body law: it is eating (one 30-day take from what lies) and maturing (young wait up to a month); takes and maturing at the gene's pace bring the 30-day node back within the noise (+0.015 / +0.011 / +0.001), 20 days too on the globe and the region; a balance exact in time moves today's eaters by -0.04 to -0.16 | segments (X55); P1b's law effect accepted (X56); G2 floors the brain only, x0.39 / x0.42 (X57); fallbacks 20 days, then young-only, on the segmented law (X58); "likely" narrowed to G2 at 30 or 20 days (X59) |
| PR-S | across different worlds a gain counts only above 19% (island) and 11% (globe); on one world above 0.1%, and a law step's own year moves the world by up to about 1.3% | law steps are judged per update and per stand-year on turned saves, whole years only above 2% (X50); no gain moves |
| AIR4-R1, AIR4.0 | the plateau fault is fixed; TR-BDF2 lands near the 0.3 K limit | AIR4 stays a research step at risk |
| P1.2 | the month law for stands: about 26 µs a stand update against 18 today (173k against 121k instructions), so a stand-year never beats today's ~182 µs (about 316 at 30 days, 225 at 60, 200 at 90); land -0.78 K, S50 0.636, not explained | not kept (X62): P1.x, P1b, G3 and MI3a leave the rows; plants: today's day law at 60 and 90 days with C2's causes fixed (PR-C2, X63); G2 on today's law (X66); the end restated (X69) |

**The rows that move** (four threads, ms a year saved, Z1's scale, in the order of 5.1). "After P1.2" counts plant
updates at about 139 ms (island) and 77 ms (globe) a year at today's ~182 µs a stand-year (the rows' own base: P1.x's
-43 / -24 at x0.69), and animal node updates at about 100 and 170 ms (G2's base in 5.1, -73 at x0.28 and -121 at x0.31):

| Step | island, 5.1 | island, after phase P | island, after P1.2 | globe, 5.1 | globe, after phase P | globe, after P1.2 | Why (after P1.2) |
|---|---|---|---|---|---|---|---|
| P1.1 to P1.4 | -73 | -43 | 0 | -40 | -24 | 0 | not kept (X62) |
| P2 (60 days) | -32 | -45 | -31 ((b)); -55 ((a)) | -22 | -31 | -17 ((b)); -31 ((a)) | today's update at 60 days: a stand-year x0.79 with the grid, x0.60 without (`lane-a.md` 2.3) |
| P3 | -13 | -13 | -13 | -16 | -16 | -16 | unchanged; mostly faces and the ring |
| P1b | 0 | -11 | 0 | 0 | -15 | 0 | dropped (X65) |
| G2 at 30 days | -73 | -55 | -20 to -23 | -121 | -88 | -51 to -54 | the decision only, on today's body: x0.77 to 0.80 / x0.68 to 0.70 of the node updates' cost (X66) |
| MI3a, MI4 | -6 | -2 | -2 | -8 | -2 | -2 | MI3a dropped (X67); MI4 gated, G16 |
| G3 | -9 | -4 | 0 | -12 | -6 | 0 | dropped (X68) |
| G9a, G9b | -10 | -10 | -22 ((b)); -27 ((a)) | -6 | -6 | -12 ((b)); -15 ((a)) | 90 days: a stand-year x0.62 with the grid, x0.40 without; G9b gated, not counted |
| G14 | -32 to -51 | -15 to -35% | -15 to -35% | -40 to -65 | -13 to -35% | -13 to -35% | G14-P1 (its low end per bench) |

**The end, restated after P1.2.** Without any lane A step but P3 and MI4 the year before G14 is about 350 ms (island)
and 394 ms (globe). With PR-C2's variant (b) at 60 and 90 days and G2 at 30 days it is about 276 and 313 ms; at 20 days
about 280 and 324; with variant (a) about 247 and 296; with stands at 60 only about 298 and 325; with no plant pace (P3
not built either) about 342 and 358; without G2 about 297 and 365. With G14 at -15 to -35% (island) and -13 to -35%
(globe):

| Case | island22 | globe36 |
|---|---|---|
| variant (b) at 60 and 90, G2 at 30 days | 179 to 235 ms, 4.1 to 5.4 times | 203 to 272 ms, 6.1 to 8.1 times |
| variant (b) at 60 and 90, G2 at 20 days | 182 to 238 ms, 4.0 to 5.3 times | 211 to 282 ms, 5.9 to 7.8 times |
| **likely: the span of the two** | **4.0 to 5.4 times** | **5.9 to 8.1 times** |
| variant (a) at 60 and 90, G2 at 30 days | 160 to 210 ms, 4.6 to 6.0 times | 192 to 257 ms, 6.4 to 8.6 times |
| stands at 60 only, G2 at 30 days | 193 to 253 ms, 3.8 to 5.0 times | 211 to 282 ms, 5.9 to 7.8 times |
| no plant pace passes (P2, G9a, P3 not built), G2 at 30 days | 222 to 290 ms, 3.3 to 4.3 times | 233 to 311 ms, 5.3 to 7.1 times |
| without G2 (variant (b)) | 193 to 252 ms, 3.8 to 5.0 times | 237 to 318 ms, 5.2 to 7.0 times |
| without G14 (variant (b) or (a), G2 at 30 days) | 3.5 to 3.9 times | 5.3 to 5.6 times |

Where it went: the month's plan counted lane A's steps at about 168 ms (island) and 170 (globe); now about 75 and 82 in
the likely case (plants 53 and 29, G2 22 and 53). The plants keep most of their gain (fewer updates of today's update
in place of a cheaper update at 30 days), but the grid that keeps maturing and births on today's cadence costs about 5
µs a grid point. G2 loses most of its: it saves the decision (37% / 50% of an animal update), not the body's day,
which the month would have made cheaper.

What would lift it: PR-C2's variant (a) passing at 60 and 90 (no grid); G14 at the good end of its range (G14-P2 after
G4, G5, AIR2, AIR3); AIR4 without its climate risk; a cheaper body's day at its gene's pace (GB says where), as a step
of its own. **The owner is told:** 10 times is out of reach on both benches with the steps planned: likely about 4.0 to
5.4 times on the island and 5.9 to 8.1 on the globe.

### 5.1 Four threads, ms of a year (the goal's measure)

"Likely" is the same as "year" down to G4; from G5 on it counts G5's expected fallback (Change keeps 10 days: about
half of G5's gain), half of G12 (a guess until R3's clock fix lands and it is measured) and G14's bound with the serial
parts. Both columns take a stand update of 6 µs after P1.2 to P3; after PR-M it is 10.5 to 11 µs (9 after P3), and
5.0 restates the rows that this and the other probes move. **After P1.2 the lane A rows of this table (P1.1 to P1.4,
P2, G2, MI3a, G3, G9a) are the month's plan, kept as a record; 5.0 holds their present values (X69).**

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

P1b, MI3b, G16, E4, E5 are counted as 0 here (P1b and MI3b are law steps, both dropped now; G16, E4 and E5 are small or gated).
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
| P1.1 to P1.4 | 0 | 0 | P1.1 done (bit for bit); P1.2, P1.3 not kept, P1.4 dropped (X62) |
| P2.1 | about 0 | about 0 | a law step at today's pace; its effect reported |
| P2.2 (60 days) | -6.3% ((b)); -12% ((a)) | -3.1% ((b)); -5.9% ((a)) | plant updates are 30.0% and 14.7% of a 1T year (R2); a stand-year x0.79 with the grid, x0.60 without (`lane-a.md` 2.3) |
| P3 | -5% | -3% | |
| P1b | 0 | 0 | dropped (X65) |
| E1, E2, AIR2, G1, G6, G7, G8, G14, G15 | 0 to +3% | 0 to +3% | they share work out; they cost a little on one thread |
| AIR1 | about 0 | -2% (Flow -8% or more) | the island already needs 2.26 rounds |
| AIR3 | -1% | -1% | |
| MI1, MI5 | -1 to -2% | -2 to -3% (MI1 -0.3%) | `choose::act` is 11.6% and 17.8% of a 1T year (Z1) |
| MI4 (after G2, gated) | about -0.5% | about -0.5 to -1% | after G2 a node's decisions are 0.30 to 0.31 of today; MI3a dropped (X67) |
| G2 | -6 to -7% | -10 to -11% | the decision only, on today's body: animal node updates x0.77 to 0.80 and x0.68 to 0.70 (X66); region24 about -2% |
| G3 | 0 | 0 | dropped (X68) |
| G4, G5 | -5% | -5% | faces x0.22; Change x0.4 |
| G9a, G9b, G12, AIR4, G11 | -4.9% ((b)) or -6% ((a)), -2%, -2%, -6%, -2% | -2.4% ((b)) or -2.9% ((a)), -1%, -2%, -10%, -3% | G9a: 90 days on the day law |

### 5.3 What reaches 10 times, and what is at risk

| | island22 | globe36 |
|---|---|---|
| needs | no case of 5.0 reaches it: even with variant (a) and G2 at 30 days the year before G14 is about 247 ms, and 96 needs -61% | no case of 5.0 reaches it: about 296 ms with variant (a), and 165 needs -44% (G14's good end is -35%) |
| likely with everything (5.0: variant (b) at 60 and 90, G2 at 30 or 20 days) | 4.0 to 5.4 times | 5.9 to 8.1 times |
| at risk | PR-C2: C2 failed the pace test's x2 at 140 days; stands at 60 only: 3.8 to 5.0 times; no plant pace: 3.3 to 4.3 | the same: 5.9 to 7.8; 5.3 to 7.1 |
| at risk | G2: its 30-day check on today's law with (g) off (PR-A2 ran (g) on); fallbacks 20 days, then young-only (X58); without G2: 3.8 to 5.0 | without G2: 5.2 to 7.0 |
| at risk | G14: the hardest step; G14-P1 saved 15% (globe 13%) today, G14-P2 decides | AIR4: its climate test (0.3 K) |
| at risk | G5 (its fallback is expected), G12 (not measured) | the same |
| without G14 | 3.5 to 3.9 times | 5.3 to 5.6 times |
| without G14 and AIR4 | 3.1 to 3.4 times | 4.7 to 4.9 times |

## 6. Order and lanes

```
Phase P   R1 ─ R2 ─ R3, probes: PR-M, PR-B, PR-A, PR-S, G14-P1, AIR4-R1 ─► review (section 5 restated)
Phase 1   A: (P1.1 done; P1.2 not kept) PR-C2 ─ P2.1 ─ P2.2 ;  G2 (beside) ───┐
          B: E1a ─ (MI1) ─ E1b.1 ─ E1b.2 ─ E1b.3 ─ E1c ─ E2 ───────────────────┤
          C: AIR1 ─────────────────────────── (E1c) ─ AIR3 ─ AIR2 ─────────────┤─► lane reviews (merge main)
          D: MI1 ─ MI5 ────────────────────────────────────────────────────────┤
          E: G8 ──────────────────────────── (E1c) ─ G1 ─ G7 ─ (E2) ─ G15 ─────┘
Phase 2   P3 ─ G13, G6 ;  P3 ─ G9a (also after G5) ─ G9b (gated)
          G4 ─ G5 ─ G9a ;  G2 ─ MI4 (gated) ;  G16 ;  G12 ;  AIR4 ─ G11 ;  E4, E5 (gated)
          G14-P2 ─ G14 (last) ;  GM, GB, E7 (measures)        ─► review ─► merge main ─► Z
```

**Where steps collide, this order holds** (X23):
- MI1 lands before E1b.1 (both touch the faces).
- E1b.3 lands before P3 (P3 runs inside the new pipeline).
- E1c builds the one domain runner and the one `book_by_array` helper. AIR2, G1, G6, G7 use the runner; AIR3 and G1
  book through the helper. AIR2 comes after E1c and cuts its domains by air node. E4 never touches the air's book
  (one way for it: AIR3).
- G9a lands after G5 (its settings are multiples of `phys_days` 30 and `batch_days` 10; the reference is every survey,
  30 days).
- MI5 lands before G2 (both change `spread`; MI5 is merged). G2 lands before MI4 (X36): its gate is measured again
  after it. G2 and P2.1 both touch `survey::update` (the grid, the habit): they merge one at a time, the second
  rebases.
- P2.2 (`node_days` becomes `field_days`) lands before P3; G2's setting is `think_days`, so no name is reused.
- Stale gates are measured again on the merged branch just before their step: E2's busiest share, E5's serial share,
  MI4's sharing.
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
| `laws.md` 6.2 (upkeep the stock cannot pay is paid from the time's income; the unpaid share dies at a rate over `D`), 6.6 (growth's reserve over `D`), 4.4 (a stand's maturing and births on its 36-day grid, if variant (b)); `numbers.md` `reserve_days` 36 | P2.1 |
| handover 7.1 stands, 7.3 population nodes; `laws.md` 4.4 (stands' pace); `numbers.md` `field_days` 60 (`node_days` renamed) | P2.2 (G9a moves `field_days` to 90) |
| `laws.md` 0, 4.4, 5.4 (a node with a brain decides about every `think_days` and takes by its last decision between); handover 7.3; `numbers.md` `think_days` 30 | G2 |
| plan 2.2 rule 2 (a batch is 10 days; individuals and smart bodies may run sub-batches) | G4 |
| `numbers.md` `batch_days` 10 (S64 replaced), `phys_days` 30, `flow_days` 60 | G4, G5, G12 |
| handover 7.3 the air's method; `numbers.md` `calm_tol` | AIR4 |
| `CLAUDE.md` 3.4: "When the physics runs beside the bodies, it works out its moves from the store at the window's start and books them after the bodies; each node's outgoing moves are cut in a fixed order, each against what the node holds after the ones before it, one share for each reaction or move as a whole, so no order of the two lanes can overdraw it."; handover 7.3 the step loop | G14 |

## 8. Risks

| Risk | Answer |
|---|---|
| The sum stays under 10 times | Said in section 1; each step is judged on its own kill line |
| P2.1 moves the key numbers (S50, light-eaters, births) | Announced law changes, each with its effect reported: a deep famine kills `1 - e^-u`, not `u`; a stand pays its upkeep from the time's light before members die. Stands keep their own temperature and reactions (P1.2's land -0.78 K came with the month's ground temperature and new reactions). Level 3 against the references after AIR4.0; a law change is its own commit with its effect |
| PR-C2 fails the pace test's x2 (C2 failed at 140 days, births about half) | variant (b): maturing and births on the stand's 36-day grid; else 60 only, or 36 stays (section 5.0's cases); the probe costs no core lines |
| A longer stand pace grows the world's plants (C2's remade globe held more bodies and cost 45% more) | PR-C2 counts the plant nodes at year 30 against the reference; a rise is counted in its gain |
| The day law's other windows at a long pace (the release limit and the light catalyst's contact read on the update's breath, the light liquid as a store, one room of roots per update) | C2 passed the pace test at 70 days with them in; PR-C2's splitting test and counters say if they matter, and a fix (X44's window) is then a step of its own |
| G2's 30-day check fails on today's law with (g) off (PR-A2 ran (g) on) | the fallbacks of X58 (20 days, then young-only, then revert); the habit repeats the last decision's body takes first if the globe's body takes are the cause |
| Habits slow learning (G2 learns once a decision) | The learning scene must pass; the answer is a shorter `think_days`, never a looser test |
| Tiles let a crowded tile run short of a scarce gas | E2 is level 3; the share per tile is `Mesh::share`; the E2 audit |
| G14's two lanes fight for memory, or its serial part stays large | G14-P1 and G14-P2 before the build, month windows timed apart, the serial part reported; G6 in the bodies' lane, the tape's frozen copy; the cut counter shows the lag |
| AIR4 fails its climate test | The island then needs G14 at its good end; the globe still passes with G14 |
| G5 grows S84's bias (expected: 75% of Change's rests are under 10 days) | Change keeps 10 days, the survey and state take 30; the likely column already counts this |
| Lanes collide in `survey.rs`, `spread.rs` and golden | Section 6's order; one merge at a time; `main` merged at each lane review and before Z |
| Snapshot bumps break the comparison of two builds | Section 2's judging method: turned saves (PR-S, X50) |

## 9. Lines

Core at the round's start: 9,602 code lines. Estimates from the designs (net), after the replan of lane A (X62 to X69):
lane A +130 (P2.1 +30, the grid +40 if variant (b), G2 +60; `lane-a.md` 5), P3 +155, engine +120, air +257 (AIR1 +12,
AIR3 -10, AIR2 +35, AIR4 +140, G11 +80), minds +48 (MI3a's +145 not written, X67), gap steps +310 (G14 +250, G1 +20,
G4/G5/G12 +30, G6 +25, G7 +30, G13 +10, G15/G16 +5; G2 is now in lane A, G3 is dropped): about **+1,020, to 10,620**.
The gated steps (G9b +100, E4 +30, E5 +150) bring it to about 10,900; if the estimates run 20% over, about 11,300. The
ceiling stays **12,000** (X22, past handover 14.3's 20% with the owner's yes): each step names what it deletes; at the
end of the round the ceiling is set to the lines then plus 200.
