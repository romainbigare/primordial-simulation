# Rules for working on Emergent World

Read this before you change anything. They keep the code a clean base. If a rule here and `docs/handover.md` disagree,
ask the owner.

## 1. Where things are

| What | Where |
|---|---|
| What we build, the principles, the plan | `docs/handover.md` (the spec) |
| How each law works now | `docs/laws.md` |
| Every free number: unit, anchor, mark (physical, anchored, tuned) | `docs/numbers.md` |
| What the tape records | `docs/tape.md` |
| The plan, the priorities, the owner's decisions | `review/roadmap.md` |
| How to lead the work (for the orchestrator) | `review/orchestrator.md` |
| The laws | `core/src/<law>/` (`flow`, `change`, `bond`, `copy`, `choose`), bodies in `core/src/body/`, plants in `core/src/cover/`, the ground in `core/src/space/` |
| How the map is drawn | `tape/src/ground/` (tiles) and `web/` (the page) |
| How the page is looked at and measured | `web/look/` |
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
5. **No compatibility code.** No `#[serde(default)]`, no readers of old files. When the saved `World` changes, bump
   `SNAPSHOT_VERSION` in `tape`. Old snapshots and tapes are not supported.
6. **No experiment switches on main.** Try an idea on a branch. A switch stays only if it is a control run that a law is
   checked against (today: none).
7. **Counters are not state.** `World::counts` holds numbers for the tape only; no law may read them.
8. **Clear names.** No two different things with the same name in different modules. No reused variable names for
   different quantities inside one function.
9. **No Earth words in `core/` and `worldgen/`** (`cli/tests/word_ban.rs`), including substance names. Use numbers and
   physical words.
10. `cargo clippy --release --workspace --all-targets` gives no warnings.
11. **Size.** Keep `core` small by computing less, never by cramming lines. There is no line count to pass.
12. **Replace, do not patch.** When you build a general piece, delete the special code it replaces in the same change.

## 3. The physics must stay honest

1. **Everything is kept.** Each element, the energy and the members of a node. Every transfer books both sides through
   a shared helper. A drift above about 1e-9 is a bug: find it, do not hide it. **The books are 64-bit:** numbers that
   hold matter or energy stay 64-bit, and each move is booked on both sides with one 64-bit number. 32-bit is for
   numbers that hold nothing (cards, brain rows, faces, rates).
2. **Counts are real.** A member count is never rounded up (`Body::n()` is "at least one" only where the physics means
   one body). Means over a node or a group are weighted by members.
3. **One source for each quantity.** One ground temperature (`World::t_ground_last`), one melting rule, one way to pay
   for lifting gas into the air. When two places need the same number, they read the same thing.
4. **A law never overdraws a node.** It never asks a node for more than it holds; where it could, it limits that
   node's own outgoing links, never a whole field. `store::apply` only adds, sets rounding below zero to zero and
   counts it. A real overdraw breaks the books.
5. **Validate inputs in one place.** `Chemistry::validate` and the config checks refuse bad worlds with a clear error.
6. **One writer for each number.** In a parallel phase each number has one task that may write it. What a task does to
   another task's numbers waits in its outbox and lands in the next phase, in the task that owns it, in an order fixed
   by the data (domain, cell, id), never by which thread ends first.

## 4. Comments and docs

1. **A comment says what the code does and the physical reason**, with a literature reference (author year) when there
   is one. Present tense. Say it once, where it belongs.
2. **No history anywhere: not in code, not in docs. Git holds it.** No dates, no "it was 8.6 before", no run results,
   no "fixed" or "bug" notes.
3. **Doc comments of `Physics` and `Config` fields are one line** (unit and meaning). The anchor and the reasoning are in
   `docs/numbers.md`, one short row per number.
4. **Docs describe the present.** When a law changes, change the sentence in `docs/laws.md` in the same commit.
5. **No study files in the repository.** Prototypes, probe runs and their pictures live on branches.
6. Write to the owner in very simple English: short sentences, common words.

## 5. Tests

1. **Few, fast and meaningful** (`docs/handover.md` 12.3). A test states a law or an invariant, not the story of a bug.
2. Use the shared kit `worldgen::testkit` for setup. Never copy setup code between test files.
3. **Slow tests are `#[ignore]`** (`cargo test --release --workspace --no-fail-fast -- --ignored`): run them before the
   last push. The normal suite stays under about 30 seconds.
4. Never loosen a tolerance to make a test pass. Never restore old physics inside a test.
5. Delete the tests of a mechanism you delete.

## 6. Working

1. **A pure clean-up keeps the results bit for bit.** Compare a few short runs (same fingerprint, same
   `series.csv`). Keep the order of floating-point sums; `a*b*c` is not `a*(b*c)`.
2. **A change of the laws is its own commit**, with the physics reason and its effect on a few numbers of a long run
   (populations, temperatures, drift).
3. Before you push: build, all tests, clippy, the wasm build (`cargo build -p pcore --target wasm32-unknown-unknown`).
