# The laws as they are now

What each law does, its rule, its physical reason and where it lives. How the code does it (rounds, caches, buffers,
orderings) is said once, in the code's doc comments; the code is the truth. Numbers, units and anchors are in
`docs/numbers.md`, what a run records in `docs/tape.md`. Names in backticks are code names under `core/src/` unless a
crate is named.

**Where the code is** (`core/src/`): time in `clock/` and `schedule.rs`; the shape of the world in `space/`; how a
cell's means lie over its own ground in `inside/`; shared maths in `solve/` (`sweep`, `line`); nodes and books in
`store/`, `world.rs`, `check.rs`; ground heat in `ground.rs`; settings in `params.rs`, constants in `units.rs`; the
cover in `cover/`; the life of each cell and its chunks in `life/`; what lies between the natural world and its takers
in `contract/`; keyed random numbers in `rng.rs`; the tape's events in `event.rs`; threads in `par.rs` (the same
results on any count of threads).

**Words.** A **cell** is a square of the map (2 km on the island, latitude-longitude on a globe) or a block of 2 x 2
or 4 x 4 of them. A **node** is anything that holds stuff: a cell's ground, an air column, a cell's cover. A **gene** is
a number a kind of cover inherits, often log10 (1 means ×10). The **key liquid** is the liquid life's chemistry runs
in (substance 0). **Light-eaters** (plant-like) are the **cover** of each cell, a field (3). The **open low ground**
is the sea: what lies under 0 m joined to the map's lowest sample (on a globe, all that lies under 0 m).

## 0. The world, one step, the books

**The map** (`space/`, `World::mesh`): cells, links with widths and headings, and a coarser air level. The laws step
only on links; each link takes one of 8 compass directions (`Level::way`). A map is an island of 2 km squares, its rows
spread evenly from `Config::lat_south` to `lat_north`, or a globe (`--globe`) of latitude-longitude squares whose rows
wrap east to west. An air block is 4 x 4 squares. The cells are the leaves of a quadtree under each air block
(`space/tree.rs`; only the leaves are kept, `Square::leaves`, placed in 64-bit integers of `DEPTH_MOST`); a square is
depth 2. A world's cells are made once: squares on land, cells of 1, 2 or 4 squares a side in the open low ground (on a
globe, squares in the window `--fine`, one cell per air block elsewhere). Cells across a border differ by one depth at
most; one builder makes them all (`Square::mesh`). Cells link across a border by its width, as far apart as their
centres across it, so a steady change passes exactly between two sizes. Across a corner (width 0) two cells link if
they are one depth apart at most and neither has 8 links; heat, pore liquid and creep pass none across a corner (width
0), the downhill path of surface liquid may use all 8 ways. A cell's height is the terrain's mean over it
(`Terrain::inside`).

**The terrain** (`space::terrain`) is one function of the seed and a point, at any size: value noise over octaves,
with ridges, its domain warped (Perlin 1985; Musgrave 1989). On an island a unit of the noise has a fixed length
(`ANCHOR_M` / `ACROSS`), so its coarsest valleys lie 20 to 30 km apart, as across a real range (Hovius 1996); a larger
map holds more hills, not larger ones. The world keeps its seed, its map and `World::terrain` (the rise at 0 m,
`ISLAND_LEVEL` on an island, and the island's shape).

- **Rises** (`space::rises`, `Lift`): an island's noise falls with the distance from the nearest rise, so land stands
  round each rise. `Shape::Drawn` (the default) draws one to several rises: their count by the number-area rule of
  islands, N(>A) ~ A^-B (Korčák 1938; Mandelbrot 1975; `NUMBER_AREA`), each a dome or rarely a ring round a sunken block
  (`RING_SHARE`, most breached, `BREACHED`), long, tilted and lifted by drawn amounts, the land's share drawn in
  `SHARE`. Between two rises that raise the land alike the crust sags (`DIVIDE`): a strait. `Shape::One` (`--shape
  one`, the tests' worlds) has one rise in the middle.
- **Ranges** (`Range`): along a drawn axis of each rise the land rises fastest (`RISES`): a rough range, foothills, a
  low plain; the octaves from `FINE_FROM` on keep their size on a plain. A globe's land rises alike everywhere.
- **The samples** are the leaves of `SAMPLE_DEPTH` (125 m on the island), worn (below), found from the seed when first
  read (`space::samples`), not saved.

**The wear at the world's start** (`space::wear`, in a fixed serial order in 64-bit numbers, so a seed gives one
ground on every machine). The sea stands at `LOW_STAND_M` (Fairbanks 1989). Land rises from it to the noise's height
over `WEAR_YEARS` while flowing liquid cuts it by the stream power law (Howard 1994; Whipple and Tucker 1999): it lowers
by K √A S, A the area draining through a sample (spread by Quinn et al. 1991's weights), S its slope to where it runs.
Each step is solved exactly up the ways, h = (h₀ + f h_down) / (1 + f), f = K dt √A / d (Braun and Willett 2013), so
any step is stable. Under a closed basin's level the liquid does not cut, and what comes in fills the basin from its
floor up; the rest runs on over its pass (`space::ponds`). Slopes creep (`CREEP`; Culling 1960), and the ground floats
back up by `FLOATS` of what was removed, spread over `BENDS_M` (Airy isostasy; Molnar and England 1990;
`space::slopes`). Time runs on grids from coarse to fine (`STAGES`, `MOST_SAMPLES`), each carried to the next by the
lean of `space::below`. At the end the sea rises to 0 m and drowns the valleys cut toward the low stand.

- **The bed's makes** (`space::strata`): a few makes, each a mix of the chemistry's bed substances (`makes`), each cut
  faster than the first by its substances' shares over their bond strengths, as Flow's wear breaks it (`Strata::cut`).
  They lie in folded layers (`LAYERS`, `THICK_M`, `FOLD_M`), with bodies of the hardest make standing through them
  (`BODY_ABOVE`), thin seams (`Lies::Seam`) and rims round some bodies (`Lies::Rim`). How deep the wear has cut, plus
  the fold, says which make a sample's surface is in (`Strata::make`), and the wear cuts at its K: a soft layer opens a
  vale, a hard one stands as a scarp. A cell's bed is its samples' makes by area.
- **The shelf** (`space::shelf`): the load that reaches the sea is spread along the shore (`ALONG_M`) and builds a
  platform `TOPSET_M` under the low stand, as wide as the load needs, fronted by a slope of `FORESET` (Gilbert 1885).
  After the sea rises it is a drowned shelf. What the wear took off the land lies in the basins, on the shelf and off
  the map (`Samples::books`, `wear::Books`).
- **The high stand** (`space::stand`): the sea then stands at 0 m for `HIGH_STAND_YEARS` (Fairbanks 1989; Stanley and
  Warne 1994). Each mouth brings its catchment's yield, Qs = ω Q^0.31 A^0.5 R T (`YIELD_OMEGA` and the rest; Syvitski
  and Milliman 2007): Q from `RUNOFF_M`, A its area, R its highest ground, T the surface temperature over 273.15 K. The
  catchment loses that ground where the wear lowered it fastest, never under where it runs to.
- **The lobes** (`space::delta`, `space::fill`, `space::hollows`), largest load first: a mouth fills the liquid from
  its channel's entry, in order of least cost (Dijkstra), each sample up to the surface of the channel's flood
  (`Terrain::channel` carrying `FLOOD_RUNOFF`; Leopold and Maddock 1953). A step costs more in deeper liquid
  (`DEPTH_COST_M`), more away from a coast open to the waves (`ALONG_COAST`; openness is the fetch over `FETCH_REF_M`,
  Håkanson 1981), less on along a sheltered channel (`DOWN_CHANNEL`) (Wright and Coleman 1973; Galloway 1975). A front
  falls at `FORESET`. Hollows a lobe cuts off are filled to their spill level (Barnes, Lehman and Mulla 2014). What found
  no room is `Books::unlaid`, what catchments held back `Books::held`. Loose ground is as old as the mean of its layers
  (`Samples::age`). **Known limit:** the volumes ignore the load's pores.
- **The loose ground** (`space::mantle`, `Samples::loose`): on land, the depth where the bed's breaking into loose
  ground (`BREAKS`, e times slower under each `BREAKS_UNDER_M`; Heimsath, Dietrich, Nishiizumi and Finkel 1997) balances
  what takes it away, the smaller of two balances: against the lowering E of the last step, `BREAKS_UNDER_M` ln(`BREAKS`
  / E) plus what settled (none where E outruns the breaking, at most what a lowering of `SLOWEST` of the breaking
  keeps); against creep down slope S, a depth H where H `CREEP` S / (1 − (S / `REST`)²) / `CREEP_DEPTH_M` =
  a `BREAKS` exp(−H / `BREAKS_UNDER_M`) + what settled, a the area draining per m of width (Roering, Kirchner and
  Dietrich 1999); none at `REST` or steeper. So steep slopes are bare and their feet
  hold what they shed. On the open low ground: the shelf over `OOZE_M` of fine load.
- **The flood** (`space::drain`) from all the open low ground does not raise the ground (Barnes, Lehman and Mulla
  2014): it reaches samples by the level their liquid stands to, lowest first. Each sample runs to a neighbour reached
  before it that the surface falls to, drawn in proportion to the fall (Fairfield and Leymarie 1991), so ways follow the
  mean fall, not the grid; across a corner only where both side samples lie lower. Under a basin's level and on a plain
  the surface rises along the flood's way by `FLOOD_RISE` per m, so ways in a basin run to its pass (Planchon and
  Darboux 2001; Cordonnier, Bovy and Braun 2019). A basin under `GRADED_M` deep is filled. The samples of a basin under
  its level are **a pond** (`Samples::pond`, `Samples::ponds`): its pass is the first sample out on its liquid's way,
  its level the pass's height; the parts of the open low ground are ponds of no pass.
- **The channels** (`space::network`): the area draining through a sample is its own and all that runs to it; the
  runoff runs down these ways (1.2). The wetness index is found with them (`space::samples::sample_index`).
- **Facing the sun**: a cell's mean rise toward the south over its samples (`Samples::rise_south`), held in
  `World::rise_south`, found again at each build (`World::rise_at_build`); the sun reads it (1.4).
- **Below the samples** (`space::below`, `Terrain::heights_on`): a leaf's quarters depart from its mean by the quartic
  through it and two neighbours each way (`LEAN`; average interpolation, Donoho 1993), which keeps the mean, so the
  ground has no steps or kinks; the wear's grids scale the lean down where it would make a new pit or peak (Barth and
  Jespersen 1989). Nothing random is laid on it. A shell finds the leaf at a point by `Tree::cell_at`.

**One step** (`schedule::step`, a month; time in whole seconds) is a fixed list of passes (`schedule.rs` `fn month`),
each for the whole month:

1. The month's start (`schedule::START`; alone, `month_start`): what stands on each cell (`contract::stands`); the
   surroundings (`contract::surroundings`: Flow's state built in full and each cell's place from it); each cell's soil
   from its ground (`soil::month`).
2. At a year's start every `build_years`: the build of the samples (`flow::build`, 1.3).
3. Flow (`flow::chain` books its parts as one): the state; the air in steps of `AIR_STEP_DAYS` (two a month), each at
   its own time in the month and each followed by the sink into the bed's store; the liquid; heat sideways and creep
   every 12 months (`CREEP_MONTHS`); the recheck of slow ground every 3 months (`flow::refresh`, `REFRESH_MONTHS`).
4. The month of life (`life::month`): where the soil's liquid saturates each cell's land (`contract::wet_grounds`);
   the ways to liquid where takers live (`contract::ways_to_liquid`); each cell's cover, then its takers, on the chunks;
   the flames (`flame::month`, 3c); the seeds and what takers sent land.
5. Change (`change::step`).

In debug builds each pass is followed by `check::broken`.

**Edits and their settling** (`core/src/edit/`). Edits are made between two months (3, **Edits**); `Edit::valid`
refuses one whose place or numbers are not finite, or whose brush's radius is under none. Each acts through the law it
belongs to; matter and heat an edit adds or removes come from or go to the outside node (`World::cross`), with the
potential energy of where they lie, so the books close.

- **Ground under a brush** (`edit::Act::Ground`, `World::shape`, `edit/shape.rs`): raised or lowered by (1 − (d/r)²)²
  of its height at the centre. A raise lays the cell's bed rock and, at a stroke's top, `edit_top_m` of loose ground of
  one terrain make drawn for the stroke (`Make::of`, `World::draw_make`); a lowering takes loose ground first, then
  rock, never more than the cell holds. Where in a cell the ground moved is kept on its samples (`Relief::shaped`).
- **It comes to rest** (`World::rest`, `World::slid`, `space::slide`): one paint's edits are laid in layers no higher
  than `LAYER` of the radius, the ground resting after each, so a held brush raises the same hill at any pace. First
  what edits moved spreads as creep would (`slide::spread`, `SPREAD`, out to `SPREAD_RADII` radii); only edited height
  spreads, so the old relief stays. Then a sample steeper than `REST` (or than it stood before any edit, if steeper)
  slides to its lower neighbours until it is `SLID_UNDER_DEG` gentler (a pile rests a little under its failing angle;
  `slide::fails`, `slide::slide`, at most `SLIDE_PASSES` passes, volume kept). A slope is the upwind steepest way
  east-west and north-south (as in the eikonal equation), so a rest is round. What a cell takes in is laid as loose
  ground of the stroke's make. A basin's ground does not slide.
- **A basin of liquid** (`edit::Act::Basin`, `World::basin`, `edit/basin.rs`): its level is the lowest level of the
  closed ponds it reaches, else its rim's outlet. The ground under the brush, and of a pond it joins, is cut to a bowl,
  depth × (1 − (d/r)²) / (1 − (d₀/r)²), d measured from the shore, so brushes laid over each other make one pond with
  one floor. Dry rim samples that would let the liquid out are raised to `edit_bank_m` over the level, but the outlet;
  banks past the brush are cut toward the level with no step. Nothing is filled; no pond it does not reach is lowered.
  The key liquid fills the pond to its pass, with what the pores under it lack (`flow::pores_lack`), from the outside
  node at the ground's temperature, poured into each cell's part of that pond (`flow::pour`); none where the floor opens
  to the sea. What pores and other liquid took is poured again, at most `TOP_UPS` times.
- **Settling** (`World::settle`): what an edit starts that the laws treat as instant in a month runs at once, with no
  time on the clock. Where the ground was shaped: (a) it is worn (`World::wear_edits`, `World::worn`, `space/shaped.rs`)
  as the start's wear would after `edit_wear_years`: the terrain's swells (`terrain::swells`) by how far each sample
  moved, then `WORN_STEPS` steps of the start's law, no sample going under where it stood before the edits; what the
  wear takes leaves the world; a basin's ground is not worn. (b) The samples are built again (`flow::build::reshape`:
  no lobes, the offsets and pond numbers kept, a new `Relief::version`), and what runs through each sample is rerouted
  down the new ways (`build::reroute`), so a river runs round a raised hill at once. (c) The month's start runs
  (`month_start`). (d) The liquid settles (`flow::settle`: the runoff's pass over no time; nothing drains, seeps, mixes
  or carries load). Then strikes and added heat burn (`flame::now`, 3c). This is honest because a month's step already
  treats these as one event, and every move goes through the store. The samples' loose ground follows
  (`mantle::edited`). **Known limit:** the samples' bed make does not follow an edit; the soil keeps its texture.
