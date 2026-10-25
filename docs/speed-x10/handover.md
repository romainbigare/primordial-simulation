# Speed x10: handover to the next session

This file is for a new session (a new agent) that continues the speed x10 work. Read it first, then the files it
points to. It is written in very simple English, as the owner wants for everything written to them.

## 1. Where things are

| What | Where |
|---|---|
| The branch of this work | `speed/x10` (pushed). `main` holds the last round as it ended and is **not** changed by this work until the owner merges the branch (X1) |
| The plan (the single source of the steps) | `docs/speed-x10/plan.md` |
| The decisions of this round (X1 and on) | `docs/speed-x10/decisions.md` |
| The low-level designs | `docs/speed-x10/design/` (`lane-a.md`, lane A after P1.2, which replaces `month.md` (not kept) and parts of `plants.md`, `animals.md` and `gap.md`; `plants.md`, `engine.md`, `air.md`, `minds.md`, `gap.md`, `animals.md`); each has a note at its top saying which of its steps are kept, dropped or renamed. Where a design and the plan disagree, the plan wins |
| The measures of this round | `docs/speed-x10/measures.md` (created by R2; see section 4 here for its state) |
| What changed and when | `docs/history.md`, section 6 "Speed x10" |
| The rules | `CLAUDE.md` (already updated by R1 for this round: ceiling 12,000, 64-bit books, the world the same with any number of threads) and `docs/handover.md` (the spec) |
| The last round (background) | `docs/reports/speed.md` (the result: about 1.4 to 2 times; why not 10), `docs/speed-post-foundation/` (its plan, decisions S1 to S84, measures) |

## 2. The goal and the honest outlook

- **Goal: 10 times.** A settled year on 4 cores, 10 times faster than at the start of the last round: island22
  0.91 s to about 0.09 s; globe36 1.49 s to about 0.15 s. Today (start of this branch): island22 about 0.61 to 0.66 s,
  globe36 about 0.74 to 0.81 s.
- **The owner's rules for this round:** bad ideas are dropped from the start; a review or step that drops a strategy
  must propose an alternative (with its gain and the rule or law it changes); laws and ground rules may change for
  speed; the three rules for smart bodies hold (individuals stay real; the step length per place; kinds of body code).
  The architect takes all decisions and logs them in `decisions.md`. The owner does not want to be asked for permission
  for each edit.
