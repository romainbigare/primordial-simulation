# Speed after the foundation: the plan

**Status:** the owner asked for this work on 24 October 2026: build ideas A to E of the speed list (group F waits).
The owner asked the architect to take every decision and log it: they are in `decisions.md` next to this file. What was
built and measured goes in `docs/history.md` (section 5) and in `measures.md` next to this file. Where this file and
`docs/handover.md` differ, the handover wins until it is updated.

This file is for the agents who do the work and for the owner. It is written in simple English (handover 0).

---

## 0. In short

- **The goal:** a settled world runs **10 times faster** on the owner's 4-core machine, with the same laws and the
  same honest books. So 1,000 years of a planet of about 3,000 cells fit 15 minutes, against about 300 today.
- **Why it is slow today** (`docs/reports/foundation-l.md`): bodies are 60 to 80% of a year and run on one core; one
  body update costs about 40 µs (about 200,000 instructions); the number of bodies grows for centuries; the physics
  did not get cheaper; three cores of four sit idle.
- **What we build**, in five groups:
  - **A, measure and free wins:** a bench kit, a profile, better compiler settings.
  - **B, fewer bodies:** one node per kind per cell with its young inside; light that reads the sizes inside a node;
    cells that cannot fill without end.
  - **C, fewer updates:** an exact step over a node's interval, so nodes update half as often; dormant bodies sleep;
    bodies update in batches of days.
  - **D, all four cores:** the world is cut into **domains** (the cells under one air node); each domain runs its
    bodies on its own thread; what crosses a domain's edge waits in an **outbox** booked in a fixed order. The
    physics runs its per-cell work on threads too.
  - **E, a cheaper update:** bodies stored as columns inside each domain; the update of bodies without a brain as
    stages over all of them at once.
- **Ten times is not sure.** Amdahl's law (section 1.3) says both the bodies and the physics must get much faster.
  Each step measures its gain; a step that does not pay is taken out again and logged.

---

## 1. The goal and how we measure it

### 1.1 The targets

| Target | Today (`docs/reports/foundation-l.md`) | Goal |
|---|---|---|
| Island 22 × 22, settled (bench `island22`) | about 1.5 s a year | **0.15 s a year** |
| Globe 36 × 18, settled (bench `globe36`) | about 2.1 s a year | **0.21 s a year** |
| 1,000 years of the 36 × 18 globe | 36.6 min | under 15 min (on the way) |
| Largest planet in 15 minutes (cost model, plan 5.7 of the foundation) | about 290 cells | about 3,000 cells |

A settled world is the world after its life has settled: its bodies per cell are what a long run has, not what year 10
has. "10 times faster" counts every gain: fewer bodies (B), fewer updates (C), more cores (D), cheaper updates (A, E).

### 1.2 How we measure (step A1 builds this)

- **Bench worlds.** `scripts/bench.sh` makes two snapshots once for each snapshot version and each world-changing
  step: `island22` (`ew run --size 22 --seed 1`, year 300) and `globe36` (`ew run --globe --size 18 --seed 1`, year
  200). It keeps them in a folder outside the repository (`$BENCH_DIR`, default `~/.cache/ew-bench`). It then runs
  `ew resume` for 5 years from each and prints, per simulated year: Flow, Change, bodies, parts, tape (from the
  `ms_*` columns), the bodies alive, the body updates, and the seconds a year. Nothing else runs on the machine.
- **Profile.** `scripts/bench.sh profile <bench>` runs one year under `valgrind --tool=callgrind` (there is no
  `perf` here) and prints the top functions by inclusive and exclusive instructions. Instruction counts do not
  depend on what else runs, so they also compare two builds exactly.
- **The record.** Each step that touches speed adds one row to `measures.md`: the commit, both benches (s a year
  and each stage), bodies, updates, core code lines. The owner reads the whole story there.
- **Few long runs.** The benches take seconds. Long runs (1,000 years) wait for the last phase, when they are fast.

### 1.3 The arithmetic of 10 times

A settled year is about 70% bodies and 30% the rest (Flow, Change, parts, tape). If bodies became infinitely fast and
the rest stayed, a year would take 0.30 of today: only 3.3 times faster. So:

- **Bodies** must become about 15 to 20 times cheaper: B (about 2 to 3 times by fewer bodies), C (about 1.5 to 2 by
  fewer updates), D (about 3 on 4 cores), E (about 1.3 to 2 by a cheaper update).
- **The rest** must become about 4 to 5 times cheaper: A2 (about 1.1 to 1.2), D3 (about 2.5 to 3.5 on 4 cores), and
  tape work moved off the main thread (D1).

