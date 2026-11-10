# The laws as they are now

What each law does, its rule, its physical reason and where it lives. How the code does it (rounds, caches, buffers,
orderings) is said once, in the code's doc comments; the code is the truth. Numbers, units and anchors are in
`docs/numbers.md`, what a run records in `docs/tape.md`.

**Where the code is** (`core/src/`): time in `clock/` and `schedule.rs`; the shape of the world in `space/`; how a
cell's means lie over its own ground in `inside/`; shared
maths in `solve/` (`sweep`, `line`); nodes and books in `store/`, `world.rs`, `check.rs`; ground heat in
`ground.rs`; settings in `params.rs`, constants in `units.rs`; bodies in `body/`, `life.rs`, `nodes.rs`, `lineage.rs`,
`brain.rs`; the cells as bodies see them in `survey.rs`; paths in `lanes.rs`; keyed random numbers in `rng.rs`; the
tape's events in `event.rs`; threads in `par.rs` (the same results with and without, on any count of threads).

**Words.** A **cell** is a square of the map (2 km on the island, latitude-longitude on a globe) or a block of 2 x 2
or 4 x 4 of them. A **node** is anything that holds stuff: a cell's ground, an air column, a body, a part. A
**body** is a living node with genes and maybe a brain; a **member** is one living thing in it (one member: an
**individual**; many: a **population node**). A **gene** is a number a body inherits, often log10 (1 means ×10) and
"off" below 0. The **key liquid** is the liquid a body's chemistry runs in (substance 0). **Light-eaters** (plant-like)
are not bodies but the **cover** of each cell, a field (4.5); bodies are **eaters**.

## 0. The world, one step, the books
**The map** (`space/`, `World::mesh`): cells, links with widths and headings, and a coarser air level. The laws step
only on links. Each link takes one of 8 compass directions (`Level::way`), so the brain's 8 directions hold everywhere.
An island of 2 km squares, or a globe (`--globe`) of latitude-longitude squares on a sphere of 6,371 km whose rows wrap
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
the middle of its map, says how high the ground would stand; the world keeps what it needs beside its seed and its map
(`World::terrain`: the rise its level 0 m lies at). The
noise's finest octave is 670 m on the 64 x 64 island and 200 km on a globe. The ground is held at **samples**, the
leaves of `SAMPLE_DEPTH` (1/64 of a block: 125 m on the island, 20 km on a globe of 64 rows), and **the samples are
worn** (`space::samples`, `space::wear`, found once from the seed when first read, not saved): the ground has a
history. The noise is read at the centres of the samples of a grid of 2 x 2 samples to one (`NOISE_BY`,
`wear::noise_grid`) and carried smooth to finer grids. The open low ground (on a globe every sample under 0 m; on an
island those under 0 m joined to its lowest, so ground under 0 m inland is a closed basin) stays as the noise has it; land rises from 0 m at a steady rate to the noise's height
over 2 million years (`WEAR_YEARS`; ground under 0 m does not rise) while flowing liquid cuts it by the stream power
law (Howard 1994; Whipple and Tucker 1999): it lowers by K √A S, A the area that drains through a sample, spread down
every lower way by Quinn's weights (Quinn et al. 1991), S the slope to the sample its liquid runs to, m = 1/2 and n = 1,
K = 3e-7 a year (`CUT`), each step solved exactly from the open low ground up the ways (h = (h₀ + f h_down) / (1 + f),
f = K dt √A / d; Braun and Willett 2013), so a step of any length is stable, 0 m the height of the low ground's
liquid. Under a closed basin's level the liquid does not cut. What a step cuts spreads down as the liquid does; a
closed basin keeps all that comes into it and fills its floor from the lowest up, as one level, toward the height its
pass has now, and what it cannot hold runs on over the pass (`space::ponds`); what reaches the open low ground leaves
the land. The slopes creep as a diffusion of 3e-3 m² a year (`CREEP`, the anchor of the core's own creep; Culling
1960), and the ground floats back up by 2700/3300 of what a step took away, spread over 50 km as a plate bends (Airy
isostasy, `FLOATS`, `BENDS_M`; Molnar and England 1990). The land first gets a roughness of a metre drawn from the
seed (`ROUGH_M`), which the first ways find. The time runs on three grids (`STAGES`): 55% of it in 24 steps on a grid
of 4 x 4 samples to one, then 20% in 6 steps on one of 2 x 2, then 25% in 3 steps on the samples' own, so the
valleys are cut at the samples' size; each grid takes the noise's mean over each of its samples (or the noise carried
to it), and each is carried to the next by the smooth lean below (held between
the lowest and the highest of a sample and its eight neighbours, so it makes no new pit and no new peak); a map whose
first grid would hold more than 65,536 samples runs every stage on grids twice as coarse until it fits
(`MOST_SAMPLES`). It runs in a fixed serial order in 64-bit numbers, so a seed gives one ground on every machine.
**The flood** (`space::drain`) over the worn samples, from all the open low ground, does not raise the ground (Barnes,
Lehman and Mulla 2014): it reaches the samples by the level their liquid stands to, lowest first (a sample's own
height where it drains; under a closed basin's level the height of its lowest pass), then the lowest on the flood's
surface, then the least index. Each sample's liquid runs to the neighbour reached before it that the flood's surface
falls to most steeply (D8, O'Callaghan and Mark 1984): the ground where it drains, under a basin's level that level
raised by 1e-6 per m along the way the flood came (`FLOOD_RISE`), so inside a basin the way runs to its pass
(Planchon and Darboux 2001; Cordonnier, Bovy and Braun 2019). A closed basin whose deepest sample lies less than a
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
the samples goes on across their quarters. Nothing random is laid on it. So a leaf has a mean height at any depth. What needs the cell at a
point (the tape's frames, a byte a square; the pictures) asks the tree for the leaf that holds it (`Tree::cell_at`).

**One step** (`schedule::step`, a month; time in whole seconds, the calendar from it) is a fixed list of passes
(`schedule::month`), each for the whole month: the survey (`survey::survey`: one task per chunk on the pool with the
feature `threads`, in each of its cells the wear of movers, the shade and the sums; a death that lets go of a part
queues that for the serial pass, as in a batch; it rebuilds Flow's
state in full for the cells' surroundings, and Flow's own pass leaves it then); Flow: the air in steps of
`AIR_STEP_DAYS` (15 days, two a month, each at its own time in the month), after each the sink of the liquid over
field capacity into the bed's store; then the liquid and erosion; heat sideways and creep yearly; the cover's month;
Change; Bond (each chunk's cell by cell, the chunks on the pool: in a cell the turns of its parts in the order of its
list, on its view, then the loads of its structures in the order of their roots); then the bodies (`survey::batch`).
**The bodies' step** (`survey/steps.rs`): a move is one link a decision, and every cell steps a month (the pace the
numbers of bodies are anchored at). The random draws of the bodies are keyed by the day (`World::key_at`). **Every
body updates at the month's start**, after the month's Flow, Change and Bond: one batch holds every living body.
**An update: a decision, then the physics of the step ahead in sub-steps** (`survey::updates`). A body decides once
at each month's start (5.3), and then its own physics runs over the month ahead in sub-steps of one **physiological
step** (`World::phys_grid`: 7.5 days, a quarter of a month), four a month, each by the same laws (`body::live`:
drink, upkeep, heat, frost, growth, the brood and births, wear; 6), and before each but the first it eats by the take
law for the sub-step as far as its act leaves its members to eating (`choose::meal`, `Body::busy`; its decision's
meal fed the first, 5.3). The physiological step is a body's own
time: a meal (its stores' room holds what it burns until its next meal), the filling of its brood to a child and its
turn between its cold and warm state are days to weeks. Each sub-step integrates the time from where the body's
physics reached (`Body::last`; of a node, the mean by members, `nodes::rejoin`, so members who joined it since count
less) to its end, at least a day: upkeep and the brain's cost, drink,
heat, frost (6.5), growth and wear go by those days; the death draws of an individual are keyed by the sub-step's last
second (`World::key_ending`) and deaths and births are dated by its day (`Calendar::day_ending`).
After a batch every body's physics has reached the next month's start, in the surroundings of the month its physics
ran in. A body that lands in a cell in the serial pass
(a mover, a litter) runs its physics on from where it reached to that cell's next step in a pass after the landing
(`survey::batch_of`), as the bodies there did; one with no physics behind it (a founder) first decides by its spare
now (`body::newborn`). Its learning and its watch go by the time since its last decision (`Body::decided`, 5.4).
What looks ahead goes by the time to its next meal or step: what it takes by the take law, and the room its stores
keep, by the physiological step; what it gives, bites and does to parts by its cell's step (5.3); the fuel growth
keeps back for its upkeep to its next meal (6.6); the heat of a meal let out, 6.3; the look-ahead of a plan by the
cell's step (5.3). So a step is a time step, not a calendar: a season's change comes from temperature (6.2). The
bodies
read the ground and the air as the month's Change and last air step left them, and what they leave on the ground (the
free solid they shed, remains) goes through the next month's Change before the next batch can take it, so most of the
free solid has rotted by then (it rots in about a week, 2.1). Every body decides at every update (5.3).
**Bodies, chunks and the serial pass** (`life`, `survey::batch`). Each ground cell stores its bodies in the order they
came (`life::Cell`), each a record of fixed size (its stores and genes in place, 6.1, 4.1), and their brains, one block
each in the cell's array (`life::Brains`: the rows, the moving averages of learning and the last trace), reused when
freed and packed again once the freed blocks hold half the array (a snapshot keeps them packed, none free); a body that
moves takes its block to the array of the cell it enters. Each cell has a **serial** (`life::Cell::serial`), given
when the cell is made, that never changes and is never given again (the world keeps the next free one); its index may
change. A body's id is the serial of the cell it was placed in and that cell's count of bodies placed (children, split
halves, parts of nodes acting apart, `life::new_id`); it never changes, and its random draws are keyed by it. An id
finds its cell through one table from serial to index (`Life::at_serial`). The cells
are cut into `Config::chunks` runs of neighbouring indices, the same for any number of threads; a pass on some cells
(a sub-step's, the serial pass's) holds in each chunk only those of its run (`life::Of`). **The parallel pass**:
each chunk is one task on the pool (`life::run`), its cells taken out of the world, the world read only; in each cell
in order, first the physics to now of a body whose physics is behind; then every body with a
brain perceives its inputs and the bodies of each lineage find their hidden layers together (5.1); then each decides
and acts, the mature ones first and then the young, each in the order they came; then the sub-steps of the step
ahead, at each first the meal of every body not leaving the cell (but at the first, whose meal its decision gave),
then the physics of each, both in that order (`survey::mature_first`: the children a body makes join their young node
before its sub-step of the same time, 4.2; a child that starts a body of its own runs the sub-steps after its birth):
so at every sub-step the bodies of a cell all eat before any lives it, as at a decision; then the next cell. So what a chunk queues, and the lineages
and born rows it makes, come cell by cell whatever the chunks. The parts it reads are as they were at the pass's start. A body perceives the others of its own cell as they are when it
decides (5.2); it takes from or gives to a body it saw only if that body still lives and is not leaving its cell. **A cell's view of the store** (`store::Moves`): its ground, bed and
bodies' node as the store held them at the pass's start with its own moves since, and **its share of its air node**
(`Mesh::share`, its area over the air node's) of the air's amounts and heat: a cell never takes more than that, so the
cells under one air node never overdraw it together (the cover's month and the parts' turns read the air the same
way). Any other node it reads as the store held it at the pass's start. What an update does outside its cell is
queued (`life::Entry`): a move to another cell (`choose::go` pays it), children who start in another cell
(`copy::emit`), a watch, an act on a part (Bond's verbs; a body that lets go of its part, dies or joins another lets go
there too). A body with a move queued, or a node's part with a part act queued, is **leaving** (`Body::leaving`): it
stays in its cell until the serial pass, but it does not mature or split, and the others there do not deal with it.
The lineages and born rows a chunk makes have ids of the chunk until it ends, when they are named in the order made.
When the chunks end the world takes their events, counters and queues chunk by chunk, and books their moves (each
array of the store by one task, the changes cell by cell in the order of the cells, `store::book_by_array`), so every
sum runs in the order of the cells and the world is the same for any count of threads and of chunks. **The serial
pass** (`life::land`; after it a pass brings the physics of the bodies that landed to their cell's next step, and
its queue lands in turn, `survey::batch_of`): the queue in rounds: first the moves and the litters land, each chunk of the cells they go to on
the pool, in a chunk cell by cell and in a cell in the order queued, and what the moves did to the lanes and the
structures follows at once (`Entry::Passed`, one by one: a structure that moves spans two cells), so a part act finds a
carried structure where it went; then the part acts, each in the chunk of its cell on the pool (where its body is; a
body's letting go of a part, where the part is), in a chunk cell by cell and in a cell in the order queued: an act on
parts touches its cell alone, as a structure lies in one cell; then the watches and the litters that go back, one by
one, in the order queued, each a serial step (`World::in_cell`); what they queue is the next round's. Last the
holders of each lineage and born rows are counted from the bodies (`Life::recount`): a lineage left
with none dies out, born rows left with none are freed. Each entry is applied by its law on the bodies and parts as
they are then. A move lands (`choose::land_move`): the body enters the cell, carries what it holds if it paid to (else
it lets go of it), counts in the cell's crossings, and its push breaks the bodies there at the next survey; members of
a node who moved join their kind there (`copy::kin`) as an amount with their genes and rows, not as a body; then its push reaches the structures there, the structure it
carries comes along (any other body that held a part of it, left behind, lets go of it), and it clears the lane
(`Entry::Passed`). A litter joins its kind there or starts a body, and its
birth is recorded then (`copy::land_litter`). A watch blends by the share of the watcher's members who watched (5.5). A
part act runs whole on a part in the body's cell (`choose::land_act`); a node's part then comes back to its node if it
holds nothing, is inside nothing, and the node is in its cell and not leaving it. An entry whose body or target is
gone does nothing: a move of a body that died (its cost was paid), a watch whose watcher is no longer of its lineage,
a part act no longer possible or that cannot be paid; less than one member of a node's children with no node to join
goes back into the parent's brood, or to the ground of the parent's cell if it died (`copy::back`).

**Inside a cell** (`inside/`, handover 5.2): a cell holds means (its cover, the liquid in its soil, what stands on
it). The ground inside it is the terrain at a grid of 8 x 8 samples, the leaves `inside::DEPTHS` (3) below the cell
(`World::ground_in`: the terrain's mean over each), and what stands lies at one level over them (`flow::curve`, 1.2).
The shell draws inside the cells by general laws at each pixel of the ground's tiles (`tape::Ground`), blending its
cells' values between the squares' centres (`tape::Table`, docs/tape.md): the soil's liquid by the wetness index
ln(a / tan β) of the terrain's samples (Beven and Kirkby 1979; a the area that drains through a sample per metre of
contour, β its slope): as in TOPMODEL, the liquid a point's pores hold departs from its cell's mean by `soil_m` per unit
of the index over its level, within none and the pores full, the level found by halving so the mean is the cell's
(`inside::soil_level`, `soil_held`); what stands where the ground lies under the cells' level; each channel as wide as
the channel law makes its own sample's discharge (`World::discharge`, 1.2); each mark of cover by
the activity of each kind of the squares about it at the mark's foot (`cover::activity`). A picture need not keep a cell's mean
exactly (the core keeps the books).
**Sleep and wake** (`clock::Sleep`, a speed tool): a calm node sleeps until an alarm or until an amount grows by more
than `wake_share`, then its law brings it up to date over the whole time since.
**One ground temperature**: every law and the tape read that of Flow's last step: life (bodies, the cover, parts) its
land's (`World::t_ground_last`, of a cell of parts its land, on which no liquid stands, or its liquid's top where it has
no land, 1.1), Change and the
potential energy the node's (its heat over its heat capacity, `FlowView`), the
tape the surface's mean by area (`World::t_surface_last`).
**Checks before a run**: `Config::validate`, `Chemistry::validate` (every reaction keeps its elements and mass).

**The books.** Every move takes from one node exactly what it gives another, with its heat; `store::apply` and the
bookings of the bodies' moves and of Change's rows (`store::book_by_array`, which adds as `apply` adds) are the store's only writers, with plain sums.
Boundary nodes stand for the sun, space and outside (`store::Boundary`, edits by `World::cross`). **Potential
energy** is one function of what the nodes hold (`flow::potential`): a ground cell is a stack from its base up, its bed
and its solids each at its middle, its liquid with what is dissolved in it as Flow holds it (1.2, `stack::Column`): in
the pores of its loose solids first (of a cell of parts, less the room its land's pores have left, 1.1), a table rising from the bed's top, and the rest standing over the cell's own
ground (its level-volume curve) at its centroid; its store `bed_depth` under its bed's top, filled from the bottom; an
air node's matter lies at the mean head of its cells (their solid surface and liquid as a column, as Flow's state
stacks them), for the height of a column above its ground is in its heat (its heat capacity at constant pressure holds
the work the column does as it rises); a bodies' node's gas and the matter of a cell's life (its bodies, its cover
with the seeds it sends until they land, its parts) at its cell's head. The parts of a month's Flow after the survey
(state, the air's steps, liquid, creep, refresh) follow each other with nothing between (`flow::chain`) and are
booked as one at their end; the survey's own
build of Flow's state, and a part run alone, are booked at their own end. In each booking every move carries its
potential energy from the height it leaves (a ground node's liquid top: the table of its pores, or the level of what
stands where that lies higher, so liquid standing in a pit under its full pores leaves with them and the top never
rises as liquid leaves, `stack::Column::top`; a bodies' node's head; an air node's mean head; a store's table; each the mean of the booking's start and
end, so a lake that drains whole takes along what it held); and each node's heat takes what came in less what left
less the change of its own (`store::fall_heat`): so a fall heats where the matter lands (the runoff's in the body it
reaches, 1.2), a lift is paid by the node it enters (the air, for a gas going up; the ground node a store's liquid rises
into), a column that rises as it fills pays for it, and the potential energy the world's
matter gained is one number of the store (`Store::lifted`), the change of that function. Within a month's Flow the later
air steps and the liquid read the heat before the month's lifts and falls are paid. The other laws lift a gas into
the air from its cell's head to its air node's mean ground by the same rule (`World::lift`); a ground node that reacts
(Change) pays what its stack and the air over it gain (`flow::gain`: a head that rises lifts its air node's matter by
the cell's share of the node's area), and its gas leaves from its head (`change::booking`). **Life and its ground**:
life's matter counts with its ground node, as Flow books it. What life gives its ground or takes from it (remains, shed
matter, the gut's bulk, the cover's losses and what it takes up, parts that fall apart, edges' wear, drinking, grazing,
rock picked up) gains the potential energy that the function finds between the cell as it was and as it is, the matter
from the head into the stack or back with what that raises or lowers above it and the air over the cell
(`flow::gain`), and the ground node's heat pays it (`store::life_moved`, for the moves of a view of the store and those
`store::apply` books from a node of life): a fall heats the ground it lands in, a lift is paid by the ground it leaves.
Life that goes to another cell's head (a body that moves and the structure it carries, a litter, seeds) carries its
potential energy from the head it left, and the ground it lands on takes the difference (`store::life_went`). An edit
(`World::cross`) takes the potential energy of what it places from its boundary node. So every element and the energy
are kept; `check::drift` checks it yearly against `check::DRIFT` (1e-9), with the potential energy as the function of
what the nodes hold (`check::Totals::potential`), not as `Store::lifted` books it, so a
move that books no lift or fall shows.
**No overdraw.** A law never asks a node for more than it holds; where it could, it cuts that node's own outgoing
links (`solve::sweep::within`), never a whole field. A body takes no more than its cell's view holds, what the
bodies before it in the pass left (checked in debug builds). `apply` and `book_by_array` only add and set rounding below zero to zero, counted
(`Counters::rounded`). A body takes ground heat no further than `T_FLOOR` (`World::take_from_ground`).

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
year is far below the explicit limit on cells of kilometres (1.8).
**A cell's parts** (`flow::parts`): the ground of a cell with liquid standing over its pores is a short list of parts
under one air, each with its own heat and matter: its **land**, where its own ground lies above what stands (liquid
and soft solid, `Curve::wet` of it), the liquid's **top**, down to `h_mix` under the liquid's level, and its **deep**
below that (`Curve::volume` under that depth), which holds the ground under the liquid. A dry cell is one node; a cell
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
pores runs down to the top with its heat (`moves::settle`). The share under the liquid follows what stands on it: a
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
edge of the ground under the liquid (`Curve::shore`, found at the samples), each from its middle, half its area over
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
melting point (lowered by solutes, 2.5) until that substance has all frozen or melted. A node whose heat melts or
freezes all of it within a step stays there only for the share of the step that takes, then goes on from that point,
and a soft cover that melts away holds its surface at the melting point only until it is gone (`air::radiation`), so a
long step ends where short ones do. The state of a node is found in full at each month's start; between, it is solved again only on a
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
- **Pores first**: the loose ground's pores (`porosity`) fill first, under any soft cover; only liquid above them stands
  and has a head (`flow::land::stand`), the depth bodies and Change read.
- **Each cell's own ground** (`flow::curve`): what stands fills the cell's ground from its lowest point up, so its
  head is a level over that ground, and a cell is wet where part of its ground lies under it. The ground is the
  terrain's samples inside the cell (`World::ground_in`, 8 x 8), their departures from their mean laid on the
  cell's mean surface: the cell's level-volume curve, found once. A cell's liquid leaves it from its
  lowest ground, as the terrain inside it drains: a cell whose mean lies under its neighbours' is no basin where its
  ground drains (`flow::levels`, `flow::lowest` read it). The liquid's potential energy is where it lies: in the
  pores, and over the cell's own ground at its centroid (0, "The books").
- **Bodies of standing liquid** (`flow::bodies`, `flow::basin`): a body is a pond of the terrain's samples (0: the
  samples of a closed basin under its pass, or a part of the open low ground joined through its samples, which has no
  pass) held by the cells whose lowest sample lies in it, as a cell's own ground fills from its lowest point up. Its
  liquid stays in the ground nodes of those cells. The bodies, their cells and the ways to them are found once for the
  world's cells; at each month's start, each body's level from what stands over its cells' pores, and the cells under
  it (their mean surface under it: the others are its shore). The lowest body, the open low ground, has no code of its
  own. **Known limit:** a pond whose cells' lowest samples all lie outside it holds no body (its liquid runs through);
  a cell's ground at the eight by eight samples inside it (above) is coarser than the terrain's samples, so a body's
  volume by level is its cells' curves', not its samples'.
