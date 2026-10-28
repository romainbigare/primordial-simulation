# Rules for working on Emergent World

Read this before you change anything. These rules come from the clean-up after M3, when the code and the docs had grown
full of history, copies and switches. They keep the code a clean base. If a rule here and `docs/handover.md` disagree,
ask the owner.

## 1. Where things are

| What | Where |
|---|---|
| What we build, the principles, the plan | `docs/handover.md` (the spec) |
| How each law works now | `docs/laws.md` |
| Every free number: unit, anchor, mark (physical, anchored, tuned) | `docs/numbers.md` |
| What the tape records | `docs/tape.md` |
| What changed and when, what was tried and not kept | `docs/history.md` |
| The work now: the objects round (paused) | Start with the handover `docs/lighter/handover.md`. The round lives on the branch `objects` (not merged): its plan, decisions and designs are in `docs/objects/` there |
| Lighter world (done) | The result: `docs/reports/lighter.md`. The plan, the decisions (L1 to L47), the measures, the designs and the handover: `docs/lighter/` (`plan.md`, `decisions.md`, `measures.md`, `design/`, `handover.md`). The work is merged into `main`. The earlier rounds' rules below hold |
| Speed x10 (done) | The result: `docs/reports/speed-x10.md`. The plan, the decisions (X1 to X88), the measures and the designs: `docs/speed-x10/` (`plan.md`, `decisions.md`, `measures.md`, `handover.md`, `design/`). The work is merged into `main`. The earlier rounds' rules below hold |
| Speed after the foundation (done) | The result: `docs/reports/speed.md`. The plan, the decisions (S1 to S84) and the measures: `docs/speed-post-foundation/` (`plan.md`, `decisions.md`, `measures.md`). Its rules in 2 of the plan and the foundation's main rule in `docs/foundation/plan.md` hold for any new speed work |
| The foundation before M4 (done) | `docs/foundation/plan.md` (read its main rule first) |
| The laws | `core/src/<law>/` (`flow`, `change`, `bond`, `copy`, `choose`), bodies in `core/src/body/` |
| Physical constants and units | `core/src/units.rs` |
| Tuned numbers | `Physics` and `Config` in `core/src/params.rs` (each with a row in `docs/numbers.md`) |
| Shared physics helpers | `core/src/ground.rs`, `core/src/chem/`, `core/src/math.rs` |

## 2. Writing code

1. **One place for each idea.** Before you write a formula, search for it. Moving matter with its heat, latent heat,
   the melting rule, a ground temperature, a gene distance: each has one helper. Use it or extend it. Never copy it.
2. **One folder per law.** A file stays under about 350 lines, a function under about 80. Split a long function into
   named stages that say what they do.
3. **No magic numbers.** A physical constant lives in `units.rs`. A free number of a law lives in `Physics` or
   `Config` with its row in `docs/numbers.md`. A number that is part of how the code works (a tolerance, a cadence) is a
   named `const` next to its use, with a one-line doc. Small guards against a division by zero (`.max(1e-9)`) may stay
   as literals.
4. **No dead code.** No unused functions, fields, exports, counters or arguments. Delete them; git keeps them.
5. **No compatibility code.** No `#[serde(default)]`, no readers of old files, no padding for old snapshots. When the
   saved `World` changes, bump `SNAPSHOT_VERSION` in `tape`. Old snapshots and tapes are not supported.
6. **No experiment switches on main.** Try an idea on a branch. A switch stays only if it is a control run that a law is
   checked against (today: `--individuals`, `--no-mix`, `--null`).
7. **Counters are not state.** `World::counts` holds numbers for the tape only; no law may read them. Anything a law
   reads is simulation state.
8. **Clear names.** No two different things with the same name in different modules. No reused variable names for
   different quantities inside one function.
9. **No Earth words in `core/` and `worldgen/`** (`cli/tests/word_ban.rs`), including substance names (nitrogen,
   carbon, salt, sugar...). Use numbers and physical words.
10. `cargo clippy --release --workspace --all-targets` gives no warnings.
11. **Size.** `core` holds at most 10,673 lines of code (`docs/lighter/decisions.md` L4, `docs/speed-x10/decisions.md` X22 and X88; the owner said yes to going past
    the 20% of `docs/handover.md` 14.3), not counting comments and blank lines (`docs/handover.md` 14.3; the test `cli/tests/core_size.rs` counts them). Simplify before
    you add. Never delete useful comments to save lines: they are not counted. Never pick a slower or less honest
    design to save lines.
12. **Replace, do not patch.** When you build a general piece, delete the special code it replaces in the same change
    (the main rule of `docs/foundation/plan.md`).

## 3. The physics must stay honest