- **Expected result (plan section 5.0, after P1.2):** "likely" about 4.0 to 5.4 times on the island and 5.9 to 8.1 on
  the globe with every step; 10 times is out of reach with the steps planned. The steps that carry most: **P2.2 and
  G9a** (stands at 60 and 90 days on today's law, after the probe PR-C2), **G2** (a node decides about every 30 days,
  its body keeps its gene's pace) and **G14** (physics beside the bodies).

## 3. What is done (on `speed/x10`)

| Commit | What |
|---|---|
| 43e6efc .. 8f7486a, 6535132 | The plan, its designs and decisions, through three adversarial review rounds; **approved** in round 3 (six small fixes applied in 6535132) |
| 4cc7b81 | **R1**: the rules true at once (CLAUDE.md 2.11, 3.1, 6.1, its table; handover 12.1, 12.3, 12.5, 14.3, 14.5; `cli/tests/core_size.rs` `CEILING` 12,000) |
| f579a5d | **R3**: the long-step test runs a world `Config::validate` accepts (`phys_days` 2) and every `tape/tests` world is validated; the liquid's clock counts a `flow_days` over a month from the world's start (`schedule::days`); `validate` refuses a `flow_days` over a month that is not a whole number of months; a new test. The long-step test passes (t_land 0.145 K, seed 1 at 0.306 K) |
| ac8a3f2 | **AIR4.0, a law fix** (golden re-pinned): in `Surface::balance` (`core/src/flow/air/radiation.rs`) a melting plateau used up inside an air step held for the whole step and restarted on the wrong side of the fusion enthalpy (a sign error), and the snow cover's cap held after the snow was gone. Now the node stays on the plateau only for its share of the step. Long-step test t_land 0.145 to 0.118 K, winter 0.026 to 0.002 K; a new test `a_plateau_used_up_in_one_long_step_ends_where_short_steps_end`; the test `liquid_wears_its_bed_with_the_work_its_load_leaves_free` now holds its land's temperature (`hold_ground`) |
| 413f552, fb6eb94 (merged 3162b9d) | **R2**: every gate measured on benches remade in one hour, and the key-number references fixed (`measures.md`, section "R2"; its table "The gates at a glance" and "What is measured and what is not yet") |

Probes done (pushed branches; their results are in `measures.md`):
- **PR-B** (`probe/brain-loss`): no body lost its brain in 1,000 years of island22 or globe36 (seed 1). The switch in
  P1.2 is still needed by the law (X15) and is tested by a test kit case. In `measures.md`.
- **AIR4-R1** (`probe/air4-r1`, commit a74a2f5): why 30-day air steps warm the land (+0.71 K, winter +1.17 K over 6
  seeds). Most of it was the plateau fault fixed in AIR4.0. What is left after the fix (+0.29 / -0.10 / +0.86 K) is
  backward Euler's first-order phase error; calm months picked by hand only move it to another season. AIR4 should
  build TR-BDF2 (second order) in calm months chosen by its error estimate, with events (melt-out, freeze-through,
  the cover cap, the bucket drying) handled inside each stage. A Richardson estimate puts it near the 0.3 K limit
  (+0.19 / +0.26 / +0.08 K). Expected Flow x1.2 to x1.5 for 6 to 12 calm months; the kill line x1.4 needs about 10
  calm months. A smaller fault is left: a plateau not used up also drifts in a long step (a 20 cm ice cover 73%
  melted after one 3-day step, 61% after short steps).

## 4. What is not done yet, in order

1. **R2 is done** (`measures.md`). What it found, in short: plant updates are 30% of island22's year (about 37% with
   their share of the rest), so P3 stays; stands differ from their ground's temperature by over 3 K in about half of
   their updates, so P1.3's stage 6 must book that step; E2's gate is met (busiest share at 4 cells 0.43 / 0.55 /
   0.26 of today's); E4's gate is met (booking 9% of a 4-thread year); calm cells (AIR1) skip rows mainly on the
   globe (27%; island 2%); **MI3a's gate fails** (sleep share 14 / 11 / 29%, the gate is 40%: by X36 MI3a is re-gated
   after G2 and will likely not be built); MI4's gate is met (sharing 1.95 / 2.17); G15 is 0.02 to 0.11 from its line;
   the key numbers are all in range except S50. Weak spots it names: some shares are proxies, tiles were counted not
   timed, the wall times were taken on a busy machine (judge by instructions).
   **Its benches lived in the scratchpad, which is gone with the container.** AIR4.0 (a law fix) also landed after
   them. So the next session first remakes the benches on the branch's head (`scripts/bench.sh --remake` with
   `BENCH_DIR` set, a few minutes each), and takes the key-number references there.
2. **The probes that need the benches** (plan section 4, phase P): **PR-M** (the plant month prototype: µs per stand, error
   against `body::live` at 2 and 6 days, a first look at `D` = gene against `D` = 36), **PR-A** (what starves animal
   nodes at a 30-day pace, cause by cause), **PR-S** (the noise of the judging method for steps that change the
   snapshot), **G14-P1** (the first overlap probe: the air step and the batch joined; month windows timed apart; the
   serial part reported).
3. **The review after phase P:** the architect restates plan section 5 with R2 and the probes. If PR-A and G14-P1
   both fail, the owner is told the likely end is about 6 times (island) and 8 (globe).
4. **Phase 1, five lanes** (plan section 4): A plants and animals (P1.1 → P1.2 → P1.3 → P1.4 → P2 → P1b); B engine
   (E1a → E1b.1 → E1b.2 → E1b.3 → E1c → E2; MI1 lands before E1b.1); C air (AIR1 → AIR3 → AIR2, after E1c); D minds
   (MI1, MI5); E no-law steps (G8, then G1, G7 after E1c, G15 after E2). Then phase 2 (G4, G5, G2, G3, P3, G9a/b,
   G12, AIR4 TR-BDF2, G11, G16, MI3a/MI4/MI3b re-gated after G2, E4/E5 gated, G14-P2 then G14 last; GM and E7
   measures) and phase Z.