- **Running off** (`flow::runoff`), once a month in one pass: each cell in no body sends what may run off (what stands
  over its pores, what came up from the store, and the pores' drainage to the channels, below) down the samples' ways
  from each of its samples by its share of the cell's area, to the cell each way lands in: the cell of the first sample
  of a body on it, or that body's first cell (`bodies::Bodies::ways`). The bodies then come in turn, each before the
  body its pass leads to: each cell's pores keep what they hold and all the rest stands at one level over its cells'
  own ground (`curve::level_over`, exact), no higher than its pass; each cell that holds more than its pores and what
  stands on its ground under that level gives the share of its liquid over it, with that share of its heat; mixed, it
  goes to the cells that hold less, by what each lacks (`basin::share_out`), and what the body holds over its pass
  leaves, mixed, to the body its pass's way reaches, in the same month. What a cell sends and what a body lets over its
  pass go into the samples' ways at each sample and at the pass, and one pass over the samples from the last the flood
  reached to the first, each before the one its liquid runs to, sums what runs through each (a body's sample keeps what
  reaches it): the **discharge** of each sample (`World::discharge`, mol/s, a moving average over about a year). A body
  that receives less than it loses (evaporation, 1.5) falls under its pass and keeps what was dissolved in it.
  **Known limit:** the sea's slope under wind and rotation is not modelled, and what levels it crosses the sea at once.
  **Each body mixes its heat in a month** (`basin::mix`) under each air block: then its cells off their melting
  plateaus, with no soft cover or cover of bodies and the same phases, each take the block's temperature (their heat
  apart from the fusion enthalpy each holds, over their heat capacity; their floor's top too, which is part of each
  cell's ground node); of a cell of parts (1.1), its liquid's top.
- **The channel** (`flow::channel`): what a cell passes on runs in a channel whose width and depth grow as powers of
  its discharge Q, m³/s (hydraulic geometry downstream, Leopold and Maddock 1953): width `channel_width` ×
  Q^`channel_width_exp`, depth `channel_depth` × Q^`channel_depth_exp`, and its liquid runs at Q over width × depth.
  The discharge of a cell is the most that runs through one of its samples, its main channel (`World::liquid_flux`, mol/s,
  a moving average over about a year, from the samples' discharge), at the mol per m³ of the liquid its ground holds
  (`World::liquid_in`). Its size does not depend on the cell's: the push of moving
  liquid on structures (3.2) and every shell read it here.
- **Pore water** flows by Darcy's law (an explicit sweep on the liquid table, the store's too, 1.8): `seep` ×
  saturated thickness × width / distance, at most half of what would level two cells, and no cell giving more than it
  holds over field capacity; pores at or under field capacity (`field`) give nothing. Over cells of kilometres a
  liquid step moves a table a thousandth of the way to its neighbours', far below the explicit limit. Liquid above it sinks to the deep store
  (`drain::sink`, at most `seep` a second and the store's room) right after each of the air's steps, which bring the
  rain: held until the month's liquid step, it would cover and darken the ground (`land::reflectance`, 1.4) and
  warm it. The store flows by `bed_seep` (`flow::deep`). Real land drains to channels a few hundred metres apart, so both also drain by Darcy's law down
  the cell's steepest slope over a hillslope of `hill` into the runoff (`flow::drain`). Pore water moves before the
  runoff's pass, so the bodies end the month at one level. **None of this runs under a body**: the loose ground of a
  cell under its body's level lies under it with its pores full, and the bed's store under it is not drained, so no
  liquid sinks there, no pore or store flow reaches or leaves it, and its store feeds no channel.
- **A cell of parts** (1.1): what the liquid laws bring to it gathers in its liquid's top; what they take leaves its
  top and its deep by their shares of the liquid they hold (what stands leaves from the liquid), and its land only for
  what they lack, which keeps its own pores' liquid; at the parts' temperature by the liquid each holds
  (`parts::to_liquid`, `land::t_now`).
- **What goes with it**: its share of every liquid and of what is dissolved, with heat. Liquid passing through a node
  keeps its fusion heat whatever the node's phase and leaves at the temperature of the node mixed with what came in
  (on a melting plateau the plateau's substance melts or freezes first), so it never takes more heat than the node
  holds and what stays keeps that temperature. A salt is a liquid that never
  boils (11, 12): it moves with the liquid and stays when it evaporates. **Its potential energy goes with it** (0,
  "The books"): each move of the pass goes from the cell it leaves to the cell it lands in, its potential energy from
  the height it leaves (`potential::apply`), and the fall turns to heat where it lands. The head it loses on the way
  does work (1.3).

### 1.3 Erosion and creep
The head lost by moving liquid does work where it is lost on land (`flow::runoff::erode`, `wear`; in a body and below
0 m it wears nothing): in each land cell in no body, g times what runs through each of its samples in the month, kg,
times the fall to the sample it runs to (the samples' heights, which the run's erosion does not change). It
**carries** up to `carry` m³ of loose solids per J of the cell's work (at most all that lies loose), which go with the
cell's runoff to the cells its ways land in, where they settle, and with the work its load leaves free (the free share of that capacity: the cover effect, Sklar and Dietrich 2004) **wears** `wear` m³ of
bed per J over its strength into loose ground. **Creep** (`flow::creep`, yearly, out of land cells):
loose ground slides downhill as a slow diffusion, `creep` × softness (1 / (hardness + `hard_min`)) × thickness × slope,
across each border by its width (none across a corner). Bodies hold the ground: carrying
and creep are cut by 1 / (1 + their mass per area / `hold`). Most rock leaving the land is lime dissolved by reaction 8. **Burial**: the loose ground mixes over what lies on it (bioturbation, Darwin 1881), so a part no body holds and no bond joins sinks `bury` m/s into it, no deeper than the loose ground is, and once buried past its smallest outer size it joins its cell's ground (`bond/month.rs::bury`, 3.1).

### 1.4 The air's step, the sun and the bodies' share
`flow::air::step`, every `AIR_STEP_DAYS` (15). First the gas of the bodies' nodes comes up into their air nodes. Then each cell's exchange over the step with the air it sees is solved once (the sea under one air
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
its block and the wind (1.6). The book takes each cell's law at the mean air of the step (`air::book`), which is what
the days summed, so the air ends just where its days left it and no node gives more than it holds. A 15-day step
gives the climate of 1-day ones within 0.3 K on the land (`the_long_step_keeps_the_climate`, `tape/tests/ranges.rs`).
- **Sun** (`radiation`): the daily mean by latitude, season and day length (Berger 1978), declination sin δ = sin ε sin
  λ, the mean over the step's days. **On a slope** (`on_slope`), per m² of the cell's plan: by Lambert's cosine law the
  beam on ground of upward normal N = (−∂z/∂x, −∂z/∂y, 1) is S (s · N), s the unit vector to the sun, linear in the
  slope, so a cell takes the sun of its mean slope and a square's sun is the sum of its cells'. Over the hours the sun
  is up the east-west part cancels and the north-south part is the slope of the daily mean with latitude: Q + r ∂Q/∂φ,
  r the cell's mean rise toward the south (section 0), the equivalent latitude of Lee (1962) and Swift (1976) in plan form. An
  hour a face turns away from the sun counts against it, as the shadow it casts on the ground beyond, so the beam is
  kept whatever the slopes; a cell whose day sums below zero takes none (the one place a square and its cells differ).
  A cell under a body's level (1.2) lies level and takes the flat sun. All the sunlight that passes the clouds is taken as beam:
  a cloud is one reflectance, with no share of the sky it covers, so what passes it cannot be split into beam and
  diffuse; diffuse light would fall on the plan as on flat ground. The cost is a product and a sum per cell a step,
  with the slope with latitude found beside the daily mean, once per latitude. The surface reflects as its cover is at the step's start (`land::reflectance`),
  over the cell's own ground: the share of it under the level of what stands over its pores (`curve::Curve::wet`) as a
  column with that liquid over the share, the rest as one whose liquid is what its pores hold, by area;
  where a soft cover lies, as it is at the step's end, so a cover that melts away darkens the ground in its step. Snow
  reflects from the step after it falls.
- **Heat radiation**: surfaces emit σT⁴; the air absorbs 1 − exp(−x), x the optical depth of the gases (each as the
  square root past its own column amount) and clouds, and emits from 2 grey layers that cool with height, so more gas
  lifts the layer that reaches space into colder air and the ground warms. The layers are each air node's, over its
  mean ground; the air touching a cell is the air it sees (bilinearly, its four nearest nodes), colder by the drop with
  height from that air's mean height.
- **Clouds** (`exchange::cloud`): scattering depth x = condensation rate / `cloud`, from what each air node rained in
  the step before; they reflect x / (1 + x) of the sunlight (two-stream, Lacis and Hansen 1974) and absorb heat
  radiation at depth `cloud_ir` × x (Stephens 1978). A globe of Earth's size without life holds about Earth's
  climate. **Known limit:** their depth is spread like the air's mass, not set where they condense. The ground also
  trades sensible heat with the air (`sensible`).
- **The bodies' node** (`Class::Bodies`, one per cell): light the cover absorbs warms the ground, except the share its
  light reaction stores (`Cover::keep`), booked from the sun into this node; the cover takes it at its month, and what
  it does not claim goes back to the ground before each air step (`survey::release`).
- **A body as one surface** (`air::surf`): under each air node a body's cells that mix as one (1.2) face the air as
  one surface: one balance with their summed area, heat, heat capacity and gases, at their mean temperature (by heat
  capacity), reflectance, height and air (by area) and the sunlight of their mean latitude. It answers the air as one
  (`surf::Patch`): its law, linear in the air it sees (its cells' weights by area), is summed into its node once, and
  the book takes it once, at the mean of that air over the step. Each cell takes its share of what it gives: of the heat
  by its heat capacity, so they warm alike, and of each gas by what it holds of it, so none gives more than it holds.
  Its rain falls on it as on one place, at the air touching it by area, shared to its cells by area (1.5).
- **Gas from bodies and the cover** goes into the bodies' node with its latent heat; the air's step takes it up with
  its lift.
- **Shade**: the cover's kinds share its light by height (4.5); bodies catch no light.

### 1.5 Evaporation, rain and dissolved gases
`flow::air::cells`, `daily`, `rain`, `exchange`. **Evaporation** by vapour pressure (the ground's saturation pressure against
the air's): wet ground at the open rate, drier ground in proportion to its liquid (Manabe 1969's bucket), over the
cell's own ground as its reflectance is (the share the standing liquid covers holds what its pores hold and all that
stands, the rest what its pores hold, each a bucket by its area), taking the heat it held at the node's temperature; a node over a substance's boiling point boils of it what its heat over that
point pays, cooling to it (`land::boil_off`; gas coming out of the liquid too). **Rain**: what an air node holds of a
gas over what saturates the air touching each of its cells (colder higher up) rains out in `rain_time`. Each node's air
lies over the cells that see it (bilinearly), each by its share; the cells' saturation is taken as the step starts, as
a ratio to the node's, so the day sums them in rising order. In the air's day the vapour of each block (1.6), evened
as the eddies even it, follows what its cells evaporate by their law and what its nodes rain: both are linear in the
vapour between two of the ratios at which a cell starts or stops raining, so the day goes exactly from one such ratio
to the next, with no smaller steps (`rain::Pool`); the air outside the block is as at the day's start. What a node
rained in the step falls on those cells (the sea under one air node as one place, 1.4) by what each would hold over its own saturation at the ratio the node rained
at. It lands after the step, so a cell does not evaporate it again within that step, and it reaches each cell at the
temperature of the air touching it (`touching`), the air paying what warms it on the way down. Where the node's air at
its mean ground is below the gas's melting point it falls as a solid (snow) and the air keeps the fusion enthalpy its
freezing frees; condensing frees the vaporisation enthalpy in the air. **Lift**: gas going up is lifted from the
ground's head to its air node's mean ground, paid by the air's heat; rain's fall gives it back (0, "The books"). **Dissolving**
(`exchange::dissolve`, once a month, at the end of the liquid step, over the month, at the ground's temperature as the
liquid step leaves it and under the air of the last air step's start; the gases that dissolve are chosen at the month's
start and evaporate in no air step of it): a gas that cannot condense dissolves in the ground's liquid toward Henry's share, through the
air's film (`evap`) and the liquid's (`dissolve`) in series, freeing its vaporisation enthalpy, and comes out when the
liquid warms, thins or freezes. The sea under one air node (1.4) takes it up as one liquid, each cell by its share of that
liquid, and gives it up by what each holds of it. **Known limit:** a liquid takes up gas into all its depth, its deep
as well as its top (1.1), so a deep sea takes up gas in months; a real one takes centuries.

### 1.6 Wind
`flow::air::wind`, `daily`. Eddies (`mix`) even out the air within a day over the distance sqrt(`mix` × day), about
340 km (Taylor 1922, `daily::BLOCK_DAY`): the air nodes lie in blocks of that side on the map, and each day every gas of
a block is shared among its nodes by their volumes and its heat so that each has the block's warmth (its temperature
raised to a common height by the block's drop with height), so its pressure, shares and warmth are the block's. An
island smaller than a block (the islands of 64 and 128 cells) is one block, and has no wind: only the evening. Across the links between blocks gas moves by three pushes, each day from the air at
its start. **Pressure** (p = nRT/V), evened in the time sound takes to cross the link (Lamb waves, Lindzen 1990),
explicitly: the sender's air, each link at most half of what evens it over the count of links of the busier of its
nodes, so no node gives more than half of each gas it holds. **Known limit:** sound is faster than that, so a pressure
difference across many links of blocks takes days to even out. **Eddies** mix the columns (`mix`, width over length),
each mol with its node's heat. **Overturning**: the lower half flows to the warmer column and the upper half back, a
mass flux of `wind` per K/m of the warmth gradient along the link (its difference over its length), turned by the
planet's rotation (`rotation`), through half the link's cross-section, so it does not depend on the size of the nodes.
Each half carries the air of the node it leaves, vapour crowding low, each mol with its node's heat per mol, and the
upper half holds more heat per mol than the lower by the rise of the static energy cp T + M g z over half the column's
depth, (M g - cp gam) × depth / 2, gam the column's drop with height (none where the air cools with height as fast as
dry air rising; Neelin and Held 1987). So the heat it carries a second is its mass flux times the difference of the
two nodes' heat per mol, less that rise: the first part grows with the length of the link, the second does not.
**Known limit:** as a half carries its node's air, on coarse nodes the overturning also mixes the two, as eddies of
about its speed times half the length of the link would. The eddies and the overturning are one implicit (backward
Euler) step a day, first along the lines the links between blocks make east and west, then along those north and south,
each line solved exactly (`solve::line`: Thomas; a ring by Sherman and Morrison): stable at any rate, no gas of a node
goes below zero, and the day keeps its rates on any grid. **Known limit:** their rates per mol a node holds are those
of the air step's start (15 days), so a node whose air grows or shrinks within the step moves a little more or less
of it than its rates say. Mass that crosses is
lifted between the nodes' mean grounds by the heat of the node it enters. A cell sees its air node and three
neighbours bilinearly (`flow/scratch.rs`). Air moves as whole columns, so bodies use the anchored `wind_ground` (6.3).

### 1.7 Paths
`lanes.rs`. A body moving through a cell spends energy pushing through the bodies there (`choose::go`, `choose::wear`),
which breaks them (6.8) and clears the link (`lanes::clear`) of the ground whose cover that energy breaks (the energy
over the cell's breaking energy per m²); a cleared link is cheaper (`lanes::resist`) and grows back yearly
(`lanes::regrow`, `regrow`). This one rule is what can make paths.

### 1.8 The step by the size
An explicit step is safe while `dt Σ g / cap` of each node stays within 1/2 (no pattern changes sign in a step,
`solve::sweep`). For a diffusion of D m²/s on a cell of side Δ with four borders that is 4 D dt / Δ² ≤ 1/2, so a step
of `dt` is safe down to Δ = √(8 D dt). On the island (month 2.59e6 s, year 3.11e7 s):

| Process | Step | D or rate | Safe down to |
|---|---|---|---|
| Pore flow, Darcy (`seep`) | month | `seep` × 1 m of loose ground / `porosity`: 9e-6 m²/s | 14 m |
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
absorbed light and store `light_yield` of it. **Catalyst**: a body runs no reactor: its diet genes (one per
reaction bodies may run, 6.1) switch its burnings and what it digests on or off (2.4, 6.2, 6.7). **Open world**
(`Reaction::open`): in ground, liquid and air live the decomposers (a part's walls rot at their rate, 3.1), whose catalyst `open` works as far as the
node is moist: 1 + (`open` − 1) × moisture (`change::open_catalyst`). Moisture is how wet the node is as living things
feel it: d / (d + `wet_depth`), d the depth of its liquid in its pores and standing (`FlowView::moist`,
`Physics::wetness`), so decomposers work on dry land too, slower; an air node has none. `open` is 1 for most reactions; 200 for reaction 1 (the free sugar's rot, about a
week); 5 for 7 (body matter rots 3.8 times faster than the frame); 10^6 for 12 (the frame rots by the decomposers
only: bare, a millionth of that). No decomposer works in a body (2.4).
`change::rate` (catalyst × k exp(−Ea/T) × the shares and contact of `pace`) is the one rate of all nodes and of a part's
walls (3.1). **Frozen**: a bare reaction whose rate constant k exp(−Ea/T) would
use under 1e-12 of an input over the world's year is skipped in every node (`frozen_below`, `NEGLIGIBLE`), a speed
tolerance.

### 2.2 Where it runs
**Ground and air nodes** (`change::step`, `Reactor::react`) react when due over all the time since, at Flow's last
temperature (a plateau's fusion heat set aside), then sleep until their reactions would use 5% of an input or warm
them 0.5 K, at most `sleep_days`. The nodes under one air node react as one task (a block), each into a row of what its reactions changed and the gas it gives its air node; the rows are booked after the call, each array of the store by one task (`change/booking.rs`, `store::book_by_array`), so an air node gets the gas of its ground nodes in the order of their cells whatever thread ran them. Gas a ground node takes or gives
(what its reactions made less what the node itself gained, so it never gives more than it holds) crosses to its air node with its heat and latent heat. **Parts** run no reactor: their walls rot to the ground (3.1);
**bodies** burn and digest by their diet (2.4); **growth** is a conversion bodies and
the cover run, paid by work (4.5, 6.6).

### 2.3 What gas a ground node touches
Its own gas and its cell's share of the air. **Under standing liquid** only the gas dissolved there, renewed through
the liquid's film in τ = depth / `dissolve` (`FlowView::standing`): a gas it takes is held at Henry's share × (1 +
made·τ/eq) / (1 + used·τ/eq). So dead matter on the sea floor rots as fast as the film brings the oxidising gas.

### 2.4 Change inside bodies
Bodies run no reactor: they run three conversions of the chemistry, each booked by its stoichiometry. **Burning** on
demand to pay their costs (the reactions of their diet that release heat, `Chemistry::burners`: in chemistry 0 the
fuel's, reaction 1, and the body matter's, 7), as far as their fuel and their air node's gases allow (`body::burn`).
**Growth** by `Chemistry::build` and `builds` (6.6). **Digestion** (`Body::digest`): the reverse of growth for matter
taken in. A body's diet is its reaction genes: on above 0, it burns or digests what that reaction takes (`Body::uses`).
The light reaction runs only in the cover (4.5), the decomposers' reactions only in the open world (2.1).

### 2.5 Phases and the freezing law
Solid, liquid or gas by melting and boiling points (`Chemistry::phase`). A liquid's melting point in a mixture falls by
the ideal-solution law (`Chemistry::melt_in`): 1/T = 1/Tm − R · ln(x) / Lf, x the solvent's share among itself and
its solutes. Only the solvent (the most abundant liquid) is lowered; solutes are liquids melting below it (the salts 11
and 12, the fat 6); ice and liquid count together, so it freezes at one point. One law for the stores of bodies,
ground plateaus, ice cover (`World::thaw`), parts and Change; a body's frost goes by its key liquids' own melting
points (6.5). **Latent heat**: a gas in the air holds its melting and boiling heat (`Substance::latent_gas`), paid or
got as it enters or leaves; a body holds the fusion heat of its key liquids at any temperature (`Body::fused`: it has
no melting plateau, frost kills a share of its members instead, 6.5), and liquid it takes or gives carries it
(`Body::heat_with`). **The densest temperature** of a liquid (`Chemistry::dense_in`, what overturns a liquid's top and
deep, 1.1) is its pure liquid's (`Substance::dense`) lowered in proportion to the mass of its solutes per mass of it
(Despretz's law, `Substance::dense_drop`), and no lower than its melting point there: a liquid salty enough is densest
where it freezes. The sea of chemistry 0 is nearly fresh: about 0.2 g/kg dissolved (a real sea about 35), so it is
densest at 276.9 K, stops overturning below that and freezes at its top, as a lake does.

### 2.6 Pairs at balance and the sea's carbon
`change::reversible` finds each pair of reactions where one undoes the other; `Reactor::balance` finds from the
chemistry alone where a pair goes at equal rates. In chemistry 0: 8 makes the dissolved salt 11 from lime, the carbon
gas and the liquid; 9 turns it back. Wet cells start at that balance (`worldgen::terrain`), so the sea holds several
times the air's carbon and gives it back when the air thins, with no rule for the sea.

### 2.7 Chemistry 0 in short
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
| 10 | body matter (≈ tissue): C118H196N6O101 |
| 11 | the rock salt, lives dissolved (≈ Ca(HCO3)2) |
| 12 | the nitrogen mineral, lives dissolved |
| 13 | the frame, a solid bodies build (≈ cellulose) |

Reactions: 0 light to sugar; 1 burning sugar (its rot in the open world); 2 and 3 burning and making fat; 4 burning
methane; 5 rotting to methane; 6 rock weathering; 7 rotting or burning body matter (gives back the nitrogen mineral); 8 and 9 the lime–salt pair; 10 fixing nitrogen; 11 the nitrogen mineral breaking down when hot; 12 rotting
of the frame; 13 splitting the frame into sugar (needs an enzyme). Dead matter rots in moist open ground at a litter
constant (12: about a quarter a year at 290 K; 7: about one a year). **Growth** (`Chemistry::build`, bodies only, paid by
work): matter from 118 sugar + 3 nitrogen mineral; frame from 6 sugar.
`Chemistry::minerals` are growth's inputs no reaction burns (12), `materials` the solids bodies build (13).
The air's carbon gas (3) settles where the cover's light reaction takes it up as fast as rot and burning give it back:
the world has no source of it (an input from below, as volcanoes give, would be an edit from outside the laws, and is
not built).

## 3. Bond: things can be joined, joins break when pushed too hard
Files: `core/src/bond/` (`verbs`, `edge`, `sense`, `month`, `bonds`); also `choose::go`, `Body::breaking`.

### 3.1 Parts and structures
A **part** is a made object (solid walls, the liquid in its hollow, heat, 3 sizes, a hollowness), a store node
(`Class::Part`) stored in the list of the cell it lies in (`life::Cell::parts`), with an id that never changes: the
serial of the cell it was made in and that cell's count of parts made (`bond::Part::id`, as a body's, 0); held or not,
it lies in the cell of
its holder. The bonds of a cell's parts are the cell's (`life::Cell::bonds`): a structure lies in one cell, and only a
structure that moves (carried) spans two cells for that step, which is serial. A body makes one with "form"
(`bond::form`, `c_form` per kg, at most `form` of its mass): of the solids it holds above what it keeps and, in the share its brain sets, of the loose solids of its cell's ground in their proportions, as far as the ground holds them, with their heat, at `c_handle` per kg more (on land mostly rock, so the part is hard, 3.3). A **bond** (`bond::Bonds`) joins two parts; its strength comes from the weaker material and the contact area (`tensile`, `joint`), at `c_join` per m². A
**structure** is the parts joined by bonds and moves as one. A structure's **design** (`bond::designs`) is the hash of
its bond graph with its parts' labels: each part's material band (the hardness and the bond strength of its walls) and
shape band (its hollow share and the elongation of its sides), each cut into `BANDS` (4) bands, and the band of the mass
of its walls (`MASS_STEP`: a factor of 2, so alike parts are within 26% in size), by `WL_ROUNDS` (3) rounds
of neighbour hashing (Weisfeiler and Lehman 1968; Shervashidze et al. 2011): it is how alike objects find each other, as
a lineage is for bodies, and the pattern finder's key (`docs/analyze.md`); it is never copied.
**Lots** (`bond/lots.rs`; handover 7.1 applied to objects): the alike parts that lie alone in a cell (held by no body,
no body inside them, joined by no bond) are one record of `count` parts (`Part::count`, real, as a node's members),
which holds the amounts and heat of all of them and the sizes and hollowness of each. Monthly, before their turns, the
lone parts of a cell of one design and one band of burial (their depth over their smallest size, `BANDS` bands) join
into the first of them in the cell's list, which keeps its id (`lots::join_alike`, `Part::join`): counts, amounts and
heat add up, sizes, hollowness and depth become means by count, and the sizes fit the walls of one part. A body that
acts on a part of a lot acts on one split out of it (`lots::some_of`, `Part::split`: its share 1 / count of each
amount and of the heat, the lot keeping the rest, so the two add up to rounding), a part of its own with the next id of
its cell, and one that holds takes as many as it can hold (3.3); a break finds no bond in a lot. **A cap of lots**, a
speed tool: a cell keeps at most `MAX_LOTS` (16) lots of lone parts; past them its lots join again by a coarser key, the
least band first (`lots::key`): no band of burial, then a mass band 2 to 64 times as wide, then no band of elongation,
of hollowness, of bond strength, of hardness, and last none of mass, until they are no more; so the records of lone
parts are at most `MAX_LOTS` a cell, whatever the parts, and so is the work of the monthly pass over parts, which goes
over each record a fixed number of times (a world of 1e6 parts placed by an edit holds 16 records a cell from its first
month on, `core/tests/lots.rs`), as `MAX_NODES` bounds the bodies (4.4). Alike parts lose their own history, as members of a node
do, and their sizes within a band. A part runs no reactions. Monthly (`bond/month.rs`) the
share of its walls the open world's decomposers would rot in the month (`bond/month.rs::rot`: each reaction they speed,
`open` above 1, at the rate of Change, `change::rate`, at its cell ground's temperature and moisture, each substance of
its walls a phase of its own, as a solid is in Change, and the gas it needs taken from its cell's air by its share
there) goes to its cell's ground with its heat, where Change rots it on, as do walls no longer solid; then it takes its
cell's ground temperature. The walls are the scarce input of that rot (a part holds far less than its cell's air), so
it is first order in them: one share per substance fits every part of a cell (a reaction the open world speeds takes
at most one substance that bodies hold, and walls hold only those: `Chemistry::validate`). The parts of a cell take
their turns together, on the cell's view of the store: one move to the ground per substance, one move of heat
(`bond/month.rs::warm`). That move is what the parts that warm want less what the parts that cool give; where the
ground cannot give it all (it gives no heat below `T_FLOOR`, `Moves::take_from_ground`), the parts that cool still
reach its temperature and those that warm share what it gives and what the others gave, each by what it wants. Rot and
heat act on a lot's amounts as on a part's: the share is one for every part of a cell. A part whose walls (of one part
of a lot) fall under `min_kg` falls apart (`bond::fall_apart`): all it holds, its walls, the liquid in its hollow and
its heat, goes to its cell's ground, whether its walls ran off in its turn or a body took them (`bond::verbs::take`),
and it is gone with its bonds; so does a part buried past its smallest outer size (1.3), while a bonded structure stays
on top. **Burial by a share**: the depths of a lot's parts are an even spread, its mean and half width
(`Part::buried`, `Part::spread`; one part's is 0); each month it moves down by `bury` × the month, its shallowest and
deepest no deeper than the loose ground, and the share of the spread past the smallest size falls apart (`Part::sunk`,
`bond::fall`), the rest then spread from the shallowest to that size, as the members of a node mature by share (4.4).
A join pools the spreads (mean by count; variance an even spread's half width squared over 3 plus each one's gap to the
mean), and the band of burial keeps a lot's spread within about a band, so each part is buried within a quarter of
its time of its own.

### 3.2 Loads and failure
Monthly (`bond/month.rs`), from the part a body holds out along the bonds, each bond carries the weight hanging on it,
less buoyancy, plus the push of moving liquid and of bodies passing (`bump` of their weight); past its strength it
breaks. Moving liquid pushes a face with ½ ρ v², v the speed of the cell's channel (1.2, `Physics::channel`) where liquid
stands at least a centimetre deep in the cell. A structure no body holds lies on the ground and bears no load.
Structures do not drift; only a body carries one (3.5).

### 3.3 Holding, floating
Holding is a bond whose strength is the body's **force**, strength gene × living body × gravity × `grip` × its
vigour (`bond::bears`); a body holds only while its force bears the structure's weight. Fuel adds no force; a hurt body
is weaker, and so is a cold one: its **vigour** (`Body::vigour`) is its pace (6.2) at its temperature, at most 1, as
muscle's power falls with temperature by the rate law of what powers it (a Q10 of 2 to 3; Bennett 1990). In chemistry 0
a body at 280 K has 0.38 of its force, at 260 K 0.05; at and above `t_cat` (290 K), where the force and the walk are
anchored, all of it. A bite's force (`feed::take_body`) is each one's force with its vigour. 
**A node holds a lot** (3.1): one part in each member's hand, so a body takes hold of as many parts of a lot as
it has members (one for an individual), as its force bears with the weight of each one's structure, and as the lot
holds, split out of it where they are fewer (`bond::verbs::can_hold`), and pays to handle each; at least one part (or all
of a lot of fewer), or none. Holding, carrying (`bond::carry`), riding and the loads (3.2) go by all the parts held,
their weight, displaced liquid and the drag of moving liquid on each. Monthly, before the lone parts join, a body that
holds more parts than it has members (members left or died) lets go of the
rest, which lie alone (`lots::hold_by_members`). A member that joins its part to another (verb 3) joins one of the lot. **In liquid** (`bond::wading`): the share of a member's height under its cell's standing liquid is the depth
over its height, at most 1; its height is its size, the side of a cube of its volume (`Physics::side`, 0.31 m at
30 kg). Where the liquid is deeper than its height a member swims: it breathes its air (2.4), so it keeps to the
surface, and the ground and the cover lie within its reach only as deep as its height and its reach, each its size
(no gene lets a body dive; 5.3). A body rides the structure it holds dry, at the surface, where the liquid is deeper
than its height and what that structure displaces weighs as much as both (Archimedes): it reaches down its reach
alone. The share under the liquid passes its heat to the liquid, not the air (6.3). No body digs the bed.

**A held edge concentrates force** (`bond/edge.rs`). Without one a body breaks matter bluntly: it gives way through its
whole width (3.6). Pressed with the edge of a held part, only a zone as wide as the edge gives way (Atkins 2009): for
each material k of what is broken, the energy and the force fall to the share c_k = min(1, e / l), e the thickness of
the part's walls (their volume over half its outer surface, `Part::thickness`), l the width of what is broken (a member
of a body, `Body::width`; the cover, its volume per m²; a joint, the root of its contact area), where the edge is
harder than that material (scratch hardness, `Substance::hardness`: an indenter marks only what is softer, Tabor 1951),
else c_k = 1. Over a thing, c̄ = Σ V σ c_k / Σ V σ (`Body::cut_by`, `Cover::cut_by`). A node's parts are one in each
of m of its n members' hands (a lot of m held, at most n), so the edges leave the node 1 − (1 − c̄) m / n
(`Edge::leaves`), as m of its members cutting and the others breaking bluntly: a node of 100 holding 100 edges bites
as 100 individuals each with one do (`core/tests/lots.rs`). The edges wear together, by the work done through all of
them, on the lot's walls. A bite pays that share of the breaking and its
force counts 1 / that share against the other's (`feed::take_body`); grazing pays it of breaking the cover
(`feed::take_node`); breaking a bond (verb 4) needs it of the bond's strength, from an edge of another structure. A part
whose walls have no breaking stress holds no edge. **The edge wears** by Archard's law (Archard 1953): it loses the
volume `abrade` × W / H of its walls, W the work done through it, H = `TABOR` × their breaking stress (Tabor's
relation for ductile solids, used for every solid: rock is in truth far harder, a known limit), each substance in its
share, to its cell's ground with its heat, in the landing of the acts on parts (`bond::blunted`); under `min_kg` the part
falls apart. In chemistry 0 body matter and the frame have the same scratch hardness (0.2), so no part of them cuts a
body or the cover: only rock (substances 7 to 9, 0.35 to 0.7) does, which a body picks up from its ground (3.1). Pushing through (6.8) gains nothing from an edge: it
pools all movers' energy in a cell.

### 3.4 Containment
Walls that close a space make an **inside** (`bond::room`) joined to its cell by one link (`bond::Enclosure`): heat
passes the walls, each part its face over the still air on both sides of it (`r_air` a side, as ISO 6946's surface
resistances) in series with its thickness over its conductivity, and the openings (`leak` per m²); light and rain only
the openings. **The inside holds air** (`bond::room_air`): a share of its cell's air, not made. The heat that leaves
the bodies inside through their coats goes into that air, and from it through the walls and openings to the outside,
so in its steady state the air stands above the outside by the heat of all the bodies inside over the link's
conductance. Its own time, its heat capacity (about 1.2 kJ per m³ and K) over that conductance, is seconds to minutes,
far below a sub-step: it stands there at every sub-step and holds no heat of its own, so what the bodies lose reaches
their ground as outside, and the books need no node for it. A body inside stands over that air, raised by the heat the
others inside lost in their last sub-step (`Lived::lost`: those before it in the cell's order this sub-step's, the rest
the one before), through
its coat in series with the link: its own heat warms the air too (6.3). So several bodies in one room warm one another,
and a node's members warm their room together. "Shelter" is only this low conductance. A
body enters only where it fits (its volume within the space the walls close, with the bodies already there), and one
that outgrows the space, by growth or by members joining it, is no longer inside (`bond::leave_outgrown`, at its
update).

### 3.5 Moving needs force
A mover pushes what it passes with `bump` of its weight, so it moves only if its force is at least `bump` × its whole
mass and load × gravity (`choose::go`). The starting plants are too weak to move. A move refused for room or force
changes nothing: a body lets go of what it cannot carry only once it can step. A move that passes both but cannot pay
its cost does not happen either, and the body has let go of what it cannot carry before it was asked to pay.

### 3.6 What a body is built of resists breaking
**Breaking** (`Body::breaking`) a share f takes f × Σ V × each material's breaking stress (`rupture`), V the volume of
soft tissue and each built solid: a bite pays it, pushing through breaks what its energy pays for (6.8). This is blunt
breaking; a held edge needs less (3.3).

## 4. Copy: a piece of the graph is copied, with small changes
Files: `core/src/copy/` (`mix`, `grow`, `litter`), `nodes.rs`, `body/genes.rs`, `lineage.rs`; the cover in `core/src/cover/` (4.5).

### 4.1 Genes
16 scalar genes (`body::Layout`; `HEAT`, `STRENGTH`, `CHILD`, `COPY_MASS`, `MUT`, `LEARN`, the brain sizes
`HIDDEN`, `MEMORY`, `SEEN`, `SIGHT`, and `DISPERSE`, `REPAIR`, `WET`, `ALLOC`, `KEEP_T`, `COOL_T`; meanings in
`body/genes.rs`), a diet gene per reaction bodies may run (on above 0), per store a safe minimum, an uptake and a
keep gene (6.1; in place, `body::Genes`), and the born brain rows. **Mutation** (`body::vary`): parent's gene plus
noise of one step × 10^`MUT`,
within range (not the brain-size genes of a body with no brain, 4.4); born rows change with `row_chance` by
`row_noise`. Children of their parent's born rows and lineage who join a node, and a node's children who start a
node in its cell, keep their parent's born rows: the change is their spread, pooled into the row spread of the node
they join or start (4.4); a litter draws it only where it starts a body (`copy::land_litter`). **Likeness** (`body::alike`): every
gene within `alike` steps; it decides who mixes, joins and merges.

### 4.2 Children come from a brood
A body is **mature** (`Body::adult`) when its living body per member reaches `COPY_MASS` (a young node's members each
when theirs does, 4.4); then growth (6.6) puts the
share `ALLOC` into a **brood** (`Body::brood`), matter held for children that leaves with the body's matter in the same
share (`Body::lose`). Whenever the brood covers a child, children are made (`copy::copy`) whatever the brain does:
bodies copy by physics only, at each sub-step of the body's update (its physiological step, 0), all that its brood
covers then. The brood leaves with members who die. **Born through the sub-step**: growth builds at one rate over a
sub-step, first what the body's maturity lacks, then the brood its share, so the brood fills evenly over that part of
the sub-step and the children are born through it as it covers them (`copy::Fill`): a node's members evenly from the
time the brood passed what it holds back, an individual's whole children each when the brood reached it, those it
covered before at the sub-step's start. A child counts its time from its birth (`Body::last`, `Body::decided`): from
then to the sub-step's end it takes what lies in its cell by the take law, as a body does for the part of its step no
act fills (5.3; `copy::feed`), and the first sub-step that integrates it, its young node's or its own, runs it from its
birth by the laws of every body (members who join a node count from when they came, `nodes::rejoin`). In a cell the
mature bodies run each sub-step before the young (`survey::mature_first`), all after every body there took its meal
for it, so the children of a sub-step join their young node, fed from their birth, after its meal and before its
sub-step of the same time, as in a cell whose step is the sub-step: they grow and pay inside it, in its heat, and
eat with it at every sub-step after, whatever the step. A child has a body of
10^`CHILD` × the copy mass,
provisions of the other stores in its parent's proportions (the key liquid × 10^`WET`, so children may be dry and
dormant), its share of heat, and the genes plus noise. A node keeps half a child of brood per member
(`copy::BROOD_HELD`), so its members' broods lie evenly between none and one child, as individuals' do. A share
`DISPERSE` of the children disperses by the dispersal kernel (`copy::lot`): a lot starts at a point drawn evenly over
its cell and moves by an offset drawn from a kernel of two scales, a near and a far one (Clark et al. 1999: most within
tens of metres, a long tail to km). Each scale is an isotropic Gaussian, `Physics::kernel_near` and `kernel_far` m
wide; the far one is drawn with the share `kernel_far_share`. The lot lands in the cell that holds the point (none: off
the map, so it stays), so in expectation what lands in a cell is the kernel integrated over it from the source's area,
at any size of source and target (a cell and its four quarters give the same totals), and a lot costs the same
whatever the cells. The draws are keyed by stable data, so threads and chunk order change nothing. Each child is one
lot (`copy::landing`): a node's children that disperse go together to one cell, and from a square of 2 km nearly all
stay, the far scale sending a small share across a border. One way for all of a node's children of an update (for an
individual, that chance), where they land when the batch ends (0). Less than one member with no node to
join waits in the brood, or, landed in a neighbour cell, goes back into it. Children who start a body in their parent's
cell run the sub-steps after their birth, eating by the take law, and first decide at the cell's next step; those who
join a young node share its decisions; a litter lands, joins or starts a body (`copy::land_litter`), and its physics
runs on from its birth to that cell's next step after the landing (0).

### 4.3 Two parents
Only where brains are big: a mature individual (a body of count 1) of a lineage whose brain has at least `MATE_HIDDEN`
(8) hidden units (`copy::mates`) takes the most alike mature **partner** in its cell (`copy::mix::partner`); each gene
and born row of the children is one parent's, drawn evenly, then mutated (`mix`, `mix_rows`). The partner gives
nothing. Without a partner, one parent. Bodies not alike never mix, so **kinds are the groups that mix**. Any other
body's children have one parent: a node's members mix inside it (one variance per gene and no covariances is free
recombination, Bulmer 1971), and so do the members of a small brain's node.

### 4.4 Population nodes
Every body is a node: one body for many members, and a node of one member is an individual in the same maths (one
rule of death, of taking, of children for both: what a share of a node's members does, an individual does with that
chance).
- **Exact members** (`nodes::Lot` for every split, `nodes::rejoin` for every join, inside `nodes::add` for merges and
  births): members, stores, heat and brood split by shares and join as sums; genes, sizes and own rows become means by
  members, and the spread per gene (`Body::spread`) pools each one's spread and its gap to the new mean, as the
  spread of its members' living body (`Body::body_spread`) pools around the mean member's; a node keeps its born rows, and its row spread (`Body::row_spread`) pools the joiners' spread and their gap to them. Members of a node with a brain that take any action but a move or a
  bond (`choose::spread`) act as the node itself while the others wait aside with their share; these come back by
  `rejoin` alone: their genes and spreads are the node's, so only amounts and sizes (members who fed are now larger
  than those who waited) join; what members who watched copy is applied when the batch ends (5.5).
- **One node per kind and stage** in a cell: one lineage per node (one brain). Children join the young node of their
  lineage in their cell (`copy::kin`), and members who move join the node of their lineage and stage where they land,
  or start one; in a cell of `MAX_NODES` bodies or more with no node of their lineage, they join the first node of
  their starting kind and stage, taking its brain, born rows, lineage and brain-size genes (so the cap below holds
  through the month).
- **Maturity by share** (`copy::mature`): a young node's members mature as each reaches the copy mass, not all at once
  when its mean does. At the end of each sub-step of its physics, the members past the copy mass move to the mature
  node of its lineage in its cell (`copy::kin`), or start one with a copy of its brain if there is none and they are
  at least one member (fewer wait), by `nodes::Lot` (members, stores, heat and brood exact): the flux across the size
  of maturity of a McKendrick–von Foerster population (McKendrick 1926; von Foerster 1959). The node's sizes are taken
  as spread evenly over the width their variance (`Body::body_spread`) gives around its mean member, at most from
  nothing to twice the mean; the movers are the share of that even spread past the copy mass, holding the larger
  share of the amounts their larger bodies hold, and each part keeps its own even spread. Members grow alike in a
  sub-step, so growth moves the spread up as it is; children who join at their birth size widen it. At least one
  member stays young. A node of members all alike matures whole when its mean does, as an individual does.
- **Merging** (`nodes::crowd`, monthly, in each cell after the batch's serial pass, one task per chunk; a merge that
  lets go of a part queues that, as in a batch): the bodies of one lineage and stage in a cell, in the room of the same
  structure (its root) or none and holding the same structure, parts of one design (a lot no bond joins, by its design
  and mass band, `bond::held_kind`) or nothing (`nodes::Kin`), join the largest of them,
  where that is a node, they are more than `PAIR` (2: a pair of individuals may mate) or their individuals may not mate
  (4.3). Then a node joins a larger
  one of another lineage of its stage, room and part whose genes are alike (`body::alike`, on the means) and whose
  brain has the same sizes: its members take that brain, its born rows and its lineage, and its rows become the mean by
  members (`nodes::merge`); members of another lineage keep the node's brain-size genes, so no merge moves its sizes. A
  joiner's lot joins the lot the node holds where both are of one design (`bond::join_held`), so the node holds the
  parts its members held. This merge is
  a speed tool with physical side effects: the joiners' members drop any other structure they held, which stays in the
  cell for any body to take, and a lineage's brain spreads to alike bodies of another. Selection acts between cells and
  lineages, not within a cell.
- **Three caps, speed tools.** After its merge (`nodes::cap`) a cell that holds more than `MAX_NODES` (8) bodies keeps
  one young and one mature node of each starting kind in each room (or outside any): the bodies of one kind, stage and
  room join the largest of them, whatever their lineage, brain sizes and part, taking its brain, born rows, lineage and
  brain-size genes; its rows become the mean by members where the brains have one shape, else they stay its own. A
  cell that still holds more than `MAX_NODES` bodies (a kind in the rooms of many structures) bounds its rooms too
  (`nodes::across_rooms`): its `MAX_NODES` largest bodies stay, and each one past them joins the largest of its kind and
  stage in the cell whatever their rooms, its members leaving their room for that body's, where they stay while they
  fit (3.4), unless it is the largest of its kind and stage. In any
  other cell a body with fewer members than `node_floor` (1%) of the largest body of its lineage, stage and room joins
  that one, unless it may mate (`copy::mates`, 4.3). Either way members, stores, heat and brood join as sums, and a part
  the joiner held stays in its cell, held by none. So a small group (a few members who took a part) does not cost an
  update of its own, while a pair that may mate and a lineage just founded stay apart; a crowded cell holds at most two
  bodies per kind present and room, and at most `MAX_NODES` and two per kind present across its rooms, and kinds never
  mix. A starting kind keeps at most `MAX_LINEAGES` (8) living
  lineages (`lineage::resize`): past them a body whose sizes change joins the living lineage of its kind nearest in
  sizes, and stays in its own if that is the nearest. The cap of a crowded cell and the cap of lineages make bodies of
  different heritage share a brain; the three caps bound the work, not a law. They count per cell, not per m²: a cell
  is the unit of the work. The pair that may mate, a watch, a room and a gift count per cell too: bodies meet who share
  a cell; a law of meeting by the area swept, as the bite has (5.3), would make that per m², and is not built.
- **Choices as a spread of actions** (`choose/spread.rs`) for a node with a brain (5.3).
- **Pace**: every node updates and decides at its cell's step, as an individual does (0).
- **Brain size** (`lineage::resize`): a mean brain-size gene a whole unit off its lineage's starts a new lineage. A
  body with no brain does not mutate its brain-size genes (`body::vary`): they do nothing without one, so a brain
  arises only in a lineage founded with one (decision 72). An
  individual's child joins its parent's lineage, or its partner's when its size genes fit that and not its parent's
  (its rows carried over to that brain; `copy::child_lineage`). A lineage starts only where a body's brain-size gene
  moves a whole unit from its lineage's (`lineage::resize`, at its update) and no living lineage of its kind has the
  sizes it moves to (it joins that one), within `MAX_LINEAGES`. Living: as at the start of the month's pass
  (`Life::lineages_of`), so bodies that move to the same new sizes in one pass found a lineage each, whoever shares
  their chunk.

### 4.5 The cover: light-eaters as a field
`core/src/cover/`. The light-eaters are not bodies: each cell holds up to `SLOTS` (4) kinds of **cover**, each the
amounts of three substances of chemistry 0 per cell (the fuel 5 its light reaction makes, body matter 10 and its walls
13: `cover::Parts`, found from the chemistry's roles) and 9 genes (`cover::LIGHT`, `LEAF`, `WALLS`,
`STORE`, `FROST`, `MOIST`, `SEED`, `REPAIR`, `FIX`; ranges in `cover::RANGE`). Its **living cover** is its body
matter with the liquid of living tissue (matter / (1 − `WET_TISSUE`), as a body's living body, 6.1, `Cover::living`);
its walls are a frame, which pays no upkeep and does not wear (as a body's, 6.6), so its capacity, capture area,
upkeep, wear and fixing go by its living cover, as the maintenance breathing of wood goes by its living cells (Ryan
1990, from memory). The cover of a cell is a node of the
store (`Class::Cover`) whose law books its own side; it is kept with the cell's bodies (`life::Cell::cover`), so the
chunk of the cell is its one writer. Its temperature is its ground's: at the end of each month it trades heat with its
ground to that temperature. **One month** (`cover::step`, its own process after the air's step at a month's start, one
task per chunk), slot by slot, the tallest (most walls per m²) first:
- **Light**: the light the cell caught since its last month per m² of capture area (`World::light`), on its capture
  area (10^`LEAF` m² per kg of living cover) less what it reflects (its parts' colours by volume), shared among its
  kinds by height (Beer's law, `math::shade`); under standing liquid times e^(−depth / `light_depth`).
- **Light reaction** (reaction 0): the share `light_yield` of the light caught is stored, at most its capacity
  10^`LIGHT` × 10^`MOIST` mol per kg of living cover per s, times the reaction's pace k e^(−Ea/T) over its value at
  `t_cat`, times its activity a = min(1, wet / 10^`MOIST`)^`dormancy` (wet: how wet its ground is as bodies feel it,
  d / (d + `wet_depth`), d the depth of its key liquid that is liquid, in its pores and standing, `Physics::wetness`); both the light
  and the capacity limit go with the air's share x of the reaction's gas as (x − `gas_floor`) / (x + 2 `gas_floor`)
  over its value at `gas_ref` (`Physics::gas_factor`; the light-limited rate of Farquhar, von Caemmerer and Berry
  1980): nothing at or below the floor, so the cover cannot strip the air of that gas. All kinds of a cell are cut
  together to the liquid share of its ground's key liquid and the carbon gas of its share of its air node; its
  burnings and fixing take their gas from what that share still offers, so the cover never takes more of the air's
  gas than its cell's share holds (`grow::Books::room`). A cover that works on drier ground works more slowly: its capacity goes with its
  moisture need. The light its bodies' node set aside for it (`Cover::keep`, 1.4) pays first; what it stores beyond
  that comes from its ground, which that light warmed.
- **Upkeep**, paid by burning its fuel (reaction 1) with the air's oxidising gas: per kg of living cover and day
  `c_basal` plus `c_light` × what its capacity could store in a day (dearer by 1 + (melting point of the key liquid −
  `FROST`) / `c_firm` for a cover that stands the cold), plus `c_leaf` per m² of capture area; times its repair factor
  (`Physics::repair`), its activity and the burning's pace at T (seasons come from temperature only). The share unpaid
  dies (`SPENT`). **Wear**: the share 1 − e^−wear dies (`AGE`), wear its upkeep at an activity of at least `dormant`
  over (living cover × `e_life` × 10^`REPAIR`), as bodies wear (6.7). **Frost**: below its `FROST` gene, lowered by its dormancy (`Physics::harden` × (1 − a)), the share
  min(1, (limit − T) / `frost_k`) dies (`PHASE`), active or not.
- **Growth**: the fuel over its reserve (10^`STORE` kg per kg of built cover) and the minerals of its ground (shared
  out as kinds build, tallest first) build body matter and walls (10^`WALLS` kg per kg of matter) by the conversions of growth (`Chemistry::build`,
  `builds`), paying `c_copy` per kg by burning fuel. A fixer (`FIX` above `FIX_OFF`) makes the mineral its growth lacks by the fixing reaction (10), at
  most 10^`FIX` mol per kg of living cover per s at the reaction's pace, paid by the reaction's fuel. The fuel past its reserve that
  it could not build of goes to its ground, as roots exude the sugars a plant cannot use (Jones, Hodge and Kuzyakov
  2004), where the open world's rot takes it.
- **Seeds**: the share `SEED` of what it builds leaves in `seed::LOTS` (8) equal lots, each landing where the dispersal
  kernel draws it, as children do (`copy::lot`, 4.2); the lots of a cell make a packet each (`seed::send`; what lands
  in its own cell or off the map stays there), with one generation's mutation of its genes (noise × each gene's step × `cover_mut`); all of it where one more kg would
  not pay: what a kg more would store with the light at the bottom of its layer (`math::shade`) is less than a kg's
  upkeep, so the slot is **full** (`Cover::full`) and stops thickening where its lowest leaves would not pay for
  themselves (Monsi and Saeki 1953; Hirose 2005). When every cell has had its month the seeds land (`cover::land`),
  cell by cell in the order made (the potential energy of the life that left a cell's head and came to it is booked
  once for each cell and way, `store::life_sent`): they join the most alike slot of their starting kind in the cell (every gene
  within `APART` (0.5) × `alike` steps, `cover::distance`; the genes the mean by built cover; they die if that slot is
  full), else an empty slot; where none is empty, the two most alike slots of one starting kind join to make one
  (`seed::room`), so a kind new to a cell can take hold beside another's variants; else the lightest slot if they
  outweigh its built cover (whose cover dies as remains); else they die as remains. So a variant farther than that from its kind founds a slot of its own and competes with
  the others for light by height. Slots of a cell within the same distance share one (`merge_alike`). Selection is
  between slots and cells: what grows better sends more seeds (the frost gene falls where winters are cold,
  `core/tests/cover.rs`).
- **What it takes and gives**: liquids and minerals from and to its ground with their heat; gases from the air (with
  their heat at the air's temperature and their latent heat) and to its cell's bodies' node (with theirs), as bodies
  breathe; its dead (remains) to its ground as those substances, where the open world's rot runs (reactions 1, 7,
  12). Every move is booked through the cell's view (`store::Moves`), its gas within the cell's share of the air.
- **Eaten and broken**: an animal's take from the place takes from its ground and its cover by one search
  (`feed::take_node`, 5.3), paying to break what it takes of the cover (`Cover::breaking`, as a bite pays); movers that
  push through a cell break the cover by its share of the cell's mass, as they break bodies (6.8, `choose::wear`).
- **What it is to the rest**: its mass, capture area, the light it absorbs and the energy to break it are one more
  body of the cell in the survey's sums (`Life::sum`), so it shades, holds the ground (1.3), hides bodies, slows
  movers and makes lanes (1.7) as bodies do.

## 5. Choose: a small brain sees a little, acts, learns and copies others
Files: `core/src/choose/` (`mod`, `perceive`, `learn`, `feed`, `go`, `spread`), `core/src/brain.rs`.

### 5.1 The brain
One hidden layer over what the body perceives and remembers. Its **base** belongs to a lineage and changes only by
evolution; its **rows** are genes of each body: fixed action scores, shared scorers (per direction, per verb and body
or part seen), the shape of a part and the share of it picked up from the ground, the next memory and a critic. Sizes are genes with costs: hidden units
(0 to 64; 0 is no brain, no decisions, no cost), memory (0 to 12), bodies seen (1 to 6); sight (1 to 4 cells) costs
nothing, as it reads another level of the same features with the same rows (5.2). One brain per body or node (`Body::rows`); a child starts with its born rows plus what its parent
learned (`Life::inherit`). **Evaluated by lineage** (`choose::Minds`): in each cell the bodies with a brain perceive
after the physics of all its bodies and before any acts; the inputs of each lineage's bodies lie in one buffer, and
the sums of their hidden layers are one matrix product with the lineage's base (`brain::sums`); each body then reads its
own rows from its hidden layer (`brain::think`) and learns on its own.

### 5.2 Perception
Local (`choose/perceive.rs`). **Each cell's features** (`choose::cells`, after each survey, at a month's start and at
each sub-step (0), and only while a body lives: nothing else reads them, and a body placed comes between months, before
the survey of its first month; the same for every brain, 18 numbers each scaled to about one): its ground temperature, the depth of its standing liquid, its
surface height, the shade of its cover, the mass per m² of each slot of its cover and of the cover's fuel and body
matter (what most diets eat), the fuel and body matter loose on its ground, the mass per m² of its animals in three
classes of member mass (below 1 kg, to 100 kg, above), their members and mean strength, how hard it is to push
through, its parts and the members that moved in per month (in the last month at a month's start, since the month's
start inside it; the counts per square of the map, so a cell of many squares counts as one of its squares, and the
members that moved in per square's side of its border, for what crosses a border goes with its length). The same
features smoothed over each cell and its neighbours once, twice and three times make what a body of sight 2, 3 and 4
sees of a cell it would step into (the mean by area of the cells about that many links around it, `Life::feat`); a
cell finer than a square of the map counts in that as its square, whose features are its cells' mean by area and whose
neighbours are the squares around it (`perceive::regions`), so what a body sees past the cell it would step into
reaches as far in metres from a fine cell as from a square (sight 1, the cell itself, is one step in either). Cells
coarser than a square (the open water of an island) see by their links. **A brain's inputs**: its own stores against their
minimums, its warmth, the heat it loses (the ln of the heat that left it through its coat in its last sub-step over the
heat its upkeep released, within ±3: above 0 where it burns more to keep warm, the less so inside walls, 3.4, below 0
where it sweats; 6.3), size and the season, its own cell's features, what Bond senses (3.4), its memory. **Each
direction** with a link: the features of the cell it leads to as its sight sees it, the temperature and height as
differences from its own cell's, and how hard its link is to pass (`lanes::resist`). **Bodies**: up to `SEEN` of its
own cell, as they are when it decides (their looks: a member's mass over its own, strength and colour; and their
well-being), from a random place in the cell's list; walls and cover hide them. Up to 3 parts of its cell. Of a part, held or seen, it reads its mass against its own, its walls'
scratch hardness, its hollowness and its edge: ln of its walls' thickness over its own width (`bond::sense`,
`bond::features`), the share an edge would leave of breaking a body like itself (3.3).

### 5.3 Actions
**Planning, one step ahead** (`choose::plan`): before it draws, a body adds to the score of each move the critic's
value (5.4) of the state it would be in in the cell the move leads to (its own state, that cell's features in place of
its own cell's) less its value where it is, which staying keeps, both at its next decision, its cell's step ahead
(`discount` over it, as the critic learns): γ(V(s') − V(s)), times `plan_weight` (1 per unit of well-being); off in a null run.
One action is drawn by the scores at each decision (`choose::act`), paid by its own reactions (`body::pay`) or failed;
a node spreads its members over its `MOST_ACTS` (4) likeliest actions (the moves counted as one; a speed setting) by
their chances brought back to a sum of 1 (`spread::top_chances`), and its brain learns from those chances, so the
gradient's mean is 0; members drawn for no action rest and count in the mean the advantage is measured from
(`choose::spread`). Every body decides at every update, at its cell's step (0). The members of a node that move leave as one group: count × the sum of the chances to go in every
direction, at most all (less than one member: one member with that chance; fewer than one member left behind: the whole node,
with the chance that keeps the mean at count × that share), all in the one direction drawn by its chance over the sum,
so each direction gets its share of members on average, in groups. A
group moves only if the cell it enters has room and it bears the `bump` share of its weight (`choose::go::can_go`);
else it takes food where it is for its step and comes back. **rest**; **move** (`choose::go`:
`walk` per kg and metre, climbing, the bodies in the way, the drag of standing liquid; needs force, 3.5); **take**,
**put**, **give** (`choose/feed.rs`); **watch** (5.5); Bond's form, hold, take from or put to a part, join, break, let go,
enter (`bond/verbs.rs`). A move lands when the batch ends, and an act on a part and a watch are applied then (0).
No copy action, no "farm", "trade" or "attack": those must be patterns of these.

**Moving through liquid** (`Physics::drag`): its members push the standing liquid aside, half the link in each cell, by
the share of their height under it (3.3): per metre, for each member, ½ ρ `DRAG` (s L²) v² over `MUSCLE`, ρ the
liquid's density, s that share, L its size, v its walking speed, `DRAG` 1.05 (a cube face on; Hoerner 1965), `MUSCLE` 0.25 (the
share of what muscles burn that becomes work; Smith, Barclay and Loiselle 2005). At one Froude number it is the same
for every size per kg, 5.2 J a metre in a liquid as dense as bodies, against the walk's 3.0 at 30 kg and 1.0 at a tonne: a body not
built to swim pays several times on land's cost to cross liquid (Williams 1999). Its force must still give the push of
a mover (3.5).

**An act takes time** (`choose::act_as`): the step is the body's time to its next update, and the act drawn takes what
it needs of it. A move takes the walk: its distance at the body's walking speed (`Physics::speed`: every size walks at
one Froude number v² / (g L) = 0.25, L the side of a cube of a member's volume, the dynamic similarity of Alexander and
Jayes 1983; 0.9 m/s at 30 kg, so a link of 2 km takes 40 minutes) times its vigour (3.3): a cold body walks slower,
and through liquid pays the drag of its slower speed. Putting down and letting go take none. Every other
act (rest, give, a bite, a watch, Bond's acts on parts) fills the step (`Body::busy`): what it gives, bites and does
goes by the time it is given. Taking leaves the step to eating. For the rest of its step a body takes food where it
ends by the take law (`feed::take_node`): a mover where it lands (`choose::land_move`, for the time from its landing
to that cell's next sub-step, `World::days_to_phys`, less its walk), one that takes, puts down or lets go, or whose
move fails, where it is: at the decision for its first sub-step of physics, and before each later sub-step for that
one (`feed::meal`; a node's members as the share of them whose act leaves the step to eating, 0). So a monthly mover
and four quarter-month movers walk as far and eat as much, in expectation, and a move gives up an hour of eating, not
a step.
The default is to eat, not to rest, because the take law already holds a body's hunger: it takes nothing when its
stores are full (their room, below) and pays to handle only what it takes, so eating while it does nothing else is
physics, not a choice made for the brain; the brain still chooses what fills the step (rest is its own act, all the
step), and learns from what a move brings where it lands. A node's members share their time by the same rule
(`choose::spread`, 4.4): what a share of its members does with its step, an individual does with that chance.

**Taking** (`feed::intake`, one law for the ground and for bodies) goes by uptake genes (`Body::wanted`) at `c_handle`
per kg, of food only (`Body::uses`: what it burns, the body matter and built solids it digests, the minerals it builds
of; not what it cannot use, nor the key liquids it drinks, 6.4): per kg of supply, Holling's type II take × s /
(take + s), s what the swept area holds (`sweep`) of its food, so thin food takes time to find (Holling 1959). The
minerals it builds of count in s as food does: one search finds all it uses, so rich ground shortens it. **Grazing**
(`feed::take_node`): it reaches what lies within its reach (3.3): under liquid deeper than its height and its reach,
neither the cover nor the solids of the ground, only what is liquid there. A bite of the cover takes all its parts in
the field's proportions; what the body does not use
(the frame without the enzyme of reaction 13) is bulk that fills its gut (all it takes is at most its take × supply)
and passes through it to the ground at once with its heat, giving nothing and never becoming the body's own frame
(a gut passage of days, Van Soest 1994). Another body's
members lie over the cell like the ground's food: a bite reaches those the swept area holds (one individual body
whole, by one draw with the chance the area holds it, whether a node or an individual takes it; of a node, less than one member is one whole member met with that chance:
it takes, tears and pays that meeting's outcome times the chance), limited by force against theirs. **Giving** (`Body::spare`) is of what the other uses, of stores only (no
matter, brood, built solids or key liquid below level), above `keep_min` × their minimum, at most what the members met
could take. The bite, giving and the acts on parts go by the time to the body's next update, its cell's step
(`World::step_days`); taking by the take law by the time to its next meal, its physiological step (0): what it takes
then feeds it until then, as the upkeep of that time is paid from it in the sub-step that follows; a body of a finer
cell searches its smaller cell for a shorter step, so the share of the cell its area holds is the same per m² and per
day. **The room of its stores for a meal** (`Body::room`): it takes up to the level of each store and, of a store it
burns, what its upkeep burns of it for the time the meal feeds it (`body::ahead`, at its temperature and activity now),
as a sequence of short steps each eats what it burnt: so one meal of a week is the meals of shorter ones, not one
store's room.

### 5.4 Learning
**Well-being** (`Body::wellbeing_at`): the energy its stores could release, in months' worth of its starting brain's
thinking, less how near it is to its limits; no reward is hand-shaped. **Within a life** (`choose::learn`,
`brain::learn`): a policy gradient against the critic (actor-critic) on the change of its own well-being since its last
decision, the critic's estimate from now discounted over the time since (`discount` per month, so `discount`^(t /
month)), at its `LEARN` rate, in units of the root of the critic's running mean square of errors, each step over 1 +
the hidden layer's square (the normalised least-mean-squares rule, Nagumo and Noda 1967): the rate is the share of a
row's error a month's step corrects, whatever the scale of the hidden layer or the number of the brain's inputs; that
mean and each row's mean square of steps both weigh the newest value by `learn_beta` per month (1 − (1 − β)^(t / month)).
**Per second**: a body learns alike over a year whatever the pace of its decisions. The critic's error is of the time
(the change of well-being and the discount over it), so the critic corrects the rate's share of it at each decision
(continuous-time TD learning, Doya 2000); the rows' step is the rate times t / month, in units of the spread of the
error per month (the error over t / month, its floor a share per month), so a short step's error is weighed against the
spread of the same rate, not of a longer step's error; the critic's largest step is the rate times t / month too. t is the time since its last decision (`Body::decided`, 0). A node learns
as from one member drawn among those who stayed, with the node's change plus how much more its part's action raised it
than the mean, so its brain moves as an individual's does. **Across lives**: evolution of born rows and base.

### 5.5 Imitation and memory
**Watching** (`choose::learn::watched`) costs the turn; when the batch ends, if both live and the watcher is still of
the lineage it watched in, the watcher's rows move part way (`blend` per month, over its turn: the time since its last decision) toward the watched one's, if the brains have the
same sizes, less the less alike they are, times the share of the watcher's members now who watched (for a node, the
mean by members). **Memory**: the brain's memory numbers (`MEMORY`), which it reads and writes at each decision.
**Signals and the memory of others are parked**: no body emits a signal or remembers whom it dealt with; the brain's
memory numbers are its only memory. **Thinking** (`Physics::think`) costs by brain size and mass^0.25 per month, paid by the
time of each update (6.2), whatever the pace of its decisions. **Null run** (`--null`): no learning or imitation, rows frozen, genes still evolve.

## 6. Bodies
Files: `core/src/body/`, `copy::grow`. Bodies are the eaters (animals); the light-eaters are the cover (4.5).
A body's update is its decision, then its physics over the step ahead in sub-steps of its physiological step, a
week (`World::phys_grid`, `survey::updates`; 0), each with a meal but the first, which its decision gave it.
`body::live` runs a sub-step in stages: drinking, upkeep, heat, store checks, growth and children, shedding. So every
law of this section runs over a week in every cell, whatever the cell's step. Its trades are gathered as moves on its
cell's view and booked when its chunk ends (`store::Moves`, 0), so nodes and individuals take alike, never more than
is there.

### 6.1 What a body is
**Stores** (all members together) and heat. A body holds only the substances of bodies (`Chemistry::body`, one
place each in a fixed array): the liquid and the energy-rich solid of life's roles and every substance growth and the
other built solids take or make that is not a gas at the surface; in chemistry 0 the key liquid (0), the fuel (5), the
body matter (10), the mineral (12) and the built solid (13). It runs only the reactions whose every
substance is one of these or a gas, taking at least one of these; what such a reaction makes that it cannot hold leaves
it as a gas. **Body matter** is what it is built of; its **living
body** (`Body::built`) is matter less brood, over 1 − `WET_TISSUE`. Taking, building, upkeep and force go by the living
body; fuel and extra liquid are carried, and storing fuel costs no work. `Body::size` (with its liquid) sets the
minimums; `Body::mass` (everything) moving and load. **Levels** (`Body::level`): the key liquid h / (1 − h)
of matter (h its keep gene), a built solid its share of matter, others keep gene × kg of living body; a full store is
satiety. **Supply** (`Body::supply`): living body^`intake_exp` from `m_ref` × its pace (6.2). Bodies catch no light:
their capture area is 0.

### 6.2 Upkeep and paying
`body::upkeep` (`body/pay.rs`):

```
tissue = living body × (copy mass / m_ref)^(intake_exp − 1)
upkeep = [ days × tissue × (c_basal + c_strength·STRENGTH + learning) + days / month × thinking × members ]
         × repair (1 − c_repair + c_repair·2^REPAIR) × activity × pace(T)
```

Thinking (`Physics::think`) is a month's cost of the brain: a body pays it by the time, so one that decides every
hour pays over a year what one that decides every month pays (`Config::span`).

Costs go by the kg while supply goes with mass^0.75 (West, Brown and Enquist 1997), so a body grows until keeping what
it has takes all it can supply: its costs set its size, not a cap; across kinds adults pay Kleiber's mass^0.75.
Kleiber's resting rate is a mammal's, its muscles included: a body of strength 1, as muscular as a mammal, pays it
with its basal and strength costs together (`c_basal`, `c_strength`).
**Pace** (`Body::pace`): the bare rate law of its main burning, k e^(−Ea/T), over its value at `t_cat`; supply and
upkeep follow it, so seasons come from temperature, not a calendar: a cold body takes, builds and pays less per day.
Its force and its walking speed follow it below `t_cat` (its vigour, 3.3): a cold body pays little, and is weak and
slow. **Paying** (`body::pay`, `body::burn`): its
burnings run on demand as far as its fuel and the air's gases allow; the gas they take comes from its air node and the
gas they make goes to its cell's bodies' node, each with its heat and latent heat (gas exchange is free: no pores);
the work becomes heat in its cell's ground, and the body keeps its temperature. The unpaid share of upkeep dies
(`SPENT`).

### 6.3 Heat
A body's temperature follows its surroundings' (`live::keep_warm`, `exchange::around`): the heat its surviving members'
upkeep released over the sub-step leaves through its coat (`Body::conductance`) to its ground; inside walls to the air
of its room, which the others there hold above the ground, through its coat in series with the walls (3.4). The heat
ends in its ground either way. Each
member has the surface `meeh` × its mass^(2/3) (Meeh 1879); its coat insulates it by the still air at its surface
(`r_air`) and the share `HEAT` (0 to 1) of the best coat, `r_coat` (fur and the tissue under it: a mammal's least
conductance, Herreid and Kessel 1967); bare, by that air alone (`Body::bare`). **In liquid** the share of its height
under its cell's standing liquid (3.3) passes heat through the liquid's film, `r_liquid` (free convection in liquids,
50 to 1,000 W/(m² K) against about 8 at still air; Incropera and DeWitt 2002), and the share `wet_coat` of its coat,
whose still air the liquid displaces (furs immersed keep about a tenth of their insulation; Scholander et al. 1950):
a body of 30 kg with the founders' coat passes 1.0 W/K in the air and 5.8 W/K swimming, so to keep warm in cold liquid
it burns about six times what it burns in air as cold. Its heat balance over the time dt,
C dT/dt = P - g (T - tg) (Newton's law of cooling; an animal's energy budget, Porter and Gates 1969; `live::Balance`),
moves its temperature from where it is toward where that heat holds it (its surroundings', tg, plus the heat over the
coat's conductance) by the share 1 - e^(-g dt / C) its heat capacity C lets it go in the time: a body's own time C / g is hours
to days, but a large node in a closed room, whose walls pass little, moves only part way in a week. So what it gains
from its ground is never more than the heat it released, unless the ground is the warmer. Below its `KEEP_T` gene it
burns what holds it at `KEEP_T` at the end; above its `COOL_T` gene it opens its coat as far as it needs to stand there,
up to its bare surface (blood to the skin, the coat laid flat), only while its surroundings are colder than it (in
hotter ones an open coat lets their heat in, so it keeps its coat as closed as its gene makes it), and past that
evaporates its key liquid for the excess,
at the liquid's vaporisation enthalpy, never below its safe minimum, drinking as it sweats what its uptake leaves it
for the sub-step (6.4), the vapour going to the bodies' node with its latent heat (`live::hold`); what it sweats and, below, boils off is paid by the heat that reaches it, which its ground gives back, never more than the ground holds above its floor (`live::keep_warm`). Its key liquid boils at its boiling point: past it the heat boils the liquid off, below its
safe minimum too, and holds the body there while it lasts; one that boils dry dies there (`check_stores`), the rest of
its heat staying in its ground. **The temperature it stands at** (`live::standing`): its upkeep over the sub-step goes
with the pace (6.2) of its mean temperature over the time, the mean of that exact solution with the heat the upkeep
releases, held by its genes as above; as that heat goes with the pace, the temperature is a fixed point, found from
where the body is the way its heat moves it (the next equilibrium up or down). So a body of hours to days pays its
sub-step at the temperature it stands at, not the one it started at; the open coat lets the heat of a warmer pace out, so a body's own heat does not drive its pace up for
ever. Its heat then comes to the temperature the balance ends at with its ground (`body::settle_heat`). After a
meal, its decision's or a sub-step's (`live::fade_meal_heat`), what it took and digesting it moved its temperature
by, from where it stood, fades over the time to its next physics (its physiological step) by the share e^(-g dt / C):
the heat of a meal leaves through its coat to its ground; a meal colder than the body is warmed by the body's own heat
as it eats, drawn back from its ground, which its upkeep's heat went to, at most what its upkeep releases over that
time at a mean rate, W: after its decision its last sub-step's (`Lived::released`), after a meal by the take law that
of its upkeep over the meal (`choose::eat_for`; a newborn's or a lander's meal comes before any sub-step of its own). So a body of hours to days starts its next sub-step where it stood. **Two states, one at every step.** As its pace rises with its temperature, a body whose own heat over its
coat warms it past its surroundings by more than T² / Ea (about 12 K for chemistry 0's main burning, Ea 7,760 K: there
the pace's rise, Ea / T² of it per K, feeds the heat faster than the coat lets it out) runs on to a warm state, at most
its `COOL_T`, where its coat opens; below that it lies cold, or at its `KEEP_T` by burning. In a room the walls add to
its coat and the others' heat warms its air, so bodies that share a closed room reach the warm state sooner. Its time C / g is hours to a
day, far less than a sub-step, so each sub-step ends at the stable state its heat leads to from where it stood
(`live::standing`), and as it eats at each sub-step by the take law at the pace it stands at then (5.3), a node that
turns warm in spring eats at the warm pace from the next week on, in a square as in its cells of 500 m. **Known limit:** a node holds one heat state for all its members. Where they sit near the turning point (their
heat over their coat near T² / Ea past their surroundings), what decides it is a few tenths of a kelvin or of a
percent: a node and the same members split over finer cells, which differ that much (the air read bilinearly across a
square, the chance of each cell's births and genes), may settle in different states, the finer form holding a mix. A
young node, which its newborns keep light, sits there for months in a mild season. The general remedy is the split of
a node whose members differ (spec 7.1), here by heat state as maturity splits by size (4.4); it is not built. A node's
children are born through its sub-steps and live from their birth inside their young node, in its heat (4.2). Its
key liquids hold their fusion heat at any temperature (`Body::fused`): a body
has no melting plateau, so its temperature is its heat over its heat capacity, that heat set aside (2.5).

### 6.4 Water: drinking
A body drinks its key liquids from its cell's ground up to the level it keeps, as far as the ground holds them liquid,
each day at most its uptake gene's share of that level, paying `c_handle` per kg; they come with their heat. It drinks
first in its sub-step (`live::drink`), as it drinks while it sweats, builds and gives its children their share over
the time: its upkeep and its building go at the activity (6.5) of what it could drink, not of what its last
sub-step's growth, children and sweat left. What its uptake leaves of the sub-step's days it drinks as it sweats
(`live::evaporate`), so in a sub-step it sweats what it would drink and sweat in shorter ones, not one fill of its
level. It catches no rain and has no roots: its minerals come from what it digests (6.9).

### 6.5 Activity and dormancy
`Body::activity`: a = min(1, l / l*)^`dormancy`, l the key liquid, l* its level. It multiplies upkeep and the building
allowance: a dry body pays almost nothing and can hardly act; it wears at an activity of at least `dormant` (0.02),
so a dormant body lives about 50 times its active life, as dry seeds and spores do. **Frost** (`live::check_stores`): its members
stand in microclimates that spread over `frost_k` (3 K) around the temperature T its heat holds it at over its ground
(6.3), so below a key liquid's melting point
the share f = min(1, (limit − T) / `frost_k`) freezes and dies (`PHASE`) in a month, the limit the melting point
lowered by the body's dormancy, `Physics::harden` × (1 − a) (cold hardening: tissue that stops its chemistry in
the cold stands a lower lethal temperature, Levitt 1980; Sakai and Larcher 1987); one law for bodies and cover
(`Physics::frozen`). f is a share that composes: over
the t its sub-step integrates 1 − (1 − f)^(t / month) (`Span::share64`), so a cold spell kills alike in one sub-step
or in many; an individual dies with that chance. A dormant body
(little free liquid) lives through frosts down to its hardened limit; the cover freezes by the same law below its `FROST`
gene (4.5). **Boiling** kills.

### 6.6 Growth
`copy::grow`. Its fuel and minerals become matter at `c_copy` of work per kg, paid by burning (`body::pay`), as far as
all allow: the **allowance** (`c_build` × supply / `m_ref` × days × activity), the **spare** (what its fuel can release
beyond its upkeep to its next meal, its physiological step), and its stores down to their minimums. Built solids (the frame) come first by their
share (`Body::share`), up to level; they pay no upkeep. **Healing is growth.**

### 6.7 Taking, digesting, minimums, ageing
Intake at most `take` × supply a day, of what lies within its reach (5.3), over the time a meal feeds it (to its next
meal, or for a bite to its next update), within the room of its stores for that time (`Body::room`: their levels and
what its upkeep burns of them until then). **Digestion** (`Body::digest`): matter taken is broken back into what it
was built of, so food enters as fuel, and only if its diet breaks it (`Body::uses`). **Minimums**
(`Body::lacking`): a store below its minimum kills (`LACK`), except a fuel, whose minimum is a reserve. **Shedding**
(`shed`): minerals, built solids and key liquid past level, and anything it neither burns nor is made of. **Ageing**:
wear grows by the energy upkeep turns over per kg, over `e_life` × 10^`REPAIR` (the rate of living); 1 − e^−wear of
the time since dies (`AGE`), in nodes as in individuals. **One rule of death** (`body::live::lose_share`): what kills
a share of a node's members kills an individual with that chance.

### 6.8 Damage
`body/death.rs`. **A bite** (`take_body`) tears from each member reached the share taken and pays to break it (3.6;
less with a held edge, whose force then counts for more, 3.3);
matter breaks first and spills its stores; in a node those members lose it (grazing) or die (`body::wound`). **Pushing
through** (`choose::wear`) breaks what its energy pays for: the energy movers spent in a cell is kept until the next
survey (`life::Cell::pushed`); then each body there, a mover that stayed too, takes its share by mass and loses what that
breaks of it, each of its members alike. So a cell's bodies are worn once a survey, not once for each move into it.
A member dies when `lethal` of its body is torn at once; a hurt adult is
immature until healed. **Starving** (`starved`): a body below its copy mass that did not rebuild what it lost keeps
the share x and dies with chance 1 − x^a, a = ln 2 / −ln(1 − `lethal`), so starving to the lethal loss is survived
with even chances in one update or many (`SPENT`).
Causes: `LACK`, `PHASE`, `SPENT`, `TAKEN`, `BROKEN`, `AGE`; at death all goes to the ground (`body::die`).

### 6.9 Materials and minerals in bodies
Built solids count in mass and breaking, not living body; children get them, gifts never give them. Bodies get
minerals by digestion (the cover from its ground, 4.5); what is past its level it sheds. Fixing (10) runs in the cover
only (4.5).

## 7. Soil and minerals
The minerals (`Chemistry::minerals`) live in the ground's liquid and move with it, so rain washes them to low ground
and the sea and fertility differs by place. **Nitrogen mineral (12)**: placed when life starts (`worldgen::life::soil`)
on land under less than half a metre of liquid; then only fixing adds it, rotting (7) returns it, heat (11) sends it
to the air. Body matter holds no rock element: the **rock salt (11)** is rock chemistry, made in wet cells from lime
(8), turned back by 9, near balance (2.6).

## 8. Starting conditions (not laws)
Placed by `worldgen` (`worldgen::schedule::Schedule`, carried by a snapshot) and printed by every run; the laws never
read a kind.
- **Terrain**: a function in core (`space::terrain`, 0), which `worldgen::terrain` builds the cells on: an island, or a globe with 29% of its area above 0 m (`LAND_SHARE`, the level it finds kept in
  `World::terrain`); a metre of loose ground; the sea's salt at
  balance (2.6); ground and air temperatures falling with latitude and height. The terrain's samples are worn (0):
  land rises from 0 m to the noise's height over 2 million years while flowing liquid cuts it, its slopes creep and it
  floats back up as it is unloaded; the load settles in the closed basins the cutting leaves, and those it does not fill
  are ponds, each with its pass and level. On an island the open water is coarse (`coarse_low`): an air block (8 km) whose squares all lie
  below 0 m, none within a square of land (`LAND_RING`), is one cell, split where it borders finer cells until cells
  across a border differ by one size at most; the coasts stay in 2 km squares. A globe is one cell an air block but in a
  rectangle of air blocks in squares (`--fine`), by the same rule. A cell's height is the terrain's mean over it
  (`Terrain::inside`, on the worn samples, 0), whatever its size.
- **Life** at year 2 (`--life-year`): the cover (`worldgen::life::KINDS` rows of `Place::Field`: kg per m² and genes,
  on every open land cell by its chance), then eaters 20 years later; `--roster` places thirteen kinds (`ROSTER`), six of
  them cover: a herb, a small fast herb, a big slow cover with walls, the same of wet ground (fully active only near
  saturation, more capacity), a woody cover of dry ground (active at a tenth of saturation, thick leaves, slow) and a
  tall fast cover with seeds spread wide. Founders
  are adults on land under at most half a metre of liquid; mass 30% fuel (60% for the two big slow eater kinds).
- **Brains**: eaters start with hand-set biases (`Seeding::hand`; `--neutral-eaters` without), since neutral founders
  die before they learn: they take from the place, its ground and its cover (1), before they bite a body (0.5), and keep
  out of deep liquid (a move's odds fall as 1 / (1 + depth / m)). They score making a part at -10, so they make one only
  by rare chance (a draw in about 1e6 decisions); learning and selection may raise it (the tape's `form_*` columns count
  who draws it). Genes and rows get a spray (`--spray`).
- **Fixers**: a tenth of the cells of the starting cover carry the fixing gene (`FIXERS`, `FIX_GENE`; `--fixers`).
- **Founder genes**: some measured from the laws (`LITTER_X`); bodies build solids at a small share (`BUILT_SHARE`);
  `HEAT`, `CHILD`, `ALLOC` the same for all; `KEEP_T` and `COOL_T` 5 K above their liquid's melting point and 5 K below where life's enzymes fail (320 K) (`EATER_KEEP_T`, `EATER_COOL_T`).