1. **Everything is kept.** Each element, the energy and the members of a node. Every transfer books both sides through
   a shared helper. A drift above about 1e-12 is a bug: find it, do not hide it. **The books are 64-bit:** numbers that
   hold matter or energy stay 64-bit, and each move is booked on both sides with one 64-bit number. 32-bit is for
   numbers that hold nothing (cards, brain rows, faces, rates), and only where a step measures a gain (`docs/speed-x10/decisions.md` X4).
2. **Counts are real.** A member count is never rounded up (`Body::n()` is "at least one" only where the physics means
   one body). Means over a node or a group are weighted by members.
3. **One source for each quantity.** One ground temperature (`World::t_ground_last`), one melting rule, one way to pay
   for lifting gas into the air. When two places need the same number, they read the same thing.
4. **A law never overdraws a node.** It never asks a node for more than it holds; where it could, it limits that
   node's own outgoing links, never a whole field. `store::apply` only adds, sets rounding below zero to zero and
   counts it. A real overdraw breaks the books.
5. **Validate inputs in one place.** `Chemistry::validate` and the config checks refuse bad worlds with a clear error
   (limits like `MAX_SUBSTANCES`, periods that must divide each other, numbers that must be positive).
6. **One writer for each number.** In a parallel phase each number has one task that may write it: a domain its
   bodies, its cells and its view; a booker one array of the store (`store::book_by_array`). What a task does to
   another task's numbers waits in its outbox and lands in the next phase, in the task that owns it, in an order fixed
   by the data (domain, cell, id), never by which thread ends first. Per-domain work runs on the one runner
   (`par::longest_first`; `survey::run` is its use for body domains).

## 4. Comments and docs

1. **A comment says what the code does and the physical reason**, with a literature reference (author year) when there
   is one. Present tense. Say it once, where it belongs.
2. **Never write history in code or in the spec.** No dates, no "review 4, finding 3", no "it was 8.6 before", no run
   results ("seed 1, year 60: ..."), no "fixed" or "bug" notes. History goes in `docs/history.md`, one line per change,
   and in commit messages.
3. **Doc comments of `Physics` and `Config` fields are one line** (unit and meaning). The anchor and the reasoning are in
   `docs/numbers.md`, one short row per number.
4. **Docs describe the present.** When a law changes, change the sentence in `docs/laws.md` in the same commit. Do not
   add a dated paragraph next to the old one.
5. **No study files in the repository.** Prototypes, patches, probe runs and their pictures live on branches. Only the
   watch report of a milestone goes in `docs/reports/`.
6. Write to the owner in very simple English: short sentences, common words.

## 5. Tests

1. **Few, fast and meaningful** (`docs/handover.md` 12.3). A test states a law or an invariant, not the story of a bug.
2. Use the shared kit `worldgen::testkit` for setup. Never copy setup code between test files.
3. **Slow tests are `#[ignore]`** (`cargo test --release --workspace --no-fail-fast -- --ignored`): run them before the
   last push of a phase, and the golden test also without threads (`cargo test --release -p tape --test golden`). There is no CI on GitHub for now. The
   normal suite stays under about 30 seconds.
4. **The golden test** (`tape/tests/golden.rs`) pins the fingerprint of the whole world for three small runs. A pure
   clean-up must keep it. A change of the laws, or speed work that changes the order of sums, re-pins it in the same
   commit, and the commit says why.
5. Never loosen a tolerance to make a test pass. Never restore old physics inside a test.
6. Delete the tests of a mechanism you delete.

## 6. Working

1. **A pure clean-up keeps the results bit for bit.** Check the golden test, and compare a few short runs (same
   fingerprint, same `series.csv`). Keep the order of floating-point sums; `a*b*c` is not `a*(b*c)`.
   **Speed work may change the order of sums** (check level 2 of `docs/foundation/plan.md` 6.1): it re-pins the golden
   test and shows the same world (a few series columns, or the key numbers where life's choices amplify rounding).
   **The number of threads does not change the world:** every order is fixed by the data, so the golden test gives the
   same fingerprints with and without threads (`docs/speed-x10/decisions.md` X3).
2. **A change of the laws is its own commit**, with the physics reason and its effect on a few numbers of a long run
   (populations, temperatures, drift).
3. Before you push: build, all tests, clippy, the wasm build (`cargo build -p pcore --target wasm32-unknown-unknown`).
4. Reviews of the code and of the laws happen in rounds with the owner. What stays banned is proof machinery: receipts,
   cross-machine determinism, replay proofs (`docs/handover.md` 14.5).
5. **The air's solver tolerances are speed numbers** (`STILL_K`, `STILL_SHARE`, `CALM_K`, `CALM_SHARE` in
   `core/src/flow/air/step.rs`), like a time step. A change re-pins the golden test and must pass the long-step test and
   the globe's climate test (`tape/tests/ranges.rs`), in the same commit.
