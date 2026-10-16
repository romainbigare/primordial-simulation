# Session summary: 14 to 15 October 2026

This file lists what was built or fixed in this session, what was not done, and what comes next.
Main is at `b147ca2` (plus this file). All tests pass. Details are in `docs/handover.md` (one decisions
row for each change) and in `docs/reports/`.

## 1. Review 4 (before M4)

- **Brain size can change inside a node.** A node can now split along its brain-size genes. Before,
  brain size was frozen in nodes for 300 years.
- **The tape shows each starting kind.** New columns per kind (`root_<k>_*`): deaths by cause, births,
  real life, mass, genes, catalyst genes. Also the land's summer, winter and seasonal swing.
- **Breathing goes by the body's open surface** (leaf area plus skin).
- **Intake goes with mass^0.75** (take, sweep, gifts).
- **Roster big eaters start with bigger stores** (60% of mass).
- **Speed work:** fewer memory allocations, a faster path for nodes, optional threads
  (`--features threads`).
- Watch report 5 measured all of this.

## 2. Review 5

- **Likeness counts brain sizes by whole units.**
- **One reference mass** (1 kg) for brain cost, intake and heat.
- **Building a body takes time:** a daily building allowance (`c_build`).
- **Every body pays a basal upkeep** (Kleiber).
- **Leaf area goes with mass^0.75.**
- **`docs/numbers.md`:** every number has a row, a unit, a source and a mark (physical, anchored,
  tuned). A test keeps the count of tuned numbers from growing.
- **Individuals mode** (`--individuals`): animals are never nodes, for checking nodes against real
  individuals.
- Watch report 6 measured all of this.

## 3. Review 6 (adversarial review and the "short lives, small bodies" review)

- **The eaters' start is no longer a lottery.** Animals start with hand-set brain biases. Before,
  97 to 99% of the founders died in their first year.
- **Nodes mix only with their own kind** (`WITHIN` 2 to 0.5). Small eaters of remains had been
  flooding the eaters' nodes.
- **Costs scale with size,** anchored at the 30 kg founder.
- **Moving costs the real cost of transport** (Taylor).
- **A fair big-brain test** (`--clever-year`). Run (seeds 1 to 3): no verdict, the test cannot tell (see below).
- **Bug fixes:** building could be paid twice in one update; the satiety target had no upper limit;
  tests that could not fail; the rain setting differed between tests and runs.
- **`numbers.md` tells the truth:** `c_build` and `c_basal` marked tuned, plus a table of "rules
  chosen for a run".
- Watch report 7 measured all of this. The law "a body is built, not just filled" was tried, failed,
  and was saved as a patch, not merged.

## 4. Body matter and food storage (the owner's decision)

- **A new substance, body matter.** A body is now body matter (made from sugar, costs work and
  time) plus a food store (sugar and fat, cheap).
- **Eaters digest:** body matter they eat becomes sugar, and they must build their own matter again.
- **Stores are bounded by the body,** not by the whole mass.
- **Satiety at every size is off.** It killed every animal 4 times. A full store is the satiety.
- **Plants pay less to build because they hold more water,** not because of a number per kind.
- Watch report 8 measured all of this.

## 5. Review 7 (plants and lives, two Opus agents)

- **Bodies copy by physics only.** The copy trigger reads the body that was built. The brain's copy
  chance is gone. Before, a grown plant never copied by physics.
- **Fixed bugs:** digested matter overfilled the store; stale repair genes; the fuel's safe minimum
  was a death line (it is now a reserve); a newborn's leaf was computed too early.
- **Activity needs liquid** (a new refinement of Law 2): a dry body is nearly dormant
  (Harrington, n = 5.7). Frost kills in proportion to activity. A gene (WET) sets a child's share of
  liquid.
- **Reproduction by surplus:** a body is mature at its copy mass. A share of what it builds (ALLOC)
  goes to children of a size set by a gene (CHILD). Parents keep their body.
- **Young and adult members are kept apart** in nodes.
- **Tape:** deaths of young and adult members, adult life, births by month, moves per member.

