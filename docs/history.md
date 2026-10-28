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

**After the round-4 review**

- Finding 1: a move out of the bed takes only what the bed can lose exactly (`Move::loosen`): worn amounts below the
  bed's rounding made matter; 48 x 48 seed 1 without life, energy drift at year 20 1.8e-13 to 3.4e-14 (flat).
- Finding 2: moving liquid wears its bed only with the work its load leaves free (`flow::liquid::load`): 48 x 48 seed
  1, year 30, light-eaters 1.60e10 to 1.52e10 kg, land 285.49 to 285.48 K.
- Finding 3: the decomposers' catalyst `open` works as far as the node is moist (`Body::catalyst`, `FlowView::moist`);
  the frame rots only in the open world (26: `k` / 1e6, `open` 1e6), body matter 5 times faster there (decision 48):
  48 x 48 seed 1, year 30, dead body matter on land 0.75 to 0.37 kg/m², frame 0.024 to 0.065, free sugar 0.009 to
  0.064.
- Finding 4: a taker's key liquids do not count as food in Holling's search (`feed::intake`): 48 x 48 seed 1, year
  30, eaters 4.3 to 2.9 kg/km² of land (placed at year 22).
- Finding 5: a bite reaches at least one whole member only of an individual body; of a node, what the swept area
  holds: 48 x 48 seed 1, year 30, eaters 2.9 to 2.5 kg/km² of land.
- Items 6 and 7: upkeep follows the bare rate law (`Body::upkeep_pace`, decision 49); `c_basal` at Kleiber (2e4 to
  6.3e4), `c_strength` at the resting muscle (7.677e4 to 1.31e4), `c_build` anchored on herbs (decisions 50, 51):
  48 x 48 seeds 1, 2, 3, year 60, light-eaters 10.49, 10.51, 10.54 to 10.40, 10.46, 10.54 log10 kg, eaters 5.6e3,
  8.0e4, 4.4e3 to 1.3e3, 2.6e4, 2.9e3 kg/km² of land (year 30: 2.5, 13, 14 to 1.6, 8.2, 9.3), land 287.0, 290.7,
  289.5 to 287.2, 290.9, 289.7 K; under 0.1% of body-days lay past the unfolding.
- Item 2b: founder light-eaters build their frame at a herb's share of cell walls (`WALL_SHARE` 0.4, decision 52):
  48 x 48 seeds 1, 2, 3, year 60, light-eaters 10.40, 10.46, 10.54 to 10.31, 10.37, 10.38 log10 kg, eaters 1.3e3,
  2.6e4, 2.9e3 to 190, 1.9e4, 5.7e3 kg/km² of land, land 287.2, 290.9, 289.7 to 286.6, 290.7, 289.3 K; year 100,
  eaters 8.6e3, 8.0e4, 3.8e3 to 7.8e3, 6.3e4, 2.8e3; seed 2's light-eaters' frame share falls to 0.26 by year 60 and
  0.20 by year 100.
- Items 2c, 2d: the eaters' check runs to year 100 and holds that they stop growing (in years 91 to 100 their births
  pass their deaths by under a tenth of the births); its bound is set on soil animals, 3e5 kg/km²: 48 x 48 seeds 1,
  2, 3, year 100, 7.8e3, 6.3e4, 2.8e3 kg/km² of land, births less deaths +6%, +6%, -15% of the births.
- The key-number references of the heat-absorbing gas and of life are set from the 3-seed runs after items 6, 7 and 2b
  (decision 53): the gas 339 to 388.8 ppm, light-eaters 10.49 to 10.38 log10 kg, eaters 5.20 to 4.54, their lives
  2.07 to 1.81 and 11.3 to 9.32 yr; the other keys hold (land 287.51 K, rivers 170.5).
- F7: the nodes check adds rates per member-year (rule d) and takes seed 5 as its noise (decision 54); it runs in 24
  minutes on 4 cores. Rules b and c hold in both plans; a holds for the default plan's seed 5 only; d fails: eaters in
  nodes die of starving 0.20 a member-year against 0.063 as individuals (seed 5; noise 0.20 against 0.089).

**After the round-5 review**

- Item 5: past `crowd` nodes a cell merges its most alike bodies whatever they hold (decision 56): 48 x 48 seed 2,
  years 91 to 100, the most bodies in a cell 61-803 to 15-53, bodies 6,641 to 5,950, parts 980 to 112, eaters
  1.85e8 to 1.86e8 kg, light-eaters 5.10e10 to 5.07e10 kg, land 290.63 to 290.61 K; body time -20%.
- Members of a node that come back after acting (`choose::spread`) keep the node's genes and spreads (`nodes::add`
  with no genes): the bodies' instructions in a year from year 90 (48 x 48 seed 2) -2.2%, `nodes::add` 2.77e9 to
  2.31e9; a rounding of the spreads, so the golden runs are pinned again; years 91 to 100, eaters 1.86e8 to 1.83e8
  kg, light-eaters 5.07e10 to 5.00e10 kg, land 290.61 K both.
- A node's part that stays in its cell acts out of any slot (`choose::spread`); `Life::remove` takes the body out of
  its slot, uncloned; perception stops at the bodies it sees: one year from year 90, the bodies' instructions 21.18e9
  to 20.80e9 (-1.8%), `spread` 4.59e9 to 4.27e9; the golden runs are pinned again (slots and ids); years 91 to 100,
  eaters 1.83e8 to 1.85e8 kg, light-eaters 5.00e10 to 5.00e10 kg, land 290.61 to 290.63 K.
- The cost model (plan 5.7) is measured on a settled world: 48 x 48 seed 2, years 91 to 100, side by side on one
  machine, 8.26 s a year before the three changes above and 6.99 s after (bodies 6,643 to 5,647 ms, Parts 335 to 48);
  about 300 cells fit the time budget, against 250.
- Intake: bodies take, and are given, only what they use (`Body::uses`: what they burn, digest or build of) and count
  only that in Holling's response; sand and rock no longer feed individuals, and drinking no longer takes part of what
  they take; a body no longer drinks from a part. 48 x 48 seed 2, years 36-40: eaters 1.5e5 to 4.8e5 kg, their life
  11.5 to 10.8 yr, starving 40 to 145 a year, land 290.1 K, drift 8e-15.
- An individual taken by an individual is found with the chance the taker's swept area holds it, as a node's members
  are (one draw for that and its escape). 48 x 48 seed 2, years 36-40: eaters 4.8e5 to 4.1e5 kg, taken 6 to 11 a year,
  starving 145 to 124 a year, land 290.1 K.
- Decisions 45 to 54 are cut to one sentence of decision and one of why; decision 50 names the known over-count of
  strength and catalysts paid on top of Kleiber (about 17% at the eater founder), decision 52 the frame's real worth
  (shading order, bite cost) and the plan names selection's blind spot inside a node.
- The owner allows 700 more code lines for running the bodies on threads: the core ceiling is 9,700 (decision 42).
- The key numbers after the intake changes (64 x 64, seeds 1 to 3, year 30) all hold; the eaters' references are set
  from them (decision 57): eaters 4.54 to 4.86 log10 kg, their life 9.32 to 10.47 yr; land 287.51 K, gas 389.1 ppm,
  rivers 170.8. The eaters' check fails: at 48 x 48, years 91 to 100, seed 1's eaters still grow (births pass deaths
  by 12% of births; 2.6e4, 9.1e4, 2.8e4 kg/km² of land; +12%, -1%, -22%).
- F7: the nodes check compares rule (d) at matched density and reads the default plan over years 31 to 40 (decision
  58); it runs in 5 minutes. After the intake changes the eaters' starving agrees: at matched density nodes against
  individuals 1.22, 1.40 and 0.68 times (32 x 32, seeds 2, 3, 5). Rules b (both plans) and c (roster) hold; a and c
  fail for the default seed 1 (a few dozen eaters); d fails on rare causes (eaters taken: nodes 0.0013 a member-year,
  individuals none) and on the roster's small eaters' starving.
- A body sees the parts of the cells within its sight as the last survey saw them, as it sees their other features
  (`choose::cells`), so what an update reads lies one link from it. 48 x 48 seed 2, years 51-60: light-eaters 2.05e10
  to 2.19e10 kg, eaters 7.1e7 to 9.7e7 kg (seed noise), land 290.66 to 290.71 K, drift 1e-14.

**After the round-6 review**

- The bodies run on one thread again (decision 59, "tried and not kept" below); the core ceiling is back to 9,000 and
  the 700 lines the owner allowed wait for threads over bodies that pay (decision 42). Core 9,567 to 8,998 code lines.
- A bite that reaches less than one member of a node meets one whole member with that chance and takes, tears and
  pays that meeting's outcome times the chance (`feed::take_body`), as an individual's draw does: 48 x 48 seeds 1-3,
  years 51-60, eaters taken 363, 1,080, 421 to 4, 731, 111 a year, eaters 8.8e6, 9.7e7, 7.3e7 to 5.9e6, 7.4e7, 7.7e7
  kg, light-eaters 1.95, 2.19, 2.11e10 to 1.93, 2.10, 2.08e10 kg, land 286.58, 290.71, 289.29 to 286.58, 290.69,
  289.33 K, drift 1e-14. F7: the eaters' deaths by bites agree; rule (d) fails on light-eaters taken (seed 5: nodes
  1.1e-6 a member-year against 3.1e-6, noise 2.3 times) and the roster's small eaters.
- The eaters' check reads births less deaths over years 71 to 100, as a settled decade swings by about 15% of births
  (seed 1 by decade from year 81: 11.5, 5.7, 10.1, -14.9, 7.8%); the limit stays 10%. Seeds 1-3: 7.7, 3.7 and -6.0%,
  1.7e4, 8.8e4 and 3.0e4 kg/km² of land at year 100: it passes.
- laws.md says that merging past `crowd` drops the structure the joiners held, and that the minerals a body builds of
  count in Holling's s.
- The chemistry finds once what may share each liquid (`Chemistry::mixes`) and the order its substances melt in
  (`Chemistry::by_melt`); a body's plateau state reads that order and sorts nothing where nothing dissolved lowers a
  point. Bit for bit the same; the bodies' instructions per update 242k to 231k (48 x 48 seed 2, year 91).
- A body taking from the ground works out the ground's heat capacity only for what it takes, not for every
  substance. Bit for bit the same; the bodies' instructions per update 231k to 220k.
- A body finds once which of its ground's substances are liquid (`physics::soil`) and once what it wants of each
  (`Body::wanted`). Bit for bit the same; the bodies' instructions per update 220k to 216k.
- Where a key liquid's soaking and its loss through the pores meet (`exchange::through`) is found by Newton's method
  from above (5 to 8 steps) instead of 40 halvings; it is more exact, so the history differs by rounding. Golden
  re-pinned. 48 x 48 seeds 1-3, years 51-60: light-eaters 1.94, 2.10, 2.08e10 to 1.85, 2.09, 2.16e10 kg, eaters 5.9e6,
  7.4e7, 7.7e7 to 6.1e6, 1.0e8, 4.7e7 kg (seed noise), land 286.6, 290.7, 289.3 K unchanged, drift 1e-14. The bodies'
  instructions per update 216k to 211k.
- A body's colour, volume and energy at a temperature read only the stores it holds, and its activity takes no power
  when its key liquids are at their level. Bit for bit the same; the bodies' instructions per update 211k to 206k.
- A body knows from its genes, as bits, what its burnings take and what its catalysts break (`Consts::fuel`,
  `Consts::broken`), instead of searching its reactions for each substance. Bit for bit the same; the bodies'
  instructions per update 206k to 201k.
- What a body's air node holds in all, as bodies see it (`World::a_seen_all`), is summed from what its update
  already gathered. Bit for bit the same; the bodies' instructions per update 201k to 197k.
- The cost model (plan 5.7) after these: the bodies of a settled 48 x 48 year (seed 2, years 91-93) 4.2 to 3.6 s on
  one thread, 47 to 40 µs an update (242k to 197k instructions); a settled cell 2.2 to 1.9 ms a year, so about 470
  settled cells fit 0.9 s a year (375 before).

**After the round-7 review**

- The intake law (`feed::intake`) loses its member count, which changed nothing but rounding. Golden re-pinned.
  48 x 48 seeds 1-3, years 51-60: light-eaters 2.01, 2.18, 2.18e10 kg, eaters 4.9e6, 8.6e7, 4.4e7 kg (seed noise),
  land 286.58, 290.71, 289.37 K, drift 1e-14.
- The prey decides the draw of a bite (`feed::take_body`): an individual taken by a node escapes or dies whole by one
  draw with the chance the node's area holds it, as when an individual takes it; before, a node's bite killed it for
  certain. Golden re-pinned. 48 x 48 seeds 1-3, years 51-60: light-eaters 2.01, 2.18, 2.18e10 to 1.96, 2.13, 2.05e10
  kg, eaters 4.9e6, 8.6e7, 4.4e7 to 5.6e6, 1.1e8, 6.9e7 kg, eaters taken 3, 903, 243 to 64, 1,358, 292 a year,
  light-eaters taken 2.0e3, 6.7e5, 1.6e5 to 1.2e5, 8.9e5, 2.7e5 members a year, land 286.58, 290.71, 289.37 to 286.60,
  290.71, 289.35 K, drift 1e-14.
- A reactor counts what it took from and gave to the gas around it as each reaction asked (`Reactor::dpool`), not as
  the pool's change, where a small draw on a large pool rounds away; bodies, parts and the ground read that one count.
  The yearly line prints each element's signed drift. Golden re-pinned. 48 x 48 seed 1 at year 100: element 2's drift
  3.6e-14 to 3.3e-14, 7.0e-15 in years 91-100 both times: the most of it rounds away in the air nodes' sums.
- The checks of a world's inputs (`Config::validate`, `Chemistry::validate`) are one line each (`params::require`); a
  NaN density or vaporisation enthalpy is refused too. Bit for bit the same. Core 8,993 to 8,971 code
  lines.
