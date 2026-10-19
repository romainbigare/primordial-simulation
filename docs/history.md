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
- **Nodes keeping their members' wear** (Phase F, the mean and the variance of their wear, those past 1 dying, the
  maturing carrying the wear of their sizes): nodes' eaters lived twice the individuals' and the eaters' life left its
  key range (7.1 yr against at most 5.8); one rule of death for both replaced it: branch `nodes-wear-tried`.
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

- B6: the `Ledger` goes. Sun, space and outside are three boundary nodes of the store (`Class::Boundary`, saved in
  `World::boundary` and `World::e_boundary`); radiation and edits book what crosses on them (`World::cross`, or a
  `Move` from or to one, which `apply` never scales), and `check::drift` reads the world's totals with theirs. The
  history is unchanged (`series.csv`, `months.csv` byte-identical) and the drift printed is the same (in full, the
  energy drift bit for bit, the element drift to 6 digits: the outside node sums substances, not elements); snapshot
  version 6, golden re-pinned (the saved state changed).
- B9: `Rng::of_node(seed, seconds, NodeId, purpose)`, a second key beside `Rng::new(seed, day, index, purpose)`. It packs
  slot and generation into one number and tags the key, so it never gives the stream of `new` for the same inputs. No
  draw uses it yet; every old draw keeps its key (golden unchanged).
- B8: `clock::Sleep` (`World::sleep`, saved): per class a queue of node alarms by time, then slot, and per node the
  time it was last brought up to date; `store::apply` wakes a node when a change brings it more than `wake_share`
  (a new speed setting, in the new table "Speed settings" of `docs/numbers.md`) of what it holds. Boundary nodes run no
  process and never sleep. No law sleeps a node yet: `series.csv` and `months.csv` unchanged; snapshot version 7 and
  golden re-pinned for the new fields.
- C4: tests that one big step is close to many small steps for `solve::lines` and, for `solve::downhill`, that one
  rain in one pass equals the same rain in parts with a pass each (linear rule); tests only.
- C3: the wind's tridiagonal solve moves from `flow/air/wind.rs` to `solve::lines::along_line`; results unchanged
  (golden kept).
- C1: `solve::spread`, one backward Euler step along links (conductance per link, capacity per node) by conjugate
  gradient with a Jacobi preconditioner, a bounded number of iterations in a fixed order; it returns the link fluxes
  and moves each node only by them, so the books are exact even unconverged. No law calls it yet (golden unchanged).
- B3 and B5: the store `core/src/store/`. A node table gives each ground, air and deep node an id (`NodeId`: slot and
  generation), a class, a parent and a cell; it follows from the grid, so a world builds it when made and when loaded,
  with the same ids, and it is not saved. `store::apply` books moves, transforms and link fluxes in a fixed order and
  scales down what takes from a node that holds too little (`Counters::scaled`). No law calls it yet; results unchanged.
- B1, B2: `core/src/space/`: a `Mesh` of levels (cells with area, centre and latitude; links each once with their two
  cells, distance and width; compressed adjacency; parents and children), the square builder (`space::square`, which
  now holds `DIRS`, `FORWARD` and the square layout) and stacks of layers with vertical links. `Grid` is a thin view
  on the mesh: cell count, area, air nodes (`air_of` is the parent at the air level) and latitude come from it; the
  mesh is built from the grid's numbers and `Config`'s latitudes, not saved. Results unchanged (golden fingerprints).
- C2: `solve::downhill`: the cells sorted by head (a topological order of the downhill links) and one pass down it
  that moves rows of amounts by the shares a rule gives each link, with exact per-link fluxes and no cell below zero.
  On bare test terrain with rain it ends one liquid step as `flow::liquid`'s sweep does to rounding
  (`core/tests/solve.rs`); not yet used by a law (D2).
- B4 measured, not built: the amount arrays `ground`, `bed`, `deep` and `air` (208 B per node each) are about 5% of a
  world with life (128 x 128, year 4: 10 of 186 MB; 64 x 64, year 30: 2.6 of 54 MB, where born brain rows are 63%
  and bodies 23%). Loose ground fills as life spreads (zeros 30% at year 4, 9% at year 30); bed (82% zeros) and deep
  (65%) would gain, about 40 MB of 1.5 to 1.7 GB at 512 x 256 (3%). Flow already loops only over the substances
  present (`Scratch::loose`, `bedk`, `gas`). Kept dense until the amounts, not brains and bodies, bound the memory.
- B7: the clock (`core/src/clock/`): time is whole seconds (`World::time`, was `World::day`); the day, the month, the
  year and the season are derived (`clock::Calendar`, `Config::calendar`); `schedule.rs` runs its month as processes
  with periods (`clock::Every`, `due`, `next`) in one fixed order. Same history (series and months byte for byte);
  golden re-pinned and `SNAPSHOT_VERSION` 5, since the saved time is now in seconds. Flow's own periods stay in
  `flow::step` until Phase D.

**Phase D: the physics laws on the new base**

- D4: radiation reads the latitude and the area of each cell from the mesh (`mesh().cells()`), not by rows of the
  map; it works on any mesh. Same fingerprints.
- D8: the cleared ground of a lane is the first link field: `Links::cleared` (`store/links.rs`), one value per link of
  the cell level (was `World::pass`, four forward edges per cell, empty until first used); `lanes.rs` and the tape read
  it by link (`Level::link` finds the link of two neighbours). Same history (series and months byte for byte, with
  `--roster` to year 22, where 230 links are worn); golden re-pinned and `SNAPSHOT_VERSION` 8, since the saved field
  changes its layout. The `worn` layer of the viewer now shows the most worn of all links of a cell, not only of its
  east and south ones.
- D0: `store::apply` keeps its working space between calls (`store::Booking`, `World::booking`, not saved) and works on
  the numbers its changes touch alone, not on zeroed arrays of the whole world; it books each link flux one column
  (substance or heat) at a time, so a flux names each substance once. Same results bit for bit. Heat on the 64,770
  cell links of 128 x 128: from 6.1 ms (43 times a plain loop) to 0.35 to 0.4 ms (2.6 to 3 times).
- D1: heat conducts sideways through the ground once a year (`flow::land::conduct`: `solve::spread` on the cell links
  with a shared border, booked as heat-only link fluxes by `store::apply`), with the bed's mean conductivity kept in
  `Scratch::bed`. It was removed from the yearly creep before (`5f7b59a`); creep keeps only the loose solids that slide
  with their heat. Snapshot version 9, golden re-pinned.
- D7: parts are nodes of the store (`Class::Part`): the node table gives each its id and cell, reuses a freed slot
  with a new generation (`life::free_place`) and is now saved (snapshot version 10); `Part::gen`, `Part::cell` and
  `Parts::free` go, and bodies name the part they hold and the one they are inside by `NodeId`. Bonds are one link
  list (`bond::Bonds`, each bond once, not twice in each part); the inside of a structure is joined to its cell by
  one link (`bond::Enclosure`: how closed, heat conductance) that the body's heat, light, rain and evaporation read
  (`bond::enclosure`, was `walls_around`). Same physics; only the drift of a floating structure draws from a new key
  (`Rng::of_node`); golden re-pinned.
- D3 (a), the law: the wind turns by the latitude of each edge between two air nodes, the mean of the two nodes'
  latitudes (it took the latitude of one row of cells near the edge). 64 x 64, seed 1, years 11 to 20: land
  -0.03 K, rain on land +0.1%, carbon gas +0.6%; golden re-pinned.
- D3 (b): the air on the mesh. The exchange of ground and air reads each cell's air node from the mesh's parent
  relation and the areas of cells and air nodes from the mesh (`Scratch::cells` and `per` deleted); a cell's share of
  its air node is its area over the node's (`Grid::air_share(i)`; it was 1/16 also where an edge node covers fewer
  cells). The wind runs on the links of the air level: along lines where they form lines (`wind/along`), by
  `solve::spread` on any other mesh (`wind/mesh`, with a test on a mesh of triangles); booked per node, not by
  `store::apply` (its scaling cuts gas passing through a node). Without life the same to rounding (2 years of the
  golden seeds, 64 x 64 and 128 x 128 within 5e-11); maps whose side is not a multiple of 4 change at their edges;
  golden re-pinned (with life, any rounding changes the bodies' history).
- A link flux takes from a node only its net loss over all the flux's links (a net change within 1e-12 of what passes
  through is rounding) and is scaled as a whole by `store::apply`: liquid that passes through a dry cell was scaled
  to nothing. A link may join nodes of two classes (`LinkFlux::class`, the vertical links of a stack: ground and
  deep store), so a law books a vertical exchange as one flux, not as one move per substance and cell. Same results
  (golden unchanged).
- D2: liquid on the new base. Surface liquid moves down the cells in one `solve::downhill` pass (the order renewed
  every liquid step) with the old sweep's rule as its closure; the load rides in the pass's rows and settles where
  the share of the liquid that leaves is small. Pore liquid and the deep store are the two layers of each cell's
  stack (`space::Stack`): one `solve::spread` moves both by Darcy's law across the borders of cells (no longer
  across corners); the vertical link carries the sinking (a rate, as before) and the springs and the spill of a
  full store. Every move goes through `store::apply`: link fluxes of the links that carry something, the vertical
  ones between the two classes. `downhill::pass` keeps a cell's rows side by side and gives the crossings of its
  links (`downhill::Crossed`), not a flux for every link and row. Deleted: the sorted sweep (`order_key`,
  `ORDER_TOP`, `ORDER_PER_M`, `CELL_BITS`, `CELL_MASK`, `Outlets`, `lower`, `runoff`, `send`, `carry`),
  `pore_flow`, the old `drain` and `stores`, `land::darcy`, `Scratch::{nbr, done, sed, sed_e}` (`Scratch::order` is a `downhill::Order`), and the
  downhill parity test of `core/tests/solve.rs`. Snapshot version 11, golden re-pinned.
- D5: the work of moving liquid on its bed (carrying, dropping, wear) runs at every liquid step in the downhill
  pass, with no multiplier; `WORK_MONTHS` and `Scratch::owe_work` are deleted. The young 32 x 32 island of
  `an_island_loses_ground_at_a_real_rate` loses 0.133 mm a year (main: 0.138); "land worn down" (64 x 64, years 21
  to 30) 0.048 mm a year. Snapshot version 12, golden re-pinned.
- `store::apply` books link fluxes alone first and checks after (`book_first`): each column is added as the careful
  path gives it, and only the numbers that ended below zero are looked at; when one is short beyond rounding, every
  number is set back from the values kept before each change, and the careful path runs. Nodes are woken at the end
  of the call. Same results (golden unchanged); Flow at 128 x 128 without life about 15% faster.
