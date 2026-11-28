# How we work

How to build, test, look at pictures and lead helpers on this repository. The rules are in `CLAUDE.md`; the plan and
the owner's decisions are in `review/roadmap.md`.

## 1. The owner

- Writes short messages and reads very simple English only: short sentences, common words. Bad news first.
- Gives a direction and expects it done. The lead agent plans, reviews and merges; helpers (subagents) write the code.
  At most two helpers that build code at once (Opus, or Sonnet for narrow clean-ups: check its claims).
- Judges by pictures. Every change of what the world or the page shows comes with before | after pictures: same seed,
  date and place. Look at them yourself first: work can pass every test and still look worse.
- Wants tests run sparingly: one full gate for a merge; the slow tests only when a law changed.

## 2. The machine

Work runs in a cloud container: 4 cores, about 16 GB of memory, **no graphics card**, a fixed disk allowance. The
container starts fresh and can restart without warning: commit early, push often. Anything not pushed is lost.

The cloud environment `primordial-simulation` has a setup script that prepares each new container:
the system's EGL (for llvmpipe, section 6), the WebAssembly target, and a first build (`fast` profile, the page and
its wasm). In a container without it, run:

```
apt-get install -y --no-install-recommends libegl1 libegl-mesa0   # apt-get update first if it fails
rustup target add wasm32-unknown-unknown
cargo build --profile fast --workspace --all-targets
cd web && npm run build && npm run wasm
```

## 3. Build profiles

| Profile | Use | What it is |
|---|---|---|
| `fast` | all daily work: builds, tests, runs, the picture tool's worlds | release without link-time optimisation, 16 parts, incremental |
| `release` | the gate before main, and timing the core's speed | link-time optimisation, one part: the fastest code, slow to build |
| `wasm` | the page's WebAssembly (`npm run wasm`) | release without debug info |

`fast` gives the same worlds bit for bit as `release` (same `hash`, same `series.csv`); its code runs about 10 %
slower. Measured on this machine:

| Step | `release` | `fast` |
|---|---|---|
| A change in `core`, then `ew` built | 85 s | 15 s |
| A change in `core`, then all test programs built | 6.5 min | 20 s |
| Cold build of the workspace with its tests | 7 min | 2 min |

## 4. The daily loop

1. Change the code.
2. Build what you run: `cargo build --profile fast -p cli` (the binary is `target/fast/ew`).
3. Run only the tests of what you changed: `cargo test --profile fast -p pcore --test flow` (one test file, seconds).
4. Run a world: `target/fast/ew run --seed 1 --size 32 --band 20,65 --roster --years 20 --out DIR` (about 10 s).
   Without `--fauna-year` no takers (animals) are placed: runs and tests are without animals for now.
5. For a change of the page or its shaders: `cd web && npm run build`, then pictures (section 6).

The whole fast suite (`cargo test --profile fast --workspace`) takes about 2 min to run. The slow tests are
`#[ignore]`: `cargo test --profile fast --workspace --no-fail-fast -- --ignored`, only when a law changed.

## 5. The gate (before every push to main)

Run it in the background; it takes 10 to 15 minutes from cold.

```
cargo build --release --workspace --all-targets
cargo test --release --workspace --no-fail-fast 2>&1 | grep "^test result" | awk '{p+=$4; f+=$6} END {print "passed",p,"failed",f}'
cargo clippy --release --workspace --all-targets      # no warning
cargo build -p pcore --target wasm32-unknown-unknown
cd web && npm run build && npm run wasm
```

For a change of the laws, also the slow tests (`-- --ignored`: 9 tests, in `core/tests/flow.rs`, `core/tests/cover.rs`,
`tape/tests/ranges.rs` and `tape/tests/scale.rs`). The key numbers' references (`tape/tests/ranges.rs`)
are measured again only for a change of the laws that moves the world on purpose, in a commit of its own, and the
owner is told.

