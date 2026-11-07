# What a run records (the tape)

The code that writes it is `tape/src/rec/` (the yearly columns in `tape/src/rec/cols/`). Names use numbers, never
Earth names: `<j>` is an element, `<k>` a substance, `<r>` a reaction, `<q>` a mineral (`Chemistry::minerals`: 0
the mineral of the fixing reaction's gas, 1 the second), `<K>` a starting kind (a row of `worldgen::life::KINDS`).
A mean of nothing is written `NaN` (`null` in the viewer's file, drawn as a gap).

## Files of a run (`ew run --out DIR`)

| File | What it holds |
|---|---|
| `snap_NNNNN.bin.gz` | at the end of the run, and every `--snapshots-every` years if given: the whole world, the run's plan (what is placed when, and the camera's path), and what the tape keeps across years (gzip of bincode, after the tag `EWSN` and the format version); `ew resume --from` goes on from it (the budget of cells and the floor in the world's `Config`, the rest of the camera's path in the plan), and a resumed tape equals the unbroken one. A snapshot is taken at a month's start, so it holds the state and no cache: what the laws build again before they read it at a month's first step (Flow's caches, `flow::Scratch`; the survey's `env` and `seen`; the survey's sums of the cells, `Life::sum`, and the numbers each body's genes give, `Body::k`, both worked out again at load, as the tape reads the sums first) is not saved |
| `series.csv` | one row a year, the columns below |
| `months.csv` | one row a month (below); only with `--months` |
| `physics.json` | the run's `Physics` and `Config`; `ew numbers OLD NEW` compares two runs |
| `world.ewf.gz` | the viewer's file: `EWF2`, the header's length, the header (JSON: size, whether the world is a globe and its edge latitudes, layers, frames, series and units, events, the months rows, the camera's path), then the frames: one every `--frames-every` years (10), and one a month in the first `--monthly-years` years (2) |
| `yearNNNN.png` | map, temperature, rain, life, traffic and the depth of the cells (each square by the cell at its centre: a square light grey, coarser dark, finer warmer) every `--pictures-every` years, at the start and at the end; none by default |

**How the files are written.** Every 10 years (`TAPE_EVERY` in `cli/src/main.rs`), at every snapshot and at the run's
end the shell copies the tape and, at a snapshot, makes the snapshot's bytes, and a writer thread (`cli/src/writer.rs`) compresses and writes them while the world goes on: at most one job at a
time, and the run waits for it before the next job and at its end. A write error stops the run at the end of the year
it happens in. The job writes the tape first and the snapshot last, so a tape is never behind a snapshot, and
`ew resume` refuses a tape whose months stop before the snapshot's. `world.ewf.gz` is written again whole (gzip at its
fast level; the viewer reads any level) and `physics.json`. `series.csv` and `months.csv` are not written again: a file
the writer wrote before gets only the rows added since, at its end. A file is written whole when its header line
changed (a column, or a starting kind, that is new) and at the first write of a resumed run, which cuts the file back to
the rows the snapshot's day holds. A snapshot and the viewer's file go to a `.part` file that takes the name when whole;
a run killed during a write leaves its `.part` file, which nothing reads. A stopped run keeps the tape up to its last
write, and resumes only from a snapshot.

**Raw events** (`core/src/event.rs`): born (with kind, one or two parents), died (with cause, kind,
the light catalyst of the dead), took, gave, watched, bond made or broken, first time a substance appears, a cell split
into its quarters or joined from them (named by its serial, with its depth; counted in `splits` and `joins`). Only physical facts; the viewer shows the first appearances.
Took, gave and watched name bodies by their id (`Body::id`): the serial of the cell the body was placed in (laws.md
0) times 2^40, plus that cell's count of bodies placed then (1, 2, ...: children,
split halves, parts of nodes acting apart, founders). An id never
changes and is never given again, wherever the body goes. What the recorder keeps across months and years of cells
(where each body was at the last month, the busiest cells and the worn edges a decade ago) names them by serial, so a
split or a join of other cells leaves it whole; the land is listed again after a split or a join.

**The camera's path** (`ew run --camera FILE`, `refine::Camera`) is an input of the run, as the placements of life
are: refinement reads it at each month's start (laws.md 0), so the same seed, flags and path give the same world. The
viewer's file holds it as `camera`, a list of shots in the order of their months, each `{month, x, y, w, h, depth, rest}`:
from month `month` on (whole months since the start) the window from column `x` and row `y` of the map, `w` x `h`
squares, asks for cells of `depth` below their air block (a square of an island is depth 2); a window of no area is
the camera gone, and with `rest` the budget of cells is the cells as built from that month on (laws.md 0). A snapshot carries it in the run's plan.