## 6. Body size by an energy ceiling (the owner's decision)

- **The cost of keeping tissue goes with mass^1.0. Supply goes with mass^0.75.** Growth stops where
  the two meet. No size cap.
- **Intake, building and wear go by the built body.**
- Speed work on genes, merges and brain rows.

## 7. The queued mechanics

| Mechanic | State |
|---|---|
| Partial damage and healing (bites, fire and pushing tear body matter first; hurt bodies are weaker and heal at the build rate) | done |
| The cost of living follows temperature (upkeep, intake and building by the rate law) | done |
| The heat a body makes stays in it (leaves through its skin, fur and walls) | done |
| Two-parent copying (each gene from one of two alike, mature parents) | done |
| Soil nutrient | done (section 9) |
| Natural fire starts | waits for M4 (the owner's decision) |
| Breeding seasons | emerged by themselves: plants in spring, eaters in summer. No calendar rule. |

## 8. Carbon (review 8)

- **Dead plant matter rots at a litter rate** (Olson 1963), 4 times faster than before. The
  founders' burning genes were lowered so living bodies are unchanged. Carbon gas at year 60 rose
  from 100 to 202 ppm to 223 to 311 ppm.
- **Bug fix:** a gift to a body in a just-loaded world was different from the same gift in a world
  that was never saved.
- **The sea's carbon store** (the owner's request): a dissolved salt, lime + carbon gas + water,
  with the reaction going both ways. It is plain chemistry, with no special code for the sea. The
  sea holds more with more water, and dry land holds none. At the start the sea is at balance with
  the 400 ppm air, worked out from the reaction speeds. The air now stays at about 300 to 360 ppm.

## 9. Soil water and nutrients (the owner's request: "from pure physics and chemistry")

- **Soil water:** loose ground has 40% pore space. Rain fills the pores first, and only the water
  above them runs off. Pore water moves by Darcy's law. Bodies drink from it.
- **Nitrogen:** body matter needs a nitrogen salt from the ground. Rotting gives it back. The salt
  moves with the water.
