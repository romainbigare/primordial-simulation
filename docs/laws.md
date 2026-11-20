# The laws as they are now

What each law does, its rule, its physical reason and where it lives. How the code does it (rounds, caches, buffers,
orderings) is said once, in the code's doc comments; the code is the truth. Numbers, units and anchors are in
`docs/numbers.md`, what a run records in `docs/tape.md`.

**Where the code is** (`core/src/`): time in `clock/` and `schedule.rs`; the shape of the world in `space/`; how a
cell's means lie over its own ground in `inside/`; shared
maths in `solve/` (`sweep`, `line`); nodes and books in `store/`, `world.rs`, `check.rs`; ground heat in
`ground.rs`; settings in `params.rs`, constants in `units.rs`; the cover in `cover/`, the life of each cell and the
chunks it runs on in `life/`; what lies between the natural world and its takers in `contract.rs`; keyed random
numbers in `rng.rs`; the tape's events in `event.rs`; threads in `par.rs` (the same results with and without, on any
count of threads).

**Words.** A **cell** is a square of the map (2 km on the island, latitude-longitude on a globe) or a block of 2 x 2
or 4 x 4 of them. A **node** is anything that holds stuff: a cell's ground, an air column, a cell's cover. A **gene** is
a number a kind of cover inherits, often log10 (1 means ×10). The **key liquid** is the liquid life's chemistry runs
in (substance 0). **Light-eaters** (plant-like) are the **cover** of each cell, a field (3).