**A pure clean-up keeps results bit for bit.** Check it with a pair of runs, before and after: the same `ew run`, equal
`hash` in the last line, equal `series.csv` but its `ms_*` columns.

## 6. Pictures of the page (`web/look/`)

`cd web && npm run look -- <command>`; the commands are in `web/look/README.md`. It needs `npm run build` and
`npm run wasm` first.

- The browser draws WebGL2 with **Mesa llvmpipe** (software, every core). The tool prints `renderer: ...` at start.
  If it says SwiftShader, install the EGL packages (section 2): SwiftShader is twice as slow.
- **The first picture of a page costs about 1 to 1.5 min**: the browser makes the shaders' machine code. Every picture
  after it in the same page costs 1 to 5 s. So take many pictures in one command (`set`, `years`, `cost`), not many
  `shot` commands. The `seed1` set (28 pictures) takes under 5 min.
- A picture that is not whole within 5 min is never saved: the tool stops with an error.
- Frames per second in this browser mean nothing. The measure of a pixel's cost is the count (`look cost`).
- The worlds the tool opens are made by `ew` (`fast` profile) and kept in `out/look/worlds/` until the code of `core`,
  `worldgen`, `tape` or `cli` changes.

## 7. The owner's live page

The owner tests the page as a private artifact: https://claude.ai/artifact/GKxSQCDfgBAwwL2B8RvEuC. It shows `main`.
Their page for tests and comments is https://claude.ai/artifact/RN1QQ9J1awdrSS3iJ8cmSY.
To update it after a push to main:

1. `cd web && npm run build && npm run wasm`.
2. Copy `dist/ew.wasm` and the pictures beside the page (the detail's files that `web/textures/strips.json` names,
   `fire.jpg`, `snags.png`, each with its `@256`, `@128`, `@64` copies) to a scratch folder, and the page without its doctype, its
   `html`, `head` and `body` tags, its `meta charset` and `meta viewport` tags and its `link rel=icon`.
3. Publish with the Artifact tool to that URL: the page as `file_path`, and in `files` `ew.wasm` (content type
   `application/wasm`) and each picture under its own name. A republish keeps the files left out. If it is refused
   because the page was not read in this session, read it first.

`index.html?world=<URL of a snapshot>` opens a saved world paused at its date. The page's own new world is
`ew run --seed 1 --size 32 --band 20,65 --roster --fauna-year 52 --years 43 --snapshots-at 42.75 --out DIR`.

## 8. Leading helpers

- Give each helper a full brief: the goal, the owner's own words, what is known, the files, what not to touch, what
  to measure before and after, the gate, the report's form (simple English, bad news first), and: commit, never push,
  no pull request.
- Each helper works in its own worktree (`isolation: worktree`). A new worktree starts from `origin/main`: tell the
  helper to merge the work branch first.
- Measure first: a table of numbers before any change, then one change per commit. A helper stops and asks when a key
  number leaves its range.
- A helper cannot give permission. An action denied to a helper is not done for it.
- When helpers meet their usage limit, start them with `model: "opus"`.
- Before merging: every commit signed (`git cat-file -p <commit> | grep -c '^gpgsig'` gives 1), the pictures looked
  at, the law commits' messages read. Merge with `git merge --no-ff`. On a conflict of `SNAPSHOT_VERSION`
  (`tape/src/snapshot.rs`), take a number above both sides.
- **Disk:** each worktree's `target` holds 2 to 3 GB. Delete it when the helper is done and its branch is merged or
  pushed. Helpers delete scratch runs once their numbers are in a table.
- **Ports:** helpers serve their page builds on port 8765; the lead uses its own (8791) and stops only that one.
- **Branches:** push a helper's branch to a `wip/` name as soon as it holds something. Prototypes, probes and their
  pictures live on branches, never on main.
- Push to `main` only as a fast-forward of a commit whose gate passed:
  `git fetch -q origin main && git merge-base --is-ancestor origin/main HEAD && git push origin HEAD:main`, then
  `git branch -f main origin/main`.
