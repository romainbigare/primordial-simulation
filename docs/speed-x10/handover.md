# Speed x10: handover to the next session

The speed x10 round is done. This file is for a new session. Read it first, then the files it points to. It is written
in very simple English, as the owner wants for everything written to them.

## 1. Where things are

| What | Where |
|---|---|
| The result, for the owner | `docs/reports/speed-x10.md` (where the speed stands, what was kept and not kept, why not 10 times, the open items) |
| The branch of this work | `speed/x10`. `main` holds the last round as it ended and is **not** changed by this work until the owner merges the branch (X1) |
| The plan, with the status of every step | `docs/speed-x10/plan.md` (each row says done, not kept or not built) |
| The decisions of this round | `docs/speed-x10/decisions.md` (X1 to X88) |
| The measures of this round | `docs/speed-x10/measures.md` (one section for each step; "Z: the year at the round's end" and "Z: the final checks" are the end) |
| The low-level designs | `docs/speed-x10/design/` (each has a note at its top saying which of its steps are kept, dropped or renamed; `lane-a.md` replaces `month.md`) |
| What changed and when | `docs/history.md`, section 6 "Speed x10" |
| The rules | `CLAUDE.md` and `docs/handover.md` (the spec). The core ceiling is 10,643 code lines (X88) |
| The rounds before | `docs/reports/speed.md` and `docs/speed-post-foundation/`; `docs/foundation/plan.md` |

## 2. Where it ended

- **The goal was 10 times; the result is about 2.6 times (island) and 2.9 (globe)** against the last round's baseline.
  Against this round's start a settled year on 4 cores is x1.77 (island22: 460 to 260 ms), x1.45 (globe36: 522 to 361)
  and x1.44 (region24: 761 to 529). 1,000 years: island 5.6 min, globe 5.2 min, region 11.8 min. The largest globe that
  runs 1,000 years in 15 minutes has size 28 (1,568 cells).
- **The books are exact** (drift under 2e-14 in 1,000 years), the world is alive and the key numbers are in range but
  S50 (known).
- **Built and kept:** the engine chain (E1a to E1d), G1, G6, G7 (the physics on the pool), AIR1, AIR3, G11 (the air),
  G2, G4, G12, MI5, MI1, G8, G15 (mimalloc), PL1, R3 and AIR4.0 (fixes). **Not kept or not built:** the month law (P1.2),
  the plant pace (PR-C2, P2.1, P2.2, P3, G9a, G13), AIR2, G5, AIR4, G14, and the gated E2, E4, E5, G9b, MI4, G16. The
  report has the reasons and the branches.
- **The slow suite** ran at the end of phase Z: 14 of 17 pass; S50 and S84 fail (known); the individuals test was
  fragile on seed 1 and now runs seeds 1 to 3. The debug checks pass.
- **Core:** 10,443 code lines; the ceiling is 10,643 (X88).

## 3. Open items (the report has the full list)

- S50 (stored share) and S84 (the sleep bias on `air_3`; a fix on `sleep/pair-calm-proposal` costs about 20% of a run).
- The liquid's 60-day step warms the land about 0.2 K; the long-step test passes at +0.293 K of its 0.3 K limit.
- The plants' copy-mass gene climbs at long paces under every law (PR-C2); its cause is not found.
- The noise of `off_by_noise` includes the trend; the sleep test judges seed 1 only.
- A node of 1 to 2 members in `choose/spread.rs` `stays` acts whole, so the mean of the members that act is above
  `count x chance` (a law question).
- Where the next speed is: the body batch (`survey::update` is 42% to 54% of a year on one thread, `to_growth` half of
  it). PL1's section in `measures.md` lists what is left and what each piece would cost in law.

## 4. How the work was done (for a new round)

- **Agents.** The architect takes the decisions and logs them; Opus for law changes and every adversarial review; Sonnet
  for normal code, tests and docs; Haiku for measures. Steps on different files run in parallel, each in its own git
  worktree, and merge one at a time. After each lane, an adversarial review and a fixer, until `VERDICT: APPROVED`.
- **Commits.** One step per commit; a law change is its own commit with the physics reason and its effect on a few
  numbers (`CLAUDE.md` 6.2). Each message ends with `Lines: +X -Y (core code N)`
  (`cargo test --release -p cli --test core_size -- --nocapture`).
- **Before a push:** build, all tests, clippy, the wasm build, the golden test without threads; level-3 steps run the
  key-number test once; at the end of a phase the slow suite and the debug checks.
- **Measures.** Callgrind instructions of a year on one thread for most steps; wall time on 4 threads with 5 or more
  interleaved runs on a quiet machine, under one lock. A step that changes the saved world is judged on one world: the
  base's save turned to the step's snapshot (X50). Remade benches are not comparable across laws (S65).
- **Disk** is limited: delete scratch builds when done.

## 5. Branches

| Branch | What |
|---|---|
| `speed/x10` | this work |
| `main` | the last round's end; merged into `speed/x10` at aae6d68 |
| `x10/p1.2-not-kept`, `probe/pr-c2`, `x10/air2-not-kept`, `x10/g5-not-kept`, `x10/air4-not-kept` | tried and not kept |
| `probe/brain-loss`, `probe/air4-r1`, `probe/gm` | the probes PR-B, AIR4-R1 and GM (never merged) |
| `sleep/pair-calm-proposal` | S84's fix proposal |
| `speed/e1-probe`, `speed/e23-*-not-kept`, `speed/b*-not-kept`, `speed/c2-not-kept` | the round before's tried-and-not-kept work |
