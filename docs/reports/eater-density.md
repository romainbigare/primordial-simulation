# How crowded the eaters are, against real animals (16 Oct 2026)

A study. Nothing in the laws changed. Main f94a700, 64 x 64, 60 years, seeds 1 / 2 / 3. "R11" is the one change of
e13d911 (the key liquid past its level drains out in `body::shed`) on the same main. The numbers per year come from
a probe (`eater-density-probe.patch`, not merged: tallies of the light energy plants store, the upkeep paid, the
body matter built and the tissue-days, per starting kind; a snapshot reader; a diagnostic switch `EW_NO_LITTER`).
The probe leaves the history the same bit for bit (a resumed run with it gave the same eater kg at years 55 to 60).

## The land

Cells are 2 km (4 km2). Land (above the sea) is 1,567 / 1,351 / 1,436 of the 4,096 cells: 6,268 / 5,404 / 5,744 km2.

## The numbers at year 60 (year 60 means of the tape; composition from the snapshot)

| | seed 1 | seed 2 | seed 3 |
|---|---|---|---|
| eaters, main: t/km2 (member kg) | 267 (125) | 231 (122) | 351 (132) |
| of their mass: liquid past the level / liquid kept / sugar / body matter | 63 / 15 / 16 / 5% | 61 / 16 / 17 / 5% | 67 / 14 / 14 / 4.5% |
| main without the liquid past the level (static), t/km2 | 100 | 89 | 116 |
| main, living tissue (`Body::built`, matter over 0.25), t/km2 | 51 | 46 | 59 |
| eaters with R11: t/km2 (member kg) | 49 (47) | 72 (45) | 34 (48) |
| R11: tissue t/km2 (share of mass); the rest is sugar | 25 (54%) | 39 (54%) | 17 (53%) |
| growth of eater kg, years 50 to 60, per year: main / R11 | 0.17 / 0.13 | 0.14 / 0.11 | 0.16 / 0.16 |
| plants: standing kg/m2 fresh (liquid share) | 6.3 (65%) | 5.3 (61%) | 6.8 (67%) |
| plants: dry kg/m2 (body matter + stored sugar) | 2.15 (0.74 + 1.40) | 1.98 (0.64 + 1.34) | 2.17 (0.69 + 1.49) |
| plants: gross production (sugar made by light) kg/m2/yr | 5.7 | 5.6 | 6.1 |
| plants: net production (gross less upkeep and building work), kg dry/m2/yr | 1.94 | 1.90 | 2.22 |
| dead plant matter on the ground, kg dry/m2 | 2.15 | 1.98 | 1.95 |
| eaters eat, g/m2/yr (share of net production) | 225 (11.6%) | 225 (11.8%) | 278 (12.5%) |
| of that, from living plants | 0.5% | 5.4% | 1.4% |
| eater upkeep, kJ per kg of tissue per day (year mean, at 286 to 288 K) | 162 | 178 | 170 |
| eater food, kJ per kg of tissue per day | 190 | 209 | 201 |
| body matter made / food eaten (production efficiency) | 4.4% | 4.5% | 4.5% |

With R11 the eaters eat 4.4 to 10.7% of net production, pay 175 to 191 kJ per kg of tissue a day, and keep 4.5%.

## Against real figures

Real figures (sources; "from memory" where I could not check):
- Large mammal herbivores, Africa: 53 to 11,810 kg/km2, about 1,700 typical without elephants (Hempson et al. 2015, Science 350).
- Large ectotherm herbivores: Aldabra giant tortoises up to 58 t/km2, "more than the large mammal herbivores of Africa"
  (Science Advances, sciadv.abq2574; it cites Coe, Bourn and Swingland 1979, not checked). Grasshoppers 0.1 to 3 t/km2,
  earthworms (eaters of dead plant matter) 10 to 100 t/km2 or more (both from memory).
- Cost of living at 25 kg: Kleiber's basal 3.4 m^0.75 W = 133 kJ/kg/day (checked, `docs/numbers.md`); field rate of
  mammals 320 to 420 kJ/kg/day (Nagy 1994 and 1999, from memory); of reptiles about 65 kJ/kg/day at their active
  temperature, about half that at 287 K (Nagy et al. 1999, from memory).
- Production efficiency (body made / food): mammals 1 to 3%, ectotherms 10 to 30% (from memory).
- Net production: grassland 0.2 to 1.5, savanna about 0.9, temperate forest about 1.2 kg dry/m2/yr; the Miami model
  gives about 1.1 for 14 C and 0.7 m of rain (Whittaker, Lieth, from memory). Standing dry mass: grassland 0.2 to 2,
  forest 10 to 50 kg/m2. Plants store 5 to 20% of their dry mass as sugar and starch (from memory).
