# The laws as they are now

What each law does, its rule, its physical reason and where it lives. How the code does it (rounds, caches, buffers,
orderings) is said once, in the code's doc comments; the code is the truth. Numbers, units and anchors are in
`docs/numbers.md`, what a run records in `docs/tape.md`.

**Where the code is** (`core/src/`): time in `clock/` and `schedule.rs`; the shape of the world in `space/`, and how
what it keeps per cell follows a change of the cells in `remap.rs`; shared
maths in `solve/` (`sweep`, `downhill`, `line`); nodes and books in `store/`, `world.rs`, `check.rs`; ground heat in
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
east to west; an air block is 4 x 4 squares. The cells are a sparse tree under each air block (`space/tree.rs`): the
block is the root, a cell is a leaf at any depth down to 1/64 of the block's side (`DEPTH_MOST`), and only the leaves
are kept (`Square::leaves`), numbered by their north-west corners row by row; a square is depth 2. Today's worlds start
with cells of 1, 2 or 4 squares on a side (worldgen); no law splits or joins them yet. Cells across a border differ by
one depth at most (2:1). One builder makes every cell (`Square::mesh`). Cells link across a border by its width, as far
apart as their centres across it, so a steady change across a border of two sizes passes exactly; across a corner
(width 0) only where four cells of one size meet, so no cell has more than 8 links. The map's samples are its squares
(`Square::samples`: a cell coarser than a square cut into its squares, a finer one as it is): the air level is built
on them, the same whatever the sizes of the cells, and a cell's starting height is the mean by area of the terrain on
its samples (a cell finer than a square takes the height at its own centre). What needs the cell at a point (the tape's
frames, a byte a square; the pictures) asks the tree for the leaf that holds it (`Tree::cell_at`).

**One step** (`schedule::step`, a month; time in whole seconds, the calendar from it) is a fixed list of passes
(`schedule::month`), each for the whole month: the survey (`survey::survey`: one task per chunk on the pool with the
feature `threads`, in each of its cells the wear of movers, the shade and the sums; a death that lets go of a part
queues that for the serial pass, as in a batch; it rebuilds Flow's
state in full for the cells' surroundings, and Flow's own pass leaves it then); Flow: the air in steps of
`AIR_STEP_DAYS` (15 days, two a month, each at its own time in the month), after each the sink of the liquid over
field capacity into the bed's store; then the liquid and erosion; heat sideways and creep yearly; the cover's month;
Change; Bond (each chunk's cell by cell, the chunks on the pool: in a cell the turns of its parts in the order of its
list, on its view, then the loads of its structures in the order of their roots); then the bodies (`survey::batch`).
**Every body updates once a month**, at the month's start, after the month's Flow, Change and Bond: one batch
holds every living body, and a body placed during it first updates at the next month's start. Each update integrates
the time since the body's last (`Body::last`; of a node, the mean by members, `nodes::rejoin`, so members who joined it
since count less): its upkeep, its brain's cost and learning (5.4), drink, heat and growth go by those days, and what
it takes, gives and does to parts by the step of its update (`Config::update_days`, a month), so the month is a time
step, not a calendar: a season's change comes from temperature (6.2). The bodies
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
are cut into `Config::chunks` runs of neighbouring indices, the same for any number of threads. **The parallel pass**:
each chunk is one task on the pool (`life::run`), its cells taken out of the world, the world read only; in each cell
in order, the physics of each body, in the order they came (a body placed in the pass first updates at the next month);
then every body with a brain perceives its inputs and the bodies of each lineage find their hidden layers together
(5.1); then each decides and acts, in the same order; then the next cell. So what a chunk queues, and the lineages
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
pass** (`life::land`): the queue in rounds: first the moves and the litters land, each chunk of the cells they go to on
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

