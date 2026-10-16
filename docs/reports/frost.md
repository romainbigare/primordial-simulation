# Frost deaths: a study (16 Oct 2026, on main 4a3c4fc)

The owner asked what to do with the many cold-blooded deaths since real leaf pores made the island colder.
A study: measure, find causes, prototype general mechanisms, propose. Nothing is landed.
Runs: 64x64, 60 years, seeds 1 to 3, plain and roster. Deaths are means of years 41 to 60 (the tape's
`root_<k>_died_phase`, members a year); stocks are means of years 56 to 60; carbon gas from the year-60 snapshot.
The per-death numbers come from a probe (commits 432be30 and 4b15e1c, not for merge: `EW_FROST=1` prints every frost death and every
ground cell under 276 K from year 41).

## 1. Where, when and whom frost kills (main)

| | plain 1 / 2 / 3 | roster 1 / 2 / 3 |
|---|---|---|
| plants killed a year (share of members) | 1.7e8 / 1.9e8 / 8.9e7 (0.3 to 0.6%) | 1.8e8 / 1.0e8 / 1.7e8 (all light-eaters) |
| of them on ground at exactly 273.00 K | 89 / 97 / 95% | 79 / 82 / 83% |
| animals killed a year | 640 / 13,400 / 110 | 3.2e6 / 2.5e6 / 2.7e6 |
| land cells under 273 K at least once (of about 1,400) | 175 / 212 / 104 | 1,049 / 744 / 1,058 |
| days a year such a cell is under 273 K, p50 / p90 | 4 / 47, 1 / 16, 4 / 30 | 8 / 41, 8 / 42, 14 / 47 |

**Plants** (all kinds) die where the ground sits on its melting plateau: half frozen, held at 273.00 K by its fusion
enthalpy (Flow has latent heat for the ground), the plant 0.02 to 0.05 K under it (it transpires). A body has one
temperature, so all its liquid counts as frozen and it loses its activity's share (0.4 to 1). In a real plant the sap
holds solutes and freezes 0.5 to 2 K lower.

**Animals** die on really frozen ground: the ground at the last flow step 266 to 270 K (p50), the body 0.5 to 3 K
over it, fully active (activity 1), high cells (p50 340 to 1,660 m plain, 240 to 860 m roster), October to February.
The cell had been frozen for 3 to 6 days in a row (64 to 99% of them within 6 days), 30% of the animal's 10-day interval.
The air above is 6 to 7 K colder than the ground at those steps (p50): bodies relax to the ground, so they are already
"in shelter" against the air.

**Does the update sample a cold moment the body could not have reached?** No for the body itself: its time constant
(heat capacity / conductance) is 0.15 to 0.2 days for the 60 kg eaters, 0.05 days for the 2 to 5 kg roster animals,
0.001 days for plants. Any body follows its ground within hours, so its lumped temperature is right for that moment.
**But the cold moment of the ground is an artefact of the plants' updates.** A plant node (interval 19 to 25 days;
the nodes of a cell often update the same day) settles all the heat it drew since its last update with the ground at
once: mostly the latent heat of its transpiration, 2 to 5 K of the ground node (its 2 m skin and loose ground, 6.5e6 J/(m2 K)) per plant node,
every month of the year (75,667 settlements over 0.5 K in years 40 to 42, seed 2, all by plants). The ground of a cold
cell swings in a sawtooth with the plants' interval (cell 2406, seed 2, z 1,663 m: 258.8, 265.7, 270.9 K, then above
276, then 264.0 ... every 18 days). 99% of the eaters killed by frost in years 40 to 42 (seed 2) had a plant
settlement over 2 K in their cell in the 6 days before. The light the plants store had the same problem and was
solved by a pool (`World::light_pool`); the heat was not.

Bodies hold solutes already: 0.4 to 0.6 mol of the nitrogen salt and 0.02 to 0.13 of the dissolved lime per kg of
their liquid (plants and eaters alike, year 60). The sea holds 0.0008 mol/kg, the soil water 0.003 to 0.005.

## 2. The mechanisms judged

