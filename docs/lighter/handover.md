# Handover: where the work stands, and how to go on

For the next agent. Read `CLAUDE.md` first (its rules override everything here), then this file, then the files it
names. Write to the owner in very simple English: short sentences, common words.

## 1. Where things stand

- **`main`** holds the speed x10 round and the round "Lighter world" (compute less). Both are done. Their reports:
  `docs/reports/speed-x10.md` and `docs/reports/lighter.md`.
- **The round "Lighter world"** changed what the world computes. Kept: cheaper saves (F1, F2, F3), the parts' reactor
  search (G1), the skip of bare reactions too slow to matter (B1), children of a node at most once in 30 days (H1), the
  liquid's sink at every physics step (L1). A settled year is x1.10 (island22), x1.30 (globe36), x1.02 (region24) on 4
  threads; 1,000 years x1.22 to x1.41; the largest globe in 15 minutes is size 30 (1,800 cells). Everything tried and
  not kept is in the report, each on its own branch.
- **The objects round is paused, 3 of its 12 steps built**, on the branch **`objects`** (not merged into `main`: its
  laws are half done). Its docs live on that branch: `docs/objects/plan.md` (the steps, the gate, the order),
  `decisions.md` (O1 to O36), `scale.md`, `materials.md`, `measures.md`. Built, reviewed and merged on `objects`:
  - **OP:** the start. Today's world holds few parts (about 22 on globe36, 61 on the 64 x 64 island), nearly all flesh
    or fuel, ended by the parts' reactor passing 1e6 K.
  - **O1:** bonds stored per cell (level 1).
  - **O2:** a part reacts its walls at its ground's temperature (no more 1e6 K); products that do not fit the wall
    (Pilling-Bedworth ratio under 1 or over 2) leave it with their heat; `form` never takes body matter and draws its
    matter by what is within reach over its cost; a node forms and puts one member's share.
  - **O3:** rot keeps a part's outer size and makes its walls porous; strength by (solid share)^(3/2) (Gibson 1981),
    zero under 0.16; joints read their walls as they are; a part falls when its walls cannot bear its own weight;
    `min_kg` only when a part forms. Parts now stay steady (globe36 about 53, the island about 34) and end only by laws.
- **Left in the objects round, in this order** (each its own step, reviewed before its merge, as `plan.md` says): O4 the
  pieces node per cell (dead frame, shells, bones), OB burial, OD binding by drying (mud, from properties only), OF
  liquid and frost in pores (it must end the 47 lime parts lying in the sea on globe36), O5 wear, O6 repair (`fill` must
  react), O7 sleep, O8 dug pieces, OI the same matter (a lone lying part made of its cell's matter joins it), OL the
  pieces' size (probe first), OS the scale probe, O10 part nodes. The gate: no class of parts grows over 1,000 years on
  globe36 and the 64 x 64 island; objects at most 5% of a year; the budgets per object-year at 10^4 to 10^6.
- **Before you build on `objects`:** merge `main` into it (it was made from `speed/lighter` before L1, the drift tests'
  fix and phase Z), remake the benches with its build (snapshot 53), and check the ceiling: the objects round passes
  10,673 lines (the owner approved it for this round; each step raises `CEILING` in its own commit,
  `docs/objects/decisions.md`).

## 2. The owner's rules for this work

- The architect (the agent who leads) takes the decisions and logs them (L-rows for lighter, O-rows for objects); the
  owner does not want to be asked per edit. A dropped idea needs an alternative.
- Laws, not outcomes: everything comes from the five laws and the properties of matter. No timer, no special rule, no
  exception for any Earth-like idea (the owner's words for objects: "only through properties and laws").
- The cycle: plan, adversarial review, build, adversarial review, fix, until accepted. Measures before builds where a
  doubt decides the design (probes on `probe/...` branches, never merged).
- Kill lines: a speed step must give at least 1% of the year for 20 lines or less, 1.5% for 21 to 40, about 3% for a
  law step (L2). Steps for scale (objects) are judged at scale and must not cost more than 1% of today's year.
- Use agents: the strongest for law steps and every review, normal ones for clear code, small ones for gathering facts.
  Each agent works in its own git worktree and branch and never pushes; the architect merges one at a time.

## 3. The brief every agent got

Give each agent these rules (they were in a scratch file):
- Read `CLAUDE.md`, `docs/handover.md`, `docs/laws.md`, the round's plan and decisions, and the design note of its step.
- Check levels: 1 bit for bit (golden unchanged, with and without threads); 2 the same to rounding (golden re-pinned);
  3 the same world (golden re-pinned, the key-number test against the references in `measures.md`); 4 a new law for a
  part of the world (the plan says what must still hold and which tests change). Never loosen a tolerance. Bump
  `SNAPSHOT_VERSION` when the saved `World` or any `Config` field changes, and remake the benches.
- Before a commit: `cargo build --release --workspace`; `cargo test --release --workspace`; `cargo clippy --release
  --workspace --all-targets` (no warnings); `cargo build -p pcore --target wasm32-unknown-unknown`;
  `RAYON_NUM_THREADS=1 cargo test --release -p tape --test golden`; `CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=true cargo
  test --release --workspace --target-dir target/dbg`; for steps that touch worldgen or math, `cargo test -p tape
  --test golden` (plain debug). Slow tests (`-- --ignored`) before the last push of a phase.
- Commits: a first line `<Round> <ID>: <what>`, a short body (what, why, the measure), then `Lines: +X -Y (core code
  N)` and the attribution lines. A law change is its own commit with its physics reason and its effect on a few numbers.
  One line per step in `docs/history.md`; the measure in the round's `measures.md`.
- Measures: callgrind instructions of one year on one thread are the main measure; wall time on 4 threads only under a
  lock, interleaved, with the load said. Across remade worlds only large cuts count.
- Machine care: 4 cores, about 18 GB of free disk. One heavy thing per agent. Delete build folders when done. Kill
  processes by id, never by name.

## 4. Open choices for the owner

The full list is in `docs/reports/lighter.md` ("Open items for you") and `docs/lighter/plan.md`. The main ones:
1. **Plants and lime.** With an honest height credit (HP1, branch `lighter/hp1-not-kept`) plants stiffen with lime (17
   to 34% of dry mass; real plants hold 0.5 to 3% calcium) because chemistry 0 makes lime cheaper than frame. Proposed:
   change that price in a round about life, then HP1; trees and shrubs should follow.
2. **Living bodies do not rot** (left as is by the owner): the law is ready on `probe/pr3` (variant b0); it makes the
   eaters x35.
3. **Small nodes** (branch `study/small-nodes-not-kept`): the old over-draw hides a weakness of young animal nodes
   against individuals; not found yet.
4. **The objects round** above, and its smaller choices in `docs/objects/decisions.md`.
5. **Watch items** of one seed: the end's globe 0.5 K colder in its last 50 years; the end's island with fewer animal
   lineages (2 against 9). Three seeds would say if either is real.
