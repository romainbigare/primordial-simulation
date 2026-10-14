# The numbers of the world: what each one is, what it is anchored to, and how far to trust it

**Written:** 14 October 2026 (review 5, row 6; handover 3 and 14.3), marks revised the same day by review 6 (finding 9). **Keep it true:** a test (`core/tests/numbers.rs`) fails when a field of `Physics` or `Config` has no row here, or a row names a field that does not exist. A new number cannot be added without saying what it is.

## How to read this table

Every field of `Config` and of `Physics` (`core/src/lib.rs`) has one row. A row says:

- **unit:** the unit of the number.
- **fact:** the fact of matter or energy it is (a capacity, a cost, a conductance, a rate, a reference mass, a speed-tool setting...). A number that is not a fact of matter or energy is a setting, and the row says so.
- **anchor:** the real-world size it is anchored to, with the source in a few words. "Run:" means the run or the reason it was set for. "None found" means I found no real size in the code, the handover or the reports.
- **mark:** one of three, plus two for things that are not numbers.
  - **physical:** a measured size, used as measured.
  - **anchored:** a real order of magnitude, chosen within it.
  - **tuned:** set so a run came out a certain way. The row names the run or the reason. `tuned, first guess` is the same mark for a number with no real size and no recorded run: it was written by hand in M0 to M3 to get a sensible start and kept (commit `bc47a66` moved them into `Physics` unchanged). They count as tuned, as they have no size behind them. The first job of any of them is to find a size and become anchored.
  - **switch:** an on/off setting of an experiment, not a number of the world. **container:** a field that holds other rows.

**The M7 world search varies only tuned and anchored numbers, within their anchors, and never the physical ones.** Switches are not searched. For an anchored number "within its anchor" means within the real order named in its row (about a factor of 3 each way unless the row says more). For a tuned number it means around its value, by the factor its row suggests, and every run prints the numbers it used (`physics.json`, review 5, drift 1, fix 4).

**The budget for tuned numbers is in handover 14.3.** The count is at the end of this file.

Some numbers in this file were found by reading doc comments and reports, not by checking a source. Where I give my own arithmetic for a real size, the row says "my arithmetic". Rows I am less sure of say "unchecked".

## Config