## 0. The world, one step, the books
**The map** (`space/`, `World::mesh`): cells, links with widths and headings, and a coarser air level. The laws step
only on links. Each link takes one of 8 compass directions (`Level::way`).
An island of 2 km squares whose rows are spread evenly from `Config::lat_south` to `lat_north` whatever its size
(`--band S,N`), or a globe (`--globe`) of latitude-longitude squares on a sphere of 6,371 km whose rows wrap
east to west; an air block is 4 x 4 squares. The cells are the leaves of a sparse tree under each air block
(`space/tree.rs`): the block is the root and only the leaves are kept (`Square::leaves`), numbered by their north-west
corners row by row, counted in leaves of the most depth (`DEPTH_MOST`) as 64-bit numbers (a map is under 2^53 of them a
side, so a place is exact as a 64-bit float); a square is depth 2. A world's cells are made once and stay for its whole
life: squares on land and cells of 1, 2 or 4 squares on a side in the open low ground (worldgen; on a globe the squares
of a window, `--fine`, and an air block a cell elsewhere). Cells across a border differ by one depth at most (2:1). One
builder makes every cell (`Square::mesh`). Cells link across a border by its width, as far apart as their centres
across it, so a steady change across a border of two sizes passes exactly; across a corner (width 0) where four cells
meet, whatever their sizes, the two across it one depth apart at most and neither with 8 links already (the cells in
order), so no cell has more than 8 links. The air level is built on the map's squares. The map's samples are its
squares (`Square::samples`: a cell coarser than a square cut into its squares): each reads its air there (1.4). A
cell's height is the terrain's mean over it (`Terrain::inside`, below), the height its ground inside it stands about.
**The terrain**
(`space::terrain`) is one function of the world's seed and a point of the map, at any size: value noise summed over
octaves, with ridges, its domain warped (Perlin 1985; Musgrave 1989), raised to heights, an island's falling away from
its rises, says how high the ground would stand. On an island a unit of the noise is 21.3 km on every map
(`ANCHOR_M`: 6 units across the island of 64 squares, which the key numbers are measured on), so its coarsest ridges
and valleys lie 20 to 30 km apart, as the main valleys across a range do (Hovius 1996), and a hill is a hill's size on
a map of any size: a larger map holds more of them, not larger ones; a globe's noise has 6 units round its equator.
**An island's crust is raised by rises** (`space::rises`, `Lift`):
the noise's large shape falls by (the distance from the nearest rise, in its reaches)^p, so land stands round each rise
and low ground that the liquid fills lies between them. An island of **drawn rises** (`Shape::Drawn`, the default of
`ew run` and of the page) has one to several, all drawn from its seed: their count the most its map holds or one
fewer, the most the count of lands of at least 600 km² that the number-area rule gives for all its land in one,
√(land / 600 km²) (Korčák 1938; Mandelbrot 1975: N(>A) ~ A^-B with B about 1/2), so it grows with the map's side (1
or 2 on 32 squares, 2 to 4 on 64, 5 to 7 on 128, by the drawn share); the area each raises as k^(-1/B) for the k-th
largest, B drawn from 0.5 (a main land and outliers) to 0.9 (lands nearer in size), the exponents of the archipelagos
of the one planet known; each a dome or, one in 33, a ring round a sunken block (a caldera's rim), up to four times as
long as wide at any heading (a raised block), its flanks falling with p drawn from 1.8 (a broad dome) to 3 (a block
with a flat top and steep flanks), tilted toward a drawn side (a tilted block) and lifted by up to ±0.1. A dome is
tilted by up to 0.8 units of the noise; 85 rings in 100 by 0.5 to 0.8, more than the noise's large shape mostly rises
over the level (its mean, 0.5), so their rim lies under 0 m on their low side and their inner basin opens to the
open low ground (a breached rim, as most marine calderas are); the rest by less, closed. Where two rises raise the
land nearly alike, the land is lower by 0.5 units of the noise, falling off by e every 0.2 that one rise lowers it
more than the other (`DIVIDE`): the crust sags between two raised blocks, so a strait runs between their lands. The
largest are placed first, each where its land (the reach where its fall is 0.75, the large shape's rise over the
level in nine places of ten) stays 2 squares from the map's edge, shortened and then shrunk where it does not fit, and
as far from the others as 24 draws find, their middles at least as far apart as their mean extents (where the fall
is 0.5) summed. Their reaches are scaled together so the share of the map's squares whose unworn noise stands above
the level is the share drawn from the seed, 30 to 45%: the least of 14 scales from 0.2 to 3 times the one rise's
reach that holds it, refined by 6 halvings (rises that grow into each other sag between them, so more land is not
always the larger scale's). The wear and the drowning take some: 22 to 43% of the map is land on drawn islands of 64
squares, seeds 1 to 12, 33% on average, against 24 to 37% (31%) on islands of one rise. An island of **one rise**
(`Shape::One`, `--shape one`; the tests' worlds) has one in the middle of its map, reaching half its side, p = 1.6.
**Each rise has its range** (`Range`): along an axis drawn from the seed (any heading, up to a
unit of the noise from the middle of the map, bent by the warp; on a drawn rise along its length, fading past its
ends) lies a range, a band half 0.7 to 1.4 units wide, its offset and width for a rise reaching 64 km (the one rise of
the map of 64 squares) and in proportion to its reach for any other (drawn: a range is as wide as its raised block is) where the land's rise over its level is 1.3 times the mean on
the axis, falling by a bell across it to 0.2 times the mean far from every range (`RISES`): a high rough range, its
foothills, and a wide low plain where the land rises slowly. That scales the noise's large shape; its base's octaves
from the third on (`FINE_FROM`, a quarter of the coarsest's wavelength and finer) keep their own size where the land
rises slower than the mean and grow with it where faster, so a plain keeps its own swells, low hills and shallow
hollows of tens of metres while its large heights stay low. A globe's land rises alike everywhere (its noise is of the scale of its continents, its cells
wider than a range); the world keeps what it needs beside its seed and its map
(`World::terrain`: the rise its level 0 m lies at, and an island's shape). The
noise's finest octave is 670 m on every island and 200 km on a globe of 64 rows. The ground is held at **samples**, the
leaves of `SAMPLE_DEPTH` (1/64 of a block: 125 m on the island, 20 km on a globe of 64 rows), and **the samples are
worn** (`space::samples`, `space::wear`, found once from the seed when first read, not saved): the ground has a
history. The noise is read at the centres of the samples of a grid of 2 x 2 samples to one (`NOISE_BY`,
`wear::noise_grid`) and carried smooth to finer grids. **The sea stands low while the land is worn**: its liquid
stands 100 m under 0 m (`LOW_STAND_M`; the last low stand of the one planet known was about 120 m under today's,
Fairbanks 1989). The open low ground then (on a globe every sample under the low stand; on an island those under it
joined to its lowest, so ground under it inland is a closed basin) stays as the noise has it; land rises from the low
stand at a steady rate to the noise's height over 2 million years (`WEAR_YEARS`; ground under the low stand does not
rise) while flowing liquid cuts it by the stream power
law (Howard 1994; Whipple and Tucker 1999): it lowers by K √A S, A the area that drains through a sample, spread down
every lower way by Quinn's weights (Quinn et al. 1991), S the slope to the sample its liquid runs to, m = 1/2 and n = 1,
K = 3e-7 a year (`CUT`) on the bed, each step solved exactly from the open low ground up the ways (h = (h₀ + f h_down)
/ (1 + f), f = K dt √A / d; Braun and Willett 2013), so a step of any length is stable, the low stand the height of
the low ground's liquid. At the end the liquid rises to 0 m: the open low ground is what lies under 0 m joined to the
open low ground of the low stand (on an island, to its lowest sample), so the valleys cut down toward the low stand
by the shore are drowned: inlets, bays and small islands off the coast. An island's level (`ISLAND_LEVEL`, 0.27) keeps
its land's share of the map about what it was with no low stand; a globe's is found on the noise as before, and the
drowning takes about half a percent of its area from its land (28.4% to 27.9% on a globe of 32 rows). **The bed is of
a few makes** (`space::strata`), each a mix of the chemistry's own bed substances (`makes`, section 8; five in
chemistry 0), and flowing liquid wears each as much faster than the first as Flow's wear breaks it (the sum of each
substance's share over its bond strength, 1.3; `Terrain::cut`: 1, 0.88, 1.11, 1.18 and 1.36 in chemistry 0). They lie
in **layers**: a stack of 12 layers, each 60 to 400 m thick and of a make drawn from the seed (never the make of the
layer over it), repeating under them (`LAYERS`, `THICK_M`), folded up and down by up to 2 km by fractal noise of
three scales from about 8 km (`FOLD_M`, `FOLD_PER_SQUARE`); and through them stand **bodies** of the hardest make
where fractal noise over four scales from about 4 km down passes 0.66 (`BODY_ABOVE`), as rock cooled from a melt
rises through strata. The top of the stack rises with the land and floats back up with it, so how deep the wear has
cut under it, plus the fold, says which layer a sample's surface is in; at each step the wear cuts each sample at the
rate of the make it has reached. So a soft layer opens into a wide vale, a hard one stands as a scarp or caps a
tableland, and folded layers clip out in bands round the heights. A cell's bed is the mean of its samples' makes by
area, substance by substance (`Samples::make`). Under a closed
basin's level the liquid does not cut. What a step cuts spreads down as the liquid does; a closed basin keeps all that
comes into it and fills its floor from the lowest up, as one level, toward the height its pass has now, and what it
cannot hold runs on over the pass (`space::ponds`); what reaches the open low ground lies there at the end (the shelf,
below). The slopes creep as a diffusion of 3e-3 m² a year (`CREEP`, the anchor of the core's own creep; Culling 1960),
and what creeps into the open low ground goes to the shelf too; the ground floats back up by 2700/3300 of what a step
took away, spread over 50 km as a plate bends (Airy isostasy, `FLOATS`, `BENDS_M`; Molnar and England 1990; on a grid
coarse enough that 50 km spans 16 of its samples, `BEND_SAMPLES`, blended back between their centres). The land
first gets a roughness of a metre drawn from the seed (`ROUGH_M`), which the first ways find. The time runs on three
grids (`STAGES`): 40% of it in 16 steps on a grid of 4 x 4 samples to one, then 30% in 8 steps on one of 2 x 2, then
30% in 4 steps on the samples' own, so the valleys are cut at the samples' size; each grid takes the noise's mean over
each of its samples (or the noise carried to it), and each is carried to the next by the smooth lean below (which
makes no new pit and no new peak); a map whose first grid would hold more than 65,536 samples runs every stage on grids
twice as coarse until it fits (`MOST_SAMPLES`). **The shelf** (`space::shelf`): the load that reached the open low
ground is spread along the shore by the waves, each sample of the shore taking the load that reached the shore within
5 km of it as a share of that stretch (`ALONG_M`); from it the load builds out a platform 10 m under the low stand
(`TOPSET_M`, a delta's topset), as wide as that load in proportion (a round lobe: a sample the open low ground joins to
a shore's sample lies inside that one's platform by its width less the straight distance between them), fronted by a
slope of 0.3 down to the floor (`FORESET`, the load at rest under liquid; Gilbert 1885). So a channel that brings much
load builds a fan at its mouth and a coast that gets little keeps its steep drop; its edge is rounded over 250 m as a
clinoform's top rolls over (`ROUNDED_M`: the mean of the thickness over a square that wide, the land in it as none),
never over the platform's top; the width is found so the shelf holds all the load (where the platform fills the
open low ground of a small map, the rest leaves the map), and what the wear took off the land is what lies in the
basins, on the shelf and off the map (`Samples::books`). Laid at the low stand's shore, the shelf lies 110 m under 0 m
once the liquid has risen: a drowned shelf, its front where the low stand's coast was. **The high stand**
(`space::stand`): then the liquid stands at 0 m for 6,000 years (`HIGH_STAND_YEARS`; the sea's rise ended about then on
the one planet known, Fairbanks 1989; Stanley and Warne 1994), and the channels fill the liquid they run into. A flood
from the open low ground at 0 m finds each mouth and the ground that drains to it with no closed basin on its way (a
basin keeps what comes into it, so its catchment brings none); each mouth brings the load its catchment yields, by the
law fitted to the load of the large channels of the one planet known (Syvitski and Milliman 2007, read for catchments
of 1 to 100 km²): Qs = 6e-4 Mt a year × Q^0.31 A^0.5 R T, Q its discharge in km³ a year (0.3 m a year of runoff,
`RUNOFF_M`), A its area in km², R the height of its highest ground in km and T the typical surface temperature
(`T_SURF`) in degrees over 273.15 K, of a bed of 2,700 kg/m³. Its catchment gives that ground where the wear's last
stage lowered it fastest, each sample no lower than the one its liquid runs to (or the liquid's level) or the top of
its mouth's lobe; what that holds back the mouth does not bring. **The lobes** (`space::delta`), largest load first,
each on the ground the ones before it left: a mouth fills the liquid from where its channel comes in, each sample up
to the **top** a deposit builds to there, the surface of the channel's flood (as deep as a channel of the terrain's law,
`Terrain::channel`, carries `FLOOD_RUNOFF` from the area that drains to that sample, the mouths that share it together;
Leopold and Maddock 1953), the samples in the order of the least cost to reach them (Dijkstra), until its load is laid,
the last in part. A step costs its length, more by the depth of the liquid under the top over 10 m (`DEPTH_COST_M`: the
lobe follows a drowned valley and the shallows, in a lagoon too), more by 3 (`ALONG_COAST`) times how
open the sample lies to the waves times the square of how far the step leads away from the coast (the waves spread a
lobe along the shore; Wright and Coleman 1973; Galloway 1975), and less by 0.5 (`DOWN_CHANNEL`) times how sheltered it
lies times how far the step leads on along the channel's last steps (the channel carries it out). How open a sample
lies is the mean, over the ways out of it that lead over open liquid, of the run of open liquid along each, at most
50 km, over 50 km (`FETCH_REF_M`; an effective fetch, Håkanson 1981); the way out from the coast is the rise of the
distance from the land. The waves shape the lobe, not its height: on a coast they build beaches up to the level and
over it. Round each lobe a front falls from its top at 0.3 (`FORESET`), from the samples it filled last (never over
its top). The hollows the lobes leave (the liquid a lobe cut off from the rest of its body of liquid, not another body;
the valleys behind them that drain to a mouth and lie under the highest top) are filled to where liquid would spill out
of them (a flood from the open liquid, Barnes, Lehman and Mulla 2014), each by the lobe of most load the flood's way
to it crossed (one it does not reach, by the lobe of the lowest ground round it), from that lobe's front and then the samples it filled
last, at most all the lobe holds: where it holds less, its hollows are filled by that share. A lobe's edge is its
front: it is not rounded. So each lobe holds its mouth's load: what the land gave is what lies in the lobes, what a lobe found no room for (`Books::unlaid`),
and what left the map; what the catchments held back is apart (`Books::held`). The top of what a lobe laid at a sample
lies there for the share of its load laid after it, of the 6,000 years (its front, none; a filled hollow, half); a
sample's loose ground is as old as the mean of its layers by
depth (`Samples::age`). **Known limit:** the volumes of the wear are heights times areas: the
load's pores are not counted,
so a deposit stands about 1.7 times thinner than one of the same rock would. **The loose ground** the wear leaves (`Samples::loose`):
on land (`space::mantle`) the depth where the bed's breaking into loose ground balances what takes it away, the
smaller of two balances. The bed breaks at 7.7e-5 m a year under none, e times slower under each 0.4 m (`BREAKS`,
`BREAKS_UNDER_M`; Heimsath, Dietrich, Nishiizumi and Finkel 1997). Against the lowering of the last step E (cut and
creep), what settled on a sample in the last stage (in the basins, at the foot of the slopes) and over it 0.4 ln(7.7e-5
/ E) m, none where E outruns the breaking, at most what a lowering of a millionth of the breaking keeps (`SLOWEST`).
Against the creep of the loose ground down its slope S: all the loose ground made on the area a that drains through a
sample per m of its width, with what settled there as fast as it came over the stage, passes it and creeps at 3e-3 m²
a year a m of its depth (`CREEP` at 1 m, `CREEP_DEPTH_M`) times S / (1 − (S / 0.8)²), a flux that grows without bound as
the slope nears the slope loose ground rests at (`REST`; Roering, Kirchner and Dietrich 1999), so a depth H holds where
H 3e-3 S / (1 − (S / 0.8)²) = a 7.7e-5 exp(−H / 0.4) + what settled; none at 0.8 or steeper. So a steep slope's bed is
bare and the gentle ground at its foot holds the loose ground it sheds; on the open low ground the shelf over a metre of the fine load that
settles everywhere (`OOZE_M`). It runs in a fixed serial order in 64-bit numbers, so a seed gives one ground on every
machine.
**The flood** (`space::drain`) over the worn samples, from all the open low ground, does not raise the ground (Barnes,
Lehman and Mulla 2014): it reaches the samples across their sides by the level their liquid stands to, lowest first (a
sample's own height where it drains; under a closed basin's level the height of its lowest pass), then the lowest on
the flood's surface, then the least index. Each sample's liquid runs to one of the neighbours reached before it that
the surface falls to, by a fixed draw of the sample, with a chance in proportion to the fall (Fairfield and Leymarie
1991), so on an even slope the ways run on the mean along its fall and not along the grid's 8 directions; across a
corner only from ground that drains and where both samples beside the corner lie lower too, so no way passes over the
ridge between them. Under a basin's level, and on a level plain, the surface is that level raised along the way the
flood came by 1e-6 per m (`FLOOD_RISE`) to twice that by the draw of the sample reached, so inside a basin the ways run
to its pass (Planchon and Darboux 2001; Cordonnier, Bovy and Braun 2019) and across a plain not in straight lines
along the grid. A closed basin whose deepest sample lies less than a
metre under its level (`GRADED_M`) is filled to it, as the floods that spread a plain's load leave it. The samples of
one basin under its level are **a pond** (`Samples::pond`, `Samples::ponds`): its pass is the first sample out of it
on the way its liquid leaves by, and its level that pass's height. Every cell's height is a mean over the samples. So
is the way a cell faces the sun (1.4): its mean rise toward the south, each sample's central difference north to south
(one-sided at the map's edge) by area (`Terrain::rise_south`), found once when the world is built and kept
(`World::rise_south`). **The terrain's channels** (`space::network`, found with the samples): each sample's liquid runs where the
flood sends it (from every sample of land a way leads to the open low ground), and the area that drains through a
sample is its own and all that runs to it; the runoff runs down these ways (1.2). Below the samples the ground goes on as one function of the
seed and the map that every cell and every pixel a shell draws read (`space::below`, `Terrain::heights`): the means of
the quarters of a leaf of side ℓ depart from its mean by a smooth continuation of the coarser heights that keeps it:
the quarters lean as the quartic through the leaf and two neighbours each way on each axis does (average
interpolation, Donoho 1993), so the ground has no steps and its slope no kinks at the leaves' edges and a slope across
the samples goes on across their quarters. (The wear carries its grids so with the whole lean scaled down where a
quarter would pass the lowest or the highest of the leaf and its eight neighbours, Barth and Jespersen 1989, so it makes
no new pit and no new peak.) Nothing random is laid on it. So a leaf has a mean height at any depth. What needs the cell at a
point (the tape's frames, a byte a square; the pictures) asks the tree for the leaf that holds it (`Tree::cell_at`).

**One step** (`schedule::step`, a month; time in whole seconds, the calendar from it) is a fixed list of passes
(`schedule::month`), each for the whole month. First the month's start (`schedule::START`, alone `month_start`): what
stands on each cell (`contract::stands`), and the surroundings (`contract::surroundings`: Flow's state rebuilt in full
and each cell's place from it, which Flow's own pass then leaves). Then Flow: the air in steps of `AIR_STEP_DAYS` (15
days, two a month, each at its own time in the month), after each the sink of the liquid over field capacity into the
bed's store; then the liquid and erosion; heat sideways and creep yearly. Then the month of life (`life::month`: the cover's,
then the takers') and Change.
**Chunks and the parallel pass** (`life::run`). The cells are cut into `Config::chunks` runs of neighbouring indices,
the same for any number of threads. Each chunk is one task on the pool, its cells' life taken out of the world, the
world read only; in each cell in order the cover's month and then the takers' run on the cell's **view of the store** (`store::Moves`): its
ground, bed and aside node as the store held them at the pass's start with its own moves since, and **its share of its
air node** (`Mesh::share`, its area over the air node's) of the air's amounts and heat: a cell never takes more than
that, so the cells under one air node never overdraw it together. When the chunks end the world takes their counters
and books their moves (each array of the store by one task, the changes cell by cell in the order of the cells,
`store::book_by_array`), so every sum runs in the order of the cells and the world is the same for any count of threads
and of chunks. A serial step on one cell runs the same way on a chunk of its own (`World::in_cell`).

**The contract** (`contract`): what lies between the natural world (the ground, the air, the cover) and what takes
from it: its takers are the takers' field (`fauna`, 3b) and the plain taker of the tests (`worldgen::testkit::Taker`). The natural world names no
module of a taker (`cli/tests/sides.rs`). It is plain data and functions over the one source, with no copy. **What a
cell shows** a taker: its cover as a whole, each slot by the height its plants stand at, and the seeds lying on its
ground (`contract::shown`, `contract::slots`, `Layer::Seeds`): the mass and the amount of each substance a taker can reach, and the energy to break it (`life::breaking`: each part's volume times its breaking stress, body matter as living tissue at `rho_body`); its place as the month's start left it
(`contract::place`: the ground's temperature, the depth of the liquid standing on it, its height, and the depth of the
soft solid lying on its land, `Scratch::soft_land`, at the solid's density). **Snow hides** what lies under it
(`contract::above`): a plant's matter is spread evenly over its height (the stand's law draws it as a stem of its
height), so of a slot whose plants stand h high under snow d deep a taker reaches the share 1 − d / h (none at d ≥ h),
and of the seeds lying, on the ground, nothing where any snow lies. **Take**
(`contract::take`): a taker bites a mass of the whole cover, one slot or the seeds lying; a bite takes all the parts
it reaches in their own proportions, as a mouth takes a piece of a plant, never more than it can reach, with its heat; the cover books its side (`Cover::take`), the taker its own. **Give**
(`contract::give`): matter and heat back to the ground, through the cell's view of the store. A slot of cover shows a taker
two layers, its soft part and its frame (`Layer::Soft`, `Layer::Frame`; the frame with its main stem's width), and
`contract::cut` gives the share of a frame a jaw of a force can cut (3b). **The key liquid**
(`contract::liquid`): what a cell's ground holds of it, split four ways: open (standing on it, as deep as the month's
start found it), in its pores (the cover's roots reach it, a mouth does not), frozen and lying on it as the soft solid
(as deep as the month's start found it), and frozen in its pores; and beside them what ran over it in the month (its main channel's month,
`World::liquid_month`), which a mouth reaches though it is no amount of the cell's. **The way to it**
(`contract::ways_to_liquid`, at the start of life's month, after Flow's): for each decade of liquid (`LEVELS`), the shortest way
along the links between cells' centres over land from each cell to the nearest cell whose open liquid over the month
(standing and running) holds that much (Dijkstra 1959; ties by the cell's index); `contract::to_liquid` gives it for a need
rounded up to its decade. **Vapour** (`contract::vapour`):
a taker sheds heat by turning liquid and soft solid of its ground into vapour (what it drank of the running liquid
leaves the cell's liquid, which the running liquid joins), given to the cell's aside node as the cover's books give a gas; it
pays the vapour's latent heat less what the ground held (a solid's heat of fusion too). **Press**
(`contract::press`): work spent on a cell breaks its cover by the share it pays for (`life::breaking` of all its slots), and what breaks
goes to the ground as remains with its heat. **What stands on a cell** (`contract::Stand`): the cover's mass, capture
area and the light it absorbs and keeps, at each month's start (`Cover::stand`); Flow reads it (1.3, 1.4), its capture
area spread over the cell, or over the land part of a cell of parts (none over its liquid's top). **The land of a
cell** (`contract::land`): its area times the share of it out of the liquid that stands over its pores
(`flow::land_shares`), at each month's start; what the land offers life. A cell shows
itself as the month's start left its place and what stands, and its cover as it is; takes land in the takers' month,
and the cover feels them in its own next month.

**Inside a cell** (`inside/`, handover 5.2): a cell holds means (its cover, the liquid in its soil, what stands on
it). The ground inside it is the terrain's worn samples inside it, and what stands lies on them (`flow::curve`, 1.2).
The shell draws inside the cells by general laws at each pixel of the ground's tiles (`tape::Ground`), blending its
cells' values between the squares' centres (`tape::Table`, docs/tape.md): the soil's liquid by the wetness index
ln(a / tan β) of the terrain's samples (Beven and Kirkby 1979; a the area that drains through a sample per metre of
contour, β its slope, no less than a valley floor's 1e-3; found once with the samples, `space::samples::sample_index`):
as in TOPMODEL, the cell's liquid table departs from its mean at each point by `soil_m` per unit of the index over its
level (Beven and Kirkby 1979; Beven 1997). The table is where Flow keeps the liquid: with the loose ground over field
capacity, in it (its room what its pores lack); else under it in the bed's store (its room the loose ground's pores over
field capacity and what the store lacks of `bed_porosity` × `bed_depth`) (`Means::deficit`). The room at a point is the
cell's less `soil_m` (λ − c), within none and the most, the level c found by halving so the mean room is the cell's: it
moves no liquid, only shares it out (`inside::soil_level`, `deficit_at`). A sample where liquid flows in its channel
(its yearly discharge above none, and it drains a channel's source, the square of the channels' spacing, 2 `hill` a
side; Horton 1945) is the table at the surface over its channel's width (the channel law on its discharge,
`World::channel`): that share of its area counts at an infinite index, its room none, the rest at its own
(`inside::flowing`, `channel_share`, `parts_now`). A point whose room is none is saturated, its
pores full; elsewhere its soil holds the cell's liquid departed the same way (`soil_held`). The laws read it over a
cell's land samples (the highest, by its land's share of its area; `inside::wet_ground`): the share saturated, at an
index of at least c plus the mean room over `soil_m`, and the drier rest, which holds what that share does not
(`inside::rest`). The month's start finds it for each cell (`contract::surroundings`, `World::life`'s `wet`), and every
law reads that one share: flames (3c), and how much of the land is waterlogged, its pores holding no gas: the saturated
share and, of the drier rest, the share its liquid fills from the bottom up over field capacity (`inside::airless`,
`Pores::full`), which roots (3) and the decomposers (2.3) read. Where the rest holds more than field capacity that is
the share the cell's mean fill gives; where it holds less it adds the wet strips of a drier cell. What stands where the ground lies under the cells' level; each channel as wide as
the channel law makes its own sample's discharge (`World::discharge`, 1.2); each mark of cover by
the activity of each kind of the squares about it at the mark's foot (`cover::activity`), with the cell's pores half full and
the liquid departing about its samples' mean index, so where a plant stands is the place's, not the month's. A picture need not keep a cell's mean
exactly (the core keeps the books).
**Sleep and wake** (`clock::Sleep`, a speed tool): a calm node sleeps until an alarm or until an amount grows by more
than `wake_share`, then its law brings it up to date over the whole time since.
**One ground temperature**: every law and the tape read that of Flow's last step: the cover its
land's (`World::t_ground_last`, of a cell of parts its land, on which no liquid stands, or its liquid's top where it has
no land, 1.1), Change and the
potential energy the node's (its heat over its heat capacity, `FlowView`), the
tape the surface's mean by area (`World::t_surface_last`).
**Checks before a run**: `Config::validate`, `Chemistry::validate` (every reaction keeps its elements and mass).

**The books.** Every move takes from one node exactly what it gives another, with its heat; `store::apply` and the
bookings of the chunks' moves and of Change's rows (`store::book_by_array`, which adds as `apply` adds) are the store's only writers, with plain sums.
Boundary nodes stand for the sun, space and outside (`store::Boundary`, edits by `World::cross`). **Potential
energy** is one function of what the nodes hold (`flow::potential`): a ground cell is a stack from its base up, its bed
and its solids each at its middle, its liquid with what is dissolved in it as Flow holds it (1.2, `stack::Column`): in
the pores of its loose solids first (of a cell of parts, less the room its land's pores have left, 1.1), a table rising from the bed's top, and the rest standing over the cell's own
ground (its level-volume curve) at its centroid; its store `bed_depth` under its bed's top, filled from the bottom; an
air node's matter lies at the mean head of its cells (their solid surface and liquid as a column, as Flow's state
stacks them), for the height of a column above its ground is in its heat (its heat capacity at constant pressure holds
the work the column does as it rises); an aside node's gas and the matter of a cell's life (its cover with the seeds it
sends until they land) at its cell's head. The parts of a month's Flow after its start
(state, the air's steps, liquid, creep, refresh) follow each other with nothing between (`flow::chain`) and are
booked as one at their end; the build of Flow's state at the month's start (`contract::surroundings`), and a part run
alone, are booked at their own end. In each booking every move carries its
potential energy from the height it leaves (a ground node's liquid top: the table of its pores, or the level of what
stands where that lies higher, so liquid standing in a pit under its full pores leaves with them and the top never
rises as liquid leaves, `stack::Column::top`; an aside node's head; an air node's mean head; a store's table; each the mean of the booking's start and
end, so a lake that drains whole takes along what it held); what only passes through a ground node from ground node
to ground node in one booking (as much as both came in from them and left to them: a body's liquid that its pass
gathers in one cell and shares out, a pit's that spills and runs back) carries on the energy of what came in, mixed,
found by sweeps over the moves (`potential::relayed`), so its fall lands where it stays, never as a lift or a fall of
a cell it only crosses; and each node's heat takes what came in less what left
less the change of its own (`store::fall_heat`): so a fall heats where the matter lands (the runoff's in the body it
reaches, 1.2), a lift is paid by the node it enters (the air, for a gas going up; the ground node a store's liquid rises
into), a column that rises as it fills pays for it, and the potential energy the world's
matter gained is one number of the store (`Store::lifted`), the change of that function. Within a month's Flow the later
air steps and the liquid read the heat before the month's lifts and falls are paid. The other laws lift a gas into
the air from its cell's head to its air node's mean ground by the same rule (`World::lift`), between the heads the
potential energy found at Flow's last booking (`Scratch::lift_at`: the matter's heights after Flow moved it, by the
stack the potential energy reads, not Flow's own surface, whose slow ground is found again only at its refresh); a ground node that reacts
(Change) pays what its stack and the air over it gain (`flow::gain`: a head that rises lifts its air node's matter by
the cell's share of the node's area), and its gas leaves from its head (`change::booking`). **Life and its ground**:
life's matter counts with its ground node, as Flow books it. What life gives its ground or takes from it (the cover's
losses and what it takes up, what a taker gives) gains the potential energy that the function finds between the cell as it was and as it is, the matter
from the head into the stack or back with what that raises or lowers above it and the air over the cell
(`flow::gain`), and the ground node's heat pays it (`store::life_moved`, for the moves of a view of the store and those
`store::apply` books from the cover): a fall heats the ground it lands in, a lift is paid by the ground it leaves.
Seeds that go to another cell's head carry their potential energy from the head they left, and the ground they land on
takes the difference (`store::life_sent`). An edit
(`World::cross`) takes the potential energy of what it places from its boundary node. So every element and the energy
are kept; `check::drift` checks it yearly against `check::DRIFT` (1e-9), with the potential energy as the function of
what the nodes hold (`check::Totals::potential`), not as `Store::lifted` books it, so a
move that books no lift or fall shows.
**No overdraw.** A law never asks a node for more than it holds; where it could, it cuts that node's own outgoing
links (`solve::sweep::within`), never a whole field. A cell's life takes no more than its cell's view holds (checked in
debug builds). `apply` and `book_by_array` only add and set rounding below zero to zero, counted (`Counters::rounded`).
Nothing takes ground heat further than `T_FLOOR` (`World::take_from_ground`).

## 1. Flow: stuff moves from high push to low push
Files: `core/src/flow/` (ground, liquid, erosion), `core/src/flow/air/` (air, radiation, rain, wind).

### 1.1 The ground of a cell
Layers from the bottom: the bed (bonded rock), loose ground, soft cover, standing liquid (`flow::land::surface`); under
them a deep store (`Class::Deep`, `bed_depth`, `bed_porosity`) that bodies and Change do not reach. The soft cover is
a solid near its melting point (`soft`), as snow and ice. It holds no liquid: its pores close as it settles, as snow
becomes ice (Cuffey and Paterson 2010). It insulates the node under it (its depth over its conductivity), and its
surface warms no further than its melting point: past it the top melts and the melt soaks in, so the node takes all
the heat that reaches the surface (`air::radiation`). A cell is **land**
when its solid surface is above 0 m (`World::is_land`). The top `skin_m` trades heat with the surface. Heat spreads
sideways yearly by one explicit `solve::sweep`, a link passing the harmonic mean of conductivity × thickness, times
width over length (Patankar 1980; `flow::land::conduct`), between land cells (`flow::on_land`): rock spreads a hot spot one cell in about 70,000 years, so a
year is far below the explicit limit on cells of kilometres (1.7).
**A cell's parts** (`flow::parts`): the ground of a cell with liquid standing over its pores is a short list of parts
under one air, each with its own heat and matter: its **land**, where its own ground lies above what stands (liquid
and soft solid, `Fill::wet` of it), the liquid's **top**, down to `h_mix` under the liquid's level, and its **deep**
below that (`Fill::deep`), which holds the ground under the liquid. A dry cell is one node; a cell
partly under a liquid no deeper than `h_mix` has land and top; a cell all under a liquid deeper than that, top and
deep; a coast cell over a deep liquid all three. The land and the top face the air, each with its own heat, phases,
soft cover and reflectance: a mosaic of tiles (Avissar and Pielke 1989; Koster and Suarez 1992); the top is the
liquid's mixed layer, the deep the liquid under its thermocline (a slab ocean with a deep, Hansen et al. 1984). The
store holds the cell's one ground node; Flow keeps how its matter and heat lie between the parts (`parts::Parts`,
saved): the share of the area under the liquid, and of each part its heat and its mol of each substance. A part holds
its share of the bed's skin (the land its area's, the liquid's bottom the rest) and finds its own melting state,
layers and surface from what it holds (`parts::solve`, at every build of the state), as a node does: so ice forms on
the top at its melting point, as soft cover. What other laws move (Change, life, heat sideways, the potential energy's
falls) is shared between the parts: of each substance by the share of it each holds, of the heat by their heat
capacities, so a heat all share warms them alike (`moves::reconcile`). What Flow moves of one part it books there: the
air's step the land's and the top's own exchange (1.4), the liquid laws what they bring to the top and what they take
by each part's share of the liquid (1.2). The land keeps the liquid its pores hold, its own soil's; what lies over its
pores runs down to the top with its heat (`moves::settle`). The share under the liquid follows what stands on it, the pores of the land it comes to cover filling first from what stands, so it covers what is left after they fill (`parts::covers`): a
land the liquid covers gives the liquid's bottom all it holds there, a ground the liquid leaves gives the land its
bed's skin, its solids and the liquid its pores hold, so what stands is the same either way (`moves::strip`). The deep
holds the liquid that stands deeper than `h_mix` and the top the rest, the liquid that moves between them taking its
heat at the temperature of the part it leaves; a top over a deep it has not had gives it the ground under the liquid
and its pores' liquid, a deep that holds no liquid gives the top all it holds (`deep::layer`). **The top and the deep
swap liquid** once a month, before the basin levels and mixes its tops (`deep::mix`, 1.2): equal volumes of their liquid, each with its heat,
through the thermocline by eddy diffusion (`thermocline`), over the area of the ground under the deep's level and the
distance between the two parts' middles, as two volumes that even out exactly as they would alone; and all the way
where the top is the denser, so the column overturns: the top's main liquid lies nearer the temperature at which it is
densest with what the top holds dissolved (`Chemistry::dense_in`, 2.5; a liquid grows lighter both ways from there) than the deep's. So a top that cools toward
that temperature sinks and mixes the column, and one that cools past it lies on the deep and freezes. The land and the top pass each other heat at the shore by conduction through their active layers across the
edge of the ground under the liquid (`Fill::shore`, found at the samples), each from its middle, half its area over
that edge away, in series (two half cells, as heat conducts sideways between cells): over cells of kilometres this is
far below what the air gives them, so they meet through the air they share; in a cell of ten metres they even out in
months. The node's state, which every other law reads, is the parts': its heat capacity theirs, its temperature its
heat over that apart from their fusion enthalpy, each substance molten by the share of it they hold molten (the one
with the most fusion enthalpy as its plateau), its pores those under the liquid and what the land's hold, so what
stands over them is what stands over the liquid's. A cell whose land no longer stands apart (nothing stands, or all
the cell is under its liquid no deeper than `h_mix`) is one node again; a cell where liquid comes to stand over part of
its ground, or over all of it deeper than `h_mix`, is made parts at its temperature and phases, the liquid holding all
that stands over the pores and its area's share of the rest (`parts::create`); its next solve makes its deep. No part holds less heat than its matter at `T_FLOOR`; the first other that can gives what it lacks.
**Known limit:** the parts hold one mixture of the substances that move with the liquid, shared by what other laws
move; a part's own salt (a sea's against its land's rain) is kept apart only as the liquid moves it, and the
top overturns by temperature alone: a saltier top does not sink for its salt. The top's depth is one number; a real
mixed layer deepens in winter and thins in summer.
**Melting plateaus** (`ground::melting`): a node holds the fusion enthalpy of what is molten in it, so it stays at a
melting point (lowered by solutes, 2.4) until that substance has all frozen or melted. A node whose heat melts or
freezes all of it within a step stays there only for the share of the step that takes, then goes on from that point,
and a soft cover that melts away holds its surface at the melting point only until it is gone (`air::radiation`): the
share of the step it stays is the share its fusion enthalpy is of the heat the step brings at the melting point, and
the rest of the step is the same balance from the melting point. **Known limit:** each surface solve is backward
Euler, so a step of length h against a node's time constant τ (its heat capacity over what its balance changes per K)
answers 1/(1 + h/τ) of its distance from balance where the exact answer is e^(-h/τ). A 3-day step on the dry land
of the small world has h/τ of 0.5 to 0.9 (cell 55: 0.665, 0.540 for the rest after its plateau): it ends 0.03 to 1.7 K
from 48 short steps in the sun and 1.2 to 5.4 K in the dark (where the air is also taken at its start), and it
answers to the length of the rest after a plateau too weakly (cell 55: the plateau delays the end by 0.41 K in one
step, 0.58 K by the exact relaxation, 0.71 K in short steps). The state of a node is found in full at each month's start; between, it is solved again only on a
plateau or when its temperature leaves the span between the melting points its last solve found, and its surface
(soft cover, reflectance, pores) is found again once it moved by 1% of the depth that hides half of a soft cover with
the liquid standing on it (`land::ground_state`). As matter moves, its heat capacity and fusion enthalpy follow
(`Scratch::note`). The state is brought up to date after each of the air's steps, whose heat melts and freezes, before
the sink into the bed's store and the liquid read it; and a node on a plateau that liquid left or reached finds its
molten share again from its heat (`liquid::book`): what moved of the plateau's substance was liquid, and what stays is
what the heat holds molten, as every law and the shell then read it.

### 1.2 Liquid
`flow::liquid`, once a month, after the air's steps; last, gases dissolve in the liquid (1.5). The sink into the bed's store ends each of the air's steps
(`flow::air`).
- **Pores first**: the loose ground's pores (each cell's porosity, `World::pores`, 4) fill first, under any soft cover; only liquid above them stands
  and has a head (`flow::land::stand`), the depth bodies and Change read.
- **Each cell's own ground** (`flow::curve`): the ground inside a cell is the terrain's samples inside it, their
  departures from their mean laid on the cell's mean surface, found once. The samples of the cell that lie in a pond
  (0) are its **parts** of the bodies of standing liquid, one part for each pond; what stands over its pores lies on
  them, each part at its body's level, by what each held at the last runoff's pass (`State::held`, saved), so a cell
  holds land that drains and parts of one body or more (`curve::Fill`). A cell with no pond's samples holds what stands
  only until the runoff's pass, from its lowest sample up. A cell is wet where part of its ground lies under the
  liquid; its lowest ground is its lowest sample (`flow::levels`, `flow::lowest` read them). The liquid's potential
  energy is where it lies: in the pores, and over each part at its centroid (0, "The books").
- **Bodies of standing liquid** (`flow::bodies`, `flow::basin`): a body is a pond of the terrain's samples (0: the
  samples of a closed basin under its pass, or a part of the open low ground joined through its samples, which has no
  pass), and every pond is one. Its volume by level is its own samples': Σ area × (level - floor) over them, each
  sample's floor its height laid on its cell's mean surface. It is held by every cell that holds samples of it, each
  by its part, and its liquid stays in the ground nodes of those cells. The bodies, their parts and the ways to them
  are found once for the world's cells; at each month's start, each body's level from what its parts held at the last
  pass (at the world's start, its pass: section 8), and the cells under it (their mean surface under it: the others are its shore). The lowest body, the open low
  ground, has no code of its own.
- **Running off** (`flow::runoff`), once a month in one pass: each cell sends what may run off (what came up from the
  store, the pores' drainage to the channels, below, which what stands over them refills first, and what stands over
  its pores over what its parts held at the last pass; its parts keep the rest, each by what it held) down the
  samples' ways from each of its samples by its share of the cell's area. A way reaches the body of the first pond's sample on it, in the cell that holds that
  sample, and what reaches a part is that part's (`bodies::Bodies::ways`); liquid on a cell's ground outside any pond
  does not stand. The liquid goes straight from the cell it leaves to the cell its way reaches: it does not wet the
  cells it passes, and the heat of its fall lands where it lands. The bodies then come in turn, each before the body
  its pass leads to: what its parts hold stands at one level over its own samples (`level::level_with`), no
  higher than its pass, the pores of the ground it newly covers filling first (`parts::pores_first`: in a cell of
  parts what its land's pores lack, from what stands; in a node that is one, its pore space, from the liquid all its
  pores hold first): a body over deep dry loose ground drains into it until its pores are full. What its samples hold
  under a level and what those pores take both grow with the level, linearly between two heights of samples: the
  level is exact, on the line through the samples under a level found by halving toward it; where it stands at a
  height of samples whose pores take more than is left, those pores take what is left; each cell whose part holds more than its samples hold under that level gives the share of its
  liquid over it, with that share of its heat; mixed, it goes to the cells whose part holds less, by what each lacks
  (`basin::share_out`), and what the body holds over its pass leaves, mixed, to the part its pass's way reaches, in
  the same month. Each part then holds what its samples hold under its body's level; a cell that was one node becomes
  a cell of parts (1.1) whose liquid holds what its part holds over its pores, the pores under it full, its land
  keeping the rest of its pores' liquid (`parts::create`). What a cell sends from its
  samples in no pond and what a body lets over its pass go into the samples' ways at each sample and at the pass, and
  one pass over the samples from the last the flood reached to the first, each before the one its liquid runs to, sums
  what runs through each (a pond's sample keeps what reaches it): the **discharge** of each sample (`World::discharge`,
  mol/s, a moving average over about a year). A body that receives less than it loses (evaporation, 1.5) falls under
  its pass and keeps what was dissolved in it.
  **Known limit:** the sea's slope under wind and rotation is not modelled, and what levels it crosses the sea at once.
  **Each body mixes its heat in a month** (`basin::mix`) under each air block: then its cells off their melting
  plateaus, with no soft cover or cover of bodies and the same phases, each take the block's temperature (their heat
  apart from the fusion enthalpy each holds, over their heat capacity; their floor's top too, which is part of each
  cell's ground node); of a cell of parts (1.1), its liquid's top.
- **The ground by height** (`flow::shores`, `flow::Shore`): the level of each of a cell's parts, by which its
  samples are found by height: the ground above a height, the height above which an area lies, and of the ground above
  a height its land, its ground under the liquid, and of that the ground shallower or deeper than a depth; the land and
  the ground under the liquid together are all of the cell, the land as `flow::land_shares` finds it.
- **The channel** (`flow::channel`): what a cell passes on runs in a channel whose width and depth grow as powers of
  its discharge Q, m³/s (hydraulic geometry downstream, Leopold and Maddock 1953): width `channel_width` ×
  Q^`channel_width_exp`, depth `channel_depth` × Q^`channel_depth_exp`, and its liquid runs at Q over width × depth.
  The discharge of a cell is the most that runs through one of its samples, its main channel (`World::liquid_flux`, mol/s,
  a moving average over about a year, from the samples' discharge; the month's own, `World::liquid_month`, its own shed
  among it, is what a taker can drink of the running liquid), at the mol per m³ of the liquid its ground holds
  (`World::liquid_in`). Its size does not depend on the cell's: every shell reads it
  here.
- **Pore water** flows by Darcy's law (an explicit sweep on the liquid table, the store's too, 1.7): the conductivity
  of the two cells in series (the harmonic mean of their `Pores::seep`, 4) × saturated thickness × width / distance,
  at most half of what would level two cells, and no cell giving more than it holds over field capacity; pores at or
  under their field capacity (`Pores::field`) give nothing. Over cells of kilometres a
  liquid step moves a table a thousandth of the way to its neighbours', far below the explicit limit. Liquid above it sinks to the deep store
  (`drain::sink`, at most its conductivity a second and the store's room) right after each of the air's steps, which bring the
  rain: held until the month's liquid step, it would cover and darken the ground (`land::reflectance`, 1.4) and
  warm it. The store flows by `bed_seep` (`flow::deep`). Real land drains to channels a few hundred metres apart, so both also drain by Darcy's law down
  the cell's steepest slope over a hillslope of `hill` into the runoff (`flow::drain`). Pore water moves before the
  runoff's pass, so the bodies end the month at one level. **None of this runs under a body**: the loose ground under
  a body's liquid lies with its pores full, and the bed's store under it is not drained. So in a cell of parts (1.1)
  the sink, the pore flow and the store's drain act on its land alone (`liquid::soil_share`, `liquid::mobile`): what
  sinks or flows away is its land's liquid over field capacity, on its land's area and pores, at its land's
  temperature; its store drains by its land's share of the cell's area, and what comes up gathers in its liquid. Its
  land's pores drain to its liquid, its land's lowest ground, as a cell's pores drain to its channels
  (`drain::to_channels`, `drain::land_drains`): a move inside the cell from its land to its liquid's top, with its heat
  at the land's temperature; in the runoff's pass the pores of a cell of parts or under a body drain to no channel.
  Both drain down the slope of the cell's solid ground (its land drains to its liquid). A link between two cells
  passes, in each layer, the product of their land's shares (`liquid::reached`), so no liquid sinks into, flows
  through or comes up from the ground and the store of a cell all under a body.
- **A cell of parts** (1.1): what the liquid laws bring to it gathers in its liquid's top; what the laws under the
  surface take leaves its land (`parts::Leaves`); what the others take leaves its top and its deep by their shares of
  the liquid they hold (what stands leaves from the liquid), and its land only for what they lack, which keeps its
  own pores' liquid; at the parts' temperature by the liquid each holds (`parts::to_liquid`, `land::t_now`).
- **What goes with it**: its share of every liquid and of what is dissolved, with heat. Liquid passing through a node
  keeps its fusion heat whatever the node's phase and leaves at the temperature of the node mixed with what came in
  (on a melting plateau the plateau's substance melts or freezes first), so it never takes more heat than the node
  holds and what stays keeps that temperature. A salt is a liquid that never
  boils (11, 12): it moves with the liquid and stays when it evaporates. **Its potential energy goes with it** (0,
  "The books"): each move of the pass goes from the cell it leaves to the cell it lands in, its potential energy from
  the height it leaves (`potential::apply`; what only passes through a cell carries on what it came with), and the
  fall turns to heat where it lands. The head it loses on the way
  does work (1.3).

### 1.3 Erosion and creep
The head lost by moving liquid does work where it is lost on land (`flow::runoff::erode`, `wear`; in a body and below
0 m it wears nothing): in each land cell, g times what runs through each of its samples in no pond in the month, kg,
times the fall to the sample it runs to (the samples' heights, which the run's erosion does not change). It
**carries** up to `carry` m³ of loose solids per J of the cell's work times its loose ground's erodibility against a
loam's (`Texture::erodibility`: M^1.14 (12 − OM) of Wischmeier and Smith 1978, M its middle share times its share
that is not fine, OM its organic per cent read to 4; middle particles are carried most easily, coarse ones are heavy,
fine ones and organic matter bind; a coarse or a fine ground carries a tenth to a third of a loam's, one of middle
particles up to four times) (at most all that lies loose), which go with the
cell's runoff to the cells its ways land in, where they settle, and with the work its load leaves free (the free share of that capacity: the cover effect, Sklar and Dietrich 2004) **wears** `wear` m³ of
bed per J over its strength into loose ground. The load is booked apart from the liquid (`runoff::Crossings`), each
with its own heat. **Known limit:** the load goes straight from the cell to the cells its ways land in, as the liquid
does (1.2): it is not checked against what the channel can carry at each sample on its way, so it does not settle
where the channel slows (where a sample's fall is less than the one before it, on a plain, at a body's shore), and
the cells it passes neither gain nor lose by it. The heat of the liquid's fall lands with the liquid, not along its
way. **Creep** (`flow::creep`, yearly, out of land cells):
loose ground slides downhill as a slow diffusion, `creep` × softness (1 / (hardness + `hard_min`)) × thickness × slope,
across each border by its width (none across a corner). What stands on a cell holds the ground: carrying and creep
are cut by 1 / (1 + its mass per area / `hold`) (`contract::Stand`). Most rock leaving the land is lime dissolved by
reaction 8.

### 1.4 The air's step, the sun and the cover's share
`flow::air::step`, every `AIR_STEP_DAYS` (15). First the gas of the aside nodes comes up into their air nodes. Then each cell's exchange over the step with the air it sees is solved once (the sea under one air
node as one surface, below), in its own implicit surface balance (Newton's steps about its end temperature, with its melting plateaus, 1.1), at the air of the step's
start: sunlight, heat radiation, sensible heat with the air touching it and what evaporates from it by its saturation
deficit (`air::cells::exchange`); of a cell of parts (1.1), what faces the air: its land and its liquid's top each
solve their own balance, the land under the sun of the cell's slope, the top, which lies level, under the flat sun,
and the cell answers the air as their sum (`cells::parts_respond`), each booked what its own law gives at the step's
mean air, the rain on the cell landing on each by its area (`book::parts_moved`); a top with no land beside it answers
as the cell. Its slopes with the air it sees come from that same balance, on the branch it ended
on (the implicit function theorem: on a melting plateau with the share of the step it lasts, none at a soft cover's
cap; the reflectance is taken as fixed, though a soft cover's end reflectance moves with the air), so the exchange is
one law, linear in the air it sees. The slopes are cut so that, however that air moves in the
step (its temperature by up to 60 K, each gas between none and twice what it held), the cell never gives more of a gas
than it holds, nor more heat than it holds with its sunlight (`cells::hold`). The air level then runs day by day,
explicitly, on plain arrays of its nodes (`air::daily`): each node's exchange with its ground by that law summed over
its cells, its heat radiation to space and its rain (1.5). Each block's heat, then its vapour with its rain, are linear
in them through the day, so the day follows them exactly, the block warming and moistening as one as the eddies even
it, the air outside it as at the day's start; days of an hour give the same climate within 0.03 K. Then the evening of
its block and the wind (1.6). The book takes each cell's law at the mean air of the step (`air::book`), with what the
step's rain added to what it evaporates (1.5), which is what the days summed, so the air ends just where its days left it and no node gives more than it holds. A 15-day step
gives the climate of 1-day ones within 0.3 K on the land and the land's runoff within its spread from year to year
(`the_long_step_keeps_the_climate`, `tape/tests/ranges.rs`).
- **Sun** (`radiation`): the daily mean by latitude, season and day length (Berger 1978), declination sin δ = sin ε sin
  λ, the mean over the step's days. **On a slope** (`on_slope`), per m² of the cell's plan: by Lambert's cosine law the
  beam on ground of upward normal N = (−∂z/∂x, −∂z/∂y, 1) is S (s · N), s the unit vector to the sun, linear in the
  slope, so a cell takes the sun of its mean slope and a square's sun is the sum of its cells'. Over the hours the sun
  is up the east-west part cancels and the north-south part is the slope of the daily mean with latitude: Q + r ∂Q/∂φ,
  r the cell's mean rise toward the south (section 0), the equivalent latitude of Lee (1962) and Swift (1976) in plan form. An
  hour a face turns away from the sun counts against it, as the shadow it casts on the ground beyond, so the beam is
  kept whatever the slopes; a cell whose day sums below zero takes none (the one place a square and its cells differ).
  A cell under a body's level (1.2) lies level and takes the flat sun. **Through the air** (`radiation::Light`, read
  by the surface's step and by the books alike): the gases of its air node absorb sunlight (`radiation::Column`) with the
  optical depth of the amount on its path (`Chemistry::sun_depth`: `sw_abs` a mol/m², past `sw_sat` as the square root
  of the amount, the form of their heat radiation) and their molecules scatter it (Rayleigh: `scatter` a mol/m² on the
  path, half of it on up and half down, so the surface gets what passes and half of what is scattered, and again half
  of what it reflects that the air scatters back), on the way down along the sun's slant, 1 / its mean height over
  the hours it is up (`sun_height`), and of what the surface and its cover reflect on the way back up as diffuse
  light (slant 1.66, Elsasser's diffusivity factor). What the air absorbs warms its node (booked from the sun to the
  air, and summed into its days as a gain the air does not move); what reaches the surface, the surface's balance
  takes; what passes back up leaves to space. The three add up to what came in, to rounding. Under the share of the
  node's sky that its cloud covers, the cloud lies over that column (**Clouds**, below), and a cell takes the clear part
  and the part under the cloud each by its share. All the sunlight at the top of the air is taken as on the cell's
  slope, whatever a cloud scatters of it; diffuse light would fall on the plan as on flat ground. The cost is a product and a sum per cell a step,
  with the slope with latitude found beside the daily mean, once per latitude. The surface reflects as its cover is at the step's start (`land::reflectance`),
  over the cell's own ground: the share of it under the level of what stands over its pores (`curve::Fill::wet`) as a
  column with that liquid over the share, the rest as one whose liquid is what its pores hold, by area;
  where a soft cover lies, as it is at the step's end, so a cover that melts away darkens the ground in its step. Snow
  reflects from the step after it falls. **Known limit:** a substance reflects the same share of the sun at every
  height of the sun and in every state (its `refl`), where a smooth liquid reflects more of a low sun (Fresnel) and
  old or wet snow less than fresh.
- **Heat radiation** (`radiation::band_depths`, `radiation::Heat`): surfaces emit σT⁴, split in three bands of wavelength
  (`Chemistry::heat_spans`), each body sending in a band its share at its own temperature (Planck's law,
  `Chemistry::heat_shares`; the shares sum to 1). In each band a gas's lines have the optical depth
  `Chemistry::depth` (in proportion to its column, past `sat` as its square root) and lie where its amount times the
  pressure is, as the pressure broadens them: at height hk H / (hk + H), hk the height its amount fills
  (`Scratch::hk`), H the air's. Its continuum (`cont`) goes by its amount times its own pressure, in every band; that
  product falls by e over hk / 2, so the continuum lies at half the gas's height. Each band is a core of strong lines
  `line_spread` times its depth and wings 1 / that times, each half its light; under a cloud (below) its drops, which
  are grey, add their depth to every part at the air's height; each part absorbs 1 − exp(−x) and emits from 2 grey layers that cool with height, so more gas
  lifts the layer that reaches space into colder air and the ground warms. The layers are each air node's, over its
  mean ground; the air touching a cell is the air it sees (bilinearly, its four nearest nodes), colder by the drop with
  height from that air's mean height.
- **Clouds** (`exchange::cloud`, `radiation::Cloud`): each air node's cloud covers the share b of its sky that is
  saturated (`Seen::clouded`, 1.5), a mean over the step before. What the node rained in that step stays droplets for
  `cloud_life` before it gathers into drops that fall (the cloud time of Smith and Barstad 2004; a falling drop is too
  wide to take much light), under the cloud's share alone: a path of liquid of the node's condensation rate ×
  `cloud_life` / b, in droplets of radius `drop_m`.
  A droplet much wider than the light's wave takes twice the light that falls on its cross-section out of a beam, so
  the cloud's optical depth to sunlight is 3 path / (2 `drop_m`) (Stephens 1978). **Its sunlight**: it reflects,
  passes and absorbs as two streams give (Eddington's form, the forward peak of the scattering taken as light that
  passes: Meador and Weaver 1980; Joseph, Wiscombe and Weinman 1976) under the sun's mean height (`sun_height`), its
  droplets scattering forward by the mean cosine `drop_forward`; in the share `drop_dark[0]` of the sunlight (the
  near-infrared, which the liquid absorbs) a droplet absorbs the share `drop_dark[1]` of what it meets (Slingo 1989),
  and that warms the air node with what its gases absorb. The cloud lies over the node's column: what it passes
  straight goes on as the beam, what it passes scattered as diffuse light, and its base sends back down its share of
  what the column and the surface reflect up (`Column::under`). **Its heat radiation**: a droplet absorbs the heat
  radiation that falls on its cross-section, half of what it takes out of a beam of sunlight, so the cloud's depth is
  half its depth to sunlight on the slant of diffuse light, the same in every band (`radiation::drops_heat`). The
  clear part of a node's sky and the part under its cloud are each their own column, for sunlight
  (`radiation::Light`) and for heat radiation (`radiation::Skies`); a cell's balance and the node's own radiation to
  space take the two by the cloud's share. **Known limit:** the cloud's droplets are spread like the air's mass, not
  set where they condense, so a deep cloud sends its heat to space from the top of the air. The ground also trades
  sensible heat with the air (`sensible`).
- **The aside node** (`Class::Aside`, one per cell): light the cover absorbs warms the ground, except the share its
  light reaction stores (`Cover::keep`), booked from the sun into this node; the cover takes it at its month, and what
  it does not claim goes back to the ground before each air step (`cover::release`).
- **A body as one surface** (`air::surf`): under each air node a body's cells that mix as one (1.2) face the air as
  one surface: one balance with their summed area, heat, heat capacity and gases, at their mean temperature (by heat
  capacity), reflectance, height and air (by area) and the sunlight of their mean latitude. It answers the air as one
  (`surf::Patch`): its law, linear in the air it sees (its cells' weights by area), is summed into its node once, and
  the book takes it once, at the mean of that air over the step. Each cell takes its share of what it gives: of the heat
  by its heat capacity, so they warm alike, and of each gas by what it holds of it, so none gives more than it holds.
  Its rain falls on it as on one place, at the air touching it by area, shared to its cells by area (1.5).
- **Gas from the cover** goes into the aside node with its latent heat; the air's step takes it up with
  its lift.
- **Shade**: the cover's kinds share its light by height (3).

### 1.5 Evaporation, rain and dissolved gases
`flow::air::cells`, `daily`, `rain`, `soak`, `exchange`. **Evaporation** by vapour pressure (the ground's saturation pressure against
the air's): a liquid in the ground's pores by the share of it the ground holds for roots (`Pores::available`, the
same share the plants feel, 3): at the open rate at field capacity and over, none at the wilting point, in proportion
between (Manabe 1969's bucket, its store the liquid a ground holds for roots): the air does not dry it below the
wilting point (below it a ground dries only slowly, from its top, stage two of its drying; Ritchie 1972); a solid in
the pores down to none; over the
cell's own ground as its reflectance is (the share the standing liquid covers holds what its pores hold and all that
stands, the rest what its pores hold, each a bucket by its area); and through a second film beside the air's where
one stands over its land (`World::second_film`: the cover's open pores, 3), in series with the air's film, at its
open rate while the share of the ground's liquid behind it lasts (no wet threshold: its roots reach that liquid),
from what the first leaves; both inside the surface's balance, taking the heat it held at the node's temperature; a node over a substance's boiling point boils of it what its heat over that
point pays, cooling to it (`land::boil_off`; gas coming out of the liquid too). **Rain**: a node's air is not evenly
moist (`seen`). Its moisture is spread evenly from wetter to drier, over a width of 1 − `moist_min` of what saturates
it either side of its middle, and its saturated part holds no more than saturates it. So its moistest air saturates
when the node holds `moist_min` of what saturates it, all of it at saturation, and between them the share
b = 1 − √((1 − r) / (1 − `moist_min`)) at the node's ratio r to that (`seen::saturated`; Sundqvist et al. 1989, whose
critical humidity is measured from cloud cover against humidity, Quaas 2012). The air over each cell is
`seen::MOIST_PARTS` equal parts that saturate one after another by that law (`seen::parts`), against what saturates
the air touching the cell (colder higher up). **The ground lifts the air** (`air::rise`). The wind over the ground is
the overturning's own low flow (1.6) turned by the spin: `wind` times the warmth's gradient, down it inside the
month's cell and turned by the spin across it. It comes from the planet's gradient at the node's latitude and month
where the map's edges are open, from the air's own where a node is a block alone, and is none where a node shares its
block (its air is evened) or the world has no planet's air. The warmth falls toward the pole at every latitude, so
this wind is an easterly at every latitude here, of 0.4 to 1.6 m/s. **Known limits:** there are no westerlies of the
middle latitudes, which the eddies drive (Vallis 2017, 15), so there the wet side of a range is its east, where on a planet
like ours it is its west; and the wind is some five times weaker than the measured trades (5 to 10 m/s), as the
overturning it is part of carries about a fifth of the measured flux (1.6). Where it climbs the ground's rise along it (the surface the air touches, the
liquid's on the liquid, fitted to the bordering cells) it lifts the air at the ground's density, and where it
descends lets it sink; summed per node, a rising (or sinking) mass flux. In `rain_time` the share f = min(1, |that| ×
`rain_time` / the lower half's mol) of the node's lower air changes places with its upper half's, which holds
ρ = split / (1 − split) of the vapour per mol the lower holds (`Scratch::split`, the column's profile): rising air
condenses down to the upper half's cold, so the node saturates at what saturates it times 1 − f (1 − ρ); sinking air
brings that much drier air down, so the node saturates only at that over 1 − f (1 − ρ) (Smith and Barstad 2004: rain
where the wind climbs, a lee where it descends). It changes only what the rain law sees within a step: no number is
kept, and the rain is booked as all rain is. **Known limit:** the rise is summed per node, so what it adds to a
node's rain lands as the node's other rain does, by each place's own saturation and the drift: a node that holds a
windward coast rains more over its sea too, which does not rise. Each saturated part rains out in `rain_time` what the node holds over
the ratio the part saturates at: only the saturated share condenses, and the node's mean stays under saturation. That
share is the cloud's share of the sky (`Seen::clouded`, 1.4). Evaporation reads the node's mean vapour: it is linear
in the vapour, so the mean over the parts is what the mean vapour gives. Each node's air
lies over the cells that see it (bilinearly), each by its share; the cells' saturation is taken as the step starts, as
a ratio to the node's, so the day sums the parts in rising order. In the air's day the vapour of each block (1.6), evened
as the eddies even it, follows what its cells evaporate by their law and what its nodes rain: both are linear in the
vapour between two of the ratios at which a cell starts or stops raining, so the day goes exactly from one such ratio
to the next, with no smaller steps (`rain::Pool`); the air outside the block is as at the day's start. What a node
rains in a day condenses over those cells (the sea under one air node as one place, 1.4) by what the parts over
each would hold over their own saturation at the ratio the node rains at that day, so it is heaviest over high ground; it lands there by the share
1 - e^(-d / L) of it, d the place's width (the root of its area), L the drift's length, the node's low wind
(`rise::low_wind`) times `rain_drift_s`, the time its drops take to form and fall (the linear theory of orographic
rain, Smith and Barstad 2004: a cloud time and a fallout time of about 1000 s each); the rest drifts
(`seen::Seen::landing`). It leaves the node's centre and lands along the wind by an exponential of mean L; each point of
its path lays its share over the nodes round it as a cell reads them (bilinearly), and each node over the places that
see it by its air's share of each (`drift`). So the drift crosses the nodes' borders, and no cell's rain is cut off
where the lattice of the nodes ends; a share whose path leaves the map lands on the map by the rest's shares. Where a
node has no wind (its air shares a block, or the world has no planet's air) its rain lands where it condenses.
**Known limit:** on a map whose cells are
as wide as an air node (the globe), the cells that see a node reach half a node into the next rows, so part of a
node's rain lands a row away, toward the wetter node's neighbours; and no frozen cover moves. So the polar band of the
lifeless globe gains snow it does not lose: its ice grows, about 40 m in 40 years (an island's cells are far smaller
than its air's nodes, and its ice holds steady). **The rain in the bucket** (`soak`): a cell's exchange over the step
is solved without the rain that falls in it; the rain lands day by day as the air's days make it, and each day's rain
wets the bucket it falls on (the share of the ground the standing liquid leaves, or all of it), followed exactly
through the day under the open rate of the step's balance, what rises over the wet bucket sinking as it falls (the
bed's store takes it after the step, `drain::sink`). What the bucket then evaporates over what it would have without
the rain goes into the cell's air node that day, the cell's other outputs moving with it by the slopes of its
balance (its latent heat cools the surface), and the book takes the same; the rain itself is booked once, as it
landed. The second film and a body's surface, which evaporate at their open rate, take no more for it. The rain reaches each cell at the temperature of the air touching it (`touching`),
the air paying what warms it on the way down. Where the node's air at its mean ground is below the gas's melting point
it falls as a solid (snow) and the air keeps the fusion enthalpy its freezing frees; condensing frees the vaporisation
enthalpy in the air. **Lift**: gas going up is lifted from the ground's head to its air node's mean ground, paid by the
air's heat; rain's fall gives it back (0, "The books"). **Dissolving** (`exchange::dissolve`, once a month, at the end of the liquid step, over the month, at the ground's temperature as the
liquid step leaves it and under the air of the last air step's start; the gases that dissolve are chosen at the month's
start and evaporate in no air step of it): a gas that cannot condense dissolves in the ground's liquid toward Henry's share, through the
air's film (`evap`) and the liquid's (`dissolve`) in series, freeing its vaporisation enthalpy, and comes out when the
liquid warms, thins or freezes. The sea under one air node (1.4) takes it up as one liquid, each cell by its share of that
liquid, and gives it up by what each holds of it. **Known limit:** a liquid takes up gas into all its depth, its deep
as well as its top (1.1), so a deep sea takes up gas in months; a real one takes centuries.

### 1.6 Wind
`flow::air::wind`, `daily`. Eddies (`mix`) even out the air within a day over the distance sqrt(`mix` × day), about
300 km (Taylor 1922, `daily::BLOCK_DAY`): the air nodes lie in blocks of that side on the map, and each day every gas of
a block is shared among its nodes by their volumes and its heat so that each has the block's warmth (its temperature
raised to a common height by the block's drop with height), so its pressure, shares and warmth are the block's. An
island smaller than a block (the islands of 64 and 128 cells) is one block, and has no wind: only the evening. Across the links between blocks gas moves by three pushes, each day from the air at
its start. **Pressure** (p = nRT/V), evened in the time sound takes to cross the link (Lamb waves, Lindzen 1990),
explicitly: the sender's air, each link at most half of what evens it over the count of links of the busier of its
nodes, so no node gives more than half of each gas it holds. **Known limit:** sound is faster than that, so a pressure
difference across many links of blocks takes days to even out. **Eddies** mix the columns: as many mol each way, at
`mix` × the column's mol × width over length a second, so every gas, the vapour with its latent heat, mixes by its
share of the column. The heat a mol of them carries is its node's near-ground air's static energy, cp T + M g z at the
column's ground (`wind::aloft`), not the column's mean heat per mol: eddies stir the lower air, where the contrast is
the ground's, and a column's heat aloft (how fast it cools with height) is not theirs to even out. Both sides book the
one number, so the books stay exact; the energy a mol carries over the column's mean heat goes as a fixed flux of
the day, from the air at the step's start. So their heat goes as -(p_s / g) `mix` ∇h, h the near-ground air's energy
per kg, the diffusion the eddies of the measured climate follow (Hwang and Frierson 2010). **Known limit:** the vapour
goes by its share of the whole column, not the near-ground air's, so its latent heat carried is about a third of what
that diffusion of the near-ground air's moisture would carry. **Overturning**: the lower half flows to the warmer column and the upper half back, a
mass flux of `wind` per K/m of the warmth gradient along the link (the mean of its two ends' gradients, each fitted
to its node's neighbours, `wind::gradient`, an edge node's among them the columns of the planet's air beyond it,
as a node inside has neighbours on every side: a wave no longer than two nodes is the mesh's own mode and no flow of
air's, and a flow that answered to a difference one node wide would feed it, moist air drawn to the warmer node
warming it as it rains; the eddies even it), turned by the
planet's rotation (`rotation`), through half the link's cross-section, so it does not depend on the size of the nodes.
Down the gradient it runs only inside the month's cell of the low latitudes (`air::cell`): the upper air flows
poleward keeping its angular momentum, and past a latitude the warmth's fall can no longer balance the wind that gives
it (Held and Hou 1980). In their small-angle form, with the thermal Rossby number R = g H Δ / (Ω a)², H the air's
depth (`air_top_m`), a the planet's radius, Ω its spin (`units::SPIN` × `rotation`), Δ the fractional fall of the
warmth of radiative balance from the equator to the pole, from the planet's own yearly sunlight by latitude,
1 - (pole's / equator's)^¼ (with the default sun and tilt Δ = 0.198): a heating that peaks on the equator makes the
year's cell, from the equator to ±sqrt(5R/3) = 22.2°. The month's heating peaks where the sun by latitude, as the low
liquid under it feels it, is greatest within the year's cell: a layer of `h_mix` of the key liquid answers the sun
with a first-order lag, τ = its heat capacity / (4 σ T³), T the warmth of radiative balance of the equator's yearly
sunlight (τ = 1.18 years with the defaults; each harmonic of the year damped by 1 / (1 + i k ω τ)); the peak swings
between about 5.5° south and north, some two months after the solstices. About a peak off the equator the cell is
Lindzen and Hou's (1988), in the same form: its two halves meet at the latitude the air rises at with one warmth, each
ends where its warmth (angular momentum kept from there) meets radiative balance, and each holds as much warmth as
radiative balance (equal areas); where the summer half has none, the winter half alone. So the air rises poleward of
the peak (near 19° at a peak of 5.5°), and the winter half, across the equator, is the wide one (to about 36°); the
rain belt and the sinking branch's dry air follow the months. The share of the flux down the gradient is whole where
the air rises and falls as 1 - (distance from there / the half's width)² to none at the half's edge, where the
theory's cell ends, and is none beyond; the part turned across the link stays. With `rotation` 0 the cell spans the
planet. The same at a link inside the map and at an island's open edge, by the link's latitude; an
island alone is at its own rows' latitudes (`lat_south`, `lat_north`). **Known limits:** the cell of the middle
latitudes that turns the other way (Ferrel's), which the eddies drive, is not made; past the cell the eddies alone
carry heat poleward. The year's cell is narrower than the measured one (22° against about 30°), as its edge is taken
from the yearly sun's contrast (0.198), where the measured contrast of the air in radiative balance is nearer a third
(Held and Hou 1980 take 1/3); and it is weaker: at half its edge its lower half carries about a fifth of the measured
cell's 1e11 kg/s (`wind`).
Each half carries the air of the node it leaves, vapour crowding low, each mol with its node's heat per mol, and the
upper half holds more heat per mol than the lower by the rise of the static energy cp T + M g z over half the column's
depth, (M g - cp gam) × depth / 2, gam the column's drop with height (none where the air cools with height as fast as
dry air rising; Neelin and Held 1987). The part of the flux down the gradient carries that rise from the warmer column
to the colder; the part turned across the link runs along the isotherms, where what its halves carry of it warms no
column (a flow along the isotherms crosses none; Holton 2004, 3.4), and it carries none of it. So the heat it carries a
second is its mass flux times the difference of the two nodes' heat per mol, less the rise times its part down the
gradient: the first part grows with the length of the link, the second does not.
**Known limit:** as a half carries its node's air, on coarse nodes the overturning also mixes the two, as eddies of
about its speed times half the length of the link would. The eddies and the overturning are one implicit (backward
Euler) step a day, first along the lines the links between blocks make east and west, then along those north and south,
each line solved exactly (`solve::line`: Thomas; a ring by Sherman and Morrison): stable at any rate, no gas of a node
goes below zero, and the day keeps its rates on any grid. **Known limit:** their rates per mol a node holds are those
of the air step's start (15 days), so a node whose air grows or shrinks within the step moves a little more or less
of it than its rates say. Mass that crosses is
lifted between the nodes' mean grounds by the heat of the node it enters. A cell sees its air node and three
neighbours bilinearly (`flow/scratch.rs`): everything it reads of its air, the air's temperature, vapour and drop
with height, the share of the sunlight its air and cloud let through, the heat radiation its air sends down and how
much of the ground's it absorbs, and the height its vapour fills, is shared so between the nodes' centres, and none
steps at a node's border (`cells::exchange`, `World::near_ground`); what it gives its air and takes from it is booked
with its own node. Air moves as whole columns.
**The map's stretch** (`air::stretch`): the air over a small map behaves as the air over the band
of latitudes its rows stand for, while the ground keeps its true metres. The air's sideways laws take every horizontal
length `stretch` times longer: a column stands for one `stretch` times longer and wider, so it holds `stretch`² times
its gas and its faces are `stretch` times wider, and a rate between two stretched columns moves the map's column the
share of it the map's column holds (`Stretch::per_column`). So the eddies' rate between two columns falls as
1 / `stretch`² (`mix` × width / length, over `stretch`²), the overturning's as 1 / `stretch`² too (its mass flux
does not depend on the length, as above, and the column holds `stretch`² times more; its speed, `wind` times the
gradient, falls as 1 / `stretch`), the turning is the same (the warmth's difference across a link is its gradient
times its offset), the share of the pressure's difference evened in a day falls as 1 / `stretch` (sound takes that much
longer to cross), and the blocks' side in nodes divides by `stretch`, and with it the sea's mixing under each block
(1.2). The exchanges up and down (the ground and the air, the sun, heat radiation, rain out) are per m² and do not read
it, nor the rain's drift, nor anything on the ground. `stretch` is the band's length on the planet (`Config::radius_m`
times the band in radians) over the map's from south to north, for a world in the planet's air (below); alone, and
on a globe (its own band), 1.
**Alone or in the planet's air** (`air::edge`, `air::planet`, `World::planet`). A world is made either alone: its air
closed at the map's edges, it makes its own climate from its own sun, sea and life (every world made without a band:
`ew run` with no `--band`; the page makes none); or in the planet's air: made with a band (`ew run --band S,N`, the page's Wide or Mild, or
`band=`), it holds the planet's table (`flow::Planet`) and, its band longer than its own span (`stretch` above 1), its
edges open to the air of its latitudes, and its air's sideways laws take the band's stretch (alone, the map's own
metres: a closed box stretched over a wide band freezes over). The table's presence is the state. The planet's air comes from a strip of the planet from pole to pole, run by
these laws when the world is built (`worldgen::planet`): three air blocks round, rows of 5°, all of it low open ground
(liquid over all of it), for 10 years from its starting state (an island of 32 x 32 on the band 20° to 65° feels a run of
20 years within 0.2 K in every row), the same strip for every island (the page makes it once a session);
its last year's air per row and month (its temperature, warmth, mol per m² and each gas's share) is saved with the
world.
Beyond each air node of the map's edges lies a column of that air, north and south at the latitude one row of nodes
on, east and west at the node's own, at the time of the year (between rows and months, linearly), over low open ground
at 0 m, its warmth's gradient the strip's. It is linked to the node as two nodes are (`wind::push`, the stretch's
lengths): its eddies and its overturning, the lower half outward where it is warmer and the upper back, each half of
its own air, turned by the rotation; across the east and west edges the turned overturning of the planet's gradient
is its mean wind along the row, in at one side and out at the other in the lower half and back in the upper; its
pressure is the planet's, so no evening crosses. Each day, after the wind, each such link is one implicit step with
the column beyond held. What crosses comes from and goes to the boundary node for outside, with its heat and the
latent heat its gases hold in the air; mass that enters pays its lift from 0 m, mass that leaves takes its potential
energy there (`potential::Lifts`). The heat that comes in at each edge is counted (`Counters::edges`, printed by
`ew run`). **The planet's low liquid**: where the edges are open (`World::edges_open`), the part of the open low ground
that holds a cell at the map's edge (`Body::edge`) is joined to the planet's low liquid, which stands at the level the
world is made with (`space::wear::STAND_M`, 0 m): in the month's runoff (1.2, `runoff::stand`) it stands at that level
as at a pass, what stands over it leaving, mixed, for the boundary node for outside, and what it lacks of it coming in
from there at its cells at the edge, by the area their parts cover, as the liquid and heat those cells hold, mixed (the
planet's strip holds only air). So what the island's air gains or loses through the edges as vapour comes back as
liquid, and its level holds each month. Lakes and closed basins, and the open low ground of a world alone, are not
joined; a cell of land at the map's edge opens nothing, only the cells of the open low ground there. What crosses is
counted (`Counters::edge_liquid`, m³ and its heat; the series' `edge_liquid`, `ew run`'s line of the edges).

### 1.7 Each law takes one step
An explicit step is safe while `dt Σ g / cap` of each node stays within 1/2 (no pattern changes sign in a step,
`solve::sweep`). For a diffusion of D m²/s on a cell of side Δ with four borders that is 4 D dt / Δ² ≤ 1/2, so a step
of `dt` is safe down to Δ = √(8 D dt). On the island (month 2.59e6 s, year 3.11e7 s):

| Process | Step | D or rate | Safe down to |
|---|---|---|---|
| Pore flow, Darcy (`Pores::seep`) | month | conductivity × 1 m of loose ground / porosity: 9e-6 m²/s for a loam, 8e-5 for the most coarse | 14 m, 41 m |
| The bed's store, Darcy (`bed_seep`) | month | `bed_seep` × the depth it fills / `bed_porosity`: 4.5e-3 m²/s full (30 m) | 305 m full |
| Heat sideways (`land::conduct`) | year | k / (ρ c) of the active layer, about 1.1e-6 m²/s | 17 m |
| Creep (`creep`) | year | `creep` × softness × 1 m: 1.3e-9 m²/s for the softest solid | 0.6 m |
| Erosion and carrying (`carry`, `wear`) | month | the bed lowers `carry` ρ g Q per m of drop a step; at the largest river of the 64 x 64 island (Q 1.9e8 m³ a month) under half the drop to the next cell | 9 m |
| Runoff (`runoff`) | month | one pass down the samples' ways: liquid crosses the map in one step | any size |
| The bodies' levelling (`runoff`, `basin`) | month | one level, found exactly | any size |
| The air's exchange with a cell (`air::cells`) | 15 days | implicit in the cell, per m²; the air's days run on the blocks | any size |

The drain to the channels inside a cell (`hill`), the sink into the store and what comes up are exact for any step in
the cell itself. The cells as built are squares of kilometres (on a globe hundreds of km) or coarser, far above every
limit of the table, so each law takes one step.

## 2. Change: substances react inside nodes, faster when hot
Files: `core/src/change/` (the reactor), `core/src/chem/` (chemistry and its physics), `worldgen/src/chem0.toml`.

### 2.1 The rate law
```
rate    = catalyst · k · exp(−Ea / T) · contact · Π over inputs (share of its phase ^ order)
lim     = the least, over the inputs, of (amount / count)      (the scarce input)
contact = what lim would be if each input were all of its phase
extent  = lim · (1 − (1 + p·z)^(−1/p)),   z = rate / lim · time   (1 − exp(−z) when p = 0)
p       = (order of the scarce input − 1) · (1 − its share)   for a gas or liquid; 0 for a solid
```

**Share** is an input's share of its phase; a solid is a phase of its own, so a reaction between a solid and the air
goes like a surface. **Order** is the count or `Reaction::order`. **Extent** is the exact integral over the step
(`change/kernel.rs`), never past the scarce input. **Heat**: −ΔH to the node (`Chemistry::dh`); formation enthalpies
are of the solid, so a liquid holds its fusion heat and a gas its vaporisation heat too. **Light** reactions take
absorbed light and store `light_yield` of it. **Open world** (`Reaction::open`): in ground, liquid and air live the
decomposers, whose catalyst `open` works as far as the
node is moist: 1 + (`open` − 1) × moisture (`change::open_catalyst`). Moisture is how wet the node is as living things
feel it: d / (d + `wet_depth`), d the depth of its liquid in its pores and standing (`FlowView::moist`,
`Physics::wetness`), so decomposers work on dry land too, slower; an air node has none. `open` is 1 for most reactions; 200 for reaction 1 (the free sugar's rot, about a
week); 5 for 7 (body matter rots 3.8 times faster than the frame); 10^6 for 12 (the frame rots by the decomposers
only: bare, a millionth of that).
`change::rate` (catalyst × k exp(−Ea/T) × the shares and contact of `pace`) is the one rate of all nodes. **Frozen**: a bare reaction whose rate constant k exp(−Ea/T) would
use under 1e-12 of an input over the world's year is skipped in every node (`frozen_below`, `NEGLIGIBLE`), a speed
tolerance.

### 2.2 Where it runs
**Ground and air nodes** (`change::step`, `Reactor::react`) react when due over all the time since, at Flow's last
temperature (a plateau's fusion heat set aside), then sleep until their reactions would use 5% of an input or warm
them 0.5 K, at most `sleep_days`. The nodes under one air node react as one task (a block), each into a row of what its reactions changed and the gas it gives its air node; the rows are booked after the call, each array of the store by one task (`change/booking.rs`, `store::book_by_array`), so an air node gets the gas of its ground nodes in the order of their cells whatever thread ran them. Gas a ground node takes or gives
(what its reactions made less what the node itself gained, so it never gives more than it holds) crosses to its air node with its heat and latent heat. **Growth** is a
conversion the cover runs, paid by work (3).

### 2.3 What gas a ground node touches
Its own gas and its cell's share of the air. **Under standing liquid** only the gas dissolved there, renewed through
the liquid's film in τ = depth / `dissolve` (`FlowView::standing`): a gas it takes is held at Henry's share × (1 +
made·τ/eq) / (1 + used·τ/eq). So dead matter on the sea floor rots as fast as the film brings the oxidising gas.
**In full pores** (on ground with no liquid standing over it): a reaction of the open world that takes a gas (the
decomposers' rot) runs only in the share of the node whose pores hold gas, 1 − the share waterlogged
(`inside::airless`, 0: its land's saturated share, and of the drier rest what it holds over field capacity, which fills
its pores from the bottom up; the same rule by which roots in full pores get no gas, 3). So ground whose pores stay
full keeps its dead matter and builds organic ground, as peat builds on wet flat ground and valley floors (Clymo 1984).

### 2.4 Phases and the freezing law
Solid, liquid or gas by melting and boiling points (`Chemistry::phase`). A liquid's melting point in a mixture falls by
the ideal-solution law (`Chemistry::melt_in`): 1/T = 1/Tm − R · ln(x) / Lf, x the solvent's share among itself and
its solutes. Only the solvent (the most abundant liquid) is lowered; solutes are liquids melting below it (the salts 11
and 12, the fat 6); ice and liquid count together, so it freezes at one point. One law for ground plateaus, ice cover
(`World::thaw`) and Change. **Latent heat**: a gas in the air holds its melting and boiling heat
(`Substance::latent_gas`), paid or got as it enters or leaves, and liquid taken or given carries
it. **The densest temperature** of a liquid (`Chemistry::dense_in`, what overturns a liquid's top and
deep, 1.1) is its pure liquid's (`Substance::dense`) lowered in proportion to the mass of its solutes per mass of it
(Despretz's law, `Substance::dense_drop`), and no lower than its melting point there: a liquid salty enough is densest
where it freezes. The sea of chemistry 0 is nearly fresh: about 0.2 g/kg dissolved (a real sea about 35), so it is
densest at 276.9 K, stops overturning below that and freezes at its top, as a lake does.

### 2.5 Pairs at balance and the sea's carbon
`change::reversible` finds each pair of reactions where one undoes the other; `Reactor::balance` finds from the
chemistry alone where a pair goes at equal rates. In chemistry 0: 8 makes the dissolved salt 11 from lime, the carbon
gas and the liquid; 9 turns it back. Wet cells start at that balance (`worldgen::terrain`), so the sea holds several
times the air's carbon and gives it back when the air thins, with no rule for the sea.

### 2.6 Chemistry 0 in short
One fixed Earth-like chemistry until M7 (handover 6.0): 6 elements, 14 substances, 14 reactions, numbers only in
`core` and `worldgen`; Earth names in `library/chem0.toml`. Molar masses are sums over elements, so reactions keep mass.

| Number | Role (≈ Earth name, for reading only) |
|---|---|
| 0 | the common liquid, key liquid of bodies (≈ water) |
| 1–4 | air gases: inert, oxidising, carbon gas, methane-like (≈ N2, O2, CO2, CH4) |
| 5 | energy-rich solid, the light reaction's product and the main fuel (≈ sugar) |
| 6 | fat-like store |
| 7, 8 | rock: sand and common rock (≈ SiO2, CaSiO3) |
| 9 | lime (≈ CaCO3), rock |
| 10 | body matter (≈ tissue): C120H198N6O107 and one of the rock element (Ca) |
| 11 | the rock salt, lives dissolved (≈ Ca(HCO3)2) |
| 12 | the nitrogen mineral, lives dissolved |
| 13 | the frame, a solid bodies build (≈ cellulose) |
| 14 | the slow organic matter, held on fine and middle particles (`bound`; ≈ humus): C59H78N6O42 |

Reactions: 0 light to sugar; 1 burning sugar (its rot in the open world); 2 and 3 burning and making fat; 4 burning
methane; 5 rotting to methane; 6 rock weathering; 7 rotting or burning body matter (gives back the nitrogen mineral and the salt 11); 8 and 9 the lime–salt pair; 10 fixing nitrogen; 11 the nitrogen mineral breaking down when hot; 12 rotting
of the frame; 13 splitting the frame into sugar (needs an enzyme); 14 body matter rotting into the slow organic matter
(beside 7, at a quarter of its rate: a fifth of what rots of it goes this way, keeping half its carbon and all its
nitrogen, its salt 11 back to the ground); 15 the slow organic matter rotting (25 times slower than 7, its nitrogen back as the mineral). Dead matter rots in moist open ground at a litter
constant (12: about a quarter a year at 290 K; 7: about one a year). **Growth** (`Chemistry::build`, bodies only, paid by
work): matter from 118 sugar + 3 nitrogen mineral + 1 salt 11; frame from 6 sugar.
`Chemistry::minerals` are growth's inputs no reaction burns (12 and 11), `materials` the solids bodies build (13).
The air's carbon gas (3) settles where the cover's light reaction takes it up as fast as rot and burning give it back:
the world has no source of it (an input from below, as volcanoes give, would be an edit from outside the laws, and is
not built).

## 3. Life: the cover, light-eaters as a field
`core/src/cover/`. Each cell holds up to its world's `Config::variety` kinds of **cover** (a setting of a world, 8
unless a start chooses other, at most `cover::SLOTS`, 12, the size of its arrays), each the
amounts of three substances of chemistry 0 per cell (the fuel 5 its light reaction makes, body matter 10 and its walls
13: `cover::Parts`, found from the chemistry's roles) and 10 genes (`cover::LIGHT`, `LEAF`, `WALLS`,
`STORE`, `FROST`, `MOIST`, `SEED`, `REPAIR`, `FIX`, `GAS`; ranges in `cover::RANGE`). Its **living cover** is its body
matter with the liquid of living tissue (matter / (1 − `WET_TISSUE`), `Cover::living`);
its walls are a frame, which pays no upkeep and does not wear, so its capacity, capture area,
upkeep, wear and fixing go by its living cover, as the maintenance breathing of wood goes by its living cells (Ryan
1990, from memory). The cover of a cell is a node of the
store (`Class::Life`, the life of its cell) whose law books its own side; it is kept with the cell's bodies (`life::Cell::cover`), so the
chunk of the cell is its one writer. **It stands on the ground above its line** (`cover::ground`, `cover::footing`): a cell's ground is all of it, its land
(`contract::land`) and its ground under the standing liquid, found by height from the terrain's samples inside it and
the level of each of its parts at the month's start (`flow::Shore`, `contract::levels`, from `flow::shores`). Each slot
holds the cell's ground above its line (`Cover::line`, m over the cell's lowest ground): plants do not move, so the
line holds where they stand while the level moves, and a rising level stands them in deeper liquid. Its roots work on
its land as the land's liquid lets them, and on its ground under the liquid, its pores full, as far as their gas
spaces reach down them past the depth standing over it (**Roots in full pores**, below; its loose ground taken as its
land's): one rule for where a plant's roots work, so a kind of few gas spaces holds land and the barest edge of the
liquid and one of many stands out into it as far as its gas reaches down its roots, as swamp trees and emergent herbs
stand in water to a depth their kind tolerates (Kozlowski 1997). A slot founded by seeds takes, at its first month, the
line of all the ground its kind may hold (`Shore::found`: the highest level of its parts that hold liquid less its
reach; where no ground lies above a slot's line, founded so or worn below it, the slot holds the cell's highest
ground, `Shore::holding`, and lives there on its store where all of it lies deeper than its reach; `cover::UNSET` till then). The share of the cell's ground above its line is the slot's (`Cover::stands`), and
of it the share whose loose ground holds its roots (`Cover::rooted`): their product (`Cover::on`) is the ground its
plants hold, taken as nested over the slots as the loose depths are (`canopy::rings`; the ground under the liquid is
the cell's low ground). Its leaves layer over that ground (its light and its shade), its plants find their room on it
(the self-thinning law, each slot against its own ground, of another slot's plants the share on it the lesser ground
over that slot's), and it is placed per m² of that ground. Under the liquid its activity is full (its pores hold all
they can), its roots work as far as that depth lets them, and they reach the liquid of the parts under it
(`Soak::pond`). How wet its land is to it is the land part's own liquid over that part's area for a cell of parts (no
more than the cell holds; `flow::parts` keeps each part's matter), else the cell's over its area. **Its plants under
the liquid past their roots' reach live on their store** (`upkeep::airless`): their roots get no oxidising gas, so
they gain nothing (their roots do not work) and pay their share of the slot's upkeep from their store at the yield of
their fuel without that gas (`units::FERMENT_YIELD`: 2 ATP a glucose fermented against about 32 burnt, so 16 times
the fuel; what they ferment is booked burnt, its products burnt about them); the share of it they burnt adds up month
by month (`Cover::drained`; plants that came to stand there since the last month come
with all of theirs), and when it reaches all of it they die as whole plants of every group alike, each plant left
keeping its mass, to its remains (counted as `DROWNED`), and the line rises to where none stands so. The burning keeps
its pace's fall with the cold, so a flood in the cold season takes far longer to kill them than one in the warm
(Kozlowski 1997; Glenz and others 2006); a flood that leaves before their store is spent kills none. **Ground below
its line is taken by seedlings only**: its grown plants never spread over it. Seeds lying wake on the ground their
kind may hold at the least root need; those that join a stand of their kind land evenly over it, and those that land
below its line where their roots get the gas take the ground they hold as dense as the self-thinning law lets them
stand (`stand::plant_ground`, no more than lies there), and the line falls to hold it (`soak::lowered`). In a cell of
two parts or more the one line holds over all of them. Seeds lying on ground under the liquid past their reach wait,
worn and frozen by their laws, and wake only where their kind can stand. Its temperature is
its ground's: at the end of each month it trades heat with its
ground to that temperature. **One month** (`cover::step`, its own process after the air's step at a month's start, one
task per chunk), slot by slot, the tallest first. **Plants**: a slot holds one pool of matter, and its plants in up
to `GROUPS` (4) **size groups** (`cover/groups.rs`), each a real count (`Cover::plants`) and a share of the slot's
built cover (`Cover::share`), the smallest plants first: a group's plant's mass is its share of the built cover over
its count. The slot's walls are shared over its groups by each group's built cover times the share of walls a plant
of its mass builds (`wall_kg`, below), each at most all of its plant: what a full group cannot hold goes to the
others by the same weights, so the groups hold all the slot's walls (a slot that has shed nearly all its living cover
stands with its large plants bare, its leaves on its small ones); the rest of each group's built cover is its living
cover and its share of the capture area (`Cover::sizes`). A plant's height is that of a plant of its mass whose walls make a stem of `stem_form`
and `stem_solid` at the walls' density, as tall as `tall` × its width^(2/3), the elastic buckling limit (McMahon 1973).
The slot's **large plants** (its last group) set its height (`Cover::height`): the top of its crown for the light, the
loose ground its roots need and the stem a jaw meets (the page draws every group, each its own size); its conducting walls are each group's
capture area as tall as its plants (`Cover::lift`). Seeds that found a slot are one group of one plant per `seed_kg`;
seeds that join a stand of their kind come in as a group of their own, and where all its groups are held they join
its group of the smallest plants (`Cover::recruit`): nothing happens to a plant because smaller plants arrived. Slots
that join pool their groups: groups of plants of one mass are one, and while they are more than `GROUPS` the two
groups of smaller plants (all but the largest) nearest in mass fuse, as cohorts fuse in the ecosystem demography model
(Moorcroft, Hurtt and Pacala 2001).
So seedlings that join a stand leave its large plants as they were. What a slot builds in a month goes to its groups
by each group's capture area times the light its leaves get, its leaves under those of its larger plants: through the
slot's layer the light falls by e^(the capture area it passes, `canopy::shade`), and a group's leaves get the mean
over their own share (`Cover::growth`): the large plants take the most light a leaf, the asymmetry of the competition
for light by size (Weiner 1990), and seedlings, all leaf, grow fast where light reaches them and slowly in the shade;
groups that pass each other keep the order of their plants. A stand holds at most (`thin_kg` / kg per m²)² plants a
m², the self-thinning law (Yoda and others 1963), a law of its plants' mean mass and their count: a plant of the mean
mass m holds (m / `thin_kg`)^(2/3) m² of ground in a full stand. The law is the crowns of a stand filling its space,
so plants crowd the plants whose crowns share their layer of the air (`Cover::crown`: from its top down the depth its
capture area fills at `crown_density`), whatever slot holds them, and the layers of a stand thin apart (an understory
under a closed canopy): after its seeds wake and after it builds (`stand::thin`), the ground the plants of a slot's
layer hold, each other slot's counted by the share of the slot's crown its own crown shares, is set against the
cell's ground, and where it is more the slot loses plants, not mass, as its whole layer would (by that excess cubed),
**the small first** (`stand::lose`; the suppressed plants of a crowded stand die first, Yoda and others 1963; Weiner
1990), the survivors taking what the dead freed: its smallest group loses plants until its survivors are as large as
the next group's plants, the two then fuse, and what is left falls on the large plants, but they lose no more than
the crowd of the large plants of the slots that share their layer asks, each at their own mass: smaller plants take
no room from them (the size-asymmetric competition of Weiner 1990; Purves and others 2008). So seedlings that crowd a
closed stand die in its month, and live where it has room. So a stand cut into slots thins as the whole. What dies of unpaid upkeep, wear or frost is whole plants of every group
alike (each count falls with the mass); what a taker bites or the slot sheds is mass of every group alike. So a stand
that recruits sits on the thinning line, and its large plants grow with their own mass: a kind that holds much mass
without leaf (walls, which pay no upkeep) stands tall, and a stand of few seeds is of few large plants until its own
seeds fill it:
- **Light**: the light the cell caught since its last month per m² of capture area (`World::light`), on its capture
  area (10^`LEAF` m² per kg of living cover) less what it reflects (its parts' colours by volume), passing down
  through the crowns of its kinds (Beer's law, `canopy::shade`): each kind's capture area spread evenly through its
  crown (`Cover::crown`), the light at each height absorbed by the capture area of all the crowns there, whatever
  slot holds them, so the tallest take it first, crowns that share a layer share its light, and a seedling lives on
  what reaches below the crowns above it; under standing liquid times e^(−depth / `light_depth`). **Roots**: a
  kind's plants root in `root_per_height` of their height of loose ground, at least `root_least` (rooting depth grows
  with a plant's size; Schenk and Jackson 2002), and hold a point of the cell where its loose ground reaches that need
  (none below `rooted_from` of it, all from it, smoothly between: bare bed anchors nothing and holds no liquid in
  pores). Each cell keeps the share of its area in each of 24 bins of loose depth, from its terrain's samples, found
  when the world is built (`World::loose_bins`, `cover::loose_bins`), and a kind holds the share s of the cell their
  mean gives (`Cover::rooted`, found again only when its need has moved by more than a bin). It stands on that share
  alone, its capture area x / s per unit of it; the shares are nested, as ground that holds deep roots holds shallow
  ones, so the cell's ground falls into rings (`canopy::rings`), each held by the kinds of a share at least its own, and
  the light passes down through the crowns that stand on each ring (with s = 1 everywhere, Beer's law through the
  canopy). So tall kinds compete on a cell's deep ground only, low kinds on its thin ground too, and on bare bed
  nothing but the smallest stands.
- **Light reaction** (reaction 0): the share `light_yield` of the light caught is stored, at most its capacity
  10^`LIGHT` × 10^`MOIST` mol per kg of living cover per s, times the reaction's pace k e^(−Ea/T) over its value at
  `t_cat`, times its activity a = min(1, w / 10^`MOIST`), a straight line. **Drought** is felt against the ground's own
  pores (`cover::wet`, `Pores::available`): w is the share of what its ground holds for roots that it holds, (θ −
  wilt) / (field − wilt) within 0 and 1 (its cell's field capacity and wilting point, `World::pores`, 4), θ the share of the pores of the ground it stands on that its key liquid
  that is liquid fills (Flow's pore space, `Scratch::land_pores`; standing liquid counts, so θ may pass 1): none at the
  wilting point, all at field capacity (the available water of soil physics; Saxton and Rawls 2006). The ground's
  liquid is one store over the depth of its loose ground (Flow keeps no liquid by depth), so a deep root and a shallow
  one draw the same share. `MOIST` is where on that scale a kind is fully active: a plant transpires freely until its
  roots have drawn about half of what is available (FAO-56's depletion fraction 0.5; Allen and others 1998), `MOIST`
  log10 0.5 = −0.3, and from there its transpiration falls in a straight line with what is left, to none at the wilting
  point (FAO-56's water stress coefficient Ks; a leaf's pores close as its ground dries, its tissue keeps its liquid);
  a kind of dry ground, `MOIST` −1, stays active to a tenth, and pays for it in capacity (its capacity per kg is
  10^`MOIST` times its light gene's). Frozen ground holds no liquid that is liquid: w = 0. **Roots in full pores**
  (`cover::soak`, `cover::working`): up to field capacity the ground's liquid lies in its small pores all through it
  and its large pores hold the gas, so every root breathes; what it holds over field capacity fills the rest of its
  pores from the bottom up (Flow's one store over the depth of its loose ground), and over its top it stands; its
  saturated share's pores are full (`inside::airless`, 0), so its table lies that much higher. A root
  under that table gets the oxidising gas its burning needs only down its own gas spaces, from the air over the ground
  or over the liquid standing on it, whose depth that gas crosses first: a kind's `GAS` gene is the share φ of its
  roots' volume that is gas space, and its gas reaches (2 D C φ / ((1 − φ) M))^½ m down a root (`cover::aired`;
  Armstrong 1979's longest root), D the gas's diffusivity in the air (`units::GAS_DIFFUSIVITY`, another gas's by
  Graham's law), C its concentration in the air at the ground (`World::near_ground` over R T), M what a m³ of living
  root burns of it a second (fine roots breathe `c_basal` a kg a day at the burning's pace, at `rho_body` kg a m³, over
  the energy the burning frees a mol of that gas; `cover::breath`). Of its roots, as deep as it needs and no deeper
  than its loose ground, the share above the table and within that reach below it works (Feddes, Kowalik and Zaradny
  1978: none at saturation for a plant with no gas spaces), and only those draw the liquid its pores lose: the light
  reaction it runs and what one more kg would gain are times that share, as a flooded plant's pores close as in a
  drought (Kozlowski 1997); its activity, by which it wears and hardens, is its drought's alone. So wetter ground is
  not always better: a kind of few gas spaces (φ 0.05, the 2 to 7% of plants of drained ground; Justin and Armstrong
  1987) loses most of its roots where its pores fill, one of many (0.3, of the 20 to 55% of plants of wet ground)
  keeps them, and a cold month, whose slow burning lets the gas reach far, costs less than a warm one (flooding in the
  dormant season harms trees far less; Kozlowski 1997). A seed wakes as far as its first roots (`root_least`) work. Both the light
  and the capacity limit go with the air's share x of the reaction's gas as (x − `gas_floor`) / (x + 2 `gas_floor`)
  over its value at `gas_ref` (`Physics::gas_factor`; the light-limited rate of Farquhar, von Caemmerer and Berry
  1980): nothing at or below the floor, so the cover cannot strip the air of that gas. **Liquid for gas**: its pores that
  let the gas in let the key liquid's vapour out; per mol of gas taken in it loses r mol of the liquid, r = (D_v / D_g)
  (p_sat − p_v) / (`drawdown` p_g) (Cowan and Farquhar 1977; Farquhar and Sharkey 1982; `cover::thirst`): p_sat the
  liquid's saturation at the leaf (`Chemistry::p_sat` at its ground's temperature), p_v and p_g the partial pressures
  of the vapour and the gas in the air near its ground, as the ground's own evaporation reads them
  (`World::near_ground`, 1.5), D_v / D_g the ratio of their diffusivities, the root of the ratio of their molar
  masses (Graham's law). The cover does not move that vapour itself: it opens its pores as wide as the liquid its roots
  reach pays for at that rate, and leaves the film they make, D_v / D_g × T / (`drawdown` p_g) per mol a s of gas
  taken in (`cover::pores`), with the share of the ground's liquid its roots reach (`Cover::film`, `Cover::reach`), for
  the ground's evaporation to pass the liquid through day by day, inside the surface's energy balance (1.5). Each slot draws the liquid from
  the pores its roots reach: the liquid of each ring of its ground is the slots' that stand on it, and where they would
  draw more there each draws it in proportion to what it would (`grow::drawn`); where the ground cannot give it the
  pores close and all slots store less together. All kinds of a cell are cut
  together to the liquid share of its ground's key liquid and the carbon gas of its share of its air node; its
  burnings and fixing take their gas from what that share still offers, so the cover never takes more of the air's
  gas than its cell's share holds (`grow::Books::room`). A cover that works on drier ground works more slowly: its capacity goes with its
  moisture need. The light its aside node set aside for it (`Cover::keep`, 1.4) pays first; what it stores beyond
  that comes from its ground, which that light warmed.
- **Upkeep**, paid by burning its fuel (reaction 1) with the air's oxidising gas: per kg of living cover and day
  `c_basal` plus `c_capacity` × what its capacity could store in a day (dearer by 1 + (melting point of the key liquid −
  `FROST`) / `c_firm` for a cover that stands the cold), plus `c_leaf` per m² of capture area; times its repair factor
  (`Physics::repair`) and the burning's pace at T, in every month: its living cover keeps its liquid whatever its ground
  holds, so it breathes in a dry month too, and only its gain follows the liquid it can draw (Flexas and others 2006;
  Atkin and Macherel 2009); a dry seed's upkeep falls with its liquid, a leaf's does not; and its
  conducting walls, alive in every season, `c_sapwood` per kg a day at the burning's pace: by the pipe model their
  cross-section goes with the capture area they feed, `huber` m² a m², as tall as the plants of each group
  (`Cover::lift`; Shinozaki and others 1964), at most its walls (`stand::sapwood`), so a tall plant pays for its height every month; and its fine roots,
  as many kg of living cover as the key liquid it drew in the month needs at `root_uptake` × (1 − φ) mol a kg a s (the
  minerals come in that liquid by mass flow; Barber 1995; `upkeep::uptake`): the layer that keeps the oxidising gas in a
  root's gas spaces also keeps the liquid and its minerals out (a barrier to radial loss; Colmer 2003; Garthwaite and
  others 2008), taken to tighten as the gas spaces grow, a choice of form with no number of its own, so gas spaces cost
  more fine roots on any ground (a slot whose fine roots would cost all a leaf gains keeps its pores shut); which breathe `c_basal` a kg a day at the burning's pace and are
  built anew `root_turnover` times a year at what a kg of living cover costs to build (`Parts::living_cost`; Gill and
  Jackson 2000). The share unpaid
  dies (`SPENT`). **Wear**: the share 1 − e^−wear dies (`AGE`), wear its upkeep at an activity of at least `dormant`
  over (living cover × `e_life` × 10^`REPAIR`), by the rate of living (Rubner 1908). **Frost**: its `FROST` gene is the limit it
  reaches hardened, when dormant; growing tissue loses that hardiness and freezes at its key liquid's melting point,
  the limit at activity a the melting point less (melting point − `FROST`) × (1 − a) (`cover::frozen`); below it the
  share min(1, (limit − T) / `frost_k`) dies (`PHASE`). Hardiness is the kind's own and paid for in every month
  (`c_firm`, above), not given by dormancy (Levitt 1980; Sakai and Larcher 1987).
- **Budget**: what the cover's parts and its seeds lying hold as fuel (`Parts::content`: the fuel what its burning frees, matter and walls
  the fuel their conversions take) changes only by the flows its law books where it moves them
  (`Counters::cover_budget`, `cover::BUDGET`): the light reaction's fuel, each part of the upkeep, the work of
  building, what it sheds, what dies, the fuel its roots give its ground, seeds out and landed, the seeds lying that
  die, what takers take or break, what is placed, rounding, and what flames burn or kill (3c). The seeds' flows in kg (landed, sprouted, worn, frozen,
  taken: `Counters::seeds`) and what each cell's cover built in the last whole year (`Counters::grown_last`) are counted
  for the tape. The tape records them each year.
- **Growth**: the fuel over its reserve (10^`STORE` kg per kg of built cover) and the minerals of its ground (shared
  out over the slots, `grow::shares`: where the ground holds what they would take, `build::wants`, each gets its share
  of all of it by that; where it is short, by their fine roots (as many as the liquid each drew needs, the minerals
  coming in that liquid), none more than it would take, what that leaves shared again by roots: roots compete below
  ground in proportion to their size, Casper and Jackson 1997, so a flush of seedlings takes a seedling's share of a
  scarce mineral, not a share by how fast it could grow; whatever slots hold the kinds) build body matter and walls (10^`WALLS` kg per kg of matter, times m / (m +
  `wall_kg`) for a plant of m kg: a seedling is mostly leaf and the share of stem grows with a plant's size, Poorter
  and others 2012; of each group of its plants by its share of what the slot builds, `Cover::walled_new`) by the conversions of growth (`Chemistry::build`,
  `builds`), paying `c_copy` per kg by burning fuel. A fixer (`FIX` above `FIX_OFF`) makes the mineral its growth lacks by the fixing reaction (10), at
  most 10^`FIX` mol per kg of living cover per s at the reaction's pace, paid by the reaction's fuel. Then it builds
  walls alone of the fuel past its reserve, up to the walls its plants hold for their mass (each group's by its share,
  `Cover::walled_held`; `grow::frame`): walls are
  built of the fuel alone and pay no upkeep, so a plant short of minerals, or whose leaves are all its light pays for
  (it is **full** and builds no living cover then), still grows its frame and with it its height, and its stand thins
  out. In season (below), the fuel past its reserve that it could not build of goes to its ground, as roots exude the sugars a plant cannot use (Jones, Hodge and Kuzyakov
  2004), where the open world's rot takes it.
- **Seeds**: the share `SEED` of what it builds leaves in `seed::LOTS` (8) equal lots, each landing where the dispersal
  kernel draws it (`life::lot`: a lot starts at a point drawn evenly over its cell and moves by an offset drawn from a
  kernel of two scales, a near and a far one, each an isotropic Gaussian, `kernel_near` and `kernel_far`: most within
  tens of metres, a long tail to km, Clark et al. 1999); the lots of a cell make a packet each (`seed::send`; what lands
  in its own cell or off the map stays there), with one generation's mutation of its genes (noise × each gene's step ×
  `mut_size`), of body matter first and then walls (an embryo and its store); where its lots land and how they vary are
  drawn by the cell and the slot's genes, rounded to 1/1024 of each gene's step (`rng::Purpose::Cover`, `KEY_STEPS`),
  not by its place among the cell's slots; all of it where one more kg would not pay and its plants hold their walls (short of them it builds walls alone, above): what a kg more would store with the light at the bottom of
  its layer (`canopy::shade`) is less than a kg's upkeep, so the slot is **full** (`Cover::full`) and stops thickening
  where its lowest leaves would not pay for themselves (Monsi and Saeki 1953; Hirose 2005). By the same marginal rule
  run backwards a full slot also sheds the layers of its canopy that do not pay (`season::shed`): through its crown its
  light falls as e^-z with the capture area z above a leaf, whatever slots' leaves it passes, so what a kg more would
  store at depth z is its crown base's times e^(y − z), y the capture area through its crown (`Slot::layer`); its
  living cover below the depth where that falls to its upkeep a kg goes to its remains, its walls and plants stay. **Shedding for a season** is the same rule over the months a leaf waits to gain
  again (Kikuzawa 1991). A leaf does not come alone: what it gains needs the fine roots that draw its liquid, whose
  cost is a share of the gain set by the month's air and its kind's gas spaces (`upkeep::root_share`: the liquid a
  unit of the light reaction draws, with what its pores lose for its gas, over what a kg of its fine roots carries,
  `upkeep::uptake`, at the roots' breathing and renewal); from all of it its pores would lose more than they bring,
  and they stay shut that month: that slot draws nothing and gains nothing (Cowan and Farquhar 1977, from memory). A slot is **in season** while its top leaves gain, less their fine
  roots, what a kg of leaf and the walls that feed it cost to keep for the month (`upkeep::keep_per_kg`: its upkeep,
  its capture area's and its conducting walls'), its top leaves those at the top of its crown, whatever slots share
  that crown (`season::pay`), so a stand cut into slots keeps its season as the whole; a layer that does not pay so in season lies in its own canopy's shade
  and pays no more, so it goes as above (the slot `full`), once it has paid its month's upkeep. Out of season none of its leaves pay until the season comes back, after the
  months its kinds expect without gain: the share of the last year's months in which its cell's ground held nothing
  for roots, frozen or dry to the wilting point (`Cover::idle`, a running mean over a year's months, kept by the
  cell's cover whether it holds slots or not, so a place's seasons outlive its plants), times the months of a year.
  In the month its gain stops, before that month's upkeep is taken, it keeps the layers whose loss over those months,
  `keep_per_kg` less their own gain, is less than building them anew (`Parts::living_cost`); in a month of no gain its
  layers are alike, so it keeps all its leaves or drops all of them. Of those it would keep it keeps no more than its
  fuel can keep through those months (this one at least; a leaf kept needs no building anew): what it cannot pay for
  it drops as leaves and stands leafless with its store, rather than die of it as whole plants. With the costs of
  chemistry 0 the tree kind drops its leaves for a season of four warm months (295 K) without liquid and keeps them through a frozen season of
  any length, where the burning's pace is low. What it drops is its living cover (its leaves and the living cells that
  serve them); its walls stand, its plants stay, and out of season it builds nothing and gives none of its fuel to its
  ground, so its fuel is its store. A slot with no leaves pays no upkeep: its walls pay none, its conducting walls and
  fine roots go with its leaves and the liquid they draw. **Known limit:** a resting stem and crown breathe a little
  (their living cells), so a real leafless plant spends its store slowly through its season; here it spends none. What one more kg of leaf would gain is known with no leaves
  too (its capture area per kg at the light that reaches its height): once that pays it is in season again and
  builds, of its whole store while it holds no leaves, and matter alone in that month, on the walls it stands on. So
  a tree stands bare through a long dry season and leafs out from its store; a herb keeps its stems and its store and
  grows its leaves again, or, where its store is spent, comes back from its seeds lying. Each seed carries its own
  store, fuel at its `STORE` gene per kg of it, from the slot's fuel: a slot short of fuel makes fewer seeds, not
  lighter ones. When every cell has had its month the seeds land (`cover::land`), cell by cell in the order made (the
  potential energy of the life that left a cell's head and came to it is booked once for each cell and way,
  `store::life_sent`) and **lie on its ground** (`cover::Lying`, `lying.rs`) in up to `SLOTS` (12) lots at any variety, each its
  amounts of the three parts, its genes and its starting kind, at the cover's temperature (their heat is the cover's,
  and they are matter of the cover's node: in its books, its budget and its potential energy). Seeds that land join the
  most alike lot of their kind within `APART` × `alike` steps (the genes the mean by built matter), else an empty lot,
  else the two nearest lots join and they take the lot freed: the rule of the slots (`life::kin`). A seed is a small dormant
  plant, so the cover's laws hold for it: each month, before its cover's own, of each lot the share its wear gives dies
  (the wear of dormant cover: its upkeep at full activity a kg, `upkeep_rates`, at the floor `dormant`, over `e_life` ×
  10^`REPAIR`) and the share frozen below its hardened limit (`cover::frozen` at activity 0), to the ground as remains
  (`seeds_died` of the budget); of the rest the share min(1, a × pace) **wakes**, a a seed's activity on that ground
  by the seeds' rule, min(1, w / 10^`MOIST`)^`dormancy` (Harrington's: dry tissue, `Physics::activity`),
  and pace its burning's pace at the ground's temperature over that at `t_cat`: at `t_cat` all that is active wakes in a
  month (seeds of temperate plants sprout in days to weeks once warm and wet; Bewley 1997, from memory), on frozen or dry
  ground none. What wakes takes hold (`seed::lodge`): it joins the stand of its starting kind in the cell, the slots
  of it within `APART` (0.5) × `alike` steps (in every gene, `cover::distance`), shared over them by their built cover
  (the genes of each the mean by built cover; in each a group of plants of their own, or the group of its smallest
  plants where all are held, `Cover::recruit`);
  else they found a slot of their own: an empty one; else the slot **in deficit**, whose balance over about its last
  year (`Cover::net`: what its light reaction stored less its upkeep, per kg, a running mean over a year's months) is
  below zero, the worst first (its cover dies as remains); else the two slots nearest each other by their genes,
  of any starting kind, join (`seed::room`: the slots are a limit of the bookkeeping, and the two hold the same niche
  to within what it resolves) and they found the slot it frees (a world of one kind a cell, `Config::variety` 1, has
  no two to join: the seeds stay lying). So a cell of more slots is never closed to seeds and a holder that pays its
  way is never pushed out by weight: what lives after is the light's to decide, as seedlings live on their store and on
  the light that reaches the bottom of the layers above them, and what does not pay dies (the gap models, Shugart
  1984). A variant farther than `APART` × `alike` from its kind founds a slot of its own and competes with the others
  for light by height. Slots of a cell within the same distance share one (`merge_alike`). A slot is the bookkeeping of a kind in a cell, not
  a thing of the world: every law above acts on the stand, so a cell whose slot is cut into alike slots stands at the
  end of that month as the uncut cell does, when the pieces merge back, in every month of its year, its dry season too
  (`core/tests/stand.rs`). Selection is between slots
  and cells: what grows better sends more seeds (frost selects the hardy where it kills the tender, `core/tests/cover.rs`).
- **What it takes and gives**: liquids and minerals from and to its ground with their heat; gases from the air (with
  their heat at the air's temperature and their latent heat) and to its cell's aside node (with theirs), as living things
  breathe; its dead (remains) to its ground as those substances, where the open world's rot runs (reactions 1, 7,
  12). Every move is booked through the cell's view (`store::Moves`), its gas within the cell's share of the air.
- **Taken and broken**: only through the contract (section 0): a taker takes from the whole cover, from one slot or
  from the seeds lying (`contract::take`, `Layer::Seeds`), each slot or lot giving the same share of all its parts; work
  pressed on a cell breaks the cover by its share (`contract::press`).
- **What it is to the rest**: its mass, capture area and the light it absorbs and keeps are what stands on its cell
  (`Cover::stand`, `contract::Stand`), so it shades (1.4) and holds the ground (1.3).

## 3c. Flames in the remains on the ground
`core/src/flame/`. Once a month, after the cover's month and before its seeds land (`life::month`), the remains of life
lying on a land cell (its ground's body matter and walls, `contract::remains`) may burn. What burns into what, and the
heat, come from the chemistry's own burning reactions (`Chemistry::burner` of each part); the speed comes from the heat
balance of the bed, not from the reactions' rates, which are those of rot (real surface fires are limited by heating
the next fuel; Rothermel 1972). Three stages, each number with one writer:
- **The cured leaves**: a slot whose plants stand no taller than `surface_m` keeps the leaves it sheds standing on its
  plants, cured (`Cover::cured`, part of the cover's node; `season::shed_to`), and so do the body matter and walls of
  its plants that die (`upkeep`; their fuel goes to the ground), as the dead leaves and stems of herbs stand through a
  dry season; they fall to the ground's remains with a mean life of `cured_life` (`grow::fall`, booked
  as shed). Taller plants drop theirs at once.
- **The bed** (`flame::bed`, each land cell its own, in parallel): the load w of its remains, kg a m² of land, and of
  the cured leaves standing over them; the share of its ground they cover, 1 − e^(−a w), a the capture area its leaves held per kg dry (its cover's `LEAF` gene over the dry share
  of living tissue, its slots' mean by built cover; none without cover). **The layer** (`flame::bed`, `Layer`): vapour diffuses through
  it, so in a steady state its relative vapour h runs in a straight line from its top, the air near the ground
  (`World::near_ground`, its vapour over saturation at the ground's temperature), to its bottom, between that and
  saturation by the share s of what its ground holds for roots that it holds (`cover::soak`, `Pores::available`, the
  share by which the ground's evaporation draws its liquid; Manabe 1969) on the drier part of its land: the share of its
  land saturated at the month's start (`inside::saturated_share`, 0) is too wet to carry a flame, so the share of its ground the remains cover
  is times one less that share, and s is read from what the rest holds; the key liquid frozen in its pores counted
  with it; where snow lies (`contract::Place::snow`) it hides the remains and they do not burn.
  At each depth it holds the sorption curve
  `sorb_scale` (−ln(1 − h))^`sorb_power` per kg dry (Nelson 2000; h at most 0.99), and as duff, drawing liquid up from
  the ground, `duff_hold` × s × the share of its depth over that. A burning kg frees h_c (its parts' burning heats by mass); lighting a kg takes Q = c (`t_light` −
  T) + M (c_l (T_boil − T) + L_v, + L_f below the liquid's melting point), all from the chemistry's substances, M what the curve holds halfway down its part that
  flames. The part that **flames** is its top down to where what the curve holds reaches the moisture of extinction (`flux_ahead` h_c − c
  (`t_light` − T)) / (c_l (T_boil − T) + L_v), where the share `flux_ahead` of h_c still lights the next kg (0.18 for
  the remains of chemistry 0 at 295 K): fine dead fuel dries with the air; the cured leaves, in the air near the
  ground, flame all where its top does, as a loose bed at `cured_rho`; the part that **smoulders** after the flame
  goes on down to where all its liquid, the duff's with it, reaches `smoulder_moist`, the most liquid at which organic
  matter smoulders (dry peat burns through, the bottom of a layer on moist ground does not). It **carries a
  flame** where its remains cover more than the site-percolation threshold of the square lattice
  (`flame::CARRY_SHARE`, 0.5927) and the load that flames is at least the **least load** σ (`t_burn`⁴ − T⁴) τ / (h_c −
  Q), τ the flaming time of its particles (below): a bed thinner than that frees less heat while it flames than its top
  sends to the open sky at the flaming bed's temperature, so its flame cools and goes out (0.15 kg/m² of dry remains at
  295 K; real grass fires stop under 0.1 to 0.2).
- **Strikes and runs** (`flame::shares`, serial, over the cells that carry a flame, in the order of the cells): cells
  that share an edge join into a region of A m² of covered ground (each cell's land times its cover). A region expects
  `strikes` × A × the month's share of a year of strikes that light it; their count is a Poisson draw and each lights
  a cell drawn by its covered ground, from the world's seed by the month and the region's first cell
  (`rng::Purpose::Flame`; the same draws for any number of threads). Each flame runs from the cell it lit over the
  region's cells for `front_life`, crossing half the way between two cells' centres at each one's speed R (below),
  the earliest first (`flame::run`). Every cell it reaches burns all of its land that its remains cover (f = its
  cover): a patch burns whole, keeps no remains to carry a flame, and burns again only once its cover's leaves have
  built them back. So the land burnt is a mosaic of patches of different ages, from that feedback alone (no rule keeps
  a cell from burning). The wind stretches a run downwind: along the way between two cells a bed's speed is R (1 + φ_w),
  φ_w Rothermel's (1972) wind factor of the wind's speed along that way at mid-flame: the low wind (`World::low_wind`,
  the air's at its last step) times Albini and Baughman's (1979) share of it that reaches the flame (`flame::sheltered`,
  Andrews 2012): over open ground by the depth of what flames (about 0.3 for a cured bed), under a canopy by its height
  and how its crowns fill the space under it (0.1 to 0.2 under closed tall crowns), by the share of the ground that the
  slots taller than `surface_m` cover, 1 − e^(−their capture area a m²); the shade itself reaches the bed through the
  ground's temperature and the air near the ground, which its layer reads (`World::near_ground`, `thaw_last`), the particles' surface to volume ratio 4 / `fine_d` and the packing of what flames (`flame::downwind`); none
  against the wind.
- **The burn** (`flame::burn`, each cell its own, one task per chunk, only where f > 0): on f the cured leaves and the
  part of its remains that flames burn, and after them the part that smoulders; a flame through what flames, its depth
  the cured leaves at `cured_rho` and the remains at `litter_rho`, moves at R = `flux_ahead` h_c δ / (τ Q), δ its depth, τ = 8 minutes an inch of
  `fine_d` its particles' flaming time (Anderson 1969); its intensity is I = h_c w R (Byram 1959), its flame
  0.0775 I^0.46 m long (Byram 1959), and its plume kills leaves up to 0.1483 I^(2/3) m (Van Wagner 1973), I in kW a m.
  Of each slot on f (`flame::crown_burns`): each of its size groups (`Cover::sizes`) whose plants stand no taller than
  the scorch height dies above the ground on f (`Cover::die_under`): a group of plants no taller than `surface_m` keeps
  its plants, its store and its part below the ground (`base_share` of its built cover) and sprouts again from it, as
  burnt herbs and shrubs do, what stood above its base burning by its burning reactions; a group of taller plants loses the share f of its plants and of its built cover, their fuel by their share of
  the slot's built cover to the ground and their walls standing dead (`Cover::snag`, part of the cover's node), which
  fall to the ground with a mean life of `snag_life` (`grow::fall`, booked as died). Of a slot taller than `surface_m`
  the body matter of its crown (`contract::crown`, spread evenly over it) of the plants that live burns where it stands
  in the flame, and dies to the ground between the flame and the scorch height: a young tree must outgrow the flame between fires
  (Bond and Midgley 2001). Seeds lying survive. Gases go to the cell's
  aside node and so to the air, what is not a gas (the liquid of the reactions, the nitrogen mineral of body matter, as
  ash) to its ground, where the soil keeps it on its exchange sites or the liquid washes it away (4). The mineral the
  burning gives first breaks down as the chemistry breaks it down hot (`flame::break_down`: each reaction that takes it
  alone, reaction 11 in chemistry 0, runs over the share 1 − e^(−k τ) of it, k its rate constant at `t_burn`): at 1,100
  K all of it, so the fixed element of what burns goes to the air as its gas (Raison 1979: from 500 °C nearly all of
  it is lost); through the cell's view as the cover's books do (`life::book`, the oxidising gas within the cell's
  share of its air, `Books::room`); the heat the burning frees goes to the air node over the cell. What it burnt and
  killed is counted (`Counters::flame`, `flame_air`, `cover_lost` by `life::FLAME`, the budget's `burnt`), and what the
  flames did on each cell in the month (`flame::Flamed`: the share burnt, the flame's length, the minerals left and
  the gas given, per m² of land), the share burnt in each month of the last year and the months since it last
  burnt, for the tape (`Counters::flame_month`, `burnt_months`, `burnt_age`).
- **Known limits:** dead leaves keep all their nitrogen (body matter's share of it is fixed by chemistry 0, and a
  plant has no part that could hold the mineral it would pull back; real plants pull back about half, Aerts 1996), so
  a flame sends 2 to 5 times the real nitrogen to the air (about 20 to 30 g a m² burnt); the slow organic matter held on the mineral particles (14) does not burn; the smouldering part
  burns in the month with no depth or speed of its own; the liquid the remains hold is not boiled off into the air
  (the heat balance counts it); the plants of chemistry 0 hold no element that stays solid but the fixed one, so
  the ash is the fixed element's mineral that the flame did not break down; the scorch height has
  no term for the air's temperature or wind; no crown fire; no bark; every year is the same, so return times come
  from the fuel's regrowth and the strikes alone.

## 3b. Life: the takers, as a field

Each ground cell holds up to 8 slots of takers (`fauna::Fauna`, `life::Cell::fauna`): a slot is one kind of takers
in its cell, in two groups, its adults and its young; each group's **members** a real count (whole below `N_WHOLE` =
64) and the mol of the fuel they store (5) and of their body matter (10, life's `Parts`); the slot's heat, J (each slot
at its own temperature), its balance over about a year (`Fauna::net`), and 8 genes (`fauna::MASS`, `CHILD`,
`YOUNG`, `ENZYME`, `REACH`, `KEEP`, `COAT`, `REPAIR`). A member's **living mass** is its share of the body matter with
the liquid of living tissue (matter / (1 − `WET_TISSUE`), as the cover's); that liquid is not in the books. The takers
of a cell are part of the node of its life (`Class::Life`): the chunk of the cell is their one writer. They reach the
land only through the contract (section 0; `cli/tests/sides.rs`), and their books are the life's (`life::Books`,
`life::book`). They take from the cover and the seeds lying, digest, and live on what they took and their store. They live on the cell's
land, as its cover does (`contract::land`): they search its land, the remains they rest in lie on it (`contract::remains`,
kg a m² of land), and their mass per area is per km² of land (`fauna::per_km2`); on a cell whose land goes under they
find nothing to take and die there by the law of upkeep, their matter to the cell's ground, and young that land on a
cell with no land die there.

**One month** (`fauna::step`), in the same pass of a cell's life, right after its cover's month, group by group (a
young member is a body of its own mass, with its own band, floor, upkeep and heat):
- **Body and pace.** A member stands at its `KEEP` gene where its ground (`contract::place`) is colder, else at its
  ground's temperature; its upkeep runs at the pace of its burning there, the rate law's at its temperature over that
  at `t_cat` (Gillooly et al. 2001): 5.6 at 310 K, so a body that keeps no heat of its own pays far less where it is
  cold. Every rate runs at the pace of its body's temperature over the pace at the temperature its number was given
  at: its upkeep, its gut's capacity and its growth (Kleiber's numbers) over the pace at `t_cat`; its work (walking,
  chewing) over the pace at `WORK_T` (310 K, the body temperature of the mammals the rows of work were measured on), at
  most 1. So a body at 288 K walks and chews at 0.6 of a warm body's speed and pays a sixth of its upkeep, and a cold
  body's speed and power fall with its temperature as measured (Q10 about 2 to 3 below 25 °C in lizards, Bennett 1980
  and 1990; the rate law's is 2.3 near 300 K).
- **Upkeep at rest**: Kleiber's law, (`c_basal` + `c_muscle`) × m^`upkeep_exp` × pace × its repair factor
  (`Physics::repair`) a day per member, m its living mass, kg (Kleiber 1932; Elia 1992); and its gut's capacity,
  `c_capacity` of the fuel its capacity (`ENZYME`, mol of walls a kg living a s at `t_cat`) could free, at the same
  pace and repair, as the cover pays for its light capacity.
- **What it reaches** (`fauna::take`): each slot of the cover as two layers by the height its plants stand at, its
  soft part and its frame, and the seeds lying (`contract::slots`, `contract::shown`). The soft part of a slot is its
  living matter and its fuel (the leaves that capture light, and the living cells of its sapwood); the frame is its
  walls (its stems and branches, and the walls of its leaves). **A mouth breaks what its jaw can rupture**
  (`contract::cut`): a jaw closes with `jaw` × m^`jaw_exp` N and cuts an axis of width d where that force is the frame's
  stress for a cut (`Substance::shear`) over the solid wall of its section (`stem_solid` of π d² / 4); a stand whose
  branches keep their section from stem to twigs (Shinozaki and others 1964), their lengths as the radius to 2/3 (West,
  Brown and Enquist 1999), holds the share (d / D)^`thin_exp` of its frame in stems thinner than d, D its main stem's
  width (`Cover::width`, the stand's law), at most all. So a 30 g body cuts stems of 0.8 mm, a 400 kg one of 6 mm, and a
  tree's trunk is nobody's. A member of length L = (m / `rho_body`)^(1/3) searches the band of heights
  ± `BAND` (half a decade) about its `REACH` gene, no higher than `reach_len` L, and bites no lower than `bite_floor` L
  over the snow: a slot's soft part, and the thin stems of its frame that carry its leaves, stand in its crown, from h −
  L / `crown_density` to its height h, L its capture area over the ground its plants hold (its land times the share its
  roots hold), no lower than the ground (`contract::crown`): a grown tree's leaves stand high, a sward's crown is its
  whole height; of a crown it reaches the share within that, the matter spread evenly over it (`contract::reached`); the seeds lying
  only where its floor and its band's bottom are under `GROUND_M` (1 cm). What lies out of reach is the cover's refuge.
- **The take** (Holling 1959, the disk law over many layers and slots): a member searches a strip as wide as its body
  at its walking speed v = √(`walk_froude` g L) (Alexander and Jayes 1983), bite by bite: a bite clips the cover on
  the area of its mouth, A = (`bite_breadth` m^`bite_exp`)² (the incisor arcade's breadth, Illius and Gordon 1987;
  bite area as its square, Gordon, Illius and Milne 1996), and takes `bite_time` to clip whatever it holds (Shipley et
  al. 1994); the walk to the next bite and the clipping add, so the ground it clips a s is a = 1 / (1 / (v L) +
  `bite_time` / A) m²/s (it walks only between bites, at a / L m/s), for `busy` of the day (T),
  and breaks what it takes by its chewing power P = `chew` m^`upkeep_exp`: a layer of D kg per m² of which it reaches
  w, each kg taking b J to break (`Shown::breaking`), gives it I = T a w D / (1 + a Σ w D b / P) kg a day. Over the
  month in `SUBSTEPS` (6) steps each layer falls by S (1 − e^(−k Δ)), k = Σ n I / S, so it is never overdrawn and the
  slots share it by their rates, not by an order; the rates are those of the stocks at each step's middle (a half step
  at the start's rates), so a slot switches to the next layer as one runs out. Then one bite of each layer for all
  (`contract::take`: all its parts in their proportions), shared by the slots' parts.
- **Heat.** A member held above its ground loses g (T_b − t) through its coat, g = `meeh` m^(2/3) / (`r_air` + COAT ×
  `r_coat`) W/K (Meeh 1879; Scholander et al. 1950). It makes the heat of its rest and of its walk while searching
  (v × its searching time at `c_move` × (m / `move_ref`)^`move_exp` J per kg and m, Taylor et al. 1982); where that is
  less than its body loses, it burns the rest to keep its temperature; where more, the excess must leave as vapour of
  the key liquid, at its heat of vaporisation. A body at its ground's temperature gives all its heat to its ground and
  needs no vapour.
- **A body at rest sits in what lies on the ground** (`fauna::shell`). For its working time (`busy` of the day) it is
  in the open; for the rest it gathers around itself the remains lying on its ground (the dead body matter and walls
  of its ground node, `contract::remains`, at `litter_rho`): its cell's remains shared by all the members of the cell's
  slots alike (each sits in its own heap), no more than lie on the ground it searches in a day. That mass is a shell
  around the sphere of the body's surface (its Meeh area, radius r = √(A / 4π)), out to R, R³ = r³ + 3 V / 4π; heat
  crosses it as through a spherical shell, (1/r − 1/R) / (4π `litter_k`) K/W, in series with its coat, at the ground's
  temperature outside. A shell without end passes 4π k r: a 30 g body loses 1.17 W in the open at 35 K and 0.50 W in a
  kg of remains, as a nest about halves a small mammal's loss; a 400 kg body's open loss is already under the heat its
  rest makes, so it gains nothing. In its shell a body loses no less than it makes (it sheds its shell before it must
  sweat in it). The remains it sits in stay where they are; the soft solid lying is not part of the shell (Flow keeps
  no density or conductivity of the lying solid).
- **Thirst** (`fauna::thirst`). A body drinks open liquid only: the liquid standing on its cell, what ran over it in
  the month, and the soft solid lying there, which costs its heat of fusion (`contract::liquid`); and it has the liquid
  of what it eats (`WET_TISSUE` of the living mass of the body matter it takes). Its need is the vapour that cools it at
  its work plus the least liquid it passes through, `drink_least` × m^`drink_exp` kg a day (Nagy and Peterson 1988),
  which goes back to the cell in the month and is not booked. The slots of a cell share its open liquid by what they
  ask of it. **The way to the liquid** (`fauna::way`): a slot still short walks to the nearest cell whose open liquid
  over the month holds its shortfall (`contract::to_liquid`) and back, as often as its shortfall and what a member can lose
  as liquid between drinks (`dry_most` of its mass) ask, at its walking speed and the law of walking's work, which it
  burns; what the day's hours cannot walk stays short. Where a slot's liquid is still less than its need its work is
  cut first (searching, handling and building together) down to where its heat to shed needs no more liquid than it
  has: less work makes less heat. Only where at no work its need is still unmet do its members die, by the unmet share
  (`LACK`). The vapour shed leaves the cell's liquid (the running liquid, what they ate and what they fetched join it
  there), then the soft solid, for the cell's aside node with its latent heat, paid from the bodies' heat
  (`contract::vapour`); where the cell's ground holds less, less is shed and its heat goes to the ground. What was
  fetched is the far cell's: it waits in the cell's outbox and, after every chunk ran, is taken from that cell's ground
  (as far as its liquid then holds, the asks on it shared by what they ask) and given to the walkers' cell's ground with
  its heat, carried in their bodies: its potential energy is booked as for life that moves between cells
  (`fauna::land`, `contract::fetched`).
- **Digestion** (`fauna::gut`, by the chemistry's own reactions): its gut splits walls into fuel (the reaction that
  makes fuel of the walls alone, taking the key liquid of the ground) up to its capacity × its living mass × pace; it
  burns for its upkeep and heat first the fuel it took and split, then the body matter it took (the burning of body
  matter, which gives its mineral to the ground at once), then its store, each no more than the gas the cell's share of
  its air node offers (`Books::room`); the fuel it took and did not burn goes to its store; the walls it did not split
  pass to the ground; the body matter it did not burn is for growth.
- **Deaths**, each cause a share of the members on what the one before left (as the cover's): the share of the energy
  unpaid (`SPENT`); 1 − e^-wear (`AGE`), wear the energy a member burns per kg over `e_life` × 10^REPAIR (Rubner 1908);
  the frozen share at its body's temperature (`Physics::frozen`, the key liquid's melting point lowered by `harden` ×
  (1 − min(1, pace)): `PHASE`); and `LACK`. Below `N_WHOLE` members the dead are a binomial draw (keyed by the seed,
  the month, the cell and the slot, `Purpose::Fauna`); a count that falls below it becomes whole by a draw that keeps
  its mean, never rounded up as a rule. The dead's share of the body matter and the store goes to the ground as
  remains.
- **Growth and births** (`fauna::grow`), after the deaths, slot by slot. The share `YOUNG` of the fuel each group's
  store gained in the month, and what each store holds beyond `store_most` of its group's living mass, are the slot's
  pot. It pays the work of building, `c_copy` a kg of body matter, by burning; the body matter built is the matter its
  groups took and did not burn (so growth needs matter in the food). A member builds at most Kleiber's rest
  ((`c_basal` + `c_muscle`) m^`upkeep_exp` at the pace of its body's temperature) over the energy a kg of living tissue
  holds and costs to build (`Parts::living_cost`, 6.8e6 J a kg): the cap of growth of West, Brown and Enquist 2001
  (dm/dt at most B0 m^(3/4) / E_m, their E_m the energy content of tissue, 7e6 J a kg, their B0 Kleiber's), with the
  temperature as Gillooly et al. 2002 find it. First the young grow, each at its cap; the share that crosses the adult's
  mass in the month joins the adults with its share of the young's amounts (von Foerster's flux across a mass, the
  young's masses taken as even in log mass between a newborn's, `CHILD` of the adult's, and the adult's: the share
  min(1, Δln m / ln(M / m0)), Δln m the young's growth in the month). Then the adults bear newborns, one parent each, a
  newborn holding `CHILD` of the adult's mass. An unborn grows from nothing by the same law, dm/dt = a m^(3/4)
  (1 − u), u = (m / M)^(1/4), a her cap over M^(3/4), fed from her pot: it reaches the newborn's u_c = `CHILD`^(1/4)
  after 4 M^(1/4) (−ln(1 − u_c)) / a (26 days for the 30 g kind, 258 for the 400 kg kind at 310 K). It burns
  a E_m m^(3/4) (E_m the energy a kg of living tissue holds and costs, `Parts::living_cost`), of which the share 1 − u
  becomes body, and its mother pays all of it: over the time of bearing 4 E_m ∫₀^u_c u³ / (1 − u) du / u_c⁴ a kg of
  newborn (`grow::unborn`: 1.4 to 1.7 times E_m for the starting kinds). The pot pays that less E_m, its upkeep before
  birth, and the work of building, `c_copy` a kg of body matter; the body matter itself is what she took, so nothing is
  paid twice. Its burning is greatest at its birth, so her cap feeds at once a litter of 1 / u_c³; litter after litter
  she bears the share u_c / (4 (−ln(1 − u_c))) of her cap (`grow::bearing`): a quarter for a small newborn, 0.18 to
  0.20 for the starting kinds. Below `N_WHOLE` the young
  that mature are a binomial draw and the newborns the whole part of what was built and one more by a draw on the rest;
  what was built stays (a draw never makes matter, it moves the mean mass of the young a little), and a group of no
  members gives its amounts to the slot's other group (matter built into newborns none of whom was drawn stays in the
  mother). What is left of the pot goes back to the stores up to their most; the rest of it and of the matter passes to
  the ground. The draws of a slot in a month are one keyed stream (`Purpose::Fauna`), in the order: deaths of its
  adults, of its young, its maturing, its births, the dispersal of its maturing young, the mutation of its newborns.
  A slot of no members is empty.
- **The young leave** (`fauna::land`). The young that mature in a month leave in lots (one a member under `N_WHOLE`,
  else 8), each landing in the cell the kernel of the seeds draws (`life::lot`: two Gaussian scales, `kernel_near` and
  `kernel_far`); the lots that land in another cell leave the slot's adults for the cell's outbox with their heat at
  the body's temperature. After every chunk ran they land, cell by cell in the order of the cells and of what each
  sent: on land they join the adults of the slot of their kind they are alike to there, or found one, by the slot rule
  below (a slot in deficit they take dies as remains), their count whole under `N_WHOLE` by a keyed draw; on a cell
  with no land, or with no room, they die there, their matter and heat to its ground. The potential energy of what
  moved is booked as for the seeds (`contract::travelled`).
- **Mutation and the slot rule** (`fauna::lodge`, the shared rule of `life::kin`). The newborns of a slot in a month
  are one packet; its genes are the slot's after one generation's mutation (each gene moves by noise of its step in
  `fauna::RANGE` times `mut_size`, within its range: `life::kin::mutate`, as the cover's seeds). It joins the slot of its
  starting kind within `APART` × `alike` steps of it in every gene (the genes the mean by living mass); else it founds
  a slot of its own: an empty one, else the one most in deficit (`Fauna::net`, the slot's balance over about a year:
  what it digested less its upkeep per kg living a month, a running mean; not its own slot nor one whose newborns wait
  to lodge), whose members die as remains (counted as unpaid); else it stays in its parent slot with no new genes.
  Slots of a cell alike in every gene join (the lighter into the heavier, the genes and balance the mean by living
  mass). They never join because all slots are held (unlike the cover's): a mean of two unlike kinds is good at
  neither. One generation moves a gene by about `mut_size` (0.063) of its step and the join is 2 steps wide, so a
  packet almost never founds a slot by its own mutation: the rule selects between slots that exist.
- **The books.** Gases from and to the cell's share of its air node, remains to its ground, each with its heat, through
  the books of the cell's life; each slot's heat ends at its heat capacity times its body's temperature, and the rest of
  what the burning released goes to the ground.
- **What it is to the rest**: its mass lies at its cell's head (`flow::stack`, the potential energy); it stands on its
  cell as nothing (`contract::Stand` is the cover's: takers neither shade nor hold the ground).

## 4. Soil and minerals
**Texture** (`soil`): the loose ground of each of the terrain's samples is fine grain (under 2 µm), middle grain
(2 to 50 µm) and coarse grain (50 µm to 2 mm), each class spread evenly over the logarithm of its sizes; and organic matter, of each cell (below). Three laws, over what the wear at the world's start left (0), set the shares
(`soil::samples`); no table places a kind of soil:
- **The bed's grains.** Each make breaks into coarse grain by its substances' hardness and into middle grain by the
  rest, means by their volume shares (`Make::coarse`, `worldgen::makes_of`): a hard grain keeps its size as it is
  broken and carried, a soft one wears finer.
- **Weathering.** The share of a make whose substances a reaction of the open world takes with a gas of the starting
  air into another solid (`Make::weathers`; in chemistry 0 the dark mineral 8, by reaction 6) turns its middle and coarse
  grain fine as 1 − exp(−t / 1e5 years) (`WEATHER_YEARS`; the lifetime of a silicate grain in the field, White and
  Brantley 2003), t the time the loose ground has lain: its depth over how fast it is renewed, from below as fast as
  the ground lowers or from above as fast as it settled in the wear's last stage (`Samples::age`; at least the
  lowering of `SLOWEST`); where the high stand laid a layer over it (0), the mean of the two layers' ages by their
  depths. A substance that weathers only into liquid or gas dissolves away and leaves no fine grain.
  So thin ground on steep slopes, quickly renewed, keeps its coarse grain; old flat ground on a weathering bed is fine.
- **Sorting.** What settled on a sample in the wear's last stage (`Samples::settled`; all of the open low ground's) is
  sorted by the flow that laid it: grains settle where they sink faster than the flow's shear velocity u* = √(g d S)
  stirs them (Bagnold 1966), a grain of diameter D sinking at R g D² / (18 ν + (0.75 R g D³)^½) (Ferguson and Church
  2004; R its make's grains' density over the liquid's less one, `Make::buoyant`; ν the liquid's viscosity,
  `units::LIQUID_VISCOSITY`). The flow is a flood of 1e-6 m/s of runoff from the area that drains through the sample
  (`FLOOD_RUNOFF`), as deep as Flow's channels carry it (`channel_depth`), down the sample's slope to the sample its
  liquid runs to; under standing liquid (a pond, the open low ground) it is still. What settles is the load's grain
  between the least size that sinks there and the least that the flow at its main sample upstream carried on (all
  sizes where none runs to it), a span at least twice as wide (`SORTING`); where that span holds none of the load,
  its class nearest the span. So channels and fans are coarse, the floors of ponds and the sea fine (Hjulström 1935).
  The settled share of the loose ground holds that grain, the rest the weathered bed's.

A cell's texture (`World::soil`, found when the world is built) is its samples' mean weighted by the loose ground each
gives its ground node (at most `skin_m` of it; by area where it holds none).

**Organic matter** (`soil::month`, at each month's start): a cell's organic share is the share of the volume of its
ground node's solids that stay solid at the surface (not its frozen liquid) that is of the substances life's parts are
made of or that its particles hold (`Substance::bound`, the slow organic matter 14). **The slow pool**: a fifth of the
body matter that rots becomes the slow organic matter (reaction 14), which rots 25 times slower (15), slower again by
the share `SHELTERED` (0.75) × its fine and middle particles' share (`Texture::shelters`: the texture slows the rot of
the organic matter held on mineral faces, Parton et al. 1987; Six et al. 2002): 25 years on coarse ground, a century
on ground of fine and middle particles alone, at 290 K moist. So it builds over decades to centuries, most where the
ground is fine, and holds its nitrogen from the plants until it rots. The fresh dead matter is the dead matter its cover and its takers leave on its ground (3, 3b), which rots there by the open world's
reactions (1, 7, 12; 2.1), faster where the ground is warm and moist and slower under standing liquid, where the
oxidising gas comes only through the liquid's film (2.3). No store of its own: it is what the ground holds. So ground
under a dense cover grows richer in organic matter and holds more liquid, and bare ground loses it: the soil and its
cover shape each other. Where the share moves by more than a twentieth of itself (`ORGANIC_STEP`, a speed tolerance), the cell's pores and its node's state are found again (`flow::new_pores`): the liquid they
hold or let stand rises or falls with them, and its potential energy goes to or from the ground's heat.

**Pores** (`soil::Pores`, `World::pores`): each cell's porosity θs, field capacity θ33 (the liquid held at −33 kPa)
and wilting point θ1500 (−1500 kPa), and its conductivity when full Ks. Its mineral part's follow from its texture by
the regressions of Saxton and Rawls (2006) on the coarse share S and the fine share C with no organic matter, held
inside the range they are fitted to (C 0.05 to 0.6, S 0.05 to 0.95): θ1500 and θ33 each a fitted sum of S, C, S·C and
one, then a fitted second step on it; θs = θ33 + the fitted share between field capacity and full − 0.097 S + 0.043;
Ks = 1930 (θs − θ33)^(3 − λ) mm an hour, λ = ln(θ33 / θ1500) / ln(1500 / 33). Its organic part is decomposed organic
ground: 0.85 of pores, 0.6 of its volume held at field capacity, 0.3 at the wilting point, 1e-6 m/s (`ORGANIC`). The
two mix by the organic share o: θs, θ33 and θ1500 by volume, Ks as (1 − o) ln Ks + o ln Ks of each (any share
works). Flow keeps field capacity and the wilting point as shares of the pores (θ33 / θs, θ1500 / θs). A loam (0.4
coarse, 0.2 fine) has 0.39 of pores, 0.64 of them at field capacity, 0.31 at the wilting point, and passes 2.2e-6
m/s; a coarse ground passes up to 3e-5 m/s and holds a quarter of a loam's liquid for roots, a fine one under 1e-8
m/s. So coarse ground takes the rain in fast and holds little of it; fine ground sheds what falls faster than it
passes, and holds more, much of it past the roots' reach (1.2, 1.5, 3).

The minerals (`Chemistry::minerals`) live in the ground's liquid and move with it, so rain washes them to low ground
and the sea and fertility differs by place. **Exchange sites** (`soil::exchange`): the charged faces of a cell's fine
particles and its organic matter hold a share of its dissolved minerals in a linear balance with what stays
dissolved: sites of 1,060 mol of charge a m³ of fine particles and 2,800 a m³ of organic matter (`SITES_FINE`,
`SITES_ORGANIC`), each holding the minerals of 0.04 m³ of the liquid (`HELD_PER_SITE`), so the sites hold those of
`Soil::holds` m³ (found each month's start, `soil::month`) and the share V / (V + holds) of a mineral stays dissolved
in the V m³ of the cell's other liquids (`soil::dissolved`), found when Flow builds the state of its ground in full
(each month's start) and kept until the next (`State::dissolved`). Only that share is liquid (`Sp::liquid`): it moves
with the liquid that sinks, flows through the pores and runs off (`flow::liquid::mobile`, `basin::movable`) and takes
its liquid volume in the pores and over them. The rest is part of the ground everywhere Flow reads it: a solid at the
solid's volume in the solid surface, in no pores and no cover, that neither moves nor fills (`land::layers`,
`stack::ground_layers`, `land::rise`); roots take from all of it (the sites give back to the liquid as fast as roots
take from it). So coarse ground lets the rain wash its
minerals away and fine and organic ground keeps them: a loam's 2 m of loose ground holds some 97% of its minerals
on its sites at field capacity, a coarse ground some 90%, so the coarse loses them three to four times as fast. **Nitrogen mineral (12)**: placed when life starts (`worldgen::life::soil`)
per m² of land, on every open land cell; then only fixing adds it, rotting (7) returns it, heat (11) sends it
to the air. The **rock salt (11)**, made in wet cells from lime (8) and turned back by 9, near balance (2.5), is the
second mineral of growth: body matter holds one of its rock element (1.2% of its mass), rotting (7, 14) gives it back,
and a flame leaves it as lime (9 at `t_burn`, laws.md 3c), which weathers back (8) where the ground is wet.

## 5. Starting conditions (not laws)
Placed by `worldgen` (`worldgen::schedule::Schedule`, carried by a snapshot) and printed by every run; the laws never
read a kind.
- **Terrain**: a function in core (`space::terrain`, 0), which `worldgen::terrain` builds the cells on: an island of
  drawn rises or of one rise (`--shape`, 0), or a globe with 29% of its area above 0 m (`LAND_SHARE`, the level it finds kept in
  `World::terrain`); the bed of the chemistry's `makes`, each over the share of the cell's samples whose surface the
  wear left in it (`space::strata`); the loose ground the wear left (0: the mantle on land, the shelf on the open low ground), each cell the
  mean over its samples of the top `skin_m` of each sample's at most: the ground node is the layer that exchanges with
  the surface within a year (its heat, and the pores roots and evaporation draw from), and what the wear left deeper lies
  under it, packed with the bed (the bed's store, `Class::Deep`, its aquifer); the sea's salt at
  balance (2.5); ground and air temperatures falling with latitude and height. The terrain's samples are worn (0):
  land rises from a low stand of the sea 100 m under 0 m to the noise's height over 2 million years while flowing
  liquid cuts it, its slopes creep and it floats back up as it is unloaded; the load settles in the closed basins the
  cutting leaves, and those it does not fill are ponds, each with its pass and level; what reaches the open low ground
  lies there as a shelf; then the sea rises to 0 m and drowns what lies under it, and stands there 6,000 years while the
  channels fill the heads of the valleys it drowned (the high stand, 0). **The starting
  liquid** (`World::full_bodies`): the open low ground stands at 0 m and every body of standing liquid with a pass
  stands full to its pass, each part of it holding what its samples hold under that level, as the runoff's pass leaves
  a body (1.2); a cell that holds no pond's samples holds what its ground dips under 0 m. Each cell that holds some of
  it has all its pores full (pores first, 1.2) and starts molten, a body's liquid as the sea's, (2 K over the liquid's melting point, `MOLTEN_MARGIN`) at balance
  with the air (2.5). So a body starts where a body whose gains beat its losses ends; one that loses more than it gains
  falls to its own level by the laws (1.2, 1.5). On an island the open water is coarse (`coarse_low`): an air block (8 km) whose squares all lie
  below 0 m, none within a square of land (`LAND_RING`), is one cell, split where it borders finer cells until cells
  across a border differ by one size at most; the coasts stay in 2 km squares. A globe is one cell an air block but in a
  rectangle of air blocks in squares (`--fine`), by the same rule. A cell's height is the terrain's mean over it
  (`Terrain::inside`, on the worn samples, 0), whatever its size.
- **Open land**: a cell that holds land (`flow::land_shares`: the share of its area out of the liquid that stands over
  its pores, as the page draws the cover on it; the land part of a cell of parts among them, also where the cell's
  mean ground lies under the level). Its soil and its takers start per m² of its land; each kind of cover starts per m²
  of the ground it stands on (`cover::place`, `cover::footing`), out into the liquid as far as its roots work.
- **Life** at year 2 (`--life-year`): the cover (`worldgen::life::KINDS`: kg per m² of land and genes, on every open land cell
  by its chance, each kind's draws keyed by its own key, its label its row, `Cover::root`); without `--roster` the herb
  (`ONE`), with it the six kinds in their order (`ROSTER`): a small fast herb, a herb, a big slow cover with walls, the same of wet ground (fully active only near saturation, more capacity), a woody
  cover of dry ground (active at a tenth of saturation, thick leaves, slow) and a tall fast cover with seeds spread
  wide. Its genes get a spray of mutation (`--spray`).
- **Fixers**: a tenth of the cells of the starting cover carry the fixing gene (`FIXERS`, `FIX_GENE`; `--fixers`).
- **Takers**, only where a run asks for them (`--fauna-year`; none by default, and none on the page): the five kinds of `worldgen::life::TAKER_KINDS`
  (30 g, 3 kg, 60 kg and 400 kg kinds that keep 310 to 311 K, and a 2 kg kind that keeps none) on every open land cell,
  all adults, at a third of Damuth's line of members per area of land against body mass (`START_SHARE`; Damuth 1981), whole
  below `N_WHOLE` by a keyed draw, each with a store of a tenth of its living mass as fuel (`START_STORE`), at its
  body's temperature, booked from the outside node (`fauna::place`, `contract::placed`).