- An air node keeps what rounding takes from its amounts as `store::apply` adds to them (`Store::air_carry`, a
  Neumaier sum folded back into the amount), and the books count it: a body's small take of a main gas no longer
  rounds away. Golden re-pinned; snapshot 42. 48 x 48 seed 1 at year 100: element 2's drift 3.3e-14 to 9e-17, the
  worst element 7e-16, flat from year 10 (before: 7.0e-15 a decade in years 91-100, rising). Flow about 6% dearer
  (530 against 500 ms a year at year 90, bodies unchanged; about 1% of the year).
- The frost test builds its chemistry again after it changes a melting point, as what follows from the substances is
  made once (`Chemistry::subs` says so).
- The nodes check counts deaths by being taken per member-year and per kg of eaters per km² of the same year (the
  linear part of Holling's response), as the eaters grow many times over in a window. Default plan: light-eaters taken
  agree (seed 5: 1.1e-6 against 3.1e-6 a member-year before, no break now); rule (d) still fails on seed 1's few
  eaters (spent 0.30 against 1.58 a member-year). Roster, seed 1: light-eaters taken 2.2 times, past the noise. Rule
  (d) stays not held. Judging "less than one member" per taker member (`per = reached.max(b.n() / o.n()).min(1.0)`)
  brought the roster's light-eaters taken within a factor of two in one run, not past the noise: not kept, an open
  item of F7.
- Decision 42 says the present (ceiling 9,000, the 8,000 goal gone, 700 lines waiting for threads that pay) and is the
  one place for it; the handover and the plan point to it. Plan 5.7 and 6.5, and 5.6 and 10.1, each say a thing once.

**Phase L**

- L3: a ground node keeps what rounding takes from its amounts as `store::apply` adds to them, as an air node does
  (`Store::carry`): a body's small exchange with a node under deep standing liquid rounded away and lost element 0
  (5e-16 a year once bodies crowd such cells). Golden re-pinned; snapshot 43. 22 x 22 island seed 1 at year 300:
  element 0's drift -7.5e-15 to 1.6e-16 (flat), energy 7.8e-15 to 1.8e-15; the step of a lifeless island 5% more
  instructions (callgrind).
- L3: the surface balance holds a ground node at its temperature where its soft cover's resistance is too small to
  change it (`r den` under the last digit): a cover of almost nothing gave 1 / r = inf, a NaN in the air step and a
  stop of the 24 x 24 refined-region run at year 884. Bit for bit the same on the golden runs.
- L2: planet runs of 1,000 years (threads for Flow and Change, on a machine shared with three other runs): the 36 x 18
  globe the cost model allows took 102 minutes (bodies 1,068 at year 100 to 19,517, up to 236 in a cell), a 22 x 22
  island 24 minutes (its eaters died by year 28), the 16 x 8 globe 2.6 minutes (decision 60). A body update costs 31 to
  48 us, as the cost model says; the number of bodies does not stay near 2.4 a cell. The budget is not met on a planet
  where the eaters live.
- L3: the books over 1,000 years with threads and a refined region (24 x 24, 8 x 8 refined): elements within 5e-16,
  energy 3.0e-13 at the end (a rise in years 890 to 940). The 36 x 18 globe (before the ground carry): energy drift
  4.3e-9 from one liquid step in year 868 that overdrew a ground node's heat by 1.5e19 J, set to zero and counted in
  `Counters::rounded`: open.
- L4: the run's maps of the globe and of the refined island at years 10, 100 and 1,000, and the viewer's map and charts
  taken headless (`docs/reports/foundation-l/`). The viewer's `standalone.html` does not load the world it embeds (its
  script runs before the world's tag): open.
- `ew` checks its words against the usage text: an unknown word or a flag without its value stops with the usage
  (exit 2), and `--help` or `help` prints it; before, `ew run --help` started a run in `./out`.
- The viewer's `standalone.html` puts the world's tag ahead of its script, which reads it as it runs: it loads the
  world it embeds (checked headless) instead of asking for a file.
- Fixing (reaction 24) gives all the carbon it burns back as the heat-absorbing gas, written for two of the fixed gas
  (decision 61): its one unit of the burnable gas 21 per fixing has no sink at the air's temperatures, so it piled up in
  the air for good. 22 x 22 island seed 1, years 300 and 830: gas 21 in the air 9.9e10 and 2.4e11 mol before (the air
  started with 2.6e11 mol of the heat-absorbing gas), none now; that gas 156 and 111 ppm before, 149 and 166 now; land
  286.0 and 281.9 K before, 285.5 and 286.3 now; light-eaters 2.4e10 and 2.2e10 kg before, 2.4e10 and 3.4e10 now;
  bodies 3,010 and 8,024 before, 3,691 and 7,692 now; element drift 3.9e-16 at year 300. Golden re-pinned.
- L1: the reference runs on the new engine (`3546680`; A2's 64 x 64 runs, normal and roster, 60 years; the 128 x 128
  runs to 100 years, decision 62; series in `docs/reports/foundation-l/`): the key numbers hold (land 287.52 K, gas
  387.9 ppm, rivers 170, light-eaters 10.40, eaters 4.88 log10 kg). Against A2: swing +3 K, rivers / 2.2, gas +130
  ppm and eaters' life x 3.7 at 64 x 64, by the converged air step, clouds and 40°, the plants' costs and the costs at
  `t_cat`; at 128 x 128 the light-eaters still grow at year 100 (+0.5 log10 kg, bodies x 4) and seed 5's energy drift
  is 1.0e-7 from years 94-95, as a ground node reaches `T_FLOOR`: open.
- A full store stops the enzyme that fills it (end-product inhibition max(0, 1 - store / level), decision 63): under
  1 / (1 + store / level) plants held their stored solid at 2.1 to 4.5 times their level, 88 to 95% of their carbon.
  Seed 1, year 300: 22 x 22 island 149 -> 466 ppm, land 285.5 -> 287.7 K, light-eaters 2.4e10 -> 1.1e10 kg; 36 x 18
  globe 86 -> 333 ppm, land 282.3 -> 285.3 K, ice 11.5 -> 7.2% of the area.
- L3: liquid that comes into a node holds its fusion enthalpy as it passes, whatever the node's phase (a load holds
  none), so what passes on leaves at a temperature the node pays for: liquid at 244 K running through a frozen node at
  268 K was warmed to 268 K at the node's cost and overdrew it by 1.5e19 J (36 x 18 globe, year 868). Golden
  re-pinned. That globe resumed at year 860: energy drift at year 870 4.8e-9 to 1.9e-13, heat not given 2.1e7 J to 0.
- L3: the head liquid loses running out of a node heats what runs out (as under the surface, where it goes), not the
  node it leaves: a hot node that 97% of its liquid left kept the fall's 1e17 J, reached 465 K, boiled off the rest in
  the next air step with more heat than it held and overdrew it (128 x 128 island seed 5, year 94). Golden re-pinned.
- L3: what runs out of a node on the surface leaves at the temperature of the node mixed with what came in (on a
  melting plateau the plateau's substance melts or freezes first), not at most at the node's last temperature: liquid
  at 365 K running through a node at 292 K left at 292 K and the node kept the difference (the same run, the next
  node down). Golden re-pinned.
- L3: a ground node evaporates of the rain of an air step only what it gets once the air's cut has set it: what
  evaporates was limited by what it held and the rain asked of the air, so a cut rain let a node give 2e6 mol more of
  a gas than it held (the same run, years 94-95; element drift 3e-11). Golden re-pinned.
- `docs/reports/foundation-l/` keeps the 11 pictures and, per run, the series gzipped (the 1,000-year runs: the rows of
  every tenth year) and the run log gzipped: 14 to 2.4 MB (`docs/reports/README.md`). No code change.
- Snapshot size measured, no code change (decision 64). 48 x 48 seed 2, year 60: 19.6 MB (25.6 MB before gzip; 5,134
  bodies, 1,083 of them with a brain); 82% of the file is four sets of f32: brain rows 6.3 MB, born rows 6.0 MB, genes
  2.4 MB, spreads 1.2 MB; the nodes' store, sleep and flow state 1.6 MB. 128 x 128 seed 1, years 10 and 20 (plants
  only, 22,725 bodies at year 10): 31.7 and 35.4 MB; genes 10.8 and spreads 5.6 MB of the 31.7, the fixed nodes' store
  6.2, sleep 2.2 (the rounding carry of the ground, 2.0 MB, is noise in its low bits). Tried on these files and not
  kept: byte planes of the f32 (-6% of that stream), a gene-major layout (-5% more), XOR with the born rows (-2%) and
  with the parent's genes (no gene equals its parent's), gzip level 6 (-1%), the stream split by width with planes
  (-5% in all, -9% with level 6). Save 0.2 s and load 0.15 s at 48 x 48 (0.34 and 0.35 s at 128 x 128, year 10).
- The air step's table of saturation pressures takes each entry from `Chemistry::p_sat` (one exponential each, built
  once), not from a series between exact entries: 15 code lines fewer, the same speed; the last digits move. Golden
  re-pinned.
- A soft cover (snow, ice) holds no liquid in pores, and its top melts: its surface warms no further than its melting
  point, and the node under it takes the heat past that (decision 65). With loam's pores the cover held a quarter of
  its volume as melt, and a thick cover kept its node from the summer air, so snow on mountains grew for good and the
  sea shrank. Seed 1, year 300: 22 x 22 island liquid on land 44 -> 0.14 km³, ice on land 196 -> 0 km³, sea 428 ->
  651 km³, land 287.8 -> 292.3 K, gas 3 534 -> 291 ppm, rivers 6 -> 84, eaters 0 -> 1.8e5 members, element drift
  1.8e-16 -> 3.6e-16; 36 x 18 globe liquid on land 4.6e6 -> 6.4e5 km³ (an inland sea now fills toward its
  rim at 952 m), ice on land 3.2e7 -> 4.2e3 km³, sea 5.1e7 -> 8.4e7 km³ (8.45e7 at the start), land 286.1 -> 288.3 K,
  gas 3 434 -> 162 ppm, element drift 3.1e-16 -> 2.0e-16. Golden re-pinned.
- Rain from air below its melting point at the ground falls as a solid (snow): the air keeps the fusion enthalpy its
  freezing frees. All rain brought the ground its fusion enthalpy, which froze out in cold ground and held snowy
  mountains at the melting point (42 W/m² on a cell with 4 m of rain a year). Seed 1, year 300: 22 x 22 island land
  292.3 -> 292.4 K, gas 3 291 -> 277 ppm, rivers 84 -> 89; 36 x 18 globe ice on land 4.2e3 -> 5.1e3 km³, liquid on
  land 6.4e5 -> 7.0e5 km³, land 288.3 -> 288.9 K, gas 3 162 -> 211 ppm; element drift 2.6e-16 and 2.0e-16. Golden
  re-pinned.
- The air step boils of a substance over its boiling point only what the node's heat over that point pays, cooling it
  to that point (`Scratch::boil_off`, shared with gas coming out of the liquid): it boiled all of it whatever heat that
  took (a 100 m pool 2 K over its boiling point lost all its liquid in a step and ended at 4,900 K). Bit for bit the
  same on the golden runs; a test holds it.
- What evaporates leaves at its node's temperature (the heat it held there), not at its surface's, in the solve, the
  books and the end state, as gas coming out of the liquid does: under a soft cover the surface may be tens of K
  warmer than the node, which then paid heat it did not hold. Seed 1, year 300: 22 x 22 island land 292.4 -> 292.8 K,
  gas 3 277 -> 298 ppm, rivers 89 -> 96; 36 x 18 globe land 288.9 -> 288.5 K, gas 3 211 -> 193 ppm, ice on land 5.1e3
  -> 4.4e3 km³; element drift 2.3e-16 and 5.2e-16. Golden re-pinned.
- The series records the frozen cover on land (`ice_land`, km³): `liquid_land` and `ice_frac` did not show the ice
  that grew on mountains. Open: ice does not flow (decision 66); on the 16 x 8 globe, whose land starts at 271 K, ice
  on land still grows (3.3e7 km³ at year 100, 7.9e7 at year 300, the sea from 8.8e7 to 1.6e7 km³), and on the 36 x 18
  globe an inland sea fills toward its rim at 952 m (6.5e5 km³ standing on land at year 300).
- The key numbers after snow melts (64 x 64, seeds 1 to 3): land 287.83 -> 291.79 K, swing 13.61 -> 15.13 K, rivers
  136.1 -> 712.4, land worn down 0.0424 -> 0.141 mm/yr are their new references (decision 67); rain 1.793e-3, gas
  406.0 ppm, light-eaters 10.25 and eaters 5.34 log10 kg, their lives 1.33 and 13.95 yr hold their old ones.