| name | unit | fact | anchor (source) | mark |
|---|---|---|---|---|
| `sun` | W/m² | strength of the star at the top of the air | the solar constant, 1361 | physical |
| `tilt` | degrees | axial tilt: sets the size of the seasons | the planet's tilt, 23.4 | physical |
| `lat_south` | degrees | latitude of the map's south edge | run: the island spans 2.3° (256 km); centred at 32° so the starting ground stays near 287 K (40° gave 279 K), handover 6.1 | tuned |
| `lat_north` | degrees | latitude of the map's north edge | the same run as `lat_south` (31 + 2.3) | tuned |
| `rotation` | none (0 = none) | strength of the turning of moving air by the planet's spin | 1 is the real size of the spin's effect; the wind closure is crude (unchecked) | anchored |
| `days_per_month` | days | the calendar | 12 x 30 = 360 days for a year of 365 | anchored |
| `months_per_year` | months | the calendar | the same | anchored |
| `phys_days` | days | speed-tool setting: how often ground and air physics run | run: the table of channel cells against schedules in the doc comment (seed 5, 50 years, 128x128) | tuned |
| `air_days` | days | speed-tool setting: how often air moves between nodes | the same table: 3 days loses nothing against 15 once the moisture path runs every 3 | tuned |
| `exchange_days` | days | speed-tool setting: how often evaporation and rain run | the same table; the default is 3 (the 3/3/15 row: 1405 channel cells), what `ew` runs; it was 9 (484 cells) while the shell set 3, so tests ran another rain path (review 6, finding 11) | tuned |
| `flow_days` | days | speed-tool setting: how often liquid and creep move | the same table | tuned |
| `skin_m` | m | depth of ground that exchanges heat with the surface | the depth the yearly swing reaches in soil is 2 to 4 m (my arithmetic, unchecked) | anchored |
| `air_top_m` | m | height of the air column | the lower air of Earth, 8 to 16 km | anchored |
| `gravity` | m/s² | pull of the planet | 9.81 | physical |
| `sensible` | W/(m² K) | heat passed between ground and the air touching it | measured bulk exchange over land in wind, 5 to 30 (unchecked) | anchored |
| `evap` | m/s | transfer speed on the air's side of a surface (evaporation) | usual exchange speed over open ground in wind, 0.005 to 0.02 (unchecked) | anchored |
| `dissolve` | m/s | transfer speed on the liquid's side of a surface for a dissolving gas | slightly soluble gases over open liquid in wind, 5e-5 (4 m a day), doc comment | anchored |
| `wind` | m/s per K | overturning air speed per K of warmth difference between air nodes | none found; a sea breeze of a few m/s for about 10 K is the same order (my arithmetic, unchecked) | tuned, first guess |
| `mix` | m²/s | mixing of neighbouring air columns by eddies | run: set so the air crosses the map in weeks (2e4 over 256 km: 38 days); real eddy mixing over weather scales is 10 to 100 times more | tuned |
| `cloud` | mol/(m² s) | condensation at which droplets cover half the sky | none found; 1.2e-2 is a light rain of 0.8 mm an hour (my arithmetic) | tuned, first guess |
| `carry` | m³/J | loose solid volume moving liquid can carry per J it dissipates | none found; set in M2 with the first channels | tuned, first guess |
| `wear` | m³/J, times 1/bond strength | bonded volume broken per J dissipated | none found | tuned, first guess |
| `regrow` | m/s | speed at which bodies spread back into a cleared lane | 1 nm/s is 3 cm a year, the slow end of the spread of a ground cover by runners (my arithmetic, unchecked) | anchored |
| `creep` | m/s per unit slope, times 1/hardness | loose solids creeping down a slope | none found; 1e-6 m/s is 30 m a year at a slope of 1, far above soil creep in life (mm to cm a year, unchecked) | tuned, first guess |
| `crowd` | nodes per cell | speed-tool setting: past this many body nodes in a cell, the most alike merge | run: speed; it must give the same patterns as individuals (to be checked at M4, review 5 row 8) | tuned |
| `share_brains` | on/off | bodies that stay near each other share one brain's rows | a setting, to check that sharing gives the same patterns | switch |
| `nodes` | on/off | animals live in population nodes (on) or only as individuals (off, `--individuals`) | a setting, for the check that nodes give the same patterns as individuals (handover 7.1, review 5 row 8) | switch |
| `sate_all` | on/off | satiety at every size (on) or only past `full` (off, the default; `ew run --sate-all`) | the decision after watch report 7 asked for it at every size; with it all the animals of six 64x64 runs died in their first year (handover 4, 14 Oct 2026), so the default stays the rule chosen for a run in the table below. A setting, to repeat that result | switch |
| `split_rows` | on/off | a node also splits along its born rows | an experiment, off: 2 to 2.5 times slower for no gain, doc comment | switch |
| `null` | on/off | a null run: no learning, no imitation, brain rows frozen | the run to compare a seed with (review 3, M6) | switch |
| `phys` | (a `Physics`) | holds the numbers below | | container |

## Physics

### Change and Flow, as bodies feel them

