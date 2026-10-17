# History of M0 to M3, in short

This file is the short history of version 2 up to the end of M3. The laws as they are now are
in `docs/laws.md`; the plan is in `docs/handover.md`. Git keeps everything: to read an old file,
run `git show <commit>:<path>`. Commit 154554a is the last state before the M3 clean-up; all the
old handover rows, reports and patches are in it.

## 1. What changed, day by day

One line per decision or change: date, what changed, and the merge (or the commit, when it went
straight to main).

**11 October 2026: M0 to M3 built**

- Workspace, chemistry 0, terrain, CLI pictures: `3b98e84`.
- History tape (snapshots, series, events, frames; run and resume): `1f44959`.
- Richness checks, chemistry generator, word-ban test, CI: `4c01fb9`.
- Web viewer (2D atlas map): `e16b9e6`.
- Law 2, Change: reaction solver with sub-steps: `29185c6`.
- Law 1, Flow: radiation, wind, rain, rivers, carving: `be38674`; latent heat, clouds, multi-rate: `12643b9`.
- M1: bodies, catalysts, Copy with mutation, simple Choose: `e8d6899`.
- M2: group brains, learning, memory, signals, watching, giving, movement wear: `58a9f06`.
- M3: Bond v1 (parts, bonds, structures, load, inside, floating): `c6ab9be`.

**12 October 2026: after the M3 check point review**

- Bodies are real-sized; population nodes with a member count; no density cap: `3812b5f`, `a96a108`.
- Nodes merge when alike; a cell keeps at most `crowd` nodes: `95f6f69`, `ab6f04a`.
- Watch reports and the M0–M3 review kept in `docs/reports/`: `869dafa`.

**13 October 2026: review 2 and review 3**

- Nodes keep a spread, split when too varied, act on bonds through one member: `2e2d56d`.
- A body holds a part only while its own force bears it: `0778c27`.
- Reaction rate first order in the scarce input: `22c291f`.
- Snapshot buffer, kinds by diet, the null run: `d5479cf`.
- Catalysts cost by capacity; catalysts unfold when hot; charcoal no longer burns cold: `c57f7d8`.
- Node math keeps gene means without selection; brain size can evolve in nodes: `94cefbd`.
- Greenhouse in four layers, island latitudes match its size, gases dissolve in liquid: `f78e891`.
- Light passes bodies from the tallest down (shade): `9a8bec6`.
- The roster of ten starting kinds (`--roster`): `b18f50e`.
- Gifts bounded by the receiver; a brain can shrink to nothing: `9089704`.

**14 October 2026: reviews 4 to 7, watch reports 3 to 8**

- Intake goes with the body, not what it holds; children join their own kind's node: `5944414`.
- Bodies die of age by their rate of living (`REPAIR` gene): `da375d7`.
- Catalysts cost a tenth of an eater's upkeep (`c_cat`): `444dabf`.
- A child costs what it is built of, not its liquid: `5dda2ff`.
- Per-kind tape columns and the land's season: `023717c`; more columns: `8f654c8`, `4ba6b7f`.
- Breathing through the open surface; intake with mass^0.75: `34a7bfe`.
- A node can split along its brain-size genes: `475ad38`.
- One reference mass; build rate; basal upkeep; leaf with mass^0.75: `851ac26`.
- Every number gets a row in `docs/numbers.md`, held by a test: `0546a17`.
- Individuals mode (`--individuals`) and `copy::open`: `33ef14a`.
- Likeness counts brain sizes by whole units: `d902f24`.
- A fair big-brain test (`--clever-year`): `551d0d4`.
- Nodes mix only with their own kind (`WITHIN` 2 → 0.5): `863ceab`.
- Hand-set eater brains at the start (neutral founders were a lottery): review 6, `03a5d01`; numbers marked honestly: `07bc5a3`.
- Body matter: growth is building, storing fuel is cheap; digestion; stores bounded by the body: `afdf3fc`, `c19a846`, `02ee75c`, `a15251a`.
- Bodies copy by physics only: `1d5358c`.
- Digested matter fills the room of what it becomes; a fuel's minimum is a reserve: `ccfbde9`.
- Activity needs liquid (dormancy; `WET` gene): `cab28ea`.
- Reproduction by surplus (maturity, `ALLOC`, brood, `CHILD`); size and maturity classes in nodes: `53586fe`.
- Body size by an energy ceiling (tissue costs by the kg of the body): `d2374d0`.

