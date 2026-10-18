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