| name | unit | fact | anchor (source) | mark |
|---|---|---|---|---|
| `light_depth` | m | liquid thickness over a body that holds back 63% of its light | clear sea water: broadband light falls by e in 1 to 20 m; 10 m is the clear end (unchecked) | anchored |
| `gas_depth` | m | liquid thickness over a body that cuts its gas exchange by a factor e | none found; a gas moves 10^4 times slower in liquid than in air, so 1 m is generous (my arithmetic) | tuned, first guess |
| `wet_depth` | m | standing liquid depth at which the ground counts as half wet | a film or puddle of about 1 cm | anchored |
| `shelter` | W kg/(m⁴ K) | heat the cover of other bodies passes to a body (layer of m kg/m² passes shelter/m W per K and m²) | run: 220 gives a 30 kg member under 10 kg/m² of cover 3.4 K more warmth, as thick walls give 3.3 K (`body.rs`) | tuned |
| `c_cat` | J per kg^0.75 per day per unit of capacity (1/s) | upkeep of catalysts: a body pays for what its catalysts can do | keeping enzymes costs a resting animal 10 to 25% of its upkeep, doc comment (14 Oct 2026). Per kg^0.75 (`Body::equiv`) since the same day (review 6 finding 2), anchored at the 30 kg founders: 1.5e9 x 30^0.25 = 3.51e9 | anchored |
| `t_cat` | K | the temperature at which upkeep measures catalyst capacity | a typical surface temperature; the same as `worldgen::life::T_SURF`, which now reads this (one constant, `pcore::T_SURF`) | anchored |
| `unfold` | K | width over which catalysts lose their shape above their gene | life's enzymes unfold within a few K | anchored |
| `t_firm` | K | catalysts that keep their shape up to here cost no more | about where life's enzymes fail, 320 K | anchored |
| `c_firm` | K | each this many K more of the firmness gene adds the upkeep once more | none found; at 360 K catalysts cost three times as much | tuned, first guess |
| `c_strength` | J per kg^0.75 per day at strength 1 (at the 1 kg reference, `Body::equiv`) | upkeep of strength | about 1 W/kg at 30 kg; a 30 kg mammal rests at about 45 W (Kleiber, report 4), so this is the order. Per kg^0.75 like `c_basal`, intake and building (14 Oct 2026, review 6 finding 2), anchored at the 30 kg founders: 1e5 x 30^0.25 = 2.34e5 | anchored |
| `c_leaf` | J per m² per day | upkeep of light-capture area | run: so the starting plants pay what they paid before. A leaf's own breathing is about 4e4 J per m² per day (1 µmol of carbon gas per m² per s, my arithmetic), 20 times more | tuned |
| `c_brain` | J per kg of a member of `m_ref` kg, per decision | cost of one decision of the starting brain (12 hidden, 6 memory, 3 seen, sight 1) | none found; kept as a speed incentive on purpose: it pushes bodies toward long intervals (plan 7.3, handover 6.5). The four numbers below split it | tuned |
| `m_ref` | kg | the one reference mass: the member at which a per-kg number is per kg (brain cost, `take`, `sweep`, the heat gene) | a unit, not a size: 1 kg. Three reference masses (`m_ref`, `intake_ref`, `heat_mass`) were made one on 14 Oct 2026 | anchored |
| `c_hidden` | J per kg of a member of `m_ref` kg, per decision | cost per hidden unit | derived: a share of `c_brain` (12 x 5e3 = 6e4 of 1e5) | tuned, first guess |
| `c_memory` | the same | cost per memory number | derived: a share of `c_brain` (6 x 5e3/3 = 1e4) | tuned, first guess |
| `c_seen` | the same | cost per body perceived | derived: a share of `c_brain` (3 x 5e3 = 1.5e4) | tuned, first guess |
| `c_sight` | the same | cost per square cell of sight | derived: a share of `c_brain` (1 x 1.5e4) | tuned, first guess |
| `c_learn` | J per kg^0.75 per day at learning rate 1 | upkeep of learning | none found; per kg^0.75 since 14 Oct 2026, anchored in size at the 30 kg founders (3e4 x 30^0.25 = 7.02e4) | tuned, first guess |
| `c_signal` | J per kg^0.75 per day at signal size 1 | cost of signalling | none found; per kg^0.75 since 14 Oct 2026, anchored in size at the 30 kg founders (200 x 30^0.25 = 468) | tuned, first guess |
| `e_life` | J per kg | energy a kg of body turns over in its upkeep before it is worn out, at a repair gene of 0 | Rubner 1908: many kinds that move turn over about 1,000 kJ per gram of body in a life | anchored |
| `c_repair` | share | share of upkeep that is repair at repair 0 | remaking a body's own matter takes 20 to 30% of resting upkeep in life | anchored |

### Choose