## 5. How the work is done (the method the owner asked for)

- **Agents.** Opus 5.5 for complex code (law changes, structure of the update) and for every adversarial review;
  Sonnet 5.5 for normal code, tests and docs; Haiku 5.5 for data and measures. Steps on different files run in
  parallel, each in its own git worktree, and merge one at a time into `speed/x10` (fast-forward when possible, or a
  cherry-pick; keep both sides of `docs/history.md` and `measures.md` conflicts).
- **After each big step** (each lane's chain, phase 2, Z): an adversarial Opus review, then an Opus fixer, until the
  reviewer says `VERDICT: APPROVED`. A step is judged on its own kill line; a step under its kill line is reverted and
  logged in `history.md` ("tried and not kept") with its branch pushed.
- **Commits.** One step per commit; a law change is its own commit with the physics reason and its effect on a few
  numbers (CLAUDE.md 6.2); every commit message ends with `Lines: +X -Y (core code N)` (N from
  `cargo test --release -p cli --test core_size -- --nocapture`) and then the lines
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` and the session line. Push `speed/x10` after each merge.
- **Before a push:** `cargo build --release --workspace`, `cargo test --release --workspace` (about 18 s),
  `cargo clippy --release --workspace --all-targets` with no warnings, `cargo build -p pcore --target
  wasm32-unknown-unknown`, the golden test without threads (`cargo test --release -p tape --test golden`). Level-3
  steps re-pin golden in their commit and run the key-number test once
  (`cargo test --release -p tape --test ranges the_key_numbers -- --ignored --nocapture`). At the end of a phase: the
  slow suite (`cargo test --release --workspace --no-fail-fast -- --ignored`, about 70 minutes) and the debug checks
  (`cargo test -p pcore --test outbox` and the store, body, copy, choose tests without `--release`).
- **Measures.** Callgrind instructions of a year on one thread (`scripts/bench.sh profile`) for most steps; wall time
  on 4 threads with 5 or more interleaved runs on a quiet machine for thread steps. Every timed run takes the lock
  `flock <scratchpad>/x10/timing.lock`, and waits while other builds run. Remade benches are not comparable across laws
  (S65): judge a step on one world (the same save with both builds), or per update on each build's own bench when the
  snapshot changes (X24).
- **Disk** is limited (about 250 GB shared, often 15 to 20 GB free): every agent deletes its scratch builds
  (`target/`) when done; remove finished worktrees (`git worktree remove -f -f <path>`).

## 6. Known open items for the owner

- **S50** (from the last round): the light-eaters' stored share fails its key-number range (0.556 against "under 0.5");
  known since the foundation.
- **S84**: the sleep of Change biases the sea's gas 3 slightly; a fix is on the branch `sleep/pair-calm-proposal` and
  costs about 20% of a run; a law question.
- **S83**: the canopy check takes the densest 5% of land cells (a rank) since plants weigh about half.
- **X14**: the stands' reserve `D` (the gene's interval, or today's 36 days) is chosen by P1.2's key-number runs.
- **X22**: the core ceiling is 12,000 with the owner's yes ("whatever is sensible").
- The last round's report (`docs/reports/speed.md`) lists its own open items (S49, S59/S66, the C2 finding, phases B
  and C).

## 7. Branches

| Branch | What |
|---|---|
| `speed/x10` | this work |
| `main` | the last round's end; merged into `speed/x10` at aae6d68; merge it again at each lane review and before Z (X1) |
| `probe/brain-loss`, `probe/air4-r1` | the probes PR-B and AIR4-R1 (never merged) |
| `sleep/pair-calm-proposal` | S84's fix proposal |
| `speed/e1-probe`, `speed/e23-*-not-kept`, `speed/b*-not-kept`, `speed/c2-not-kept` | the last round's tried-and-not-kept work |