**The cell table** (`remap.rs`): what the world keeps per cell of the finest level (the store's nodes of a cell and
their sleep, its life, Flow's state of its ground, its moving averages) and per link of it follows a change of the
cells in one place (`World::remap`, between steps): each number moves with its cell or link, an old cell no new one
names is gone, and a new cell holds nothing and takes the next serial, so every id still finds its body or part. No
law changes the cells.
**Sleep and wake** (`clock::Sleep`, a speed tool): a calm node sleeps until an alarm or until an amount grows by more
than `wake_share`, then its law brings it up to date over the whole time since.
**One ground temperature**: every law and the tape read that of Flow's last step (`World::t_ground_last`).
**Checks before a run**: `Config::validate`, `Chemistry::validate` (every reaction keeps its elements and mass).

**The books.** Every move takes from one node exactly what it gives another, with its heat; `store::apply` and the
bookings of the bodies' moves and of Change's rows (`store::book_by_array`, which adds as `apply` adds) are the store's only writers, with plain sums.
Boundary nodes stand for the sun, space and outside (`store::Boundary`, edits by `World::cross`). **Potential
energy** is one function of what the nodes hold (`flow::potential`): a ground cell is a stack from its base up, each
layer at its middle (its bed, its loose solids, its liquid with what is dissolved in it, as its head stacks them,
1.1), its store `bed_depth` under its bed's top, filled from the bottom; an air node's matter lies at the mean head of
its cells, for the height of a column above its ground is in its heat (its heat capacity at constant pressure holds
the work the column does as it rises); a bodies' node's gas and the matter of a cell's life (its bodies, its cover, its
parts) at its cell's head. The parts of a month's Flow after the survey (state, the air's steps, liquid, creep,
refresh) follow each other with nothing between (`flow::chain`) and are booked as one at their end; the survey's own
build of Flow's state, and a part run alone, are booked at their own end. In each booking every move carries its
potential energy from the height it leaves (a ground node's head, an air node's mean head, a store's table, each the
mean of the booking's start and end, so a lake that drains whole takes along what it held), and each node's heat takes
what came in less what left less the change of its own (`store::fall_heat`):
so a fall heats where the matter lands, a lift is paid by the node it enters (the air, for a gas going up; the ground
node a store's liquid rises into), a column that rises as it fills pays for it, and the potential energy the world's
matter gained is one number of the store (`Store::lifted`), the change of that function. Within a month's Flow the later
air steps and the liquid read the heat before the month's lifts and falls are paid. The other laws lift a gas into
the air from its cell's head to its air node's mean ground by the same rule (`World::lift`); a ground node that reacts
(Change) pays what its stack gains, and its gas leaves from its head (`change::booking`). So every element and the
energy are kept;
`check::drift` checks it yearly against `check::DRIFT` (1e-9).
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
year is far below the explicit limit.
**Melting plateaus** (`ground::melting`): a node holds the fusion enthalpy of what is molten in it, so it stays at a
melting point (lowered by solutes, 2.5) until that substance has all frozen or melted. A node whose heat melts or
freezes all of it within a step stays there only for the share of the step that takes, then goes on from that point,
and a soft cover that melts away holds its surface at the melting point only until it is gone (`air::radiation`), so a
long step ends where short ones do. The state of a node is found in full at each month's start; between, it is solved again only on a
plateau or when its temperature leaves the span between the melting points its last solve found, and its surface
(soft cover, reflectance, pores) is found again once it moved by 1% of the depth that hides half of a soft cover with
the liquid standing on it (`land::ground_state`). As matter moves, its heat capacity and fusion enthalpy follow
(`Scratch::note`).

### 1.2 Liquid
`flow::liquid`, once a month, after the air's steps; last, gases dissolve in the liquid (1.5). The sink into the bed's store ends each of the air's steps
(`flow::air`).
- **Pores first**: the loose ground's pores (`porosity`) fill first, under any soft cover; only liquid above them stands
  and has a head (`flow::land::stand`), the depth bodies and Change read.
- **The sea** (`flow::basin`, the low basin): the lowest cell and the cells joined to it through cells whose solid surface lies under
  the level its liquid stands at are the sea, found at each month's start. A connected liquid body has one level: once
  a month, after the runoff has reached it, all the sea's liquid stands at one level, found from all of it and the
  floors of its cells (each solid surface less its pore space, which the liquid fills first; sorted, so the level is
  exact). Each sea cell that holds more than its depth under that level gives the share of its liquid over it, with
  that share of its heat; mixed, it goes to the cells that hold less, by what each lacks. **Known limit:** the sea's
  slope under wind and rotation is not modelled, and what levels it crosses the sea at once. **The sea of each air
  block mixes its heat in a month** (`basin::mix_heat`): then its cells off their melting plateaus, with no soft cover
  or cover of bodies and the same phases, each take the block's temperature (their heat apart from the fusion enthalpy
  each holds, over their heat capacity; their floor's top too, which is part of each cell's ground node).
- **Spill levels**: a priority flood over the ground gives each cell its spill level (Barnes, Lehman and Mulla 2014;
  `downhill::Order`), starting from the sea (from the lowest cell where there is none). Liquid stands below it (or in
  the lowest basin where there is no sea); elsewhere it passes on, counting as no higher than its ground, so it crosses
  the map in one step while depressions fill and spill. Lakes (closed basins above the sea) fill and spill this way.
- **Running off** (`solve::downhill`): each cell but the sea's sends its liquid down its steepest way by D-infinity
  (Tarboton 1997; `Level::steepest`): two neighbours next in compass order make a triangle with the centre, the plane
  through their drops gives its slope and way, and the steepest triangle shares the liquid between its two neighbours
  by angle. To a neighbour already passed whose liquid stands it sends at most what levels the two heads, whatever
  their areas; to one already passed whose liquid passes on (upstream in the flood) it sends none, so a pit over its
  rim spills by its way out. What reaches a sea cell stays there and joins the sea.
- **Pore water** flows by Darcy's law (one explicit `solve::sweep` on the liquid table, the store's too): `seep` ×
  saturated thickness × width / distance, at most half of what would level two cells, and no cell giving more than it
  holds over field capacity; pores at or under field capacity (`field`) give nothing. Over cells of kilometres a
  liquid step moves a table a thousandth of the way to its neighbours', far below the explicit limit. Liquid above it sinks to the deep store
  (`drain::sink`, at most `seep` a second and the store's room) right after each of the air's steps, which bring the
  rain: held until the month's liquid step, it would cover and darken the ground (`land::reflectance`, 1.4) and
  warm it. The store flows by `bed_seep`. Real land drains to channels a few hundred metres apart, so both also drain by Darcy's law down
  the cell's steepest slope over a hillslope of `hill` into the runoff (`flow::drain`). **None of this runs under the
  sea**: its loose ground lies under it with its pores full, and the bed's store under it is not drained, so no liquid
  sinks there, no pore or store flow reaches or leaves a sea cell, and its store feeds no channel.
- **What goes with it**: its share of every liquid and of what is dissolved, with heat. Liquid passing through a node
  keeps its fusion heat whatever the node's phase and leaves at the temperature of the node mixed with what came in
  (on a melting plateau the plateau's substance melts or freezes first), so it never takes more heat than the node
  holds and what stays keeps that temperature. A salt is a liquid that never
  boils (11, 12): it moves with the liquid and stays when it evaporates. Its fall heats the node it lands in (0, "The
  books"), and the head it loses on the way out does work (1.3).

### 1.3 Erosion and creep
The head lost by moving liquid does work where it is lost on land (`flow::liquid::load`, `wear`; below 0 m its load
settles and it wears nothing): it **carries** up to `carry`
m³ of loose solids per J (dropping what is over capacity, so load settles where flow slows), and with the work its
load leaves free (the free share of that capacity: the cover effect, Sklar and Dietrich 2004) **wears** `wear` m³ of
bed per J over its strength into loose ground. **Creep** (`flow::creep`, yearly, out of land cells):
loose ground slides downhill as a slow diffusion, `creep` × softness (1 / (hardness + `hard_min`)) × thickness × slope,
across each border by its width (none across a corner). Bodies hold the ground: carrying
and creep are cut by 1 / (1 + their mass per area / `hold`). Most rock leaving the land is lime dissolved by reaction 8. **Burial**: the loose ground mixes over what lies on it (bioturbation, Darwin 1881), so a part no body holds and no bond joins sinks `bury` m/s into it, no deeper than the loose ground is, and once buried past its smallest outer size it joins its cell's ground (`bond/month.rs::bury`, 3.1).

### 1.4 The air's step, the sun and the bodies' share
`flow::air::step`, every `AIR_STEP_DAYS` (15). First the gas of the bodies' nodes comes up into their air nodes. Then each cell's exchange over the step with the air it sees is solved once (the sea under one air
node as one surface, below), in its own implicit surface balance (Newton's steps about its end temperature, with its melting plateaus, 1.1), at the air of the step's
start: sunlight, heat radiation, sensible heat with the air touching it and what evaporates from it by its saturation
deficit (`air::cells::exchange`). Its slopes with the air it sees come from that same balance, on the branch it ended
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
  λ, the mean over the step's days. The surface reflects as its cover is at the step's start (`land::reflectance`);
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
- **The sea as one surface** (`air::surf`): under each air node the sea's cells that mix as one (1.2) face the air as
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
the air's): wet ground at the open rate, drier ground in proportion to its liquid (Manabe 1969's bucket), taking the
heat it held at the node's temperature; a node over a substance's boiling point boils of it what its heat over that
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
liquid, and gives it up by what each holds of it. **Known limit:** a liquid's whole depth is one mixed layer, so a deep sea takes up gas
in months and warms over centuries; a real one is the other way round.

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
(`Body::heat_with`).

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
**structure** is the parts joined by bonds and moves as one. A part runs no reactions. Monthly (`bond/month.rs`) the
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
reach its temperature and those that warm share what it gives and what the others gave, each by what it wants. A part
whose walls fall under `min_kg` falls apart (`bond::fall_apart`): all it holds, its walls, the liquid in its hollow and its heat, goes to its cell's ground,
whether its walls ran off in its turn or a body took them (`bond::verbs::take`), and it is gone with its bonds; so does a part buried past its smallest outer size (1.3), while a bonded structure stays on top.

### 3.2 Loads and failure
Monthly (`bond/month.rs`), from the part a body holds out along the bonds, each bond carries the weight hanging on it,
less buoyancy, plus the push of moving liquid and of bodies passing (`bump` of their weight); past its strength it
breaks. A structure no body holds lies on the ground and bears no load.
Structures do not drift; only a body carries one (3.5).

### 3.3 Holding, floating
Holding is a bond whose strength is the body's **force**, strength gene × living body × gravity × `grip`
(`bond::bears`); a body holds only while its force bears the structure's weight. Fuel adds no force; a hurt body is
weaker. A body is **afloat** where the liquid is deeper than `deep` and what it and its structure displace weighs as
much as they do (Archimedes): no liquid stands over it (`bond::depth`). No body digs the bed.

**A held edge concentrates force** (`bond/edge.rs`). Without one a body breaks matter bluntly: it gives way through its
whole width (3.6). Pressed with the edge of a held part, only a zone as wide as the edge gives way (Atkins 2009): for
each material k of what is broken, the energy and the force fall to the share c_k = min(1, e / l), e the thickness of
the part's walls (their volume over half its outer surface, `Part::thickness`), l the width of what is broken (a member
of a body, `Body::width`; the cover, its volume per m²; a joint, the root of its contact area), where the edge is
harder than that material (scratch hardness, `Substance::hardness`: an indenter marks only what is softer, Tabor 1951),
else c_k = 1. Over a thing, c̄ = Σ V σ c_k / Σ V σ (`Body::cut_by`, `Cover::cut_by`). A node's part is in one member's
hand, so it leaves the node 1 − (1 − c̄) / n (`bond::edge_leaves`). A bite pays that share of the breaking and its
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
passes the walls (conductivity × face / thickness) and the openings (`leak` per m²), light and rain only the openings.
A body inside exchanges through that link in series with its own surface. "Shelter" is only this low conductance. A
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
A body is **mature** (`Body::adult`) when its living body per member reaches `COPY_MASS`; then growth (6.6) puts the
share `ALLOC` into a **brood** (`Body::brood`), matter held for children that leaves with the body's matter in the same
share (`Body::lose`). Whenever the brood covers a child, children are made (`copy::copy`) whatever the brain does:
bodies copy by physics only, at the body's update (once a month), all that its brood covers then. The brood fills at
every update and
leaves with members who die. A child has a body of 10^`CHILD` × the copy mass,
provisions of the other stores in its parent's proportions (the key liquid × 10^`WET`, so children may be dry and
dormant), its share of heat, and the genes plus noise. A node keeps half a child of brood per member
(`copy::BROOD_HELD`), so its members' broods lie evenly between none and one child, as individuals' do. A share
`DISPERSE` of the children starts in a neighbour cell in a random compass direction (`Level::way`), as far as
`spread_m` reaches (`Level::reach`: always out of a 2 km square, a quarter of the time out of an 8 km one), one for all of a node's children of an
update (for an individual, that chance), where they land when the batch ends (0). Less than one member with no node to
join waits in the brood, or, landed in a neighbour cell, goes back into it.

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
  members, and the spread per gene (`Body::spread`) pools each one's spread and its gap to the new mean; a node keeps
  its born rows, and its row spread (`Body::row_spread`) pools the joiners' spread and their gap to them. Members of a node with a brain that take any action but a move or a
  bond (`choose::spread`) act as the node itself while the others wait aside with their share; these come back by
  `rejoin` alone: their genes and spreads are the node's, so only amounts and sizes (members who fed are now larger
  than those who waited) join; what members who watched copy is applied when the batch ends (5.5).