| name | unit | fact | anchor (source) | mark |
|---|---|---|---|---|
| `c_move` | J per kg per m, for a 30 kg member; others pay it x (m1 / 30 kg)^-0.32 | energy to move one kg one metre on the flat | the real cost of transport, 10.7 m^-0.32 J per kg per m (Taylor et al. 1982, from running, walking and flying animals), which is 3.6 at 30 kg; 3 fits a 30 kg body. The mass law is `Physics::walk` (14 Oct 2026, review 6 finding 2; before, 3 for every size) | anchored |
| `rho_pass` | kg/m² | body mass per area that doubles the cost of entering a cell | none found | tuned, first guess |
| `lane` | m | width of the passage a mover clears through bodies | the width of a body or a trail, about 1 m | anchored |
| `layer` | m | the bodies of a cell must fit in a layer this thick over it | a tall plant, about 10 m; a ceiling far above real cover (a forest holds under 100 kg/m², this allows 10,000) | anchored |
| `rho_body` | kg/m³ | density of bodies | living matter is about the density of liquid, 1,000 | physical |
| `walk_wear` | kg/J | body mass broken per J a mover dissipates against it, times (strength + 0.1) | none found; review 2: do not tune it to force trails | tuned, first guess |
| `take` | share per day | most a body can take per day of the mass it takes by, per kg at `m_ref` | a 57 kg eater (20.8 kg-equivalents) takes at most 1.04 kg a day, 1.8% of its mass (5% of its kg-equivalents; 5% of the mass of a 1 kg member). Real food intake is 1 to 5% of body mass a day, and this is about 2.7 times Kleiber's resting need for that eater (1.04 kg of sugar at 15.6 MJ/kg is 16 MJ a day against 3.4 m^0.75 W = 6.1 MJ), a real appetite. Review 5 (drift 1) listed it as tuned; review 6 (finding 2) showed that it does not decide which size wins (the costs by the kg did, `c_strength`, `c_cat`, `c_learn`, `c_signal` now by kg^0.75) | anchored |
| `intake_exp` | none | what a member takes, sweeps and receives goes with its mass that works to this power | Kleiber: 0.75, the exponent of intake and metabolism in life | anchored |
| `sweep` | m² per kg per day | area a body works per day per kg of its mass that works, at `m_ref` | run: set for eaters to live (review 5, drift 1). A 30 kg body works 300 m² in 10 days | tuned |
| `c_handle` | J per kg | energy to handle one kg taken or put | none found; review 5 lists it as tuned | tuned |
| `shape_noise` | none | spread of the shape of a part made, around its mean | none found | tuned, first guess |
| `signal_noise` | none | spread of the signal, around its mean | none found | tuned, first guess |
| `discount` | none | weight of well-being one decision ahead against well-being now | the usual 0.9 of learning machines | anchored |
| `blend` | share | share of the way a watcher's rows move toward the watched one's | none found | tuned, first guess |
| `leave` | cells | a body further than this from its group's centre leaves it | speed-tool setting for group brains (handover 7.0) | tuned |
| `join` | cells | a body within this of a bigger group's centre for `stay` days joins it | the same | tuned |
| `stay` | days | the days a body must stay near a bigger group to join it | the same | tuned |
| `learn_beta` | share | weight of the newest step in a learned row's mean square of steps | the usual 0.1 of a running average (0.9 memory) in learning machines | anchored |
| `learn_most` | learning-rate units | the most a row moves in one step | none found | tuned, first guess |

### Bond