- **Fixing founders** (the owner's choice): 1 in 10 starting plants can take nitrogen from the air,
  paid with their food. After the start, evolution sets the mix. Counted as a "rule chosen for a
  run" (budget 2 to 3). Without fixers, life collapses.
- **Calcium, the rock nutrient** (the owner chose "both"): body matter holds 1.2% calcium, taken
  from the same dissolved salt as the sea carbon. One code path for all minerals.
- **Erosion at real rates** (the owner's decision): `carry`, `wear` and `creep` come from real
  measurements, not tuned guesses. Erosion had been 1,000 to 10,000 times too fast. Now the island
  wears down 0.01 to 0.07 mm a year (Earth's mean is about 0.05). Tuned numbers went from 66 to 63.

## 10. Law 2 without a jump (the owner's decision)

- **The reaction law had a jump:** a reaction's speed changed about 6,000 times when its scarcest
  input switched from a gas to a solid.
- **New law:** true mass action with the contact of the phases. It has no jump, and the books close.
- **Re-anchored from the same real figures:** the sea carbon speed, the plants' starting light gene
  (`LIGHT_X`) and the price of enzymes (`c_cat`).
- **Result:** plants are lighter on 2 of 3 seeds. Eaters are heavier. Carbon gas ends 10 to 80 ppm
  higher.
- **The step follows the rate law (16 Oct).** A step held a reaction's starting rate. For a salt far past its
  balance this lost all of it in one step, where the law leaves a few percent. Now the step uses the integral of
  the rate law. No new number.

## 11. Numbers at the end of the session

| | start of session | now |
|---|---|---|
| tuned numbers | 67 (when first counted, review 5) | 62 |
| rules chosen for a run | 4 (when first counted, review 6) | 3 |
| core lines (goal 8,000 through M7) | 7,029 | 8,576 (7% over) |
| plants: adult life (seeds 1 to 3, 60 years) | about 1 to 2 months (all members) | about 0.8 to 1.2 years |
| eaters: adult life (seeds 1 to 3, 60 years) | about 1 to 2 months (all members) | about 1.4 to 2.7 years (measured before section 10) |
| carbon gas at year 60 | 44 to 181 ppm | 307 to 334 ppm |

## 12. What was discussed but not done

Things that were decided to wait:

- **Natural fire starts:** in M4. They need a "burning share of a cell" law.
- **Disease** (bodies inside bodies): after M4.
- **More speed work:** during M4.
- **Size-class nodes** (`COHORTS`): built but off, as the owner asked. Only young and adult members are
  kept apart.

Things that are open and not built:

- **Nitrogen fixing is too cheap:** closed. It now burns 14 units of fuel per N2 (6 g C per g N, Layzell 1979), and its heat pays no upkeep (handover, "Fixing costs what it costs"). Fixers and the flood of the mineral barely changed.
- **End-product inhibition (15 Oct 2026, the owner agreed): built, one general law, not merged.** A body's catalyst is slowed by
  its store of the reaction's product: speed-up over 1 + store / level (the level the body keeps; no new number). Gases, the key
  liquid and the burnings run for upkeep are not slowed. Seeds 1 to 3 (64 x 64, 60 years): the world's nitrogen mineral 68 to 97 ->
  33 to 44 times the soil's gift; fixers 49 to 81% -> 43 to 74% of plant members (no real change); `nlim0` 6 to 23% -> 9 to 25%;
  plants -14% to +8%; carbon gas +10 to +18 ppm. Soil already differed a lot between cells and still does (fixers make the rich
  cells). The flood is halved, not removed: the store sits at its level, where the law gives one half. A stronger form (a full store
  stops it) gives 14 to 18 times the gift and is the owner's call (handover, Section 4, "End-product inhibition").
- **The body heat test:** closed. Found and fixed: the body's
  reactions ran 9 K too cold after its upkeep, and the test left out the latent heat of the liquid the burning makes.
  Within 2% again.
- **The roster's big animals** (100 to 600 kg) died out in most runs: **mostly closed.** Their founders starved: across kinds a
  big kind paid the same per kg as a small one. Built (the owner agreed): Kleiber across kinds, founders placed as adults, and R4
  (handover, "Why the roster's big animals die"). The 150 kg and 600 kg kinds now live 60 years on seeds 1 to 3, the 100 kg on 2;
  the 600 kg kind is few (1 to 120 bodies). Normal eaters are 2 to 5 times lighter, plants 16 to 38%. Bodies gave away their own matter in
  gifts (`choose::give`): fixed on 15 Oct, a gift is of stores only (handover, "The rock salt in bodies, R11 and gifts").
- **Eater numbers swing up to 1,000 times** between almost identical runs: **closed, no longer true on main 1c7e5d1.**
  Eaters at year 60 over seeds 1 to 9: 4.1e8 to 1.75e9 kg (4.3 times; log10 sd 0.20), at year 30 3.1e5 to 8.6e5.
  Seed 1 with `--spray 19`, `--spray 21`, `--life-year 3`, `--fixers 0.11`: 1.2e9 to 1.7e9 (seed 1 itself 1.2e9).
  The old swing was the founding bottleneck: on 03123d8 (the report's main) 50 to 80% of the founders died in two
  years, and the run took off after 5 to 15 years, or never (seed 5 died out); then it grew 0.45 a year, so the lag
  alone gave 1,000 times. Adult founders breed at once: members are 3 to 6 times the founders by year 24 on every
  seed. No cycle: eaters still grow 0.18 to 0.23 a year in years 45 to 60 (doubling in about 3.5 years), no deaths of
  lack, under 5% eaten, at 1 to 6% of the plants' mass. So year 60 measures how fast they grow, not a level. Reports: give log10 kg
  at year 60 and the growth rate over years 30 to 60, three seeds at least; under 2 times is noise.
- **Erosion is very slow in the first 10 years:** closed. The measure counted dead body matter piling up on the land as rock;
  without it the land loses 0.07 to 0.085 mm a year from the first year (handover, "Erosion at real rates").
- **The sea floor uses the air's oxygen,** and the sea's carbon reaction takes gas from the whole air
  column, not from the water. **Closed on 16 Oct 2026 (branch, not merged yet):** ground under standing liquid
  reacts with the dissolved gas through the liquid's film. It waited because 52 to 56% of the land stood waterlogged.
  The water budget showed why: rain 0.73 to 0.76 m a year, 58 to 60% evaporated, 30% ran off, the rest filled basins;
  standing liquid evaporated at half the open rate, wet soil at 0.36 to 0.39 of it. Now the bed's store and the wet
  loose ground drain to channels inside each cell (Darcy's law over a 250 m hillslope, one anchored number) and wet
  ground evaporates at the open rate: waterlogged land 10 to 11% (half of it basins), evaporation 67 to 69% of the rain.
  With that the film moves the carbon gas by -18 to 0 ppm at year 60 (handover, "Soil water" and "The sea floor
  reacts"). Found on the way: plants give 0.21 to 0.25 m of liquid a year to the air (real 0.3 to 0.6), but they
  catch 5 to 6 m a year from the air's vapour (7 times the rain) and soak under 3 mm from the ground, so the soil's
  liquid never goes through plants. Changed on 16 Oct (section 14).
  The 2/3 power of the sea's store comes from the pair's contacts, not from the gas source: still open.
- **Review 7 items:** R4 (the liquid shortcut) is built (the living body is matter over 1 - 0.75); R5 was done by the energy
  ceiling; R6 is not built: `c_build` is anchored at 1e4 to real P/B instead (tuned 63 -> 62). Liquid made in a body above its limit drains out (R11): ready, not landed (below). Plants that
  walk (fix 7) is closed: moving needs force, plants move under 0.1 times a year (was 8 to 11).
  R11 first killed the eaters on all three seeds: the rock salt fell out in their bodies. The cause was a
  step error in Law 2 (the rate was held over the step, so the salt went to its last mol in one update). Fixed
  on 15 Oct. With R11 the eaters now live on all seeds and all can move, but on seed 3 they are 4 times fewer, in the
  roster the big kinds die more often (4 of 9 alive at year 60 against 6 of 9), and a learning test fails at its noise
  edge. So R11 was not landed; it is one commit (e13d911). Landed on 16 Oct, the owner's decision (section 14).
- **A body keeps a temperature** (the owner agreed, 15 Oct): a gene `KEEP_T`; a body colder than it burns fuel to stay
  at it (handover, "A body keeps a temperature"). Built, not merged. All founders start at 250 K (it does nothing). On seeds 1
  to 3 (60 years) the gene does not move (249 to 252 K) and the insulation gene does not move: not selected on the mild island,
  as expected. Plants and eaters as on main (under 2 times). Found: bodies radiate from their own temperature past their
  insulation, so no body gets under about 3.6 W/K at 30 kg, 3.8 times real fur. The owner decides on putting radiation
  behind the insulation. Done on 16 Oct (section 14).
- **A second, dearer body matter** (protein-like) for animals: an idea, not built.
- **The big-brain test:** run on seeds 1 to 3 (64 x 64, 60 years, copies at year 40). It cannot tell.
  The control (same brain, root 11) joins the ordinary eaters' nodes in the first year, so there is no real control.
  The big brain starts with 300 members against 10^5 and loses members for 10 years (it dies of `SPENT` at 0.06 to 0.11
  per member a year). From year 51 it is born more (0.36 to 0.43 per member against 0.29) and dies as often, so it grows
  faster (0.28 to 0.33 a year against 0.18 to 0.23). But it starts small on an island that is not full, and its brain-size
  gene stays at 48 (no selection seen). So "a big brain pays" is a hint, not a proof. It needs a control that does not merge.
- **All results are 60 years, 64 x 64.** The 300-year runs were never made.

## 13. Next steps (suggested order)

1. **The three 300-year runs at 128 x 128.** Check that life lasts. Check whether plants evolve
   their light gene, whether fixing goes down by itself, whether the seasons hold, and where the
   carbon settles.
2. **The small-cell world** for the nodes-against-individuals check, with plants included.
3. **Fix the open problems** that the runs confirm: done in the follow-up on 15 Oct (above). Left for the owner:
   (R11 and the plants' liquid: done on 16 Oct, section 14), a solid
   store of the rock element in animals, the stronger form of product inhibition (a full store stops the enzyme), a control that does not merge for the big-brain
   test, and `bump` (tuned) now decides whether a body can move.
4. **Core size:** two simplify passes (same history bit for bit) took it from 8,707 to 8,467 lines (6% over 8,000).
   Almost no dead code is left; going lower means removing mechanisms (the speed schedule of Change, about 220
   lines; `COPY_BY_CHOICE`; tape-only tallies). The owner decides, or accepts the 6% with this reason.
5. **Real figures:** checked against sources in `docs/numbers.md` (most agree; `seep` was a wrong sum and is fixed; the
   eaters are 20 to 100 times denser than real large plant-eaters, Hempson et al. 2015). **Studied (16 Oct, `docs/reports/eater-density.md`):**
   61 to 67% of their kg is liquid past the level (R11 removes it: 34 to 72 t/km2 left, against 231 to 351). Their cost of living is
   not the cause (half a mammal's, 2.5 times a reptile's). They are cold-blooded, and their tissue (17 to 59 t/km2) is no more than
   Aldabra's tortoises (58). The fault is the food: 95 to 99.5% is dead plant matter, which is digested like meat; on living plants
   alone they die out. They are still growing and could reach about 8 times year 60. Owner's choices: R11, a structural plant matter.
6. **Then M4:** the planet with real seasons, frost and dry seasons, natural fire starts, and speed
   work. Disease after M4.

## 14. Three body-physics changes (16 Oct 2026, the owner approved; a branch of main 5de4eed, not merged)

- **R11 landed:** liquid past the level a body keeps drains to the ground with its heat. Eater members weigh
  49 to 54 kg, not 121 to 124; eater kg is 4 to 7 times lower; they live on all seeds. The learning test passes unchanged.
- **Rain, not vapour:** a body takes its share of the cell's rain (1 - exp(-x) by capture area, shared by height as
  the light is), from the cell's liquid. Plants now take 0.06 to 0.08 m a year, 70% of it rain they catch, 30% roots;
  they transpire 0.06 to 0.08 m a year (real 0.3 to 0.6) and are dry a fifth of the time. Of the rain 68 to 70% goes
  back to the air (real about 60%), but mostly from the ground: transpiration is 10 to 13% of it (real about 60%).
  Roots that take what a plant transpires would be the next step.
- **Radiation behind the coat:** a coat's outer surface radiates; fur gives 1.0 W/K for 30 kg (real 0.94; before 4.4).
  Bare bodies (all founders) are unchanged. The textbook form (the gene is the coat, still air always outside) was
  measured and dropped: bare eaters 4 times less coupled, they died out on 2 of 3 seeds. A body whose upkeep climbs
  9% a K has no steady warmth past about 4 K of heat over conductance, so no body can wear real fur at rest.
- **Not moved:** `HEAT` and `KEEP_T` stay where they started. Roster: the big kinds alive at year 60, 5 of 9 (main 8).
- Details: handover, "Three body-physics changes".


## 15. Bodies built of several materials (16 Oct 2026, the owner agreed; a branch of main cc86483, not merged)

- **Step 0, the store:** plants held 62 to 64% of their dry mass as the energy-rich solid (real 5 to 20%). Not a bug:
  founders start at 63% (30% of their mass), a full store would be 78%, and storing cost nothing. Holding up (step 2)
  makes the store a load: plants then hold 49 to 53%.
- **Step 1:** chemistry lists the solids a body can build besides matter (a frame, substance 25, cellulose-like; lime,
  substance 10, of the rock salt). The keep gene of each is its share in what a body builds; founders start at 0.02 of
  each. No upkeep for them. A body keeps its own lime now (to its level).
- **Step 2:** height is what a body's soft tissue and built solids hold up against its weight (Greenhill's column, Young's
  modulus of each solid). Plants' share genes rise (frame 0.05 to 0.09, lime 0.06 to 0.13 at year 60); plants stand 2.1 to
  2.6 m. Eaters drop both.
- **Step 3:** a bite pays to break what it tears, pushing through breaks what its energy pays for (by the stress each solid
  breaks at; `walk_wear` gone); a body takes matter and built solids only if its own catalyst breaks them, and what it does
  not break passes back to the ground. No founder has the frame's enzyme: given to the roster's eaters of remains it cost a
  third of their upkeep and made them 10,000 times fewer. Roster: the big kinds alive at year 60, 6 of 9 (main 5).
- **Surprise:** plants build more lime than frame (as stiff per kg, needs no fuel, does not rot; the frame rots inside the
  living body at the litter constant). Bites barely pay to break (a few kJ per kg against 15 MJ of food): hard parts protect
  by not being digested, not by force.
- **Open:** the litter constants run inside living bodies; real living stems and bones do not rot. The owner decides.
- Details: handover Section 4, "Bodies built of several materials".
## 16. One heat network, cooling by evaporating, roots that refill (16 Oct 2026; a branch of main cc86483, not merged)

- **One heat network for every body:** core, coat (`HEAT`), outer surface (skin and both faces of the leaf), still air and
  radiation in parallel. The coat rule is gone. The founders' old contact cannot be kept (eaters 64 kg: 55 W/K, a bare
  body passes at most 15.5): they start bare, `HEAT` 3.5. Fur gives 0.9 W/K for 31 kg.
- **Cooling by evaporating:** a new gene `COOL_T`, the mirror of `KEEP_T`; above it a body evaporates what holds it
  there, limited by its still air (Lewis relation), the air's deficit and its liquid's safe minimum. One level was
  measured and failed (eaters dried themselves out). Founders at 320 K; not selected on the mild island. A furred body
  cannot rest at 310 K: it would need 10.6 kg of liquid a day.
- **Roots refill:** the soak is a flow along the contact solved exactly over the update. Plants are dry 1 to 2% of the
  time (main 19 to 25%), but transpiration stays 0.04 to 0.06 m a year (real 0.3 to 0.6): it is set by the breath (`GAS`).
- **Life:** eaters 5 to 20 times fewer at year 60 (they take off later); **the roster's 0.1 kg light-eaters die out on
  seeds 1 and 2** (already with the network alone, seed 2), big kinds 3 of 9 alive (main 5). Not for merging as it is.
- Details: handover, "One heat network, cooling by evaporating, roots that refill".

## 17. Air by wind and size, a leaf's pores; resting upkeep at a cold-blooded rate (16 Oct 2026, on main 9967317; not merged)

- **(a)** Every body's boundary layer goes by the wind near the ground (1.7 m/s, anchored: Flow has no wind there) and
  its size; the `GAS` gene is a stomatal conductance, light-eaters start with a real leaf's (0.25 mol/(m2 s)); pores close
  as a body dries; a soaking plant loses what flows through it. **Transpiration 0.30 to 0.33 m a year (main 0.04 to 0.05),
  47 to 51% of the land's evaporation (real about 60%).** Eaters 2 to 3 times fewer; roster eaters 9 to 600 times fewer,
  the big kinds lost; carbon gas 145 to 170 ppm in the roster (main 210 to 240). Kind 3 lives on all seeds.
- **(b)** The five costs of tissue x 0.068, to a reptile's resting rate. It breaks the eaters (seeds 1 and 2, all roster
  seeds): the light-eaters walk (strength is a gene and now nearly free) and trample them; the eaters are not denser.
- Not for merging as it is. Details: handover, "Air by wind and size".