- Share of plant production eaten by animals: grasslands 10 to 50% of the living plants' production, forests under 10%
  (Cebrian 1999, from memory). Dead plant matter goes mostly to microbes and small animals; large animals cannot live on it.

What follows:
1. **The cost of living is not the cause.** 162 to 191 kJ per kg of tissue a day is 1.2 to 1.4 times Kleiber's basal,
   0.4 to 0.6 times a mammal's field rate and 2.5 to 3 times a reptile's (5 to 6 times a reptile at the same 287 K).
   The eaters are cold-blooded (1.2 K over the ground). Against ectotherms they pay too much, which makes them fewer.
2. **Food per kg is what an ectotherm needs.** Eating 190 to 221 kJ a day per kg of tissue, a real reptile would hold
   1.2 to 2 times more tissue on the same food. Production efficiency 4.5% sits between mammals and ectotherms.
3. **So "20 to 100 times" is mostly the wrong comparison plus the liquid.** Seed 1 against 1,700 kg/km2 is 157 times.
   The liquid past the level is a factor 2.7 (R11 removes it). The sugar store is 16% (main) to 45% (R11) of the mass.
   The living tissue is 46 to 59 t/km2 (main) and 17 to 39 (R11): against mammals that is 10 to 35 times typical and
   1.5 to 5 times the highest square; against the best ectotherm figure (58 t/km2) it is 0.3 to 1 time. **At year 60,
   counted as tissue and against cold-blooded animals, no excess is left.**
4. **The real fault is what they eat.** 95 to 99.5% of their food is dead plant matter from the ground; living plants
   give 0.06 to 0.6% of the plants' production. **Diagnostic (seed 1, `EW_NO_LITTER`: no body takes the solid or body
   matter from the ground): the eaters die out by year 38** (868 founders, 1,030 members at most). On living plants
   alone a founder took about its upkeep (32 kg a year against 39 on main in years 23 to 25) and bit a plant about 6
   times a year; why so seldom, the choice or the bite, I did not separate. In the model a dead plant is the same body
   matter and sugar as a living one, and as an animal: digested in full at 15.6 to 15.9 MJ/kg. Its only rival is the
   open-world decay (0.2 to 0.27 a year at 290 K, anchored to a forest floor). Real dead plant matter is mostly cellulose
   and lignin, poor in nitrogen, and the microbes that live on it keep most of its energy.
5. **They have not levelled off, and the level would be high.** A body takes what fits its room (a full store is its
   satiety); food density limits it only under about 0.05 kg/m2 (`reach`: `sweep` 1 m2/kg/day x density against `take`
   0.05). The dead matter is 2 kg/m2 and has fallen 17% since year 50 (seed 1: 2.46 at year 50, 2.15 at 60; 2.59 in the
   no-eater world). So nothing stops them before they take most of the plants' net production: 1.9 kg/m2/yr at about
   200 kJ/kg/day is about 410 t/km2 of tissue, 8 times year 60, 2 kg/m2 of whole mass on main (0.8 with R11). At 0.11 to
   0.17 a year that is about 15 years more. An extrapolation, not a run. No predator, no disease and no lack yet.
6. **Plants**: the plant mass includes its liquid (61 to 67%). Dry 2.0 to 2.2 kg/m2 is at the top of grassland. Net
   production 1.9 to 2.2 is about twice the Miami model for this climate; gross 5.6 to 6.1 is high; net over gross 0.34
   (real about 0.5). Two thirds of their dry mass is stored sugar. The dead matter turns over about 0.7 a year in a world
   without eaters (input about the net production, stock 2.6 kg/m2), faster than the decay constants; I did not find the
   other way out.

## Answers

(1) **R11 is part of the answer.** 61 to 67% of the eaters' kg is liquid past the level; without it 89 to 116 t/km2
remain; R11 runs give 34 to 72 t/km2 (also fewer members). It does not touch the food, so the level they will reach
stays about 8 times year 60.
(2) **The cause is the food, not the cost of living.** The eaters are 25 kg cold-blooded eaters of dead plant matter.
What must change, if the owner wants plant-eaters of living plants: (a) the chemistry: plants build of the same body
matter as animals, so dead plant matter feeds them like meat; a structural plant matter that only a catalyst (with its
real price) breaks down would make dead matter food for whatever evolves it (a decision: a new substance, its numbers);
(b) find why living plants alone cannot feed the founders (a study). R11 is the owner's (one commit). Warm blood is
another agent's work: with mammal costs (2 to 2.5 times now) the same food holds 2 to 2.5 times less tissue.