| | general? | anchored? | effect | code |
|---|---|---|---|---|
| (c) a body's heat drawn from the ground is held aside as time passes (the light pool, for heat) | yes: every body's exchange with its cell; a time-step fix, as the light pool was | no new number | the sawtooth goes; animals 3.5 to 6 times more in the roster (section 3) | +36 lines in core (prototype 1, 1142955) |
| (b) what is dissolved lowers the melting point (ideal solution) | yes: any liquid that holds anything; the same law as `t_sat` | from the liquid's own melting point and fusion enthalpy: 1.86 K kg/mol for water (measured 1.86) | bodies freeze about 1 K lower; plants' frost 1.4 to 20 times less (4 to 8 in the roster; section 3) | +18 lines in core (prototype 2, 606e2a9) |
| (a) latent heat of freezing in bodies | yes; Flow already has it for ground, lakes and sea (the plateaus) | yes, data (6.0 kJ/mol) | small here. A body's liquid holds L/c = 59 K of its heat (liquid 0.44 of its mass, 2,500 J/(kg K)), so it takes 59 K x tau / dT to freeze: 9 days per K under the melting point for a 60 kg eater, 3 for a 2.5 kg animal, minutes for a plant. The update holds the ground's temperature of the moment for its whole interval, so it helps only bodies over about 50 kg in spells shorter than their interval | 40 to 80 lines: the ledger already books a body's liquid with its fusion enthalpy (soaking adds it as heat), but about 50 readings of a body's temperature (`e / c`) would have to take it off |
| (b) in Flow (ground, lakes, sea) | yes | as above | none now: the sea holds 0.0008 mol/kg (0.0015 K), the soil water 0.003 to 0.005. Only with a salty sea (a chemistry change) | 15 to 25 lines in the plateau code |
| (d) a deeper, warmer ground | no missing physics found | | bodies already relax to the 2 m skin, not to the air, which is 6 to 7 K colder; a burrow adds nothing at this resolution | none |
| (e) death share = activity | partly physical (a dry body holds little free liquid) | | with (b) a drying body freezes lower by itself (its molality goes as 1 / its liquid: about 7 K at its safe minimum); with (a) the share that dies would be the share that froze, a cleaner form than activity. Not worth changing alone | with (a) |
| warm blood (`KEEP_T`) and fur (`HEAT`) | already in | | not selected in 60 years on any run | none |

## 3. The prototypes measured

main > (c) prototype 1 > (b) prototype 2 > both. Seeds 1 / 2 / 3. Energy drift at year 60 5e-14 to 1.3e-13 on every run.
`cargo test --release` passes with both.

| plain | main | (c) | (b) | (c) + (b) |
|---|---|---|---|---|
| coldest month of the land, K | 283.2 / 284.3 / 283.8 | 284.0 / 285.2 / 284.5 | 283.2 / 284.4 / 283.4 | 283.9 / 285.1 / 284.6 |
| land cells under 273 K at least once | 175 / 212 / 104 | 31 / 18 / 40 | 148 / 215 / 172 | 47 / 32 / 17 |
| cells with ice cover, % | 0.71 / 0.37 / 0.46 | 0 / 0 / 0.02 | 0.81 / 0.36 / 0.87 | 0.06 / 0.01 / 0.01 |
| carbon gas, ppm | 247 / 257 / 253 | 253 / 260 / 250 | 245 / 255 / 247 | 244 / 250 / 262 |
| plants, 1e10 kg | 3.6 / 3.4 / 3.7 | 3.7 / 3.6 / 4.3 | 3.7 / 3.1 / 4.0 | 4.2 / 3.2 / 3.8 |
| plants killed by frost, % of members a year | 0.48 / 0.61 / 0.27 | 0.49 / 0.19 / 0.45 | 0.10 / 0.03 / 0.20 | 0.28 / 0.06 / 0.09 |
| eaters, 1e7 kg | 2.2 / 11 / 4.0 | 10 / 13 / 7.4 | 8.8 / 10 / 2.4 | 10 / 7.2 / 3.3 |
| eaters killed by frost a year (% of members) | 640 / 13,400 / 110 (0.38 / 1.6 / 0.04) | 3,300 / 150 / 680 (0.47 / 0.01 / 0.13) | 5,000 / 1,900 / 840 (0.86 / 0.23 / 0.56) | 1,000 / 2,200 / 29 (0.13 / 0.40 / 0.01) |