- **One node per kind and stage** in a cell: one lineage per node (one brain). Children join the young node of their
  lineage in their cell (`copy::kin`), and members who move join the node of their lineage and stage where they land,
  or start one; in a cell of `MAX_NODES` bodies or more with no node of their lineage, they join the first node of
  their starting kind and stage, taking its brain, born rows, lineage and brain-size genes (so the cap below holds
  through the month); a young node matures as a whole when its mean member reaches the copy mass.
- **Merging** (`nodes::crowd`, monthly, in each cell after the batch's serial pass, one task per chunk; a merge that
  lets go of a part queues that, as in a batch): the bodies of one lineage and stage in a cell, in the room of the same
  structure (its root) or none and holding the same structure or nothing (`nodes::Kin`), join the largest of them,
  where that is a node, they are more than `PAIR` (2: a pair of individuals may mate) or their individuals may not mate
  (4.3). Then a node joins a larger
  one of another lineage of its stage, room and part whose genes are alike (`body::alike`, on the means) and whose
  brain has the same sizes: its members take that brain, its born rows and its lineage, and its rows become the mean by
  members (`nodes::merge`); members of another lineage keep the node's brain-size genes, so no merge moves its sizes. This merge is
  a speed tool with physical side effects: the joiners' members drop the structure they held, which stays in the cell
  for any body to take, and a lineage's brain spreads to alike bodies of another. Selection acts between cells and
  lineages, not within a cell.
