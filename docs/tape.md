# What a run records (the tape)

The code that writes it is `tape/src/rec/` (the yearly columns in `tape/src/rec/cols/`), the map layers in
`tape/src/layers.rs`, the shell's writes in `cli/src/main.rs` and `cli/src/writer.rs`. Names use numbers, never Earth
names: `<j>` is an element, `<k>` a substance, `<q>` a mineral (`Chemistry::minerals`, the inputs of growth that no
reaction burns; in chemistry 0, 0 is substance 12 and 1 is substance 11), `<K>` a starting kind of cover (its label,
`Cover::root`). A mean of nothing is `NaN` (`null` in the viewer's file, drawn as a gap).

## Files of a run (`ew run --out DIR`)

| File | What it holds |
|---|---|
| `snap_NNNNN.bin.gz` | the whole world and the shell's state (the plan of what is placed when, and `tape::Kept`: the substances seen so far), at the end of the run and every `--snapshots-every` years (0 by default: only at the end). Gzip of the tag `EWSN`, the format version (`u32`, `SNAPSHOT_VERSION`) and bincode. `ew resume --from` goes on from it, and the resumed tape equals the unbroken one |
| `snap_NNNNN_MM.bin.gz` | a snapshot `MM` months into year `NNNNN`, at each date of `--snapshots-at` (42.75: `snap_00042_09`). The page opens it; `ew resume` refuses it (it goes on from a year's start) |
| `series.csv` | one row a year, the columns below |
| `physics.json` | `{"physics": …, "config": …}`: the run's `Physics` and `Config` (without `chunks`), keys sorted; `ew numbers OLD NEW` lists the fields that differ |
| `world.ewf.gz` | the viewer's file (below) |
| `yearNNNN.png` | five panels side by side: map, temperature, rain, the cover (life) and the depth of the cells (a cell of one square light grey, coarser dark, finer warmer); every `--pictures-every` years, at the start and at the end; none by default |

Other commands write files of a snapshot, not of a run: `ew view` a PNG of one panel (`view.png`), `ew cells` a table of
the cells (`cells.csv`), `ew card` the test card (`worldgen::testkit::card`) as a snapshot with a plan that places nothing.

**Snapshots.** A snapshot is taken at a month's start and holds no cache: what the laws build again before they read it
(Flow's caches, `flow::Scratch`; each cell's place, `Life::env`; what stands on each cell, `contract::Stand`, worked out
again at load) is not saved. A snapshot of another format version is refused with both versions named.

**How the files are written.** Every 10 years (`TAPE_EVERY`), at every snapshot and at the run's end, the main thread
copies the tape and makes the snapshot's bytes; a writer thread (`cli/src/writer.rs`) compresses and writes them while
the world goes on, one job at a time. The run waits for the last job before the next and at its end. A write error stops
the run at the end of that year. The tape goes first and the snapshot last, so a tape is never behind a snapshot, and
`ew resume` refuses a tape whose months stop before the snapshot's. `world.ewf.gz` and `physics.json` are written whole
each time. `series.csv` only gets the rows added since its last write; it is written whole when its header line changed
(a new column) and at a resumed run's first write, which cuts it back to the snapshot's year. A snapshot and the
viewer's file are written to a `.part` file that takes the name when whole. A run that stops keeps the tape up to its
last write and resumes only from a snapshot.

## The viewer's file, `world.ewf.gz`

Gzip of `EWF2`, the header's length (`u32`, little-endian), the header (JSON), then the frames.

| Header key | What it holds |
|---|---|
| `w`, `h`, `cell_m` | the map in squares, and a square's side, m (on a globe its height, and its width at the equator) |
| `globe`, `lat_north`, `lat_south` | whether the rows wrap east to west, and the latitudes of the north and south edges, degrees |
| `seed`, `days_per_year` | the world's seed and its year in days; a resumed run checks them with `w` and `h` |
| `layers` | each map layer: `name`, `unit`, `scale` (`lin`, `log` or `raw`), `min`, `max` |
| `frames` | of each frame its `day` and `year` |
| `series`, `series_units` | the columns of `series.csv` and their units |
| `events` | `day`, `year`, `text`: a substance's first appearance (`substance k first appears`) and each edit (`edit: …`, `Edit::line`; `core/src/event.rs`) |
| `months` | a row a month, its `year` and `month`, which a resumed run checks its tape by |

**Frames:** one every `--frames-every` years (10), and one a month in the first `--monthly-years` years (2). A frame is
every layer in table order, one byte a square, row by row; a square shows the cell that holds its centre
(`tape::pixels`). A byte is 0 for none; `lin` maps `min..max` to 0..255, `log` maps `min..max` to 1..255 (under `min`
is 0), `raw` is 255 × the value.

On a globe row `y` of `h` lies at latitude `lat_north + (lat_south - lat_north) (y + 0.5) / h`, and column `x` of `w` at
`(x + 0.5) 360 / w` degrees east of the map's west edge.

## Map layers (`tape::LAYERS`)

Per cell. "Per m² of land" is over the cell's land (`contract::lands`), none where it holds no land.

| Layer | Unit, scale | Meaning |
|---|---|---|
| `z` | m, lin −800 to 4000 | surface height |
| `depth` | m, log 0.01 to 3000 | liquid depth: in the pores and standing |
| `ice` | m, log 0.001 to 100 | Flow's soft cover, snow and ice (`Scratch::soft`) |
| `flux` | mol/s, log 1 to 1e7 | liquid flux (`World::liquid_flux`) |
| `catch` | 1, log 1 to 1e5 | upstream area drained, in units of the cell's own (`World::catchment`) |
| `channel_w`, `channel_d` | m, log 0.1 to 1000; 0.01 to 30 | width and depth of the cell's main channel (`Physics::channel` of `flux`) |
| `loose` | m, log 0.01 to 100 | depth of loose ground |
| `wet` | 1, lin 0 to 1 | how wet the ground is (`Physics::wetness` of the liquid in its pores and standing) |
| `filled` | 1, lin 0 to 1 | share of the land's pores the key liquid fills, as life's month found it (`WetGround::filled`) |
| `coarse`, `fine` | 1, lin 0 to 1 | shares of coarse and of fine particles of the loose ground (`soil::Texture`) |
| `organic` | 1, log 3e-4 to 0.05 | share of organic matter of the loose ground's solid volume |
| `litter`, `slow` | kg/m², log 0.01 to 30; 0.1 to 10 | fresh dead matter, and the slow organic pool (`soil::holdings`) |
| `mineral_free`, `mineral_held` | kg/m², log 1e-5 to 0.1; 1e-5 to 0.3 | the nitrogen mineral dissolved, and held on the exchange sites |
| `rot` | 1/yr, log 0.01 to 10 | how fast dead frame rots there |
| `conduct` | m/s, log 1e-9 to 1e-4 | conductivity of the pores (`Pores::seep`) |
| `avail` | m, log 0.01 to 0.3 | liquid the loose ground holds for roots when full |
| `wet_share` | 1, lin 0 to 1 | share of the land whose pores the soil's liquid fills (`WetGround::share`) |
| `lambda_sat` | 1, lin 0 to 30 | wetness index ln(a / tan β) at and over which the land is saturated |
| `roots` | 1, lin 0 to 1 | share of the liquid for roots the drier rest of the land held, as the cover read it last month |
| `t_ground`, `t_air` | K, lin 220 to 330 | ground and air temperature |
| `rain`, `rises` | mol/m²/s, log 1e-6 to 1e-2; 1e-5 to 1e-2 | rain, and what rises less what condenses (moving averages over about a year) |
| `sheds` | mol/m²/s, lin −0.004 to 0.004 | rain less what rises: what goes to runoff and the ground below |
| `wind_u`, `wind_v` | m/s, lin −20 to 20 | low wind to the east and to the north at the air's last step |
| `vapor` | mol/m², log 1 to 1e5 | column of condensable gases |
| `absorber` | 1, lin 0 to 5 | heat-absorbing column: the gases' optical depth summed over the bands |
| `plants`, `wood` | kg/m² of land, log 0.03 to 300; 0.01 to 300 | the cover's mass, and its walls |
| `leaf` | m²/m² of land, log 0.01 to 30 | the cover's capture area |
| `seeds` | kg/m² of land, log 1e-5 to 0.1 | seeds lying on the ground |
| `grown` | kg/m²/yr of land, log 0.01 to 10 | dry matter the cover built in the last whole year (0 in the first year) |
| `takers` | kg/km² of land, log 1 to 1e4 | living mass of the takers (`fauna::per_km2`) |
| `cured`, `snag` | kg/m², log 0.01 to 3; 0.01 to 30 | cured leaves standing, and dead walls flames left standing |
| `flame`, `flame_len` | 1, lin 0 to 1; m, log 0.01 to 30 | last month: share of the land flames burnt, and their length |
| `burnt` | 1/yr, lin 0 to 1 | share of the land burnt in the last year's months (twice counts twice) |
| `burnt_age` | months, lin 0 to 120 | whole months since the last burn (120 if never) |
| `burnt_last`, `crown_kill` | 1, lin 0 to 1 | of the last burn: share of the land burnt, and share of the standing soft tissue it killed |
| `ash`, `flame_air` | kg/m² of land, log 1e-5 to 1; 1e-3 to 10 | last month: minerals the flames left on the ground, and gas they gave the air |
| `ground_r`, `ground_g`, `ground_b` | raw 0 to 1 | the ground's colour: its loose solids, else its bonded ones, under the litter of the cover's remains (over the share 1 − e^−(kg/m² / `LITTER_KG`)) |

## `series.csv`: the world

Means over the map are weighted by cell area (`measure::area_mean`); counts of cells count a cell once.

| Column | Unit | Meaning |
|---|---|---|
| `year` | yr | the year |
| `elem_<j>` | mol | element j in the world (must not change) |
| `min<q>_land`, `min<q>_sea` | kg/m² | mineral q in the ground, per m² of land and of sea |
| `min<q>_dead_land`, `min<q>_dead_sea`, `min<q>_cover`, `min<q>_deep` | 10⁹ mol | mineral q in dead matter on land and sea, in the cover (each at what its rot gives back), and in the bed's stores |
| `energy` | J | heat and chemical energy in the world |
| `t_ground`, `t_air` | K | mean ground and air temperature |
| `t_land`, `t_land_summer`, `t_land_winter` | K | the land's ground as life feels it (`World::t_ground_last`): mean of the year's months, warmest month, coldest month |
| `absorber` | 1 | mean heat-absorbing column |
| `air_<k>` | 1 | mole fraction of heat-absorbing gas k in the air |
| `ice_frac` | 1 | share of the area under 1 cm or more of soft cover |
| `liquid_land`, `ice_land` | km³ | liquid, and soft cover, on land cells |
| `rain`, `rain_land` | mol/m²/s | mean rain, and on land (moving averages over about a year) |
| `evap_land`, `runoff_sea` | mol/m²/s | on land: what rises less what condenses; what leaves land cells for the sea, over and under the surface (`World::off_land`), per m² of land |
| `lakes`, `lake_area`, `lake_m3` | 1, km², km³ | bodies of standing liquid above the open low ground that hold liquid, their area and volume (`World::pools`) |
| `low_level` | m | level of the open low ground's largest body |
| `edge_liquid`, `edge_liquid_heat` | km³, PJ | liquid that crossed into the open low ground at the map's open edges since the start, and its heat |
| `load_picked`, `load_sea`, `load_lakes`, `load_land`, `bed_worn`, `load_unlaid`, `load_short` | Mm³ | running liquid's load since the start (`Counters::load`): picked up; carried into the open low ground; into the other bodies; settled on land; bed worn into loose ground; at the samples' builds, what the mouths' lobes found no room for, and what their moves lacked |
| `rivers` | 1 | dry land cells (under at most `STANDING_M`, 1 m, of liquid) that are rivers: flux over `RIVER_FLOOR` (9e3 mol/s) and `catch` over `RIVER_CATCH` (10) |
| `river_flux` | mol/s | the liquid flux of those cells, summed |
| `cells` | 1 | cells of the map |
| `cover_slots`, `cover_kg` | 1, kg | slots of cover in the world, and its mass |
| `cover_died_<cause>` | kg/yr | cover that died in the year: `lack`, `phase` (frost), `spent` (upkeep unpaid), `taken` (by a taker), `broken` (pressed loose), `age`, `drowned`, `flame` |
| `cover_budget_<flow>` | J/yr | flows of the cover's budget, + into the cover (`cover::BUDGET`): `stored`, `basal`, `capacity`, `firmness`, `leaves`, `sapwood`, `root_breath`, `root_renew`, `building`, `shed`, `died`, `exuded`, `seeds`, `seeds_died`, `takers`, `placed`, `rounded`, `burnt` |
| `seeds_kg` | kg | seeds lying on the ground |
| `seeds_<flow>` | kg/yr | seed flows (`cover::SEED_FLOWS`): `landed`, `sprouted`, `worn`, `frozen`, `taken` |
| `burnt_land` | 1/yr | share of the land flames burnt in the last year's months (twice counts twice) |
| `flame_<flow>` | kg/yr | what flames burnt and killed (`flame::FLOWS`): `remains`, `smouldered`, `living`, `scorched`, `killed`, `killed_walls`, `cured`; and `plants` (1/yr), the plants killed |
| `flame_air_<k>` | kg/yr | of each gas at the surface, what flames gave the air less what they took |
| `cover_median`, `cover_max` | kg/m² | the cover's mass on a land cell: median and largest |
| `dead_<k>` | kg/m² | on the ground of the land: dead fuel, body matter, the solids life builds, and what particles hold |
| `ms_flow`, `ms_change`, `ms_cover`, `ms_tape` | ms/yr | wall time since the last row in Flow, Change, the cover and the tape (the shell measures it; NaN in the first row). `ms_tape` holds the recording, the snapshot's bytes, the tape's copy, any wait for the writer and the pictures; the writer's own work and the yearly check are not in it |

## `series.csv`: by starting kind

`root_<K>_*` for every kind up to the largest seen; a kind that died out reads 0 (its means `NaN`). Means are by built
cover.

| Column | Unit | Meaning |
|---|---|---|
| `slots`, `cover_kg` | 1, kg | its slots, its mass |
| `cover_area` | m²/m² | its capture area per m² of the world's land |
| `cover_<gene>` | the gene's (`cover::GENES`) | mean of each gene: `light`, `leaf`, `walls`, `store`, `frost`, `moist`, `seed`, `repair`, `fix`, `gas`, `split_curbs`, `burn_curbs` |

## What the live page is sent (not recorded)

The page (`wasm/src/`) opens a snapshot (`open_world`) or makes a new world, and saves the live world as a snapshot of
`ew run` (`save_world`). With each frame it gets the frame's layer bytes and the cells' table. These are not on the
tape; the module docs in `tape/src/ground/` and `tape/src/table.rs` say how each number is made.

**Ground tiles** (`tape::Ground`, `Ground::tile`). Made from the terrain's samples as last built; a build of the samples
(`flow::build`) remakes the tiles it changed (`Ground::rebuild`). A tile holds `TILE` × `TILE` texels (256) and
`GUTTER` (2) more on each edge, row by row, as 16-bit floats in two planes of four a texel. Depths run from the finest
where one tile holds the map to the first whose texels are no longer than 1 m (`tape::depths`).

| Plane | Numbers a texel |
|---|---|
| 1 | height as two halves (their sum holds it to a millimetre); the wetness index ln(a / tan β); distance to the nearest channel's line, m, signed by side (60,000: none) |
| 2 | that channel's sample, as a step from the texel's sample (0: none); one more than the pond whose liquid may stand there (0: none; at most `PONDS_EXACT`, 2,048); how far along the channel its nearest point lies, m, modulo `ALONG_WRAP_M` (128 m); distance to the channel's line without its bends, m |

**Samples' rock** (`Ground::rock`, `tape/src/ground/rock.rs`). Four planes of four 16-bit floats a terrain sample, row
by row; `Ground::rock_rows` sends the rows a build changed.

| Plane | Numbers a sample |
|---|---|
| 1 | depth of loose ground, m; largest stone its channel moves, m (`STONE_M` √A S, at most 2 m; 0 for a sink or under a pond); slope of its reach; its channel's links (step down, +8 where it joins as a tributary, +17 × step to its main neighbour upstream) |
| 2 | reflectance of its bed's make (red, green, blue); wavelength of its channel's bends, m |
| 3 | shares of coarse and fine particles; log10 of 1 m/s over its pores' conductivity; liquid for roots, on the `avail` layer's scale |
| 4 | slope up to the highest ground toward the light; 0; depth of the load laid on it, m; log10 of the years its loose ground has lain |

**Cells' table** (`tape::Table::frame`, f32 little-endian):

| Part | What it holds |
|---|---|
| head | melting point of the frozen cover, K |
| `ROW` (11) a square | of the cell at its centre: mean terrain height, m; ground temperature, K; how fast the air cools with height, K/m; ice, m; level of its standing liquid and its lowest ground, m; depth that stands, m; liquid in the pores and what they hold full, m; the wetness index level its soil's liquid departs about; share of its area that is land |
| kinds, `KIND_ROW` (42) each, a square | its slots standing on any of its ground, heaviest first, per m² of the ground each stands on (`tape/src/stands.rs` `rows`): 22 numbers of the slot (kind, built cover, its large plants, genes, activity, frozen share, rooting, slot, patch), 4 numbers for each of 4 groups of plants (plants a m², height, stem's width, crown's width), 4 of where it stands (line under the level, share under liquid, density on saturated strips and on the drier rest) |
| `PLACE_ROW` (6) a square | point (x, y, squares) and reach of each patch past the first |
| `DEAD_ROW` (9) a square | of each patch: dead walls standing, kg/m²; their height, m; years since they died |
| ponds | level of each pond, m (NaN where no body holds it or it holds less than `DRY_M`, 0.1 m, on its mean) |
| fetch | of each pond, the side of a square as large as its liquid's surface, m |
| last | the number of kinds a square holds (`stands::kinds`) |

**Discharge and load** (`discharge`): of each terrain sample, what runs through it, m³/s (`Table::discharge`), and what
it carries, g/m³ (`Table::load`), as two 16-bit floats; sent again when one has changed by `DISCHARGE_SHOWS` (0.1) or
`LOAD_SHOWS` (0.25). **Plumes** (`plumes`): where load came into a body last month, the sample (u32) and kg/s (f32).

**Slots in view** (`things`, `tape/src/things.rs`): close up, of each cell in a window, its patches (share, age) and
each slot's kind, built cover and capture area per m², genes and patch.