These are the plan's guesses. `measures.md` holds the real numbers.

---

## 2. The rules of this work

### 2.1 The repository's rules hold

`CLAUDE.md` holds for every step: one place for each idea, no magic numbers, no dead code, no compatibility code,
no experiment switches, clear names, no Earth words in `core` and `worldgen`, clippy clean, exact books, comments
that say what and why (with a reference), no history in code, docs that describe the present.

The main rule of the foundation holds too (`docs/foundation/plan.md`, "The main rule"): **a general piece replaces
the special code it makes useless, in the same step.** No new pool, cache or catch-up for timing. Two functions of the
same shape become one. Every commit message ends with its line count: `Lines: +X -Y (core code N)`.

### 2.2 The three rules for future smart bodies (the owner's wish)

The work must not close the door to smart, people-like bodies later (handover 13.2, "No people-like kind"):

1. **Individuals stay real.** Merging is a speed tool. An individual with a brain that learned (its rows differ
   from the rows it was born with) is never merged away by a speed rule. A node can always be split back into
   individuals (`nodes::open` stays and is tested).
2. **The step length is chosen per place.** The batch length (C3) and the domain clock are built so that a domain
   could later run with a shorter batch (a focus zone, handover 7.3), without a new design. Today one value holds for
   all domains.
3. **Two kinds of body code.** The batched path of E2 is for bodies without a brain. Bodies with a brain keep a
   per-body path. Nothing in E2 may assume that every body takes the same branches.

### 2.3 Checks for each step

| Check | When |
|---|---|
| `cargo build --release --workspace` | every step |
| `cargo test --release --workspace` (the normal suite, under about 30 s, with the golden test) | every step |
| `cargo clippy --release --workspace --all-targets` with no warning | every step |
| `cargo build -p pcore --target wasm32-unknown-unknown` | every step |
| `cargo test --release -p tape --test golden` (the golden test without threads) | every step that touches threads or order |
| `scripts/bench.sh` (both benches, 5 years each) | every step that touches speed |
| The slow suite (`cargo test --release --workspace -- --ignored --no-fail-fast`) | at the end of each phase only |
| The equivalence test of a new speed setting (× 2 and ÷ 2) | in the step that adds or changes it |

**Tests are few and fast** (`CLAUDE.md` 5). A new test states a law or an invariant. A step that deletes a
mechanism deletes its tests.

**Check levels** are those of `docs/foundation/plan.md` 6.1: level 1 (bit for bit), level 2 (the same to rounding,
for speed work that changes the order of sums), level 3 (the same world: key numbers in their ranges). Each step
names its level. A level-2 or level-3 step re-pins the golden test in the same commit and says why.

### 2.4 The code ceiling

`core` holds 8,998 code lines; the ceiling is 9,000 (`docs/foundation/decisions.md` 42). Until D2 lands, each step
must delete as many core lines as it adds. The owner's 700 lines "for running the bodies on threads" (decision 42)
open when D2 shows its gain on the benches: the ceiling becomes 9,700 in that commit (decision S3). Never pack lines
to save count; comments are not counted.

### 2.5 How a step is worked and closed (the agents)

The architect plans, splits the work, reviews and merges. For each step:

1. **Context:** a Haiku agent gathers facts (call sites, numbers, profiles) when the step needs a wide search.
2. **Build:** an Opus agent for complex code (B1, B3, C2, D2, E), a Sonnet agent for normal code (A, B2, C1, C3, D1,
   D3, docs). Steps on different files run in parallel, each in its own git worktree.
