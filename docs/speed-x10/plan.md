# Speed x10: the plan

Status: plan, under review. Branch `speed/x10`. The low-level design of each part is in `design/` (`plants.md`,
`engine.md`, `air.md`, `minds.md`); this file joins them, takes the decisions between them (`decisions.md`, X1 and on)
and gives the order of the work. Measures go in `measures.md`. History goes in `docs/history.md`, section 6.

## 1. Why, and what to expect

The last round (`docs/reports/speed.md`) made a year about 2 times faster and stopped: it changed how the computer does
the work, not what the world computes. The owner now accepts changes of laws and rules for speed, and named eleven
ideas (section 3). The designs studied each one against the code; this plan builds the ones that can pay and drops the
ones the designs showed to be bad from the start (section 3 says which and why).

**What the four designs found, in short:**

| Part | Biggest finding | Expected gain |
|---|---|---|
| Plants | Plants are 38% (island) and 21% (globe) of a year, not most of it: an animal update costs twice a plant's. Plants become *stands* that run one month-long law (upkeep and light together, exact solution per phase), so they update far less often | island 1.25 to 1.45 times, globe 1.1 to 1.2 |
| Engine | The 32 to 40% of the bodies' time on one core is not the price of determinism: it is one thread doing every write that crosses a domain. These writes can be shared out by destination in an order fixed by the data. Domains become tiles of 4 cells. 32-bit books do not pay and would break the books | bodies' stage -16 to -19% on 4 cores; 2 to 3 times today's design on 16 cores |
| Air | The air is already on a grid 4 x 4 times coarser than the ground, and its solve is 6 to 8% of its step. Longer air steps warm the winters past the climate test. What pays: calm cells keep their rows, the per-cell loops on all cores, a dense book | Flow 1.5 to 1.65 times (sure steps), up to 2.5 to 2.9 if the research steps pass |
| Minds | Decisions are 16% of a year; acting (node parts split off and back) is 40 to 64% of that, the network itself 1.4 to 2.1%. Habits (think only on change), shared senses of a place, movers as one group | year 1.05 to 1.08 times; calm animals updating less often 1.10 to 1.22 more |

**Honest arithmetic.** Multiplied, the sure steps give a year about 1.8 to 2.4 times faster on 4 cores than today, so
about 3 to 4.5 times the start of the last round. The research and gated steps could add 1.3 to 1.6 times more. On 16
cores the engine's pipeline adds 2 to 3 times over today's design. **10 times on 4 cores is not in sight with these
ideas; it is in sight on 16 or more cores, or with the gated steps passing.** 100 times needs a different engine
(a GPU for Flow and the stands, `docs/reports/speed.md`), which is not in this plan. The plan says so now so that no step
is judged against a hope.

## 2. Rules for this round

All of `CLAUDE.md` holds, except what decisions X1 to X6 change (the new wording is written into `CLAUDE.md` and the
handover in step R1). The working rules of the last round hold (`docs/speed-post-foundation/plan.md` section 2): check
levels, how a gain is judged (S45: callgrind instructions on one thread; wall time on 4 threads, 5 or more interleaved
runs on a quiet machine), kill lines, one step per commit, a review after each big step, the three rules for smart
bodies (plan 2.2 there: individuals stay real; the step length can be chosen per place; two kinds of body code).

**Check levels.** Level 1 bit for bit; level 2 the same to rounding; level 3 the same world: the key numbers
(`tape/tests/ranges.rs`) in their ranges, the nodes check, the pace tests. A level-3 step re-pins the golden test in its
commit and runs the key-number test once.

**Agents.** Opus for steps that change a law or the structure of the update; Sonnet for normal code, tests and docs;
Haiku for measures and data. Steps on different files run in parallel worktrees and merge one at a time; a step that
re-pins golden rebases and re-pins on the branch it merges into.

**Measures.** Each lane remakes the benches with its own code (S65: remade worlds are not comparable across laws) and
judges a step on one world: the same save run with both builds. Wall-time runs wait for a quiet machine (no other
cargo, rustc, valgrind or ew).