- **Three caps, speed tools.** After its merge (`nodes::cap`) a cell that holds more than `MAX_NODES` (8) bodies keeps
  one young and one mature node of each starting kind in each room (or outside any): the bodies of one kind, stage and
  room join the largest of them, whatever their lineage, brain sizes and part, taking its brain, born rows, lineage and
  brain-size genes; its rows become the mean by members where the brains have one shape, else they stay its own. In any
  other cell a body with fewer members than `node_floor` (1%) of the largest body of its lineage, stage and room joins
  that one, unless it may mate (`copy::mates`, 4.3). Either way members, stores, heat and brood join as sums, and a part
  the joiner held stays in its cell, held by none. So a small group (a few members who took a part) does not cost an
  update of its own, while a pair that may mate and a lineage just founded stay apart; a crowded cell holds at most two
  bodies per kind present and room, and kinds never mix. A starting kind keeps at most `MAX_LINEAGES` (8) living
  lineages (`lineage::resize`): past them a body whose sizes change joins the living lineage of its kind nearest in
  sizes, and stays in its own if that is the nearest. The cap of a crowded cell and the cap of lineages make bodies of
  different heritage share a brain; the three caps bound the work, not a law.
- **Choices as a spread of actions** (`choose/spread.rs`) for a node with a brain (5.3).
- **Pace**: every node updates and decides once a month, as an individual does (0).
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
`STORE`, `FROST`, `MOIST`, `SEED`, `REPAIR`, `FIX`; ranges in `cover::RANGE`). The cover of a cell is a node of the
store (`Class::Cover`) whose law books its own side; it is kept with the cell's bodies (`life::Cell::cover`), so the
chunk of the cell is its one writer. Its temperature is its ground's: at the end of each month it trades heat with its
ground to that temperature. **One month** (`cover::step`, its own process after the air's step at a month's start, one
task per chunk), slot by slot, the tallest (most walls per m²) first:
- **Light**: the light the cell caught since its last month per m² of capture area (`World::light`), on its capture
  area (10^`LEAF` m² per kg of built cover) less what it reflects (its parts' colours by volume), shared among its
  kinds by height (Beer's law, `math::shade`); under standing liquid times e^(−depth / `light_depth`).
- **Light reaction** (reaction 0): the share `light_yield` of the light caught is stored, at most its capacity
  10^`LIGHT` × 10^`MOIST` mol per kg of built cover per s, times the reaction's pace k e^(−Ea/T) over its value at
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
- **Upkeep**, paid by burning its fuel (reaction 1) with the air's oxidising gas: per kg of built cover and day
  `c_basal` plus `c_light` × what its capacity could store in a day (dearer by 1 + (melting point of the key liquid −
  `FROST`) / `c_firm` for a cover that stands the cold), plus `c_leaf` per m² of capture area; times its repair factor
  (`Physics::repair`), its activity and the burning's pace at T (seasons come from temperature only). The share unpaid
  dies (`SPENT`). **Wear**: the share 1 − e^−wear dies (`AGE`), wear its upkeep at an activity of at least `dormant`
  over (built cover × `e_life` × 10^`REPAIR`), as bodies wear (6.7). **Frost**: below its `FROST` gene the share min(1, (`FROST` − T) / `frost_k`) × a
  dies (`PHASE`).
- **Growth**: the fuel over its reserve (10^`STORE` kg per kg of built cover) and the minerals of its ground (shared
  out as kinds build, tallest first) build body matter and walls (10^`WALLS` kg per kg of matter) by the conversions of growth (`Chemistry::build`,
  `builds`), paying `c_copy` per kg by burning fuel. A fixer (`FIX` above `FIX_OFF`) makes the mineral its growth lacks by the fixing reaction (10), at
  most 10^`FIX` mol per kg per s at the reaction's pace, paid by the reaction's fuel. The fuel past its reserve that
  it could not build of goes to its ground, as roots exude the sugars a plant cannot use (Jones, Hodge and Kuzyakov
  2004), where the open world's rot takes it.
- **Seeds**: the share `SEED` of what it builds leaves for the neighbour cell of a random compass direction, as far
  as `spread_m` reaches (`Level::reach`; or stays where there is none), with one generation's mutation of its genes (noise × each gene's step × `cover_mut`); all of it where one more kg would
  not pay: what a kg more would store with the light at the bottom of its layer (`math::shade`) is less than a kg's
  upkeep, so the slot is **full** (`Cover::full`) and stops thickening where its lowest leaves would not pay for
  themselves (Monsi and Saeki 1953; Hirose 2005). When every cell has had its month the seeds land (`cover::land`),
  cell by cell in the order made: they join the most alike slot of their starting kind in the cell (every gene
  within `APART` (0.5) × `alike` steps, `cover::distance`; the genes the mean by built cover; they die if that slot is
  full), else an empty slot, else the lightest slot if they outweigh its built cover (whose cover dies as remains);
  else they die as remains. So a variant farther than that from its kind founds a slot of its own and competes with
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
Local (`choose/perceive.rs`). **Each cell's features** (`choose::cells`, once a month after the survey, the same for
every brain, 18 numbers each scaled to about one): its ground temperature, the depth of its standing liquid, its
surface height, the shade of its cover, the mass per m² of each slot of its cover and of the cover's fuel and body
matter (what most diets eat), the fuel and body matter loose on its ground, the mass per m² of its animals in three
classes of member mass (below 1 kg, to 100 kg, above), their members and mean strength, how hard it is to push
through, its parts and the members that moved in last month (the counts per square of the map, so a cell of many squares counts as one of its squares). The same
features smoothed over each cell and its neighbours once, twice and three times make what a body of sight 2, 3 and 4
sees of a cell it would step into (the mean by area of the cells about that many links around it, `Life::feat`). **A brain's inputs**: its own stores against their
minimums, its warmth, size and the season, its own cell's features, what Bond senses (3.4), its memory. **Each
direction** with a link: the features of the cell it leads to as its sight sees it, the temperature and height as
differences from its own cell's, and how hard its link is to pass (`lanes::resist`). **Bodies**: up to `SEEN` of its
own cell, as they are when it decides (their looks: a member's mass over its own, strength and colour; and their
well-being), from a random place in the cell's list; walls and cover hide them. Up to 3 parts of its cell. Of a part, held or seen, it reads its mass against its own, its walls'
scratch hardness, its hollowness and its edge: ln of its walls' thickness over its own width (`bond::sense`,
`bond::features`), the share an edge would leave of breaking a body like itself (3.3).