| name | unit | fact | anchor (source) | mark |
|---|---|---|---|---|
| `tensile` | Pa | tensile strength of a solid of bond strength 1 | 10 MPa: the order of soft solids made by life and of weak rock (unchecked) | anchored |
| `joint` | share | share of the contact area a joint pressed by a body holds | none found | tuned, first guess |
| `c_form` | J per kg | energy to shape one kg into a part | none found | tuned, first guess |
| `c_join` | J per m² | energy to make a joint, per m² of contact | none found | tuned, first guess |
| `leak` | W/(m² K) | heat passed through an opening | free and light-wind exchange at a surface, 5 to 25 (unchecked) | anchored |
| `form` | share | most of its mass a body puts into one part | none found | tuned, first guess |
| `min_kg` | kg | walls lighter than this fall apart | none found | tuned, first guess |
| `lever` | none | a held part multiplies a body's force by 1 + lever x hardness x ... | none found | tuned, first guess |
| `grip` | share of weight | force of a body of strength 1 | none found; animals push and pull about their own weight (unchecked) | tuned, first guess |
| `bump` | share of weight | push of a body moving through a cell on each structure there | none found | tuned, first guess |
| `dig` | share per day | most bonded ground a body breaks per day of its interval, as a share of its mass | none found | tuned, first guess |
| `deep` | m | liquid deeper than this is crossed afloat by a body holding a structure the liquid holds up | none found | tuned, first guess |
| `afloat` | share | the cost of crossing afloat, as a share of the cost of walking | none found | tuned, first guess |

### Copy

| name | unit | fact | anchor (source) | mark |
|---|---|---|---|---|
| `c_copy` | J per kg of body matter | work to build one kg of body matter, the growth of a body (`copy::grow`); a child costs its parent none (it is given matter already built) | growth in life keeps 10 to 20% of the energy it eats, so a kg of body costs about 1e7 J, doc comment. With the 1.11 kg of the energy-rich solid that 6 x 30 g make 162 g of matter from, a kg of matter takes 27.3 MJ of fuel, 62% efficient (14 Oct 2026: it was per kg of the dry mass of a child, paid into a child store, and growth was free) | anchored |
| `new_lineage` | chance per child | a child starts a new lineage with a changed base | none found | tuned, first guess |
| `base_noise` | none | size of that change | none found | tuned, first guess |
| `row_chance` | chance per child | a child's born brain rows change | none found | tuned, first guess |
| `row_noise` | none | step of that change per generation | none found | tuned, first guess |
| `new_noise` | none | spread of the weights a bigger brain adds | none found | tuned, first guess |
| `full` | times the copy mass, of body | a member whose body (`Body::size`: its matter and the liquid it keeps with it, not its fuel) is this many times its copy mass copies whatever its brain does (the copy trigger); growth builds a body up to it (`copy::grow`); it caps the mass that works; and it gates satiety: only a member past it takes no more than it can use (`choose::sate`, unless `sate_all`) | run: "copies at twice its copy mass", set so bodies cannot grow without end (review 3, M2). It did not go away with satiety (review 5 row 1): the number still decides when a body copies and, since 14 Oct 2026, from where satiety acts (a rule chosen for a run, see below). By body since 14 Oct 2026 (the decision after watch report 7), not the whole mass | tuned |
| `alike` | mutation steps | nodes whose genes differ by at most this merge; a node splits when its halves differ by more | run: speed-tool threshold, 4 steps (review 3, M3); to be checked against individuals at M4 | tuned |

### Flow, as ground cover and moving liquid feel it

| name | unit | fact | anchor (source) | mark |
|---|---|---|---|---|
| `hide` | m, three numbers | thickness of a layer that hides half of what lies below it: soft solid 0.2, liquid 0.5, loose 0.02 | none found | tuned, first guess |
| `soft` | share of melting point | a solid warmer than this share of its melting point is soft cover lying on top | none found; solids soften and sinter at 0.3 to 0.8 of their melting point in K (unchecked) | tuned, first guess |
| `split` | share | liquid goes only to neighbours at least this share as steep as the steepest | none found; a rule of the routing of liquid | tuned, first guess |
| `still` | m | liquid does not move for a smaller head difference | none found; a film of a centimetre does not run on rough ground (unchecked) | tuned, first guess |
| `hold` | kg/m² | body mass per area that halves what moving liquid can carry off a cell and how fast its loose solids creep | none found | tuned, first guess |

### Added by review 5 (rows 1 and 2, 14 Oct 2026)