## 3. The eleven ideas: what is built, what is dropped

| # | Owner's idea | What we build | Design | Decision |
|---|---|---|---|---|
| 1 | Plants as a field | Plants (bodies of a lineage with no brain) become *stands*: a stand is the plant node of a cell, stepped by one month-long law (P1); then a field step per cell in the survey takes them out of the ring (P3) | plants 3, 4 | X7 |
| 2 | A growth table per genome | A *card*: the constants of a genome's month law, made in `prepare` (P1). A grid table is dropped | plants 6 | X8 |
| 3 | Upkeep and light together across the interval | The month law for plants (P1); then the same law for animals (P1b), so there is one energy law and animals can update less often (MI6) | plants 4 | X9 |
| 4 | Accept a different history with threads | Not needed: the pipeline E1, E2 shares the cross-domain writes out in an order fixed by the data, with the same gain; the history stays the same with any number of threads. An order-free variant is dropped | engine 2, 3 | X3 |
| 5 | Exact books per cell and month, 32-bit inside | Dropped for numbers that hold matter or energy: they stay 64-bit (32 bits would break the books and buy no speed). 32-bit is allowed for numbers that hold nothing (cards, brain rows, faces) where a step measures a gain | engine 5 | X4 |
| 6 | Fewer air steps where calm; a coarser air grid | Calm cells keep their rows and a calm solver (AIR1); the per-cell loops on all cores (AIR2); a dense book (AIR3); calm months by an error-controlled step, research (AIR4); shared sea balance, gated (AIR6). A coarser air grid is dropped: the air is already 4 x 4 coarser and its solve is 6 to 8% of its step | air 4 to 8 | X10 |
| 7 | Raise the code ceiling | 11,500 code lines in `core` | engine 4 | X2 |
| 8 | Think only when something changes | Habits (MI3): a body thinks again when its senses move, a body of another lineage comes or goes, its habit is no longer possible, or a season has passed | minds 2 | X11 |
| 9 | Think in groups | Shared senses of a place (MI4); a node's movers leave as one group (MI5). One decision for unlike nodes is not built (a merge of unlike nodes, S66) | minds 3 | X11 |
| 10 | Neural networks in batches | Dropped: each body owns 76% of its brain's weights, so a batch shares little (about 0.1% of a year), and the network is 1.4 to 2.1% of a year | minds 4 | X12 |
| 11 | Full thinking for smart bodies | `--full-minds` (every body thinks at every update) is the control run of habits; a lineage at or over `Config::full_hidden` hidden units is never a node and always thinks fully (MI8) | minds 5 | X13 |

## 4. The steps

Each step has its full description, done-when and kill line in its design file; the table gives the order, the level,
the agent and the gate. "4T" is wall time on 4 threads; "instr." is callgrind instructions of a year on one thread.

### Phase R: rules and measures (first)

| Step | What | Level | Agent | Done when / kill line |
|---|---|---|---|---|
| R1 | The new rules: `CLAUDE.md` (2.11 the ceiling 11,500; 3 the books; 6.1 the levels), handover 12 and 14, `cli/tests/core_size.rs` (X1 to X6); docs `speed-x10/` | docs | Sonnet | the rules read as X1 to X6 say |
| R2 | Gate measures on remade benches at the branch's start: plant and animal shares (P0), the busiest share for tiles of 1, 4, 16 cells and the serial phase's parts (E0), the sleep share and the place-table sharing (MI0, a probe on a branch), the air's rows and rounds (air 3) | none | Haiku | `measures.md` holds them |
| R3 | The long-step test runs a world `Config::validate` accepts (`phys_days` 2, not 3), and calls `validate` (AIR-T) | test | Sonnet | the test passes or its failure is reported |

### Phase 1: four lanes in parallel

Each lane is a chain; the lanes touch different files (plants: `body/`, `copy/`, `survey` stands; engine: `outbox`,
`life/`, `survey::batch`, `store/`; air: `flow/`, `solve/`; minds: `choose/`, `brain/`). Where two lanes touch one file
(`survey.rs`), the later merge rebases.