### 5.3 Actions
**Planning, one step ahead** (`choose::plan`): before it draws, a body adds to the score of each move the critic's
value (5.4) of the state it would be in in the cell the move leads to (its own state, that cell's features in place of
its own cell's) less its value where it is, which staying keeps, both at its next decision, a step of its update
ahead (`discount` over it, as the critic learns): γ(V(s') − V(s)), times `plan_weight` (1 per unit of well-being); off in a null run.
One action is drawn by the scores at each decision (`choose::act`), paid by its own reactions (`body::pay`) or failed;
a node spreads its members over its `MOST_ACTS` (4) likeliest actions (the moves counted as one; a speed setting) by
their chances brought back to a sum of 1 (`spread::top_chances`), and its brain learns from those chances, so the
gradient's mean is 0; members drawn for no action rest and count in the mean the advantage is measured from
(`choose::spread`). Every body decides at every update, once a
month. The members of a node that move leave as one group: count × the sum of the chances to go in every
direction, at most all (less than one member: one member with that chance; fewer than one member left behind: the whole node,
with the chance that keeps the mean at count × that share), all in the one direction drawn by its chance over the sum,
so each direction gets its share of members on average, in groups. A
group splits off only if the cell it enters has room and the node bears the `bump` share of its weight
(`choose::go::can_go`). **rest**; **move** (`choose::go`:
`walk` per kg and metre, climbing, the bodies in the way; needs force, 3.5); **take**, **put**, **give**
(`choose/feed.rs`); **watch** (5.5); Bond's form, hold, take from or put to a part, join, break, let go,
enter (`bond/verbs.rs`). A move lands when the batch ends, and an act on a part and a watch are applied then (0).
No copy action, no "farm", "trade" or "attack": those must be patterns of these.

**Taking** (`feed::intake`, one law for the ground and for bodies) goes by uptake genes (`Body::wanted`) at `c_handle`
per kg, of food only (`Body::uses`: what it burns, the body matter and built solids it digests, the minerals it builds
of; not what it cannot use, nor the key liquids it drinks, 6.4): per kg of supply, Holling's type II take × s /
(take + s), s what the swept area holds (`sweep`) of its food, so thin food takes time to find (Holling 1959). The
minerals it builds of count in s as food does: one search finds all it uses, so rich ground shortens it. **Grazing**
(`feed::take_node`): a bite of the cover takes all its parts in the field's proportions; what the body does not use
(the frame without the enzyme of reaction 13) is bulk that fills its gut (all it takes is at most its take × supply)
and passes through it to the ground at once with its heat, giving nothing and never becoming the body's own frame
(a gut passage of days, Van Soest 1994). Another body's
members lie over the cell like the ground's food: a bite reaches those the swept area holds (one individual body
whole, by one draw with the chance the area holds it, whether a node or an individual takes it; of a node, less than one member is one whole member met with that chance:
it takes, tears and pays that meeting's outcome times the chance), limited by force against theirs. **Giving** (`Body::spare`) is of what the other uses, of stores only (no
matter, brood, built solids or key liquid below level), above `keep_min` × their minimum, at most what the members met
could take.

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
(continuous-time TD learning, Doya 2000); the rows' step, in units of that error's spread, is the rate times t / month,
as are the floor of the spread and the critic's largest step. t is the time its update integrated (0). A node learns
as from one member drawn among those who stayed, with the node's change plus how much more its part's action raised it
than the mean, so its brain moves as an individual's does. **Across lives**: evolution of born rows and base.

### 5.5 Imitation and memory
**Watching** (`choose::learn::watched`) costs the turn; when the batch ends, if both live and the watcher is still of
the lineage it watched in, the watcher's rows move part way (`blend` per month, over its turn: a step of its update) toward the watched one's, if the brains have the
same sizes, less the less alike they are, times the share of the watcher's members now who watched (for a node, the
mean by members). **Memory**: the brain's memory numbers (`MEMORY`), which it reads and writes at each decision.
**Signals and the memory of others are parked**: no body emits a signal or remembers whom it dealt with; the brain's
memory numbers are its only memory. **Thinking** (`Physics::think`) costs by brain size and mass^0.25 per month, paid by the
time of each update (6.2), whatever the pace of its decisions. **Null run** (`--null`): no learning or imitation, rows frozen, genes still evolve.

## 6. Bodies
Files: `core/src/body/`, `copy::grow`. Bodies are the eaters (animals); the light-eaters are the cover (4.5).
`body::live` runs an update in stages: upkeep, drinking, heat, store checks, growth and children, shedding; then the
decision. Its trades are gathered as moves on its cell's view and booked when its chunk ends (`store::Moves`, 0), so
nodes and individuals take alike, never more than is there.

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
**Pace** (`Body::pace`): the bare rate law of its main burning, k e^(−Ea/T), over its value at `t_cat`; supply and
upkeep follow it, so seasons come from temperature, not a calendar. **Paying** (`body::pay`, `body::burn`): its
burnings run on demand as far as its fuel and the air's gases allow; the gas they take comes from its air node and the
gas they make goes to its cell's bodies' node, each with its heat and latent heat (gas exchange is free: no pores);
the work becomes heat in its cell's ground, and the body keeps its temperature. The unpaid share of upkeep dies
(`SPENT`).

### 6.3 Heat
A body's temperature follows its ground's (`live::keep_warm`): the heat its surviving members' upkeep released over
the update leaves through its coat (`Body::conductance`; in series with the walls around it, 3.4) to the ground. Each
member has the surface `meeh` × its mass^(2/3) (Meeh 1879); its coat insulates it by the still air at its surface
(`r_air`) and the share `HEAT` (0 to 1) of the best coat, `r_coat` (fur and the tissue under it: a mammal's least
conductance, Herreid and Kessel 1967); bare, by that air alone (`Body::bare`). Its temperature moves from where it is
toward where that heat holds it (the ground's, plus the heat over the coat's conductance) by the share 1 - e^(-g dt /
C) its heat capacity C lets it go in the time: a body's own time C / g is hours to days, but a large node in a closed
room, whose walls pass little, moves only part way in a month. So what it gains from its ground is never more than the
heat it released, unless the ground is the warmer. Below its `KEEP_T` gene it burns what holds it at `KEEP_T` at the
end; above its `COOL_T` gene it opens its coat as far as it needs to stand there, up to its bare surface (blood to the
skin, the coat laid flat), and past that evaporates its key liquid for the excess, at the liquid's vaporisation
enthalpy, never below its safe minimum, the vapour going to the bodies' node with its latent heat. Its key liquid boils
at its boiling point: past it the heat boils the liquid off, below its safe minimum too, and holds the body there while
it lasts; one that boils dry dies there (`check_stores`), the rest of its heat staying in its ground. Its upkeep goes with
the pace of its temperature (6.2), so a warmer body makes more heat; the open coat lets that heat out, so a body's own
heat does not drive its pace up month after month. Its heat then comes to that temperature with its ground
(`body::settle_heat`). After its decision (`live::shed_meal_heat`) the heat of what it took and of digesting it, above
the temperature it stood at, leaves through its coat to its ground over the time to its next update by the same share
(a body of hours to days sheds it all); a body its intake cooled stays cooler, drawing no heat from its ground, and
its own heat warms it at its next update. Its key liquids hold their fusion heat at any temperature (`Body::fused`):
a body has no melting plateau, so its temperature is its heat over its heat capacity, that heat set aside (2.5).

### 6.4 Water: drinking
A body drinks its key liquids from its cell's ground up to the level it keeps, as far as the ground holds them liquid,
each day at most its uptake gene's share of that level, paying `c_handle` per kg; they come with their heat. It
catches no rain and has no roots: its minerals come from what it digests (6.9).

### 6.5 Activity and dormancy
`Body::activity`: a = min(1, l / l*)^`dormancy`, l the key liquid, l* its level. It multiplies upkeep and the building
allowance: a dry body pays almost nothing and can hardly act; it wears at an activity of at least `dormant` (0.02),
so a dormant body lives about 50 times its active life, as dry seeds and spores do. **Frost** (`live::check_stores`): its members
stand in microclimates that spread over `frost_k` (3 K) around the temperature T its heat holds it at over its ground
(6.3), so below a key liquid's melting point
the share min(1, (melt − T) / `frost_k`) × a freezes and dies (`PHASE`), an individual with that chance: a dormant body
(little free liquid) lives through frost, as seeds and spores do; the cover freezes by the same law below its `FROST`
gene (4.5). **Boiling** kills.

### 6.6 Growth
`copy::grow`. Its fuel and minerals become matter at `c_copy` of work per kg, paid by burning (`body::pay`), as far as
all allow: the **allowance** (`c_build` × supply / `m_ref` × days × activity), the **spare** (what its fuel can release
beyond a month's upkeep), and its stores down to their minimums. Built solids (the frame) come first by their
share (`Body::share`), up to level; they pay no upkeep. **Healing is growth.**

### 6.7 Taking, digesting, minimums, ageing
Intake at most `take` × supply a day (5.3). **Digestion** (`Body::digest`): matter taken is broken back into what it
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
- **Terrain** (`worldgen::terrain`): an island, or a globe with 29% of its area above 0 m (`LAND_SHARE`); a metre of
  loose ground; the sea's salt at balance (2.6); ground and air temperatures falling with latitude and height. Every
  land cell drains from the start (`drain`): a priority flood raises closed basins (only) to their outlet at 1 m per km
  (`DRAIN_SLOPE`; Planchon and Darboux 2001), from every cell below 0 m on a globe, from those joined to the lowest
  cell on an island. On an island the open water is coarse (`coarse_low`): an air block (8 km) whose squares all lie
  below 0 m, none within a square of land (`LAND_RING`), is one cell, split where it borders finer cells until cells
  across a border differ by one size at most; the coasts stay in 2 km squares. A globe is one cell an air block but in a
  rectangle of air blocks in squares (`--fine`), by the same rule. A cell of many squares has the mean height
  of its squares by area.
- **Life** at year 2 (`--life-year`): the cover (`worldgen::life::KINDS` rows of `Place::Field`: kg per m² and genes,
  on every open land cell by its chance), then eaters 20 years later; `--roster` places ten kinds (`ROSTER`). Founders
  are adults on land under at most half a metre of liquid; mass 30% fuel (60% for the two big slow eater kinds).
- **Brains**: eaters start with hand-set biases (`Seeding::hand`; `--neutral-eaters` without), since neutral founders
  die before they learn: they take from the place, its ground and its cover (1), before they bite a body (0.5), and keep
  out of deep liquid (a move's odds fall as 1 / (1 + depth / m)). Genes and rows get a spray (`--spray`).
- **Fixers**: a tenth of the cells of the starting cover carry the fixing gene (`FIXERS`, `FIX_GENE`; `--fixers`).
- **Founder genes**: some measured from the laws (`LITTER_X`); bodies build solids at a small share (`BUILT_SHARE`);
  `HEAT`, `CHILD`, `ALLOC` the same for all; `KEEP_T` and `COOL_T` 5 K above their liquid's melting point and 5 K below where life's enzymes fail (320 K) (`EATER_KEEP_T`, `EATER_COOL_T`).