- The live page settles when a stroke ends, the world paused; while a ground stroke is held, liquid, channels and
  plants do not follow it.

**Chunks and the parallel pass** (`life::chunk`, `life::chunk::run`). The cells are cut into `Config::chunks` runs of
neighbouring indices, the same for any number of threads. Each chunk is one task, the world read only. In each cell in
order the cover's month and then the takers' run on the cell's **view of the store** (`store::Moves`,
`store/gather.rs`): its ground, bed and aside node as the store held them, with its own moves since, and **its share of
its air node** (`Mesh::share`, its area over the node's), so the cells under one air node never overdraw it together.
When the chunks end the world books their moves array by array, cell by cell in order (`World::take_chunks`,
`store::book_by_array`), so the world is the same for any count of threads and chunks. A serial step on one cell runs
on a chunk of its own (`World::in_cell`).

**The contract** (`contract/`): what lies between the natural world (ground, air, cover) and what takes from it: the
takers' field (`fauna`, 3b) and the plain taker of the tests (`worldgen::testkit::Taker`). The natural world names no
module of a taker (`cli/tests/sides.rs`). It is plain data and functions over the one source, with no copy.

| What | Function | Rule |
|---|---|---|
| What a cell shows | `shown`, `slots`, `Layer::Seeds` | the cover whole, each slot by its height, the seeds lying; mass, amounts, energy to break it (`life::breaking`) |
| Its place | `place` | ground temperature, standing liquid's depth, height, soft solid on its land (`Scratch::soft_land`) |
| Snow hides | `above` | matter spreads evenly up a plant: snow d deep on a slot h high hides d / h; any snow hides the seeds |
| Take | `take` | a bite of all a layer's parts in their proportions, with heat, never more than is there (`Cover::take`) |
| Give | `give` | matter and heat back to the ground through the cell's view |
| Cut | `cut` | the share of a frame (`Layer::Frame`) a jaw of a force cuts (3b) |
| Key liquid | `liquid` | open, in pores (roots reach it, a mouth does not), frozen on top, frozen in pores; and the month's flow in its main channel (`World::liquid_month`) |
| Way to liquid | `ways_to_liquid`, `to_liquid` | per decade of need (`LEVELS`), the shortest way over land cells to the nearest open liquid that holds it (Dijkstra 1959; ties by index) |
| Vapour | `vapour` | a taker sheds heat by evaporating its ground's liquid or soft solid into the aside node, paying the latent heat |
| Press | `press` | work breaks the cover by the share it pays for; what breaks goes to the ground as remains |
| What stands | `Stand`, `stand` | mass, capture area, light absorbed and kept (`Cover::stand`); Flow reads it (1.3, 1.4) |
| Land | `land` | the cell's area out of the liquid over its pores (`flow::land_shares`) |

A cell's place, what stands and its land are as the month's start left them; its cover is as it is now. Takes land in
the takers' month; the cover feels them in its next month.

**Inside a cell** (`inside/`, handover 5.2): a cell holds means (its cover, its soil's liquid, what stands on it). The
ground inside it is the terrain's worn samples, and what stands lies on them (`flow::curve`, 1.2). The shell draws inside
the cells by the same general laws at each pixel of the ground's tiles (`tape::Ground`, `tape::Table`, `docs/tape.md`).

- **The liquid table** (TOPMODEL; Beven and Kirkby 1979; Beven 1997): the room at a point is the cell's room less
  `soil_m` (λ − c), λ the wetness index ln(a / tan β) (tan β at least `inside::TAN_MIN`), held between none and full, c
  found exactly so the mean room is the cell's (`inside::soil_level`, `deficit_at`, `soil_held`). The table lies in the
  loose ground over field capacity, else in the bed's store (`Means::deficit`). It moves no liquid; it shares it out.
- **Channels**: a sample whose channel flows (it drains a channel's source, (2 `hill`)², Horton 1945, and its yearly
  discharge is over none: `inside::drains_source`, `flowing`) has the table at the surface over its channel's width
  (`World::channel`): that share counts at an infinite index (`inside::channel_share`).
- **Wet ground** (`inside::wet_ground`, found by `contract::wet_grounds` at the start of life's month from Flow's
  state then): over a cell's land samples, the share saturated and the fill of the drier rest (`inside::rest`). Every law
  reads that one moment (`inside::land_filled`, `saturated_share`): the cover's roots (`cover::soak`), flames (3c), and
  the share of land that holds no gas in its pores (`inside::airless`: the saturated share plus, of the rest, what its
  liquid fills from the bottom over field capacity, `Pores::full`), which roots (3) and decomposers (2.3) read.
- **Patches** (`inside::patch_ground`): each patch of a cell's cover (3) stands on its own ground, nothing stored.

A picture need not keep a cell's mean exactly; the core keeps the books.

**Sleep and wake** (`clock::sleep`, a speed tool): a calm node sleeps until an alarm or until an amount grows by more
than `wake_share`; then its law brings it up to date over the whole time since. A node a gain woke first runs the time
it slept on what it held then, then the step on all it holds. What moves between nodes never sleeps.

**One ground temperature**: every law and the tape read Flow's last solve: the cover its land's (`World::t_ground_last`;
of a cell of parts its land, or its liquid's top where it has no land, 1.1), Change and the potential energy the node's
(`FlowView`), the tape the surface's mean by area (`World::t_surface_last`).

**Checks before a run**: `Config::validate`, `Chemistry::validate` (every reaction keeps its elements and mass).

**The books.** Every move takes from one node exactly what it gives another, with its heat. `store::apply` and the
booking by array (`store::book_by_array`, which adds as `apply` adds) are the store's only writers. Boundary nodes stand
for the sun, space and outside (`store::Boundary`; edits by `World::cross`).

- **Potential energy** is one function of what the nodes hold (`flow::potential`, `flow/potential.rs`). A ground cell
  is a stack from its base up (`stack::Column`): bed and solids each at its middle; its liquid (with what is dissolved)
  first in the pores of its loose solids (of a cell of parts, less the room its land's pores have left, 1.1) as a table
  from the bed's top, the rest standing over the cell's own ground at its centroid; its store `bed_depth` under the bed's
  top, filled from the bottom. An air node's matter lies at the mean head of its cells (the column's height is in its
  heat at constant pressure). An aside node's gas and a cell's life (cover, seeds until they land) lie at the cell's head.
- **Booking.** Each move carries its potential energy from the height it leaves (a ground node's liquid top,
  `stack::Column::top`, which never rises as liquid leaves; an aside node's head; an air node's mean head; a store's
  table), the mean of the booking's start and end. What only passes through a ground node in one booking carries on what
  came in, mixed (`potential::relayed`), so a fall lands where the matter stays. Each node's heat takes what came in less
  what left less the change of its own (`store::fall_heat`): a fall heats where it lands, a lift is paid by the node it
  enters, and the matter's gain is one number (`Store::lifted`). A month's Flow after its start is booked as one
  (`flow::chain`); the month's start, and a part run alone, at their own end.
- **Other laws.** A gas lifted into the air goes from its cell's head to its air node's mean ground (`World::lift`, by
  the heads of Flow's last booking, `Scratch::lift_at`), paid by the air. A reacting ground node (Change) pays what its
  stack and its air gain (`flow::gain`); its gas leaves from its head (`change::booking`). What life gives or takes gains
  the potential energy the function finds between the cell before and after (`flow::gain`), paid by the ground node's
  heat (`store::life_moved`); seeds sent to another cell carry theirs (`store::life_sent`). An edit takes it from its
  boundary node.
- So every element and the energy are kept. `check::drift` checks them against `check::DRIFT` (1e-9), with the
  potential energy as the function of what the nodes hold (`check::Totals::potential`), not as `Store::lifted` booked
  it, so a move that books no lift or fall shows.

**No overdraw.** A law never asks a node for more than it holds; where it could, it cuts that node's own outgoing links
(`solve::sweep::within`), never a whole field. A cell's life takes no more than its view holds (checked in debug
builds). `apply` and `book_by_array` only add, and set rounding below zero to zero, counted (`Counters::rounded`).
Nothing takes ground heat below `T_FLOOR` (`store::Moves::take_from_ground`; what could not be given is counted in
`Counters::clamped_heat`).

## 1. Flow: stuff moves from high push to low push
Files: `core/src/flow/` (ground, liquid, erosion), `core/src/flow/parts/` (a cell's parts), `core/src/flow/runoff/` (the
runoff and its load), `core/src/flow/air/` (air, radiation, rain, wind).

### 1.1 The ground of a cell
Layers from the bottom: the bed (bonded rock), loose ground, soft cover, standing liquid (`land::layers`,
`land::surface_of`); under them a deep store (`Class::Deep`, `bed_depth`, `bed_porosity`) that bodies and Change do not
reach. The loose ground's depth over a cell is held in bins from its samples (`cover::loose_bins`, `Samples::loose`),
which roots read.

- **Soft cover** is a solid warmer than the share `soft` of its melting point, lying on top: snow and ice. It holds no liquid: its pores close as it
  settles (Cuffey and Paterson 2010). It insulates the node under it (its depth over its conductivity), and its surface
  warms no further than its melting point: past it the top melts, so the node takes all the heat that reaches the
  surface (`air::radiation`).
- **The solid surface** is one height (`World::surface_z`): Flow's, kept as matter moves; before Flow first built its
  state, the same sum from its amounts. Only the potential energy stacks its own from the store (`stack::Column`).
- **Heat capacity** is the bed's skin (`skin_m` of rock, `ground::skin_cap`) plus the loose amounts
  (`ground::ground_cap`), for Flow, Change and life alike.
- **Land** asks two questions. Against the sea: a cell's solid surface is above 0 m (`World::is_land`, `flow::on_land`,
  `space::wear::STAND_M`); what leaves it for a cell under that reaches the sea, and creep and sideways heat act on it.
  For life: the area out of the liquid that stands over its pores (`contract::land`, `flow::land_shares`).
- **Heat sideways** (`land::conduct`), yearly, one explicit `solve::sweep` between land cells: a link passes the
  harmonic mean of conductivity × thickness of the active layers, times width over length (two half cells in series;
  Patankar 1980).

**A cell's parts** (`flow::parts`). The ground of a cell with liquid standing over its pores is a short list of parts
under one air, each with its own heat and matter: its **land**, where its ground lies above what stands (`Fill::wet`);
the liquid's **top**, down to `h_mix` under the liquid's level; and its **deep** below that (`Fill::deep`), which holds
the ground under the liquid. A dry cell is one node; a cell partly under liquid no deeper than `h_mix` has land and top;
all under deeper liquid, top and deep; a coast over deep liquid, all three. The land and the top face the air, each
with its own heat, phases, soft cover and reflectance: a mosaic of tiles (Avissar and Pielke 1989; Koster and Suarez
1992); the top is a mixed layer over a deep (a slab ocean, Hansen et al. 1984).

- The store holds the cell's one ground node; Flow keeps how its matter and heat lie between the parts (`parts::Parts`,
  saved) and solves each part's state from what it holds (`parts::solve`, at every build of the state). Ice forms on the
  top at its melting point as soft cover.
- What other laws move (Change, life, sideways heat, falls) is shared between the parts: each substance by the share of
  it each holds, heat by heat capacity (`moves::reconcile`). What Flow moves of one part it books there.
- The land keeps its own pores' liquid; what lies over them runs to the top with its heat (`moves::settle`). The share
  under the liquid follows what stands, newly covered pores filling first (`moves::covers`), and ground changes part
  with all it holds (`moves::strip`), so what stands is the same either way. Liquid moving between top and deep takes
  the heat of the part it leaves (`parts/deep.rs` `fn layer`).
- **The top and the deep swap liquid** once a month, after the runoff's levelling and before the body mixes its heat
  (`parts::mix`, `flow/parts/deep.rs`; 1.2): equal volumes, each with its heat, by eddy diffusion through the
  thermocline (`thermocline`), as two volumes that even out exactly; and all the way where the top is denser, so the
  column overturns: the top's main liquid is nearer the temperature where it is densest with what it holds dissolved
  (`Chemistry::dense_in`) than the deep's. So a top that cools toward that temperature sinks and mixes the column, and
  one that cools past it lies on the deep and freezes.
- **The shore**: the land and the top pass heat by conduction across the edge of the ground under the liquid
  (`Fill::shore`, `parts::conductance` in `flow/parts/node.rs`), in series from each middle. On kilometre cells this is
  far below what the air gives them.
- **The node's state**, which every other law reads (`parts/node.rs` `fn row`): its heat capacity the parts', its
  temperature their heat over that apart from their fusion enthalpy, each substance molten by the share they hold molten,
  its pores those under the liquid and what the land's hold.
- A cell whose land no longer stands apart is one node again; a cell where liquid comes to stand over part of its
  ground, or over all of it deeper than `h_mix`, is made parts at its temperature and phases (`parts::create`). No part
  holds less heat than its matter at `T_FLOOR`.
- **Known limits:** the parts share one mixture of the substances that move with the liquid, so a sea's salt and its
  land's rain stay apart only as the liquid moves them, and the top overturns by temperature alone. The top's depth is
  fixed; a real mixed layer deepens in winter.

**Melting plateaus** (`ground::melting`): a node holds the fusion enthalpy of what is molten in it, so it stays at a
melting point (lowered by solutes, 2.4) until that substance has all frozen or melted. A node whose heat melts or
freezes all of it within a step stays there only for the share of the step that takes, then goes on from that point;
a soft cover that melts away holds its surface at the melting point only until it is gone (`air::radiation`).
**Known limit:** each surface solve is backward Euler: a step h against a node's time constant τ (heat capacity over
what its balance changes per K) ends 1/(1 + h/τ) of its distance from balance, where the exact answer is e^(−h/τ); it
errs most where h/τ is large (dry land, and in the dark, where the air is also taken at the step's start), and it
answers weakly to the length of the rest of a step after a plateau. The long-step test bounds what this does to the
climate (1.4).

- The state is found in full at each month's start (`land::ground_state`). Between, a node is solved again only on a
  plateau or when its temperature leaves the span between the melting points of its last solve; its surface (soft
  cover, reflectance, pores) only once it moved by `SURFACE_SHARE` of the depth that hides half a soft cover plus the
  liquid standing on it. As matter moves, heat capacity and fusion enthalpy follow (`Scratch::note`).
- The state is brought up to date after each air step, before the sink and the liquid read it; a node on a plateau that
  liquid left or reached finds its molten share again from its heat (`liquid::book`).
- Life reads its ground as Flow's last solve left it (`World::thaw_last`, `World::molten_last`), so what it takes or
  gives carries the fusion enthalpy of that share.

### 1.2 Liquid
`flow::liquid` (`flow/liquid.rs` `fn liquid`), once a month after the air's steps, in this order: the bed's stores
drain to the channels inside each cell (`drain::springs`); pore liquid moves between cells (`drain::under`); a cell of
parts' land drains to its liquid (`drain::land_drains`); the runoff's pass (`runoff::surface`); the top and the deep
swap (`parts::mix`, 1.1); each body mixes its heat (`basin::mix`); last, gases dissolve (1.5). The sink into the bed's
store ends each air step (`drain::sink`).

- **Pores first**: the loose ground's pores (each cell's porosity, `World::pores`, 4) fill first, under any soft cover;
  only liquid above them stands and has a head (`land::table`, `land::standing`), the depth bodies and Change read.