- Pushing through wears a cell's bodies once a survey (`choose::wear`, decision 70): the energy movers spend in a cell
  is kept (`Life::pushed`, snapshot 45) and then shared by mass over the bodies there, so a move no longer wounds every
  body of its cell (a cell's moves cost the square of its bodies: the 24 x 24 region run took 17 min for year 421). The
  lane clears by that energy over the cell's breaking energy per m² (`Life::sum`). Golden re-pinned. 22 x 22 island
  seed 1, years 291-300: land 293.7 -> 294.6 K, light-eaters 3.6e9 -> 5.8e9 kg, eaters 4.5e6 -> 6.5e6 kg; at year 300
  bodies 2,386 -> 2,107, element drift 2.1e-16 -> 2.6e-16.
- The 24 x 24 island with the 8 x 8 refined region again, after decision 70 (seed 1, `docs/reports/foundation-l/`):
  stopped at year 780 after 90 min, as a year then took 40 s and climbed. A year: 2.1 s at year 100 (bodies 1.7 s), 4.1
  s at year 400 (bodies 2.9 s, parts 0.65 s), 40 s at year 780 (bodies 30 s, parts 9.4 s). Bodies 3,502 (at most 19 in
  a cell), 6,077 (39), 32,427 (261). Elements within 5.2e-16, energy within 2.3e-14, heat not given 0. The bodies grow
  by lineages that never merge (decision 71, open): a child takes its brain-size genes from a partner of another
  lineage a whole unit off its own and starts a lineage of its own. The run before decision 70 had it in the eaters
  from year 370 (lineages alive 16 at year 370, 402 at year 405, eater bodies 2,142 to 7,454 at the same 2e5
  members); this one in the light-eaters from year 720 (alive 18 at year 700, 412 at year 780). Most of those bodies
  are in the refined cells: the crowd merge keeps some nodes per cell, and those cells have a quarter of the area.
- An individual's child joins its parent's lineage when its brain-size genes fit it, else its partner's when they
  fit that, its rows carried over to that brain; a new lineage starts only when neither fits (decision 71). The golden
  runs keep their fingerprints (no child there fits only its partner's lineage). The 24 x 24 region run, seed 1, the
  same to year 720; stopped at year 780 after 88 min as before: lineages alive 41 / 72 / 328 at years 730 / 740 / 780
  (42 / 88 / 412 before), bodies 33,301 (at most 319 in a cell; 32,427 and 261 before), a year 45 s (bodies 34 s, parts
  10 s); at year 100 / 400 a year 2.5 / 3.8 s, bodies 3,502 (19) / 6,077 (39). Elements within 5.2e-16, energy 2.3e-14,
  heat not given 0. Land 293.4 / 293.4 / 292.6 K, gas 3 439 / 424 / 422 ppm, light-eaters 2.5e9 / 5.0e9 / 4.9e9 kg,
  eaters 4.0e6 / 9.9e5 / 2.1e6 kg at years 100 / 400 / 780. In years 730-740 no child joined its partner's lineage;
  of 174 new lineages 134 came from `lineage::resize` (light-eater nodes whose seen and hidden genes, which do nothing
  without a brain, drift a unit: 3 -> 4 and 0 -> 1), 38 from children that fit neither parent, 2 from a new base
  (decision 72, open). 17 x 17 island 1,000 years in 15.9 min beside the region run: bodies 1,523 (29 in a cell) at
  year 500, 1,535 (28) at year 1,000; eaters alive (1.4e5 members, 8.7e6 kg).
- A body with no brain does not mutate its brain-size genes (`body::vary`, decision 72): light-eaters' seen and
  hidden genes drifted a unit and each node started a lineage of its own. Golden re-pinned. The 24 x 24 region run,
  seed 1, now runs 1,000 years in 86 min: lineages alive 2 / 4 / 4 / 6 at years 100 / 400 / 780 / 1,000 (412 at 780
  before); bodies 2,891 (at most 21 in a cell) / 7,676 (49) / 10,026 (59) / 11,536 (73), 33,301 (319) at 780 before;
  a year 2.4 / 4.1 / 7.6 / 9.6 s (bodies 1.9 / 3.6 / 7.0 / 9.0 s, Flow 0.34 s, Change 0.11 s, parts under 0.02 s),
  45 s at 780 before. Elements within 3.7e-16, energy 4.4e-14, heat not given 0. Land 293.5 / 293.6 / 293.8 / 293.8
  K, gas 3 442 / 432 / 415 / 417 ppm, light-eaters 2.5e9 / 5.4e9 / 6.0e9 / 5.0e9 kg, eaters 1.1e6 / 6.5e6 / 3.0e7 /
  3.5e7 kg. The bodies still grow (eaters 1.4e6 members at year 1,000). 17 x 17 island 1,000 years in 18.2 min
  beside it: bodies 1,706 (28 in a cell) at year 500, 2,251 (46) at year 1,000; eaters alive (2.2e5 members, 9.3e6
  kg).
- L5: the foundation watch report (`docs/reports/foundation-l.md`), covering Phases E to L.

**After the final review**

- The yearly line prints what `store::apply` set from just below zero to zero (`Counters::rounded`, mol and J) next to
  the heat not given, so an overdraw cannot hide.
- The critic's running mean square of errors weighs the newest error by `learn_beta` (0.1), as each row's mean square
  of steps does, not by a second number (0.05). `body::MARGIN` and `PHASE_MARGIN` have rows in numbers.md (decision
  68). Golden re-pinned. 48 x 48 seed 1, years 31-40: light-eaters 1.15e10 kg both, eaters 7.1e5 -> 6.5e5 kg, land
  292.1 K both, element drift 2e-16 both.
- Past `crowd` nodes in a cell the most alike pairs merge whatever room they are in, as whatever they hold (decision
  56): the one that joins takes the other's room. The golden runs keep their fingerprints (no rooms in two years). 36 x
  18 globe seed 1, years 101-200: bodies 2,974 -> 2,497, the bodies' stage 2.25 -> 1.98 s a year, parts 167 -> 127 ms
  a year, land 289.1 -> 289.2 K, eaters 6.6e12 -> 6.3e12 kg; at year 200 at most 142 bodies in a cell both times (in
  the new run 89 eater nodes of that cell are unlike in their genes, not apart by rooms).
- Clean-up, bit for bit the same: a body's energy at a plateau state is the heat that would leave with all its stores
  (`Body::heat_with`; `fusion_in` goes), the light a layer intercepts is `math::mean_decay` (the alias `intercept`
  goes), and the survey no longer sizes the cells' light, canopy and rain sums the world is made with.
- Selection inside a node counts as gain body matter and, of the matter the stores could make, at most what one
  interval's growth allowance builds (`copy::buildable`, decision 69): storing cost nothing, so the keep gene of the
  stored solid climbed and plants hoarded it. Golden re-pinned. 22 x 22 island seed 1, years 291-300 (the solid as
  share of plants' dry mass and their keep gene at year 290): heat-absorbing gas 268 -> 435 ppm, flat from year 100
  (before, falling), the solid 86 -> 70% of dry mass, keep gene 4.9 -> 1.0 (0.9 at the start), frame 0.6 -> 0.1% of
  fresh mass, light-eaters 1.6e10 -> 3.6e9 kg, eaters 1.3e6 -> 4.5e6 kg, bodies 2,898 -> 2,463, land 292.2 -> 293.7 K,
  element drift 1.4e-16 -> 2.1e-16. Seed 2: gas 324 -> 445 ppm, keep gene 4.7 -> 2.5, the solid 86 -> 80% of dry
  mass. 64 x 64 key numbers (seeds 1-3): land 291.79 -> 292.15 K, gas 406.1 -> 411.3 ppm, light-eaters 10.24 both,
  eaters 5.32 -> 5.34 log10 kg, all in range. Counting built solids at their mass too kept the frame at 7 to 14% of
  fresh mass but made lime a fifth of it (seed 1, year 290): not kept.
- The tape no longer clusters bodies into kinds (the columns `kinds` and `kinds_members`, a weighted k-means that the
  handover said not to read); `trophic_max` and `trophic_high` go by starting kind. 22 x 22 island at year 290, ten
  years: the tape 83 -> 21 ms a year, the year 1,596 -> 1,533 ms, the same world.
- Clean-up, bit for bit the same: the world is made with its per-cell lists of bodies and parts and its ring of updates
  sized (`Life::new`), so `Life::fit_map` and `bond::ready`, which sized them at each survey and month, go.
- The series records the land's water budget: what evaporates from land (`evap_land`) and what runs over it into the
  sea (`runoff_sea`), from moving averages Flow keeps per cell (`World::evap`, `World::off_land`; snapshot 44). Golden
  re-pinned (the world holds two more lists), the same laws. 36 x 18 globe seed 1, years 31-40: land rain 3.1 m a year
  = evaporation 0.62 + runoff 2.33 (Earth: 0.76 = 0.48 + 0.28, Trenberth et al. 2007); the sea evaporates 1.40 m a
  year and gets 0.45 of rain (Earth about 1.2 and 1.1); vapour 0.82% of the air (Earth about 0.4%), the ground 293.4 K
  (land 290.4). The air carries the sea's vapour onto land and rains it there: open.
- The key-number test holds two numbers to wide bounds around Earth's: rain on land within 3 times 0.76 m a year, and
  the light-eaters' stored solid under half their dry mass; the lifeless globe's test holds its whole surface within 4
  K of 288 K, its vapour within 2 times 0.4% and its rain on land as the island's. The globe passes (290.05 K, 0.69%,
  1.78 m); the island's rain passes (1.02 m) and its stored share fails (0.552; seeds 0.558, 0.547, 0.551): open.
- L2-L4: the long runs again on the final foundation code (seed 1, threads for Flow and Change, 4 cores with one other
  run at most; `docs/reports/foundation-l/`). 22 x 22 island 1,000 years in 25.1 min (1.5 s a year; 2,351 bodies, at
  most 35 in a cell; land 293.2 K; eaters alive); 36 x 18 globe 1,000 years in 36.6 min (2.2 s a year; 1,559 bodies, at
  most 31; land 290.5 K, ice 1.8% of the area); 17 x 17 island 11.5 min and 24 x 12 globe 9.0 min (the sizes that fit 15
  minutes). Over all years: elements within 7e-16, energy within 9e-14, heat not given 0. The 24 x 24 island with the 8
  x 8 refined region was stopped in year 421, 63 min in: the eaters' bodies grew from 10,403 (year 400) to 28,187
  (year 420, 312 in a cell) and year 421 took over 17 min (a mover's wear touches every body of its cell: open). CPU time is 1.0 to 1.03 of
  wall time: only two stages run on threads.

**Tried and not kept in the foundation**

- Threads over bodies in waves of blocks, each block on a world of its thread's own (decision 59): the bodies of a
  settled 24 x 24 island at year 90 took 611 ms a year before, 665 on one thread and 800 on four; 48 x 48 gained 1.4
  times on four; an update on a thread's world cost 24-33% more, with about 1,475 waits between waves a year. Before
  it: blocks in four fixed colours moving their whole reach (moving cost more than the updates), and regions run
  before the waves (half the updates fell on their borders; bodies 3.6 to 4.0 s a year).

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
- Round 5 finding 4: a node's members' excess over the copy mass spread evenly from none to twice the mean, the
  starving rule averaged over it (closed form): at matched density nodes starved 2.12, 0.30 and 0.35 times as much
  as individuals (seeds 2, 3, 5) against 1.22, 1.40 and 0.68 without it.
- Final review item 8: a node that expects less than one death in an update loses one whole member with that chance
  (`body::live::lose_share`). Nodes check: rule (a) past the noise 4 -> 2 (the roster's small kinds' lives closer), but
  rule (d) 5 -> 7, and the default plan's seed 1, which held every rule, broke (d) (its eaters died spent 2.4e-2
  against 5.0e-2 a member-year): not kept.

## 5. Speed after the foundation

- A1: the bench kit (scripts/bench.sh) and the baseline (docs/speed-post-foundation/measures.md).
- A2: `codegen-units = 1` kept (island22 -7%, globe36 -4% a year; build 25.6 -> 36.4 s; golden re-pinned at level 2). Not kept: fat LTO (under 2%), `target-cpu=native` (slower), `panic = "abort"` (tests use `should_panic`), PGO (5% and 4% over the kept setting, under 8%).
- A3: three `ew` runs side by side take 3.94 s against 11.33 s one after another (2.9 times faster); `RAYON_NUM_THREADS` 1, 2 or default changes the total by 1% only. Seeds in one test process share rayon's one global pool, so they do not oversubscribe. Advice in `docs/speed-post-foundation/measures.md`.
- E2.1: members of a node that stay in their cell act as the node itself while the others wait aside, and come back by `nodes::rejoin` (amounts, sizes, rows after watching); gone: the clone of the node per part, the pool and cap of the row spread at each watch, and the `see` per part; `nodes::take_in` goes. globe36 bodies -4% (best runs; -7% of their instructions); level 3, golden re-pinned; eaters key number 5.391 against 5.354, out of the old range, reference set again (S49); core 8,998 -> 8,996.
- D3: `par::rows` has no least size of task any more (rows split as far as threads are idle) and `ew` makes its main thread one of the pool's threads (`par::adopt_thread`): Flow 150 -> 122 ms (island22) and 229 -> 188 ms (globe36) a year, 1.23 and 1.22 times faster, bit for bit (golden unchanged with and without threads). Tried and not kept: the air's `gather` per air node, `fixed`, `room` and liquid `slopes` on threads (slower or the same: a call into the pool costs 30 to 55 µs); the other candidate loops take under 200 µs a call and were not built.
- E2.2: `pow`, `exp` and `expm1` in the body update: the rate of the main burning at `t_cat`, the copy mass and the tissue factor of upkeep are kept in `Body::k` (worked out at each change of the genes), the powers 2/3 and 1/4 are a cube root and square roots, `Reactor::sub_step` works out `gone` once, and `physics` takes its leaf area from the film; one year on one thread, island22 -2.10%, globe36 -1.41% of the instructions, and per body update -3.44% and -2.18% of the body day (1.91% and 1.09% of the year's instructions, under the 2% kill line by that measure); level 2, golden re-pinned. Not done: `intake_exp` stays a free number, so `equiv` keeps its `pow`.
- D1 (a): not built (S56). The bench's tape stage is the recorder reading the world (`month()` about 6.5 ms, the yearly row about 5.5 ms a year on island22); the file writing a thread could take is outside it and about 0.5% of a year in a real run.
- E2.3 (tape), early: not built. The whole recorder (`Recorder::record_month`, its month and yearly row) is 2.03% of island22's and 1.38% of globe36's instructions in a year: even at no cost it would not reach the 2% kill line. Measured again in E2.3 after D2, when the bodies' share has fallen.
- B1 + B3, tried and not kept (S59): one node per kind per cell for nodes without a brain, its young and adults two stages of one store (B1), each stage its own layer of its cell's light at its member's height (B3). Benches remade against `main`: island22 bodies 2,769 -> 3,930, updates a year 27,974 -> 40,689, the bodies' day 3.36 -> 5.00 billion instructions a year (+49%); globe36 bodies 1,904 -> 1,153, updates 25,345 -> 19,048, 4.91 -> 4.16 billion (-15%): both under the kill line (-30% and -22.5%). An update costs about what it costs on `main`; island22 holds more nodes and updates them more. B1 alone: island22 3,366 bodies (+39% instructions), its young getting their adults' light; with animals in one node too, nodes check rule (a) broke (eaters as nodes 2.9 times eaters as individuals). Branch `speed/b1-b3-not-kept`.
- C2, tried and not kept (S62): its gate found that slow nodes differ from fast ones by the interval's energy budget, not by the light: `settle_upkeep` pays a whole interval's upkeep from the stock before the interval's light comes in, and growth keeps a reserve of an interval's upkeep, so the stock grows with the interval (at 144 days: adult deaths of unpaid upkeep x13, fuel stock 155 -> 327 days, births per adult -31%); light compounding through the capture area does nothing measurable. The method (upkeep the stock cannot pay paid from the interval's income before deaths; a reserve net of income) brings the pace test at 70 days within its noise (5 of 5 seeds) but not at 140 (0 of 5), breaks the test around 36 at 72 days (seed 1, `air_3`), and its remade globe36 costs 45% more instructions a year. Branch `speed/c2-not-kept`.
- C1, not built (S63): dry bodies are 3.4% (island22) and 5.5% (globe36) of the updates of bodies without a brain.
- B2, tried and not kept (S66, S67). The milder rule (past `crowd`, the most alike pair of one lineage and stage merges while the raw variance of their mixture stays within a node's cap): from `main`'s bench saves bodies -14.7% (island22) and -20.2% (globe36), a year's instructions -9.9% and -7.6%, key numbers held; the equivalence failed (`--crowd 8` +6.6%, `merge_cap` x2 +9.4% of the light-eaters' mass against a noise of 5%, seed 1). Branch `speed/b2-rule-not-kept`. The one predicate for every merge path, bit for bit, cost +1.41% and +0.37% of a year's instructions (the crowd +15.7% and +4.5%) for 39 core lines: branch `speed/b2-predicate-not-kept`.
- D2.1a: a body's id is its domain of birth (the cells under one air node) and that domain's count of births, given at birth and never changed (S10); the rng keys, `Body::known`, `Body::link`, `Part::held` and the events use it; an id table finds a body. Updates go domain by domain, cell by cell, then by id; sums over bodies domain by domain, cell by cell, each cell's in the order they came; a snapshot keeps the bodies in that order and not where they are stored. Level 3: golden re-pinned, snapshot version 46, benches remade (`/tmp/d21-bench`). Per update against `main` on saves remade by each: +3.5% (island) and +6.2% (globe), from the other worlds the new keys give. Core 9,008 -> 9,107.
- D2.1b: `Life` holds a `Domain` per air node: its bodies (local slots, free list), its cells' lists and its ring; a body is named by a `Handle` (domain, slot) for the length of an update and by its id beyond; a body that steps into another domain's cell moves to that domain's storage when it is put back (`Life::put`), with its due update. Level 1 against D2.1a (golden and 3 years from the remade saves bit for bit). Per update +0.89% (island22) and +0.93% (globe36). Core 9,107 -> 9,149.
- D2.2: bodies update in batches of `batch_days` (2) days; a body's interval is its `INTERVAL` gene rounded to the nearest whole number of batches (5 days becomes 6), at least one, so every due day is a batch's start and no body runs early (S8, S64); `Body::due` and the ring are indexed by batch, a body placed during a batch first updates in a later one; `survey::day` is `survey::batch`, `Body::prepare` takes the `Config`; `Config::validate` refuses a batch that does not divide `phys_days` and the month, or `node_days` that is not a multiple. Level 3: golden re-pinned, snapshot version 47, benches remade (`/tmp/d22-bench`). A batch holds 4.95 ms (island22, 178 updates) and 5.51 ms (globe36, 144) of body work; per update -1.3% and -3.5% against D2.1b on other worlds. Key numbers hold (eaters 5.350); batches of 1 day against 2 move each key number of life by 9% of its noise or less. Core 9,149 -> 9,163.
- books: a ground node's gas to the air is what its reactions made less what the node itself gained (`change::react_ground`), not the reactor's own plain sum of the gas it exchanged; that sum added terms of the size of the air's gas (1e5 mol) that cancel, and differed from the exactly summed `made` by their rounding (1.5e-11 mol), so a node holding none of a gas was asked for a little more than it held (the debug assert of `store::apply` in `a_node_sleeps_by_its_own_rates`; release set it to zero and counted it). Level 3: golden re-pinned (the same with and without threads).
- D2.3: the bodies of a batch see each other only by the faces taken at its start (`survey::Faces`: id, members, visible traits, signal, well-being, room; `Choice::seen` holds faces) and the store through their domain's view (`store::Moves`: the ground, bed and bodies' nodes of its cells and its air node as the batch's moves left them, each taken from the store at its first move; another domain's node as at the batch's start); the batch's moves are booked by one `apply` per domain after it; `copy::sample` runs in a checkpoint of the view and rolls back (S38); taking and giving find the body by id, still in the cell; `heat_cap_ground`, `thaw`, `dig` and `anchor` read through the view; `Life::holds` goes; the view is indexed by a cell's slot in its domain (`Life::cell_slot`). The bench has `region24` and the column `busiest_domain` (0.170, 0.228, 0.274). Level 3: golden re-pinned, no snapshot change, benches remade (`/tmp/d23-bench`). Per update on the same saves: +0.35% and +0.13% (D2.2's), -0.15%, +0.50% and -4.4% (D2.3's, with `region24`). Key numbers hold (eaters 5.364). Core 9,181 -> 9,364.
- D2.4a: the outbox and the serial phase, the update code still on `&mut World` (S69). Every move to another cell (`choose::go` pays and queues it, `choose::land` lands it), every litter that starts in another cell (`copy::emit` takes it from the parent, `copy::land` joins or places it, or gives it back to the parent's brood or to the ground of its cell), every watch (`choose::learn::watched`, by the share of members who watched) and every act on a part (`choose::part_act`; a node's part with one queued stays a body and comes back after it) goes to its domain's outbox (`outbox::Entry`), applied in the serial phase domain by domain in the order made; a body with a queued move is leaving (`Body::leaving`: it does not split, is not seen in its cell, nor taken from, given to, joined or mixed with); bodies that hold or are inside a part run in the serial phase; lineages and born rows made in the parallel phase get provisional ids of their domain, made global after it (`Life::remap`); holders' counts change by per-domain deltas added at the batch's end, where a lineage with no body dies out (`Life::settle`); the batch's moves are booked in domain order; `survey::batch` takes a test-only reversed order. Gone: `go`'s immediate move, `emit`'s kin search in another cell, `learn::watch` and the watcher's rows kept aside in `part_acts`, the part rejoin test in `part_acts`, `Life::unref`. `life.rs` becomes `life/` (`registry.rs`: lineages and born rows), where `enter`, `leave`, `reborn`, `new_lineage` and `new_genome` change (provisional ids, holders counted by deltas) and `brain` is new. Level 3: golden re-pinned, no snapshot change, benches remade (`/tmp/d24-bench`). Per update +0.48% (island22) and -0.39% (globe36) against `main` on the same saves; the serial phase is 9.8% and 12.0% of the bodies' instructions. Key numbers hold (eaters 5.377). Core 9,365 -> 9,577.
- D2.4a review: a holder that dies or joins another in the parallel phase lets go of its part by a part act queued in the outbox (`bond::LET_GO`, applied by `bond::dropped`); `young::linked` reads no body of another domain; a leaving body does not mature (S70); a body with a move or a part act queued does not split (`Life::queued`); a part act needs the part in the body's cell, and a node's part rejoins only a node that is not leaving; a move carries only what `go` paid to carry (else it lets go); `outbox::deliver` books its moves; names `choose::land_move`, `choose::land_act`, `copy::land_litter`, `life::Held`. Level 3: golden re-pinned (seed 3 moves; the move that carries only what it paid for and the rules of part acts change its history; seeds 1 and 2 hold). Core 9,577 -> 9,606.
- D2.4 is split in D2.4a (the outbox, the serial phase) and D2.4b (the types); the state D2.4a leaves shared (`World::moves` and its `short`, `Life::stale`, the scratch, the id table, events and counters) goes per domain in D2.4b (S71). In the D2.4a review, seed 3 of the golden test moves by two rules only: a move carries only what `go` paid for, and a part rejoins only a node that is not leaving.
- D2.4b: body code takes `(&World, &mut Domain)` (S72): the domains are taken out of the world for the parallel phase and for every serial step (`World::in_domain`); each owns its bodies and their lists and id table (`Life::find` goes to the domain a body was placed in, or the one it moved to, `Life::away`), its store view and moves (`store::Moves` per domain, its methods take the world), the light's stale flags of its cells, its events and counters, its outbox, its provisional lineages and born rows and the holders' changes; the world settles them after the parallel phase and each serial step (`World::settle`: events and counters added in domain order, ids made global, holders added outside a batch at once, in one at its end). Lineages and born rows are always provisional; a provisional lineage's id says whether it has a brain (`Body::interval`, `body::upkeep`). Let-goes (`bond::let_go`, by a load `bond::TORN`) and anchors (`bond::ANCHORS`) are part acts; `outbox::deliver` applies what applying queued; a split looks only at the part acts queued in the update (`Domain::queued`). `Life::put` and `Life::relocate` become one `Life::relocate`; `World::open_moves`, `World::place`, `World::see`, `Life::queue`, `Life::parallel` go. Each domain gets a fresh `Reactor`, `LiveTmp` and `ActTmp` per batch. The reversed-order test compares the whole world, its events and counters. Level 2: golden re-pinned; island22 two years the same (all series, every body but the ids of born rows). Per update +0.74%, all instructions -0.20%. Core 9,606 -> 9,684.
- D2.4b review: `Life::brain` and `Life::born` read only global ids; `bond::anchor` reads the bed through its domain's view; `bond::dropped` takes the body (`let_go_now` goes); one `Domain::unstore` for `remove` and `Life::relocate`; `Moves::a_seen_all` again; `Life::for_cell` and `Life::list` go; `Domain::queued` leaves out let-goes and anchors (doc); the reversed-order test has a body that anchors its part and keeps the books; S72 and measures.md name the level-3 parts (a let-go after the rest of a serial pass). Level 1. Core 9,684 -> 9,666.
- D2.5: the domains of a batch's parallel phase run on rayon's pool with the feature `threads`, one task a domain (`par::each`; in order without it); the order test runs on pools of 1 and 4 threads, in order and reversed. Level 1: golden holds with and without threads. Benches remade: the bodies' stage 1.79, 1.67 and 1.76 times faster on 4 threads than on 1 (island22, globe36, region24); with the cost per update of `main` before D2.1, 1.55, 1.63 and 1.69 times (the target 2.2, the kill line 1.6; the decision waits for D2.6, S41). Core 9,666 -> 9,678.
- D2.5b: the tasks of the parallel phase run largest first (sorted by their updates, ties by domain index), and the pool's idle threads take them in that order (`par::each` through `par_bridge`). Level 1. The bodies' stage on 4 threads -7.9%, -4.3%, -13.3% (island22, globe36, region24; 7 interleaved runs, medians); sorting alone with rayon's halving gave -5.1%, +2.4%, -4.3%. Core 9,678 -> 9,679.
- D2.6: the survey on threads. The wear of movers and the monthly merge run per domain on the pool (`survey::on_domains`, which the batch's parallel phase uses too) where no part is and no body holds or is inside one, then in those cells one by one; the shade per domain, the sums per cell (`par::rows`). `choose::wear` and `nodes::crowd` work on one cell of a domain; `Life::take_stale` goes. The Draws and marks of the air step stay serial (S73). Level 2: golden re-pinned (only the ids of born rows differ; the events and every other number the same); two years of the three benches the same in all series. The bodies' stage on 4 threads -8.9%, -6.9%, -4.4%. The order test covers the survey. Core 9,679 -> 9,691.
- Scratch kept per domain, tried and not kept: each domain's `Reactor`, `LiveTmp` and `ActTmp` kept from batch to batch (level 1). The bodies' stage on 4 threads -1.6%, +3.1%, +3.9% (island22, globe36, region24; 7 interleaved runs): no gain above the noise.
- Booking in the task, not built: books written by the domains on the pool (a raw-pointer probe) are slower, +6.3%, +5.7%, +7.1% (the domains' numbers share cache lines); the whole booking after the parallel passes is 5.3%, 4.2% and 5.9% of the bodies' stage on 4 threads; a safe design (the view sums as `apply` does, the serial phase copies it) needs about +30 core lines with 9 left (S3): stopped for the owner. After D2.6 the gain against `main` (S65) is 1.82, 1.79 and 1.98 (kill line 1.6, target 2.2); with 4 threads, 36%, 40% and 32% of the bodies' stage stays on one thread.
- D2.5–D2.6 review: the order test's first survey kills a body by wear, kills a holder that lets go of its part in a serial cell, and merges two nodes into new born rows, asserted; `Draw`, `Draws` and `claims` move to `survey/draws.rs`; `Life::sum` holds the whole row of `survey::sums` (`ROW` numbers); `nodes::crowd` without its guard, `peopled` a closure; `body::BROKEN` public for the test; plan.md's D2.6 says the Draws stay serial (S73) and `clamped_heat` goes in domain order. S74 (booking in the task not built) and S75 (D2 kept, the kill decision). Level 1. Core 9,691 -> 9,689.
- After D2: profiles and gates measured (`measures.md`). E3 is not built (about 3% repeated, gate 10%, S76); E1's gate passes as written but more than half is the instruction cache, so E1 starts as a probe (S77); D1 (b)'s gate passes (one thread alone 41% and 53% of a year) and it is built after a simplification of `core` (S78); E2.3 picks six spots (S79).
- E2.3, the repeated reads of one update: a `Payer` holds the body's activity and is made with the cover of other bodies over it (read once in `site`), `buildable` and `upkeep_at` take the activity they are given, `starved` works out its plateau state only when it loses a share, and `grow` no longer clones the reactions of the chemistry. Level 1 (golden holds with and without threads; two years of the three benches equal). One year on one thread: island22 -2.35%, globe36 -1.45% of the instructions (`Body::activity` calls 639,897 -> 287,212); core 9,689 -> 9,689.
- E2.3, `nodes::add` in one pass, tried and not kept: the genes, their spreads and the gap of the born rows in one pass without temporary lists (`body::pool_genes`, the squared gaps in chunks so they vectorise, summed in order), the same bits. One year on one thread: island22 13,968.2 -> 13,883.5 million instructions (-0.61%), globe36 9,169.5 -> 9,134.2 (-0.39%), under the kill line of 2%; the floor is the 127 genes (about 30 instructions each: the walls, `max` and `min` of NaN-safe floats) and the 1,547 born rows (a sum that keeps its order), and it needs 18 more core lines. Branch `speed/e23-add-not-kept`.
- E2.3 (tape), tried and not kept: the recorder's month, its yearly read of each body and `layers::fields` on the pool (`par::rows`, sums in the old order, level 1; branch `speed/e23-tape-not-kept`). The tape stage on 4 threads 27.9 -> 20.1 ms (island22) and 14.5 -> 11.9 ms (globe36), 0.7% and 0.3% of a year (7 interleaved runs), under the kill line of 1% on both: the reads are many cache misses, and the event counts and the sums in order stay on one thread (`measures.md`).
- E2.3, PGO again on the code after D2: 1.1% (island22) and 3.8% (globe36) on 4 threads (7 interleaved runs), under S35's 8%. Not kept; no `scripts/pgo.sh` (`measures.md`).
- E2.3 (1): the booking once per number. A domain's view keeps beside each number its change since it was taken, and `Moves::book` adds it to the store once (with `Store::carry`, as `apply` adds) in place of one `apply` over the listed moves; the list goes, the lift of gas coming down is summed in `Moves`, a move of a store node outside the domain open panics. A year on one thread: -3.02% instructions (island22), -2.03% (globe36); the bodies' stage on 4 threads -5.4% and -4.9% (medians of 6 interleaved runs). Level 2: golden re-pinned; the order test's scenario cells moved. Core 9,689 -> 9,688.
- E2.3 (2): a fast path for gathering. The view takes its air node when its domain opens and the updater's cell when the update opens; `view_load` finds them first; `take` and `give` gather their one end of the store straight; the cold taking of a cell is `Moves::take_in`; `air_held` and the `mol` argument go; `add_carried` takes an optional carry, so `apply` and `Moves::book` call it alike. Level 1: golden unchanged. A year on one thread: -2.12% instructions (island22), -1.39% (globe36, under the kill line there). The rule it relies on, no store change while a domain is open, is checked in debug builds (`apply`). Core 9,688 -> 9,689.
- E2.3 review: the spec (handover 8 and 12) names `Moves::book` beside `apply` as the store's writers, both inside `store` (architect's decision); one helper sets rounding below zero to zero and counts it (`store::zero_rounding`, for `apply` and `Moves::book`), and the booking checks in debug builds that a number ends below zero only by rounding; debug checks that no heat moves to the bed and that a domain is not opened over one not yet booked; `upkeep` takes the activity its caller read (`upkeep_at` goes); `grow` makes its payer only with an allowance; S80 (the kill line of E2.3 as applied). Level 1 (golden unchanged). Core 9,689 -> 9,696.
- E2.3 review, after: the booking's end-of-batch check that a number ends below zero only by rounding goes again (`96a5f71`): it saw only a number's net change, so a gain and an equal take in one batch left it a bound near zero and it could fire on legal rounding; `alter` checks each take against the view's running value, which is stricter. Bit for bit.
- Room for D1 (b), store, life and survey: one `Moves::give_all` gives a body's or a part's amounts and heat to a node (`remains`, `to_ground`, `bond::dissolve`); the snapshot of `Life` is one struct, borrowed when written and owned when read (`Saved` with `Cow`, the bytes the same; `Loaded` goes); `Life::root` goes (only a test read it); `Faces::make` returns the faces it refilled; `Draws::of` goes into `drawn`. Level 1 (golden unchanged with and without threads). Core 9,693 -> 9,674.
- Room for D1 (b), body, copy and nodes: `Body::set_genes` and `Body::energy_in` go (the genes change in place, then `prepare`; `heat_with` of the whole store), and with them two clones of the genes; `body::to_growth` returns the `Lived` its caller finishes (`Growth` goes); `heat_bound` takes its two heats as a pair (`PerMol` goes), `Soil::liquid` and `body::spreads` go; `pay_at` walks a reaction's inputs then outputs by `change::terms`; the loops of `plan` and `digest` as iterators; `level_at` by guards; early exits joined (`check_stores`, `mix::partner`, `digest_room`); the unused return of `soak_in`, a needless clippy allow; `merge_closest` keys a pair in one place; `nodes::kin_of` inlined; `child_rows` without its match. Level 1 (golden unchanged with and without threads). Core 9,674 -> 9,617.
- Room for D1 (b), the rest: a stack's layers are the levels they lie on (`space::Layer` and the unused `Stack::dist` go: no law read the thickness or the distance); the marks of light after an air step by `Life::each_mut` (one pass, no search of a cell's list for each body); `Chemistry::burnt` goes into its one caller; the parts a body perceives and the reactions the reactor runs by filters of their loops. Level 1 (golden unchanged with and without threads). Core 9,617 -> 9,590.
- The shell writes the tape once at a run's end (the last year's write with its snapshot went twice, the second the same); the recorder's first month after a resume, before the first survey, works out the surface height of each cell once, not twice per body (`World::surface_z`). The tape the same: two years of island22 from a bench snapshot give the same hash, `series.csv` (timing columns aside), `months.csv` and snapshot; one year on one thread 12,933.6 -> 12,883.1 million instructions (-0.39%, all four steps). Core unchanged (9,590).
- D1 (b) not built (S81): its design needs about 170 to 280 core lines (110 free), changes more laws than S9 (a body with no last take gets nothing; food and digging cannot be reserved; the physics a step late), and could hide only Flow and Change (18.5% and 33.8% of a year), under its done line.
- E1 (S77, S82): the probe of a layout in update order (a domain's bodies by cell and id, and their lists made anew) was measured on a branch (`speed/e1-probe`) and not kept: the bodies' stage on 4 threads +7.3% to +14.8% (island22) and -0.8% to +3.9% (globe36), the update's data misses unchanged; columns are not built.
- Sweep review (`2e61551`): the bed and skin thickness are checked in `Config::validate` (the sweep had removed the stack's assert that its layers have a thickness; inputs are checked in one place); the kin test of `nodes::join_alike` is `kin_key`, not a second `kin` beside `copy::kin`. Bit for bit.
- The node-churn tests (`core/tests/populations.rs`) deliver the outbox after each update, as a batch's end does: since D2.4a the children that start in another cell wait there, so the churn without selection lost them (or let them miss its deaths) and selected on dispersal, and the test of dispersing children tested no landing.
- The audit of tests that call `copy::copy` or `choose::go` on their own for a missing outbox delivery found one more: `a_young_nodes_sizes_stay_between_birth_and_the_copy_mass` (`core/tests/copy.rs`) left 298 litters of a node in the outbox, so the members it sent to a neighbour cell never went back to its brood (the young node it measures is the same to 9 digits). The other calls make children in their own cell (the `DISPERSE` gene is 0) or deliver at once.
- The canopy check of the key numbers takes the densest 5% of the land cells with life (S83): since the foundation's decision 63 no land cell passes its old 20 kg/m², so it held no cell and failed; 1 of 80 canopies now moves more than 10 K.
- The sleep test (`the_sleep_of_change_keeps_the_chemistry`) fails on `air_3` only: every setting gives the air more gas 3 than every node awake, +5.8e-7 to +1.2e-6 against a noise of 5.5e-7, more with a longer sleep. Not from the speed work: the same bias is at the first commit (`f3527d1`, +3.9e-7 to +8.7e-7, under its noise of 9.9e-7); the test shows it since `679a6a3` (a soft cover melts at its top), whose steadier climate made the noise smaller. Cause: the sea's ground nodes. Their pair 22/23 moves about 20 times its difference each way; one long span takes each reaction's scarce input as running out while the other refills it, so it errs on each by about z/2 of what it moves (`Reactor::rest` bounds each reaction alone, z up to `CALM`), a large error on the difference: a span in one piece gives the air 3e8 to 5e8 mol a year more than in 10-day pieces (seed 1, 32 x 32). Tried and not kept on `main`: counting the other reaction's refill in the reactor's extent (mends 22, not 23; the test still fails); the calm share of a pair cut by their difference over their sum passes the test (off by 1.0e-7 at most) but the sea then wakes every step and the run takes as long as with every node awake (+20%): branch `sleep/pair-calm-proposal`, for the owner.
- Z, the final check (`60a05eb`; `docs/reports/speed.md`, `docs/speed-post-foundation/measures.md` section Z). Benches made again. A settled year on 4 threads: island22 0.614 s, globe36 0.738, region24 0.888; the baseline code in the same hour 0.910 and 1.490 s (1.48 and 2.02 times; the aim was 10 times); on 1 thread 1.072, 1.183, 1.589 s. The bodies' stage is 2.04, 1.87 and 2.10 times faster on 4 threads than on 1. 1,000 years: island 22 x 22 12.8 min (before 25.1), globe 36 x 18 14.3 min (36.6), refined region 31.1 min (86), globe 40 x 20 21.7 min; the largest globe that fits 15 minutes is 36 x 18 (648 cells, before about 290). Books: worst element drift 5.6e-16, worst energy drift 7.3e-14, no heat not given. Cells crowd on the globes (623 bodies in a cell at year 410) and the region's eaters multiply in the last 100 years (28% of its run). Core 9,591 lines. Docs: handover 5.4, 7.4 (new), 8.2 and 12.6, the plan's status line and `CLAUDE.md`'s table.
- Final review, the suite's time: the storage check (`Life::unsound`) finds each body's filing in one map of id to count, domain and ring slot, not by a search of its ring slot (n log n, not quadratic; `opening_a_node_draws_members_and_keeps_everything` 9.3 -> 2.4 s); the learning scene's seeds run as tasks of rayon's pool, not on threads of their own, so a world's parallel loops never wait on a thread busy with another seed (`a_body_that_learns_lives_on_what_it_can_take` 16.2 -> 6.6 s, the same digits). The normal suite 35.0 -> 18 s. Bit for bit.
- Final review, clean-up: `Counters::add` names every field of a domain's counts, so a new one cannot be lost (a domain keeps no counts per cell, checked in debug builds); the code's pointers to the plan's section 3 point to `docs/laws.md` 0, which describes what they name. Bit for bit.
- Final review, docs: the spec states only present numbers (the recorder on the pool, not kept, and the speed work's before and aim go; `docs/reports/speed.md` has them); the report's caveat on the "before" runs names the right runs (the 22 x 22 island and 36 x 18 globe of `foundation-l.md` ran before its last three fixes); D3's share of a year; the three rules for smart bodies and the missing `nodes::may_join` among the open items; the slow suite and the nodes check ran only at Z2; the decisions that live only on branches are named.
- Final review, the baseline in turns: the baseline code (`16e6a4e`, its old saves) and today's code ran in turns, five sets on 4 threads and on 1 thread, on a quiet machine (S45; Z1's baseline was three sets after the new code). A settled year on 4 threads: island22 0.943 -> 0.664 s (1.42 times), globe36 1.553 -> 0.805 s (1.93 times); the bodies' stage per update 1.78 and 2.06 times cheaper (`measures.md` Z1, the report's table).

## 6. Speed x10 (from 25 October 2026)

- The plan (`docs/speed-x10/plan.md`; `decisions.md` X1 to X35; designs in `design/`), after the adversarial review: the plan is the single source of the steps (each with its level, done-when, kill line, needs and agent), and each design says which of its steps are kept, dropped or renamed; P1b gets its design (`animals.md`: food is a pulse at the interval's start, deaths and the reserve go by the gene's interval); the gap's steps (G1 to G16, G14-P, GM) are placed in phases and lanes; probes come first (PR-M, PR-B, PR-A, PR-S, G14-P1, AIR4-R1); the ceiling is 12,000; MI6 gives way to G2, AIR6 to G11, MI8 is dropped; the gains are milliseconds of a year on 4 threads per step (island x2.5 after phase 1, x10.9 to 12.8 with every step, about x6 to 7.5 without G14 or AIR4). No code changed.
- The plan after the second adversarial review (`plan.md`, `decisions.md` X1 to X36, `design/animals.md`, `gap.md`): `main` is merged into `speed/x10` at each lane review and before Z (X1); keep-warm is a steady drain inside P1b's upkeep, and G3 keeps only the heat's closed form (X14); P1.2 and P1b announce their law changes (a deep famine kills `1 - e^-u`; stands' reserve by the gene, 1.8 to 3.6 times smaller; PR-M runs `D` = gene against 36 and the architect chooses); G2 needs P1b and MI5, not MI3a, and MI3a and MI4 are gated again after G2 (X36); `field_days` is 30, 60 or 90 (X17); G5's fallback is expected (X18); G14 cuts each reaction as a whole and its bound counts the serial parts (island 79 to 98 ms, 9.8 to 12.2 times; globe 13.1 to 16.3, X21); G8 is level 1 for the world, 2 for the tape; section 5 gets a "likely" column (island about 8.6 to 10.5 times, globe about 11.8 to 14.7); the ceiling 12,000 goes past handover 14.3's 20% with the owner's yes (X22). No code changed.
- R1, the rules true at once (`docs/speed-x10/plan.md`, X3, X4, X22): the core ceiling is 12,000 code lines (`CLAUDE.md` 2.11, handover 14.3, `cli/tests/core_size.rs`; past the 20% of 14.3 with the owner's explicit yes); the books are 64-bit (`CLAUDE.md` 3.1, handover 12.1); the number of threads does not change the world, every order is fixed by the data (`CLAUDE.md` 6.1, handover 12.3, 12.5, 14.5); the table of `CLAUDE.md` names the work now.
- R3: the liquid's clock counts a `flow_days` over a month from the world's start (`schedule::days`: `within` is the longer of the period and the month), as creep does, and `Config::validate` refuses one that is not a whole number of months (before, `Every` cut it at the month and the liquid still ran monthly); the long-step test runs `phys_days` 2, not 3 (3 does not divide into `batch_days` 2), and the shared run helper of the tape's tests calls `Config::validate`. Golden unchanged at `flow_days` 30.
- AIR4.0, a law fix (probe AIR4-R1): a ground node that melts or freezes all of its plateau in one air step stays on it only for the share of the step its heat takes, then goes on from the melting point; a soft cover that melts away caps its surface only until it is gone (before, the plateau and the cap held for the whole step, and the restart point had the wrong sign). The erosion test keeps the ground's temperature when it strips the loose solids. Golden re-pinned. Long-step test, off from the short steps: t_land 0.145 -> 0.118 K, summer 0.158 -> 0.133 K, winter 0.026 -> 0.002 K. Two years of a 32 x 32 island: land 0.05 to 0.14 K cooler, a little more ice. Core 9,605 -> 9,606.
- G14-P1 (`measures.md`): the overlap of the air and the batch saves 15% (island) and 13% (globe) of a year; the owner set G14's kill line to 10% of a year (X38, was 15%), so G14 goes on to G14-P2. Decisions X37 (steps that need no probe start beside phase P) and X38.
- AIR1, the air's tolerances (level 3): a ground node whose air moved less than `CALM_K` (0.01 K) and `CALM_SHARE` (1e-4 of its gases) since its row keeps that row in the later rounds, and the rounds stop at a gas move of `STILL_SHARE` 2e-4 (was 1e-5). Globe36: 4.12 -> 3.25 rounds a step, cell rows -28%, Flow instructions on one thread -11.7%; the island and the region do not change. Long-step test: land +0.131 K from the short steps (limit 0.3); globe test: land 285.88 K, ice at most 0.104. Key numbers inside their noise. `CLAUDE.md` 6.5 makes the air's tolerances speed numbers. Golden re-pinned. Core 9,606 -> 9,618.
- E1a, per-domain counts of the engine step: the energy movers spent in a cell (`pushed`) and the members that moved into it (`crossings`) are kept by the cell's domain (`Domain::pushed`, `Domain::crossings`, saved with it; snapshot version 48), not in two world-wide lists; the booking of moves and the taking of outboxes visit only the domains that ran since the last (`Life::mark`). Bit for bit: the worlds of 300, 200 and 200 years from worldgen and two more years from each bench give the same series. Golden re-pinned for the new format only. One thread: the simulation's instructions island22 +0.007%, globe36 -0.008%.
- PR-A (`measures.md`, branch `probe/pr-a`): with today's law a 30-day floor for animal nodes loses 31%, 17% and 33% of the eaters in 2 years (island22, globe36, region24; seed noise 1.1, 6.4 and 1.7%); the suspected causes (a) to (d) bring back nothing, the bulk take (f) costs about 5% of the food; a new cause, the eaten mineral turning solid in the body before growth, is a third to a half of the loss on the island and the region; the rest is the adult nodes' cycle (children skipped under the copy mass, starving). A 20-day floor also fails. A floor on young nodes only, with the mineral kept, keeps the eaters. G2 waits.
- P1.1, `pay::apply_extent`: one helper books the extent of a reaction on a body (its inputs, then its outputs: gas to the air with its latent heat, the rest to the stores); `pay_at` calls it. Bit for bit.
- MI1, the cover of a face once a batch: `Faces::make` works out `Face::show` = exp(-cover) for each face, and `choose::perceive` reads it (under no cover, `UNCOVERED`, nothing is drawn, as before; a cover so thin that exp(-cover) is 1 still draws). Bit for bit. island22, one year, one thread: -0.46% instructions (under the kill line of -0.5%).
- G8, the tape on the pool: the recorder's walks over bodies and cells (the month's look at the bodies, the world's fields, the costly numbers of each body) run on the pool and are added up in the order of the data; counting the events and looking at the bodies, the fields and totals, and the yearly columns that only read the recorder run side by side (`tape/src/par.rs`, the feature `threads` of `tape`). The tape and the world are the same bit for bit. The tape's time a year on 4 threads: island22 10.07 -> 6.30 ms, globe36 11.07 -> 6.89 ms (-37%, -38%).
- MI1 kept at -0.46% (its kill line -0.5%; X54).
- The month law after phase P and its review (`docs/speed-x10/design/month.md`, decisions X41 to X53, plan rows P1.2 to P1b, G2, G3, G9a and section 5.0): the month is a plan worked out on numbers and then one booking of its net change per substance; the selection sample runs only the plan, in a scratch body; the fuel keeps X9's closed form with product inhibition and the release limit inside it, and the body's other reactions (the bare ones too, with their own orders) and growth run beside it in sub-steps by the reactor's kernel; growth keeps the upkeep of the rest of the interval, and starving is judged on the body's path; every rate from Change's law is read over the body's window `D` (36 days for stands, the gene for bodies with a brain); P1.3 is folded into P1.2; the month's own splitting test is its reference; G2's fallback is a young-only floor if the mineral is kept, G3 is gated again; a stand update is counted at 10 to 10.5 µs, not 6. Section 5 restated: likely island 5.7 to 8.6 times, globe 7.5 to 12.3 (the span of G2's two cases until PR-A2 lands). No code changed.
- The month law after PR-A2 (`docs/speed-x10/design/month.md` 5.1, 5.2, 7; decisions X55 to X59; plan rows P1.2, P1b, G2, G3 and section 5.0): a long update is the sum of the body's own gene intervals, in segments where it burns, grows and starves, and at whose boundaries its young pass to the adult node and its brood makes children; a body that takes food is visited at its gene's pace for a take only, sized by the days to its next visit, and its update's month starts a segment at each take; only the brain runs once an update (G2 floors the brain: an animal node's updates cost x0.39 and x0.42, not x0.30); G2's fallbacks are 20 days, then young-only, on the segmented law; P1b's balance exact in time is accepted as its law effect. Section 5 restated: likely island 5.9 to 8.3 times, globe 7.7 to 11.3. No code changed.
- The month law after its fourth review (`docs/speed-x10/design/month.md` 3.1, 5.1, 7; X53, X55, X57; plan rows P1.1, P1.2, P2, G9a, P1b, G2 and 5.0): growth keeps the upkeep until the next food (the next take's segment), not to the update's end; a stand's segments are of at most 36 days (today's cadence of its maturing and births), not of its gene interval; pulse rows hold every substance and heat a take added; passers and children get the marks of their day; one mutation draw per boundary; P1b counts growth bound by the reserve and by a safe minimum; P1.1 is done and no need of the month. Likely island 5.8 to 8.1 times, globe 7.6 to 11.3. No code changed.
- The month law after its fifth review (`docs/speed-x10/design/month.md` 3.2, 5.1, 7; X46, X55; plan rows P1.2, P2, G9a and 5.0): a stand's maturing and births come at the points of its own 36-day grid, counted from its birth (a phase number per stand), not at pieces cut from its update, so it keeps today's cadence at every `field_days` (P1.2's announced 36 to 30 days change is gone) and the splitting and pace tests compare like with like; a stand update about 10.5 to 11 µs at 30 days. Likely island 5.8 to 8.1 times, globe 7.7 to 11.3. No code changed.
- The month law, last notes of its review (`month.md` 5.1, X55, plan P1.2): a body with a brain's grid is the days of its updates and visits, and follows them when its schedule shifts; a stand's phase is set from its first scheduled update (or its id) at worldgen and in turned saves, and a merge keeps the receiving node's phase; P1.2's done-when reads about -9% and about -30% per stand-year. No code changed.
- E1b.1, faces per domain: a domain makes the faces its bodies show in the next batch at the end of its own task (`Domain::made`, `survey::faces`), and keeps them while nothing they show is written (`Domain::touched`, set by any write to a body, `Domain::quiet` is the write that no face shows) and while each cell's canopy is the one they were made with; before a batch only the other domains make theirs again, on the pool. Bit for bit. One thread, instructions of a year: island22 +0.71%, globe36 +0.34% (a made face that is touched later is made twice: 41% of the domains are made again at a batch's start on the island, 42% on the globe, 52% on the region).
- MI5, a node's movers leave as one group (a law change, level 3): the members that move are `count` times the sum of the chances to go in every direction, in the one direction drawn by its chance over that sum (before, a part for each direction), and `can_go` (room in the cell, force for the weight) is asked before any movers are split off; `go` uses it too, so a move refused for room or force changes nothing (a body lets go of what it cannot carry only once it can step); a move refused at payment still lets go first. Mean flows are the same; the movers come in groups. Globe36, one year, one thread: instructions -2.85%. Golden re-pinned.
- AIR1's kept rows made true (level 3): a calm ground node's row is now carried to the round's air by its own slopes, so it stays linearised about the air it was made at (before, its values were taken as made at the new air); a row ends with the air it stands at, each active gas apart, and each gas is tested against its own amount (before, the sum of the active gases). Globe36: rows -29.0% (AIR1 -28.8%), Flow on one thread -10.8% against the air of before AIR1 (AIR1 -11.5%). Long-step test: land +0.125 K; globe test: land 285.84 K. Key numbers inside their noise. Golden unchanged (no calm row in its runs). Core 9,680 -> 9,708.
- The review's small fixes of P1.1, MI1, G8, E1a and AIR1 (bit for bit): the row of one ground node moves from `flow/air/step.rs` to `flow/air/cell.rs`; `Life::take_queued` checks in debug builds that only a domain that ran has an outbox; the tape tells land by `World::is_land` everywhere and takes the world's totals only for the yearly row; the tape's `map`, `map_n` and `join` move into `pcore::par`, so one feature `threads` (of `pcore`) runs everything on the pool and the tape has no rayon of its own; words in `decisions.md` X39 and `measures.md` set right. Core 9,708 -> 9,746.
- E1b.2, landing by destination: a body with a move queued leaves its domain's storage when the domain's task ends (`Domain::post`); the moves and litters land per destination domain on the pool (`outbox::land`, in source order then order made), which then makes its faces; lane clears, pushes on structures and litters going back are queued by the landing and applied one by one after it (`Entry::Passed`, `Entry::Back`), after the watches and part acts; `World::settle` notes where moved bodies are stored. Level 3: golden re-pinned (the order of landings and events changes), the order test widened, key numbers in range (S50 known). One thread: the delivery left on the main thread falls 60 to 88%; the 4-thread kill line could not be judged on the busy machine (`measures.md` E1b.2).
- MI5's whole-node mover (a law change, level 3): when a node's group of movers would leave fewer than one member behind, the whole node moves with the chance that keeps the mean of the movers at count times the sum of the chances (before, the group became the whole node every time it was drawn, so nodes under 2 members sent more movers than their chances say). Globe36, one year, one thread: instructions 7,259.9 M before and 7,260.7 M after, so MI5's gain stays. Golden re-pinned.
- P1.2, the month law for stands, tried and not kept (branch `x10/p1.2-not-kept`): a stand update cost about 26 µs (173k instructions against 121k for today's law on 32 x 32), so a stand-year cost more than today's at 30, 60 and 90 days; the cost is in the plan's stages, the booking and the sample, not the fuel's closed form; its key-number moves (land -0.78 K, S50 0.636) were not explained (`docs/speed-x10/measures.md`). No code changed.
- E1c, booking by array beside the landing, one runner: the moves the domains gathered are booked by one helper, `store::book_by_array` (each block of the store's arrays one task, every domain's changes in domain order; a class's amounts in one task when the booking is small), in the same pool call as the landing (`World::book_moves_beside`; the store is out of the world meanwhile); per-domain work runs on one runner, `survey::run` (longest first, a scratch per thread with `par::each_init`, a phase under `par::SERIAL_BELOW` 8 items on the calling thread); `CLAUDE.md` 3.6 "one writer for each number". Level 2: in each round of landing the moves gathered before are booked first (a round that landed nothing applied its entries over open views before), so the benches' worlds move to rounding (climate columns within 1e-4 over 3 years); golden unchanged. One thread, the same world: the run -0.88% (island22), -1.07% (globe36), -0.68% (region24) instructions; the delivery and booking left on the main thread -50%, -16%, -42%. Core 9,957 -> 10,057.
- Lane A replanned after P1.2 (`docs/speed-x10/design/lane-a.md`; decisions X63 to X70; plan rows P1.2 to P1b, PR-C2, P2.1, P2.2, G2, G3, G9a, MI3a, MI3b, GB and section 5.0): plants keep today's day law at a longer pace (60, then 90 days) with C2's causes fixed inside it (the time's income pays the upkeep before members die; they die at a rate over 36 days; growth keeps 36 days of reserve; if the pace test needs it, growth, maturing and births on the stand's 36-day grid), after a probe (PR-C2); G2 floors the brain on today's law (a node decides about every 30 days and takes by its last decision between; its body keeps its gene's pace, as PR-A2 measured); P1.4, P1b, G3, MI3a and MI3b are dropped. Likely island 3.9 to 5.2 times, globe 5.6 to 7.6 (was 5.8 to 8.1 and 7.7 to 11.3), with the decision and the take measured after MI5 (`measures.md` "Lane A replan"). No code changed.
- AIR3, the air's book as column moves (level 2): the air's step books what it moves through `store::book_by_array` (each block of the store's arrays one task, the ground's gas going up into its own air node and raining out of the four it sees, then the heat, the bodies' gas, the wind and the lifts) in place of a `store::apply` call with a link list; a move of zero is not added, which changes when a carry of the ground's or the air's amounts is folded, never its value; `Store::add_far_heat` adds the sun's, space's and the potential's sums once. Golden re-pinned; climate columns within 2e-4 over 3 years from the benches, hash the same on 1, 2 and 4 threads. One thread, the air's `book` -42% (island22) and -50% (globe36) in instructions; the wall time on 4 threads is not measured. Small checks of AIR1's kept row: a carry that pushes the rain below zero, a gas move alone, the drift error per 0.01 K.
- PR-C2 (`measures.md`, branch `probe/pr-c2`): the day law at a longer stand pace fails the pace test's x2 and the splitting line in every variant; stands stay at 36 days; P2.1, P2.2, G9a, P3, G13 not built; PL1 (a cheaper stand update) is the alternative (X71).
- G2, the brain floored (a law change, level 3): a population node with a brain keeps its gene's update and its body's whole update, but decides only at the update nearest to `think_days` (20, X72) after its last decision; at its other updates the shares of its members that took from the ground and from each body of its cell then take again (`choose::Habit`), and it does nothing else; at a decision its movers are `count x min(1, Σp x paces)` and learning discounts by `discount^paces`, `paces` the gene's intervals since the last decision. Snapshot 49. One thread, the same world: an animal update x0.898 (island22) and x0.861 (globe36), the year -3.6% and -6.8% (at 30 days x0.863 and x0.803). It slows the eaters' spread in a new world: the key numbers' eaters 5.226 log10 kg at 20 days and 5.076 at 30, against 5.393; settled worlds stay within the noise. Golden re-pinned. Core 10,173 -> 10,227.
- AIR2, the air's loops on the domain runner, tried and not kept (branch `x10/air2-not-kept`): the round's rows made per air node on E1c's runner (moved to `par::longest_first`) with each node's sums in its own part, added in node order; `seen` and `dissolve` per air node in one more pool call were tried and dropped (no speed-up). Flow on 4 threads x1.06 (island22) and x0.97 (globe36) against the kill line x1.15; one thread +0.8% and +5.4% in instructions (`docs/speed-x10/measures.md` AIR2). No code changed on `speed/x10`.
- Fixes of the review of G2 (level 1, the world does not change): `discount` is per interval of its gene, not per decision (doc of the field and its `numbers.md` row); choose's `paces` (gene intervals since the last decision) is `since`, so `Body::paces` (intervals one update spans) is the only `paces`; a body resized to no brain also clears its habit; `Habit` has no `Default` and its fields are private to `choose`; the doc of `Body::habit`; a test of the law (a node decides every `think_days` and takes between as much as when it decides at every update); the take-path criterion of G2 judged against a decision's takes, and the globe's body takes at 20 days in `measures.md`.
- G1, Change per domain (level 1, bit for bit): the due ground nodes react in tasks of the nodes under one air node on the one domain runner (`par::longest_first`, which `survey::run` now also calls), each into a row, and book nothing; the rows are booked after the call by `store::book_by_array` (each array of the store one task, in the order of the nodes, so an air node's gas sums as before), in place of a `store::apply` call with link lists and transforms; the air nodes react in blocks of 8 the same way. `World::batches`, `change::Batches`, `Batch::clear` and `Batch::is_empty` are deleted; the check that a node's reactions keep the elements is a debug assertion. Golden unchanged; the three benches give the base's hash and `series.csv` on 1, 2 and 4 threads. One thread, `change::step` -19.5% (island22), -18.3% (globe36), -19.6% (region24) in instructions; Change on 4 threads (real-time priority, 20 runs) -42%, -38%, -46% by the medians: under the -50% of the plan, over the -30% kill line, because the base's rows already ran 2.1 times faster on 4 threads (the gain is the booking). Measured on `284c6ba`, then rebased on `8370448` (G2), bit for bit there. Core 10,227 -> 10,353.
- The review's fixes of AIR3 (bit for bit): the air's `book` checks in debug builds that no domain's view is open, and `store::book_by_array` checks in debug builds, as `apply` does, that a number below zero is only rounding (one shared check, `apply::overdraw_check`); `Store::add_far_heat` writes only a boundary node or the potential, and `apply` adds its far sums with it; one helper for the heat a link's far end receives (`store::less_held_more`, used by `LinkFlux::lifted` and the air's book); `Booker::column` adds a column of moves and skips its zeros, so the air's `Columns` is one `match`; `Counters::add_rounded`; `#[inline(never)]` gone from the air's `book`, `Booker::add` always inlined. Golden unchanged with and without threads; globe36, one year, one thread: instructions -0.07% (the air's step -0.28%). Core 10,353 -> 10,347.
- E1d, faces once a batch, one settle: every domain makes its faces at a batch's start, one after another on the calling thread (`survey::faces::make`); the faces made at the end of the landing, the remake of stale domains on the pool and the staleness marks (`Domain::touched`, `Domain::quiet`, `Domain::made`) are gone; the world settles once after the landing call, not once per landed domain. Bit for bit (golden unchanged with and without threads; the benches' fingerprints unchanged). One thread: the run -0.72% (island22), -0.41% (globe36), -0.94% (region24) instructions. Four threads, 20 sets on a shared machine: the bodies' stage -0.3%, +1.1%, -3.6% (inside the noise on the first two). Core 10,060 -> 9,994.
- The engine chain E1a to E1d is kept with a small gain (X74); E2 is gated again after G4 (X75).
- G7, Bond's month per domain (level 2): the parts' monthly turns run per domain on the one runner (`survey::run`, which now gives its work as `&mut`), each domain's parts in slot order on its view of the store, so a part reads the air as the parts before it left it (before, from the store after each part's booking); a part whose walls are gone gives all it holds to its ground in its turn and is freed after the turns in slot order; the moves are booked once at the end of the turns; a month that only settles counts no work, so it stays on the calling thread; `World::book_moves` (no landing beside it) books on the calling thread, the pool only for `book_moves_beside`; `Moves::t_air` is the one air temperature as a view leaves it (bodies use it too). Golden: seed 2 re-pinned, seeds 1 and 3 unchanged, the same on any number of threads; the benches' year 1 equal in every series column, later years to rounding and life's choices. Globe36, 4 threads, `ms_parts` -44% (median of 12 runs), region24 -29%, island22 -4%; one thread, Bond's month -35% in instructions. Core 10,281 -> 10,307.
- The review's fixes of the engine chain (bit for bit): the docs say moves and litters land per domain on the pool and only watches, part acts, `Passed` and `Back` are applied one by one; the three test-only functions of `Life` (`cross`, `pushed`, `relocate`) are gone (the tests use `Domain::cross`, `Domain::pushed` and a move landed by `testkit::relocate`); the snapshot shape moves to `life/saved.rs`; `outbox::land` checks in debug builds that no landed domain has an open view.
- The review's fixes of G1 (bit for bit): in debug builds `book_by_array` measures a number below zero against what its node held of all substances before the booking, as `apply` measures it against all of the node (it measured only the task's 4, so the debug checks failed a fixers test on rounding); Change's booking checks in debug builds that no domain's view is open, adds its rounding with `Counters::add_rounded`, adds its columns with `Booker::column`, reads the count of nodes from the booker and the air node from the block; one helper for the heat a move's or a link's far end receives (`store::heat_received`); the dead `store::Transform` path of `apply` and its tests are deleted, and `keeps_elements` is a method of `Chemistry`. Golden unchanged with and without threads. Core 10,281 -> 10,267.
- The spray of the genes is passed in decades (`GENE_SPRAY_DECADES`, log10 3) and not as a factor: the release build folds `log10` of a constant at build time, and the library `log10f` of the same number differs by 1 ulp, so a build without thin LTO ended on another world. Bit for bit in release (golden unchanged); a plain debug build now ends on the same pinned fingerprints. The re-pin command of the golden test names `--release`.
- The review's fixes, round 2 (bit for bit): `testkit::relocate` takes a neighbour cell only and says so when it is not one (a move lands in the cell's crossings and pushes the structures there); the two tests that moved a body to a cell by chance now move it to a neighbour; `Life::domains` and `Life::cell_slot` are crate-only again (the tests read a domain with `Life::domain` and write its crossings inside `World::in_domain`, which marks it).
- G4, batches of 10 days (a small law change, level 3): `batch_days` 2 to 10, one air step (X17, replaces S64), and `node_days` 36 to 40, a multiple of 10 (X76: 30 would cost 5 to 9% of the bodies' stage more than today on one thread). A gene of 5 to 15 days updates every 10 days, and a batch's bodies read the ground and air as that day's Change and air step left them. The bodies' stage on 4 threads x0.83 (island22), x0.78 (globe36), x0.79 (region24), the year x0.90, x0.85, x0.82; on one thread the stage -6.9%, -7.4%, -8.3%. Key numbers in range but S50; eaters 5.217, the light-eaters' life 1.42 years (1.24 at G2), from the pace of 40. The batch test (now 2 against 10 days) fails on its seed 1 at node 40 by that seed's tight noise (1 against 2 fails there too); open for the architect (X76). Golden re-pinned; snapshot unchanged (49). Core 10,283 (+2 -2).
- G4's batch test at node 40 is a known failure on its seed (X77); its design is reviewed in phase 2.
- A part that falls apart by a take keeps the books (a law fix of G7): G7 moved the gift of what a falling part holds to its ground into the parts' turns, so a part a body took under `min_kg` (`verbs::take`) gave nothing and its rest of walls, hollow liquid and heat vanished (on a 5 t part, element drift 7e-12, energy 1e-10). Now one helper, `bond::fall_apart`, gives all a part holds to its cell's ground in the part's own update and empties it, on both ways (its turn, a take); `dissolve` only frees it. The booking of the domains' views (`World::book_moves`) moves to `store/book.rs`. Golden unchanged with and without threads. In 5 years of the benches every part a take broke was taken whole (island22 186, globe36 1,216, region24 510, none with a rest), so their worlds do not change: the fix counts where a take leaves a rest under `min_kg`.
- The review's fixes of G4 (tests and docs, golden unchanged): the batch test is judged on the mean of seeds 1 to 3 against the noise of that mean and passes at node 40 (X77, no known failure); the long-step test's short run has batches of 2 days, as a batch divides the physics step; the evaporation test runs its update in the body's own batch; the growth test lays fuel that does not rot and says why it keeps batches of 2 days; X76 gives S50 at 40 against 30 with its reason, the split of the light-eaters' life between batch and pace (key numbers at batch 2, node 40), and G4's accepted law effects (`laws.md` 0 and 5.3, `numbers.md`).
- The growth test runs at the default batch of 10 days: its year starts at the body's first batch (G4 review, round 2).
- G15, the sharing between threads (level 1): `ew` uses mimalloc as its allocator; glibc's frees of blocks made on another thread (moved, merged and dead bodies) went through locked arenas. On 4 threads the bodies' stage x0.81 (island22), x0.86 (globe36), x0.86 (region24), an update 1.09 to 1.11 times the 1-thread update of before; shared cache lines between domains, `par_bridge`'s lock, spinning threads, more spin, lingering threads and domains kept on one thread were measured and not kept (`docs/speed-x10/measures.md` G15). No core code changed.
- G5, the survey and Flow's state every 30 days, tried and not kept (branch `x10/g5-not-kept`): Change at 30 days moves the land's mineral of the rock past its noise on 3 of 5 seeds (the span itself, not the sleep), so the split kept Change at 10 (`change_days`, a new field) and gave only the survey and the state to 30; that saves 0.9% of a year's work on one thread (most of the survey is the monthly merge, which runs once a month anyway), under the kill line of 3% on 4 threads. The long-step, globe climate and key-number tests passed there; the sleep test's S84 did not grow (`docs/speed-x10/measures.md` G5). No code changed; `phys_days` stays 10.
- G12, the liquid every 60 days (a small law change, level 3): `flow_days` 30 to 60. The liquid costs half (x0.50 in instructions on the three benches), the Flow stage on 4 threads x0.82, x0.82 and x0.71; the key numbers stay in their ranges but S50 (the known failure); the golden test is re-pinned; the clock test no longer expects the liquid in every month. A 60-day liquid warms the land by about 0.2 K against a 2-day one (`docs/speed-x10/measures.md` G12).
- AIR4, calm months by TR-BDF2, tried and not kept (branch `x10/air4-not-kept`): a TR-BDF2 whose two stages both start from the step's start (one booking, as `air.md` 2.4 designs it) is no better than Euler, because what a stage takes from the state at its start holds the forcing back; with its first stage booked and the state made again it is (one month's land error 0.10 K against Euler's 0.72), but at 30 days in every month the long-step test fails in winter (+0.51 K), the error estimate (the weather's step-to-step noise, 1 to 2 K an air node) chooses no month, and Flow on 4 threads stays under the kill line of x1.4 on the globe unless every month is calm (`docs/speed-x10/measures.md` AIR4). No code changed; the air keeps its 10-day Euler step.
- The slow suite at the end of phase 1 passes but for the known S50 and S84 (`measures.md`).
- G6, the survey per domain (level 2: golden seed 3 re-pinned, the world the same): one pool call, a task per domain, does in each cell the wear of movers, the monthly merge, the shade and the sums (`Life::sums`, per domain, read with `Life::sum`); a death or a merge that lets go of a part queues it in the outbox, applied after the call, so the cells with parts need no serial step; the `state` process leaves a month's start to the survey when it would be in full there (it skips when Flow's working memory notes a full build at that time, X83); the merge of nodes makes the genes in steps with the step of each gene made once. Every column of `series.csv` equal over 3 years on the 3 benches; the fingerprints differ only in the numbers of the born brain rows (they get their global ids in another order; fixed by the data, the same with and without threads). One thread, the survey -26.0% (island22), -22.0% (globe36) in instructions. Four threads, 14 sets: the survey -27.9% (island22), -40.6% (globe36); the done-when -35% is met on globe36 only. Core 10,297 -> 10,329.
- G14-P2 (`measures.md`): the overlap saves 9% (island) and 3% (globe) of a year, under 10%, so G14 is not built (X82).
- G11, the air's cell rows (level 2): a row finds once what does not move inside it (the air a node sees, the layers' glow, each active gas's evaporation constants in `Vapour`, its partial pressures; no heap copy per reflectance guess; no draws asked where no bodies draw); the end reflectance reads the node's amounts once and its melting points once (`land::melts`, shared by the plateaus and the cover), and the plateaus sort substance numbers; the bucket's slope is its derivative, not a second bucket with a nudge (the only change of the numbers; golden re-pinned). No f32 or SIMD: the surface balance's arithmetic is under a tenth of the row. One thread, `pass` -28.0% (island22) and -29.4% (globe36) in instructions, the year -1.7% and -2.8%: over the kill line (-20%), under the goal (-35%); AIR6 not tried (its gate fails). Core 10,297 -> 10,344.
- The gates of phase 2 (`measures.md`): E2, E4, E5, G9b, MI4 and G16 are not built (X84).
- PL1.1, the sets of a few substances as bits (level 1): the chemistry makes once the sets of the substances others dissolve in, of the built solids and of the light reactions; a body visits only the bits set (its key liquids, the solvents among its plateaus, the liquids of the ground); a substance that dissolves nothing melts at its own point, so its melting point among others is not worked out. A stand update one thread -6.5% (`measures.md` PL1).
- PL1.2, one look-up of the view per node (level 1): a body reads the amounts of its ground and air node and what it sees of the air through one look-up of the store's view (`Moves::amounts`, `Moves::seen`), not one per substance. A stand update one thread -1.1% (`measures.md` PL1).
- PL1.3, what goes to the ground (level 1): `Moves::give_all` finds its node in the view once; `thin` and `wound` cut the remains to what the body holds in place, and `shed` keeps its amounts on the stack and skips what stays (gas, body matter, fuel) before it works out a level. A stand update one thread -0.9% (`measures.md` PL1).
- PL1.4, values worked out once (level 1): a payer keeps the cover over the body and the rates of its burnings, so holding heat and each plan do not work them out again; a part of a node is placed with the constants of the node's genes, which it shares; evaporation reads the heat capacity once a substance; a node that members join pools its genes and spreads in place. PL1 in all: a stand update one thread -10.4% (island22), -10.7% (globe36), -10.5% (region24); the year -6.6%, -6.0%, -7.3%; the kill line (-3%) passed, the done-when (-15%) not met (`measures.md` PL1).
- PL1 fixes (level 1): the free function of a body's heat conductance to the ground is `ground_conductance` (it shared a name with `Body::conductance`) and the tests call it; one helper, `Chemistry::lowers_melt`, says which substances melt below their own point; a payer keeps the rates of its burnings in a fixed array (`MAX_REACTIONS`, 64, checked by `Chemistry::validate`); soaking looks at the ground once.
- The long-step test covers the liquid's step too: its short run sets `flow_days` to 2 days (X85); the default steps are 0.29 K off on land. G11 is recorded as built without f32 or SIMD (X86). Nits: `film`'s unit, the `seen` argument of `cell`, a case of the bucket test, `--flow-days` help, a wrapped test comment.
- The pace test is judged like the batch test (tests only, level 1, golden unchanged): `the_pace_of_nodes_keeps_life` runs seeds 1 to 3 (the stands' pace is its `node_days`) and judges each setting's seed mean against the reference's, with the noise of that mean; the two tests share `seeds_mean_by_setting`. At the defaults it passes (the light-eaters' life moves under 0.01 years at 20, 40 and 80 days against a noise of 0.17). The noise is the year-to-year range with the trend, 66 to 84% of the value for plant members, kg and births, so the test is loose on those three (named in the doc of `off_by_noise`).
- `individuals_mode_merges_no_animal` runs seeds 1 to 3 and asks for births in them together (seed 1 has none since G11's rounding moved its founders).
- The report of the round (`docs/reports/speed-x10.md`): a settled year on 4 cores x1.77 (island22), x1.45 (globe36), x1.44 (region24) against the round's start, about x2.6 and x2.9 against the last round's baseline, not x10; 1,000 years in 5.6, 5.2 and 11.8 minutes; the largest globe in 15 minutes has size 28 (1,568 cells); books under 2e-14. The core ceiling is 10,643 (the 10,443 lines plus 200, X88); the round's handover is rewritten to its end state.
- Review nits (tests and docs, level 1, golden unchanged): the habit test uses a brain whose chance to take is one half (`testkit::Brain::weights`, `eater_of`) and asserts that the habit's takes follow the saved chance to 1e-3; the outbox test reads `Domain::slot` (public) and `Domain::cells` is `pub(crate)` again; `bond::take` checks that the part lies in the body's cell (debug), and the fall-apart test also ends with the body's death on the reopened view; `measures.md` says when the debug-assertion run began and when `Transform` was deleted.

## 7. Lighter world (from 27 October 2026)

- The plan (`docs/lighter/plan.md`; `decisions.md` L1 to L35; designs in `docs/lighter/design/`), after the first adversarial review: compute less in six lanes (F the tape and saves, B living bodies do not rot, its law chosen by probe PR3 then a design review, C the cover, D the take and the view, E wider air nodes on globes, G the parts' reactor), probes first (R0, PR1, PR3, PR6, PR7, PR8, C0); ideas 2 and 4 wait for the cover (the owner's word for idea 2; A3 and A4 only if the cover fails); several plant kinds per cell that do not mix are a must for the cover (the owner's word).
- F1, the snapshot is timed (level 1, golden unchanged): its write is added to the tape's column (`ms_tape`), so a run's "per year" holds every save; the island at 300 years pays 1.1 s for 30 snapshots (3.7 ms a year, 1.7% of the run). PR1 shows that the tape's files, not the snapshots, grow with the run (`docs/lighter/measures.md`).
- F3, the tape's files written cheaper (level 1 for the world; the files hold the same rows): the viewer's file is gzip level 1 and its header is written from the recorder's data without a JSON value tree; `series.csv` and `months.csv` get only the new rows, and are written whole when their header changes (`tape/src/rec/csv.rs`). The writes of a 1,000-year island run fall from 24.9 s to 5.6 s by the run's own timers, about -5.8% of a quiet run (`docs/lighter/measures.md`).
- F2, the saves on a writer thread (level 1, the files are the same bytes or rows): the main thread makes the snapshot's bytes (into a buffer it keeps) and copies the tape; one thread compresses and writes them, one job at a time (`cli/src/writer.rs`). The main thread's part of the saves and the tape in a 300-year island run falls from 1.55 s to 0.15 s of CPU time (2.4% to 0.4% of the run, an upper bound of the gain in wall time); the tape is written before the snapshot, a viewer's file is whole or not there (`.part`, then a rename), a tape that stops before its snapshot is refused at resume, a write error stops the run at the end of that year; a new buffer for each snapshot had cost as much as the whole save (`docs/lighter/measures.md`).
- F1, the pictures too (level 1, golden unchanged): `render::save` of the `--every` years is timed in the tape's column (0.6 ms a picture on the island).
- G1 (level 1, golden unchanged): the halving search of `Reactor::sub_step` starts at ceil(log2(heat / bound)) halvings, the fewest that the concave heat can allow, instead of halving one at a time; the same sub-steps, found with about 60% fewer tries. The parts' reactor on globe36 is 28% cheaper, the year 1.2%.
- The long-step test judges the mean of six worlds (seeds 1 to 6, its own list), the limit unchanged at 0.3 K: one seed's offset of the land spans 0.10 to 0.40 K, so the mean of three moved by chance. On the base the land is 0.259 K off (`measures.md`).
- B1, the skip of tiny bare reactions (level 3, golden re-pinned): in a body's reactor a bare reaction that would use under 3.6e-5 of its scarcest input over the world's year is skipped (`kernel.rs`, `TOO_SLOW`; counted over the year, so it does not depend on the length of an update); a test states it. It replaces the law "living bodies do not rot", which probe PR3 found makes the world heavier (eaters x35 on 32 x 32, +38% updates). `catalysts_fail_when_hot` takes its bare reference outside a body. The year one thread -2.2% (island22), -0.8% (globe36), -2.4% (region24), a body reactor call -19 to -20%; key numbers, the batch test and the long-step test (six worlds: 0.262 K, the base 0.259) as before (`measures.md` B1).
- The plan after phase P (`docs/lighter/plan.md`, `decisions.md` L1 to L40, the probes' key tables in `measures.md`, `design/animals.md`): animals are the largest part of the year on `benchZ` (32 to 43%), stands 19 to 24%, the air 11 to 21%; the law "living bodies do not rot" is left to the owner (PR3: it makes the world heavier) and the skip of tiny bare reactions is built instead; D1, D2 (PR6) and E1 (PR7: the land wetter) are not built; lane G gains G2 (a part reacts as a ground node does); a new lane H puts the animals' children and maturing in lumps (PA); lane C is restated from C0 (C2 only at -10% of the year).
- The plan after the third review (`docs/lighter/plan.md`, `decisions.md` L1 to L43): lane B done (the skip; the law left to the owner); lane H restated (H1 children in lumps with a window of the world's batches and each node's phase, its checks and guard; H5 one draw per birth; H4 a probe after them; H2 not built, its gain under a law step's line); G2 level 4 for parts with its design note first, done-when -2%; the gains as upper bounds; H before C in `copy/` and `nodes.rs`.
- H1, children in lumps (level 4, golden re-pinned, snapshot 50): a population node makes its children at most once in `lump_days` (30), at its first update that may, all its brood covers then (`copy::due`; its next turn in `Body::lump_at`; a node's first children wait for a batch of its own phase, a hash of its id); the brood fills at every update and leaves with members who die. Individuals as before; stands, at 40 days, at each update. The groups of children made fall by 35 to 37%, the year by 7.0% (island22), 5.9% (globe36) and 7.1% (region24) on the same saves; the animals stay within their seeds' noise on three worlds, larger lumps found fewer nodes, not more; the key numbers, the nodes, pace, batch, long-step and eaters' tests hold (`measures.md` H1).
- G2, the parts' reactor held at its ground's temperature, not kept (branch `lighter/g2-not-kept`): the reactor's law works (136k to 5.7k instructions a call), but without the 1e6 K burn parts no longer end (26,087 after 30 years on globe36 against 660; 19,679 and rising with the built-solids rule); an open item for the owner (L43).
- H5, one draw per birth, not kept (branch `lighter/h5-not-kept`): -1.54 / -0.45 / -1.68% of the year, under a level-3 step's 3% (L41).
- The decisions after lanes G and H (`docs/lighter/decisions.md` L20, L39 to L44; `plan.md` statuses): H1 done, H5 and G2 not kept, C1a and C1 one law step, building.
- The sleep test states the gas bias it accepts (S84): the sleep leaves the heat-absorbing gas 0.3% high in six of six seeds, about 0.9% (+4 ppm) and +0.04 K of the land over a century, flat after year 21. Small, so the owner accepts it: `air_3` is held to 3e-6 (`GAS_SLEEP_BIAS`) instead of its noise, and the fix (`sleep/pair-calm-proposal`, 20% of a run) is not taken (`docs/lighter/measures.md`, "Study: the sleep's bias on the gas").
- Small nodes, not kept (branch `study/small-nodes-not-kept`; open item 8 of the speed x10 report): a node acts whole when its members drawn for an action would leave less than one member behind, so nodes under two members take more than their chances say (+8 to +11% of all node takes in a young 64 x 64 world, about 1e-4 in settled ones). Keeping the mean (O8), and a node under two members doing one option as an individual does (O8b), both lower the young eaters (5.18 to 4.89 log10 kg) and move nodes away from individuals in the nodes check: the over-draw hides another deficit of small nodes, an open item (`measures.md` "Small nodes: not kept").
- The drift and brain-size tests' scene (`churn`) makes children at every update (`lump_days` of one batch), as the other node tests since H1; H1 had left it out, so `genes_do_not_drift_without_selection` measured the noise of too few children (tests only, level 1).
- Study: why the long liquid step warms the land (X80, X85; probe on the branch `study/liquid-warm`). Rain over field capacity waits up to 60 days in the ground for the liquid step, and the liquid's cover darkens the land: reflectance -0.0069, sunlight +1.7 W/m², land +0.20 K against a 2-day liquid, in proportion to the step. Sinking it into the store every physics step (the same law, booked when the rain comes) leaves +0.05 K at any step, the long-step test 0.273 to 0.191 K, every key number in range, +0.3% of a year (`docs/lighter/measures.md`, design `docs/lighter/design/liquid.md`).
- L1, the liquid sinks into the bed at every physics step (level 3): rain over field capacity held up to 60 days in the pores darkened the land and warmed it +0.2 K against a 2-day liquid; the sink (`drain::sink`, the same law) now runs right after each air step, the rest of the liquid every `flow_days`, always on a physics step. The long-step test 0.273 to 0.191 K, land of the key numbers -0.05 K, land worn 0.134 to 0.147 mm/yr, +0.3% of a year (`docs/lighter/measures.md` L1).
- C1, the bound on plant kinds with C1a, not kept (branch `lighter/c-not-kept`): -1.2% of island22's year against a -3% line, light-eater splits x20 to x90, the second invasion of test 1 fails, light-eaters' copy mass -0.7 to -1.0 decades; C2 not built (L45).
- HP1, the hard plant law, not kept (branch `lighter/hp1-not-kept`): plants stiffen with lime (17 to 34% of dry mass) as chemistry 0 makes lime cheaper than frame, island22's year +40%; HP2 not built; an open item (L47).
- The decisions before phase Z (`docs/lighter/decisions.md` L9, L38, L42, L45 to L47; `plan.md`: what was kept, F, G1, B1, H1, L1, about -9% / -8% / -9% of the year; the owner's open choices).
- Phase Z, the round's end (`docs/reports/lighter.md`; `docs/lighter/measures.md`, the "Z" sections): against the start, each on its own remade benches, a year on one thread costs 12% / 22% / 10% fewer instructions (island22, globe36, region24; the kept steps about 9% / 8% / 9%, the rest the worlds), an animal update 13% to 22% less; a settled year on 4 cores x1.10 / x1.30 / x1.02; 1,000 years x1.41 / x1.37 / x1.22 (4.3, 3.7 and 8.7 minutes); the largest globe in 15 minutes is size 30 (1,800 cells, 12.9 minutes; it was size 28). The slow suite fails only on S50. The core ceiling is 10,673 (10,473 lines plus 200, L4).
