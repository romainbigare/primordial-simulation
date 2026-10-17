# What a run records (the tape)

Columns may change in the M3 clean-up; this list matches commit 154554a.

This list gives the main columns only. Columns made for one study are left out; they will be
removed in the clean-up. The code that writes them is `tape/src/rec.rs`.

## Files of a run (`ew run --out DIR`)

| File | What it holds |
|---|---|
| `snap_NNNNN.bin.gz` | the full world every 10 years (bincode, then gzip); `ew resume --from` reads it |
| `series.csv` | one row a year, the columns below |
| `months.csv` | one row a month (below) |
| `physics.json` | the run's `Physics` and `Config`; `ew numbers OLD NEW` compares two runs |
| `world.ewf.gz` | the viewer's file: map frames, the series, and the raw events |
| `yearNNNN.png` | a map picture every `--frames-every` years |

**Raw events** (`core/src/event.rs`): born, died (with cause, kind, adult or young), took,
gave, watched, bond made or broken, first time a substance appears. Only physical facts.

**Map layers** in the frames (`tape/src/layers.rs`): height, liquid depth, ice, liquid flow,
loose ground, ground and air temperature, rain, vapour, absorber, plant and animal mass per m²,
bodies, traffic, worn edges, structures, true colour (r, g, b).

## `series.csv`: the world

Names use numbers, never Earth names. `<j>` is an element, `<k>` a substance, `<r>` a reaction,
`<q>` a mineral (0 the nitrogen mineral, 1 the rock salt).

| Column | Unit | Meaning |
|---|---|---|
| `year`, `null` | | the year; 1 in a null run |
| `elem_<j>` | mol | total of element j in the world (must not change) |
| `energy` | J | total heat and chemical energy |
| `t_ground`, `t_air` | K | mean ground and air temperature |
| `t_land_summer`, `t_land_winter`, `t_land_swing` | K | land in its warmest and coldest month, and the difference |
| `absorber` | | mean absorption of the heat-absorbing gases |
| `air_<k>` | | share of the air that is heat-absorbing gas k |
| `ice_frac` | | share of cells under ice |
| `rain` | mol/m²/s | mean rain |
| `liquid_land`, `stand_land` | km³ | liquid on land; of it, standing (not in pores) |
| `min<q>_land`, `min<q>_sea` | kg/m² | mineral q in the ground of land and of the sea floor |
| `bodies`, `plants`, `animals` | | bodies (individuals and nodes), light-eaters, the others |
| `plant_members`, `animal_members` | | members of each |
| `body_mass` | t | mass of all bodies |
| `births` | 1/yr | members born |
| `died_lack`, `died_phase`, `died_spent`, `died_taken`, `died_broken`, `died_age` | 1/yr | members died, by cause |
| `lineages`, `kinds`, `kinds_members` | | brain lineages; kinds found by clustering genes (bodies, members) |
| `mass_min`, `mass_median`, `mass_max`, `mass_max_root` | kg | member mass; the kind of the heaviest |
| `cat_<r>` | log10 | mean catalyst gene of reaction r over all members (mixes kinds; read the per-kind columns) |

**Movement, giving, imitation, signals**

| Column | Unit | Meaning |
|---|---|---|
| `crossings` | 1/yr | moves of bodies to a neighbour cell |
| `traffic_top2`, `traffic_persist` | | share of crossings on the busiest 2% of cells; overlap with ten years before |
| `worn_edges`, `worn_new`, `worn_persist` | | edges cleared by movers (path-like), new ones, kept ones |
| `gave`, `gave_kg`, `gave_pairs`, `gave_reciprocal` | | gifts, kg given, giving pairs, pairs giving both ways |
| `watched`, `watched_groups` | | watches (imitation); groups reached |
| `learned`, `hidden`, `memory`, `seen`, `sight` | | how far learning moved brains; mean brain sizes |
| `mi_cold`, `mi_hurt`, `mi_low` | bit | how much signals say about cold, harm and low well-being |

**Parts and structures**: `parts`, `parts_held`, `structures`, `enclosed`, `bonds_joined`,
`bonds_broken`, `inside_now` (bodies inside a structure), `afloat`.

## `series.csv`: by starting kind

`root_<k>_*`: `<k>` is the row of the starting kind a body descends from. Means are by
members. A kind that died out reads 0 (its means `nan`).

| Column | Unit | Meaning |
|---|---|---|
| `members`, `bodies`, `kg` | | members, bodies, whole mass |
| `births`, `died_<cause>` | 1/yr | births and deaths by cause (they sum to the world columns) |
| `died_young`, `died_adult` | 1/yr | deaths below and at or past the copy mass |
| `life_real`, `life_adult` | yr | members standing over deaths in the year (all; adults only) |
| `life` | yr | the life the genes allow at the upkeep now |
| `mass`, `body`, `copy_mass`, `matter` | kg | mean member mass, body (`Body::size`), copy mass; matter share of mass |
| `past_copy` | | share of members that are mature |
| `activity`, `dry` | | mean activity; share below 0.5 |
| `temp` | K | how much warmer than its ground (bodies updated in the last 5 days) |
| `moves` | 1/yr | moves per member |
| `mixed` | | share of births with two parents |
| `gas`, `leaf`, `copy_log`, `interval`, `heat`, `repair`, `wet_gene`, `child` | log10 | gene means |
| `hidden`, `alloc`, `keep_t`, `cool_t` | | gene means (units, share, K, K) |
| `cat_<r>`, `cat_<r>_on` | log10, share | catalyst gene of reaction r over the members that carry it on; share that carry it |

## `months.csv`

One row a month: `year`, `month`, then for each starting kind `root_<k>_births` and
`root_<k>_died_<cause>` (members in the month), so seasons of births and deaths show. The first
columns (`mobile`, `mean_y`, `mean_z`, …) follow the animals' mean position and moves.