**A globe in the viewer's file.** The header has `globe` (the rows wrap east to west), `lat_north` and `lat_south` (the
latitudes of the north and south edges, degrees; 90 and -90 on a globe). The grid is the map: row `y` of `h` lies at
latitude `lat_north + (lat_south - lat_north) * (y + 0.5) / h`, column `x` of `w` at longitude `(x + 0.5) * 360 / w`
degrees east of the map's west edge. A cell's width shrinks with the cosine of its latitude; `cell_m` is its height
(and its width at the equator). The viewer draws the grid as a map (2:1), with a line every 30 degrees of latitude, the
map repeating east and west, and a cell's latitude and longitude in the inspect panel. An island has `globe` false and its
`lat_*` are the latitudes it lies between (not shown).

**Means over the map** (`t_ground`, `t_air`, `t_land`, `absorber`, `ice_frac`, `rain`, `rain_land`, `mass_top2`,
`mass_land`) are weighted by cell area, as the cells of a globe differ in area (`measure::area_mean`). Counts of cells
(`rivers`, the cover's median) are of cells (a cell of many squares counts one).

**Map layers** in the frames (`tape/src/layers.rs`, one byte a square of the map, row by row; a square shows the cell
that holds its centre, `tape::pixels`, as do the PNG pictures): height, liquid depth, ice, liquid flux, `catch` (the upstream area a cell drains
in units of its own: its liquid flux over its rain less evaporation times its area, so a river reads the same at any cell
size), loose ground, ground and air temperature, rain, vapour, absorber, the cover's mass, its walls (`wood`) and its capture area (`leaf`) per m², the bodies' mass per m², bodies, traffic,
worn edges, structures, true colour (r, g, b). The tape holds whole-map frames only. The live viewer also asks for
the window in view finer than its cells (`tape::fine`, the layout in `tape/src/fine.rs`), a pixel a leaf of a depth
2 to 4 below the cells, at any zoom: of each pixel the ground's height, the surface of the liquid standing in the cells
about it, and every layer blended between the cells' centres (each cell weighing a pixel by the quadratic B-spline of its distance in its own sides, Shepard 1968, so the blend is smooth with its slope) (the ground's temperature along the ground's
height, at the rate the air cools with height, `World::air_drop`), and the way each cell's liquid runs (to the cell the
steepest way down its head takes the most of, as the liquid law sends it). The ground of a pixel is the terrain there
(`space::terrain`: the mean of its samples, or finer than them the midpoint displacement's departures laid on a
quadratic continuation of the coarser heights that keeps each leaf's mean) plus an offset blended between the cells'
centres: each cell's height less the terrain's mean over it, made so the blend keeps each cell's height as its mean.
The terrain's samples are found once (`tape::Samples`). The viewer paints it over the whole-map frame: hill shading
from the fine ground, the liquid where its surface lies over the fine ground (the sea's tint by the depth there),
snow by the pixel's own temperature near the cover's melting point, rivers as lines along those ways, as wide as
their discharge (Leopold and Maddock 1953) or wider; it is not recorded. Close up it also asks
for the things of the cells in view (`tape::things`, the record layout in `tape/src/things.rs`): each body (members,
a member's mass, strength, colour, coat, copy mass, the temperatures it keeps, what it takes from where, its kind,
stage and brain, the part it holds and the structure it is inside), each slot of cover (its kind, built cover and capture area per m²
and genes) and each part, or lot of alike parts (laws.md 3.1: its count, each part's sizes, hollowness and mass, its
walls' colour and hardness, burial, holder, structure and the space it closes) with the bonds; it is drawn as avatars,
a lot part by part at places keyed by its id and each part's index, and is not recorded.

## `series.csv`: the world

| Column | Unit | Meaning |
|---|---|---|
| `year`, `null` | yr, | the year; 1 in a null run |
| `elem_<j>` | mol | element j in the world (must not change) |
| `energy` | J | heat and chemical energy in the world |
| `min<q>_land`, `min<q>_sea` | kg/m² | mineral q in the ground of land and of the sea floor |
| `min<q>_dead_land`, `min<q>_dead_sea`, `min<q>_bodies`, `min<q>_cover`, `min<q>_parts`, `min<q>_deep` | 10⁹ mol | mineral q in dead matter on land and sea, in bodies, in the cover, in parts (walls and what they hold), in the bed's stores |
| `t_ground`, `t_air` | K | mean ground and air temperature (by area) |
| `t_land`, `t_land_summer`, `t_land_winter` | K | the land's ground: the mean of the year's months, the warmest month and the coldest |
| `absorber` | | mean absorption of the heat-absorbing gases |
| `air_<k>` | | mole fraction of heat-absorbing gas k in the air |
| `ice_frac` | | share of the area under ice |
| `liquid_land` | km³ | liquid on land |
| `ice_land` | km³ | frozen cover on land |
| `rain`, `rain_land` | mol/m²/s | mean rain, and on land (a moving average over about a year) |
| `evap_land`, `runoff_sea` | mol/m²/s | on land, what rises into the air less what condenses on the ground, and what runs over the surface into cells under the sea's level, per m² of land (moving averages over about a year): with `rain_land` the land's water budget |
| `rivers` | | dry land cells (under at most `tape::STANDING_M` of liquid) that are rivers (`tape::is_river`): a liquid flux over `tape::RIVER_FLOOR` and over `tape::RIVER_CATCH` times what the cell's own area sheds (`catch`). The viewers and the report count the same way |
| `cells`, `splits`, `joins` | , 1/yr, 1/yr | the cells of the map at the year's end; cells split into their quarters and cells joined from them in the year (`World::split`, `World::join`, as refinement asks, `World::refine`) |
| `bodies` | | bodies (nodes and individuals) |
| `cover_slots`, `cover_kg` | , kg | slots of cover in the world (`cover::SLOTS` a cell at most), and its mass |
| `cover_died_<cause>` | kg/yr | cover that died in the year by cause: `spent` (upkeep unpaid), `age`, `phase` (frost), `taken` (eaten), `broken` (by movers) |
| `animal_members`, `animal_nodes` | | members of the bodies; nodes |
| `cover_median`, `cover_max` | kg/m² | the cover's mass on a land cell: median and largest |
| `body_mass` | t | mass of all bodies |
| `dead_<k>` | kg/m² | dead fuel, matter and built solids on the ground of the land |
| `births`, `died_<cause>` | 1/yr | members born; died by cause (`lack`, `phase`, `spent`, `taken`, `broken`, `age`) |
| `lineages`, `lineages_ever`, `updates` | , , 1/yr | brain lineages alive and ever; body updates |
| `busiest_chunk` | | the busiest chunk's share of a batch's body updates in the parallel pass (its bodies over all of the pass's), the year's mean over its batches: how evenly the chunks share the work |
| `cat_<r>` | log10 | mean diet gene of reaction r over all members (on above 0: the body runs or digests by that reaction; mixes kinds, read the per-kind columns); 0 for a reaction bodies may not run (laws.md 6.1) |

**Brains and kinds**

| Column | Unit | Meaning |
|---|---|---|
| `learned`, `greedy` | | how far learning moved the brains; share of decisions that drew the top-scored action |
| `hidden`, `memory`, `seen`, `sight`, `hidden_max` | , , , cells, | mean brain and sense sizes; the largest hidden layer |
| `mass_min`, `mass_median`, `mass_max`, `mass_max_root` | kg | member mass; the kind of the heaviest |
| `trophic_max`, `trophic_high` | | share of what a starting kind took from bodies that came from eaters: the kind with most bodies; the highest |

**Movement, imitation, giving**

| Column | Unit | Meaning |
|---|---|---|
| `eat_animals` | | bodies that took more from other bodies than from the cover in their life |
| `crossings`, `traffic_top2`, `traffic_persist` | 1/yr, , | moves to a neighbour cell; share on the busiest 2% of cells; overlap with ten years before |
| `worn_edges`, `worn_new`, `worn_persist` | , 1/yr, | edges cleared by movers (path-like), new ones, kept from ten years before |
| `mass_top2`, `mass_land` | kg/m² | body mass on the busiest cells and on other land |
| `watched`, `watched_kin`, `watched_bodies`, `spread_max` | 1/yr, 1/yr, , bodies | watches (imitation), of them within a lineage; bodies watched; most watchers of one |
| `gave`, `gave_kg`, `gave_eaters`, `gave_pairs`, `gave_reciprocal` | 1/yr, kg/yr, 1/yr, , | gifts, kg given, gifts by eaters, giving pairs, pairs giving both ways |
| `ate_cover`, `ate_others`, `ate_ground` | kg/yr | what eaters took of what they use (`Body::uses`: their food), by source: the cover, other bodies, the ground |

**Parts and structures**

| Column | Unit | Meaning |
|---|---|---|
| `form_<k>` | 1/yr | members that drew "form" in the year, by starting kind k (`Lineage::root`; every kind seen keeps its column) |
| `form_size_<c>` | 1/yr | the same by size class c of the brain: 2^c to 2^(c+1) - 1 hidden units (the last class all larger) |
| `form_learned`, `form_chance` | 1/yr | the same by whether "form" had the top score when drawn (learned) or not (chance) |
| `parts`, `part_records`, `parts_held`, `parts_mass`, `parts_formed` | , , , t, 1/yr | made objects, each part of a lot counted (laws.md 3.1); the records that hold them; of them held; their mass; formed in the year |
| `put_to_parts`, `taken_from_parts` | kg/yr | stores put into and taken from parts |
| `edge_kg` | kg/yr | what bites took from bodies and the cover where a held edge cut (laws.md 3.3) |
| `bonds_joined`, `structures`, `enclosed` | | bonds between parts; structures of more than one part; enclosed ones (each part of a lot one) |
| `made_held`, `made_joined`, `bonds_broken`, `bonds_failed` | 1/yr | bonds made by kind; broken by bodies; failed by load |
| `inside_now`, `inside_cold`, `inside_warm`, `cold_months` | , , , 1/yr | bodies inside walls now; share of the eaters' body-months inside in the cold and the warm; their cold months |
| `holding_liquid`, `liquid_held` | , kg | parts holding liquid and how much |

**Time per process** (measured by the shell, never by the core; NaN in the first row and in a run with no clock)

| Column | Unit | Meaning |
|---|---|---|
| `ms_flow`, `ms_change`, `ms_bodies`, `ms_parts`, `ms_refine` | ms/yr | wall time in Flow, in Change, in the bodies (survey and body batches), in the parts and in refinement (`World::refine`, its splits and joins) since the last row |
| `ms_tape` | ms/yr | wall time of the main thread in the tape itself since the last row: the months recorded, the snapshot's bytes and the copy of the tape that go to the writer thread, and any wait for it (the writer's own time runs beside the world and is not counted). The row's own month, and the snapshot and copy made after the row (at a write of the tape), land in the next row. The pictures of a year (every `--pictures-every` years) are in it; the yearly check is not timed |

## `series.csv`: by starting kind

`root_<K>_*`, for every kind up to the largest seen; a kind that died out reads 0 (its means `NaN`). Means are by
members; for a kind of cover by its built cover.

| Column | Unit | Meaning |
|---|---|---|
| `members`, `kg`, `bodies` | , kg, | members, their mass, bodies |
| `life`, `life_real` | yr | the life the genes allow at the upkeep now while active (the age law's wear of the built body); members standing over deaths in the year |
| `births`, `mixed`, `died_<cause>` | 1/yr, , 1/yr | births; share with two parents; deaths by cause (they sum to the world columns) |
| `mass`, `copy_mass`, `body`, `matter` | kg, kg, kg, | mean member mass, copy mass, body (`Body::size`); body matter's share of the mass |
| `activity`, `dry` | | mean activity; share below 0.5 |
| `alloc`, `child`, `brood` | , log10, | genes of reproduction; brood as a share of the mass |
| `copy_log`, `repair`, `wet_gene` | log10 | gene means |
| `heat` | | gene mean: the share of the best coat (`HEAT`) |
| `hidden`, `keep_t` | , K | gene means |
| `cat_<r>`, `cat_<r>_on` | log10, | diet gene of reaction r over the members that carry it on (above 0); share that do |
| `slots`, `cover_kg`, `cover_area` | , kg, m²/m² | a kind of cover: its slots, its mass, its capture area per m² of land |
| `cover_light`, `cover_leaf`, `cover_walls`, `cover_store`, `cover_moist`, `cover_repair`, `cover_fix` | log10 | the means of its genes (`cover::LIGHT` ...) |
| `cover_frost`, `cover_seed` | K, | the means of its frost and seed genes |

## `months.csv`

Written with `--months`. One row a month: `year`, `month`; the bodies: `mobile` (how many), `mean_y`, `mean_z` (mean
row and height), `mean_dy`, `mean_dz` (mean move since last month), `inside` (inside walls), `t_mobile` (mean ground
temperature under them, K), `spent`, `spent_y` (members died of `spent`, and their mean row); then for each starting
kind `root_<K>_births` and `root_<K>_died_<cause>` (members in the month), so seasons of births and deaths show.