| roster | main | (c) | (b) | (c) + (b) |
|---|---|---|---|---|
| coldest month of the land, K | 280.3 / 282.2 / 279.9 | 282.2 / 283.4 / 282.7 | 280.6 / 281.4 / 280.4 | 281.9 / 283.3 / 282.8 |
| cells with ice cover, % | 4.4 / 2.9 / 5.5 | 0.10 / 0.07 / 0.12 | 4.1 / 3.6 / 4.9 | 0.18 / 0.08 / 0.15 |
| carbon gas, ppm | 142 / 176 / 137 | 144 / 166 / 148 | 145 / 159 / 151 | 139 / 160 / 148 |
| light-eaters, 1e10 kg | 9.9 / 8.5 / 11 | 9.7 / 11 / 10 | 9.2 / 9.0 / 11 | 11 / 11 / 11 |
| light-eaters killed by frost, % a year | 1.5 / 1.1 / 1.7 | 0.50 / 0.18 / 0.74 | 0.24 / 0.14 / 0.40 | 0.35 / 0.08 / 0.48 |
| animals, 1e8 kg | 4.5 / 5.1 / 2.9 | 16 / 20 / 16 | 5.4 / 9.2 / 6.8 | 19 / 19 / 18 |
| animals killed by frost, % of members a year | 6.2 / 4.9 / 9.6 | 0.98 / 0.69 / 0.83 | 4.8 / 3.5 / 16 | 1.3 / 0.28 / 0.85 |
| eaters (kind 1), members | 9.9e3 / 9.5e3 / 2.2e5 | 8.0e4 / 1.3e7 / 9.2e5 | 1.9e5 / 3.9e5 / 6.4e6 | 2.1e5 / 1.8e6 / 3.1e6 |
| big kinds (8, 9, 10) alive at year 60 | none / none / 8, 10 | 8 / 8 / 8, 10 | none / 9 / 10 | 9 / 8, 10 / 8, 10 |

**What it says.** (c) is what the animals needed: with the ground paid as it goes, the cold cells stop freezing in
dips, the ice cover nearly goes (it reflects 85% of the light), and the coldest month comes back 0.6 to 0.8 K (plain)
and 1.3 to 2.8 K (roster), a large part of what the leaf pores took (1.7 to 2.0 K and 3 to 5 K). The roster's animals
are 3.5 to 6 times more by mass, their frost rate 6 to 12 times lower, the big kinds kept on all seeds. On the plain
island frost was never what held the eaters (0.04 to 1.6% a year); their mass moves within the spread of the seeds.
(b) takes away the plants' deaths on the melting plateau (79 to 97% of them before) and leaves the climate as it was;
alone it does little for animals (74 to 86% of them died more than 1 K under the melting point). What frost still kills with both is
real: a few high cells (17 to 47 plain) under 258 to 270 K for 6 to 21 days in a row, January to March.
Speed: 3.94 > 4.09 s a year with both (+4%; a paired run, seed 2, 30 years, two runs at once).

## 4. Recommendation (the owner decides)

1. **(c), the heat pool.** It is a bug of the time step, not a choice: the plants' heat was settled late and all at
   once, the light already is not. It needs no number. It brings back much of the warmth the leaf pores took, and the
   roster's animals and big kinds. Cost: +36 lines; two fields more in the snapshot (old snapshots load them empty).
2. **(b), what is dissolved lowers the melting point,** for bodies. Plain chemistry from the data, one law shared with
   `t_sat`; it makes antifreeze something genes can reach (the level a body keeps of a salt), and makes a drying body
   freeze lower. Cost: +18 lines. Later the same `t_melt` can serve Flow, when the sea has a salt.
3. **(a), latent heat in bodies,** at M4 when real winters come: a body would take days to freeze, not hours. It needs a
   body's heat to hold the fusion enthalpy and every reading of its temperature to take it off (about 50 places);
   (e) then becomes "the share that froze dies".
4. Not (d): no missing physics found.

Doubts. (1) Why (c) warms the summer too (+0.8 to 1.1 K plain, +1.6 to 2.4 K roster) is not fully explained: less ice
cover is measured, the rest of the path is not. (2) Plain-island eater numbers differ by seed more than by mechanism.
(3) Each body's held heat follows its rate at its last update: a body that changes its rate a lot between updates
settles the change at once, as before.