| name | unit | fact | anchor (source) | mark |
|---|---|---|---|---|
| `c_build` | J per day per kg^0.75 | most a body pays into building body matter per day: the rate at which it grows, and so at which it rebuilds what it gave its children (`copy::grow`) | the growth of young in life, 1,500 to 30,000 (von Bertalanffy; review 5, row 1). 1e4, review 5's value. It was 3e4 for part of 14 Oct, raised so the neutral-brained founder eaters of seeds 1 and 2 bred faster than they starved (a law set to win a start; review 6, finding 1); that start is now hand-set (`ew run`) and the number is back. Charged on body matter only since 14 Oct 2026 (17% of a plant's mass; the work was for the dry mass, mostly fuel, when every animal died, watch report 7): at 1e4 a 1 kg plant makes 0.37 kg of matter, about 1.5 kg of body, a year. Tuned because runs chose it within the anchor | tuned |
| `c_basal` | J per day per kg^0.75 | basal upkeep every body pays, strength and catalysts on top (2e4) | resting metabolism (Kleiber; its whole value is about 3e5 J per day per kg^0.75, of which this is the part every body pays, the rest being `c_strength` and `c_cat`; review 5, row 2). It is 1/15 of Kleiber's whole value, which review 5 chose so that a 1 kg plant keeps a spare of over twice its upkeep: a number set for a run, so marked tuned (review 6, finding 9). With the costs by kg^0.75 (14 Oct 2026) a 1 kg plant keeps 85 kJ/kg/day of income against 46 of upkeep, 1.8 times | tuned |

## Rules chosen for a run