3. **Check:** the builder runs the checks of 2.3 and adds the bench row.
4. **Review:** at the end of each phase (and of D2's big sub-steps), an adversarial Opus agent reviews the diff
   against this plan, `CLAUDE.md` and the handover. A fixing Opus agent answers every finding (fix it, or say why
   not). They loop until the reviewer approves.
5. **Close:** the architect re-reads the diff, then commits to `main` and pushes. One line per change goes in
   `docs/history.md`; a decision taken goes in `decisions.md`.

### 2.6 When a step fails its goal

A step that does not reach its "done when" stops. If it does not pay (its gain is under its kill line), it is taken
out again and gets one line in `docs/history.md` under "tried and not kept". If it pays but breaks a check, the cause
is found and fixed; a range or a tolerance is never loosened to pass (`CLAUDE.md` 5.5).

---

## 3. The architecture after this work

### 3.1 The picture

```
  world step (one month)
  ├─ survey            every phys step: per domain on threads (shade, sums, wear, crowd)        [D2.6]
  ├─ Flow, Change      per-cell work on threads, sums in a fixed order                           [D3]
  ├─ Parts (Bond)      once a month, as now
  └─ bodies            every batch of `batch_days` days                                          [C3]
        ├─ parallel:   each domain runs its due bodies in its fixed order,                       [D2]
        │              on its own bodies, its own store view and its own outbox
        │                 └─ a body without a brain: the batched stage path                      [E2]
        │                 └─ a body with a brain: the per-body path
        └─ serial:     the domains' moves booked by `apply`, then their outboxes,
                       in the order of the domains, then of the bodies
```

### 3.2 Domains (D2)

- A **domain** is one air node and the ground cells under it (4 × 4 cells today; more where a refined region is).
  Domains are fixed when the world is made. Their order is the order of the air nodes.
- **A domain owns its bodies.** `Life` holds one `Domain` per air node: its bodies (columns after E1), its cells' lists
  of bodies, its ring of updates due, and its outbox. Global registries (lineages) are read in the parallel phase and
  changed only in the serial phase.
- **Stable ids.** A body gets an id at birth that never changes: its domain of birth and that domain's counter. A body
  that moves to another domain keeps its id; an id table finds where it lives. Memory of individuals (`Body::known`)
  and the rng keys read the id, so a move between domains changes nothing for them.
- **What a body update may touch** in the parallel phase: its own body and the other bodies of its domain; the store
  nodes of its domain (its cells' ground and bodies' nodes and its air node) through the domain's **store view**
  (its nodes as the moves of this batch leave them, the way `store::Moves` keeps a cell's nodes today); the survey's
  caches and the rest of the world, read only. Everything else goes to the outbox.
- **The outbox** holds, in the order they were made: members or children that start in a cell of another domain; a
  body that moves to another domain; a watch of a body of another domain; an action on a part; a new lineage; changes
  of lineage counts; events and counters. The serial phase applies the outboxes in the order of the domains. An
  effect whose target is gone fails in a fixed way, and what it carried goes back to where it came from, so the books
  hold.
- **Same history with or without threads.** The parallel phase reads a world that does not change, and each domain is
  sequential inside, so its result does not depend on which thread ran it or when. The golden test checks it.
- **What changes in the world (level 3):** bodies of other domains are seen as they were at the batch's start; effects
  that cross a domain's edge land at the batch's end. This is the same kind of lag a body already has when it sees
  far cells "as the last survey saw them" (handover 5.6 of the foundation).

### 3.3 Batches (C3)

- Bodies update in **batches** of `Config::batch_days` days (5, the shortest interval `MIN_INTERVAL`). A batch runs, at
  its first day, every body whose due day falls inside it.
- A body keeps its **due day**; its next due day is its due day plus its interval (not its run day plus its interval),
  so intervals hold on average and nothing drifts. It integrates the real time since its last update, as today.
- A body that is born or placed in a batch is first due in a later batch.
- `batch_days` is a speed setting with its equivalence test (1, 5 and 10 days give the same key numbers to the noise).

### 3.4 One node per kind per cell (B1, B3)

- Today a kind in a cell is an adult node and a young node joined by a link; young nodes are about 74% of an island's
  bodies. After B1 a kind in a cell is **one node** whose members have many sizes: a share at the adult size and the
  young below the copy mass, with the size moments the young node keeps today (`Body::size_spread`, `size_lag`).
- Maturing is a flow inside the node (the same `math::Pass`), not a merge of two bodies. Children of a node join its
  young part. `copy/young.rs`, `Body::link` and the stage in `nodes::Kin` go.
- What depends on a member's size (supply by mass^0.75, costs by mass, height) reads the node's sizes, not only the
  mean member, where the difference matters (Jensen's inequality on a power law).
- B3: the light of a cell passes the young part and the adult part of each node as two layers by their heights
  (Purves et al. 2008, the perfect plasticity approximation; Moorcroft et al. 2001), so a node of many sizes shades and
  is shaded as its members are. The selection sample reads the light at its own height (proposal 10.3 #2 of the
  foundation plan).

### 3.5 Columns and the batched path (E1, E2)

- E1: inside a domain, bodies are columns: the scalars of all bodies in arrays, the stores in one array (bodies ×
  substances), the genes and spreads in one array each, the variable parts (brain rows, memory, trace) per body. Code
  reads a body through a small view. Same arithmetic in the same order: level 1.
- E2: the update of the due bodies without a brain of a domain runs as stages over all of them (site, upkeep,
  physics, keeping warm, store checks, growth, copy, shed, selection), each stage a loop over the bodies, so repeated
  work is done once and the hot loops are short. Bodies with a brain keep the per-body path (rule 3).

### 3.6 Physics on threads (D3)

Per-cell work that today runs in a serial loop runs through `par::rows` (rows made in parallel, then summed in the
fixed order of the cells). Sums across cells keep their order, so the history is the same bit for bit (level 1).

---

## 4. The work list

Each step has an ID, a check level, a "done when", a kill line (when it does not pay) and the agent that builds it.
"Parallel with" names the steps that can run at the same time in their own worktrees.

### Phase A: measure, and the free wins

#### A1. The bench and profile kit

- **What:** `scripts/bench.sh` as in 1.2; `measures.md` with its first rows (today's code, both benches, and a
  callgrind profile of each with the top 25 functions); `docs/history.md` gets a section "5. Speed after the
  foundation".
- **Why:** every later step must show its gain on the same worlds; the last full profile is from Phase A of the
  foundation, before the refactor.
- **Challenges:** the bench snapshots go stale when the snapshot version changes or a step changes the world: the
  script stores the snapshot version and the commit beside them and remakes them when either is older than the last
  world-changing commit (it asks with `--remake`, it never remakes silently). Wall time is noisy: the script runs each
  bench twice and keeps the lower.
- **Check level:** none (no core change). **Done when:** both benches and both profiles are in `measures.md`.
- **Agent:** Sonnet. **Parallel with:** A2, D3 (after A1's baseline is recorded).

#### A2. Compiler settings

- **What:** in `Cargo.toml`'s release profile, `lto = "fat"` and `codegen-units = 1`; in `.cargo/config.toml`,
  `target-cpu=native` for the native target only (not wasm). Then profile-guided optimisation (PGO) tried with
  `cargo pgo` or `llvm-profdata`: trained on the two benches.
- **Why:** the compiler can inline across crates and use the CPU's newer instructions.
- **Challenges:** the golden fingerprints may move (other instructions, same maths in most places); then they are
  re-pinned at level 2 and the commit says why. PGO needs a tool installed and a second build: it stays only if it
  gains at least 8% on the benches and its script is short (`scripts/pgo.sh`); else it is logged as not kept.
- **Check level:** 1 or 2. **Done when:** each setting's gain is in `measures.md`; the kept ones are on `main`.
- **Kill line:** a setting that gains under 2% is not kept.
- **Agent:** Sonnet. **Parallel with:** A1 (the baseline first), D3.

#### A3. Many worlds at once

- **What:** the slow tests that run several seeds already run them on threads (`tape/tests/ranges.rs`); they set the
  thread pool of Flow and Change so that the seeds do not fight for cores (`RAYON_NUM_THREADS` or a local pool).
  `docs/speed-post-foundation/README.md` says how to run seeds side by side with `ew` (`RAYON_NUM_THREADS=1` per run).
- **Why:** the owner's work runs three seeds; on four cores that is nearly three times faster for free.
- **Check level:** none. **Done when:** the slow suite's seed tests run side by side without oversubscribing the
  cores. **Agent:** Sonnet (small).

#### A4. A faster machine

Not code: the report tells the owner what a newer CPU would give (measured on the profile's instruction counts and
the bench's instructions per second). Logged as decision S5.

### Phase B: fewer bodies

#### B1. One node per kind per cell, its young inside it

- **What:** as in 3.4. A node's members have sizes from a child's to an adult's; the young part keeps the density of
  `copy/young.rs` (log10 size below the copy mass, `e^(-k y)` fitted to its mean and variance); the adult part keeps
  the adult size. Children of a node join its young part (`nodes::add` with their size moments); maturing moves
  members and their matter from the young part to the adult part by `math::Pass` inside one body, booked by shares as
  `nodes::Lot` does today. Brood is made from the adult part's growth only (`ALLOC`).
- **Why:** young nodes are about 74% of an island's bodies and update as often as adults: about half the updates go.
- **Design points:**
  - `Body` gains the adult share of its members (or the young count); `size_spread` and `size_lag` describe the young
    part; `Body::link`, `copy::young::{linked, link}`, `Grown::Joins` and the stage in `nodes::Kin` and
    `merge_closest`'s key go.
  - Growth (`copy::grow`) splits what the node builds between its parts by their supply (members × mass^0.75 at
    each part's size, Kleiber), so the young grow at a young member's pace and the adults fill their brood.
  - Upkeep, costs by kg, the store checks and death by wear run on the node's totals as now (they are linear in
    mass). Rates that are a power of a member's mass read the part's own mass.
  - Individuals (`count` 1) do not change. A child of an individual joins the node of its kind in its cell when it is
    alike (`copy::kin`, without the stage), else starts a body.
- **References:** Lefkovitch 1965 and Caswell 2001 (stage-structured populations); McKendrick 1926 and von Foerster
  1959 (the flow across a size); Kleiber 1932; Weiner 1990 (size hierarchies in plant stands).
- **Challenges and answers:**
  - *A node's mean member is no longer one member's size.* Every use of `member_body` and `size` is listed first
    (a Haiku agent) and each is assigned to "the node's totals" or "a part's member". The list goes in the commit
    message.
  - *The books.* Maturing inside a body moves nothing between nodes, so it cannot break the books; the tests that
    hold the books of young and adult nodes are rewritten for one node.
  - *The nodes check (F7).* It may move. It is run once at the end of phase B and its result written in the report;
    a rule that newly fails is a bug to find, not a range to widen.
- **Check level:** 3. **Done when:** bodies per settled cell fall by at least a third on `island22`; the key numbers
  hold (run once at the end of phase B); the pace test holds.
- **Kill line:** if the bodies' time on the benches falls by less than 20%, stop and report.
- **Code:** about -80 core lines (the review's estimate). **Agent:** Opus. **Parallel with:** D3.

#### B3. Light and selection read the sizes in a node

- **What:** the survey's shade (`survey::shade`) puts each node's young part and adult part as two layers at their
  own heights (`Body::height` at each part's member size); a part's light per capture area is that of its layer. The
  selection sample (`copy::sample`) reads the light at the sample member's height from the cell's sorted layers, so
  a gene that makes a member taller or shorter changes the light it gets (proposal 10.3 #2 of the foundation plan).
- **Why:** B1 puts young and adults in one body; without this, a node's young would get the adults' light. With it,
  the frame (height) is seen by selection, which today sees only its cost (decision 52).
- **References:** Purves et al. 2008 and Strigul et al. 2008 (the perfect plasticity approximation); Moorcroft et al.
  2001 (the ecosystem demography model).
- **Check level:** 3. **Done when:** a test holds that the shorter part of a node gets less light per area than its
  taller part under a closed canopy; the key numbers hold.
- **Code:** about +20 core lines. **Agent:** Opus. **Depends on:** B1. **Parallel with:** B2.

#### B2. Cells that cannot fill without end

- **What:** past `Config::crowd` nodes at a month's start, the most alike pair **of one lineage** merges whatever the
  distance of their genes (today unlike nodes never merge), their genes pooled with their spreads by members
  (`nodes::add`). An individual with a brain whose rows differ from its born rows is never merged by this rule
  (rule 2.2.1); with `--individuals`, no body with a brain merges, as today.
- **Why:** on long runs, nodes of one lineage that differ in their genes pile up (up to 73 in a cell; the refined
  region's bodies grew from 2,891 to 11,536 over 1,000 years, and a year's cost with them).
- **Challenges and answers:**
  - *Merging two unlike nodes loses the difference.* Their pooled spread keeps it as variance, as a real population
    keeps it (the variance of a mixture: within plus between). The spread's cap (`body::most`) stays a cap; a node
    whose spread is at its cap does not split while its cell is past `crowd`, as today. The `crowd` equivalence (× 2
    and ÷ 2, handover 7.1's noise) checks the world does not change past its noise.
  - *Merge, split, merge.* `nodes::split` already needs room in its cell; a merged node cannot split again while the
    cell is full. A test holds that a full cell's node count does not rise past `max(crowd, lineages there)`.
- **Check level:** 3. **Done when:** on a 1,000-year run of the refined region (phase Z) the bodies stop growing
  after they settle; on the benches the most bodies in a cell is at most `crowd` plus the lineages present.
- **Code:** about +10 core lines. **Agent:** Sonnet. **Depends on:** B1 (the stage leaves the merge key).
  **Parallel with:** B3.

### Phase C: fewer updates

#### C2. An exact step over a node's interval (node_days 36 → 72)

- **What:** find what makes nodes that update rarely differ from nodes that update often (the pace test fails at
  144 days: births -55%), and integrate those terms exactly over the interval. Then double `node_days`.
- **Why:** a node without a brain updates every `node_days` (36). At 72 its updates halve.
- **How:**
  1. Measure (a Haiku agent runs the pace test's worlds at 36, 72, 144 days and gathers births, deaths by cause,
     growth and capture area per update) and write where the error comes from.
  2. The expected terms, with the method for each:
     - growth that compounds through the capture area during the interval (a node that grows catches more light
       before its next update): the exponential integrator `(e^z − 1)/z` on the node's last specific growth rate
       (exponential time differencing; Cox and Matthews 2002), bounded by the light its cell set aside for it, so the
       books hold;
     - births and maturing during the interval: the exact solution of the node's two-stage linear model over its
       interval (Caswell 2001, the continuous-time stage matrix; a 2 × 2 exponential in closed form);
     - deaths: already exact (`1 − e^−wear`).
  3. Set `node_days` 72; the pace test then runs its × 2 and ÷ 2 (36 and 144).
- **Challenges and answers:** the light a node may claim is set aside per cell by the survey; an integrator that
  claims more than its share would overdraw the bodies' node. The claim stays inside what the cell set aside for the
  node (`survey::claims`); a test holds it.
- **Check level:** 3. **Done when:** the pace test passes at 36 and 144 around `node_days` 72; the key numbers hold.
- **Kill line:** if the pace test cannot pass at 144, `node_days` stays 36 and the integrator stays only if it makes
  the pace test closer at 72; the rest is logged.
- **Agent:** Opus. **Depends on:** B1 (the node's stages live in one body). **Parallel with:** D3.

#### C1. Dormant bodies sleep

- **What:** a node without a brain whose activity is near nothing at its update (dry or frozen: `Body::activity`,
  laws.md 6.5) sleeps until its cell can wake it: the survey checks its cell's ground temperature and liquid each
  physics step and wakes it when its activity there would pass the threshold, or at `MAX_INTERVAL`. It then
  integrates the whole time, as every update does.
- **Why:** a dry or frozen body's chemistry is nearly stopped (handover 4, "a dry body is nearly dormant"); updating
  it every interval costs time and changes nothing.
- **Measure first:** a Haiku agent counts, on both benches, the share of updates of bodies without a brain whose
  activity is under 0.05. **Kill line:** under 5% of updates, C1 is not built and is logged.
- **Check level:** 3 with an equivalence test (sleep threshold × 2 and ÷ 2 against no sleep). **Agent:** Sonnet.
  **Depends on:** C2.

#### C3. Batches of days

- **What:** as in 3.3: `Config::batch_days` (5), due days kept per body, `survey::day` runs once a batch.
- **Why:** a day holds 1.5 to 15 ms of body work: too little to share between threads (decision 59 of the
  foundation). A batch of 5 days holds five times more, and D2 needs one batch as its unit of work.
- **Challenges and answers:** a body due inside a batch runs at its start, up to 4 days early; keeping its due day
  (not its run day) keeps its pace on average. The ring of updates (`life::RING`) is taken by batch; a body placed
  during a batch is filed in a later one.
- **Check level:** 3 with its equivalence test (1, 5 and 10 days). **Agent:** Sonnet. **Depends on:** C2 (both touch
  the timing of nodes). It is the first sub-step of D2's work.

### Phase D: all four cores

#### D3. The physics on threads

- **What:** the per-cell loops of Flow that run serially today run through `par::rows`, their cross-cell sums kept
  in the fixed order of the cells. From the physics map (the air step's `fixed`, `room`, `seen`; the air's `gather`
  per air node over its cells in cell order; land's `ground_state` and `surface`; liquid's `slopes`, `mobile`,
  `surface`'s rates and `follow`). The priority-flood order, the downhill pass and the conjugate gradient's dot
  products stay serial.
- **Why:** Flow is 15 to 30% of a year and uses one core outside the air's `pass`.
- **Challenges and answers:** a sum across cells must keep its order to stay bit for bit: each parallel task writes
  its own row; the sums are made afterwards in the old order. The golden test runs with and without threads.
- **Check level:** 1 (bit for bit). **Done when:** the golden test keeps its fingerprints with and without threads;
  Flow is at least 1.8 times faster on 4 cores on the benches. **Kill line:** a loop that gains under 1% of Flow
  stays serial.
- **Agent:** Sonnet. **Parallel with:** A2, B1, B3, B2, C2 (it touches `flow/` and `solve/` only).

#### D2. Domains: the bodies on all cores

Built in sub-steps, each one committed when it passes its checks. The adversarial review runs after D2.3 and after
D2.5.

- **D2.1 Domains own their bodies (sequential).** `Life` holds a `Domain` per air node (its bodies, its cells'
  lists, its due ring); stable ids (domain of birth and its counter) with an id table; born rows are kept by each body
  (`Life::genomes` and its counts go: a body with a brain keeps its own born rows, a body without has none). Bodies
  still run in today's order. Level 3 (new ids key the random draws). Snapshot version bumped.
- **D2.2 Batches (C3).** As in 3.3.
- **D2.3 The update runs on its domain.** The update code (`body/`, `choose/`, `copy/`, `nodes.rs`, the body side of
  `bond/`) takes a context that holds the world read-only and its domain writable: the domain's bodies, its store view
  (`store::Moves` grown to the domain's nodes for the batch), its outbox, its events and counters. Effects that leave
  the domain go to the outbox (3.2). The batch's moves are booked by one `apply` per domain in the serial phase. Still
  one thread. Level 3.
- **D2.4 The outbox, whole.** Every kind of effect of 3.2, applied in the order of the domains and then of the
  bodies; an effect whose target is gone fails and gives back what it carries; new lineages and lineage counts are
  settled there. A test makes each effect fail once and checks the books.
- **D2.5 Threads.** With the feature `threads`, the domains of a batch run on rayon's pool (`par_iter_mut`); without
  it, in order. The golden test gives the same fingerprints both ways. The ceiling becomes 9,700 if the benches show
  the gain (decision S3).
- **D2.6 The survey on threads.** Shade, sums, wear and the monthly crowd run per domain in parallel; the Draws of
  the air step and the marks of light are filled per domain.
- **Why:** bodies are 60 to 80% of a year on one core. The waves of blocks of the foundation failed (decision 59)
  because a day held too little work and each block was copied onto a world of its thread's own; here a batch holds
  five days of work, and nothing is copied: each domain owns its bodies.
- **References:** the domain decomposition of agent models (FLAME GPU, Richmond et al. 2010; Repast HPC, Collier and
  North 2013); deterministic parallel order (Bocchino et al. 2009, "Parallel programming must be deterministic by
  default").
- **Challenges and answers:**
  - *A body reads the bodies of a neighbour domain* (perception of bodies within sight, watching): it reads their
    public face (visible traits, signal, well-being, id) from a face kept per body and swapped at each batch's end,
    so it never reads a body another thread is changing.
  - *Load balance:* sea domains hold few bodies. Rayon's work stealing takes domains one by one; the batch's work per
    domain is logged by the bench so an imbalance shows.
  - *The store view must not overdraw:* a domain's bodies take from its nodes in order, each seeing what the bodies
    before it left (as `store::Moves` does for one update today), so no take can pass what the node holds.
  - *Flow writes bodies* (`mark`, `draw.owed`, the bodies' node's heat): those loops go over the domains, outside the
    batches.
  - *The tape and the viewer read bodies:* `Life::alive()` and the per-cell lists keep their meaning through the
    domains.
  - *wasm:* without `threads`, the domains run in order, the same history.
- **Done when:** the golden test is the same with and without threads; on the benches the bodies' stage is at least
  2.5 times faster on 4 cores than D2.4 on one; the key numbers hold.
- **Kill line:** if the bodies' stage gains under 1.8 times on 4 cores, stop and report before E.
- **Agent:** Opus for D2.1, D2.3, D2.4, D2.5; Sonnet for D2.2 and D2.6. Sequential (they touch the same files).

#### D1. Work off the main thread

- **What:** (a) the tape's file writing (snapshot compression, series rows, viewer frames) runs on a thread of the
  `cli` while the world runs on; (b) measure the cores' idle time in a year after D2 and D3; if more than 25% of a
  year leaves three cores idle, build the physics of the next step to run beside the bodies through the bodies'
  node (decision S9 describes the design and why it waits).
- **Why:** the tape is 1 to 5% of a year; later it would be 10 to 50% of a faster one.
- **Check level:** 1 (the world is the same; only the shell's timing moves). **Agent:** Sonnet. **Parallel with:**
  D2 (it touches `cli/` and `tape/` only).

### Phase E: a cheaper update

#### E1. Bodies as columns

- **What:** as in 3.5. A domain stores its bodies as columns; the code reads a body through `BodyRef` and `BodyMut`
  views whose fields keep today's names, so the laws' code changes little.
- **Why:** each body is a struct with its own heap lists; columns keep a domain's bodies together in memory and let
  E2's loops run over arrays.
- **Check level:** 1 (only the layout changes). **Done when:** the golden test is unchanged; the benches are not
  slower. **Kill line:** E1 is the base of E2: if it is slower by more than 3%, E2 is built on today's layout instead.
- **Agent:** Opus.

#### E2. The batched path for bodies without a brain

- **What:** from a profile after E1, the stages of the update of bodies without a brain that cost most are run as
  loops over a domain's due bodies without a brain: what each stage needs is computed once per body (the plateau
  state, the mass and heat capacity, the site), and each stage is one short loop the compiler can vectorise. Bodies
  with a brain keep the per-body path (rule 3). Deaths and children inside a stage are kept as marks and settled
  after it, in the bodies' order.
- **Why:** an update is about 200,000 instructions; the profile of the foundation found the same work (the thermal
  state, sums over stores) done many times in one update.
- **Check level:** 3 (the order of the updates of a domain's bodies interleaves by stage). **Done when:** the bodies'
  stage of the benches is at least 1.3 times faster than after E1; the key numbers hold.
- **Kill line:** a stage that gains under 5% of the bodies' time keeps the per-body path.
- **Agent:** Opus. **Depends on:** E1.

### Phase Z: the final check

- **Z1:** both benches and their profiles; the cost model of a cell and of a body again; the largest planet that
  fits 15 minutes.
- **Z2:** the slow suite (key numbers, equivalence tests, the nodes check), the golden test without threads.
- **Z3:** 1,000 years of the island 22 × 22, the globe 36 × 18, the refined region and the largest planet that fits;
  books, bodies and time.
- **Z4:** the watch report `docs/reports/speed.md` with pictures (decision S2); the handover (sections 5.4, 7, 12.6),
  `docs/laws.md`, `docs/numbers.md` and `CLAUDE.md` describe the present.
- **Agent:** Sonnet for the runs and the report draft, Opus for the final review.

---

## 5. Order and parallel lanes

```
A1 ──► A2 ─────────────────────────────────────────────────────────────┐
  └──► A3                                                                │
  └──► D3 (flow/, solve/) ── runs beside B and C ──────────────────────┤
  └──► B1 ──► B3 ─┐                                                      │
            └► B2 ─┴──► (phase B review, key numbers) ──► C2 ──► C1 ────┤
                                                                         ▼
               D2.1 ──► D2.2 (C3) ──► D2.3 ──► (review) ──► D2.4 ──► D2.5 ──► D2.6 ──► (review)
                                       D1 beside D2
                                                         E1 ──► E2 ──► (review) ──► Z
```

- **A1 first:** every other step needs the baseline.
- **D3** touches only `flow/` and `solve/`: it runs from the start, beside B and C, in its own worktree.
- **B1 before B2 and B3:** both read the node's stages that B1 removes. B2 and B3 run side by side (`nodes.rs`
  against `survey.rs` and `copy/select.rs`).
- **C after B:** C2 integrates the stages that B1 puts in one body.
- **D2 after C:** D2.3 touches every file of the update; it waits until B and C are merged.
- **E after D2:** the columns live inside the domains.
- **Reviews:** after phase A (light), after phase B, after C, after D2.3, after D2.6, after E2, and the final review.

---

## 6. Risks and what to do

| Risk | What to do |
|---|---|
| The refactor of D2.3 never ends | Sub-steps that each run the island and pass the checks; D2.3 moves the code to the context first with no behaviour change where it can (level 1 with one domain per world, then level 3) |
| A speed tool makes the world (B2, C1, C3, node_days) | Its equivalence test is part of its "done when"; a change past noise stops the step |
| The nodes check (F7) breaks more | Run at the end of phase B and of phase D; a new failure is a bug to find |
| Threads give a different history | The golden test with and without threads at every D step; no step closes until it holds |
| The physics' threads change sums | D3 is level 1: the fingerprints must not move |
| Ten times is not reached | `measures.md` says where it stands at every step; the report says it plainly, with what group F would add |
| The code ceiling | Each step before D2.5 pays for its lines; after it, 9,700. Simplify before adding |
| A long run finds a books error late | The benches print the drift every year; phase Z runs 1,000 years on four worlds |

---

## 7. Words used in this plan

| Word | Meaning |
|---|---|
| bench | a settled world kept as a snapshot, run 5 years to measure speed |
| domain | the cells under one air node and the bodies in them, run as one unit |
| batch | the bodies due in `batch_days` days, run together at the batch's first day |
| outbox | what a domain's bodies did that leaves the domain, applied in a fixed order after the batch |
| store view | a domain's store nodes as the moves of its batch leave them |
| face | what others see of a body (visible traits, signal, well-being, id), kept from the batch's start |
| column | one array holding one field of all bodies of a domain |
| kill line | the gain under which a step is taken out again |