- D3 (c), the law: the wind books what crosses the air level's links over its whole solve through `store::apply`
  (`wind::book`'s own booking deleted), so a node is no longer clipped when a line asks it to pass on gas it has not
  yet received; each call starts from the air nodes' sums of their amounts now (`air::sums`), with Law 2's gas.
  64 x 64, seed 1, 20 years with life, years 11 to 20: land -0.24 K, air +0.13 K, rain on land -1.1%, carbon gas
  +0.4%, river cells -1.7%; ranges hold; golden re-pinned.
- D6: Change's ground and air nodes sleep by `clock::Sleep` (`change/screen.rs`, the turns by blocks, `LAZY`, `SLOW`,
  `FAR`, `OWN`, `SCAN`, `SAMPLE`, `FEW`, `BLOCK`, `Scratch::slow`, `Reactor::calm` and `Reactor::fastest` deleted): a
  node reacts when due for all the time since, then sleeps until its own rates would use 5% of their inputs
  (`Reactor::rest`; a reaction on a trace of its phase does not count), at most `sleep_days` (90, a new speed
  setting); `store::apply` wakes a node on amounts only, not on heat. The air nodes react one after the other.
  Sleep against no sleep, x2 and /2 (`tape/tests/ranges.rs`): the chemistry within its noise. 64 x 64, seeds 1 to 3,
  years 21 to 30: carbon gas 248.4 ppm (was 252.2), land 289.45 K (289.47), eaters 4.26 log10 kg (4.49), drift
  near 1e-14. Snapshot version 13, golden re-pinned.
