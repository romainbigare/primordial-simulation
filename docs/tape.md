# What a run records (the tape)

The code that writes it is `tape/src/rec/` (the yearly columns in `tape/src/rec/cols/`). Names use numbers, never
Earth names: `<j>` is an element, `<k>` a substance, `<r>` a reaction, `<q>` a mineral (`Chemistry::minerals`: 0
the mineral of the fixing reaction's gas, 1 the second), `<K>` a starting kind of cover (its label, `Cover::root`).
A mean of nothing is written `NaN` (`null` in the viewer's file, drawn as a gap).

## Files of a run (`ew run --out DIR`)

| File | What it holds |
|---|---|
| `snap_NNNNN.bin.gz` | at the end of the run, and every `--snapshots-every` years if given: the whole world, the run's plan (what is placed when), and what the tape keeps across years (gzip of bincode, after the tag `EWSN` and the format version); `ew resume --from` goes on from it, and a resumed tape equals the unbroken one. A snapshot is taken at a month's start, so it holds the state and no cache: what the laws build again before they read it at a month's first step (Flow's caches, `flow::Scratch`; each cell's place, `Life::env`; what stands on each cell, `contract::Stand`, worked out again at load, as the tape reads it first) is not saved |
| `series.csv` | one row a year, the columns below |
| `physics.json` | the run's `Physics` and `Config`; `ew numbers OLD NEW` compares two runs |
| `world.ewf.gz` | the viewer's file: `EWF2`, the header's length, the header (JSON: size, whether the world is a globe and its edge latitudes, layers, frames, series and units, events, the months rows), then the frames: one every `--frames-every` years (10), and one a month in the first `--monthly-years` years (2) |
| `yearNNNN.png` | map, temperature, rain, the cover (life) and the depth of the cells (each square by the cell at its centre: a square light grey, coarser dark, finer warmer) every `--pictures-every` years, at the start and at the end; none by default |

**How the files are written.** Every 10 years (`TAPE_EVERY` in `cli/src/main.rs`), at every snapshot and at the run's
end the shell copies the tape and, at a snapshot, makes the snapshot's bytes, and a writer thread (`cli/src/writer.rs`) compresses and writes them while the world goes on: at most one job at a
time, and the run waits for it before the next job and at its end. A write error stops the run at the end of the year
it happens in. The job writes the tape first and the snapshot last, so a tape is never behind a snapshot, and
`ew resume` refuses a tape whose months stop before the snapshot's. `world.ewf.gz` is written again whole (gzip at its
fast level; the viewer reads any level) and `physics.json`. `series.csv` is not written again: a file
the writer wrote before gets only the rows added since, at its end. A file is written whole when its header line
changed (a column, or a starting kind, that is new) and at the first write of a resumed run, which cuts the file back to
the rows the snapshot's day holds. A snapshot and the viewer's file go to a `.part` file that takes the name when whole;
a run killed during a write leaves its `.part` file, which nothing reads. A stopped run keeps the tape up to its last
write, and resumes only from a snapshot.

**Raw events** (`core/src/event.rs`): the first time a substance appears; the viewer shows them.

**The months** (in the viewer's file): a row a month, its year and month, which a resumed run checks its tape by.

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
size), `channel_w` and `channel_d` (the width and the depth of the cell's channel, `World::channel`), loose ground,
`wet` (how wet the cell's ground is, 0..1: `Physics::wetness` of the liquid in its pores and standing over them), ground and air temperature, rain, vapour, absorber, the cover's mass, its walls (`wood`) and its capture area (`leaf`) per m², the seeds lying on the ground (`seeds`, kg/m²), what the cover built in the last whole year (`grown`, kg of dry matter per m² a year, `Counters::grown_last`; 0 in the first year), these five per m² of the cell's land (`layers::per_land` of the cell's amount per m² of its area and its land's share, `contract::lands`: the cover and its seeds live and are drawn on the land alone, divided by the land once; 0 where the cell holds no land), the living mass of the takers per km² of the cell's land (`takers`, kg/km², `fauna::per_km2`; 0 where the cell holds no land; log from 1 to 10,000), true colour (r, g, b: the colour of what covers the surface, its frozen cover, liquid or
ground) and the ground's colour (r, g, b: its loose solids, else its bonded ones, without frozen cover or liquid, under
the litter of the cover's remains in it over the share 1 - e^-(their kg/m² / `LITTER_KG`), 0.3 kg/m²). The tape holds whole-map frames only. The live viewer draws its map from two
things, neither recorded. **The ground's tiles** (`tape::Ground`, `tape/src/ground/`), which never change, so a page
makes each once and keeps it: a tile of a depth holds 256 x 256 texels (`TILE`), each a leaf of that depth, and 2 more
along each edge (`GUTTER`), row by row from its north-west corner, as 16-bit floats (IEEE binary16, little-endian) in two
planes: first four a texel (the height as the sum of two, the nearest 16-bit float and what it lacks, so it holds to a
millimetre: a shore is the difference of two heights; the wetness index, to 0.01; the distance to the line of the
texel's channel, m, signed by its side), then four (that channel's sample, the pond, and of the nearest point of that
channel's line how far along the channel it lies and how the line curves there), 16 bytes a texel. The depths
go from the finest at which one tile holds the whole map to the first whose texels are no longer than 1 m
(`tape::depths`). Per texel: the terrain's mean height over it (`Terrain::heights`, so a texel is the mean of its four
children), and on texels finer than the samples, out of the ponds, drawn toward the floor of a channel within 250 m
(`VALLEY_M`) that drains 1 km² or more (`VALLEY_FROM`): all of it on the channel's line, less and less across to none at
250 m (the floor's share f eased from 1 to 0, its weight f / (1 - f) against the ground's 1; each straight piece of
a line gives a texel its floor at the texel's nearest point of it with that weight, as much as it is long over a
quarter of 250 m, and the pieces near a texel are summed, so the floor is smooth across the ground and has no step
where a line bends), the floor running along each sample's piece from the mean of its height and the one's upstream through its own
to the mean of its own and the next one's (the next one's own where a tributary joins it; a closed pond's level). So the
lowest line of the ground drawn is the channel's line, and the samples' steps beside a valley fade out; the mean of a
sample's texels is not kept there exactly; near a pond (within 2 samples) a valley only lowers the ground and not under
the pond's level, so a channel's mouth meets its level. Then the shore is drawn smooth (`tape/src/ground/shore.rs`): of
each sample, its height over the level of the nearest pond (the sea's 0 m), held within 4 m either side, blurred over
the samples by a Gaussian of 0.9 samples; the zero line of that offset (a cubic B-spline through the samples' centres)
is the shore, and the ground drawn keeps its own height where it lies on the offset's side of the level and is moved to
that side by half the offset where not (none from 4 m on); a closed pond's samples deeper than 3 m keep their liquid;
ground out of a pond and under its level (where its liquid spills below its pass) is drawn as it is.
Beside a pond the offset lies a little lower along a channel's line (a quarter of its bed's depth at the shore, none
from 5 of the bed's half widths across and from 4 bed widths from the shore): the liquid drowns a channel's mouth, so
the shore and its shallows turn a little up the channel.
Which of these a texel takes (in a pond: no valley; beside one: the valley held at its level and the shore drawn; else
the valley alone) is blended between the samples' centres, by the share of the texel's weight on the samples of each
kind, so the ground has no step along a sample's edge.
Under the level the bed leans to the offset at the shore and farther out to the samples' depth held within 40 m and
blurred alike, all of it in the shallows. So a coast is a smooth line with capes and bays of a few samples and the
shallows one band, none of a sample's scallops; the shore moves by about a quarter of a sample on the mean (a blur
shrinks what bulges: an island and its capes). Then the wetness index of the place, ln(a / tan β) (Beven and Kirkby 1979) of the terrain's samples (a the area
that drains through a sample, `Samples::up`, over its width, β the slope of the samples' ground, `Grid::slope`),
blended between the centres of the four samples about a texel finer than them and the mean of those a coarser one
covers; and its channel (`tape/src/ground/channels.rs`). A channel runs through each sample's point
(`tape/src/ground/stems.rs`): the mean of the centres of the samples along its stem, 6 up and 6 down, weighted 6 to 1
by nearness and the sample's own 7 (a moving average of the polyline of centres, as corner cutting smooths a polygon:
Chaikin 1974), as many each way as the stem has (a stem runs on through a body's samples, along its floor), so a
head, a stem's end and the sample a tributary joins keep their place (a tributary ends on the joined stem's point), and no more than a sample from the centre:
so a channel that steps like a stair down a valley across the grid draws as one smooth line from afar. It is drawn as each
sample's way (`Samples::to`) as a curve from the middle of the way into it (from its neighbour upstream that the most
drains through, `Samples::main`) to the middle of its own, between the points, bent at its own point (a quadratic
Bézier), and on to the middle of the next sample's line where a tributary joins; a sample that liquid runs into and none out of (a sink of the open low ground, a pond's
lowest) ends the curve at its point, which lies where the shore drawn puts the liquid (`shore::Shore::mouth`: from the
sample's centre down the smooth shore's offset to where it lies 0.5 m under the level, no farther than 4.5 samples),
so a river reaches its lake or its sea. The line wanders about that curve (`tape/src/ground/wander.rs`) and swings about its wander in the bends of a
meander belt (`bends.rs`, `belt.rs`). The samples place a valley's line no closer than a share of a sample, so the
line wanders up to 0.4 of a sample to each side (half of that on a hard bed), in long swings of no fixed length: a
smooth noise along the way down the valley, a corner of its lattice every 1.6 samples and four finer octaves, each
2.7 times as often as the last and 0.33 as strong, so each leans the line less steeply than the last and its turns
are round and of uneven size (a channel takes those whose corners lie 8 of its bed's widths apart
or more, fully from 16: a wide bed's finer turns are its bends', and a rill turns every few metres), the sum taken
1.75 times and held smoothly within the most, whose slope runs on across the corners (gradient noise, Perlin 2002), none at
a head, a join and a sink. Every channel wanders, a rill and a body's floor too, and a tributary's way on from its own piece to the line it
joins (by 0.15 of that way's length, none at its ends): none runs straight. A bed is shaped by its floods, so the bends are sized by the bed's width; a tile is made once, so that width
and the bed's depth are stand-ins: the channel law's at the drained area times `BED_RUNOFF`, 5 times (`FLOOD`: the
bankfull flood, Leopold and Maddock 1953; Dunne and Leopold 1978) `TYPICAL_RUNOFF` (8.5e-9 m/s, Earth's mean: 0.27 m a
year, Dai and Trenberth 2002). The bends swing 3.5 bed widths to each side of the wander where nothing holds them
(belts are 4 to 20 widths across, Williams 1986): all of it where the reach's slope (over 2 samples up and 2 down the
channel) is under 0.01; over 0.03 the bed is hard and 0.35 of it is left (pool-riffle reaches meander, step-pool ones
do not: Montgomery and Buffington 1997; Schumm 1985), and between the two the change is gradual. They swing no wider
than the valley's floor (4 bed depths over the rise of the ground across the valley, the samples square to the
channel's way over the same reach) and than the channel's reach less its bed; the whole sway no wider than 0.4 of the
radius the valley's own line turns on and than 0.8 of a sample; no sway at a sink, and no bends (the wander alone)
for a bed under 1 m and under standing liquid. The bends are walked down each main way from its head, from one apex to the next (a half bend, 24 steps):
along a half bend the curvature runs smoothly from the first apex's to the next one's turned the other way (a cubic of
the heading, 0 at both apexes: a curvature that falls evenly along the line where both are alike, as measured bends
do: Langbein and Leopold 1966), so the line has no kink and its curvature no jump. Each apex takes its place and its
radius from the place's draws (a function of the seed, the sample the bends began at and the half bends since): it
reaches 0.2 to 1 of the belt's half width (the smaller the more often), times a swell that falls to 0.08 over 4 half
bends (most reaches bend fully, some run nearly straight), on the other side from the last; it turns on a radius of
2 to 5 bed widths, the tighter the more often (2 to 3 in most grown bends: Hickin and Nanson 1975), gentler where
the swell is low. A half bend is as long as carries the line from its apex to the next (found by halving, 3 to 24
widths, its least up to 3 times longer by its own draw), no longer than keeps its heading within 1.85 rad of the
valley's line and carries it 2.5 widths down the valley: past a right angle a bend turns back on itself in a loop. So
no two bends are alike in length or swing. On a hard bed an apex turns on 1.5 to 12 widths (most turns gentle),
the heading keeps within 0.8 rad, and a half bend's least length is up to 12 times longer by its draw: the
line runs in nearly straight stretches from one short turn to the next. A piece's walk starts as the last one's
ended, so the line has no break from sample to sample, and along it runs the line's own length from its head; a
tributary's and a sink's last piece draw back to their wander at their end. A tile draws a sway (the bends, the
wander) where its length (a bend's 11 bed widths, Leopold and Wolman 1960; a wander's 3.2 samples) is 72 texels or
more, and none under 24 (between it grows, its turns the wider the less it is drawn): a shell draws a channel from afar
as a line several texels wide, and a sway whose turns are tighter than that line is wide reads as a saw's teeth. Each
channel reaches as far as it could be wide, w0 × (`MOST_RUNOFF` a)^b / 2 (`MOST_RUNOFF` 3.6e-6 m/s, about the wettest
month known, 9.3 m of rain at Cherrapunji in July 1861; a the drained area, w0 and b the width law of
`Physics::channel`), and one a shell may draw as a line wider than itself (its drained area at least `SHOW_M` times
the texel's size over 40) as far as half the widest line the shell draws it as on these texels (the line law, `LINE_PX`
1 css pixel times its discharge over the one that shows to the power 0.5, up to 2.5 times, where a css pixel takes 3
texels and the discharge is 10 times `TYPICAL_RUNOFF`'s, and 1.5 texels more). Of the channels within reach, a texel keeps the one whose distance over the
larger of its reach and the texel's size is least (the one that covers it first as the flux grows, or, where all are
narrower than a texel, the nearest), but a tributary takes no texel in the bed of the channel it joins (within half
that bed's width of its line, near the join). It keeps that channel's distance, signed by
the side (60,000 where none reaches), and its sample, as the step from the sample that holds the texel's centre
(1 + (dx + 21) + 43 (dy + 21); 0 for none), whole numbers a 16-bit float holds exactly; and of the nearest point of
that channel's line, how far along the channel from its head it lies, m, modulo 128 m (`ALONG_WRAP_M`: a 16-bit float
holds it to a few centimetres, and a shell lays along it only signs that come again after that length). A texel
within 2 texels, to its left and to its right or above and below, of texels that both hold one other channel, which
drains more (with none but its own channel's between), is given to that channel (`mend.rs`: a small channel that
crosses the reach of a larger one lies nearer over its scale in a strip a texel or two wide, which would notch the line
a shell draws the larger one as). Each texel also keeps the distance,
signed by the side, to its own channel's line without its bends (its curve and its wander alone; the line from the same
head; 60,000 where none reaches), and a texel that holds no line takes its sample from that one: the difference of the two
distances is the bend's swing, by which it draws the pool at the outer bank of a bend and the bar inside it. A shell finds the sample
again from the texel's centre, floor((x + 1/2) k) for a texel k samples a side, stepped by it. Off the map a texel is
its edge's. Each number is a function of the seed, the map and the place alone: the same in every tile that holds it. The second plane holds, beside the channel's sample, one more than the pond
whose liquid may stand on each texel (`tape/src/ground/shore.rs`: the pond of the sample that holds its centre or the
nearest within 2 samples each way, else of the heaviest sample about it that has one, where the smooth shore's offset
lies under `EDGE_M`, 2 m; the ground drawn lies over the level up to there, by as much as the offset where a sample
about it has no pond near, so the edge of where liquid may stand is dry ground: a smooth line of the place, the same in
every tile at every depth, never the texels' or the samples' edges; a shell draws the liquid where the ground drawn
lies under the pond's level; 0 for none), whole numbers a 16-bit float holds exactly up to `PONDS_EXACT` (2,048). **The cells' table** (`tape::Table`, `tape/src/table.rs`), made with each
frame: the melting point of the frozen cover, K, then for each square row by row 11 f32 (`ROW`) of the cell that holds
its centre: the terrain's mean height over the cell, m; its ground's temperature, K, and how fast its air cools with
height, K/m (`World::air_drop`); its ice, m; the level its standing liquid reaches and its lowest ground, m
(`flow::levels`), and the depth that stands, m; the liquid in its pores and what they hold full, m (`flow::means`), and
the level of the wetness index its soil's liquid departs about (`inside::soil_level` over the samples inside the cell;
0 where the pores are dry or full); and the share of its area that is land, its land over its area (`contract::lands`: as the
month's start found it, the land its cover lives on). Before Flow built its state the level and
lowest ground are its solid surface, the depth that stands is its liquid's, and a land cell is all land and any other
none. Then for each square row by row its
cell's kinds of cover (`tape/src/stands.rs`), the world's `Config::variety` (the header's `tiles.kinds`, 8 unless the
start chose other) of `KIND_ROW` (36) f32 each, the heaviest first: the slots of
its cover, each as the law holds it, per m² of the cell's land (`layers::per_land`; a cell that holds no land has no
kinds); of each its starting kind (-1: none); its built cover, kg/m²; its large plants (`stands::plants` of the slot's group of the largest plants, `cover::Sizes::large`, with that group's built cover and share of the capture area): the stem's width, m, plants a m² (that group's count), height and crown's width, m, the share of the
ground their crowns cover and how closed a crown is (1 - e^-their leaf area); its walls' share of its built cover; its
capture area per kg of living cover, log10 m²/kg (`LEAF`); its light capacity per m² of capture area, log10 mol/m²/s;
its moisture gene and frost gene, K; its fuel over its reserve; its activity on its cell's ground (`cover::activity`);
the share of it frozen this month (`cover::frozen`); the share of what it builds that goes to seeds (`SEED`); and the
share of the land whose loose ground holds its plants (`Cover::rooted` over the land's share, at most all of it) with
the loose ground their roots need, m (`Cover::need`; all of the land and 0 while the need of a slot of newly landed
seeds is not found); and its walls gene (`WALLS`, log10 kg of walls per kg of matter), by which the page draws a woody
kind as woody plants while they are young (web/src/style.ts `designOf`). Then of each of its groups of plants
(`cover::GROUPS`, 4; laws.md 3), the large plants first and the empty groups last (all 0), four numbers: plants a m²,
and a plant's height, stem's width and crown's width, m (`stands::plants` of that group). The page thins a
kind at each point by the cover's own rule (`cover::stand::rooted`) with these and `rooted_from`, which the live
world's header gives with the laws' numbers (wasm `meta`). A kind stands on the share of the cell that holds its roots
alone (laws.md 3): there its crowns cover the share of the ground whose light its leaves stop, 1 - e^-(leaf area /
that share), the cover's own light law, in patches where they close, each crown the ground it holds there; a plant's stem and height are the cover's own (`cover::stem`, `cover::height`, laws.md 3). Then the level of each pond, m (`World::pond_levels`, NaN where no body holds it, or where its body holds
less than `DRY_M` (0.1 m) of liquid on its mean over its surface: a basin soaked, drawn as ground), at which the page draws the pond's liquid flat over the ground that lies under it, the pond each texel names. **The samples' rock** (`Ground::rock`,
`tape/src/ground/rock.rs`), made once with the ground: of each of the terrain's samples, row by row, four 16-bit floats:
the depth of its loose ground, m (`Samples::loose`), the largest stone its channel moves, m: `STONE_M` (0.001) ×
√(the area it drains, m²) × the slope of its reach, 2 samples up and down it (the stream power the wear reads, K √A S; Bagnold 1966),
at most 2 m; 0 for a sink or a sample under a pond; a zero; and its channel's links as one whole number: the step to
the sample it runs to (1 to 8, its neighbours row by row; 8 more where it joins that one as a tributary; 0 for none)
and 17 times the step to its main neighbour upstream, by which a shell lets what runs through a channel grow evenly
from one sample's centre to the next, so its width has no step where two samples meet. Then of each sample again the
reflectance of its bed's make and a zero. **The
samples' discharge** (`Table::discharge`): what runs through each of the terrain's samples, m³/s, row by row (its
`World::discharge` at the mol per m³ of the liquid its cell's ground holds; none where that holds no liquid; a
sample of a pond that holds no liquid, or that the shore drawn leaves out of the liquid (`ground::shore_dry`), carries
no less than its main way upstream brings, so a river's line runs on along a dry body's floor and to the shore a shell
draws; under a body's liquid no channel runs). The
live page gets it as 16-bit floats with a frame when one sample's has changed by a tenth since it was last sent
(`wasm/src/lib.rs` `DISCHARGE_SHOWS`: its width by 5%), 2 bytes a sample (248 kB for a 22 island, 2 MB for a 64 one;
with most frames, as a month's flow changes that much somewhere), and sizes each channel by the channel law from
its own sample's discharge. The page's graphics card blends the table between the
squares' centres and applies the laws inside a cell at each pixel of the tiles' ground (laws.md 0, "Inside a cell").
It blends the cover's field over the squares whose cell holds land alone, their weights made whole
(`cover_field.wgsl` `on_land`): the cover is drawn on land only, and a square with no land is not bare ground beside it. A tape's map is drawn
from its frames alone: the page makes fields of the squares (`web/src/fields.ts`, a texel a square: the height and its
slope, the liquid's surface over the ground, below a dry square's ground by the depth it lacks to be wet and its height
over the wet squares beside it, the ice, the ground's temperature, the sea, the share a channel covers, `channel_w`
over the square's width, how wet the ground is, the share of the ground the cover leaves bare, the true colour and the
ground's colour), coloured by the same rules; with no kinds in a tape, its cover is one foliage colour as closed as its
capture area closes the ground.
Close up the live viewer also asks for the slots of cover of the cells in view (`tape::things`, the record layout in
`tape/src/things.rs`: each slot's kind, built cover and capture area per m² and genes); they are drawn as plants, and
are not recorded.

## `series.csv`: the world

| Column | Unit | Meaning |
|---|---|---|
| `year` | yr | the year |
| `elem_<j>` | mol | element j in the world (must not change) |
| `energy` | J | heat and chemical energy in the world |
| `min<q>_land`, `min<q>_sea` | kg/m² | mineral q in the ground of land and of the sea floor |
| `min<q>_dead_land`, `min<q>_dead_sea`, `min<q>_cover`, `min<q>_deep` | 10⁹ mol | mineral q in dead matter on land and sea, in the cover, in the bed's stores |
| `t_ground`, `t_air` | K | mean ground and air temperature (by area; of a cell of parts its land's and its liquid's top's mean by area, `World::t_surface_last`) |
| `t_land`, `t_land_summer`, `t_land_winter` | K | the land's ground as life feels it (of a cell of parts its land, `World::t_ground_last`): the mean of the year's months, the warmest month and the coldest |
| `absorber` | | mean absorption of the heat-absorbing gases |
| `air_<k>` | | mole fraction of heat-absorbing gas k in the air |
| `ice_frac` | | share of the area under ice |
| `liquid_land` | km³ | liquid on land |
| `ice_land` | km³ | frozen cover on land |
| `rain`, `rain_land` | mol/m²/s | mean rain, and on land (a moving average over about a year) |
| `evap_land`, `runoff_sea` | mol/m²/s | on land, what rises into the air less what condenses on the ground, and what runs down the terrain's samples from land cells into the open low ground (`World::off_land`), per m² of land (moving averages over about a year): with `rain_land` the land's water budget |
| `lakes`, `lake_area`, `lake_m3`, `low_level` | , km², km³, m | the bodies of standing liquid above the open low ground that hold liquid over their cells' pores (`World::pools`, laws.md 1.2), the area their liquid covers and its volume; and the level of the open low ground's largest body |
| `edge_liquid`, `edge_liquid_heat` | km³, PJ | the liquid that crossed into the open low ground at the map's open edges since the start (out counts less), and its heat (`Counters::edge_liquid`, laws.md 1.6); 0 where the edges are closed |
| `rivers` | | dry land cells (under at most `tape::STANDING_M` of liquid) that are rivers (`tape::is_river`): a liquid flux over `tape::RIVER_FLOOR` and over `tape::RIVER_CATCH` times what the cell's own area sheds (`catch`). The report counts the same way |
| `river_flux` | mol/s | the liquid flux of those river cells, summed (`tape::river_flux`): the water the count stands for |
| `cells` | | the cells of the map |
| `cover_slots`, `cover_kg` | , kg | slots of cover in the world (`Config::variety` a cell at most), and its mass |
| `cover_died_<cause>` | kg/yr | cover that died in the year by cause: `spent` (upkeep unpaid), `age`, `phase` (frost), `taken` (by a taker, `contract::take`), `broken` (pressed loose, `contract::press`); `lack` is never written to |
| `cover_budget_<flow>` | J/yr | the flows of the cover's budget in the year, + into the cover (`cover::BUDGET`): what its parts and the seeds lying on the ground hold as fuel changes by their sum. `stored` (light reaction); the upkeep's parts `basal`, `capacity`, `firmness`, `leaves`, `sapwood`, `root_breath`, `root_renew`; `building` (its work and fixing); `shed`, `died`, `exuded` (fuel its roots give the ground), `seeds` (out and landed), `seeds_died` (seeds lying that wore out or froze), `takers` (taken or pressed loose through the contract), `placed`, `rounded` |
| `seeds_kg` | kg | the seeds lying on the ground of the world (`cover::Lying`) |
| `seeds_<flow>` | kg/yr | the seeds' flows in the year (`cover::SEED_FLOWS`): `landed` (on a ground), `sprouted` (took hold in the cover), `worn`, `frozen`, `taken` (by a taker, `contract::take` of `Layer::Seeds`) |
| `cover_median`, `cover_max` | kg/m² | the cover's mass on a land cell: median and largest |
| `dead_<k>` | kg/m² | dead fuel, matter and built solids on the ground of the land |

**Time per process** (measured by the shell, never by the core; NaN in the first row and in a run with no clock)

| Column | Unit | Meaning |
|---|---|---|
| `ms_flow`, `ms_change`, `ms_cover` | ms/yr | wall time in Flow, in Change and in the cover (what stands on each cell and its month) since the last row |
| `ms_tape` | ms/yr | wall time of the main thread in the tape itself since the last row: the months recorded, the snapshot's bytes and the copy of the tape that go to the writer thread, and any wait for it (the writer's own time runs beside the world and is not counted). The row's own month, and the snapshot and copy made after the row (at a write of the tape), land in the next row. The pictures of a year (every `--pictures-every` years) are in it; the yearly check is not timed |

## `series.csv`: by starting kind

`root_<K>_*` (`K` the label of a kind of cover, `Cover::root`), for every kind up to the largest seen; a kind that died
out reads 0 (its means `NaN`). Means are by its built cover.

| Column | Unit | Meaning |
|---|---|---|
| `slots`, `cover_kg`, `cover_area` | , kg, m²/m² | its slots, its mass, its capture area per m² of land |
| `cover_light`, `cover_leaf`, `cover_walls`, `cover_store`, `cover_moist`, `cover_repair`, `cover_fix` | log10 | the means of its genes (`cover::LIGHT` ...) |
| `cover_frost`, `cover_seed` | K, | the means of its frost and seed genes |