- **Each cell's own ground** (`flow::curve`): the terrain's samples inside the cell, their departures from their mean
  laid on the cell's mean surface. The samples of the cell that lie in a pond are its **parts** of the bodies of
  standing liquid, one per pond; what stands over the pores lies on them, each part at its body's level, by what each
  held at the last runoff's pass (`State::held`, saved). A cell with no pond's samples holds what stands only until the
  runoff's pass, from its lowest sample up. The lowest ground is the lowest sample (`flow::levels`, `flow::lowest`).
- **Bodies of standing liquid** (`flow::bodies`, `flow::basin`): a body is a pond of the samples (0), closed (its
  pass) or a part of the open low ground (no pass). Ponds keep their numbers when the samples are built again
  (`Relief::ponds`, a pond's key its first sample; `space::ponds::renumber`), so a tile made before a build never reads
  another pond's level. Only a build of the years drops empty numbers, where they pass `EMPTY_SHARE` of the list
  (`ponds::compact`). A body's volume by level is its own samples': Σ area × (level − floor). Its liquid stays in the
  ground nodes of its cells, each holding its part. When the samples are built again, a part holds what its part of the
  pond it came from held (`build::held_by_pond`, `came_from`); a new part starts empty.
- **Running off** (`flow::runoff`, `flow/runoff/mod.rs`), once a month in one pass:
  - Each cell sends what may run off (what came up from the store; the pores' drainage to the channels, a share
    1 − exp(−conductivity × slope × time / (porosity × `hill`)), which what stands refills first; and what stands over its
    pores beyond what its parts held at the last pass) down the samples' ways from each of its samples, by its share of
    the cell's area. A way reaches the body of the first pond sample on it (`bodies::Bodies::ways`). The liquid goes
    straight from the cell it leaves to the cell it reaches: it wets no cell between, and its fall's heat lands where it
    lands.
  - The bodies come in turn, each before the body its pass leads to (`runoff::stand`): what its parts hold stands at one
    level over its samples (`level::level_with`), no higher than its pass, the pores of the ground it newly covers
    filling first (`parts::pores_first`). The level is found exactly, by halving between sample heights. Each cell whose
    part holds more than its samples hold under that level gives the share over it, mixed, to the cells that hold less
    (`basin::share_out`); what is over the pass leaves, mixed, to the part the pass's way reaches, in the same month.
  - A mineral of growth moves only by its share that stays dissolved: the cell's exchange sites take or give the rest
    at once (`basin::gain`). Of a cell of parts the pass moves its liquid alone (`parts::liquid_movable`): its land keeps
    its own pores' liquid.
  - What runs through each sample is summed down the ways, each sample before the one it runs to: the **discharge**
    (`World::discharge`, mol/s, a moving average over about a year). A cell's own is the most through one of its
    samples, its main channel (`World::liquid_flux`; the month's own, `World::liquid_month`, is what a taker drinks).
  - A body that gets less than it loses (evaporation, 1.5) falls under its pass and keeps what was dissolved in it.
  - **Where the edges are open** (`World::edges_open`, 1.6), the part of the open low ground at the map's edge
    (`Body::edge`) stands at 0 m as at a pass: what stands over it leaves for the outside node, and what it lacks comes in
    from there (`runoff/edge.rs` `fn come_in`), counted in `Counters::edge_liquid`.
  - **Known limit:** the sea's slope under wind and rotation is not modelled; levelling crosses the sea at once.
- **Each body mixes its heat** (`basin::mix`) under each air block: its cells off their melting plateaus, with no soft
  cover or plant cover and the same phases, take the block's temperature (of a cell of parts, its liquid's top).
- **The ground by height** (`flow::shore`, `flow::Shore`, `flow::shores`): the cell's samples by height under the
  levels its liquid stood at: the ground above a height, and of it the land and the ground under the liquid (3 reads
  it, `cover::footing`).
- **The channel** (`flow/channel.rs`, `Physics::channel`): what a cell passes on runs in a channel of width
  `channel_width` × Q^`channel_width_exp` and depth `channel_depth` × Q^`channel_depth_exp`, Q in m³/s (hydraulic
  geometry downstream, Leopold and Maddock 1953), its speed Q over width × depth. `World::channel` takes a cell's flux at
  the mol per m³ of its liquid (`World::liquid_in`). Every shell reads it here.
- **Under the surface** (`flow/drain.rs`, `flow/deep.rs`):
  - Pore liquid flows by Darcy's law, an explicit sweep on the liquid table: two cells' conductivities in series
    (harmonic mean of `Pores::seep`, 4) × saturated thickness × width / distance, at most half of what levels them, none
    from pores at or under field capacity (`Pores::field`). The bed's store flows so by `bed_seep` (`deep::flow`).
  - Liquid over field capacity sinks to the store after each air step (`drain::sink`, at most `Pores::seep` a second and
    the store's room): held to the liquid step, the rain would darken the ground (`land::reflectance`). A full store
    gives back what it cannot hold.
  - Real land drains to channels a few hundred metres apart, so pores and store also drain by Darcy's law down the
    cell's steepest slope over a hillslope of `hill` into the runoff (`drain::to_channels`, `drain::springs`).
  - **None of this runs under a body** (its pores are full, its store is not drained). In a cell of parts it acts on
    the land alone (`liquid::soil_share`, `liquid::mobile`), whose pores drain to the liquid (`drain::land_drains`); a
    link passes in each layer the product of the two cells' land shares (`liquid::reached`).
- **A cell of parts** (1.1): what the liquid laws bring gathers in its top; what the laws under the surface take leaves
  its land (`parts::Leaves`); what the others take leaves its top and deep by their liquid, and its land only for what
  they lack (`parts::to_liquid`). The land and the deep give heat at their own temperature (`Piece::heat_of`); the top
  takes the rest of the node's change of heat.
- **What goes with it**: its share of every liquid and of what is dissolved, with heat. Liquid passing through a node
  keeps its fusion heat whatever the node's phase and leaves at the temperature of the node mixed with what came in (on a
  plateau the plateau's substance melts or freezes first), so it never takes more heat than the node holds. A salt is a
  liquid that never boils: it moves with the liquid and stays when it evaporates. Each move carries its potential energy
  from the height it leaves (`potential::apply`; 0, "The books"), and the fall turns to heat where it lands. The head it
  loses on the way does work (1.3).

### 1.3 Erosion and creep
**Work** (`flow::runoff::pick`, `pick_up`): the head lost by moving liquid does work in each land cell: g × what runs
through each of its samples in no pond in the month, kg, × the fall to the sample it runs to (the samples as last built).
In a body and below 0 m it wears nothing.

- It **carries** up to `carry` m³ of loose solids per J × the loose ground's erodibility against a loam's
  (`Texture::erodibility`: M^1.14 (12 − OM), Wischmeier and Smith 1978; middle particles carry most easily) × the bare
  share 1 / (1 + mass per area of what stands / `hold`) (`flow::bare_share`), at most all that lies loose.
- The work its load leaves free (the cover effect, Sklar and Dietrich 2004) **wears** `wear` m³ of bed per J over its
  bond strength into loose ground.
- Its pick-up takes first from the relief of its samples (below), each by its work, then from the cell's loose ground.

**The load settles on its way** (`flow/runoff/load.rs`, `carriage.rs`), in the three particle classes of the texture
(4), each as the cell it came from holds them. It goes down the samples' ways each month and is followed only at **key
samples** (`Carriage`): where some class settles, each land cell's main channel, and where the ways end. Each sample's
reach is the run L to the sample it runs to. The flood (`FLOOD_RUNOFF` from the area above it) runs in its channel at
shear velocity u* = √(g H S) (`soil::flood_shear`). The share φ of a class's log-sizes that sink faster than u*
(`texture::settles`, Bagnold 1966) settles at the sinking speed V of their mean log-size over the length ξ = q / V (the
ξ-q model, Davy and Lague 2009). Of what comes in at a reach's top, P = 1 − φ (1 − e^(−L/ξ)) passes; of what is picked up
along it, R = 1 − φ + φ (ξ/L)(1 − e^(−L/ξ)) (`soil::passes`). A sample where each class passes within `PASS_TOLERANCE` is
no key sample; what leaves a key sample reaches the next by the product of the passes between (`Carriage::skip`), the
rest settles in the next one's cell. What reaches a body piles on its entry sample; where a way ends on land, all of it
settles there.

- **The relief** (`space::relief::Relief`, saved, sparse): what settles at a key sample or piles at a mouth lies on that
  sample, as solid volume per area. It says where in a cell its matter lies; the matter is booked in the cell.
- **A sample holds load up to its room**: the volume that lifts it to the threshold slope over the sample it runs to
  (u* reaches the coarsest's sinking speed, S = w² / (g H), `soil::threshold_slope`; the threshold channel, Parker 1978),
  no higher than the lowest sample that runs to it (a deposit dams no way that feeds it). Where what would settle passes
  the room, each class settles by the share the room takes and the rest passes on. So a plain builds forward.
- **Books**: the load is booked apart from the liquid (`runoff::Crossings`), each move from the cell it leaves to the
  cell it reaches, with its heat; each key sample's pool leaves by the share that passes. The load is the loose ground's
  solids (`Scratch::solids`), never a frozen liquid. What settles mixes into the texture of the cell it reaches, by
  volume over its loose ground up to `skin_m` deep (`runoff/load.rs` `fn mix`); its pores follow at the next month's
  start (`soil::month`).
- **Known limits:** between builds a fan or plain builds forward but does not fill back up its valley (the rooms are of
  the slopes as last built). The load is booked after the liquid's pass, so the liquid in the pores of the ground it
  picks up stays in its cell until the next pass. Snow and ice do not shield the ground. What the skip product takes
  settles in the next key sample's cell. A fixed share per class ignores that a class loses its coarse end as it goes.
  The heat of the liquid's fall lands with the liquid, not along its way.

**The build** (`flow::build`, at a year's start every `build_years`, after the month's start built Flow's state):

1. **The mouths lay their lobes**: what piled on the bodies' samples since the last build is laid as the start's lobes
   (0; `delta::lobes`), its top its body's level plus the flood's depth over its entry (`delta::flood_depth`), no
   higher than its pass. What a lobe lays in another cell moves there from the entry cell's loose ground with its heat
   and potential energy (`runoff::book_solids`), at most what that cell holds loose: the only matter a build moves.
2. **The samples stand at the seed's heights** with what the relief laid and edits shaped, raised so each cell's
   samples' mean is its ground exactly (`World::ground_z`: solids that stay solid at the surface, not snow and ice;
   offsets spread smoothly between cell centres, `space::relief`).
3. The flood finds the ways, ponds and wetness index again (`Terrain::built`). The sea is what lies under 0 m joined
   to the seed's lowest sample (`drain::open_from`), so a pit dug inland is a closed basin; a basin too shallow to hold
   a pond (`ponds::shallow`) is none.
4. The cells' ground, the bodies, the key samples and their rooms are found again; the parts' liquid follows their
   ponds (a new pond comes from the old one that held most of its samples). The relief joins the samples' loose ground
   (`Samples::loose`, `settled`); the loose bins and the facing are found again.

`World::is_land` stays the cell's own (its surface with snow and ice over 0 m), so a cell can be land while its samples,
built at its ground without them, lie under the level.

**Creep** (`flow::creep`, yearly, out of land cells): loose solids (`Scratch::solids`; not snow or ice) slide downhill
as a slow diffusion, `creep` × softness 1 / (hardness + `hard_min`) × volume × slope × the bare share, across each border
by its width (none across a corner), at most what levels the two surfaces.

### 1.4 The air's step, the sun and the cover's share
`flow::air::step` (`flow/air/step.rs`), every `AIR_STEP_DAYS`. Before it the heat the cover does not claim goes back to
the ground (`cover::release`). Then:

1. The gas of the aside nodes comes up into their air nodes (`book::up_from_bodies`), with its lift.
2. **Each cell's exchange** with the air it sees is solved once, an implicit surface balance (Newton's steps about its
   end temperature, with its plateaus, 1.1) at the air of the step's start: sunlight, heat radiation, sensible heat
   (`sensible`), evaporation (`air::cells` `fn exchange`). Its slopes against that air come from the same balance on the
   branch it ended on (the implicit function theorem), so the exchange is one law, linear in the air. They are cut so the
   cell never gives more gas or heat than it holds however the air moves (`outs::hold`, `AIR_SWING_K`). Of a cell of
   parts, the land (sun of the cell's slope) and the top (flat sun) each solve their own balance and the cell answers as
   their sum (`surf::parts_respond`), each part booked its own (`book::parts_moved`).
3. **The air's days** (`air::daily`), explicit on plain arrays: each node's exchange with its ground by that law, its
   heat radiation to space and its rain (1.5); each block's heat and vapour are linear in them through the day, so the
   day follows them exactly. Then the blocks are evened and the wind blows (1.6).
4. **The book** (`air::book`) takes each cell's law at the step's mean air, with the rain: what the days summed, so the
   air ends where its days left it. The 15-day step is checked against 1-day steps (`schedule::step_with_air`;
   `the_long_step_keeps_the_climate`, `tape/tests/ranges.rs`).

- **Sun** (`air::radiation`): the daily mean by latitude, season and day length (Berger 1978; `sun_by_latitude`,
  `declination`), the mean over the step's days. **On a slope** (`on_slope`): by Lambert's cosine law the beam is linear
  in the slope, so a cell takes the sun of its mean slope. Over a day the east-west part cancels and the north-south part
  is the slope of the daily mean with latitude: Q + r ∂Q/∂φ, r the cell's mean rise toward the south (0), the equivalent
  latitude (Lee 1962; Swift 1976). A cell whose day sums below zero takes none. A cell under a body's level lies level.
- **Through the air** (`radiation::Light`, `radiation::Column`): the air node's gases absorb sunlight
  (`Chemistry::sun_depth`: `sw_abs` per mol/m², past `sw_sat` as the square root of the amount) and scatter it
  (Rayleigh, `scatter`: half up, half down), down along the sun's slant (1 / its mean height, `sun_height`) and back up
  as diffuse light (slant `DIFFUSE`, Elsasser's factor). What the air absorbs warms its node; what reaches the surface
  its balance takes; what passes back up leaves to space. The three sum to what came in. The surface reflects as its
  cover is at the step's start (`land::reflectance`), over its own ground: the share under standing liquid as a column
  with that liquid, the rest by what its pores hold; a soft cover as it is at the step's end. **Known limit:** a
  substance reflects the same share at every sun height and state (`refl`); no Fresnel, no ageing of snow.
- **Heat radiation** (`radiation::band_depths`, `radiation::Heat`): surfaces emit σT⁴ in three bands
  (`Chemistry::heat_spans`), each its Planck share (`Chemistry::heat_shares`). In each band a gas's lines have the
  optical depth `Chemistry::depth` (`Substance::heat`: in proportion to its column, past saturation as the square root),
  at height hk H / (hk + H) where its amount times the pressure lies, hk the height its amount fills (`Scratch::hk`); its
  continuum (`Substance::continuum`) goes by its amount times its own pressure, at half its height. Each band is a core
  of strong lines `line_spread` times its depth and wings 1 / that; each part absorbs 1 − exp(−x) and emits from 2 grey
  layers that cool with height (`radiation::column`), so more gas lifts the layer that reaches space into colder air and
  the ground warms. The air touching a cell is the air it sees (1.6), colder by the drop with height.
- **Clouds** (`exchange::cloud`, `radiation::Cloud`): a node's cloud covers the saturated share b of its sky
  (`Seen::clouded`, 1.5) over the step before; what it rained then stays droplets for `cloud_life` (Smith and Barstad
  2004): a liquid path of condensation rate × `cloud_life` / b in droplets of `drop_m`, optical depth 3 path / (2
  `drop_m`) (Stephens 1978). For sunlight, two streams (Eddington; Meador and Weaver 1980; Joseph, Wiscombe and Weinman
  1976) with forward scattering `drop_forward`, absorbing `drop_dark` (Slingo 1989) into the air; its base sends back down
  what is reflected up (`Column::under`). For heat its depth is half that, grey, on the diffuse slant
  (`radiation::drops_heat`). Clear and clouded skies are each their own column (`radiation::Light`, `radiation::Skies`),
  taken by the cloud's share. **Known limit:** droplets are spread like the air's mass, so a deep cloud sends its heat to
  space from the top of the air.
- **The cover's share** (`contract::Stand::catch`, at the month's start): over each ring of ground its slots stand on
  in each patch (`cover::canopy::catch_rings`), the ring's share × (1 − e^(−capture area per m²)) (Beer's law per patch),
  on the cover's own ground (`cover::ground`: all of a cell of parts, else its land), over its land first, then under its
  liquid (`cover::over`), dimmed there by e^(−depth / `light_depth`) (`cover::dimmed`); the light the liquid stops warms
  the liquid. The cover reflects part of what it catches and absorbs the rest; per m² of capture area that is the cell's
  light (`World::light`), which the cover shares down through its crowns (3).
- **The aside node** (`Class::Aside`, one per cell): the light the cover absorbs warms the ground, except the share its
  light reaction stores (the cover's `keep`), booked from the sun into this node; the cover takes it in its month, and
  `cover::release` gives back what it does not claim before each air step.
- **A body as one surface** (`air::surf`, `basin::one_surface`): under each air node the cells of a body that mix as
  one (1.2) face the air as one surface (`surf::Patch`): one balance with their summed area, heat, heat capacity and
  gases, at their mean temperature, reflectance, height and air, and the sun of their mean latitude. Each cell takes its
  share of what it gives: heat by heat capacity, each gas by what it holds. Its rain falls on it as on one place (1.5).
- **Gas from the cover and from Change** goes into the aside node with its latent heat; the air's step takes it up with
  its lift, the one way a gas the ground or its life gives rises into the air (`World::lift`; 1.5).
- **Shade**: the cover's kinds share their light by height (3).

### 1.5 Evaporation, rain and dissolved gases
`flow/air/cells.rs`, `daily.rs`, `rain.rs`, `seen.rs`, `rise.rs`, `drift.rs`, `soak.rs`, `exchange.rs`.

**Evaporation** goes by vapour pressure (the ground's saturation pressure against the air's), inside the surface
balance, taking the heat it held at the node's temperature:

- A liquid in the pores, by the share the ground holds for roots (`Pores::available`, the share plants feel, 3): the
  open rate at field capacity and over, none at the wilting point, linear between (Manabe 1969's bucket); the air does
  not dry it below the wilting point (a real ground dries slowly there, from its top: stage two; Ritchie 1972). A solid
  in the pores evaporates down to none.
- Over the cell's own ground as its reflectance is: the share under standing liquid holds its pores' liquid and all that
  stands; the rest its pores' liquid; each a bucket by its area.
- Through a second film where the cover's open pores stand over its land (`World::second_film`, 3), in series with the
  air's film, at its open rate while its share of the ground's liquid lasts.
- A node over a substance's boiling point boils off what its heat over that point pays, cooling to it
  (`land::boil_off`).

**Rain.** A node's air is not evenly moist (`seen`):

- Its moisture spreads evenly over a width of 1 − `moist_min` of saturation, and its saturated part holds no more than
  saturates it. So the cloud's share at the node's ratio r to saturation is b = 1 − √((1 − r) / (1 − `moist_min`)),
  none below `moist_min`, all at 1 (`seen::saturated`; Sundqvist et al. 1989; Tompkins 2005; Quaas 2012). The air over
  each place is `seen::MOIST_PARTS` equal parts that saturate one after another (`seen::parts`), against what saturates
  the air touching that place (colder higher up).
- Each saturated part rains out in `rain_time` what the node holds over the ratio it saturates at; only the saturated
  share condenses, so the node's mean stays under saturation. Evaporation reads the node's mean vapour.
- **The ground lifts the air** (`air::rise`): the low wind (`rise::low_wind`) is the overturning's lower half (1.6),
  `wind` times the warmth's gradient the node feels (`rise::felt`: the planet's where the edges are open, the node's own
  where it is a block alone, else none), down it inside the month's cell and turned by the spin. Where it climbs the
  surface the air touches it lifts air at the ground's density; where it descends it lets air sink. In `rain_time` the
  share f = min(1, |lift| × `rain_time` / the lower half's mol) of the lower air swaps with the upper half, which holds
  ρ = split / (1 − split) of its vapour per mol (`Scratch::split`): rising air saturates at full × (1 − f (1 − ρ)),
  sinking air at full / (1 − f (1 − ρ)) (`rise::saturates`; Smith and Barstad 2004). Nothing is kept: it changes only
  what the rain law sees in the step. **Known limits:** the lift is summed per node, so a node that holds a windward coast
  rains more over its sea too. There are no westerlies of the middle latitudes (Vallis 2017, 15), so the wet side of a
  range is its east there, and the low wind is weaker than the measured trades (1.6).
- **The day** (`rain::Pool`): each block's vapour follows what its cells evaporate and what its nodes rain; both are
  linear in the vapour between two ratios at which a place starts or stops raining, so the day goes exactly from one such
  ratio to the next.
- **Where it lands** (`seen::Seen::landing`, `flow/air/drift.rs`): a node's rain condenses over its places by what the
  air over each holds over its own saturation, so most over high ground. A place keeps 1 − e^(−d / L) of it, d its
  width, L the low wind × `rain_drift_s` (drops forming and falling; Smith and Barstad 2004); the rest drifts along the
  wind by an exponential of mean L, laid bilinearly on the nodes along its path, across their borders. A share whose path
  leaves the map lands by the rest's shares; a node with no wind rains where it condenses. **Known limit:** on the globe
  part of a node's rain lands a row away, and no frozen cover moves, so a lifeless globe's polar ice grows.
- **The rain in the bucket** (`soak`): a cell's exchange is solved without the step's rain; each day's rain wets the
  bucket it falls on, followed exactly through the day under the step's open rate. What the bucket evaporates over what
  it would without the rain goes to the air that day, the cell's other outputs moving by the slopes of its balance.
- The rain reaches each cell at the temperature of the air touching it (`touching`), the air paying what warms it.
  Under the gas's melting point at the node's mean ground it falls as snow, the air keeping the fusion enthalpy;
  condensing frees the vaporisation enthalpy in the air.

**Lift**: gas going up (evaporation, and the aside node's gas at the start of the air's step,
`book::up_from_bodies`) is lifted from the ground's head to its air node's mean ground, paid by the air's heat; rain's
fall gives it back (0, "The books").

**Dissolving** (`exchange::dissolve`, once a month at the end of the liquid step, over the month, at the ground's
temperature as the liquid step leaves it, under the air of the last air step's start). The gases that dissolve are
chosen at the month's start (`exchange::henry`) and evaporate in no air step of it. A gas that cannot condense
dissolves in the ground's liquid toward Henry's share, through the air's film (`evap`) and the liquid's (`dissolve`)
in series, freeing its vaporisation enthalpy, and comes out as the liquid warms, thins or freezes. A body under one
air node takes it up as one liquid, each cell by its share. **Known limit:** a liquid takes up gas into all its depth,
deep as well as top (1.1), so a deep sea takes up gas in months; a real one takes centuries.

### 1.6 Wind
`flow/air/wind.rs`, `daily.rs`, `cell.rs`, `stretch.rs`, `edge.rs`, `planet.rs`, `weather.rs`, `draw.rs`.

**Blocks.** Eddies (`mix`) even out the air within a day over sqrt(`mix` × day) (Taylor 1922; `daily::BLOCK_DAY`):
the air nodes lie in blocks of that side (`daily::blocks`), and each day every gas of a block is shared among its nodes
by volume, and its heat so each has the block's warmth (its temperature raised to one height by the block's drop with
height). An island smaller than a block is one block: it has no wind.

**Across the links between blocks** gas moves by three pushes (`wind::pushes`, `wind::push`), each day from the air at
the step's start:

- **Pressure** (p = nRT/V), evened in the time sound takes to cross the link (Lamb waves; Lindzen 1990), explicitly:
  each link at most half of what evens it over the links of the busier node, so no node gives more than half of a gas.
  **Known limit:** real sound is faster, so a pressure difference across many blocks takes days to even out.
- **Eddies** mix the columns: as many mol each way, `mix` × the column's mol × width over length a second, so every gas
  mixes by its share of the column. The heat a mol carries is the near-ground air's static energy cp T + M g z
  (`wind::aloft`), so the heat goes as −(p_s / g) `mix` ∇h, the diffusion the eddies of the measured climate follow
  (Hwang and Frierson 2010). Both sides book the one number. **Known limit:** the vapour goes by its share of the whole
  column, not the near-ground air's, so it carries about a third of the latent heat that diffusion would.
- **Overturning**: the lower half flows to the warmer column and the upper half back, a mass flux of `wind` per K/m of
  the warmth gradient along the link (the mean of the gradients fitted at its two ends, `wind::gradient`; a wave two
  nodes long is the mesh's own mode, not a flow of air, so the gradient is never a one-link difference), turned by the
  planet's spin (`rotation`), through half the link's cross-section, so it does not depend on the size of the nodes.
  - Down the gradient it runs only inside the month's overturning cell of the low latitudes (`air::cell`, `Cell::of`):
    the upper air flows poleward keeping its angular momentum until the warmth's fall can no longer balance the wind
    (Held and Hou 1980). With R = g H Δ / (Ω a)² (H = `air_top_m`, a the radius, Ω = `units::SPIN` × `rotation`, Δ the
    fall of radiative-balance warmth from equator to pole of the yearly sun), an equatorial heating makes a cell to
    ±sqrt(5R/3). The month's heating peaks where the sun is greatest as a layer of `h_mix` of the key liquid feels it,
    with a lag; about a peak off the equator the cell is Lindzen and Hou's (1988): the air rises poleward of the peak,
    the winter half is the wide one. The share down the gradient is 1 where the air rises, falling as
    1 − (distance / half's width)² to none at the half's edge; the turned part stays. With `rotation` 0 the cell spans
    the planet.
  - Each half carries its node's air, vapour crowding low, each mol with its node's heat per mol; the upper half holds
    more heat per mol than the lower by (M g − cp gam) × depth / 2 (gam the column's drop with height; Neelin and Held
    1987). The part down the gradient carries that rise to the colder column; the part turned along the isotherms
    carries none (Holton 2004, 3.4) (`wind::halves_heat`).
  - **Known limits:** no Ferrel cell (eddies alone carry heat poleward past the cell); the year's cell is narrower and
    weaker than the measured one, as Δ comes from the yearly sun's contrast. On coarse nodes the halves also mix the two
    nodes, as eddies of about its speed would.
- The eddies and the overturning are one implicit (backward Euler) step a day, along the east-west lines of links, then
  the north-south ones, each line solved exactly (`solve::line`: Thomas; a ring by Sherman and Morrison): stable at any
  rate, no gas goes below zero. **Known limit:** their rates per mol are those of the step's start, so a node whose air
  grows in the step moves a little more or less than its rates say.
- Mass that crosses is lifted between the nodes' mean grounds by the heat of the node it enters. Air moves as whole
  columns.

**What a cell sees** (`scratch::tables`, `air::lerp`, `flow/air/corners.rs`, `World::near_ground`): each square of
the map reads its air node and the three next to it on its side of the node's centre, bilinearly; a cell reads what its
squares read, by area (up to 3 x 3 nodes, `SEES`), so each node's air is read once in all. It reads so the air's
temperature, vapour, drop with height, the light its air and cloud let through, the
heat radiation it sends down and absorbs, and the height its vapour fills. Nothing steps at a node's border. What a
cell gives and takes is booked with its own node.

**The map's stretch** (`air::stretch`, `Stretch`, `World::stretch`): the air over a small map behaves as the air over
the band of latitudes its rows stand for, while the ground keeps its true metres. Only the air's sideways laws read it:
every horizontal length counts `stretch` times longer, so a column holds `stretch`² times its gas, and a rate between two
stretched columns moves the map's column by its share (`Stretch::per_column`). So the eddies' and the overturning's
rates fall as 1 / `stretch`², pressure's evening as 1 / `stretch`, and the blocks' side in nodes divides by `stretch`
(and with it a body's mixing under each block, 1.2). The exchanges up and down, the rain's drift and the ground do not
read it. `stretch` is the band's length on the planet (`Config::radius_m` × the band in radians) over the map's south
to north for a world in the planet's air; alone, and on a globe, 1.

**Alone or in the planet's air** (`World::planet`, `air::planet`, `air::edge`). A world made without a band (`ew run`
with no `--band`) is alone: its air is closed at the map's edges, and it makes its own climate. A world made with a band
(`ew run --band S,N`, the page's band choices) holds the planet's table (`flow::Planet`); where its band is longer than
its own span (`stretch` over 1) its edges are open (`World::edges_open`). The table's presence is the state.

- **The table** is a strip of the planet from pole to pole run by these laws (`worldgen::planet`): `COLUMNS` air blocks
  round, rows of 5°, all sea, for `YEARS` years; its last year's air per row and month. It reads only chemistry 0, the
  default laws and its own seed, so one table serves every island. The build holds it (`worldgen/src/planet.bin`,
  written by `ew planet`); a test runs the strip again and holds the table to it bit for bit, so a change of the laws
  that moves it fails until the table is written again. It is saved with the world.
- **Beyond each edge node** lies a column of that air at its latitude and time of year (`Planet::at`), over sea at 0 m,
  linked as two nodes are (`wind::push`, with the stretch); across the east and west edges the turned overturning is its
  mean wind along the row; its pressure is the planet's, so no evening crosses. Each day each link is one implicit step
  with the column held. What crosses comes from and goes to the outside node with its heat and latent heat, paying its
  lift from 0 m (`potential::Lifts`); the heat is counted per edge (`Counters::edges`).
- **The planet's low liquid**: where the edges are open, the open low ground at the map's edge stands at 0 m (1.2,
  `runoff::stand`), so what the island's air gains or loses as vapour across the edges comes back as liquid. Lakes,
  closed basins and a world alone are not joined.

**The weather** (`air::weather`). The 15-day air step cannot hold the weather systems of the middle latitudes; without
them each year repeats the last. They enter as one random field of spread one: how far and how fast the eddies move the
air north or south (`Weather`).

- On a lattice of points `eddy_size` apart on the planet (map metres times the stretch), each point draws a unit normal
  each day, keyed by the seed, the day and the point (`rng::Purpose::Weather`): no state, the same on any count of
  threads. The fast part sums past draws weighted by e^(−age / `eddy_time`) over `weather::FOLDS` e-folds (a red noise).
  The slow part is what a top layer of `h_mix` sums of the same weather and lets go over its heat per K over `top_damp`,
  at sqrt(`eddy_time` / τ) of the fast part's spread (Hasselmann 1976; Frankignoul and Hasselmann 1977). A point reads
  the four lattice points about it, weights over their root sum of squares, so its spread is one everywhere.
- Its displacement is the eddies' mixing length sqrt(`mix` × `eddy_time`) (Taylor 1922), its speed `eddy_speed`; soft
  weather (`worldgen::terrain::soft_weather`, `--weather soft`) takes sqrt(`mix` / `eddy_time`). Their ratio, about a
  day, is the eddy's 1 / ω (a wave of about 6 days).
- **Across the open edges** each column beyond is read each day at its latitude less the displacement
  (`Edges::weather`); past a pole, the other meridian's air (`Planet::at`); across the equator, the other half's season.
- **The air it lifts** (`Weather::lift`): air an eddy moves along the warmth's gradient slides along surfaces of equal
  warmth (the baroclinic eddy; Holton 2004, 8.4), so it rises at its speed times their slope: the gradient the node
  feels (`rise::felt`) over how fast the warmth rises with height (`wind::End::rise`).
- **The lifted air draws the low air in** (`air::draw`), which feeds a weather system's rain (P − E = −∇·(q v); Holton
  2004, 3.5; Trenberth 1999). Each day the lifts are summed per block, and the flow on each link between blocks is the
  difference of a potential whose convergence at each block is its lift (the divergent wind; Lynch 1989): a Poisson
  problem, solved by conjugate gradients (`draw::RESIDUAL`). Each group of blocks sinks as much as it lifts. It does not
  cross the open edges (the columns beyond are a held table, and a system closes within the map's air). The flow is
  added to the overturning's (`Pushes::drawn`) and moves as it does, in the same implicit step, its halves carrying the
  static energy's rise (`wind::halves_heat`). `mix` stays whole beside it: the drawn flow's sign is random, so it is no
  part of the eddies' net flux.
- None on a globe (its edges read the strip's last year as the climate) and none on a world alone in one block.
  **Known limits:** inside a block the flow does nothing. Each half carries its sender's air, so the flow also mixes heat
  down the gradient whatever its sign. Rain inside a day is as smooth as the lift. The east-west displacement is not
  drawn. The slow part acts on every world, over land too.

### 1.7 Each law takes one step
An explicit step is safe while `dt Σ g / cap` of each node stays within 1/2 (no pattern changes sign in a step;
`solve::sweep`, Patankar 1980). For a diffusion of D m²/s on a cell of side Δ with four borders that is 4 D dt / Δ² ≤
1/2, so a step `dt` is safe down to Δ = √(8 D dt). With the defaults (month 2.59e6 s, year 3.11e7 s):

| Process | Step | D or rate | Safe down to |
|---|---|---|---|
| Pore flow, Darcy (`Pores::seep`) | month | conductivity × 1 m of loose ground / porosity: 9e-6 m²/s for a loam, 8e-5 for the most coarse | 14 m, 41 m |
| The bed's store, Darcy (`bed_seep`) | month | `bed_seep` × the depth it fills / `bed_porosity`: 4.5e-3 m²/s full | 305 m full (sub-stepped below, `deep::flow`) |
| Heat sideways (`land::conduct`) | year | k / (ρ c) of the active layer, about 1.1e-6 m²/s: rock spreads a 2 km cell's heat in about 70,000 years | 17 m |
| Creep (`creep`) | year | `creep` × softness × 1 m: 1.3e-9 m²/s for the softest solid | 0.6 m |
| Erosion and carrying (`carry`, `wear`) | month | the bed lowers `carry` ρ g Q per m of drop a step: at the largest rivers, under half the drop to the next cell | metres |
| Runoff (`flow::runoff`) | month | one pass down the samples' ways: liquid crosses the map in one step | any size |
| The bodies' levelling (`runoff`, `basin`) | month | one level, found exactly | any size |
| The air's exchange with a cell (`air::cells`) | 15 days | implicit in the cell; the air's days run on the blocks | any size |
| Eddies and overturning (`wind`) | day | implicit along lines (`solve::line`) | any size |

The drain to the channels inside a cell (`hill`), the sink into the store and what comes up are exact for any step in
the cell itself. The cells as built are kilometres wide (hundreds of km on a globe), far above every limit of the table,
so each law takes one step.


## 2. Change: substances react inside nodes, faster when hot
Files: `core/src/change/` (the reactor and its step), `core/src/chem/` (the chemistry), `worldgen/src/chem0.toml`
(chemistry 0). Light-driven reactions never run here (`Reactor::lit`): only the cover catches light (3).

### 2.1 The rate law
```
rate    = catalyst · k · exp(−Ea / T) · contact · Π over inputs (share of its phase ^ order) · Π over inhibitors 1 / (1 + c / Ki)
lim     = the least, over the inputs, of (amount / count)      (the scarce input)
contact = what lim would be if each input were all of its phase
extent  = lim · (1 − (1 + p·z)^(−1/p)),   z = rate / lim · time   (1 − exp(−z) when p = 0)
p       = (order of the scarce input − 1) · (1 − its share)   for a gas or liquid; 0 for a solid
```
- **Share, order, extent.** A solid is a phase of its own, so a solid reacting with the air goes like a surface. The
  order is the count, or `Reaction::ord`. The extent is the exact integral of dx/dt = −k x^(1+p) (`kernel::gone`),
  never past the scarce input.
- **Heat**: −ΔH to the node (`Chemistry::dh`). Formation enthalpies are of the solid; a liquid holds its fusion heat
  more, a gas its vaporisation heat too (`Chemistry::dh_at`).
- **The open world** (`Reaction::open`, `change::open_catalyst`): the decomposers' catalyst works as far as the node is
  moist, 1 + (`open` − 1) × d / (d + `wet_depth`), d its liquid's depth in pores and standing (`FlowView::moist`,
  `Physics::wetness`). An air node is not moist. Such a reaction that takes a gas runs only in the node's share whose
  pores hold gas (`Reactor::aired`, 2.3).
- **Sheltered matter**: a reaction that takes a substance held on mineral faces (`Substance::bound`, 14) runs at
  1 − `Texture::shelters` of its rate (4).
- **Inhibitors** (`Reaction::inhibit`, the chemistry's `[[inhibit]]` list): the rate is divided by 1 + c / Ki for each,
  c the inhibitor's mol per kg of all the node's matter (`Reaction::inhibited`; non-competitive inhibition, Segel 1975).
  No rule names a body or a food. The takers' gut reads the same factor when it splits walls (3b); a taker's burning
  (`fauna::gut::burn`) and the cover's upkeep do not.
- `change::rate` is the one rate, also of `Reactor::balance`.
- **Frozen**: a reaction whose open-world catalyst in the node is at most 1 and that would use under `NEGLIGIBLE` of an
  input in a year at T is skipped (`frozen_below`), a speed tolerance.
- **Sub-steps** (`Reactor::react`): the step is halved while the reactions' heat would move the node more than
  `MAX_DT_K`; a reaction that takes heat takes at most `MOST_HEAT` of the node's heat in a sub-step.
- **Books**: `Reactor::made` sums cancelling terms exactly (`math::add_times`); debug builds check each row keeps its
  elements (`Chemistry::keeps_elements`).

### 2.2 Where it runs
`change::step`, each month after life's month (`schedule.rs`), over ground nodes and then air nodes.
- A due node reacts for all the time since it last did, at Flow's last temperature; a ground node's fusion heat on a
  plateau is set aside first (`FlowView::fused`).
- **Sleep** (`clock::Sleep`): then it sleeps until its starting rates would use `CALM` of an input (reactions on a
  `TRACE` of their phase not counted) or warm it `CALM_K` (`Reactor::rest`), at most `sleep_days`. A gain of more than
  `wake_share` of an amount wakes it: it first reacts the time it slept on what it held, then the step. After the law's
  first run node i sleeps at most (1 + i mod m) / m of `sleep_days`, m the steps in it (`change::first_sleep`).
- **Rows** (`change/rows.rs`): the ground nodes under one air node are one task; each node fills a row (changes, gas
  given or taken with its heat and lift, sleep). `Rows::book` (`change/booking.rs`) books them after the parallel call,
  one task per store array (`store::book_by_array`), in row order, so threads do not change the result.
- **Gas of a ground node**: it reacts with its own gas and its cell's share of its air node. What it gives or takes is
  what its reactions made less what it kept, so it never gives more than it holds. Gas given goes to the cell's aside
  node with its heat and latent heat, and the air's next step lifts it (1.4). Gas taken comes down from the air node
  with its heat at the air's temperature; the air gets back its lift. A node gives no more heat than it holds (every gas
  given cut by one share). `Rows::fall` books the potential energy its stack gains.

### 2.3 What gas a ground node touches
- **Open ground**: its own gas and its cell's share of the air.
- **Under standing liquid**: only the gas dissolved there, renewed through the liquid's film in τ = depth / `dissolve`
  (`Reactor::film`, `through_film`). A gas it takes is held at Henry's share (`Chemistry::henry`) × (1 + made·τ/eq) /
  (1 + used·τ/eq). Dead matter under liquid rots as fast as the film brings the oxidising gas.
- **In full pores** (no liquid standing): the decomposers' rot of a gas runs only in 1 − `inside::airless` of the node
  (the land's saturated share, and of the drier rest what it holds over field capacity, filling pores from the bottom
  up; `inside::aired`). Roots in full pores follow the same rule (3). So wet ground keeps its dead matter and builds
  organic ground, as peat builds (Clymo 1984).

### 2.4 Phases and the freezing law
- Phase by melting and boiling points (`Chemistry::phase`, `phase_in`).
- **Freezing point in a mixture** (`Chemistry::melt_in`, `t_melt`): 1/T = 1/Tm − R ln(x) / Lf, x the solvent's mole
  share among itself and its solutes. Only the solvent (the most abundant liquid) is lowered; its solutes are what
  melts below it (11, 12, 6); ice and liquid count together; gases do not count. The same law sets the ground's plateaus
  (`World::thaw`) and Change's phases (`Reactor::phases`).
- **Latent heat**: a gas in the air holds its melting and boiling heat (`chem::Heat::latent_gas`); moving liquid
  carries its fusion heat (`Heat::carried`).
- **Densest temperature** (`Chemistry::dense_in`; overturning, 1.1): the pure liquid's (`Substance::dense`) lowered in
  proportion to dissolved mass per mass of liquid (Despretz's law, `Substance::dense_drop`), no lower than its melting
  point there. Chemistry 0's sea is nearly fresh, so it freezes at its top as a lake does.

### 2.5 Pairs at balance and the sea's carbon
`change::reversible` finds the pairs of reactions where one undoes the other; `Reactor::balance` bisects for the
amount at which a pair runs at equal rates. In chemistry 0, 8 makes the dissolved salt 11 from lime, the carbon gas and
the liquid; 9 turns it back. Every cell that holds liquid starts at that balance with its air (`worldgen::terrain`), so
the liquid stores carbon gas and gives it back when the air thins, with no rule for the sea.

### 2.6 Chemistry 0 in short
One fixed chemistry (handover 6.0): 11 elements, 25 substances, 24 reactions (`worldgen/src/chem0.toml`). `core` and
`worldgen` know numbers only; Earth names are in `library/chem0.toml`, which the core never reads.
`Chemistry::validate` checks that every reaction and conversion keeps elements and mass.

| Number | Role (≈ Earth name, for reading only) |
|---|---|
| 0 | the key liquid (≈ water) |
| 1–4 | air gases: inert, oxidising, carbon gas, methane-like (≈ N2, O2, CO2, CH4) |
| 5 | the fuel, the light reaction's product (≈ sugar) |
| 6 | fat-like store |
| 7, 8, 9 | rock: sand, common rock, lime (≈ SiO2, CaSiO3, CaCO3) |
| 10 | body matter: C120 H198 N6 O107 Ca (≈ tissue) |
| 11 | rock salt, lives dissolved (≈ Ca(HCO3)2) |
| 12 | nitrogen mineral, lives dissolved |
| 13 | the frame or walls (≈ cellulose) |
| 14 | slow organic matter, `bound`: C59 H78 N6 O42 (≈ humus) |
| 15–18 | free elements 6 and 7 and their oxides (≈ iron, Fe2O3, copper, CuO) |
| 19 | element 1 alone (≈ charcoal, coal) |
| 20 | a crystal nothing dissolves (≈ NaCl) |
| 21, 22 | a clay and the clay baked dry (≈ kaolinite, metakaolin) |
| 23 | of the fuel alone; slows reaction 13 (≈ a tannin unit) |
| 24 | of the fuel and 12; slows reactions 1, 2, 7 (≈ amygdalin) |

| Reaction | What it does |
|---|---|
| 0 | light to fuel (the cover runs it) |
| 1 | burning the fuel; in the open world its fast rot |
| 2, 3 | burning and making fat |
| 4 | burning methane |
| 5 | rotting to methane |
| 6 | weathering of rock 8 with the carbon gas into lime and sand |
| 7 | burning or rotting body matter; gives back 12 and 11 |
| 8, 9 | the lime–salt pair (2.5) |
| 10 | fixing nitrogen into 12, paid by burning fuel (`Chemistry::fixing`) |
| 11 | 12 breaking down when hot, its nitrogen to the air |
| 12 | rot of the frame, by the decomposers only |
| 13 | splitting the frame into fuel (needs a gut's catalyst) |
| 14 | body matter to the slow matter 14, beside 7 at a quarter of its rate; gives back 11 |
| 15 | 14 rotting, 25 times slower than 7; gives back 12 |
| 16–21 | fast only in a flame's or kiln's heat: charring the frame to 19; burning 19; 19 freeing elements 6 and 7 from their oxides; baking the clay; element 6 rusting slowly back |
| 22, 23 | rot of 23 and 24 at the free fuel's pace; 23 gives back 12 |

**Conversions** (`Chemistry::build`, `builds`; life only, no rate, paid by work): body matter of 118 fuel + 3 of 12 +
1 of 11; the frame of 6 fuel; 23 of 15 fuel; two of 24 of 43 fuel and one 12. `Chemistry::minerals` are growth's inputs
no reaction burns (12, 11); `Chemistry::materials` the solids life builds besides matter (13, 23, 24);
`Chemistry::curb` the solid that slows a reaction. Nothing adds carbon gas from below: the air's settles where the
light reaction takes it up as fast as rot and burning give it back.

## 3. Life: the cover, light-eaters as a field
`core/src/cover/`. A cell's cover is a node of the store (`Class::Life`), kept with its takers in `life::Cell`; the
chunk of the cell is its one writer and its law books its own side. **Life's month** (`life::month`) runs after Flow's
air and liquid steps and before Change: per chunk the cover's month (`cover::step`) then the takers' (3b); then the
flames (3c); then seeds land (`cover::landed`) and young takers land (`fauna::land`). All moves go through the cell's
view (`store::Moves`) and `life::book`, with their heat; at the month's end the cover's heat comes to its ground's
temperature.

**What a cell holds** (`Cover`): slots in arrays of `cover::SLOTS` (16). A cell of one patch uses its first
`Config::variety`; a cell of more patches uses all, each patch at most `variety`. Per slot: the mol of five parts
(`life::Parts`, from the chemistry's roles: fuel 5, body matter 10, walls 13, and the curbs 23 and 24,
`Chemistry::curb`); 12 genes (`cover::LIGHT`, `LEAF`, `WALLS`, `STORE`, `FROST`, `MOIST`, `SEED`, `REPAIR`, `FIX`,
`GAS`, `SPLIT_CURBS`, `BURN_CURBS`; units `cover::GENES`, steps and ranges `cover::RANGE`); its starting kind (`root`);
its plants by size group; its line and footing; `net`, `full`, `patch`, `fell`. Per cell: seeds lying (`Lying`), cured
leaves (`cured`), patches (`Patch`), `idle`, heat `e`.
- **Masses.** Living cover = body matter / (1 − `WET_TISSUE`) (`Cover::living`). Built cover = body matter + walls
  (`cover::built_kg`); a plant's mass, height, crown and room go by it. Soft tissue = body matter with the curbs it
  holds (`life::SOFT`, `cover::soft_in`): they leave with it (shed, burnt, seeds), pay no upkeep, do not wear, and are
  no part of built or living cover. Walls pay no upkeep and do not wear, so capacity, capture area, upkeep, wear and
  fixing go by living cover (Ryan 1990). The cover's own burning does not read its curbs (Poulton 1990).
- **Genes.** The fixing and curb genes are off at or below a level (`cover::OFF`, `cover::on`), and off genes are alike.
  `cover::distance` is the largest gap of a gene over its unit (`cover::UNIT`: its step, a decade for the curbs);
  `cover::niche_distance` the same over `cover::NICHE` (all but the curbs). `cover::mix` means genes by mass; between
  two starting kinds the curb genes stay the heavier's.

**Where a slot stands** (`cover::ground`, `soak::footing`, `flow::shore`). A cell's ground is all of it, land and
ground under standing liquid, by height from the terrain's samples and the levels of its parts at the month's start
(`contract::levels`); before levels exist, its land (`contract::land`). A slot holds the ground above its line
(`Cover::line`, m over the cell's lowest ground). Plants do not move: a rising level stands them in deeper liquid.
- A slot founded by seeds takes at its first month (`cover::UNSET` till then) the line of all the ground its kind may
  hold (`Shore::found`: the highest level of its liquid parts less its roots' reach); with no ground above its line it
  holds the cell's highest (`Shore::holding`).
- `Cover::stands`: share of the cell's ground above its line; `Cover::rooted`: share whose loose ground holds its roots;
  `Cover::on` their product; `Cover::holds` that times its patch's share. Shares are nested over slots
  (`canopy::rings`).
- Roots work on land as its liquid lets them and under the liquid as far as their gas spaces reach (`soak::working`):
  swamp kinds stand out into the liquid (Kozlowski 1997).
- **Plants under the liquid past their roots' reach** (`Cover::airless`, `upkeep::airless`) gain nothing and pay
  upkeep at the yield of fuel without the oxidising gas (`units::FERMENT_YIELD`; booked burnt). `Cover::drained` sums
  the share of their store burnt (newcomers there come with all of theirs); at 1 they die as whole plants (`DROWNED`)
  and the line rises to where none stands so. Their burning slows with the cold, so cold floods kill slower (Kozlowski
  1997; Glenz and others 2006).
- **Ground below its line is taken by seedlings only**: those that join their stand below its line where their roots
  get the gas take ground at self-thinning density and the line falls to hold it (`soak::lowered`, `seed::lines`).

**Plants by size** (`cover/groups.rs`). One pool of matter per slot; its plants in up to `GROUPS` size groups, each a
real count (`Cover::plants`) and a share of built cover (`Cover::share`), smallest first. Walls go to groups by built
cover times the share of walls a plant of that mass builds, m / (m + `wall_kg`) (`stand::walled`; Poorter and others
2012), each at most all of its plant; the rest is each group's living cover and capture area (`Cover::sizes`).
- Founding seeds are one group, a plant per `seed_kg`, at most the self-thinning count (`stand::founded`). Joining seeds
  are a new group, or join the smallest where all are held (`Cover::recruit`). When slots join, groups of one mass are
  one and, while too many, the two smaller nearest in mass fuse (`groups::fit`; Moorcroft, Hurtt and Pacala 2001).
- Growth goes to groups by capture area times the light under all taller leaves of the patch (`Cover::growth`): size
  asymmetry for light (Weiner 1990). Deaths by upkeep, wear, frost or drowning are whole plants of every group alike;
  bites and shedding take mass of every group alike.

**Height and crown** (`stand::height`, `stand::stem`): walls make a stem of `stem_form` and `stem_solid`, as tall as
`tall` × width^(2/3), the buckling limit (McMahon 1973). The last group sets the slot's height (`Cover::height`). The
crown (`Cover::crown`) runs down from the top the depth its capture area fills at `crown_density` over the ground its
plants hold, no lower than the ground. Conducting walls (`stand::sapwood`): the pipe model, `huber` × capture area of
cross-section as tall as each group (`Cover::lift`; Shinozaki and others 1964), at most its walls.

**Self-thinning** (`stand::thin`, after the seeds wake and after building). At most (`thin_kg` / kg per m²)² plants a
m² (Yoda and others 1963): a plant of mean mass m holds (m / `thin_kg`)^(2/3) m² (`stand::plant_ground`). Plants crowd
those of their patch whose crowns share their layer, whatever slot holds them. Where the crowd passes a slot's ground
it loses plants, not mass, by the excess cubed, **the small first** (`stand::lose`: the smallest group dies until its
survivors match the next, then they fuse). The large plants lose no more than the crowd of the large plants sharing
their layer asks (Weiner 1990; Purves and others 2008).

**The month of a cell's cover** (`grow::live`): its plants follow the wet strips (`Cover::follow_wet`); each slot finds
its footing; its seeds lying wear, freeze and wake (`lying::month`); its stand thins; each slot's state is set
(`grow::Slot`; rooted share found again when its root need moved by more than a bin, `stand::moved`); light passes down
(`canopy::shade`) and the light reaction runs (`grow::light`); in order of their light, best lit first, each slot's
season, shedding, upkeep and deaths (`season::pay`); growth (`grow::grow`); strips shares move to where plants gained;
thinning again, alike slots merge (`stand::merge_alike`), empty slots go to the remains, cured leaves and dead walls fall
(`grow::fall`); books close (`life::book`).

**Light** (`canopy::shade`). The light caught since its last month per m² of capture area (`World::light`, already
through the rings and the liquid over them, 1.4) falls on its capture area (10^`LEAF` m² per kg of living cover) less
what its parts reflect (`Cover::reflectance`). It passes down the crowns by Beer's law, each slot's capture area spread
evenly through its crown, the light at each height absorbed by all crowns of the patch there. A slot on rooted share s
has capture area x / s on that ground; nested shares make rings, each held by slots of a share at least its own. So
tall kinds compete on deep ground only and bare bed holds only the smallest. Other patches do not shade.

**Roots** (`stand::need`, `stand::rooted`): a plant roots in `root_per_height` × its height of loose ground, at least
`root_least` (Schenk and Jackson 2002). A point holds it from `rooted_from` of that need (none) to the need (all),
smooth between. Each cell keeps its area in `stand::BINS` log bins of loose depth (`World::loose_bins`,
`cover::loose_bins`), and a kind holds their mean (`stand::rooted_share`).

**Drought** (`soak::soak`, `Pores::available`, `cover::activity`): w = (θ − wilt) / (field − wilt) within 0 and 1 (4),
θ the share of its pores filled by unfrozen key liquid (Flow's one store over the loose depth, standing liquid included;
frozen ground w = 0). Activity = min(1, w / 10^`MOIST`): full until about half is drawn (FAO-56's depletion
fraction), then linear to none at the wilting point (Ks; Allen and others 1998). A dry-ground kind stays active longer
and pays: its capacity per kg is 10^`MOIST` times its light gene's.

**Roots in full pores** (`soak::working`, `aired`, `breath`). Up to field capacity every root breathes; what is held
over it fills pores from the bottom up (`Pores::full`); the saturated strips are full. A root below that table gets the
oxidising gas only down its gas spaces, φ = `GAS`: L = (2 D C φ / ((1 − φ) M))^½ (Armstrong 1979), D the gas's
diffusivity (`units::GAS_DIFFUSIVITY`, Graham's law), C its concentration near the ground (`World::near_ground` / RT),
M what a m³ of root burns of it a second (`c_basal` at the burning's pace, `rho_body`), less the depth of standing
liquid. The share of roots above the table or within L below it works (Feddes, Kowalik and Zaradny 1978). Only working
roots draw liquid, so the light reaction is times that share (a flooded plant's pores close; Kozlowski 1997); activity,
by which it wears and hardens, is drought's alone.

**Wet strips** (`Cover::strip`, `soak::gains`, `strips_share`, `strips_moved`). Plants on land stand on the saturated
strips (`inside::wet_ground`; pores full, all available) or the drier rest (`inside::rest`); roots and activity run as
the mean by plants there. Seeds take hold on each as a plant gains there; what a slot stored goes to each by gain, so
plants die back where roots fail. Ground that turns saturated or drains takes its plants. A slot's strip share is at
most the strips' share of the land.

**The light reaction** (reaction 0, `grow::light`): stores `light_yield` of the light caught, at most 10^`LIGHT` ×
10^`MOIST` mol per kg of living cover per s × its pace (k e^(−Ea/T) over its value at `t_cat`) × activity, all times
(x − `gas_floor`) / (x + 2 `gas_floor`) over its value at `gas_ref`, x the air's share of its gas (`Physics::gas_factor`;
Farquhar, von Caemmerer and Berry 1980): the cover cannot strip the air below the floor. All slots are cut together to
the liquid their roots reach (`canopy::drawn`: each ring's liquid is its slots', shared by what each would draw) and to
the gas of the cell's share of its air node (`life::Books::room`). The light its aside node set aside (`Cover::keep`,
1.4) pays first; the rest comes from its ground, which that light warmed.

**Liquid for gas** (`around::thirst`, `around::pores`): per mol of gas in, its pores lose r = (D_v / D_g)(p_sat − p_v)
/ (`drawdown` p_g) mol of the key liquid (Cowan and Farquhar 1977; Farquhar and Sharkey 1982), p_sat at the ground's
temperature, p_v and p_g near the ground (`World::near_ground`, 1.5), D_v / D_g by Graham's law. The cover does not move
that vapour itself: it opens its pores as wide as its roots' liquid pays for and leaves the film (`Cover::film`) and the
share of the ground's liquid its roots reach (`Cover::reach`) to the ground's evaporation (`World::second_film`, 1.5).
A slot whose fine roots would cost all a leaf gains (`upkeep::root_share` ≥ 1) keeps its pores shut that month.

**Upkeep** (`upkeep::upkeep`, `upkeep_rates`), paid every month by burning fuel (reaction 1) at the burning's pace at T
over `t_cat`: living tissue keeps its liquid, so it breathes in a dry month too (Flexas and others 2006; Atkin and
Macherel 2009). A day:
- per kg of living cover, `c_basal` + `c_capacity` × what its capacity could store in a day, the capacity part dearer by
  1 + (key liquid's melting point − `FROST`) / `c_firm`; times the repair factor 1 − `c_repair` + `c_repair` 2^`REPAIR`
  (`Physics::repair`);
- `c_leaf` per m² of capture area, times the repair factor;
- `c_sapwood` per kg of conducting walls;
- fine roots: as many kg as the month's drawn liquid needs at `root_uptake` × (1 − φ) (`upkeep::uptake`; minerals come
  by mass flow, Barber 1995), breathing `c_basal` a kg, renewed `root_turnover` a year at `Parts::living_cost` (Gill
  and Jackson 2000). Gas spaces tighten roots against liquid as against gas loss (Colmer 2003; Garthwaite and others
  2008), so they cost more fine roots.
It burns no more than its fuel and the air's share offers; the budget splits what it paid over the seven parts.

**Deaths** (`upkeep::upkeep`), each on what the one before left: drowned (`DROWNED`); unpaid (`SPENT`); 1 − e^(−wear)
(`AGE`), wear = living and leaf upkeep × max(activity, `dormant`) / (living cover × `e_life` × 10^`REPAIR`) (Rubner
1908); frozen (`PHASE`, `cover::frozen`): the limit is `FROST` hardened (dormant) and the key liquid's melting point at
full activity, melt − (melt − `FROST`)(1 − a), and min(1, (limit − T) / `frost_k`) dies (`Physics::frozen`). Hardiness
is paid every month (`c_firm`), not given by dormancy (Levitt 1980; Sakai and Larcher 1987). Dead plants no taller than
`surface_m` stand as cured leaves (3c), their fuel to the ground. Large plants dead of all but wear add to `Cover::fell`.

**Season and shedding** (`season::pay`, `shed`, `floor`). Through a crown the light falls as e^(−z), z the capture area
above a leaf whatever slot holds it (`Slot::layer`); one more kg at depth z gains its crown base's gain × e^(y − z).
- **In season**: its top leaves' gain, less their fine roots' share, beats a kg of leaf's month of keep with the walls
  that feed it (`upkeep::keep_per_kg`). Top leaves are the crown's top whatever slots share it.
- **Full** (`Cover::full`): one more kg at its layer's bottom would not pay. It sheds the layers that do not pay (Monsi
  and Saeki 1953; Hirose 2005) and builds no more living cover.
- **Out of season**: no leaf pays until the season comes back, after `Cover::idle` × months a year (`idle`: share of the
  last year's months its ground held nothing for roots, frozen or dry; the cell's, so it outlives its plants). Before
  the month's upkeep it keeps the layers whose loss over those months is less than building them anew
  (`Parts::living_cost`; Kikuzawa 1991), no more than its fuel keeps through them; the rest it drops as leaves. With no
  gain its layers are alike: all or none.
- Shed soft tissue goes to the remains, or stands cured on plants no taller than `surface_m` (`season::shed_to`).
  Walls, fuel and plants stay. A leafless slot pays no upkeep, builds and exudes nothing; once a leaf would pay again
  (known from its `LEAF` gene) it builds leaves of its whole store.
- `Cover::net`: stored less upkeep per kg, a running mean over a year.

**Growth** (`grow::grow`, `build::per_kg`, `build::build`, `build::frame`), in season only:
- From the fuel over its reserve (10^`STORE` × built cover; none once all leaves are shed) and its share of the
  minerals, by the conversions of growth, paying `c_copy` a kg by burning fuel.
- **Minerals** (`grow::shares`, `build::wants`): each patch's slots share its share of the ground's. Where it holds
  enough, by what each would take; where short, by fine roots (Casper and Jackson 1997), none past its want.
- **A kg** (`cover::built_shares`): matter and walls at 10^`WALLS` × `stand::walled` per kg of matter (by group,
  `Cover::walled_new`), and of soft tissue the shares 10^`SPLIT_CURBS`, 10^`BURN_CURBS` as 23 and 24 (none where off).
- **A fixer** (`FIX` on) makes the mineral its growth lacks by reaction 10, at most 10^`FIX` mol per kg living per s at
  the reaction's pace and its activity, paid in fuel.
- **Walls alone** (`build::frame`, `frame_short`): then walls of the fuel over its reserve, up to those its plants hold
  (`Cover::walled_held`). A plant short of minerals, or full, still grows taller, and its stand thins.
- **Exudation**: in season the fuel over its reserve it could not use goes to its ground (Jones, Hodge and Kuzyakov
  2004), where the open world rots it.

**Seeds.**
- **Made** (`build::build`): the share `SEED` of what it builds, all of it when full; soft tissue first, then walls;
  each carries fuel at its `STORE` gene (short fuel makes fewer seeds, not lighter ones); one generation's mutation
  (`life::kin::mutate`: noise × step × `mut_size`).
- **Dispersal** (`life::lot`, `seed::send`): `seed::LOTS` lots, each from a point drawn evenly over its cell, moved by an
  isotropic Gaussian of `kernel_near` or (chance `kernel_far_share`) `kernel_far` (Clark et al. 1999); what lands in its
  cell or off the map stays. Draws are keyed by the cell and the genes to 1/`KEY_STEPS` of a step
  (`rng::Purpose::Cover`), not by slot index.
- **Landing** (`seed::land`): after every cell's month, cell by cell in order; their potential energy is booked once
  per cell and way (`store::life_sent`). They **lie** (`Lying`) in up to `GROUND_LOTS` lots at the cover's temperature,
  in the cover's node: a seed joins the most alike lot of its kind within `APART` × `alike` (`life::kin::kin`), else an
  empty lot, else the two nearest lots by niche join.
- **Lying** (`lying::month`, first in the cover's month): of each lot the share its wear gives dies (upkeep at full
  activity × `dormant` over `e_life` × 10^`REPAIR`) and the share frozen below its hardened limit. Of the rest min(1, a ×
  pace × working roots) **wakes**: a = (w / 10^`MOIST`)^`dormancy` (`Physics::activity`; full under the liquid), pace
  over its value at `t_cat`, working roots those of its first roots on its kind's ground (Bewley 1997). Seeds under the
  liquid past their reach wait.
- **Lodging** (`seed::lodge`), in each patch by its share: what wakes joins its kind's slots in the patch within `APART`
  × `alike` in every gene, shared by their built cover; else it founds a slot: an empty one while the patch has fewer
  than `variety`; else the patch's slot most in deficit (`Cover::net` < 0), whose cover dies; else the two slots of the
  patch nearest by niche genes, of any kind, join (`seed::room`) and it takes the freed slot. With `variety` 1 nothing
  joins and seeds wait. A holder that pays its way is never pushed out by weight: what lives is the light's to decide
  (Shugart 1984).
- A slot is bookkeeping: every law acts on the stand, so a slot cut into alike slots ends each month as the uncut one
  (`core/tests/stand.rs`). Selection is between slots and cells (`core/tests/cover.rs`).

**Patches** (`cover/patches.rs`). A cell's ground falls into up to `PATCHES` patches, each a share (`Patch::share`,
summing to 1) and an age; each slot stands in one (`Cover::patch`). A patch is a stand of its own (Watt 1947; Shugart
1984): its plants hold, shade, crowd, fill crowns and merge within it alone. Soil, liquid, minerals and air stay the
cell's, each patch drawing its share; light caught is found per patch (1.4); seeds lying are one pool. The laws know no
place of a patch; for pictures a patch past the first keeps one sample (`Patch::at`) and its ground is found from it
(`inside::patch_ground`).
- **Once a year** (`patches::year`): patches age; alike ones fuse, the younger into the older (same starting kinds hold
  them, `HOLDS`; tallest plants within `ALIKE_HEIGHT`; built cover per m² within `ALIKE_COVER`); slots meet their kind,
  a patch over `variety` slots folds (`patches::fold`); then gaps open.
- **Events** (`patches::hit`, `open_event`, `carve`, `settle`) are for a share of a stand hit at once, never one plant.
  An event that hits and leaves at least `EVENT_LEAST` of a patch makes what it hit a new patch of age 0 about a drawn
  sample (`patches::point`); one that leaves less hits it whole. Room is made first (`seed::room`); with all `PATCHES`
  in use the new patch joins the one most alike to bare ground (`patches::open`).
- **Gaps** (`cover/gaps.rs`): at the year's end each patch sums `Cover::fell` of its canopy (slots taller than
  `surface_m` reaching its tallest crown's base). Where that share is an event's part, it becomes a gap
  (`rng::Purpose::Gap`): each slot gives the plants that stood there (`Cover::part`), the canopy none of its large plants
  (already dead), so the gap holds the plants under them in full light. Fallen crowns are forgotten each year.

**Edits of the cover** (`World::edit`, `edit::Act`; handover 11): an edit runs the month-start passes, then acts on the
cell under its place (`World::cell_under`).
- `Clear` (`World::clear`): the share s becomes a new patch of age 0 about the edit's sample (`World::sample_in`), every
  patch giving s; s of every slot's amounts and plants goes to the ground with its heat (`Cover::cut`, booked as
  deaths); seeds lying, cured leaves and dead walls stay.
- `Split` (`World::split`): a control; s of the first patch becomes an alike patch, nothing dying (`core/tests/stand.rs`).
- `Plant` (`cover::place`): seedlings of a kind in the newest patch, as seeds lodge.
- `Grown` (`cover::place_grown`, `cover::grown`): the world's grown stand of a kind (its slot with the most built cover
  per m² of its ground), amounts and plants per m², from the outside node; as `Plant` where none stands.
- `Strike` and `Heat` are the flames' (3c); the ground's edits are in 0.
`ew run` edits at a year's start (`--clear`, `--split`, `--plant`); the page between months (wasm `brush`).

**Books and budget** (`cover/budget.rs`): what its parts and seeds hold as fuel (`Parts::content`) changes only by
flows booked where they move (`cover::BUDGET`, `Counters::cover_budget`): stored, seven upkeep parts, building, shed,
died, exuded, seeds, seeds died, takers, placed, rounded, burnt. What rounding leaves below zero is set to zero and
counted (`Counters::rounded`); more fails a debug assertion. For the tape only, read by no law: `Counters::seeds`,
`cover_lost`, `grown_last`, `ran_month` (`cover::Ran`).

**To the rest** (the contract, 0): it takes liquid and minerals from its ground and gases from the air, gives gases
to its aside node and its dead to its ground, each with its heat. Takers take a slot's soft part or frame or the seeds
lying (`contract::take`, `contract::Layer`), each part in proportion; work pressed on a cell breaks it
(`contract::press`). What stands on its cell (`Cover::stand`, `contract::Stand`: mass, capture area, absorbed and kept
light) shades (1.4) and holds the ground (1.3).

**Known limit**: a leafless plant's resting stem breathes nothing here; a real one spends its store slowly.

## 3b. Life: the takers, as a field
`core/src/fauna/`. A cell holds up to `fauna::FS` slots (`Fauna`, `life::Cell::fauna`), each one kind in two groups,
adults and young: per group a real count of members (whole below `N_WHOLE`) and the mol of stored fuel (5) and body
matter (10); per slot its heat, its balance (`Fauna::net`), 8 genes (`fauna::MASS`, `CHILD`, `YOUNG`, `ENZYME`,
`REACH`, `KEEP`, `COAT`, `REPAIR`; `fauna::RANGE`) and its starting kind. Living mass is body matter / (1 −
`WET_TISSUE`); that liquid is not booked. They are part of the cell's life node (one writer: the chunk), reach the land
only through the contract (0; `cli/tests/sides.rs`) and book as the life (`life::Books`, `life::book`). They live on
its land (`contract::land`); where it goes under they find nothing and starve.

**One month** (`fauna::step`), right after the cover's on the same cell, group by group (a young member is a body of
its own mass):
- **Body and pace** (`body::body`): a member stands at its `KEEP` gene where its ground (`contract::place`) is colder,
  else at the ground's temperature. Upkeep, gut and growth run at its burning's pace there over that at `t_cat`
  (Gillooly et al. 2001); work (walking, chewing, clipping) at the pace over that at `WORK_T`, at most 1, the
  temperature its rates were measured at (Bennett 1980, 1990).
- **Upkeep at rest**: (`c_basal` + `c_muscle`) m^`upkeep_exp` a day × pace × repair factor (Kleiber 1932; Elia 1992),
  plus `c_capacity` of the fuel its gut's capacity (`ENZYME`) could free.
- **Reach** (`take::layers`, `take::reachable`, `contract::slots`): each cover slot as its soft part and its frame,
  standing in its crown (`contract::crown`), and the seeds lying. A member of length L = (m / `rho_body`)^(1/3) searches
  ± `fauna::BAND` decades about `REACH`, no higher than `reach_len` L, no lower than `bite_floor` L over the snow
  (`contract::reached`); seeds only where its floor and band reach under `contract::GROUND_M`.
- **A jaw cuts** (`contract::cut`) a stem of width d where `jaw` m^`jaw_exp` N is the frame's stress for a cut
  (`Substance::shear`) over `stem_solid` π d² / 4. A stand holds (d / D)^`thin_exp` of its frame in stems thinner than d
  (Shinozaki and others 1964; West, Brown and Enquist 1999), D its main stem (`Cover::width`).
- **The take** (`take::scheme`; Holling's disk law, Holling 1959): it walks at v = √(`walk_froude` g L) (Alexander and
  Jayes 1983), a bite clips A = (`bite_breadth` m^`bite_exp`)² (Illius and Gordon 1987; Gordon, Illius and Milne 1996)
  in `bite_time` (Shipley et al. 1994), so it clips a = 1 / (1 / (v L) + `bite_time` / A) m²/s for `busy` of the day
  (T), and chews with P = `chew` m^`upkeep_exp`. A layer of D kg/m², w reached, b J to break a kg (`Shown::breaking`,
  `life::breaking`), gives I = T a w D / (1 + a Σ w D b / P) kg a day. In `take::SUBSTEPS` steps each layer falls by S
  (1 − e^(−k Δ)), k = Σ n I / S, at midpoint rates: never overdrawn, shared by rate. Then one bite of each layer for all
  (`take::bite`, `contract::take`).
- **Heat**: held above its ground it loses g (T_b − t), g = `meeh` m^(2/3) / (`r_air` + `COAT` `r_coat`) (Meeh 1879;
  Scholander et al. 1950). It makes its rest and its walk (`c_move` (m / `move_ref`)^`move_exp` J per kg and m; Taylor et
  al. 1982); where that is less than its loss it burns more, where more the excess leaves as vapour. At its ground's
  temperature all its heat goes to the ground.
- **Rest in the remains** (`fauna::shell`): outside `busy` it sits in the remains on its ground (`contract::remains`,
  shared alike by the cell's members, no more than it searches in a day), a spherical shell about its surface, radius r
  = √(A / 4π) to R³ = r³ + 3V / 4π, passing (1/r − 1/R) / (4π `litter_k`) K/W in series with its coat. It loses no less
  than it makes there.
- **Thirst** (`fauna::thirst`, `contract::liquid`): it drinks open liquid only (standing, run over the cell, and the soft
  solid at its fusion heat) and has the liquid of what it eats. Its need: cooling vapour plus `drink_least`
  m^`drink_exp` kg a day (Nagy and Peterson 1988), which goes back to the cell unbooked. Slots share by what they ask. A
  slot still short walks to the nearest cell whose open liquid holds its shortfall and back (`fauna::way`,
  `contract::to_liquid`), as often as `dry_most` allows, paying the walk. Still short, its work is cut (`thirst::work`);
  only where at no work it is short do members die, by the unmet share (`LACK`). Vapour leaves the cell's liquid then
  the soft solid for its aside node with its latent heat (`contract::vapour`). Fetched liquid is taken from the far cell
  after every chunk ran (`contract::fetched`).
- **Digestion** (`gut::digest`): the gut splits walls into fuel (reaction 13, with the ground's key liquid) up to
  capacity × living mass × pace, slowed by what it took that slows the splitting (23 in chemistry 0;
  `Reaction::inhibited`, per kg of what it took). It burns first the fuel it took and split, then body matter it took
  (reaction 7, minerals to the ground), then its store, each within `Books::room`. Unburnt fuel goes to store; unsplit
  walls and other solids to the ground; unburnt matter to growth.
- **Deaths** (`month::settle`, `month::die`), each on what the one before left: unpaid (`SPENT`); 1 − e^(−wear) (`AGE`),
  wear the energy burnt per kg over `e_life` × 10^`REPAIR`; frozen (`Physics::frozen`, the key liquid's melting point
  lowered by `harden` (1 − min(1, pace)), `PHASE`); thirst (`LACK`). Below `N_WHOLE` the dead are a binomial draw and a
  count becomes whole by a draw that keeps its mean (`grow::whole`). Their matter and store go to the ground.
- **Growth and births** (`fauna::grow`). The pot is `YOUNG` of the fuel each store gained plus what each store holds over
  `store_most` of its living mass. It pays `c_copy` a kg of body matter built; the matter is what the slot took and did
  not burn. A member builds at most Kleiber's rest over `Parts::living_cost` (`grow::cap`; West, Brown and Enquist 2001;
  Gillooly et al. 2002).
  - Young grow at their cap; the share that crosses the adult mass joins the adults (von Foerster's flux, masses even in
    log between the newborn's, `CHILD` of the adult's, and the adult's: min(1, Δln m / ln(M / m0))).
  - Adults bear newborns of `CHILD` of their mass. An unborn grows by dm/dt = a m^(3/4)(1 − u), u = (m / M)^(1/4), its
    mother paying all its burning, 4 E_m ∫₀^u_c u³ / (1 − u) du / u_c⁴ per kg of newborn (`grow::unborn`), u_c =
    `CHILD`^(1/4); she bears at most u_c / (4 (−ln(1 − u_c))) of her cap (`grow::bearing`).
  - Below `N_WHOLE` maturing is a binomial draw, newborns the whole part plus one by a draw; a draw never makes matter.
    An empty group gives its amounts to the other; the pot's rest goes back to the stores up to their most, then to
    the ground. A slot of no members is empty.
  - One keyed stream per slot and month (`rng::Purpose::Fauna`), in order: deaths of adults, of young, maturing, births,
    dispersal, mutation.
- **The young leave** (`land::disperse`, `fauna::land`) in lots (one per member under `N_WHOLE`, else 8) drawn by the
  seeds' kernel (`life::lot`); lots for another cell wait in the outbox with their heat and land after every chunk, cell
  by cell, by the slot rule (a slot in deficit they take dies), whole by a keyed draw; with no land or room they die
  there (`contract::died_at`). Their potential energy is booked (`contract::travelled`).
- **Mutation and the slot rule** (`fauna::lodge`, `life::kin`): a month's newborns are one packet with the slot's genes
  after one mutation. It joins its kind's slot within `APART` × `alike` in every gene (means by living mass); else founds
  one: empty, else the one most in deficit (`Fauna::net` < 0; not its own, nor one whose newborns wait), whose members
  die as remains; else it stays in its parent slot unmutated. Slots alike in every gene merge (`lodge::merge_alike`);
  they never join because all are held (a mean of two kinds is good at neither).
- **Books**: each slot's heat ends at heat capacity × body temperature; the rest goes to the ground. Its mass lies at
  its cell's head (`flow::stack`); it stands on the cell as nothing (`contract::Stand` is the cover's).

## 3c. Flames in the remains on the ground
`core/src/flame/`. Once a month after life's month and before the seeds land (`flame::month`), the remains on a land
cell (its ground's body matter and walls, `contract::remains`) may burn. What burns into what and its heat come from
the chemistry's burning reactions (`Chemistry::burner`); the speed comes from the bed's heat balance, not the
reactions' rates (those of rot): surface fires are limited by heating the next fuel (Rothermel 1972).

**Cured leaves**: slots no taller than `surface_m` keep shed leaves and their dead plants' matter and walls standing
(`Cover::cured`, `season::shed_to`, `upkeep`), as herbs' dead stems stand through a dry season; they fall with mean life
`cured_life` (`grow::fall`). Taller plants drop theirs at once.

**1. The bed** (`flame/bed.rs`, `flame::beds`, per land cell in parallel, one per patch):
- Remains and cured leaves are one pool; each patch holds its share of the cell's cover's body matter (by its area
  share where there is none). Load w kg per m² of land; ground covered 1 − e^(−a w), a the capture area per kg dry of its
  slots' leaves (`bed::leaf_per_kg`).
- **The layer** (`bed::layer`, the cell's): its relative vapour h runs in a line from the air near the ground at the
  top (`World::near_ground`) to, at the bottom, between that and saturation by s, the share of what the drier land
  holds for roots that it holds (Manabe 1969). The saturated share (`inside::saturated_share`) carries no flame. Snow
  (`contract::Place::snow`) hides the remains.
- Liquid held: the sorption curve `sorb_scale` (−ln(1 − h))^`sorb_power` per kg dry (Nelson 2000; h ≤ `SORB_HIGHEST`),
  plus as duff `duff_hold` × s × the share of its depth.
- A burning kg frees h_c; lighting one takes Q = c (`t_light` − T) + M (c_l (T_boil − T) + L_v, + L_f below melting), M
  what the curve holds halfway down the flaming part.
- **It flames** from its top to where the liquid reaches the moisture of extinction (`flux_ahead` h_c − c (`t_light` −
  T)) / (c_l (T_boil − T) + L_v); cured leaves flame wholly where the top does, as a bed at `cured_rho`. **It smoulders**
  on down to where all its liquid reaches `smoulder_moist`.
- **It carries a flame** (`Bed::carries`) where its remains cover more than the square lattice's site-percolation
  threshold (`flame::CARRY_SHARE`; Newman and Ziff 2000) and the flaming load is at least σ (`t_burn`⁴ − T⁴) τ / (h_c −
  Q) (`bed::least`; Cheney and Sullivan 2008), τ the particles' flaming time. `flame::Unlit` says why not.

**2. Strikes and runs** (`flame/join.rs`, `flame::shares`, serial, in cell order):
- Edge-sharing cells that carry a flame form regions of A m² covered. A region expects `strikes` × A × the month's share
  of a year; the count is a Poisson draw, each lighting a cell drawn by its covered ground (`rng::Purpose::Flame`,
  keyed by the month and the region's first cell).
- A flame runs from its cell over the region for `front_life`, crossing half the way between two cells' centres at
  each one's speed (its patch covering most, `join::lead`), the earliest first (`join::run`, Dijkstra). Every cell it
  reaches burns all the covered ground of each patch that carries a flame.
- **Wind** (`flame::downwind`): along a way the speed is R (1 + φ_w), φ_w Rothermel's (1972) wind factor of the low
  wind (`World::low_wind`) along that way (none against it) at mid-flame: Albini and Baughman's (1979) share
  (`bed::sheltered`; Andrews 2012), over open ground by the flaming depth, under a canopy by its height and crown fill,
  weighted by the cover of slots taller than `surface_m`; particles' surface to volume 4 / `fine_d`.
- A burnt patch carries no flame until its leaves rebuild its remains: a firebreak, from which the mosaic of burn ages
  comes.
- **Edit strikes** (`edit::Act::Strike`, `World::strikes`, `flame::lights`) light their cell in the coming month, or at
  once when edits settle (`World::settle`, `flame::now`: no time, no drawn strikes, shown as the coming month's).
- **Heat added** (`edit::Act::Heat`, handover 11.1, `World::heated`, `bed::heated`): it boils off the liquid the remains
  and cured leaves hold over their whole depth (no more than the ground holds), then warms them to `t_light`: q J a m².
  Heat e over the share s lights s × min(1, e / q), off the saturated share, on the covered share; that bed burns whole
  and dry (`Bed::kept`) even where it would carry no flame. The vapour goes to the aside node with its heat and latent
  heat, the rest of the heat taken to the air; unused heat stays in the ground (`flame::boiled`). `flame::needed` caps
  what an edit adds.

**3. The burn** (`flame/burn.rs`, `flame::burn`, one task per chunk):
- The share f of a patch burnt is an event (3): a part becomes its own patch (`rng::Purpose::Burn`) and all its slots
  burn; a patch burnt whole is made new; under `EVENT_LEAST` its slots burn on f. Slots of a patch a flame made or burnt
  whole forget their fallen crowns.
- On f the cured leaves and the flaming remains burn, then the smouldering part. The flame moves at R = `flux_ahead` h_c δ
  / (τ Q), δ the flaming depth (cured leaves at `cured_rho`, remains at `litter_rho`), τ = 8 minutes an inch of `fine_d`
  (Anderson 1969, `flame::residence`). Intensity I = h_c w R (Byram 1959), flame `FLAME_K` I^`FLAME_EXP` m (Byram 1959),
  scorch `SCORCH_K` I^(2/3) m (Van Wagner 1973), I in kW/m (`burn::reach`).
- **Slots** (`burn::crown_burns`, `Cover::die_under`): a size group no taller than the scorch height dies above the
  ground on f. One no taller than `surface_m` keeps its plants, store and base (`base_share`) and resprouts; what stood
  above burns. A taller one loses f of its plants and built cover: soft tissue and fuel to the ground, walls standing dead
  on the patch (`Patch::snag`, `snag_h`, `snag_age`), falling with mean life `snag_life`. Of slots taller than
  `surface_m` the living crown burns in the flame and dies up to the scorch height (Bond and Midgley 2001). Seeds lying
  survive.
- **Minerals** (`burn::break_down`): each mineral the burning gives breaks down by the reaction that takes it alone over
  1 − e^(−k τ), k at `t_burn`: 12 by reaction 11, its nitrogen to the air (Raison 1979); 11 by reaction 9, to lime.
- Gases to the aside node, the rest to the ground as ash, through `life::book` within `Books::room`; the burning's heat to
  the air node over the cell.
- For the tape: `Counters::flame` (`flame::FLOWS`), `flame_air`, `cover_lost` (`life::FLAME`), the budget's `burnt`,
  and per cell `flame::Flamed` (`flame_month`, `burnt_months`, `burnt_age`, `burnt_last`); two burns of a cell in a month
  are one.

**Known limits**: dead leaves keep all their nitrogen (real plants pull back about half; Aerts 1996). The slow matter
14, the fuel and the curbs in the remains do not burn. Smouldering has no depth or speed of its own. A natural flame
boils off no liquid (its heat balance counts it). The scorch height has no term for air temperature or wind. No crown
fire, no bark; return times come from regrowth and strikes alone.

## 4. Soil and minerals
`core/src/soil/`. The loose ground of each terrain sample has a **texture**: fine (under 2 µm), middle (2 to 50 µm) and
coarse (50 µm to 2 mm) grain, each even in log size (`soil::CLASSES`; Soil Survey Staff 1993). A cell also has an
organic share. No table places a kind of soil.

**Texture** (`soil/texture.rs`, `soil::samples`, `texture_at`), three laws over what the start's wear left (0):
- **Grains of the bed**: a make breaks into coarse grain by its substances' hardness, middle by the rest, by volume
  (`Make::coarse`, `worldgen::makes_of`).
- **Weathering**: the share a reaction of the open world takes with a starting gas into another solid (`Make::weathers`;
  rock 8 by reaction 6) turns its middle and coarse grain fine as 1 − exp(−t / `WEATHER_YEARS`) (White and Brantley
  2003), t the time the loose ground has lain (`Samples::age`: depth over renewal rate, at least `mantle::SLOWEST`). A
  substance that weathers only into liquid or gas leaves no fine grain. Steep thin ground stays coarse; old flat ground
  on a weathering bed is fine.
- **Sorting**: what settled on a sample (`Samples::settled`) holds grain that sinks faster than the flood's shear
  velocity u* = √(g H S) (Bagnold 1966), sinking at R g D² / (18 ν + (0.75 R g D³)^½) (Ferguson and Church 2004;
  `Make::buoyant`, `units::LIQUID_VISCOSITY`). The flood is `FLOOD_RUNOFF` from the area draining through it, as deep as
  a channel carries it (`delta::flood_depth`); under standing liquid it is still. It settles the load between the least
  size that sinks there and the least the flow upstream carried on, a span at least `SORTING` wide (else the nearest
  class). Channels and fans are coarse, pond and sea floors fine (Hjulström 1935).

**A cell's texture** (`World::soil`, `soil::of_world`): its samples' mean by the loose ground each gives its ground node
(at most `skin_m`). Load that settles later (1.3) mixes in by volume to `skin_m` (`runoff::load::mix`); pores are found
again at a month's start where field capacity moved more than `TEXTURE_STEP` (`soil::month`).

**Organic matter** (`soil::month`): the share of the volume of the ground node's solids (solid at `T_SURF`) that is of
life's parts or `Substance::bound`. It is no store of its own: it is what the ground holds.
- Fresh dead matter from the cover and takers rots by the open world's reactions (2.1), faster warm and moist, slower
  under liquid or in full pores (2.3).
- **The slow pool**: a fifth of the body matter that rots becomes 14 (reaction 14), which rots 25 times slower
  (reaction 15), slower again by `Texture::shelters` = `SHELTERED` × the fine and middle share (Parton et al. 1987; Six
  et al. 2002). It builds over decades to centuries, most on fine ground, holding its nitrogen.
- Dense cover makes ground richer in organic matter that holds more liquid; bare ground loses it. Where the share moves
  more than `ORGANIC_STEP` of itself, pores and Flow's state are found again (`flow::new_pores`), the potential energy
  of the moved liquid to or from the ground's heat.
- `soil::holdings` reports them for the shells; no law reads it.

**Pores** (`soil::Pores`, `World::pores`): porosity θs, field capacity θ33 (−33 kPa), wilting point θ1500 (−1500 kPa),
conductivity when full Ks. The mineral part by the regressions of Saxton and Rawls (2006) on coarse share S and fine share
C, held inside their fitted range: θ1500 and θ33 from S, C, S·C and one with a second step; θs = θ33 + the fitted share
over field capacity − 0.097 S + 0.043; Ks = 1930 (θs − θ33)^(3 − λ) mm/h, λ = ln(θ33 / θ1500) / ln(1500 / 33). The
organic part is decomposed organic ground (`ORGANIC`). They mix by the organic share: θ by volume, ln Ks by volume. Flow
keeps field capacity and wilting point as shares of the pores. `Pores::available`: the share of what ground holds for
roots; `Pores::full`: the share of ground whose pores are full.

**Minerals and exchange sites** (`soil/exchange.rs`). The minerals (`Chemistry::minerals`) live in the ground's liquid
and move with it, so rain washes them to low ground. Fine particles and organic matter hold a share on their sites
(`SITES_FINE`, `SITES_ORGANIC`, each site holding the minerals of `HELD_PER_SITE` m³), a linear balance with what stays
dissolved (Brady and Weil 2008): the share V / (V + `Soil::holds`) stays dissolved in V m³ of liquid
(`soil::dissolved`). `Soil::holds` is found each month's start; Flow finds the share when it builds the ground's state
in full and keeps it (`Scratch::dissolved`, `flow::dissolved`); in a cell of parts (1.1) each part by its own sites and
liquid, so a liquid's top keeps its minerals dissolved. Only the dissolved share moves with liquid; the rest is ground
everywhere Flow reads it (mass, no volume, no pores, does not move). Roots take from all of it. So coarse ground loses
its minerals to the rain; fine and organic ground keeps them.
- **Nitrogen mineral 12**: placed when life starts per m² of open land (`worldgen::life::soil`, the chemistry's
  `init.soil`); then fixing adds it, rot (7, 15, 23) returns it, heat (11) sends it to the air.
- **Rock salt 11**: made in wet cells from lime (8), turned back by 9, near balance (2.5); growth's second mineral (one
  rock element per body matter); rot (7, 14) gives it back; a flame leaves it as lime (3c).

## 5. Starting conditions (not laws)
Placed by `worldgen` (the plan `worldgen::schedule::Schedule`, carried by a snapshot), printed by every run; the laws
never read a kind.
- **Terrain** (0): a core function (`space::terrain`) that `worldgen::terrain` builds the cells on: an island of drawn
  rises or one rise (`--shape`), or a globe with `LAND_SHARE` of its area above 0 m. The bed is of the chemistry's
  `makes` over the samples the wear left in each (`space::strata`). The ground node holds the mean of the top `skin_m`
  of each sample's loose ground, of its bed's make (a residual soil); what lies deeper is the bed's store
  (`Class::Deep`). A cell's height is the terrain's mean over it (`Terrain::inside`).
- **Cells**: on an island, open water far from land (`LAND_RING`) is one cell an air block (`coarse_low`), split so
  neighbours differ by one size at most; a globe likewise but for a rectangle in squares (`--fine`). Ground and air
  temperatures fall with latitude and height.
- **Starting liquid** (`World::full_bodies`): the open low ground at 0 m, every body with a pass full to its pass (1.2),
  a cell with no pond's samples to what dips under 0 m. Such cells have full pores, start `MOLTEN_MARGIN` over the
  liquid's melting point, at balance with the air (2.5).
- **Open land** (`flow::land_shares`): soil and takers start per m² of land, each cover kind per m² of the ground it
  stands on (`cover::place`, `cover::footing`).
- **Life** at `--life-year`: first the soil (nitrogen mineral), then the cover (`worldgen::life::KINDS`: kg per m² and
  genes, on each open land cell by its chance, each kind keyed by its own key). Without `--roster` the herb only
  (`ONE`); with it the six rows (`ROSTER`): small fast herb, herb, big slow cover with walls (`TREE`), the same of wet
  ground (`WET_TREE`), woody cover of dry ground holding 23 (`DRY_WOODY`), tall fast cover with a large seed share holding
  24 (`TALL_FAST`). `--spray` rounds of mutation (`Seeding`); `FIXERS` of the cells carry the fixing gene at `FIX_GENE`
  (`--fixers`).
- **Takers**, only with `--fauna-year` (none by default, none on the page): the five `worldgen::life::TAKER_KINDS` (30
  g, 3 kg, 60 kg and 400 kg kinds that keep a mammal's heat, and a 2 kg kind that keeps none) on every open land cell,
  all adults, at `START_SHARE` of Damuth's line of members per area against mass (Damuth 1981), whole below `N_WHOLE` by
  a keyed draw, with `START_STORE` of their living mass as fuel, at body temperature, from the outside node
  (`worldgen::life::place_takers`, `fauna::place`, `contract::placed`).