These are not numbers of the world and not rows of `Physics` or `Config`, but they were picked so a run came out a certain way (handover 2, lesson 3; review 6, finding 9), so they are listed here and counted like tuned numbers. **Each must not grow in number: four today, held by a test (`core/tests/numbers.rs`).** A new one replaces one of these or is not added. Where one is also a row above (the kept copy logits and the roster's stores are in the table of constants, marked tuned), the tuned count already has it.

| rule | what it does | chosen for (run, date) | where |
|---|---|---|---|
| satiety only past `full` | a member whose body is below `full` times its copy mass takes as its intake allows; only one past it takes no more than its reactions can use over the interval (its upkeep plus what it can build and the fuel it converts to do it) | 14 Oct 2026, reviews 5 and 6: with satiety at every size all 416 starting eaters died in their first year of seed 1; asked for again at every size by the decision after watch report 7 and tried (`Config::sate_all`, `ew run --sate-all`), it killed every animal in all six 64x64 runs (seeds 1 to 3, normal and roster), so it stays | `choose::sate` |
| the kept copy logits `H_LIGHT` and `H_EATER` | the one brain bias kept in a neutral start: the body's own chance to copy (-1.0 for light-eaters, -1.6 for eaters); they set the plants' births, since almost no plant member is past `full` (review 6, finding 4) | keeps the pace of generations; a neutral 0 would make every body that holds its copy mass copy on half of its decisions | `worldgen::life` |
| the hand-set eater start (the default for animal kinds in `ew run` and `--roster`) | animals are placed with the hand-set biases (`Seeding::hand[1]`), not neutral brains; `--neutral-eaters` for the old start; plants stay neutral | 14 Oct 2026, review 6, finding 1: 97 to 99% of neutral-brained founder eaters died in their first year, a lottery that `c_build` had been raised to win; the bottleneck is now 25 to 77 founder bodies instead of 2 to 14 | `cli/src/main.rs`, `worldgen::life::Seeding` |
| the roster's stores (`Kind::store`: 30% of mass, 60% for the two big slow eater kinds) | the fuel a starting member holds | seed 1, 64x64 roster: 100 days of store held 6 or 7 decisions of a neutral brain and the 150 kg and 600 kg kinds were gone by years 30 and 35; 200 days kept them (`life.rs`) | `worldgen::life::KINDS` |

## Constants outside the structs that act like free numbers

These are in the code, not in `Physics`. They are not covered by the test. Numerical safeguards (`T_FLOOR`, `MAX_DT_K`, `NEGLIGIBLE`...) and the schedules of the slow work (`WORK_MONTHS`, `REFRESH_MONTHS`, `REACT_MONTHS`, `LAZY`...) are left out: they change how fast or how exactly the laws run, not the laws' sizes.

| name | unit | fact | anchor (source) | mark |
|---|---|---|---|---|
| `copy::WITHIN` | standard deviations | children may lie this far beyond a node's `alike` and still join it (and nodes of a full cell merge) | run: 0.5 (14 Oct 2026; 2 before, which let nodes at the spread's cap join kinds up to 11.6 steps apart, so the roster's animal kinds blended into one small kind); a pair of children of a node held at the split threshold still joins it 199 times in 200. Speed-tool threshold: copying is asexual, so kinds never mix in individuals | tuned |
| `body::MIN_INTERVAL` | days | the shortest time between updates | a speed safeguard (5 days) | tuned |
| `body::MAX_INTERVAL` | days | the longest time between updates | one year | anchored |
| `worldgen::life::T_SURF` and `richness::T_SURF` | K | typical surface temperature, where the starting kinds' reactions and the richness checks look at phases | the same as `t_cat` (290 K): one constant, `pcore::T_SURF` | anchored |
| `MOVE_REF` (30 kg) and `MOVE_EXP` (-0.32) in `core/src/lib.rs` | kg; none | the mass at which `c_move` is the cost of moving, and the exponent of the real cost of transport against the mover's mass (`Physics::walk`) | Taylor et al. 1982: 10.7 m^-0.32 J per kg per m, 3.6 at 30 kg (14 Oct 2026, review 6 finding 2) | anchored |
| the kept copy logit, `H_LIGHT` and `H_EATER` in `life.rs` (-1.0 and -1.6) | logit | the one brain bias kept at the start: the body's own chance to copy | run: it keeps the pace of generations; a neutral 0 would make every body that holds the mass copy on half of its decisions (said in `life.rs`). A behaviour, not a size | tuned |
| `Kind::store` of the roster (0.3; 0.6 for the two big slow eater kinds) | share of mass | the energy-rich solid a starting member holds as fuel | run: seed 1, 64x64 roster, 100 days of store held 6 or 7 decisions of a neutral brain and the 150 and 600 kg kinds were all gone by year 30 and 35; 200 days kept them (`life.rs`) The rest of the mass is the body (14 Oct 2026): liquid in the share its keep gene sets, 0.75 of it, but at least 36% of the mass (`MIN_WET`, over the safe minimum of 30%: at 60% fuel 0.75 x 40% would sit on it), and body matter (`life::split`); 52.5% liquid and 17.5% matter for 30% fuel, 36% and 4% for 60% | tuned |
| the roster's intervals, copy masses and strengths (`INTERVAL`, `COPY_MASS` and `STRENGTH` genes in `KINDS`) | days, log10 kg, share | when each kind updates, where it copies, how strong it is | run: set so each kind lives its first years (600 kg members at 40 days and strength 0.95 were all dead within 6 months). Initial conditions, not laws, but they are numbers a run came out of The copy masses and `LEAF` genes were set again on 14 Oct 2026 for the body (copy mass x 0.7, or x 0.16 for the 60% kinds, LEAF - 0.75 log10 of that), so each copies at the same total mass and has the same leaf at placement | tuned |
| the year each kind is placed (`Kind::year`: 0, 10, 20, 24) | years | when the starting kinds are placed after the first life | run: once what they eat has spread (`life.rs`) | tuned |
| `Seeding::default` (20 rounds, row-bias step 0.2, genes x3) | none | how much a start is sprayed with mutation | run: one generation's step is too small for selection to pick from in the time available (review 2, problem 2). The hand-set start, the default for animals in `ew run` since 14 Oct 2026, takes one generation's step for rows and genes | tuned |

## Chemistry data that bodies live by (not rows of `Physics`)

Chemistry 0 (`worldgen/src/chem0.toml`) is data, not a struct of numbers, and is not in the tables above, but three of its numbers decide what bodies can do, so they are named here (14 Oct 2026, the decision after watch report 7). The budget does not count them: they are the chemistry's, set from facts of matter.

| name | value | fact | anchor |
|---|---|---|---|
| substance 22, body matter | C6H10O5 (162 g/mol), hf -986 kJ/mol, 1,500 kg/m³, melts at 1,500 K | what bodies are built of; 6 of the energy-rich solid (CH2O) make 1 of it and 1 of the liquid, thermoneutral: hf = 6 x -212 - -286, so it holds the energy of what it was made of; burning it gives 2,805 kJ/mol, 17.3 MJ/kg (the solid: 15.6; real cellulose 17.5) | its density 1,500 and its 17.3 MJ/kg are those of cellulose |
| `Chemistry::build`, growth | 6 of the solid -> 1 of body matter + 1 of the liquid; no rate (it is not a reaction of the open world) | what `copy::grow` runs, paid by `c_copy` of work per kg of matter | chemistry data |
| reaction 21, burning body matter | C6H10O5 + 6 O2 -> 6 CO2 + 5 H2O, k 5.7e8 /s, Ea 9,000 K | **the open-world decay of dead body matter: 5% a year at 290 K**, a little slower than the energy-rich solid's 6.6% (structure resists; its Ea is 7,760 K): rate 1.63e-9 /s = k exp(-Ea / 290) x 0.21^6 (mass action, the oxygen-like gas to its 6th power), so k exp(-Ea / 290) = 1.9e-5 /s, k = 1.9e-5 x exp(9000 / 290) = 5.7e8. A test holds it between 4.5 and 5.5% | dead wood and litter last decades in the open; bodies speed it up with a catalyst gene |
| `Chemistry::air`, the cost of catalysts | the starting air's mole fractions | the cost of a catalyst (`Body::prepare`) counts a gas of a reaction beyond first order at its share to the power `nu` - 1 | without it the 6th power of the oxygen-like gas makes the enzyme for burning body matter cost 1,800 times that for the solid for the same speed-up (its rate constant is 1.9e-5 /s against 1.0e-8); with it, 0.75 times. Reactions of order 1 in every gas (the light reaction, burning the solid) are unchanged |

## Counts

By hand and by script on 14 Oct 2026. The 99 rows of `Config` and `Physics` are their 99 fields (`sate_all`, a switch, is the new one, 14 Oct 2026). Review 6 (finding 9) moved four marks: `c_build` and `c_basal` from anchored to tuned (set for a run), `c_move` and `take` from tuned to anchored (a real cost of transport; a real appetite), so the count of tuned rows is the same.

| mark | rows | which |
|---|---|---|
| physical | 4 | `sun`, `tilt`, `gravity`, `rho_body` |
| anchored | 29, and 3 outside the structs | 29 fields (among them `c_move` and `take`) and the constants `MAX_INTERVAL`, `T_SURF` and `MOVE_REF`/`MOVE_EXP` |
| tuned (run or reason named) | 20, and 7 outside the structs | speed settings (`phys_days`, `air_days`, `exchange_days`, `flow_days`, `crowd`, `alike`, `leave`, `join`, `stay`; outside: `WITHIN`, `MIN_INTERVAL`), set for an outcome (`lat_south`, `lat_north`, `mix`, `shelter`, `c_leaf`, `c_brain`, `sweep`, `c_handle`, `full`, `c_build`, `c_basal`; outside: the kept copy logit, the roster's stores, intervals and years, the spray of the start) |
| tuned, first guess | 40 | 36 free numbers (`wind`, `cloud`, `carry`, `wear`, `creep`, `gas_depth`, `c_firm`, `c_learn`, `c_signal`, `rho_pass`, `walk_wear`, `shape_noise`, `signal_noise`, `blend`, `learn_most`, the Bond numbers `joint` to `afloat`, the mutation numbers `new_lineage` to `new_noise`, and the Flow numbers `hide` to `hold`) and the four `c_hidden`, `c_memory`, `c_seen`, `c_sight`, which are the split of `c_brain` and not four numbers |
| switch | 4 | `share_brains`, `split_rows`, `null`, `nodes` |
| container | 1 | `phys` |

Rows marked tuned in all: 67 (60 in the structs, 7 outside). The test holds that count at 67 or below (handover 14.3). Rules chosen for a run: 4, held by the test too.
