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
- **A fair big-brain test** (`--clever-year`). No verdict yet.
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

## 11. Numbers at the end of the session

| | start of session | now |
|---|---|---|
| tuned numbers | 67 (when first counted, review 5) | 63 |
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

- **Nitrogen fixing is too cheap:** closed. It now burns 14 units of fuel per N2 (6 g C per g N, Layzell 1979), and its heat pays no upkeep (handover, "Fixing costs what it costs"). Fixers and the flood of the mineral barely changed; product inhibition would be a new rule (the owner decides).
- **The body heat test:** closed. Found and fixed: the body's
  reactions ran 9 K too cold after its upkeep, and the test left out the latent heat of the liquid the burning makes.
  Within 2% again.
- **The roster's big animals** (100 to 600 kg) die out in most runs.
- **Eater numbers swing up to 1,000 times** between almost identical runs.
- **Erosion is very slow in the first 10 years:** closed. The measure counted dead body matter piling up on the land as rock;
  without it the land loses 0.07 to 0.085 mm a year from the first year (handover, "Erosion at real rates").
- **The sea floor uses the air's oxygen,** and the sea's carbon reaction takes gas from the whole air
  column, not from the water.
- **Review 7 items not built:** the "held liquid is a cheap shortcut to size" fix (R4), `c_build`
  toward its anchor (R6), and liquid made in a body above its limit drains out (R11). Plants that
  walk (fix 7) is closed: moving needs force, plants move under 0.1 times a year (was 8 to 11).
  R11 was built and measured: it kills the eaters on all three seeds, because the rock salt in a body
  decays faster when it is less diluted. It is not landed; it waits for the owner (handover, Section 4).
- **A second, dearer body matter** (protein-like) for animals: an idea, not built.
- **The big-brain test:** no verdict yet.
- **All results are 60 years, 64 x 64.** The 300-year runs were never made.

## 13. Next steps (suggested order)

1. **The three 300-year runs at 128 x 128.** Check that life lasts. Check whether plants evolve
   their light gene, whether fixing goes down by itself, whether the seasons hold, and where the
   carbon settles.
2. **The small-cell world** for the nodes-against-individuals check, with plants included.
3. **Fix the open problems** that the runs confirm: the fixing cost, the heat-test gap, the roster's
   big animals.
4. **A simplify pass on the core** to get back under 8,000 lines, or accept the extra 7% with a
   reason.
5. **Check the real figures from memory** (erosion, porosity, fixing share, calcium share) against
   their sources.
6. **Then M4:** the planet with real seasons, frost and dry seasons, natural fire starts, and speed
   work. Disease after M4.