| Lane | Steps in order | Notes |
|---|---|---|
| Plants | **P1** the plant month and the card (Opus, level 3; kill line: under 10% fewer island instr.) → **P2** `field_days` 60 if the pace test passes (Sonnet) → **P1b** the month law for animals (Opus, level 3; X9) → **P3** the field step per cell, plants out of the ring (Opus, gated: at least 5% more) | |
| Engine | **E1a, E1b, E1c** the pipeline: leaving bodies and sorted outboxes, global ids in a short serial step, landing and booking per destination on the pool, the small serial rest (Opus; kill line: under 6% on 4T) → **E2** tiles of 4 cells, each taking at most its cells' share of their air node (Opus, level 3; gated by R2) → **E4, E5** gated | |
| Air | **AIR1** calm cells and calm solver (Sonnet, level 2) → **AIR3** dense book (Sonnet, level 1 or 2) → **AIR2** the air's and liquid's per-cell loops on all cores (Opus, level 1 with the fixed-order rule) | AIR4, AIR6 in phase 2 |
| Minds | **MI1** the face cover once per batch (Sonnet, level 1) → **MI3** habits (Opus, level 3; gate: sleep share at least 40%) → **MI4** shared place senses (Sonnet, level 2) → **MI8** full minds by `full_hidden` (Sonnet) | MI5, MI6 in phase 2 |

### Phase 2: the gated and research steps

| Step | What | Where | Kept when |
|---|---|---|---|
| AIR4 | Calm months by TR-BDF2 with an error estimate: first find why long steps warm the winters (R1 of air) | lane | the climate tests hold (0.3 K) and Flow gains at least 15% |
| AIR6 | Alike open-sea cells share one surface balance | lane, gated | the gate's measure, then 10% of Flow |
| MI5 | A node's movers leave as one group | lane | the nodes check holds and decisions gain at least 10% |
| MI6 | Calm animals update less often (needs P1b) | lane | a new animal pace test holds and a year gains at least 5% |
| E7 | Scaling check on more cores (as far as the machine allows) | measure | `measures.md` |

### Phase Z: the final check

The benches and profiles, the slow suite (key numbers, nodes check, pace tests, the debug checks), 1,000-year runs of
the island, the globe, the refined region and the largest planet that fits 15 minutes, a watch report
`docs/reports/speed-x10.md` saying plainly where the speed stands against 10 times, and the docs describing the present.

## 5. Order and parallel lanes

```
R1, R2, R3 ──► plants: P1 ──► P2 ──► P1b ──► P3 ─────────────┐
           ├─► engine: E1a ──► E1b ──► E1c ──► E2 ──► (E4, E5)  ├─► review per lane ─► phase 2 ─► Z
           ├─► air:    AIR1 ──► AIR3 ──► AIR2 ────────────────┤
           └─► minds:  MI1 ──► MI3 ──► MI4 ──► MI8 ───────────┘
```

Merges into `speed/x10` go one at a time; level-3 steps rebase and re-pin. A review (adversarial Opus, then a fixer,
until approved) runs after each lane's chain and after phase 2, and a final review after Z. Wall-time measures of two
lanes never run at the same time.

## 6. Risks

| Risk | Answer |
|---|---|
| The sum stays far from 10 times | Said in section 1; each step is judged on its own kill line, not on the total |
| P1 moves the key numbers (stored share S50, eaters) | Level 3 with the key-number test; a law change is its own commit with its effect |
| Habits slow learning | The learning scene must pass; the answer is a wake on surprise, never a looser test |
| Fewer updates repeat C2's failure | P1b removes C2's cause (upkeep before light) before MI6; MI6 has its own pace test |
| Tiles let a crowded tile run short of a scarce gas | E2 is level 3 with the key numbers; the share per tile is the parts law's `Mesh::share` |
| The lanes conflict in `survey.rs` and the golden pins | One merge at a time; each re-pins on the branch it lands on |
| The ceiling is reached | 11,500 leaves about 1,900 lines; each step names its deletions |
