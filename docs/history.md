# History

What changed and when, one line per change, grouped by phase. The laws as they are now are in `docs/laws.md`; the
plan is in `docs/handover.md` and `docs/foundation/plan.md`. The long reasons and the full numbers are in the commit
messages. Commit `154554a` is the last state before the M3 clean-up: the old handover (397 KB, with every result of
reviews 2 to 8), all old reports and patches are in it (`git show 154554a:<path>`).

## 1. M0 to M3, day by day

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

**13 October 2026: reviews 2 and 3**

- Nodes keep a spread, split when too varied, act on bonds through one member: `2e2d56d`.
- A body holds a part only while its own force bears it: `0778c27`.
- Reaction rate first order in the scarce input: `22c291f`.
- Snapshot buffer, kinds by diet, the null run: `d5479cf`.
- Catalysts cost by capacity and unfold when hot; charcoal no longer burns cold: `c57f7d8`.
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
- Per-kind tape columns and the land's season: `023717c`, `8f654c8`, `4ba6b7f`.
- Breathing through the open surface; intake with mass^0.75: `34a7bfe`.
- A node can split along its brain-size genes: `475ad38`.
- One reference mass; build rate; basal upkeep; leaf with mass^0.75: `851ac26`.
- Every number gets a row in `docs/numbers.md`, held by a test: `0546a17`.
- Individuals mode (`--individuals`) and `copy::open`: `33ef14a`.
- Likeness counts brain sizes by whole units: `d902f24`.
- A fair big-brain test (`--clever-year`): `551d0d4`.
- Nodes mix only with their own kind (`WITHIN` 2 → 0.5): `863ceab`.
- Hand-set eater brains at the start (neutral founders were a lottery): `03a5d01`, `07bc5a3`.
- Body matter: growth is building, storing fuel is cheap; digestion; stores bounded by the body: `afdf3fc`, `c19a846`, `02ee75c`, `a15251a`.
- Bodies copy by physics only: `1d5358c`.
- Digested matter fills the room of what it becomes; a fuel's minimum is a reserve: `ccfbde9`.
- Activity needs liquid (dormancy; `WET` gene): `cab28ea`.
- Reproduction by surplus (maturity, `ALLOC`, brood, `CHILD`); size and maturity classes in nodes: `53586fe`.
- Body size by an energy ceiling (tissue costs by the kg of the body): `d2374d0`.

**15 October 2026: the queued mechanics, carbon, soil**