- D9 (Change): Change books through `store::apply`: each node's reactions are one `Transform` (from the extent of
  each reaction, `Reactor::made`, summed exactly where its terms cancel: a fast pair that undoes itself beside a slow
  reaction left a transform unbalanced by 1e-129 against 1e-33 mol), the gas between a ground node and its air node
  `Move`s (`change::step`'s writes to the arrays, its own guard on paying for gas and its use of `send_gas` deleted).
  The ground nodes due in a step react from the same air, each from its share, not one after the other: without
  life (16 x 16, seed 1, 2 years) carbon gas -0.003%, methane -1.4% (as much as a sleep with no wake changes it);
  64 x 64, seed 1, years 11 to 20: land -0.05 K, rain on land +1.0%, carbon gas 272.6 to 268.8 ppm, plant members
  +12%. Change at 128 x 128 without life 1,047 to 1,265 ms a year (the cost of `apply` for its moves and transforms).
  Golden re-pinned.
- An air node reacts with no liquid's film (`Reactor::film` 0): it reacted through the film of the last ground node
  that reacted before it, where that one stood under liquid. 64 x 64, seed 1, years 11 to 20: land +0.04 K, rain on
  land -1.3%, carbon gas 268.8 to 271.7 ppm, methane -3%; golden re-pinned.
- D9 (first half), the law: Flow and Bond move through the store. Radiation books the sunlight from the sun node,
  what each ground node gives its air node and what leaves each air node to space as link fluxes (one call); the
  exchange books each pass (evaporation, condensation, dissolving) as one flux from the ground nodes to their air
  nodes, the air receiving the heat less the latent heat it holds (`LinkFlux` may join ground and air:
  `LinkFlux::lifted`); creep books its slides as one flux. The potential energy is a node of the store
  (`Class::Potential`, its heat `World::potential`): every lift and fall of Flow is a link flux with it
  (`flow::Falls`). Deleted: `World::take_heat`, `take_heat_with` (and their test), the floor clamp of radiation, the
  `take_paid` of the exchange (`store::apply` is Flow's one guard, at zero), `exchange::ground_to_air`,
  `Scratch::{put, mob, rain, cloud, snap, node_tmp}` and `scratch::space`. The bed is no node yet: its wear and
  digging change it by one helper (`World::loosen`) until patches (F6). Bond's trades with parts stay its own (parts'
  amounts are not store columns). `apply` books fluxes with boundary nodes and the potential on its fast path, and
  looks for numbers below zero in the same pass. Test `cli/tests/contract.rs`: `flow/` writes no array of `World`.
  64 x 64, seed 1, 20 years with life, years 11 to 20: land -0.01 K, air +0.02 K, rain on land +0.5%, carbon gas
  -1.2%, river cells 500.6 -> 499.9; snapshot version 14, golden re-pinned.
- A link flux wakes a node by its net change over the `store::apply` call (`Giver::flush`: each amount as it was
  before its first change, marked by a bit), not by each change: liquid or gas that passes through a node wakes
  nothing, and many small gains that sum to more than `wake_share` wake it. A move or a transform wakes a node by its
  own change. Golden re-pinned.
- `flow::drain`: its two helpers for what moving liquid carries (`carried_by` for the vertical links) become one
  (`carry`, by signed shares); `carried` keeps only the choice of links. Same results (golden unchanged).
- `store::Batch`: a link flux gathered link by link (`add`, `link`, `carry`; `falls` and `fall` for the heat of falls
  and lifts with the potential's node). The exchange's passes, creep and every fall of Flow use it (`flow::Falls`
  and the exchange's own link lists deleted). Same results (golden unchanged).
- Change books the gas between each ground node and its air node as link fluxes, one per substance (`store::Batch`,
  was a `Move` for each node and gas), and the lift as a flux with the potential (its writes to `e_air` and
  `World::potential` deleted; `cli/tests/contract.rs` covers `change/`). `check::broken` lets a heat reach 0 J, where
  `apply` stops what takes from it. Without life the same (16 x 16, seeds 1 to 3, 2 years: series equal); with life
  the gas wakes an air node by its net change. Golden re-pinned.
- D9 (second half): `grid.rs` deleted. `World` holds the mesh (`World::mesh`, not saved) and the map it is built
  from (`World::map`, `World::air_scale`, saved as the grid's numbers were); the laws read cells, links, areas and air
  nodes from the mesh (`Mesh::share` for a cell's share of its air node), creep walks the cell level's links, and
  `flow::Scratch` keeps no copy of the cell area or the air node of each cell. The square map stays for what is about
  its layout (`docs/laws.md`). `cli/tests/contract.rs` covers all of `core/src`, with each exempt file named. Same
  results bit for bit (golden unchanged).
- D10 (speed), `store::apply`: before a call books its link fluxes it keeps the numbers each column may change, one
  copy of each span of nodes (`Giver::keep`); the fast path sets them back from it when one is short, and the wake
  compares against it (the value kept before each change and the bit per amount go). The sum of all that passes
  through a number below zero runs only over the columns that may change one. Same results (golden unchanged);
  Flow at 64 x 64 without life 15% fewer instructions.
- D10 (speed), `clock::Sleep`: a node sleeps and wakes by its own alarm alone (the queue of alarms by time, a
  `BTreeSet` per class, goes); the nodes due are found by a look over all the alarms, in the same order. Same
  history (`series.csv` and `months.csv` byte for byte); snapshot version 15, golden re-pinned (the saved state
  changed). Change at 128 x 128 without life about 9% faster.
- D10 (speed): `Sleep::due` gives the nodes due by slot (Change's sort of them goes); Change allocates each node's
  transform once. Same results (golden unchanged).
- D10 (speed): `Chemistry::add_elems` adds only the elements a substance holds, and `store::apply` counts the atoms of
  each substance once a call to check its transforms; `flow::liquid::book` copies the amounts before a call one
  substance at a time. Same results (golden unchanged).
- D10 (speed), `change::Reactor`: a reaction whose inputs are not all in the node is passed over at once (a bit per
  substance held, `Reactor::needs`), and the reactor's later loops run over the reactions that can run (`on`), went
  (`ran`) or set the rest (`on0`), not over all of them. Same results (golden unchanged); Change at 64 x 64 without
  life 13% fewer instructions.
- The wind skips a direction with no air link: on a map one air node across (the slow copy tests' 4 x 4 world) it
  panicked since D3 (b), so CI was red from D3 (b) to D10 while the normal suite passed; fixed, results unchanged.
- Phase D check: all ten key numbers in range, the sleep test passes; the speed goal (Flow and Change 2 times
  faster) and the size goal (600 lines fewer) are not met: `docs/reports/foundation-d.md`.

**Before Phase E: an outside review of Phase D**

- The plan follows the review: a law never overdraws a node and the store only adds (D9, D11); wake by what a node
  gained since it slept, not in `apply` (D12); check level 2 is the normal level for speed work (D10); Phase E in the
  order of the profile, the bookkeeping first, E4 (brains as one block) dropped, E8 (a body as a node in Flow's steps)
  replaced by moves and one body node per cell (D11); the thread design (5.6) before the columns; the chunk layout
  of amounts (Idea 8); no sparse amounts (B4) and no graphics card (old J2); J2 is now the longer physics step; the
  8,000-line ceiling moves to the end of the foundation (9,000 until then).
- D11 (a): the laws that could ask a node for more than it holds stop doing so in their own code. A ground node's
  reactions give the air no more gas than its heat pays for (the rest stays in it; `store::apply` scaled the whole
  flux). The pores, the deep stores and the wind on a mesh cut a node's own outgoing links where they would take more
  than it holds (`solve::spread::within`). A probe found that no Flow call ever needed the store's scaling (golden
  runs; 64 x 64 seed 1, 12 years with life); only Change's transforms did, for rounding of trace substances. Same
  results (golden unchanged; 64 x 64, 12 years: same fingerprint).
- D11 (b) and D12: `store::apply` only adds: it books each link flux, move and transform as given, sets what ends
  below zero to zero and counts it (`Counters::rounded`; a debug build asserts that it is rounding). Its scaling of
  whole fluxes, the fast path that booked first and restored, the copies of what a flux may change and `Booking`,
  `Counters::scaled` and `World::booking` go: `store/` 686 -> 349 code lines. The wake leaves `apply`: a sleeping
  node wakes when one of its amounts has grown by more than `wake_share` of what it held when it fell asleep
  (`Sleep::keep`, `Sleep::watch`, looked at by Change at its step), whatever brought it, bodies too. Change books its
  gas and its reactions in one call. A node's reactions are no longer scaled away for the rounding of a trace
  substance (a whole air node's transform was cut to 8% of it or less for 1e-30 mol). 64 x 64, seed 1, years 7 to 12 with
  life: land 288.58 -> 288.57 K, rain on land +0.1%, carbon gas -0.1%, plant mass +1.2%; air nodes due per year
  119 -> 100 with life, 114 -> 8.5 without; ground nodes as before. Snapshot version 16, golden re-pinned.
- D13 (a): the work spaces are kept between steps: one `solve::Spread` in Flow's scratch for the heat through the
  ground, the liquid under the surface and the wind on a mesh (each call made a new one); Change's batches of gas and
  reactions (`change::Batches`, `World::batches`, not saved); a `store::Batch` reuses its flat column. Same results
  (golden unchanged; 64 x 64, 12 years with life and 6 without: same fingerprints).
- D13 (b): the table of each gas's saturation pressure is built once and kept (`Scratch::sat`, not saved), from 1 K
  up to the warmest ground or air it is asked for (built again further up when needed); it was built for every gas
  at every exchange, from the coldest temperature. Its entries now lie on a fixed grid, so a value between them moves
  by up to about 1e-6. 64 x 64, seed 1, 6 years without life: rain on land within 5e-5, every series column within
  4e-4. Golden re-pinned.
- D12 (test): the sleep test (`tape/tests/ranges.rs`) also runs `wake_share` x2 and /2 against every node awake; the
  chemistry's key numbers move by 6 to more than 100 times less than their noise at every setting. The ten key
  numbers stay in their ranges (three seeds, 30 years, 64 x 64).
- D11 (speed): `store::apply` books a link flux column by column into its one or two arrays, and its element check
  reuses one room for the sums. Same results (golden unchanged). 32 x 32, seed 1, 4 years, instructions against the
  start of D11: Flow -1.8% and Change -4.6% without life, Flow -1.6% and Change -2.1% with life.

**Phase E: the life engine**

- The plan follows an architecture review and the owner's answers: a time budget sets the planet's size (D12); a
  population node updates at most about 10 times a year (D13); zones, automatic zoom, patches, more link memories,
  threads over bodies, the chunk layout, a query layer and snapshots of changes wait until after the foundation
  (D14); the globe is a lat-lon grid that wraps (D3); E2 and E3 only if a profile asks, E4 (bodies sleep by the
  clock) dropped, E5 with a private `Store`, new E6 (born rows), E7 (members split off), E8 (one clock, state apart
  from cache, fixed nodes by index); F7 first; the long step (J1) right after E; the cost model (5.7).

- E1 (a), speed: a change of a group's members changes its count at once and waits, by born rows, in the group
  (`Group::due`, holding its born rows); the sum of born rows and the rows follow in one pass when the rows are read
  (`Life::settle`: before a body thinks, before a reader of the rows, and at the end of each run of the clock),
  not at every change (`Group::weigh` re-summed every row each time). 64 x 64, seed 1, 10 years: the work on the
  sums falls from 4.1 s to 3.0 s (9% of a year to 7%: about one pass per decision, not one per change of members, but
  a pass now reads its rows from memory, not from the cache); a body update about 3% faster (runs side by side, year
  10: 42.8 to 41.3 µs and, on a busier machine, 55.6 to 53.2 µs). Years 11 to 20: land +0.02 K, carbon gas 270.4 to
  271.4 ppm, rain on land -0.4%, plant members -9% (life's choices grow the rounding); golden re-pinned.
- E1 (b), speed: a body's height is its seventh visible trait (`Body::vis`, set where the others are, and for a body
  placed or left by a node's draw), so the survey's shade sorts by it and no longer works out every body's height
  again every physics step. Same history (`series.csv` and `months.csv` byte for byte, 64 x 64, seed 1, 20 years);
  snapshot version 17, golden re-pinned (the saved bodies changed). A body update 3 to 5% faster (year 10: 42.1 to
  40.7 µs; year 20: 44.9 to 42.6 µs).
- E1 (c), speed: the temperature solve (`Body::thermal`) runs once for the stages of an update that see the body in
  the same state and is passed along: the upkeep, its plan and its paying (`upkeep_at`, `plan`, `pay_at` take the
  temperature), the checks of the stores (`lacking_at`, `starve`), each conversion of growth (its release, upkeep and
  paying: `spend` takes the mass and the plateau state); a conversion with no room or nothing to make stops before
  any solve. 31 solves per body update fall to 15. Same results bit for bit (golden unchanged; 64 x 64, seed 1, 20
  years: same fingerprint); a body update about 3% faster (year 20: 40.8 to 38.3 µs, year 10 within the noise).
- E1 (d), speed: `Chemistry::minerals` is made once, at its first use, and kept beside the chemistry (`Derived`, not
  saved: a chemistry loaded makes it again), not as a new list at every body's shedding and every intake. Same
  results bit for bit (golden unchanged; 64 x 64, seed 1, 20 years: same fingerprint); a body update 1 to 2% faster.
- E1 (e), speed: what paying needs of the body's air (its depth under liquid, the cover over it, its breath and the
  air node's sum) is worked out once per state of the body (`pay::Payer`: with its mass and plateau state) and shared by
  the plan and the paying that follow, not again in each `plan`. The boundary layer (`exchange::Film`) gives its
  surfaces to `breath`, which no longer works out the capture area and the skin twice, and the body's physics shares
  it between its heat and its breath; the air's heat capacity is kept with the chemistry (`Chemistry::air_cp`). Same
  results bit for bit (golden unchanged; 64 x 64, seed 1, 20 years: same fingerprint); a body update about 2% faster.
- E1 (f), speed, the tape: the yearly clustering of kinds (`tape::measure`) stops its rounds when no body changes its
  kind (the rounds left changed nothing), and works out each distance to a centre once (the search for the nearest
  centre and for the furthest point worked each out twice). Same series and months byte for byte (64 x 64, seed 1, 20
  years); the two clusterings of a year 89 to 44 ms (about 2% of a living year).
- E1 (g), a fix: a body placed (a child, a part split off a node) and a node left by `nodes::open` are seen as what
  they hold (`Body::vis` worked out by `World::place` and `open`), not with the node's old traits until their first
  update. 64 x 64, seed 1, years 11 to 20: plant members -0.7%, body mass +1.6%, land -0.02 K, carbon gas 272.3 to
  269.2 ppm; golden re-pinned.
- E1 (h), a fix: the decision sees the body's mass and heat capacity as its update leaves them, after it sheds
  (`Lived`, worked out by `shed`), not as they were before it shed; `options` takes the volume at the temperature
  known. 64 x 64, seed 1, years 11 to 20: plant members +1.6%, body
  mass -1.0%, land -0.02 K, carbon gas 269.2 to 271.0 ppm; golden re-pinned; a body update about 2% faster (34.95
  to 34.10 µs, runs side by side).
- One clock: the parts of Flow (radiation with the state of ground and air, the exchange, the trace turn, the wind,
  liquid, creep, the refresh) are processes of the world's clock in `schedule::processes`, each a `clock::Every`;
  Flow's own timers (`pending`, `pending_liq`, `pending_exc`, `month`, `year`, `trace`, `DUE_SLACK`) go. Each part
  now moves for the time to its next run, counted from the month's start: the first liquid step stands for 15 days
  (it stood for 3), creep also runs at the world's start (it waited a year), and the trace gases turn at days 0, 12,
  24 of each month (at every fourth exchange since the start before). 64 x 64, seed 1, 20 years: land within 0.02 K,
  rain on land within 0.4%, rivers +2%, plant members within 1.3%. Snapshot version 18, golden re-pinned.
- State apart from cache: Flow's scratch saves only its state (`flow::scratch::State`: the plateaus of its last step,
  the clouds, what the last refresh found present); its caches are rebuilt at the next liquid step, which comes first
  at a month's start, where snapshots are taken. The survey's `Life::sum`, `env` and `seen` are not saved either.
  `Body::k` stays saved: a body's numbers are empty until its first update and other bodies read them so, which a
  rebuild on load would change. Same history (`series.csv` and `months.csv` byte for byte, 64 x 64, seed 1, 20 years;
  a resume from year 10 ends on the same world); the year-20 snapshot 43.97 to 42.93 MB. Snapshot version 19, golden
  re-pinned (the saved world changed).
- Fixed nodes by class and index: a ground, air, deep, boundary or potential node is named by its class and index
  (`store::Node`, in moves and transforms); `clock::Sleep` keeps its alarms per class by index; only a part keeps a
  slot and a generation, in `bond::Parts` (with its cell). The node table (`store::Nodes`, `World::table`) goes. A
  part's slot now counts from 0, so the keyed draw of a drifting raft changes: 64 x 64, seed 1, 20 years, the same
  history byte for byte; small roster worlds differ from year 3 only by that draw (with the old key, byte for byte).
  The year-20 snapshot 42.93 to 42.90 MB. Snapshot version 20, golden re-pinned.
- The wind's two solvers measured, both kept: on 64 x 64 (256 air nodes, 930 links) a call takes 0.22 ms along lines
  and 1.09 ms by `spread` on the mesh (best of 39, one thread), five times as long. Forced on the mesh, seed 1, years
  11 to 20, the world is not the same: rain on land -28%, liquid on land -72%, river cells 503 to 104, land in summer
  +1.4 K, plant members +20%; the mesh's overturning carries a vapour as if it filled its column, so the vapour the
  lower half brings to the warmer column is lost. Not kept (a probe on a branch).
- A fix, the tape: a resumed tape reads the viewer file's series without the columns that had no value yet at the snapshot, as the unbroken run did not have them, so a kind's catalyst column that first appears later reads NaN, not 0, in the years replayed. No world change; the golden fingerprints stay.
- The owner widens the key-number range of the eaters' mass to ± 0.5 log10 (`tape/tests/ranges.rs`): learning into
  their own rows, they weigh 1.6 to 2 times more than the reference eaters (4.88 against 4.497, three seeds).
- The owner widens the key-number range of the light-eaters' life to x/÷ 3 (`tape/tests/ranges.rs`): without a brain
  they live about twice as long as the reference light-eaters (4.07 against 1.895 years, three seeds).
- The plan and the handover follow an ablation hunt (64 x 64, 32 and 60 years, two seeds): E10 culls shared group
  brains (one brain per body gives the same world at the same cost per decision; a node already shares one brain),
  the `FORGET` gene (never on), old experiment flags, the 3-day shade sort and repeated temperature solves, and saved
  caches; E6 aims at child placement for every body (29% of the bodies' time after E9); Bond's joins of parts, never
  seen in any run, stay: the owner keeps Bond.
- E9 (a), the start: light-eaters start without a brain (`HIDDEN` and `MEMORY` 0): their decisions were mostly
  failed moves, watches and random gifts. The hand-set plant bias (`H_LIGHT`), the bias kept in their neutral start
  and `--plant-bias` go. 64 x 64, seed 1, 30 years: plant mass 10.59 to 10.68 (log10 kg), eaters 4.45 to 4.78, plant
  life 2.40 to 4.01 yr, eater life 3.12 to 4.83 yr, plant members +62%, land -0.36 K, rain on land +3%, carbon gas
  239.5 to 236.4 ppm; members split off 238,734 to 488; a plant update about twice cheaper (year 20, side by side on
  a busy machine: 72 to 35 µs; bodies 7.0 to 3.6 s per year). The series test turns off the mineral's breakdown
  (reaction 25), which grew past its tolerance with more mineral in bodies. Golden re-pinned.
- E9 (b): a body may be a node in individuals mode (`--individuals`) when it has no brain, not when it is a
  light-eater: the check compares brains in nodes and alone, and no law reads the light-eater threshold any more.
  Same results (golden unchanged).
- E9 (c): a body without a brain has no born rows and is in no brain group (`life::NO_GROUP`): joins no longer mix
  rows nothing reads, the groups' columns (`groups`, `learned`) count only bodies with a brain, and a brain that grows
  back starts from new weights (`brain::rerow`). The bodies without a brain now update in the order of their cells.
  64 x 64, seed 1, 30 years: the same world (plant mass 10.676 to 10.678 log10 kg, plant life 4.01 to 4.02 yr, eaters
  4.78 to 4.75, land +0.04 K); born-row sets at year 30 5,455 to 134; snapshot 12.9 to 10.6 MB; a body update a few
  per cent faster (side by side, busy machine). Golden re-pinned.
- The wind's solver on a mesh that is not square goes (`wind/mesh`, `wind_on` and its test): the planet is a grid of
  squares (D3), so the wind always runs along lines. A row that closes on itself (round the globe, G1) is solved as a
  ring by the cyclic form of the line step (`solve::lines::along_ring`, Sherman–Morrison). Same history (golden
  unchanged).
- Fixes of the body update over its span (found by the check of E4). (1) After growth and children the key liquid soaks
  in again at the same update (`physics::soak_again`): new matter raised the level of the key liquid after the
  soaking and children took their share of it, so a body started each update drier by the growth and the children of
  the last one, and its upkeep, catalysts and building ran slower over the whole span, the more so the longer the span
  (activity at the start of an update 0.94 at 20 days, 0.79 at 72). (2) A node's members die of age at the rate of
  their wear, the share 1 - e^-wear over the span, not the share `wear` (which killed too many over a long span).
  64 x 64, seed 1, years 11 to 20: plant activity 0.50 to 0.75, plant members -22%, body mass -3%, life of plants
  2.1 to 1.7 yr, land +0.1 K, carbon gas 247 to 250 ppm; golden re-pinned.
- A fix: the cover of other bodies over a body (`exchange::cover_of`) is the mass of the others in its cell now, summed
  over them, not the cell's sum of the last survey less the body's mass now (its mass had changed since the sum, so the
  others' cover was off by that change). 64 x 64, seed 1, years 11 to 20: plant members -8%, body mass -5%, life of
  plants 1.7 to 1.6 yr, land +0.1 K, carbon gas 250 to 254 ppm; golden re-pinned.
- E4, speed: a population node without a brain updates at most every `Config::node_days` (36 days, a new speed
  setting); a body with a brain keeps its `INTERVAL` gene (eaters slowed to 36 days fell from 4.6 to 3.9 log10 kg).
  The rules that counted by the gene's interval become rates over the update (`Body::paces`): the stores it uses
  refill a level per interval of the gene (`Body::room`), and frost and a shortfall of upkeep kill at the rate their
  share per interval gives (`Body::over_span`). 64 x 64, seed 1, 20 years: body updates 97,200 to 54,200 a year,
  bodies 1713 to 1053 ms/yr (1.6 times less; 16.6 to 18.4 us per update, as each does more), plant members +15%, body
  mass +12%, life of plants 2.3 to 2.8 yr, land -0.1 K, carbon gas 243 to 242 ppm. The key numbers hold (life of
  plants 2.61 to 3.10 yr, eaters 4.64 to 4.65 log10 kg) and so does the pace test (32 x 32, 8 years), but 64 x 64,
  30 years, seeds 1 and 2 (years 21 to 30) still show a step error larger than the seeds' noise (2% in life, 0.03
  in log10 mass): life of plants +7 to +9% at 18 days, +14 to +24% at 36, +7 to +18% at 72; plant mass +0.01 to
  +0.08 log10.
- E10 (c): the experiment flags of old questions go: the big-brain test (`--clever-year`, `--clever-n`,
  `life::seed_clever`, its random stream and its test), `--spray-size` and `--spray-genes` (now `ROW_SPRAY` and
  `GENE_SPRAY` in `worldgen`) and `--regrow` (`Config::regrow` stays). Golden unchanged; snapshot version 22 (the
  schedule a snapshot carries changed). `ew run` takes another path from year 2: the founders' gene spray now uses
  log10 3 rounded right by the compiler (the command line's 3 went through the library's log10, one unit in the last
  place lower), so its runs differ by chance: the same world.
- E10 (d), speed: the survey shares out the light of a cell again only when its bodies changed since (`Life::stir`:
  a body came, went, moved, changed its members, or was seen anew, `World::see`), not every cell at every physics
  step. Same history (golden unchanged; 64 x 64, seed 1, 35 years: same fingerprint); bodies 3 to 8% less time per
  year (side by side).
- E10 (e), speed: the body's temperature solve (`Body::thermal`) is passed on where the body has not changed: from
  the upkeep to the members who die of age or of a shortfall and to the physics of the time (paying leaves the body
  at the plateau state it paid at, `Body::energy_in`), from the checks of the stores to growth (its supply and its
  first conversion) and to the members who freeze or starve, and from shedding to what it sheds (`Body::heat_with`
  and `to_ground` take the state). 12.6 solves per body update fall to 7.6. Same to rounding (64 x 64, seed 1, 35
  years: without the upkeep's part the same fingerprint); the same world (64 x 64, years 26 to 35, seeds 1 and 2:
  plant members -2% and -1%, eaters 0.6 and 1.0 times their mass, land -0.1 K, carbon gas -3 and -4 ppm); a body
  update 4 to 13% cheaper (side by side, years 10 to 30). Golden re-pinned.
- E10 (b): the `FORGET` gene and the monthly fading of what a group learned go: the founders started it at its floor,
  where it is off, and no animal ever raised it (forgetting only makes a brain worse). 21 scalar genes; snapshot
  version 23. The same world by another path (the genes' random draws moved): 64 x 64, years 26 to 35, seeds 1 and
  2: plant members -8% and -6%, eaters 1.25 and 1.3 times their mass, land -0.1 and +0.1 K, carbon gas -7 and +6 ppm.
  Golden re-pinned.
- E6 (F): a node's children have one parent; individuals keep two (its members already mix inside it). 64 x 64,
  years 30 to 35, seeds 1 and 2: plant members -4% and +10%, life of plants 3.10 to 2.99 and 2.47 to 2.78 yr,
  animal members 1106 to 1143 and 752 to 485, land and carbon gas the same; the share of the bodies' time in
  `copy` 20% to 19%. Golden re-pinned.
- E6 (A, with F1): children who join a node are booked on it, with no body built for them; one helper moves members,
  stores, heat, brood and gene moments between nodes (`nodes::Lot`, `nodes::add`) for joins, merges, splits and
  births. The same world bit for bit (golden kept). A test splits and joins a node 10,000 times. `copy` per call
  about the same (10.2 against 10.5 us, side by side): its time is in drawing the genes (31%), finding the node (27%)
  and pooling the genes (25%), not in building the body.
- E6 (D): a node's children who leave the cell no longer need to make one whole member (less than one with no node
  of their kind to join waits in the brood, as those who stay do), so small nodes disperse too and dispersal does
  not hang on the span of an update. Years 30 to 35, seeds 1 and 2: plant members +4% and -1%, life of plants 2.99
  to 3.24 and 2.78 to 2.71 yr, animal members 1143 to 1039 and 485 to 598, land and carbon gas the same. E6 in all,
  side by side with E4: bodies 2810 against 2888 ms/yr (the same), `copy` 21% to 19% of the bodies' time. Golden
  re-pinned.
- E10 (a): shared group brains go (`life::Group`, `settle`, `weigh`, `resum`, `regroup`, `lineage::own`,
  `Body::group`, `near`, `since` and the body's own adjustment `adj`, the tuned `leave`, `join` and `stay`, the
  switch `--own-brains` and `Config::share_brains`). Each body keeps its own rows (`Body::rows`, with its critic's
  `var` and `ms`): its born rows plus what it learned; a node's members share the node's. A child starts with its
  born rows plus what its parent learned; members joining a node bring their rows into its mean by members; watching
  blends the watcher's rows toward the watched one's. Tape: `groups` goes, `watched_groups` becomes `watched_bodies`,
  `learned` is over bodies (zero in a null run). Snapshot version 24; tuned numbers 61 to 58; core 8,652 to 8,414
  code lines. 64 x 64, years 26 to 35, seeds 1 and 2 (after E6): eaters 1.6 and 2.0 times their mass (5.7e4 to 8.9e4
  kg, 4.7e4 to 9.6e4) and life 3.9 to 4.4 and 4.4 to 4.7 yr, beyond the seeds' noise; before E6 the old switch
  `--own-brains` gave the same as this change (eaters 1.02e5 and 1.13e5 kg against 1.04e5 and 1.08e5): each body
  learns at its own full rate, where a group moved its rows by each member's share. Light-eaters and climate the same
  (plant members -0.7% and -0.3%, land 0 K, carbon gas -0.6 and +0.6 ppm); the same cost per body update (side by
  side). Golden re-pinned.
- E10 (f): caches out of the snapshot. A body's numbers that follow from its genes (`Body::k`) are worked out at every
  change of the genes (`Body::set_genes`; a body placed, `World::place` and `World::add`; a world loaded) and not
  saved, so they are never empty when read; the lazy fills at an update, a payment and a gift go. A gene that is off
  keeps no spread where members join a node or are born (`body::spread_kept`), and a dead body's slot keeps no rows,
  spread or genes (dead animals' rows were 2.5 MB of the snapshot). Snapshot version 25. 64 x 64, seed 1, year 30:
  snapshot 14.7 to 10.0 MB (before E10 (a) it was 10.7 MB). The same world (years 26 to 35, seeds 1 and 2: plant
  members +1% and -3%, eaters 0.98 and 1.1 times their mass, land the same, carbon gas +2 and -1 ppm); the same cost
  per update. Golden re-pinned.
- E11, young that grow up: a kind in a cell is an adult node and a young node joined by a link (`Body::link`);
  a node keeps the spread of its members' log sizes (`Body::size_spread`), and the young members who grew past the
  copy mass in an update move along the link into the adult node (`copy::mature`: the flux across a size, none
  without growth). The size classes (`class`, `alike_log`, `alike_size`, `LEAST_KG`, `CLASS_SLACK`) and the merge of
  young into adult nodes go. Years 30 to 35, seeds 1 and 2: plant members 5.28e10 to 4.27e10 and 5.03e10 to 3.54e10,
  life of plants 2.83 to 3.34 and 3.12 to 3.83 yr, animal members 1127 to 684 and 912 to 1513, land +0.4 and +0.3 K,
  carbon gas 223 to 270 and 255 to 305 ppm. Young members are still 92% and 93% of the plant members (86% and 87%
  before), their median node 0.36 and 0.37 of the copy mass (0.31 and 0.30): a newborn lives about two months and
  grows about 0.025 decades in an update of 36 days, so most die young, and 0.9% of the young mature per update. The
  old merge took whole young nodes into the adults once their mean reached the copy mass, small members too. Pace
  (node_days 0/18/36/72, years 21 to 30): plant mass flat (within 0.006 log10), life of plants -0, -4, -10, -14%
  (seed 1) and -0, -2, -2, -16% (seed 2). Golden re-pinned; snapshot version 26.
- E11, the pace of nodes made flat: a node's last day (`Body::last`, now a real number) and its mark of light become
  the means by members when members join it between updates (`nodes::add`), so its next update integrates their own
  member-days, not the whole span for members who came at its end; and a shortfall of upkeep kills the share of the
  members it leaves unpaid over the update, not that share compounded over the gene's intervals in it. Pace
  (node_days 0/18/36/72, years 21 to 30): life of plants +0, -0.4, -2.2, +2.5% (seed 1) and +0, -1.5, +3.8, +1.7%
  (seed 2), from -0, -4, -10, -14% and -0, -2, -2, -16%; mass within 0.005 log10. The time of joining members alone
  gave -0, -1, -7, -8% and -0, -3, -3, -11%. Years 30 to 35 at 36 days: plant members 4.27e10 to 4.48e10 and 3.54e10
  to 3.69e10, life of plants 3.34 to 3.69 and 3.83 to 4.08 yr. Golden re-pinned; snapshot version 27.
- E12, selection inside a node: a node without a brain also runs its update, as far as its growth, for a sample
  member one standard deviation of its spread from its mean on one gene (drawn, with its side, `Purpose::Select`),
  on the world as it is, then given back (`copy::sample`); the mean of that gene moves by the step times tanh of what
  the sample gained over the node, times the number of genes drawn from, at most a step (`copy::select`: Price's
  equation, the breeder's equation). Tried and not kept: all genes at once with drawn signs (one sample for all): the
  other genes' effects moved genes that do nothing (the brain sizes of light-eaters) by chance, 100 lineages by year
  30, and a run failed. Years 30 to 35, seeds 1 and 2: plant members 4.43e10 to 5.36e10 and 3.63e10 to 4.31e10,
  plant mass 4.18e10 to 5.33e10 and 3.41e10 to 4.16e10 kg, life of plants 3.77 to 4.37 and 3.84 to 4.11 yr, animal
  members 2617 to 1677 and 2837 to 2030, land -0.4 and -0.3 K, carbon gas 273 to 251 and 307 to 293 ppm; repair gene
  -0.97 to -0.87 in 30 years (-0.96 without). The samples are about 35% of the bodies' time (16 and 11 us each, 79%
  of them on genes that change nothing in an update). Golden re-pinned.
- E12, speed: no sample is run where the drawn gene is one the update never reads in a node without a brain (its
  children's and its brain's genes, `copy::select::UNREAD`); each gene is still drawn as often. 64 x 64, seed 1, 30
  years, side by side: samples 278,900 to 230,600 in 5 years, their time -14% (31% of the bodies' time, from 35%),
  bodies 1740 and 1718 ms/yr; the world the same to four digits (the sample's scratch of the reactor no longer runs
  before the node's update, so the last digits move). Drawing only among the genes read, the number of genes as the
  multiplier, ran as many samples and saved nothing. Golden re-pinned.
- E12, speed: no sample either for an uptake or keep gene the update cannot read (`copy::select::read`: uptake only
  as the roots' on or off for what does not dissolve; keep only for the matter, built solids, key liquids, products of
  catalysed reactions, liquids that dissolve and what the roots take). 64 x 64, seed 1, 30 years, side by side:
  samples 230,600 to 153,400 in 5 years, their time -35% (22% of the bodies' time, from 31%), the world the same to
  four digits. Golden re-pinned.
- E12, speed: a node without a brain runs its sample on one update in `sample_every` (2, a speed setting), drawn, with
  its weight and cap that many times larger. 64 x 64, seeds 1 and 2, 30 years: the samples' share of the bodies'
  time 22% to 12% (7% at 4). x2 and /2 against other draws at 1 (years 10 to 30): plant members -3% and -5% at 4,
  +1.5% and +4% at 1 (noise 2%); plant life -2% and -5%, +2% and +4% (noise 2.5%); animals, carbon gas and land
  temperature within noise. The cap not scaled with the weight lowered plant mass 8% and 11% at 4. Golden re-pinned.
- E11, the sizes of the young kept between birth and the copy mass: a node keeps the mean of its members' log10
  bodies (`Body::size_lag`), pooled as the moments of the mix, no longer the mean of a lognormal of the mean body (a
  wider spread lowered that mean, which widened the spread at the next newborns: young nodes' variances ran to 1e16
  and NaN in the 12-day air of J1, all three seeds); `copy::mature` fits a normal cut at the copy mass, so members
  leave across it at the rate a hard edge gives (an uncut normal let members pile up past it). Nodes whose size
  variance passes a quarter of the birth-to-copy range squared with a mean inside it: 336 to 10 (64 x 64, seed 1,
  year 30). Years 26 to 30: plant members 4.79e10 to 4.94e10, plant life 3.87 to 4.10 yr, plant mass 5.65e10 to
  5.38e10 kg, carbon gas 243 to 251 ppm; young 91.8% to 91.6% of plant members, their median node 0.30 to 0.28 of
  the copy mass. With J1's air all three seeds run 30 years. Snapshot version 28; golden re-pinned.
- E11, maturing from the logs of the tails (`math::ln_upper_tail`, `copy::young::above`): a young node of members
  starved for decades and newborns (sd 1.9 decades about a mean 1.8 below the copy mass) is wider than any normal cut
  at the copy mass, so `fit` takes its deepest cut and the uncut sd grows to 16 decades; the body's share of the
  maturing slice was then a ratio of two normal tails that both underflowed, 0/0, and the node went NaN (the century
  test, 16 x 16 seed 1, year 46). The shares are now ratios of tails taken from their logs, finite at any depth; the
  same law. A test of a spread that wide. Golden re-pinned (rounding).
- E5 (2), one bodies' node per cell (`store::Class::Bodies`, `World::e_bodies`): Flow books the light bodies' light
  reactions will store from the sun node into it, and before each physics step it settles with its ground to hold
  what its bodies will claim (the light each caught since by its share, the heat each draws at the rate of its last
  update); `World::light_pool`, `heat_pool`, `light_keep`, `heat_keep`, `PerSlot` and `survey::hold_heat` go (the two
  shares are `Body::keep` and `Body::drew`). The ground no longer gets the heat a body gives back ahead of its update.
  `store::apply` sums the changes of the boundary nodes and the potential apart and adds them once: the light set
  aside, cell by cell against the sun's sum, rounded away and the energy drift grew to 4.8e-13 in 30 years. 64 x 64,
  seed 1, years 25 to 30: plant members 3.88e10 to 4.03e10, plant mass 4.97e10 to 4.99e10 kg, plant life 4.51 to
  4.54 yr, land 287.91 to 287.89 K, carbon gas 252 to 254 ppm, eaters 480 to 410 members (noisy); energy drift at
  year 30 1.7e-14 to 1.6e-14; a body update 30.7 to 31.8 us. Snapshot version 30; golden re-pinned.
- E5 (1), bodies book through moves: what a body takes from and gives to its cell's ground, air and bodies' node
  (liquids, minerals, food, breath, the gas of growth, heat, remains) and a part's monthly trades with the ground and
  air are moves gathered over the update and booked by one `store::apply` at its end (`store::Moves`); the body or
  part books its own side (`Class::Body`, `Class::Part`), and reads its cell's nodes as its moves leave them
  (`World::g`, `a`, `heat_now`). A sample member drops its moves (`copy::select::Shadow` goes). `ground::draw`,
  `take_paid`, `World::take_paid`, `flow::Sky` and the direct writes of bodies and parts go; the formulas stay. The
  same world to the bit (every series column, 32 x 32, two seeds, 5 and 8 years; the whole world but the boundary
  nodes and the potential, 16 x 16 with every kind, three seeds, 4 years); only the potential's sum rounds otherwise
  (`apply` adds a call's lifts at once). Golden re-pinned.
- E5 (3), a private store: the amounts and heat of the fixed nodes are private arrays of `store::Store` (by class,
  read through `Store::amounts` and `Store::heat`), which only `store::apply` writes; the bed is a class like the
  others (`Class::Bed`, amounts and no heat), broken into loose ground by a move (`Move::loosen`), so `World::loosen`
  goes; the deep stores are sized when the world is made (`World::fit_deep` goes); an edit is moves from a boundary
  node (`World::cross`), and the tests set the store by edits (`testkit::set_amount`, `set_heat`). The compiler keeps
  the contract between the layers: `cli/tests/contract.rs` goes. The same world to the bit (the whole store and every
  body, 16 x 16 with every kind, three seeds, 4 years); the fingerprints change with the saved layout. Snapshot
  version 31; golden re-pinned.

Phase E review, round 1 (fixes):

- Merged pairs and cheaper gene numbers: one form each of `Body::thermal` (it finds its heat capacity), `temp`,
  `volume`, `supply`, `lacking`, `body::upkeep`, `copy::grow` and `body::Payer` (each takes what its caller knows);
  `copy::grow` for tests is `testkit::grow`; `Lived::cap` goes. `Body::prepare` fills the vectors of `Consts` again
  instead of making new ones, and a brain works out what it perceives of its safe minimums itself (`Consts::minln`
  goes); evaporation tests the cheap conditions before the vapour pressure. The same world to the bit (64 x 64, seed
  1, 30 years, every series column).
- One even 10-day step for the physics and the air (`phys_days` 10, a speed setting; `air_hours` 240): the survey,
  Flow's state and Change run 36 times a year, not 120, and the air's step is booked whole as it is solved
  (`air::book`; `Pending`, `dispense` and the booking a physics step at a time go). The air's step must divide the
  physics step: booked whole, a 12-day air step over 3-day physics steps warmed the land 0.8 K. The long-step test
  compares with 6-hour air steps and 3-day physics steps: every climate number within the seeds' noise (the warmest
  month 0.26 K cooler, noise 0.33). 64 x 64, seed 1, 30 years: plants 10.73 to 10.72 log10 kg, members 4.10e10 to
  4.04e10, plant life 4.75 to 4.70 yr, eaters 4.50 to 4.49 log10 kg (life 4.2 to 3.2 yr, a few hundred members),
  land 287.98 to 287.87 K, swing 12.65 to 12.49 K, rain the same, carbon gas 254 to 259 ppm, drift 7e-15 and 2.4e-14;
  Change 787 to 374 ms/yr, Flow 2055 to 1816, the whole run 130 to 109 s. Snapshot version 32; golden re-pinned.
- Why life made the air dearer: with life, three substances that cannot evaporate (saturation pressures near 1e-38
  Pa) stood in the air at 1e-14 to 1e-30 mol, put there by bodies' breath, and the air's step solved each as a gas
  (a banded solve per round, 7 numbers per cell, a crossing per cell); and the rounds went from 3.0 to 4.1 per step
  as the land's water vapour moved more (the canopy). The air's gases are now those it holds more of than the
  rounding of its sums, or that can evaporate; the ground's gas crosses to its air node as one column per gas (no
  `Batch` per cell and gas, no `FlowView` per crossing, no sums of the air done twice). 64 x 64, seed 1, 30 years:
  Flow 1811 to 1410 ms/yr at year 30 (947 at year 10, against 1208); the world within the noise a rounding change alone
  gives (plants 10.716 to 10.705 log10 kg, plant life 4.70 to 4.24 yr against 4.27 with the column booking alone,
  land 287.87 to 287.86 K, carbon gas 259.1 to 259.6 ppm). Golden re-pinned.
- No silent cuts of a body's stores: what upkeep, growth and digestion take from a store changes it whole, and what
  rounding leaves below zero is set to zero and counted in `Counters::rounded` (`body::add_store`), as `store::apply`
  does for the store's nodes; a debug build checks it is only rounding. The gas upkeep takes from the air is no longer
  cut to what the air holds (`apply` counts its rounding). The same world (golden; 64 x 64, seed 1, 8 years).
- A body's upkeep books its gas through `store::Moves::gas` with the lift, as its breath, reactions, evaporation and
  growth do: one path for every gas a body exchanges with the air. 64 x 64, seed 1, 30 years: within the noise (plants
  10.705 to 10.713 log10 kg, plant life 4.24 to 4.63 yr, land 287.86 to 287.87 K, carbon gas 259.6 to 261.3 ppm,
  energy drift 1.9e-14).
- `ew resume` into a new `--out` folder: the recorder read the surface heights of the survey's cache (`Life::env`, not
  saved) before the first survey; it now works them out as the survey does when the cache is empty. The resumed run
  matches the unbroken one (years 21 and 22, every series column). The learning test runs its seeds on threads (the
  same tally; 6.3 to 3.4 s).
- No take cap per update: a body took at most half of what its node held in one update (`MAX_TAKE_SHARE`), so the cap
  per day differed up to 7 times between a node (36 days) and an individual (5 to 20). What it breathes, soaks up,
  takes by its roots or eats is now set by the law's own rate over the span and never more than the node holds. Rate
  forms bounded by the soaking rate, and the breath of a well-mixed node, gave the same world as the plain bound. 64 x
  64, seed 1, 30 years: plant members 3.90e10 to 3.11e10, plants 10.71 to 10.64 log10 kg, plant life 4.63 to 3.16 yr
  (the cap had spread the ground's liquid and minerals over the bodies of a cell), eaters 4.38 to 4.42, land 287.87
  to 288.03 K, swing 12.45 to 12.11 K, carbon gas 261 to 261 ppm.
- `flow::step`, which only tests called, is `testkit::flow_step` (Flow's processes are public to it).
- Selection inside a node sees reserves: a sample's gain and its node's are the log of what can become body
  (`copy::buildable`: the matter, brood with it, and the matter its stores could make by the reach of growth's
  conversion) after over before, not of the matter alone; growth held by its allowance hid every gene that brings more
  fuel (a leaf, pores, a coat). 64 x 64, seed 1, 30 years: carbon gas 260.5 to 236.3 ppm (the light-eaters take more
  of it), land 288.03 to 287.54 K, swing 12.11 to 11.93 K, plants 10.64 to 10.70 log10 kg.
- Selection inside a node skips the samples whose step cannot reach a threshold in the update (a safe minimum whose
  store holds more than twice the step's upper end, `KEEP_T` and `COOL_T` farther than the step from the body and its
  ground, uptake and keep genes of what the ground does not hold as liquid or the store already holds, gases, the
  body matter's keep gene): 7,600 samples a year at 64 x 64 (35% that move nothing) against 14,400 (63%). A sample at
  every update would cost about 5% more than before the skips, so `sample_every` stays at 2 and the samples cost
  half. The light-eaters' `INTERVAL` gene stays the turnover time of their stores (decision 11).
- Bodies catch the rain that fell since their last update (`World::rained`, the cell's rain since the start, and a
  mark per body, `Body::rained`, as for light), not the yearly mean rain times the span: the seasons reach the leaves.
- `flow_days`, `crowd` and `MIN_INTERVAL` are marked speed in `docs/numbers.md`, not tuned (`alike` stays tuned: it
  says who mixes); the speed rows lose their run results; `TUNED_BUDGET` 57 to 52 (the count was 55).
- Phase E in all, from the plan's rows: E1 made a body update about 15% cheaper (64 x 64, seed 1, year 20: 42.5 to
  35.5 µs); E10 removed about 226 code lines and 3 tuned numbers; E5 kept a body update's cost and added 30 code lines
  to `core` (the move buffer and its read cache).
- The cost model (plan 5.7) after E1: a cell 290 µs and a body node 650 µs per simulated year; after the review's
  10-day steps: a cell about 0.48 ms, a body node about 360 µs (64 x 64, with life).
- `docs/laws.md` says each law once and shorter (1,117 to 776 lines): no function-by-function walk, numbers only
  where they help to read a rule (the rest are in `docs/numbers.md`); "Stability" joins the books in section 0.

**Phase F: population nodes**

- F7, the check of nodes against individuals (handover 7.1) as one slow test, `tape/tests/nodes.rs` (the run as `ew
  run` makes it moves to `tape/tests/common`, shared with `ranges.rs`). Its windows are years 41-50, and 26-30 with the
  roster (after its last kind is placed, decision 14; handover 7.1 says so): at 32 x 32 the eaters as individuals pass
  10^4 bodies by year 70 and the roster's eaters of remains 10^5 by year 28, doubling each year. It fails, past the noise of the crowd setting (2.7 times on the eaters' mass): in years
  41-50 seed 1's eaters are 24 times the mass as nodes as individuals, seed 5's 7 times lighter with a real life 3.6
  times shorter; in the roster the real life of the eaters of remains, small and big, is 2 to 2.7 times shorter as
  nodes (members die of age at a constant rate from birth, individuals at the end of their life), and gene means of
  the eaters move apart. The big slow eaters of light-eaters are too few to compare (their real life is 25 and 0.9
  years at the two crowd settings).
- F7 follow-up, the brood rots with the matter it is part of (`Body::brood_share`): a body's own reactions (the
  rotting of reaction 21) and burnings used its body matter, the brood held in it with the rest, but took it all from
  the body, so a body holding a brood shrank faster. Individuals, who hold up to a whole child, shrank about 13% more
  each update than nodes. 64 x 64, seed 1, years 31 to 35: plant members 4.22e10 to 4.12e10, plant mass 5.45e10 to
  5.21e10 kg, plant life 4.73 to 4.60 yr, eaters 730 to 477 members (noisy), eater life 5.3 to 4.3 yr, carbon gas
  247 to 252 ppm. Golden re-pinned.
- F7 follow-up, a node holds half a child of brood per member (`copy::BROOD_HELD`): its members each fill their own
  brood and make a child when it covers one, so their broods lie evenly between none and one child; a node made all
  its brood into members at once, and lost none with members who died, so its adults made 1.4 times the children of
  individuals per kg eaten (eaters, 32 x 32, seed 1, years 30 to 45: 1.25 against 0.90 children per adult a year;
  0.83 against 0.87 with this). 64 x 64, seed 1, years 31 to 35: plant members 4.12e10 to 4.08e10, plant mass
  5.21e10 to 5.31e10 kg, plant life 4.60 to 4.77 yr, eaters 477 to 271 members, eater life 4.3 to 4.0 yr, carbon gas
  252 to 250 ppm. Golden re-pinned.
- F7 follow-up, members pass a level by the steady state of a stage (`math::Pass`, for maturing): below the copy
  mass the young's log10 bodies have a density e^(-k y) over a width, fitted to their mean and variance (the steady
  state of a stage entered at one size, with growth and deaths at one rate each), not a normal cut at the copy mass.
  Members spread evenly (a stage with no deaths) mature exactly as the evenly spread do (the test now says so); a
  spread wider than the family holds is an exponential tail of the same mean (the deepest normal cut dropped the
  rest's mean by about a decade). The normal-cut code goes (`copy::young`'s `density`, `above`, `cut`,
  `fit`, `math::ln_upper_tail`). 64 x 64, seed 1, years 31 to 35: plant members 4.08e10 to 4.07e10, plant mass
  5.31e10 to 5.27e10 kg, plant life 4.77 to 4.54 yr, eaters 271 to 269 members, eater life 4.0 to 4.8 yr, carbon gas
  250 to 255 ppm. Golden re-pinned.
- F7 follow-up, a starving body dies with the chance of the share of its body it lost (`body::live::starved`; a node
  loses that share of its members, as in a frost): a body whose growth did not rebuild what rotted or burnt shrank
  for ever, a node all its members alike (young light-eater members from 5.7 kg to 3 g in 30 years, their count kept),
  an individual too (eaters of a few grams living on their fuel). Below its copy mass only: an adult shrinks towards
  it as it chooses. 64 x 64, seed 1, years 31 to 35: plant members 4.07e10 to 3.36e10, plant mass 5.27e10 to 4.76e10
  kg, plant life 4.54 to 3.26 yr, eaters 269 to 340 members, eater life 4.8 to 5.8 yr, carbon gas 255 to 251 ppm.
  Golden re-pinned.
- F7 follow-up, one rule of death (`body::live::lose_share`): what kills a share of a node's members (unpaid upkeep,
  the wear's 1 - e^-wear, frost, starving) kills an individual with that chance; an individual died at any unpaid
  upkeep and of age at a fixed life, a node's members at a constant hazard, so its young died of age as much as its old
  and the eaters of remains as nodes lived 2.2 to 2.8 times shorter. `Body::wear` goes. 64 x 64, seed 1, years 31 to
  35: plant members 3.36e10 to 3.43e10, plant mass 4.76e10 to 4.80e10 kg, plant life 3.26 to 3.36 yr, eaters 340 to
  336 members, eater life 5.8 to 6.4 yr, carbon gas 251 to 253 ppm; F7 differences past the noise 17 to 10. Snapshot
  version 34; golden re-pinned.

- F7 follow-up, speed: the maturing fit halves 40 times, on one shape a step (60, on two). 64 x 64, seed 1, years 31 to
  35, the same world within its noise: plant members 3.43e10 to 3.29e10, plant mass 4.80e10 to 4.79e10 kg, eaters 336
  to 315 members. Golden re-pinned.
- F7 follow-up, a partner is one parent (`copy::mix`): children take each gene and born row from one parent or the other
  by even chances, not by the partner's share of the members of both (a node of 50 members gave 98% of its genes); the
  branches of `share` for more than 32 children of a litter, which never ran, go. The same world to the bit at 64 x
  64, seed 1; golden unchanged; F7 differences past the noise 10 to 9 (the default start passes all three rules).

- F7 follow-up, choices as a spread of actions (handover 7.1; `choose/spread.rs`): a node with a brain took one
  action for all its members, so its members fed, moved and rested all together. Now count × chance of its members do
  each action, a part acts as so many members would, a part that moves joins a node of its kind where it lands, and the
  node learns as from one member drawn among those who stayed (with how much more its part's action raised their
  well-being than the mean): learning from the mean of all parts made node brains near certain (eaters of remains took
  from the ground at 0.999, individuals 0.958). The old one-action path for nodes goes. 32 x 32 roster, seed 1, years
  26-30, as nodes: eaters of remains 4.5e3 to 9.3e3 members (individuals 6.9e3), small eaters of light-eaters 3.4e3 to
  2.8e3 kg (individuals 970), big eaters of light-eaters 1.2e3 to 1.3e3 kg (individuals 5.6e3). Snapshot version 36;
  golden re-pinned.

- F7 follow-up, predators reach node prey as they reach individuals (`choose::take_body`, `body::wound`): from a node a
  taker reached only what the area it works holds, and a bite killed members only past a whole member's body, while an
  individual's bite reached all its prey held and killed it past half its body. Now each member of the taker meets one
  member of the other, as many as both have: it reaches what they hold, tears each of them, and kills those it leaves
  half their body or less (an individual as before); of a node's bites the share its interval gives gets away. Giving
  goes the same way (a node gave as one member). Pushing through wears each member alike; `tear` goes. 32 x 32 roster,
  seed 1, years 26-30, nodes against individuals: eaters ate other bodies 1.6 against 306 kg a year before, 68 against
  373 now; small eaters of light-eaters 2.8e3 against 970 kg before, 1.7e3 against 1.3e3 now; strong fast eaters of
  eaters 1.2e3 against 1.3e3 kg before, 510 against 1.2e3 now (few bodies). Golden re-pinned.

- F3 (decision D8): unlike nodes never merge, however full the cell; the safeguard that merged them past twice `crowd`
  goes. 32 x 32 roster, seed 1, years 26-30, as nodes: eaters 210 to 110 members (individuals 56), eaters of remains
  1.1e4 to 3.1e3 members (individuals 1.5e4), small eaters of light-eaters 1.7e3 to 6.3e3 kg (individuals 1.3e3).
  `crowd` stays a speed setting: past it the most alike merge. Golden re-pinned.

- F7 follow-up, a node is of one lineage (decision 23): alike nodes, children, maturing young and members who moved
  joined nodes of other lineages, which then acted by a brain base not theirs, and the larger's label took the smaller's
  members and genes (the eaters' copy-mass gene 1.33 as nodes against 1.63 as individuals). Only bodies of one lineage
  join or merge now; `Life::carry_rows` and `Purpose::Merge` go. 32 x 32 roster, years 26-30, seed 1, nodes against
  individuals: eaters 4.4e3 against 2.7e3 kg (copy-mass gene 1.65 against 1.63), big eaters of light-eaters 3.2e3
  against 2.8e3 kg, eaters of remains 1.1e4 against 2.6e4 kg. Golden re-pinned.

- Intake from the ground by Holling's type II response (Holling 1959; `choose::feed::take_node`): take x s / (take + s)
  for s what the area it works holds, not min(take, s), with no new number. It does not limit the eaters' booms: the
  dead matter on the ground (1 to 4 kg/m²) holds 50 to 200 times what makes s equal to take. 48 x 48, 60 years, eater
  members at year 60 in seeds 1 to 3: none (gone by year 55), 929, 447 before; 27, 1171, none (gone by year 50) with
  it: booms and busts either way. A few lines of `choose` fold, the same world. Golden re-pinned.

- The eaters' founders keep 278 K (`KEEP_T`, `EATER_KEEP_T`: their key liquid's melting point plus the 5 K margin of
  well-being), not 250 K: at 250 K the gene did nothing, so no selection could move it, and eaters on the hills froze
  in winter. A slow check holds that the eaters live to year 60 (`tape/tests/ranges.rs`, 48 x 48, seeds 1 to 3). 48 x
  48, 60 years, eater members at year 60 in seeds 1 to 3: 27, 1171, none before; 789, 3046, none (gone by year 55,
  short of fuel and frozen) now. Golden re-pinned.

- Comments of nodes, copy and the update say what the code does in two or three lines and point to laws.md; `WITHIN`
  is marked a speed setting (how members are grouped), tuned rows 52 to 51. The same world.

- F7, after the fixes above: the check holds the rules that pass (default start: b and c; roster: b) and prints the
  rest (decision 25); rule (b) compares gene moves in the copy-mass gene's units, as a 0.3 K move of `KEEP_T` (step
  5 K) flagged it. Past the crowd noise, 10 differences: default start, seed 1, the eaters 1.5e4 against 1.0e3 kg (a
  boom; the noise 7 times); roster, small eaters of light-eaters 3 to 3.5 times lighter as nodes, the big ones 5 times
  (seed 5), the eaters of eaters few (1 to 9 members) and their real lives far apart, and in seed 5 what eaters eat
  second (light-eaters as nodes, other bodies as individuals). A node is still found as one body (5.2), and nodes with
  a brain select inside on nothing: a sample without the decision saw only the costs of what pays through it and drove
  the eaters of remains to boom (8e6 against 1e4 kg), so it was dropped.

- The spread of actions in fewer lines: a member doing an action of Bond is a part of one (`choose::spread::part_acts`),
  its change of well-being counted like any part's; `Kit` holds the well-being as a function. Core code back under its
  ceiling of 9,000 lines (9,027 to 8,996). Golden re-pinned.

**Phase J: the long physics step**

- J1, what holds the step at 3 days (worlds with no life, 64 x 64, seeds 1 and 2, years 7 to 12; the steps of the
  physics, wind and exchange from 6 hours to 30 days). Rain on land falls as the step grows: 6 hours 3.9e-3 and
  4.6e-3 mol/m²/s, 12 hours 3.7e-3, 1 day 2.8e-3, 2 days 1.6e-3, 3 days 1.17e-3, 6 days 0.81e-3, 15 days 0.42e-3;
  river cells 814, 1069 (1 day), 506 (3 days), 132 (6 days), 8 (15 days); the seasonal swing on land 12.1 K at 6
  hours, 9.9 K at 3 days, 11.6 K at 15. So the 3-day world is not converged, and an accurate long step would not
  give it. The causes, term by term: radiation solves the ground implicitly but takes the air's temperature as it
  was (it relaxes in about a week: at 15 days it swings, at 30 days it grows without bound and the run fails in its
  first month); the wind evens 25% of a pressure difference per call (`EVEN`) and lets the overturning move at most
  3 columns per call (`OVER`: at 3 days it binds; 12 times looser, rain on land +33%, river cells 506 to 860), and
  its mixing evens two columns within a call; the exchange lets the air over a cell take or give no more than it
  holds in a call (its cap of evaporation at the column's saturation does not bind at 3 days); split from each other,
  each of these moves at most what a node holds per call, so what crosses per day falls as the call grows. The
  exchange alone at 3 days (the rest at 6 hours) has 52% less rain on land, the wind alone 63%. Liquid holds 30
  days.
- J1, liquid every 30 days (`flow_days` 15 to 30): the rain on land (+0.6%, +0.5%), river cells (+0.4%, -0.7%) and
  land temperature (+0.1 K) of the runs above stay; Flow about 25% cheaper (side by side, 64 x 64, seed 1: 1,853 to
  1,386 ms per year without life, 1,878 to 1,426 with life, on a busy machine). Golden re-pinned.
- J1, the air in one implicit step (`flow::air::step`, `newton`, `book`; `solve::band` replaces `solve::lines` and
  `wind/along`): radiation, evaporation, rain, the gases that dissolve and the wind solved together by backward Euler,
  Newton rounds on a banded LU solve of the whole air level; `EVEN` and `OVER` become rates (pressure evened in the
  time sound crosses a link; overturning uncapped); `air_days`, `exchange_days` and `TRACE_TURN` go; `air_hours`
  (288) comes; the step's moves are booked a physics step at a time, by Flow's state, first at each time. Long step
  against 6 hours (3 seeds, 32 x 32, 8 years with life; off, noise): land -0.09 K (0.71), warmest month -0.19 K
  (0.29), coldest +0.03 K (1.19), rain on land +0.2% (4%), absorbing gas +0.04% (4%), river cells +0.3% (37%); at 144,
  216 and 360 hours the warmest month is 0.13, 0.18 and 0.33 K cooler (360 past the noise: first order in the step).
  Air numbers re-anchored: `mix` 2e4 to 1e6 m²/s (weather eddies; anchored), `rain_time` 1 day (new, tuned: with an
  hour or two all rain falls on the coldest cells); tuned rows 61 to 58. 64 x 64, seed 1, side by side with main:
  Flow 438 against 323 ms per year without life (6 years), 860 against 470 with life (20 years), Change the same;
  rain on land 1.58e-3 against 1.15e-3 mol/m²/s without life, 1.79e-3 against 1.49e-3 with life. Air steps of 6 and
  72 hours cost 8,200 and 950 ms per year. The growth test of `copy` holds its body's ground at 289 K (the new climate
  is 4 K warmer there). Golden re-pinned.
- J1, speed: the slope of evaporation with the open rate is the step where the ground stays wet
  (`exchange::bucket_slope`): the cell pass -6% of its instructions, Flow -1%; the same world to 4 digits. Tried and
  not kept: a first guess from the step before (Newton rounds 3.1 to 3.6), the factors of the round before refined to
  1e-8 (4 corrections cost more than a factoring), the air's layers out of the surface's inner loop (no gain). Golden
  re-pinned.
- J1, the climate's references from the converged world (64 x 64, seeds 1 to 3, 30 years, years 21 to 30): land
  289.31 to 288.45 K, seasonal swing 10.19 to 12.68 K, rain on land 1.496e-3 to 1.842e-3 mol/m²/s, absorbing gas 257.7
  to 271.2 ppm, river cells 380.4 to 784.5, land worn 0.0510 to 0.0717 mm/yr. Life's stay A2's and hold: light-eaters
  10.68 log10 kg (10.53), eaters 4.41 (4.50), life of light-eaters 4.45 yr (1.90, x/ 3), of eaters 4.40 yr (2.91,
  x/ 2). The long-step test, run once: land -0.10 K (noise 0.71), warmest month -0.21 K (0.34), coldest +0.04 K
  (1.19), rain on land +0.2%, absorbing gas -0.1%, river cells +0.3%.
- J3, the air's rounds (decision 21): the slopes of the Newton step are exact where they were not (what evaporates
  follows the node's temperature as it follows the air, `Balance::follow`, 12% of that slope; the energy of a node at
  the end is its heat capacity then times its temperature, so the vapour's heat capacity is in the Jacobian; the rain
  leaves with its node's heat); the wind's pushes and the clouds come from the step's start (a fixed-point iteration
  before, which only shrank by about 0.65 a round on a globe); the damped step goes (no longer needed); the slopes are
  summed per pair of air nodes once a round. Rounds a step, side by side with main: 64 x 32 globe without life, 6 years,
  13.2 (a fifth at the cap of 16) to 6.4 (a fifth at the cap: a cold air node whose ground nodes rain out of it what
  its moist neighbour holds); 64 x 64 island without life 3.1 to 2.2. Flow 682 to 411 ms a year on the globe, 616 to
  494 on the island; rain on land 1.445e-3 to 1.443e-3 (globe), 1.559e-3 to 1.561e-3 (island). Golden re-pinned.
- J3, rain comes out of the four air nodes a ground node sees, each in the share it gives of the vapour the node sees
  (`air::shares`), with its heat, and is booked over a link to each; before, it came out of the node's own air node
  alone, so a cold air node beside a moist one was asked for more than it held, the Newton step wanted it below zero
  and the rounds stopped at their cap. Side by side with the commit before, 64 x 32 globe without life, 6 years: rounds
  6.4 to 3.6 (none at the cap), Flow 354 to 277 ms a year, rain on land 1.443e-3 to 1.607e-3 mol/m²/s; 64 x 64 island
  without life: rounds 2.2 (the same), Flow 475 to 509 ms a year (the slopes of the shares), rain on land the same.
  Golden re-pinned.
- J3, tried and not kept: a second-order air step (BDF2, Curtiss and Hirschfelder 1952: a third of what the step before
  moved booked again, then backward Euler over two thirds of the step), 64 x 32 globe without life: unstable in year 4,
  the history and the wind's pushes from the step's start grow a two-step swing of a small polar air node's heat (its
  wind heat 1.4 times larger each step) until it is overdrawn; with the pushes from each round the rounds come back,
  and it is about 50 lines past the code ceiling. A 30-day physics step with three 10-day air steps: the long-step
  test holds (land -0.05 K, warmest month -0.08 K) and Change costs half (64 x 64 with life, 320 to 160 ms a year),
  but the key numbers move past their noise: land 288.06 to 289.30 K, seasonal swing 11.83 to 12.83 K, eaters 4.33
  to 4.09 log10 kg; a 15-day physics step with 5-day air steps: land 289.69 K, out of range. So the physics step
  moves a 64 x 64 world over 30 years more than the long-step test (32 x 32, 8 years) shows. The cell pass on threads
  (already `par::rows`): the same fingerprints, no gain on the 64 x 32 globe (gather and the banded solves are serial).
  The slopes added one slice per pair of air nodes: no gain.
- J3, at the end (side by side with main, one thread): 64 x 32 globe without life, 6 years: rounds 13.2 to 3.6, Flow
  682 to 285 ms a year, Change 71 to 59; a cell's Flow and Change 0.37 to 0.17 ms a year. 64 x 64 island with life,
  20 years: rounds 3.9 to 2.9, Flow 799 / 1,273 to 810 / 1,128 ms a year (years 5 / 20), Change the same. Key numbers
  (3 seeds): land 288.06 K, swing 11.83 K, rain on land 1.806e-3, absorbing gas 251 ppm, river cells 837, land worn
  0.081 mm/yr, light-eaters 10.64, eaters 4.33, lives 2.59 and 4.10 yr. The long-step test: land -0.15 K (noise 0.76),
  warmest month -0.19 K (0.38), coldest -0.27 K (1.22), rain on land +0.1%, absorbing gas -0.2%, river cells +0.5%.

**Phase K: recording and viewer**

- K3, viewer (part without the globe): the charts are in sections by name (climate, air gases, minerals, births and
  deaths, brains, movement, parts, catalysts, one section a starting kind) with a search box; a "Paths" toggle draws
  the `worn` layer over any map view.

**Phase K: recording and viewer**

- K1, the tape and every reader take `&World`: the CLI drains the events and the yearly counts
  (`World::drain_events`, `World::take_counters`) and hands them to the recorder. The same world, `series.csv` and
  `months.csv` byte for byte on a short run; golden unchanged.
- K4, time per process: `series.csv` gets `ms_flow`, `ms_change`, `ms_bodies`, `ms_parts` and `ms_tape` (ms a year),
  measured in the CLI and handed to the recorder; the viewer groups them in "Time per process". The world is unchanged.

**Phase G: the globe**

- The start drains (with G1): every land cell has a way down to the sea from the start (`worldgen::terrain::drain`, a
  priority flood from the largest body below 0 m, 1 m per km across a filled floor); the start's air column per node
  from the mesh's areas. 64 x 64, seed 1, with life: closed basins 25 (138 cells, room for 122 km³, the deepest 931 m)
  to none (seeds 2 and 3: 16 and 19 to none); liquid on land 8.3 / 16.0 / 27.3 / 40.7 km³ at years 5 / 10 / 20 / 40
  to 2.9 / 3.2 / 2.6 / 2.2 (level from year 5); river cells at year 40 830 to 949; rain on land 1.81e-3 to 1.73e-3
  mol/m²/s. Key numbers (3 seeds) all in range: land 287.94 K, swing 12.25 K, rain 1.76e-3, absorbing gas 246 ppm,
  river cells 878, land worn 0.082 mm/yr, light-eaters 10.66, eaters 4.30, lives 2.58 and 4.04 yr. Golden re-pinned.

- G1, the lat-lon grid: a globe (`ew run --globe`, `--size` its rows, twice as many columns) of squares of latitude
  and longitude that wrap east to west, on a sphere of 6,371 km; each row's area and borders from its latitude, no
  link across a pole; latitude and longitude per cell, the way between centres on the sphere (`Level::offset`) for the
  wind's slope of warmth; the air a coarser level that wraps; the four air nodes a cell sees wrap; the banded solve
  takes the wrap as it is (the closing link lies inside the band). worldgen samples its noise on the sphere and sets
  the level so 29% of the area is land. Bodies step round the wrap on the square map (`Square::step`) until G2.
  The island is the same world bit for bit (checked with the new flag left out of the fingerprint); golden re-pinned
  for the flag the saved map now carries. 64 x 32 globe without life, 10 years: drift 5e-16 (elements) and 2e-15
  (energy), 1.15 s a year.
- G1, the sun: the declination is that of a circular orbit, sin δ = sin ε sin λ, not δ = ε sin λ, which made the
  yearly sun at the poles 0.9% too strong. Key numbers (3 seeds): land 287.94 K (+0.01), swing 12.26 K, rain on land
  1.764e-3, the same to noise. Golden re-pinned.
- The start drains to every cell below 0 m, not only to the largest low body: on a globe that raised the other low
  bodies to land (43% land against the 29% set). The island keeps its 1 to 3 low inland cells a seed as basins full
  to 0 m. 64 x 64, seed 1: liquid on land 3.2 / 4.9 / 4.8 / 4.6 km³ at years 5 / 10 / 20 / 40 (level from year 10),
  river cells at year 40 924. Key numbers (3 seeds) in range: land 287.97 K, swing 11.84 K, rain 1.80e-3, absorbing
  gas 249 ppm, river cells 836, land worn 0.080 mm/yr. Golden re-pinned (seed 1).
- The air of a globe in a folded order (decision 19): each row of air nodes is numbered 0, w-1, 1, w-2, ..., so the
  cells at the wrap read air nodes within a row and the coupled solve's band is one row wide, not two. The island is
  the same bit for bit. 64 x 32 globe without life, 6 years: 8% fewer instructions; ground by latitude band the same
  within 0.04 K, rain within 0.5%.
- G3, measured and not built (decision 20): a cell under deep standing liquid costs a land cell's Flow (its surface
  row 0.24 to 0.31 µs a round, land 0.32 to 0.38) and a third of its Change (it sleeps); on a globe the air's 13
  rounds a step and its level's solve are most of a cell's time. Tried and not kept: rows of cells whose air moved
  less than the rounds' tolerance kept between rounds (3% fewer instructions on the globe); waking a node only by a
  substance a reaction takes (no effect); `air_scale` 2 on the 64 x 32 globe (Flow 3.3 times dearer, a different
  climate).

**Owner's decisions after Phase A**

- A4 (a), the start: the light-eater founders' neutral brains keep the hand-set bias -4 of taking from a body
  before the spray (spread 0.9), so almost none starts with a bias to bite. 64 x 64, seeds 1 and 2: bites fall from
  8% to 0.2% of plant deaths in years 3 to 21; plant members at year 10 rise 1.9 to 2.5 times, at year 30 by 6 to
  19%; golden re-pinned.
- A4 (a), the law: a bite of one member's body or more kills members by its weight (the mass taken over the body,
  or the share of body matter eaten if more), not by the largest share it takes of any store; what it takes of a
  store beyond the dead members' share comes from the members who live. 64 x 64, seeds 1 and 2, with the start
  above: bites fall from 0.2% to 0.004% of plant deaths in years 3 to 21; plant members at year 30 +7% (seed 1)
  and -17% (seed 2); golden re-pinned.