**15 October 2026: the queued mechanics, carbon, soil**

- Supply and wear go by the built body too: `f02d4c2`.
- Partial damage and healing (hits break body matter, force reads the body): `d143782`.
- Upkeep, intake and building follow temperature; work heat stays in the body: `8074ad6`.
- Two-parent copying: `47bae74`.
- Dead plant matter rots at a litter constant: `77c3ca7`.
- The sea's carbon store (salt 23, reactions 22 and 23), at balance at the start: `6048a5f`.
- Soil water (pores, Darcy's law): `32d2817`.
- Soil nitrogen with fixing founders: `8bdfbcc`, counted as a rule chosen for a run: `03123d8`.
- The rock nutrient; erosion at real rates: `ba9ec80` (commits `f17e86a`, `b5e6847`).
- Law 2 by mass action with the contact of the phases (no jump): `b147ca2`.
- Real figures in `numbers.md` checked against sources: `81ad832`.
- Nitrogen fixing pays its real cost; its heat pays no upkeep: `d9af1d7`.
- `c_build` anchored to real production (P/B): `bdab246`.
- Moving needs force: `a6d7926`.
- Drainage into a store in the bed: `de1c134`.
- Kleiber across kinds, adult founders, the living body at 75% water (R4): `1c7e5d1`.
- Law 2's step follows the rate law; gifts never give body matter: `5282e76`.
- Two simplify passes on core: `f2d6791`, `5413336`.

**16 October 2026: body physics**

- A body keeps a temperature (`KEEP_T`): `0fb9f3e`.
- End-product inhibition of catalysts: `12a8bf1`.
- Drainage to channels inside a cell; wet ground evaporates at the open rate; the sea floor reacts with the dissolved gas: `5de4eed`.
- Liquid past the level drains out (R11); rain caught by leaf area: `cc86483`.
- Bodies built of several materials; height by self-buckling; breaking by strength: `9967317`; its energy leak fixed: `6aa6ccd`.
- One heat network; cooling by evaporating (`COOL_T`); roots that refill; air films by wind and size; real leaf pores: `4a3c4fc`.
- The heat a body draws from the ground is set aside as it flows: `30737df` (commit `f03056d`).
- One freezing law for bodies, ground, lakes and the sea: `154554a`.

## 2. M3 clean-up (16–17 October 2026)

The code was made clean before the foundation work, and the bugs a review found were fixed. Each
line says what was done, and for a bug what it did wrong. Merges are named where there are some.

**The clean-up (results unchanged unless the line says so)**

- One folder per law: `flow/` (`cf9b151`), `change/` (`37c9518`), `bond/` (`30f4012`), `chem/`
  (`cf8138c`), `body/` with `life.rs`, `survey.rs`, `nodes.rs` (`8464bd8`), `copy/` (`026c8ea`),
  `choose/` with `lanes.rs` and `body/intake.rs` (`8c89a6c`); `lib.rs` split (`4fc2d95`); the
  tape, cli and worldgen in folders too (merge `4c65ce0`).
- Backwards compatibility removed: old snapshots, tapes and flags are not read; a snapshot has a
  format header (`797b1a1`).
- The experiment switches `split_rows` and `sate_all` removed (`fab9e96`); dead code removed
  (`a8b64d5`).
- Diagnostic and derivable tape columns, and the counters only they read, deleted (`84853d3`,
  `3a8dcb9`); the counters in one place, `World::counts` (`32884a5`).
- Near-duplicates made one helper each (ground heat, gas settling, Arrhenius rate and others:
  `3100d49`, `83e7e64`); every random draw has its own purpose (`80dec58`).
- Tests reorganised: one file per law with a shared test kit, `worldgen::testkit` (`4639b6b`,
  `12afe06`); overlapping tests removed (`ef7615a`); slow tests `#[ignore]` and run in CI
  (`3e54e9f`); the golden test pins three small runs by their fingerprint (`968ae5e`, `902183d`).
- The docs rewritten: the handover is the plan, `laws.md`, `history.md` and `tape.md` added, old
  reports removed (merge `3269b05`).

**Bugs fixed (each changes results)**

- A child of less than one member joining a node counted as a whole member: animals were about 3
  times too many (`d817aa9`).
- A member going back to its node made the node's row spread smaller each time (`57e2965`).
- In `--individuals`, siblings shared one set of born rows (`06d09d5`).
- Brain groups weighed bodies, not members, and a new group counted 0 until the next month
  (`b3d2c0d`).
- One error too big for a number stopped a group's learning for good (`3f93399`).
- A null run still drew new brain weights (`e900262`); the copy action still had a row, a score
  and a dead draw in the brain (`43efbc5`).
- The children who stayed got their provisions from stores already cut by the children who left;
  a birth was recorded in the parent's cell (`fbf5d54`).
- Gas entering the air from bodies, parts, reactions and growth did not pay its lift, but its
  fall was paid: energy from nothing, about 3.5e16 J a year in a 48 × 48 run (`ac0c48b`, merge
  `8cb98fb`).
- Two ground temperatures (Flow's and one rebuilt from the amounts); taking from the ground read a
  survey weeks old (`064924d`, `628bd6c`).
- Liquid waited for the wind's turn when their periods differed (`9ba34e9`).
- The bed's stores flowed several cells a step toward +x and +y, one toward −x and −y
  (`1be7b83`).
- The yearly sideways heat conduction between ground nodes did almost nothing (about 1e-5 of a
  difference a year): removed (`5f7b59a`).
- Smaller Flow faults: the wind made matter from nothing when it clamped negative amounts, moves
  of molten amounts left the held fusion heat stale, pore liquid under field capacity flowed
  sideways (`f3e10b0`); the molten bit set was exact only to 53 substances (`306f41f`); drainage
  could take more than a node held (`7875ee9`).
- Land had two definitions in worldgen (`a92e71e`); an empty row in the kinds table
  (`92d3748`); the tape's yearly state was lost at a resume (`c71d668`); the tape weighed groups
  by bodies (`c548616`) (merge `62b441f`).
- A ground node on a melting plateau reacted up to about 80 K too cold (`cd363b9`).
- Formation enthalpies were the measured ones where the books count the solid's: burning the
  energy-rich solid gave about 451 kJ, not 467.5 (`71eca3f`).
- Parts settled their gas with the air without its heat or lift (`f34949c`); parts used the
  plain melting point, and gases counted as dissolved (`87f6296`).
- One substance's molar mass was not the sum of its elements: a reaction made or lost mass
  (`1f2c839`); molar masses now come from the element masses (`3c1098f`).
- The reactor froze reactions over a calendar year, not the world's year, and judged calm with
  numbers from two sub-steps (`f75aa55`) (merge `9509c6c`).
- A node that lost members lost more brood than matter, and a bite left the brood whole: deaths
  of want were about 6 times too many (`37e98f0`).
- Two rules for what a body burns (one at 290 K, one at its own temperature) (`63a3c60`).
- A dormancy of 1 gave NaN, and bad time steps were not refused in core (`cd9b219`).
- `pay::plan` would fail with more than 64 burning reactions (`0066c2b`).
- A body's molten stores held no fusion heat: it warmed by the fusion heat of every mol it drank,
  and frost killed about 10 times too many (`8ff6e90`).
- The share of light a light reaction stores was a constant in the reactor, not a number of
  `Physics` (`c9294c9`).
- The starting air thinned with height by a fixed scale, not its own (`9c1c252`).
- Tool bugs: a resume did not place the kinds still due (T1, `0bf0414`); `root_<k>_life` divided
  by the whole mass (T2, `25eca36`); the viewer drew a missing value as 0 (T3, `5bda05f`); the tests'
  default start was not `ew run`'s (T4, `9417861`); bad time steps gave no clear error (T5,
  `3bee6fc`) (merge `4c65ce0`).

## 3. Tried and not kept

Each item names where its code or its record can be found.

- **Built mass** (a body's whole dry mass as its size; every animal died):
  `git show 154554a:docs/reports/watch-7-before-m4/built-mass-not-merged.patch`.
- **First nitrogen build** (one gift of the mineral, no input; it collapsed):
  `git show 154554a:docs/reports/nitrogen-not-merged.patch` (commit `d3819ac`).
- **Review 7 prototypes** (the reviewers' own code; the fixes were rewritten and landed):
  `git show 154554a:docs/reports/review-7-plants.patch`, `…:docs/reports/review-7-lives.patch`.
- **Review 8 carbon prototypes** (a litter patch and a rough sea store):
  `git show 154554a:docs/reports/review-8-carbon-litter.patch`, `…:docs/reports/review-8-carbon-sea-prototype.patch`.
- **Law 2 smooth rate, the study's code** (the law itself was then rebuilt and adopted in `b147ca2`):
  `git show 154554a:docs/reports/law2-smooth-rate-not-merged.patch`.
- **Eater density probe** (a diagnostic, no taking from the ground):
  `git show 154554a:docs/reports/eater-density-probe.patch`.
- **Cold-blooded resting rate** (tissue costs × 0.068; strength became free and plants walked):
  commit `c8a56ad`, reverted in `f3d3650`.
- **Convex catalyst stand-in** (cost by the square of the genes): replaced by cost by capacity in `c57f7d8`.
- **First-order rate law** (per unit of the scarce input; it jumped when the scarce input changed):
  `22c291f`, replaced by `b147ca2`.
- **Catalyst costs by mass^0.75** (review 6): `a3d31a0`, replaced by `d2374d0`.
- **Satiety at every size** (killed every animal, twice): `e9705b1`; the switch
  `Config::sate_all` was removed in `fab9e96`. The gate "satiety only past `full`" went in `a15251a`.
- **The `full` factor and copying when full** (a body copied at 2 × its copy mass): replaced by
  reproduction by surplus, `2b6c8a9`.
- **The child store** (`Body::paid`, work paid ahead for children): `9089704`, removed in `c19a846`.
- **Copying by the brain's choice**: off since `1d5358c`; the copy action left the brain in `43efbc5`.
- **Splitting nodes along their brain rows**: `Config::split_rows`, off, removed in `fab9e96`.
- **R11, the first two tries** (draining the liquid past the level killed the eaters, then cost
  the big kinds): `12aaf60` reverted in `915b051`; `e13d911` reverted in `e9176f7`; landed as `0639392`.
- **Damage by the weight of the bite in nodes**: commit `5679524`; the old node rule was kept.
- **Frame enzyme in the founders** (the eaters of remains became 10,000 times fewer): removed in `7953f69`.
- **Freezing prototype 2** (only bodies): `a013312`; the general law landed in `154554a`.
- Measured and dropped, recorded in the old handover (`git show 154554a:docs/handover.md`,
  Section 4): the textbook coat (still air always outside the coat), one temperature level instead
  of `KEEP_T` and `COOL_T`, the linear form of product inhibition, lowering the salts' own melting
  points, the film with only the dissolved stock, and `c_build` at 3 × 10⁴.

## 4. Reports that were deleted

All are in commit 154554a. Read one with `git show 154554a:<path>`. Pictures are in the folder
of the same name.

| Path | What it was |
|---|---|
| `docs/session-summary-2026-10-15.md` | summary of the 14–16 October sessions |
| `docs/reports/watch-pre-m4.md` (+ folder) | the fix list before M4, three 300-year runs |
| `docs/reports/review-2-before-m4.md` | second review of Part 1 |
| `docs/reports/watch-2-before-m4.md` (+ folder) | second fix list, 300-year runs |
| `docs/reports/review-3-before-m4.md` | third review (math, physics, chemistry) |
| `docs/reports/watch-3-before-m4.md` (+ folder) | third fix list, 300-year runs |
| `docs/reports/watch-4-before-m4.md` (+ folder) | ageing, enzyme cost, 300-year runs |
| `docs/reports/review-4-before-m4.md` | fourth review |
| `docs/reports/watch-5-before-m4.md` (+ folder) | review 4 done, 100-year checks |
| `docs/reports/review-5-before-m4.md` | fifth review (lives, the roster) |
| `docs/reports/watch-6-before-m4.md` (+ folder) | review 5 done |
| `docs/reports/review-6-adversarial.md` | adversarial review |
| `docs/reports/review-6-life-size.md` (+ folder) | lives and sizes |
| `docs/reports/watch-7-before-m4.md` (+ folder) | review 6 fixes, built mass tried |
| `docs/reports/watch-8-before-m4.md` (+ folder) | body matter and food storage |
| `docs/reports/review-7-plants.md`, `review-7-lives.md` | review 7 (with patches) |
| `docs/reports/review-8-carbon.md` | review 8, carbon (with patches) |
| `docs/reports/sea-carbon.md` | the sea's carbon store |
| `docs/reports/soil-water-nitrogen.md` | soil water and nitrogen |
| `docs/reports/rock-nutrient.md` | the rock nutrient, erosion anchored |
| `docs/reports/law2-smooth-rate.md` | Law 2 without a jump |
| `docs/reports/big-animals.md` | why the roster's big animals died |
| `docs/reports/eater-density.md` | how crowded the eaters are |
| `docs/reports/frost.md` | frost deaths, two prototypes |

The watch reports of M0, M1, M2 and the M3 check point were shared as separate files and were
never in the repository.

## 5. How M0 to M3 were worked

M0 to M3 used many review rounds (reviews 2 to 8, each followed by a watch report), done by
the agent and the owner together. Each round found real faults, and the fixes are in the list
above. The old handover kept every result of every round in Section 4; at the end of M3 it was
397 KB. The clean-up moved the laws to `docs/laws.md`, this history here, and left the handover
as the plan.

## 6. The foundation (from 17 October 2026)

The work of `docs/foundation/plan.md`, one line per task.

**Phase A: prepare**

- A1: where the time goes, measured with `perf` on 64 x 64 worlds with life (`docs/reports/foundation-a.md`): bodies
  about 70% of a year; inside them the group brain sums 15%, the reactions 3%.
- A2: the reference runs of `5671305` (64 x 64 x 60 years, seeds 1 to 3, normal and roster; 128 x 128 x 300 years,
  seeds 1, 5, 7) in `docs/reports/foundation-a/`.
- A6: the code-size test `cli/tests/core_size.rs` counts the code lines of `core/src` per folder and fails above
  9,500 (7,508 at its start); it replaces the `wc -l` command of handover 14.3.
- A3: the key-number test `tape/tests/ranges.rs` runs seeds 1, 2 and 3 of `ew run --size 64` for 30 years and holds
  ten numbers of climate, land and life near the A2 references; the series gains `t_land`, `rain_land` and `rivers`
  (the river count moves from the CLI into `tape::dry_land_flux`), and the rock of a cell moves into the test kit.
- A4 (a): plant members die "taken" in years 3 to 21, before any animal: light-eater nodes take from each other
  (their neutral brains draw the take action), and a bite that is more than one member kills members by the largest
  share it takes of any store (the node rule the owner kept). Behaviour of the laws, not a bug; nothing changed.
- A4 (b): the north–south stripes of the rain map came from the wind's lines solved last (north–south); the wind now
  solves both orders from the same start and keeps the mean. 5 years without life, 64 x 64, seeds 1 to 3: land rain
  +0.7 to +2.5 %, temperatures within 0.03 K; golden re-pinned.
- A2, 128 x 128: seed 1 stopped in year 285 on a NaN in the wind. The bed's store under a hill top drains by a share
  each liquid step, so its last liquid fell below 1e-308 m³; `liquid::send` shared what left by 1 / (sum of the
  flows), which was then infinite, and 0 x infinity put NaN into the ground, then the air. Each outlet now takes
  q / (sum of the flows); golden re-pinned (rounding only).
  The drainage between stores (`drain::stores`) had the same risk (0 / 0 when such a flow rounds to 0): there
  nothing comes in, so nothing spills.

**Phase B and C: the base layers and the shared maths**

- C3: the wind's tridiagonal solve moves from `flow/air/wind.rs` to `solve::lines::along_line`; results unchanged
  (golden kept).
- C1: `solve::spread`, one backward Euler step along links (conductance per link, capacity per node) by conjugate
  gradient with a Jacobi preconditioner, a bounded number of iterations in a fixed order; it returns the link fluxes
  and moves each node only by them, so the books are exact even unconverged. No law calls it yet (golden unchanged).