- Supply and wear go by the built body too: `f02d4c2`.
- Partial damage and healing: `d143782`.
- Upkeep, intake and building follow temperature; work heat stays in the body: `8074ad6`.
- Two-parent copying: `47bae74`.
- Dead plant matter rots at a litter constant: `77c3ca7`.
- The sea's carbon store (salt 23, reactions 22 and 23), at balance at the start: `6048a5f`.
- Soil water (pores, Darcy's law): `32d2817`.
- Soil nitrogen with fixing founders: `8bdfbcc`, `03123d8`.
- The rock nutrient; erosion at real rates: `ba9ec80`.
- Law 2 by mass action with the contact of the phases (no jump): `b147ca2`.
- Real figures in `numbers.md` checked against sources: `81ad832`.
- Nitrogen fixing pays its real cost: `d9af1d7`.
- `c_build` anchored to real production (P/B): `bdab246`.
- Moving needs force: `a6d7926`.
- Drainage into a store in the bed: `de1c134`.
- Kleiber across kinds, adult founders, the living body at 75% water: `1c7e5d1`.
- Law 2's step follows the rate law; gifts never give body matter: `5282e76`.
- Two simplify passes on core: `f2d6791`, `5413336`.

**16 October 2026: body physics**

- A body keeps a temperature (`KEEP_T`): `0fb9f3e`.
- End-product inhibition of catalysts: `12a8bf1`.
- Drainage to channels; wet ground evaporates at the open rate; the sea floor reacts with the dissolved gas: `5de4eed`.
- Liquid past the level drains out; rain caught by leaf area: `cc86483`.
- Bodies of several materials; height by self-buckling; breaking by strength: `9967317`, `6aa6ccd`.
- One heat network; cooling by evaporating (`COOL_T`); roots; air films by wind and size; leaf pores: `4a3c4fc`.
- The heat a body draws from the ground is set aside as it flows: `30737df`.
- One freezing law for bodies, ground, lakes and the sea: `154554a`.

## 2. M3 clean-up (16–17 October 2026)

**The clean-up (results unchanged)**

- One folder per law: `cf9b151`, `37c9518`, `30f4012`, `cf8138c`, `8464bd8`, `026c8ea`, `8c89a6c`; `lib.rs` split
  `4fc2d95`; tape, cli and worldgen in folders (merge `4c65ce0`).
- Backwards compatibility removed; a snapshot has a format header: `797b1a1`.
- Switches `split_rows` and `sate_all` removed `fab9e96`; dead code removed `a8b64d5`.
- Derivable tape columns and their counters deleted (`84853d3`, `3a8dcb9`); counters in `World::counts` (`32884a5`).
- Near-duplicates made one helper each (`3100d49`, `83e7e64`); one purpose per random draw (`80dec58`).
- Tests: one file per law with `worldgen::testkit` (`4639b6b`, `12afe06`), overlaps removed (`ef7615a`), slow tests
  `#[ignore]` (`3e54e9f`), the golden test (`968ae5e`, `902183d`).
- The docs rewritten: the handover is the plan; `laws.md`, `history.md`, `tape.md` added (merge `3269b05`).

**Bugs fixed (each changes results)**

- A child of less than one member counted as a whole member: animals about 3 times too many (`d817aa9`).
- A member going back to its node shrank the node's row spread (`57e2965`).
- In `--individuals`, siblings shared one set of born rows (`06d09d5`).
- Brain groups weighed bodies, not members; a new group counted 0 until the next month (`b3d2c0d`).
- One error too big for a number stopped a group's learning for good (`3f93399`).
- A null run still drew new brain weights (`e900262`); the copy action left the brain (`43efbc5`).
- Children who stayed got provisions already cut by those who left; a birth was recorded in the parent's cell (`fbf5d54`).
- Gas entering the air did not pay its lift, its fall was paid: 3.5e16 J a year from nothing at 48 x 48 (`ac0c48b`).
- Two ground temperatures; taking from the ground read a survey weeks old (`064924d`, `628bd6c`).
- Liquid waited for the wind's turn when their periods differed (`9ba34e9`).
- The bed's stores flowed several cells a step toward +x and +y (`1be7b83`).
- The yearly sideways heat of the ground did almost nothing (1e-5 a year): removed (`5f7b59a`).
- Flow: the wind made matter when it clamped, molten moves left fusion heat stale, pore liquid under field capacity
  flowed (`f3e10b0`); the molten bit set held only 53 substances (`306f41f`); drainage could overdraw (`7875ee9`).
- Two land definitions in worldgen; an empty kinds row; the tape's yearly state lost at a resume; groups weighed by
  bodies (`a92e71e`, `92d3748`, `c71d668`, `c548616`).
- A ground node on a melting plateau reacted up to 80 K too cold (`cd363b9`).
- Formation enthalpies were the measured ones, not the solid's: burning gave 451 kJ, not 467.5 (`71eca3f`).
- Parts settled gas without its heat or lift; parts used the plain melting point (`f34949c`, `87f6296`).
- A molar mass was not the sum of its elements; molar masses now come from element masses (`1f2c839`, `3c1098f`).
- The reactor froze reactions over a calendar year, not the world's (`f75aa55`).
- A node that lost members lost more brood than matter: deaths of want 6 times too many (`37e98f0`).
- Two rules for what a body burns (`63a3c60`).
- A dormancy of 1 gave NaN; bad time steps not refused in core (`cd9b219`).
- `pay::plan` failed past 64 burning reactions (`0066c2b`).
- A body's molten stores held no fusion heat: frost killed 10 times too many (`8ff6e90`).
- The light share a light reaction stores moved to `Physics` (`c9294c9`).
- The starting air thinned by a fixed scale, not its own (`9c1c252`).
- Tools: resume placed no kinds still due; `root_<k>_life` divided by the whole mass; the viewer drew missing as 0;
  the tests' start was not `ew run`'s; bad steps gave no clear error (`0bf0414`, `25eca36`, `5bda05f`, `9417861`,
  `3bee6fc`).

## 3. Tried and not kept in M0 to M3

Old patches are under `154554a:docs/reports/`.

- Built mass as a body's size: every animal died (`watch-7-before-m4/built-mass-not-merged.patch`).
- First nitrogen build (one gift of the mineral): it collapsed (`nitrogen-not-merged.patch`, `d3819ac`).
- Review 7 and review 8 prototypes; the fixes were rewritten and landed (`review-7-*.patch`, `review-8-carbon-*.patch`).
- Law 2 smooth rate, the study's code; the law was rebuilt in `b147ca2` (`law2-smooth-rate-not-merged.patch`).
- Eater density probe, a diagnostic (`eater-density-probe.patch`).
- Cold-blooded resting rate (tissue costs × 0.068): plants walked (`c8a56ad`, reverted `f3d3650`).
- Catalyst cost by the square of the genes: replaced by cost by capacity (`c57f7d8`).
- First-order rate law: it jumped when the scarce input changed (`22c291f`, replaced by `b147ca2`).
- Catalyst costs by mass^0.75 (`a3d31a0`, replaced by `d2374d0`).
- Satiety at every size: killed every animal twice (`e9705b1`); satiety only past `full` (`a15251a`).
- Copying when full at 2 × the copy mass: replaced by reproduction by surplus (`2b6c8a9`).
- The child store `Body::paid` (`9089704`, removed `c19a846`).
- Copying by the brain's choice (off since `1d5358c`, gone `43efbc5`).
- Splitting nodes along brain rows (`split_rows`, removed `fab9e96`).
- Draining liquid past the level, two tries (`12aaf60`/`915b051`, `e13d911`/`e9176f7`); landed as `0639392`.
- Damage by the weight of the bite in nodes (`5679524`).
- Frame enzyme in the founders: eaters of remains 10,000 times fewer (removed `7953f69`).
- Freezing of bodies only (`a013312`); the general law landed in `154554a`.
- Recorded in the old handover, Section 4: the textbook coat, one temperature level instead of `KEEP_T` and
  `COOL_T`, linear product inhibition, lower salt melting points, the film with only the dissolved stock, `c_build`
  at 3 × 10⁴.

The deleted reports (watch 1 to 8 before M4, reviews 2 to 8, and the studies of carbon, soil, rock nutrient, Law 2,
big animals, eater density and frost) are all in `154554a:docs/reports/`. The watch reports of M0, M1, M2 and the M3
check point were never in the repository.

## 4. The foundation (from 17 October 2026)

The work of `docs/foundation/plan.md`, from `43efbc5` (about 15 s a simulated year with life at 128 x 128, a body
update about 36 µs, bodies about 80% of the time). Unless a line says otherwise, numbers are from a 64 x 64 island,
seed 1, against the commit before. "Same" means the golden fingerprints did not change; every other change re-pins
the golden test.

**Phase A: prepare**

- A1: `perf` on 64 x 64 with life (`docs/reports/foundation-a.md`): bodies about 70% of a year; group brain sums 15%,
  reactions 3%.
- A2: reference runs of `5671305` in `docs/reports/foundation-a/` (64 x 64, 60 years, seeds 1-3, normal and roster;
  128 x 128, 300 years, seeds 1, 5, 7).
- A2 fix: a drained bed store fell below 1e-308 and the liquid's share divided by it (NaN in year 285 at 128 x 128);
  each outlet now takes q / (sum of the flows).
- A3: the key-number test `tape/tests/ranges.rs` (ten numbers, seeds 1-3, 30 years); series gain `t_land`,
  `rain_land`, `rivers`.
- A6: the code-size test `cli/tests/core_size.rs` (7,508 code lines at its start), in place of `wc -l`.
- A4 (a): plant members died "taken" before any animal: light-eater nodes took from each other. Owner's decisions:
  the founders keep the hand-set bias -4 against taking from a body (bites 8% to 0.2% of plant deaths); a bite of a
  member's body or more kills by its weight, not by the largest share of a store (0.2% to 0.004%).
- A4 (b): the wind solves its lines in both orders and keeps the mean (rain map stripes gone; land rain +0.7 to +2.5%).

**Phases B and C: the base layers and the shared maths**

- B1, B2: `core/src/space/`: a `Mesh` of levels, the square builder, stacks; `Grid` a thin view on it. Same.
- B3, B5: the store `core/src/store/` and `store::apply` (moves, transforms, link fluxes). Same.
- B7: the clock: time in whole seconds, the calendar derived, processes with periods. Same history.
- B6: the `Ledger` goes: sun, space and outside are boundary nodes; `check::drift` reads them. Same history.
- B8: `clock::Sleep` with node alarms and the speed setting `wake_share`. Not used yet.
- B9: `Rng::of_node`, a second key for new draws. Same.
- C1: `solve::spread` (backward Euler on links, conjugate gradient, returns link fluxes). C2: `solve::downhill`. C3:
  the wind's tridiagonal solve in `solve::lines`. C4: big-step against small-step tests. Same.
- B4 measured, not built: the amount arrays are about 5% of a world with life (brain rows 63%); kept dense.

**Phase D: the physics laws on the new base**

- D4: radiation reads latitude and area from the mesh. Same.
- D8: cleared ground is the first link field (`Links::cleared`, was `World::pass`). Same history.
- D0: `store::apply` touches only what changes: heat on 128 x 128's links 6.1 to 0.4 ms. Same.
- D1: ground heat conducts sideways once a year by `solve::spread`.
- D7: parts are store nodes; bonds one link list; the inside of a structure one link (`bond::Enclosure`).
- D3 (a): the wind turns by the latitude of each edge: land -0.03 K.
- D3 (b): the air on the mesh (a cell's share of its air node by area; the wind on air links). Within 5e-11 without life.
- A link flux takes only a node's net loss and may join two classes. Same.
- D2: surface liquid by `downhill`, pore liquid and deep store by `spread`; the sorted sweep and `pore_flow` go.
- D5: erosion at every liquid step (`WORK_MONTHS` goes): the test island 0.138 to 0.133 mm a year.
- `store::apply` books first and checks after: Flow 15% faster at 128 x 128 without life. Same.
- D3 (c): the wind books its crossings over the whole solve: land -0.24 K, rain on land -1.1%.
- D6: Change's nodes sleep by `clock::Sleep` (`change/screen.rs` goes; `sleep_days` 90): carbon gas 252.2 to 248.4 ppm.
- D9 (Change): one transform per node through `store::apply`; due ground nodes react from the same air: carbon gas
  272.6 to 268.8 ppm, plant members +12%.
- An air node reacts with no liquid film (it used another node's): carbon gas 268.8 to 271.7 ppm.
- D9 (Flow, Bond): radiation, the exchange and creep book link fluxes; the potential energy is a node
  (`Class::Potential`); `take_heat` goes; test `cli/tests/contract.rs`. Land -0.01 K.
- A link flux wakes a node by its net change in the call. `store::Batch` gathers fluxes link by link. Same.
- D9 (end): `grid.rs` deleted; the laws read the mesh. Same.
- D10, speed, all same: `apply` keeps one copy per span (Flow -15% instructions); per-node alarms (Change 9% faster);
  `add_elems` by held elements; the reactor skips reactions without their inputs (Change -13% instructions).
- The wind skips a direction with no air link: a 4 x 4 world panicked; CI was red while the normal suite passed.
- Phase D check (`docs/reports/foundation-d.md`): key numbers in range; Flow and Change not 2 times faster; core not
  600 lines smaller.

**Before Phase E: an outside review of Phase D**

- The plan follows the review: the store only adds (D9), wake by gain (D12), level 2 for speed work (D10), Phase E by
  the profile, no sparse amounts, no graphics card, the 8,000-line ceiling at the end of the foundation.
- D11 (a): a law that could overdraw a node now stops itself (`spread::within`; reactions' gas paid by heat). Same.
- D11 (b), D12: `store::apply` only adds and counts rounding; its scaling and restores go (`store/` 686 to 349 lines);
  a node wakes by its gain since it slept. Air nodes due a year 119 to 100.
- D13: work spaces kept between steps (same); saturation tables built once on a fixed grid (series within 4e-4).
- D12 test: `wake_share` × 2 and ÷ 2 against every node awake: chemistry within its noise.
- D11, speed: `apply` books column by column: Flow -1.8%, Change -4.6% instructions.

**Phase E: the life engine**

- The plan follows an architecture review and the owner: the time budget (D12), the node pace (D13), what waits
  (D14), the lat-lon globe (D3), E5 to E8 as now, the cost model (5.7).
- E1 (a-f), the bookkeeping of a body update: group sums settled when read; height a visible trait; one temperature
  solve per stage (31 to 15 an update); mineral list made once; `pay::Payer`; clustering stops early (89 to 44 ms a
  year). A body update about 15% cheaper (42.5 to 35.5 µs, year 20).
- E1 (g, h), fixes: a placed body is seen as what it holds; the decision sees the body after it sheds.
- E8: Flow's parts are processes of the world's clock (Flow's timers go); rivers +2%.
- E8: state apart from cache: Flow's scratch and the survey save only state. Same history; snapshot 44.0 to 42.9 MB.
- E8: fixed nodes by class and index (`store::Node`); the node table goes.
- A resumed tape reads columns not yet recorded as NaN, not 0. No world change.
- The owner widens two ranges: eaters' mass ± 0.5 log10 (own rows: 4.88 against 4.497), light-eaters' life ×/÷ 3
  (no brain: 4.07 against 1.895 yr).
- An ablation hunt (two seeds, 32 and 60 years) sets E10's culls and E6's aim; Bond's joins, never seen in a run, stay.
- E9: light-eaters start without a brain (`HIDDEN`, `MEMORY` 0; `H_LIGHT` and `--plant-bias` go): plant life 2.40 to
  4.01 yr, members +62%, members split off 238,734 to 488, a plant update 72 to 35 µs. A body without a brain has no
  born rows and no group: born-row sets 5,455 to 134, snapshot 12.9 to 10.6 MB.
- E7 dropped: after E9 only about 500 members split off in 30 years.
- The wind's mesh solver goes; a row round the globe by `solve::lines::along_ring`. Same.
- Fixes of the update span: the key liquid soaks again after growth (`soak_again`); age deaths 1 - e^-wear. Plant
  activity 0.50 to 0.75, plant members -22%.
- The cover of other bodies is their mass now (`cover_of`): plant members -8%.
- E4: a node without a brain updates at most every `node_days` (36; a prototype had given light-eaters 3.5, 5.5 and 8.0
  yr at 18, 36 and 72 days); rules per interval become rates: body updates
  97,200 to 54,200 a year, bodies 1.6 times cheaper; a pace error past noise left (plant life +14 to +24% at 36 days).
- E10 (c): old experiment flags go (`--clever-year`, `--spray-*`, `--regrow`).
- E10 (d): light shared again only in cells whose bodies changed (`Life::stir`): bodies 3 to 8% faster. Same.
- E10 (e): the temperature solve passed on (12.6 to 7.6 an update): a body update 4 to 13% cheaper.
- E10 (b): the `FORGET` gene goes (never on); 21 scalar genes.
- E6 (F): a node's children have one parent. E6 (A, with F1): children who join a node are booked on it (`nodes::Lot`,
  `nodes::add`); same world, not cheaper per call. E6 (D): leaving children need not make a whole member.
- E10 (a): shared group brains go; each body keeps its rows: tuned 61 to 58, core 8,652 to 8,414; eaters 1.6 and 2.0
  times heavier (each body learns at its full rate).
- E10 (f): `Body::k` is rebuilt at each gene change and not saved; dead slots keep nothing: snapshot 14.7 to 10.0 MB.
- E11: an adult and a young node per kind and cell; the young mature across the copy mass (`copy::mature`); size
  classes go. Plant life 2.83 to 3.34 yr; young still 92% of plant members.
- E11, pace made flat: joiners average the node's last day and light mark: plant life error at 72 days -14% to +2.5%.
- E12: selection inside a node by a sample member one spread off on one gene (Price, the breeder's equation): repair
  gene -0.97 to -0.87 in 30 years; samples 35% of the bodies' time.
- E12, speed: no sample on genes the update cannot read (`select::read`), then one update in `sample_every` (2): the
  samples' share 35% to 12%.
- E11: the young keep their mean log size (`size_lag`): variances had run to 1e16 and NaN; wide nodes 336 to 10.
  Tails taken from their logs: the century test went NaN at year 46.
- E5 (2): one bodies' node per cell (`Class::Bodies`); the light and heat pools go. Energy drift 1.6e-14 at year 30.
- E5 (1): bodies and parts book through moves (`store::Moves`); `ground::draw`, `take_paid`, `flow::Sky` go. Same
  world to the bit.
- E5 (3): a private `store::Store`; the bed a class (`Move::loosen`); `cli/tests/contract.rs` goes (the compiler checks).

**Phase E review, round 1**

- Merged pairs of body functions; `Body::prepare` reuses its vectors. Same world to the bit.
- One 10-day step for physics and air (`phys_days` 10, `air_hours` 240), the air booked whole: Change 787 to 374, Flow
  2,055 to 1,816 ms a year; the run 130 to 109 s.
- The air solves only gases above rounding or that can evaporate: Flow 1,811 to 1,410 ms a year at year 30.
- No silent cuts of a body's stores (`body::add_store`, `Counters::rounded`); upkeep's gas through `Moves::gas`.
- `ew resume` into a new folder: the survey's heights are worked out when the cache is empty.
- No take cap per update (`MAX_TAKE_SHARE` goes): plant life 4.63 to 3.16 yr.
- `flow::step` moves to `testkit::flow_step`.
- Selection sees reserves (`copy::buildable`): carbon gas 260.5 to 236.3 ppm.
- Selection skips samples that cannot reach a threshold: 7,600 samples a year against 14,400; `sample_every` stays 2.
- Bodies catch the rain fallen since their last update (`World::rained`, `Body::rained`).
- `flow_days`, `crowd`, `MIN_INTERVAL` marked speed; `TUNED_BUDGET` 57 to 52.
- Phase E in all: E10 removed about 226 code lines and 3 tuned numbers; E5 added 30. Cost model after the 10-day
  steps: a cell 0.48 ms and a body node 360 µs per simulated year.
- `docs/laws.md` 1,117 to 776 lines.
- Diet of core, results unchanged: 8,998 to 8,587 code lines.

**Phase F: population nodes**

- F7: nodes against individuals as a slow test (`tape/tests/nodes.rs`). It fails: seed 1's eaters 24 times heavier
  as nodes, seed 5's 7 times lighter; roster lives 2 to 2.7 times shorter as nodes.
- F7 fix: the brood rots with the body matter it is part of (`Body::brood_share`): individuals shrank 13% more.
- F7 fix: a node holds half a child of brood per member (`copy::BROOD_HELD`): nodes made 1.4 times the children per kg.
- F7 fix: maturing by the steady state of a stage (`math::Pass`); the normal-cut code goes.
- F7 fix: a starving body dies with the chance of the share it lost (`starved`): plant life 4.54 to 3.26 yr.
- F7 fix: one rule of death for nodes and individuals (`lose_share`; `Body::wear` goes): differences 17 to 10.
- F7 fix: a partner is one parent by even chances (`copy::mix`): differences 10 to 9.
- F7 fix: choices as a spread of actions (`choose/spread.rs`); a node learns from one member drawn.
- F7 fix: predators reach node prey as individuals (`take_body`, `wound`): eating other bodies 1.6 against 306 kg a
  year to 68 against 373.
- F7 fix: a node is of one lineage (decision 23): eaters' copy-mass gene 1.33 to 1.65 (individuals 1.63).
- F3 (decision D8): unlike nodes never merge; `crowd` stays a speed setting.
- Intake from the ground by Holling's type II (Holling 1959): booms and busts either way.
- Eater founders keep 278 K (`EATER_KEEP_T`): eaters at year 60 (48 x 48, seeds 1-3) 27, 1171, none to 789, 3046,
  none.
- `WITHIN` marked a speed setting; tuned 52 to 51.
- F7 holds the rules that pass and prints the rest (decision 25). A sample without the decision for nodes with a brain
  drove a boom: not kept.
- A node's part does Bond's actions as its members together (`nodes::one` goes); core 9,027 to 8,996.
- Key numbers: eaters 3.79 log10 kg, out of range (decision 26).
- F7 after J3 and G3: 15 differences; eaters 3.66 log10 kg, still out; the rest in range.
- Evaporation pays its latent heat only with heat that is there (`physics::heat_bound`): drift had reached 1.8e-7.
- F1, F2 built with E6; F5 open.

**Phase J: the long physics step**

- J1, what held the step at 3 days (no life, steps 6 h to 30 days): rain on land 3.9e-3 at 6 h, 1.17e-3 at 3 days;
  the explicit air temperature, `EVEN`, `OVER` and the exchange's caps each bound per call.
- J1: liquid every 30 days (`flow_days` 30): Flow 25% cheaper, the world the same.
- J1: the air in one backward-Euler step (Newton on `solve::band`): within noise of 6-hour steps; `mix` 1e6 m²/s,
  new tuned `rain_time` 1 day; rain on land without life 1.15e-3 to 1.58e-3.
- J1, speed: `exchange::bucket_slope`: Flow -1%.
- J1: climate references from the converged world: land 289.31 to 288.45 K, swing 10.19 to 12.68 K.
- J2: Flow and Change keep their threads by rows on the globe: same fingerprints.
- J3: exact slopes, the wind and clouds from the step's start (decision 21): globe rounds 13.2 to 6.4.
- J3: rain out of the four air nodes a ground node sees (`air::shares`): globe rounds 6.4 to 3.6.
- J3 in all (64 x 32 globe, no life): Flow 682 to 285 ms a year; a cell's Flow and Change 0.37 to 0.17 ms a year.
- J3: the bodies' node settles with its ground in the air's step (`air::settle`): the ground had sawed 1.5 K.
- J3: bodies' gas waits in the bodies' node for the air's step; the air counts the heat capacity of that gas: land at
  10-day steps 289.53 to 288.82 K.
- J3: the long-step test compares each seed with itself (32 x 32, 20 years); it found a 1 K error, now 0.1 K.
- J3: 10-day steps within 0.15 K of 3-day ones, 30-day 0.2 to 0.3 K colder; the default stays 10 days (decision 22).
- Key-number references set again (decision 27): land 289.50 K, rain on land 1.798e-3, eaters 4.497 to 3.943.

**Phase K: recording and viewer**

- K1: readers take `&World` (`World::drain_events`, `World::take_counters`). Same.
- K2, K3: viewer file `EWF2` with the globe; yearly means weighted by cell area; the globe as a 2:1 map.
- K3: charts in sections with a search box; a "Paths" toggle.
- K4: time per process in `series.csv` (`ms_flow`, `ms_change`, `ms_bodies`, `ms_parts`, `ms_tape`).

**Phase G: the globe**

- The start drains (priority flood): closed basins 25 to none; liquid on land at year 40 40.7 to 2.2 km³.
- G1: the lat-lon globe (`ew run --globe`): squares that wrap, a sphere of 6,371 km, 29% land; 1.15 s a year at
  64 x 32 without life.
- G1: the sun's declination of a circular orbit (sin δ = sin ε sin λ): the poles had 0.9% too much sun.
- The start drains to every cell below 0 m (the globe had 43% land).
- The air of a globe in a folded order (decision 19): 8% fewer instructions.
- G3 measured, not built (decision 20): a sea cell costs a land cell's Flow and a third of its Change.
- G2: life steps on the mesh's links (`Level::way`; `Square::step` goes). The island the same to the bit.
- G4: a refined region (`ew run --region`, decision 30): 32 x 32 with an 8 x 8 region 1.02 s a year (0.74).
- Creep crosses a border as wide as the smaller cell.
- G5: coarse against fine (`tape/tests/scale.rs`): within noise but river length -171 km (decision 31).
- G6 dropped (decision 28).

**After the round-2 review**

- Finding 3: `World::a_seen` kept in the update's moves: 8.7% fewer instructions in a living year.
- Finding 14: test-only functions leave core; test files split.
- Finding 11: `math::Pass` fits by Newton; `Chemistry::burners`, `contact_rates`: 2.1% fewer instructions.
- Finding 10: hidden caps go where the law limits; `Physics::lethal` 0.5 (anchored); `keep_min` 2 (tuned, budget 50).
- Finding 13: selection's skips and `sample_every` stay (decision 4): samples 3.5% of a living year.
- Finding 5: a body floats by its density (`bond::afloat`); eater founders sweat from 315 K; eaters still die out
  starving (decision 32).
- The bodies' heat from the ground is implicit in the air's step (`survey::Draw`): sawing canopies 32 of 47 to 5 of 70.
- The air's rounds solve only condensable gases above a trace (`newton::alone`): rounds 3.7 to 2.2, Flow 1,130 to 850
  ms a year.
- Change's reactor runs on threads (`par::rows`): 270 to 220 ms a year on 4 shared cores.
- Ground with no pores counts as full (`land::filled`): a NaN; no result moves.
- Clouds absorb heat radiation (`cloud_ir`, decision 34): globe land 221.7 to 284.4 K, ice 0.95 to 0.12; the island
  at 40° (anchored, was 31° tuned).
- The air's step holds a ground node at its melting point while the fusion heat lasts.
- Upkeep and supply follow the release's temperature factor (`Body::rate_at`, decision 36): eaters at year 35 0, 0, 0
  to 26, 53, 12.
- One standing depth (`FlowView::standing`): land 287.15 to 287.72 K.
- Eater founders take from the ground first; a move weighs standing liquid by -1 (`choose::WET`, decision 37):
  first-year deaths of unpaid upkeep 675 to 410.
- Air nodes start at the temperature of their mean height: land 287.7 to 286.7 K.
- The air's step takes the surface's reflectance at the step's end: land 286.7 to 285.6 K; Flow 17% slower.
- Climate references set again: land 287.83 K, swing 13.61 K, rivers 136.1.
- Costs anchored to a resting mammal are brought from 310 K to `t_cat` (decision 38): `c_cat` 2.6306e10 to 5.483e9;
  eaters at year 30 4.63 to 5.20 log10 kg.
- Surface liquid follows the spill levels of a priority flood (`downhill::Order`, decision 31): G5's river length
  -11.5 to -9.0 km.
- `take`, an intake rate of warm animals, is brought from 310 K to `t_cat` (decision 39): 0.05 to 0.01042; eaters at
  year 30 5.20 log10 kg, at year 60 (48 x 48) 4.3e3, 2.6e5, 2.1e4 kg per km² of land.
- The island's start fills its inland basins below 0 m (`terrain::drain`, decision 17): G5's river length -9.0 to
  -7.9 km; land 285.6 to 285.2 K.

**After the round-3 review**

- Finding 4: `load_snapshot` rebuilds each cell's sums (`survey::sums`): a resume after paths exist panicked.
- Finding 2b: `ew` runs Flow and Change on threads by default (`--no-default-features` without; wasm without).
- Finding 7: liquid with no load does work on its bed at any capacity (no 1 m³ threshold): at year 30 (48 x 48, seed 2)
  eaters 1.25e5 to 1.07e5 kg, light-eaters 1.83e10 kg (the same), land 289.50 to 289.48 K.
- Finding 7: `grab`, `lever_ref`, `hard_min` become fields, `WITHIN` tuned, `MAX_KEEP` a row (decision 45); tuned
  budget 50 to 54; the same series.
- Surface liquid takes its steepest way down by D-infinity (`Level::steepest`, decision 46), `split` removed (tuned
  54 to 53): G5's river length -7.7 to -5.2 km (limit 6.7 to 4.0); 48 x 48 seed 1, years 21 to 30: rivers 27.2 to
  27.8, land 284.5 K both.
- Finding 3: nodes merge by the root of the structure they are in or hold, and bodies of one member join a node
  (`nodes::Kin`): 48 x 48 seed 2 at year 60, most bodies in a cell 407 to 134, animal bodies 2,428 to 1,905.
- Finding 1: one intake law for the ground and bodies (`feed::intake`); the free sugar rots 200 times faster in the
  open world (`Reaction::open`); eater founders bite first again (decision 47): 48 x 48, year 60, free sugar on land
  2.1 to 0.02 kg/m², eaters 1.6e4, 2.4e5, 1.4e4 to 4.6e3, 2.7e4, 1.6e4 kg/km² of land, land +0.3 to +0.8 K.
- Docs: `laws.md` says each law once (rule, reason, files), the code's doc comments say how: 925 to 454 lines.
- Docs: the plan's finished phases A to K in short; E6, F7, G5 and the speed goals marked open with their numbers; E2,
  E3 dropped; the cost model of round 3; round 3's open items in Phase L.
- Docs: `decisions.md` without run results (every row and number kept; the runs are in this file and the commits).
- Docs: handover sentences made true: the golden re-pin rule, the 8,000 lines back with the owner (decision 42).
- The review report of M0 to M3 leaves `docs/reports/` (only watch reports stay there).

**Tried and not kept in the foundation**

- Nodes keeping their members' wear: eaters' life 7.1 yr against at most 5.8 (branch `nodes-wear-tried`).
- The wind on the mesh solver: five times dearer; forced, rain on land -28%, rivers 503 to 104.
- E12: all genes at once with one sample: genes that do nothing drifted by chance, a run failed.
- E12: drawing only among the genes read: as many samples, nothing saved.
- J1: a first guess from the step before; refined factors; air layers out of the inner loop: no gain.
- J3: a second-order air step (BDF2): unstable in year 4.
- J3: a 30-day physics step: land 288.06 to 289.30 K, past noise; a 15-day step: 289.69 K, out of range.
- J3: the cell pass on more threads, and slopes summed one slice per pair: no gain.
- G3: rows skipped between rounds (3%); waking by reaction inputs (none); `air_scale` 2 (Flow 3.3 times dearer, mid
  latitudes 1.4 K colder, polar rain 20 times more).
- Finding 7: `node_days` 72: fails the pace test at 144 days (births -55% against 26% noise).
- Liquid routed without what still passes: seas drained into piles (decision 31).
- R3 finding 2d: growth's payer kept across its three conversions: most repeats follow a conversion that spent, so
  only 0.45% fewer instructions in a living year, for 5 code lines.
- R3 finding 2c: a smooth draw of the bodies' heat: not built; a cell with bodies runs 3.0 surface solves a round
  against 1.9 without (caps 8 and 8), so at most a few % of Flow.
- R3 finding 11: a reversal test (the strip moves to the other side after 10, 20 or 30 decisions; 800 seeds): the
  learner against the null after the move +2.3, -0.7 and -3.5 standard errors; it does not follow the strip.
