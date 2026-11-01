# The laws as they are now

What each law does, its rule, its physical reason and where it lives. How the code does it (rounds, caches, buffers,
orderings) is said once, in the code's doc comments; the code is the truth. Numbers, units and anchors are in
`docs/numbers.md`, what a run records in `docs/tape.md`.

**Where the code is** (`core/src/`): time in `clock/` and `schedule.rs`; the shape of the world in `space/`; shared
maths in `solve/` (`spread`, `downhill`, `band`); nodes and books in `store/`, `world.rs`, `check.rs`; ground heat in
`ground.rs`; settings in `params.rs`, constants in `units.rs`; bodies in `body/`, `life.rs`, `nodes.rs`, `lineage.rs`,
`brain.rs`; the cells as bodies see them in `survey.rs`; paths in `lanes.rs`; keyed random numbers in `rng.rs`; the
tape's events in `event.rs`; threads in `par.rs` (the same results with and without).

**Words.** A **cell** is a square of the map (2 km on the island, latitude-longitude on a globe) or a quarter of one
in a refined region. A **node** is anything that holds stuff: a cell's ground, an air column, a body, a part. A
**body** is a living node with genes and maybe a brain; a **member** is one living thing in it (one member: an
**individual**; many: a **population node**). A **gene** is a number a body inherits, often log10 (1 means ×10) and
"off" below 0. The **key liquid** is the liquid a body's chemistry runs in (substance 0). **Light-eaters** (plant-like)
are not bodies but the **cover** of each cell, a field (4.5); bodies are **eaters**.

## 0. The world, one step, the books
**The map** (`space/`, `World::mesh`): cells, links with widths and headings, and a coarser air level. The laws step
only on links. Each link takes one of 8 compass directions (`Level::way`), so the brain's 8 directions hold everywhere.
An island of 2 km squares, or a globe (`--globe`) of latitude-longitude squares on a sphere of 6,371 km whose rows wrap
east to west. A rectangle may be refined at the start (`--region`, `Square::refine`): each square becomes four
quarters, linked to a square beside them at the distance between centres across the border, so a steady change
across it passes exactly.

**One step** (`schedule::step`, a month; time in whole seconds, the calendar from it). Processes run by their periods
(`clock::Every`) in the order of `schedule::processes`: every `phys_days` the survey (`survey::survey`: one task per
domain on the pool with the feature `threads`, in each of its cells the wear of movers, the monthly merge, the shade and
the sums; a death or a merge that lets go of a part queues that in the domain's outbox, applied after the tasks as in a
batch; at a month's start Flow's state is rebuilt in full for the cells' surroundings, and its own process leaves it when
it would be in full then), the parts of Flow that are due (air every `air_hours`; after it, every physics step, the
sink of the liquid over field capacity into the bed's store; liquid and erosion every `flow_days`, always on a physics
step; heat sideways and creep yearly) and Change; on the month's first day Bond (the turns of the parts: each
domain's in slot order on its view, the domains on the pool, their moves booked in the order of the domains; then the
loads of the structures one by one); then the bodies (`survey::batch`).
**Every body updates once a month**, at the month's first day, after that day's Change, air step and Bond: one batch
holds every living body, and a body placed during it first updates at the next month's start. Each update integrates
the time since the body's last (`Body::last`): its upkeep, drink, heat, growth and the most it takes or breaks go by
those days, so the month is a time step, not a calendar: a season's change comes from temperature (6.2). The bodies
read the ground and the air as that day's Change and air step left them, and what they leave on the ground (the free
solid they shed, remains) goes through the month's steps of Change before the next batch can take it, so most of the
free solid has rotted by then (it rots in about a week, 2.1). Every body decides at every update (5.3).
**Bodies and their order** (`life`): the cells are cut into domains, the cells under one air node; each domain stores
the bodies of its cells, and a body that moves into a cell of another domain moves to its storage. A body's id is the
domain it was placed in and that domain's count of bodies placed (children, split halves, parts of nodes acting apart,
members drawn); it never changes, and its random draws are keyed by it. The bodies
of a batch update domain by domain, cell by cell, and in a cell by id; sums over bodies go domain by domain, cell by
cell, each cell's bodies in the order they came. No order depends on where a body is stored.
**What a body sees in a batch** (`survey::batch`). At a batch's start each living body leaves a face (`survey::Faces`:
its id, members, visible traits, signal, well-being, and the room it is in), domain by domain, cell by cell. A body
perceives the others only by these faces, as they were then, whatever they did in the batch since; a body placed in the batch is seen from
the next. It takes from or gives to a body it saw only if that body still lives, is still in its cell and is not
leaving it; the partner of its children and the cover over its cell are the bodies there now. It reads the store
through its domain's view (`store::Moves`): the ground, bed and bodies' node of each cell of its domain and its air
node, as the moves of the bodies before it in the batch left them; a node of another domain as it was at the batch's
start.
**A batch's two phases** (`survey::batch`, `outbox`). Body code runs on a domain taken out of the world
(`life::Domain`), the world read only: it writes the domain's bodies and lists, its store view and moves, its events
and counters and its outbox; the lineages and born rows it makes get provisional ids of the domain, and the changes of
their holders wait there. When the domain is back the world settles (`World::settle`): its events and counters are
added, those ids made global in the order made; the holders' changes are added at once outside a batch, at its end in
one, and a lineage left with no body dies out, born rows left with no holder are freed. Every serial step, a body or
part law run on its own (a part act, `copy::back`), runs so
too, on the domain of its cell (`World::in_domain`). First the parallel phase: the bodies due that hold no part and are
inside none update, each domain on a thread of the pool with the feature `threads` (`survey::run`, the domains with
the most updates first), one after another without it or when the phase is small (`par::SERIAL_BELOW`). What an
update does outside it goes to its domain's outbox
(`outbox::Entry`), in the order made: every move to another cell, inside the domain or across its edge (`choose::go`
pays and queues it), children who start in another cell (`copy::emit` takes them from the parent), a watch, and every
act on a part (Bond's verbs, form and anchoring among them; a body that lets go of its part, or dies or joins another
holding one, lets go there too). A body
with a move queued, or a node's part with a part act queued, is **leaving** (`Body::leaving`): it stays in its old
cell until the batch ends, where the rest of its update runs, but it does not mature, and the others there neither see
it in their cell nor take from it, give to it, join it or mix with it. A body with a move queued, or a part act but a let-go or an anchor, does not
split in that update; when its domain's task ends, a body with a move queued leaves the domain's storage with its move
(`Domain::post`). Then, in a fixed order: the domains back, and the world settles, domain by domain. Then the landing
phase (`outbox::land`): the moves and the litters land, each in the domain of the cell it goes to, in the order of the
domains they come from and then the order made, each domain on its own on the pool as in the parallel phase; beside
them in the same call the moves of the domains that ran are booked, each array of the store by one task in domain
order (each number a domain's moves touched gets their sum once, `store::book_by_array`; a landing reads no store);
the domains back, and the world settles once. Then one by one, each a serial step: the other entries of the outboxes (watches, part acts), domain
by domain in the order made, then what landing and applying queued, the same way (in each round the moves gathered
before are booked first, beside its landings); the bodies that hold or are inside a
part, whole updates in domain and body order, each a serial step on its domain with its view opened fresh and booked
after it, its own outbox delivered after it in the same phases; last the holders' changes added in domain order. Each
entry is applied by its law on the bodies and parts as they are then. A move lands (`choose::land_move`): the body
enters the cell with its marks of light and rain, carries what it holds if it paid to (else it lets go of it), counts
in the crossings of the cell's domain, and its push breaks the bodies there at the next survey; members of a node who
moved join their kind there (`copy::kin`); after the landings its push reaches the structures there, the structure it
carries comes along, and it clears the lane (`Entry::Passed`). A litter joins its kind there or starts a body, and its
birth is recorded then (`copy::land_litter`). A watch blends by the share of the watcher's
members who watched (5.5). A part act runs whole on a part in the body's cell (`choose::land_act`); a node's part then comes back
to its node if it holds nothing, is inside nothing, and the node is in its cell and not leaving it. An entry whose body or target is gone does nothing: a move
of a body that died (its cost was paid), a watch whose watcher is no longer of its lineage, a part act no longer
possible or that cannot be paid; less than one member of a node's children with no node to join goes back, after the
landings, into the parent's brood, or to the ground of the parent's cell if it died (`copy::back`). So a body is seen as it was at the batch's start,
and every move to another cell, every watch and every act on a part lands at the batch's end.

**Sleep and wake** (`clock::Sleep`, a speed tool): a calm node sleeps until an alarm or until an amount grows by more
than `wake_share`, then its law brings it up to date over the whole time since.
**One ground temperature**: every law and the tape read that of Flow's last step (`World::t_ground_last`).
**Checks before a run**: `Config::validate`, `Chemistry::validate` (every reaction keeps its elements and mass).

**The books.** Every move takes from one node exactly what it gives another, with its heat; `store::apply` and the
bookings of the bodies' moves and of Change's rows (`store::book_by_array`, which adds as `apply` adds) are the store's only writers; the heat of the boundary nodes and of the potential, a whole history's sum, gets each booking's changes summed apart and added once (`Store::add_far_heat`). A move out of the bed, which holds far more than its loose ground, takes only what the bed can
lose exactly (`Move::loosen`), so no amount below the bed's rounding reaches the ground for free. A ground node under
deep standing liquid holds far more of it, and an air node far more of the main gases, than a body moves: what rounding takes from their
amounts is kept apart and counted in the books (`Store::carry`, Neumaier 1974). Boundary nodes stand for the sun, space and outside (`store::Boundary`, edits by `World::cross`);
potential energy is one more node (`Class::Potential`): a lift moves heat into it, a fall takes it back. So every
element and the energy are kept; `check::drift` checks it yearly and must stay near rounding.
**No overdraw.** A law never asks a node for more than it holds; where it could, it cuts that node's own outgoing
links (`solve::sweep::within`), never a whole field. A body takes no more than its domain's view holds, what the
bodies before it in the batch left (checked in debug builds). `apply` and `book_by_array` only add and set rounding below zero to zero, counted
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
width over length (Patankar 1980; `flow::land::conduct`): rock spreads a hot spot one cell in about 70,000 years, so a
year is far below the explicit limit.
**Melting plateaus** (`ground::melting`): a node holds the fusion enthalpy of what is molten in it, so it stays at a
melting point (lowered by solutes, 2.5) until that substance has all frozen or melted. A node whose heat melts or
freezes all of it within a step stays there only for the share of the step that takes, then goes on from that point,
and a soft cover that melts away holds its surface at the melting point only until it is gone (`air::radiation`), so a
long step ends where short ones do.

### 1.2 Liquid
`flow::liquid`, every `flow_days` (60): inside a month, counted from the month's start (a whole number of physics
steps); over a month, counted from the world's start (a whole number of months); so always on a physics step, after its
sink (`Config::validate`). The sink into the bed's store runs every physics step (`flow::sink`).
- **Pores first**: the loose ground's pores (`porosity`) fill first, under any soft cover; only liquid above them stands
  and has a head (`flow::land::stand`), the depth bodies and Change read.
- **Spill levels**: a priority flood over the ground gives each cell its spill level (Barnes, Lehman and Mulla 2014;
  `downhill::Order`). Liquid stands below it or in the lowest basin; elsewhere it passes on, counting as no higher than
  its ground, so it crosses the map in one step while depressions fill and spill.
- **Running off** (`solve::downhill`): each cell sends its liquid down its steepest way by D-infinity (Tarboton 1997;
  `Level::steepest`): two neighbours next in compass order make a triangle with the centre, the plane through their
  drops gives its slope and way, and the steepest triangle shares the liquid between its two neighbours by angle.
- **Pore water** flows by Darcy's law (one explicit `solve::sweep` on the liquid table, the store's too): `seep` ×
  saturated thickness × width / distance, at most half of what would level two cells, and no cell giving more than it
  holds over field capacity; pores at or under field capacity (`field`) give nothing. Over cells of kilometres a
  liquid step moves a table a thousandth of the way to its neighbours', far below the explicit limit. Liquid above it sinks to the deep store
  (`drain::sink`, at most `seep` a second and the store's room) every physics step, right after the air's step that
  brings the rain: held until a long liquid step, it would cover and darken the ground (`land::reflectance`, 1.4) and
  warm it. The store flows by `bed_seep`. Real land drains to channels a few hundred metres apart, so both also drain by Darcy's law down
  the cell's steepest slope over a hillslope of `hill` into the runoff (`flow::drain`).
- **What goes with it**: its share of every liquid and of what is dissolved, with heat. Liquid passing through a node
  keeps its fusion heat whatever the node's phase and leaves at the temperature of the node mixed with what came in
  (on a melting plateau the plateau's substance melts or freezes first), so it never takes more heat than the node
  holds and what stays keeps that temperature. A salt is a liquid that never
  boils (23, 24): it moves with the liquid and stays when it evaporates. Lost head heats the liquid that loses it (on
  the surface what runs out of a node, under it where it goes) and does work (1.3); lifts and falls book through the
  potential node (`flow::Falls`).

### 1.3 Erosion and creep
The head lost by moving liquid does work where it is lost (`flow::liquid::load`, `wear`): it **carries** up to `carry`
m³ of loose solids per J (dropping what is over capacity, so load settles where flow slows), and with the work its
load leaves free (the free share of that capacity: the cover effect, Sklar and Dietrich 2004) **wears** `wear` m³ of
bed per J over its strength into loose ground. **Creep** (`flow::creep`, yearly): loose ground slides downhill as a
slow diffusion, `creep` × softness (1 / (hardness + `hard_min`)) × thickness × slope. Bodies hold the ground: carrying
and creep are cut by 1 / (1 + their mass per area / `hold`). Most rock leaving the land is lime dissolved by reaction 22.

### 1.4 The air's step, the sun and the bodies' share
`flow::air::step`, every `air_hours`, for any length of step. First the gas of the bodies' nodes comes up into their
air nodes and gases dissolve (1.5). Then the ground under each air node gives it its forcing for the step, summed from
its cells as they are at the step's start (`air::cells::forcing`): the heat radiation the air absorbs of theirs, their
sensible heat with the air touching them, what evaporates from them by its saturation deficit (the bucket's share of
their wetness, no more than they hold), and the saturation each cell rains at. The air level then runs day by day,
explicitly, on plain arrays of its nodes (`air::daily`): each node's exchange with its ground and its heat radiation,
the wind (1.6), then rain (1.5). Last, each cell exchanges once with the mean air its node had over the step
(`air::cells::pass`): sunlight, heat radiation, sensible heat, what evaporates and what rains on it, in its own
implicit surface balance (Newton's steps about its end temperature, with its melting plateaus, 1.1), and all of it is
booked whole (`air::book`). The air's ground is the cells' at the step's start, the cells' air the step's mean: each
part is stable for a step of any length, as each relaxes toward the other (the air's column takes about 20 days to
follow its radiation and 10 its ground; a cell's skin 3). Where an air node would end with less than nothing of a gas
(its cells evaporated less than its days counted on), the rain of its cells is cut by the share that leaves it
nothing, and their exchange made again; then what the wind takes of that gas from it.
- **Sun** (`radiation`): the daily mean by latitude, season and day length (Berger 1978), declination sin δ = sin ε sin
  λ, the mean over the step's days. The surface reflects as its cover is at the step's start (`land::reflectance`);
  where a soft cover lies or snow falls, as it is at the step's end, so new snow reflects at once and a cover that
  melts away darkens the ground in its step.
- **Heat radiation**: surfaces emit σT⁴; the air absorbs 1 − exp(−x), x the optical depth of the gases (each as the
  square root past its own column amount) and clouds, and emits from 2 grey layers that cool with height, so more gas
  lifts the layer that reaches space into colder air and the ground warms. The layers are each air node's, over its
  mean ground; the air touching a cell is colder by the drop with height from its node's mean height.
- **Clouds** (`exchange::cloud`): scattering depth x = condensation rate / `cloud`, from what each air node rained in
  the step before; they reflect x / (1 + x) of the sunlight (two-stream, Lacis and Hansen 1974) and absorb heat
  radiation at depth `cloud_ir` × x (Stephens 1978). A globe of Earth's size without life holds about Earth's
  climate. **Known limit:** their depth is spread like the air's mass, not set where they condense. The ground also
  trades sensible heat with the air (`sensible`).
- **The bodies' node** (`Class::Bodies`, one per cell): light the cover absorbs warms the ground, except the share its
  light reaction stores (`Cover::keep`), booked from the sun into this node; the cover takes it at its month, and what
  it does not claim goes back to the ground before each air step (`survey::release`).
- **Gas from bodies and the cover** goes into the bodies' node with its latent heat; the air's step takes it up with
  its lift.
- **Shade**: the cover's kinds share its light by height (4.5); bodies catch no light.

### 1.5 Evaporation, rain and dissolved gases
`flow::air::cells`, `daily`, `exchange`. **Evaporation** by vapour pressure (the ground's saturation pressure against
the air's): wet ground at the open rate, drier ground in proportion to its liquid (Manabe 1969's bucket), taking the
heat it held at the node's temperature; a node over a substance's boiling point boils of it what its heat over that
point pays, cooling to it (`Scratch::boil_off`; gas coming out of the liquid too). In the air's day an air node's
vapour relaxes toward the balance with its ground, as its cells were at the step's start. **Rain**: what an air node
holds of a gas over what saturates the air touching each of its cells (colder higher up) rains out in `rain_time`,
each day after the wind, as eddies carry vapour across the air in hours and rain takes a day. The cells' saturation is
taken as the step starts, as a ratio to the node's, so the day sums them in rising order. What a node rained in the
step falls on its cells by what each would hold over its own saturation at the ratio the node rained at. Where the air
touching a cell is below the gas's melting point it falls there as a solid (snow) and the air keeps the fusion
enthalpy its freezing frees; condensing frees the vaporisation enthalpy in the air. **Lift**: gas going up is lifted
from the ground's head to the air's mean height, paid by the air's heat; rain's fall gives it back. **Dissolving**
(`exchange::dissolve`): a gas that cannot condense dissolves in the ground's liquid toward Henry's share, through the
air's film (`evap`) and the liquid's (`dissolve`) in series, freeing its vaporisation enthalpy, and comes out when the
liquid warms, thins or freezes. **Known limit:** a liquid's whole depth is one mixed layer, so a deep sea takes up gas
in months and warms over centuries; a real one is the other way round.

### 1.6 Wind
`flow::air::wind`, `daily`. Eddies (`mix`) even out the air within a day over the distance sqrt(`mix` × day), about
300 km (Taylor 1922): the air nodes lie in blocks of that side on the map, and each day every gas and the heat of a
block are shared among its nodes by their volumes, so its pressure, shares and temperature are the block's (an
island's air is one block). Across the links between blocks gas moves by three pushes, each day from the air at its
start, explicit: the column's pressure (p = nRT/V), evened in the time sound takes to cross the link (Lamb waves,
Lindzen 1990); overturning, the lower half flowing to the warmer column and the upper half back at `wind` per K,
turned by the planet's rotation (`rotation`), vapour crowding low; and eddies mixing the columns (`mix`, width over
length). Each moves at most what evens its link in a day, and each link's pressure, and its eddies with its
overturning, at most half of that over the links of the busier of its nodes, so a node's links together move at most
half of what evens them or of what it holds and the day is stable for any rates. **Known limit:** where a push is
faster than a day (the pressure, always; the eddies over the short links near a pole) its links even in a day at
most that share, so a pressure difference across many links of blocks takes days to even out. Mass that crosses is
lifted between the nodes' mean heights by the heat of the node it enters. A cell sees its air node and three
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
absorbed light and store `light_yield` of it. **Catalyst**: a body's own (2.4, 6.5). **Open world**
(`Reaction::open`): in ground, liquid, air and parts live the decomposers, whose catalyst `open` works as far as the
node is moist: 1 + (`open` − 1) × moisture (`change::open_catalyst`). Moisture is the node's liquid over
what its pores hold at field capacity, at most 1 (`FlowView::moist`, the bucket of evaporation); a part has its cell
ground's; an air node has none. `open` is 1 for most reactions; 200 for reaction 1 (the free sugar's rot, about a
week); 5 for 21 (body matter rots 3.8 times faster than the frame); 10^6 for 26 (the frame rots by the decomposers
only: bare, a millionth of that). No decomposer works in a body (2.4).
`change::pace` is the one formula for all nodes. **Frozen**: a bare reaction whose rate constant k exp(−Ea/T) would
use under 1e-12 of an input over the world's year is skipped in every node (`frozen_below`, `NEGLIGIBLE`), a speed
tolerance.

### 2.2 Where it runs
**Ground and air nodes** (`change::step`, `Reactor::react`) react when due over all the time since, at Flow's last
temperature (a plateau's fusion heat set aside), then sleep until their reactions would use 5% of an input or warm
them 0.5 K, at most `sleep_days`. The nodes under one air node react as one task (a domain), each into a row of what its reactions changed and the gas it gives its air node; the rows are booked after the call, each array of the store by one task (`change/booking.rs`, `store::book_by_array`), so an air node gets the gas of its ground nodes in the order of their cells whatever thread ran them. Gas a ground node takes or gives
(what its reactions made less what the node itself gained, so it never gives more than it holds) crosses to its air node with its heat and latent heat (parts and bodies the same, `change/gas.rs`). **Parts** react
every 6 months (`bond::step`); **bodies** burn and digest by their diet (2.4); **growth** is a conversion only
bodies run, paid by work (6.6).

### 2.3 What gas a ground node touches
Its own gas and its cell's share of the air. **Under standing liquid** only the gas dissolved there, renewed through
the liquid's film in τ = depth / `dissolve` (`FlowView::standing`): a gas it takes is held at Henry's share × (1 +
made·τ/eq) / (1 + used·τ/eq). So dead matter on the sea floor rots as fast as the film brings the oxidising gas.

### 2.4 Change inside bodies
Bodies run no reactor: they run three conversions of the chemistry, each booked by its stoichiometry. **Burning** on
demand to pay their costs (the reactions of their diet that release heat, `Chemistry::burners`: in chemistry 0 the
fuel's, reaction 1, and the body matter's, 21), as far as their fuel and their air node's gases allow (`body::burn`).
**Growth** by `Chemistry::build` and `builds` (6.6). **Digestion** (`Body::digest`): the reverse of growth for matter
taken in. A body's diet is its reaction genes: on above 0, it burns or digests what that reaction takes (`Body::uses`).
The light reaction runs only in the cover (4.5), the decomposers' reactions only in the open world (2.1).

### 2.5 Phases and the freezing law
Solid, liquid or gas by melting and boiling points (`Chemistry::phase`). A liquid's melting point in a mixture falls by
the ideal-solution law (`Chemistry::melt_in`): 1/T = 1/Tm − R · ln(x) / Lf, x the solvent's share among itself and
its solutes. Only the solvent (the most abundant liquid) is lowered; solutes are liquids melting below it (the salts 23
and 24, the fat 6); ice and liquid count together, so it freezes at one point. One law for bodies, ground plateaus,
ice cover (`World::thaw`), parts and Change; bodies freeze their key liquids at the liquids' own melting points (6.3).
**Latent heat**: a gas in the air holds its melting and boiling heat (`Substance::latent_gas`), paid or got as it
enters or leaves; a body holds the fusion heat of its molten key liquids (`Body::thermal`), and liquid it takes or
gives carries it (`Body::heat_with`).

### 2.6 Pairs at balance and the sea's carbon
`change::reversible` finds each pair of reactions where one undoes the other; `Reactor::balance` finds from the
chemistry alone where a pair goes at equal rates. In chemistry 0: 22 makes the dissolved salt 23 from lime, the carbon
gas and the liquid; 23 turns it back. Wet cells start at that balance (`worldgen::terrain`), so the sea holds several
times the air's carbon and gives it back when the air thins, with no rule for the sea.

### 2.7 Chemistry 0 in short
One fixed Earth-like chemistry until M7 (handover 6.0): 9 elements, 26 substances, 28 reactions, numbers only in
`core` and `worldgen`; Earth names in `library/chem0.toml`. Molar masses are sums over elements, so reactions keep mass.

| Number | Role (≈ Earth name, for reading only) |
|---|---|
| 0 | the common liquid, key liquid of bodies (≈ water) |
| 1–4 | air gases: inert, oxidising, carbon gas, methane-like (≈ N2, O2, CO2, CH4) |
| 5 | energy-rich solid, the light reaction's product and the main fuel (≈ sugar) |
| 6 | fat-like store |
| 10 | lime (≈ CaCO3), rock and a solid bodies build |
| 22 | body matter (≈ tissue): C120H196N6O106Ca |
| 23 | the rock salt, lives dissolved (≈ Ca(HCO3)2) |
| 24 | the nitrogen mineral, lives dissolved |
| 25 | the frame, a solid bodies build (≈ cellulose) |

Reactions: 0 light to sugar; 1 burning sugar (its rot in the open world); 21 rotting or burning body matter (gives
back the nitrogen mineral and lime); 22 and 23 the lime–salt pair; 24 fixing nitrogen; 25 the nitrogen mineral
breaking down when hot; 26 rotting of the frame; 27 splitting the frame into sugar (needs an enzyme). Dead matter rots
in moist open ground at a litter constant (26: about a quarter a year at 290 K; 21: about one a year). **Growth** (`Chemistry::build`, bodies only, paid by
work): matter from 118 sugar + 3 nitrogen mineral + 1 rock salt; frame from 6 sugar; lime from 1 rock salt.
`Chemistry::minerals` are growth's inputs no reaction burns (24, 23), `materials` the solids bodies build (25, 10).

## 3. Bond: things can be joined, joins break when pushed too hard
Files: `core/src/bond/` (`verbs`, `sense`, `month`, `bonds`); also `choose::go`, `Body::height`, `Body::breaking`.

### 3.1 Parts and structures
A **part** is a made object (solid walls, the liquid in its hollow, heat, 3 sizes, a hollowness), a store node
(`Class::Part`) by slot and generation (`bond::id`), lying in a cell or held. A body makes one with "form"
(`bond::form`, `c_form` per kg, at most `form` of its mass). A **bond** (`bond::Bonds`) joins two parts or a part and
the bed; its strength comes from the weaker material and the contact area (`tensile`, `joint`), at `c_join` per m². A
**structure** is the parts joined by bonds and moves as one. Parts take their cell's temperature monthly. A part's turn
is a step on its domain like a body's: it reads and writes the domain's view of the store, so a part sees what the parts
before it in its domain left in the air and the ground (`bond/month.rs`). A part whose walls fall under `min_kg` falls
apart (`bond::fall_apart`): all it holds, its walls, the liquid in its hollow and its heat, goes to its cell's ground,
whether its walls ran off in its turn (it is freed after the turns) or a body took them (`bond::verbs::take`).

### 3.2 Loads and failure
Monthly (`bond/month.rs`) each bond carries the weight hanging on it, less buoyancy, plus the push of moving liquid and
of bodies passing (`bump` of their weight); past its strength it breaks, and what has no support lies on the ground. A
structure the liquid holds up drifts down the liquid's slope (`bond::afloat`) with the chance that the liquid crosses
the cell in the month.

### 3.3 Holding, digging, floating
Holding is a bond whose strength is the body's **force**, strength gene × living body × gravity × `grip`
(`bond::force`), with a lever for a held part (`bond::lever`); a body holds only while its force bears the structure's
weight. Fuel adds no force; a hurt body is weaker. **Digging** (`bond::dig`) breaks the bed by force and time. A body
is **afloat** where the liquid is deeper than `deep` and what it and its structure displace weighs as much as they do
(Archimedes): it reaches no ground, and crosses at `afloat` of walking's cost.

### 3.4 Containment
Walls that close a space make an **inside** (`bond::room`) joined to its cell by one link (`bond::Enclosure`): heat
passes the walls (conductivity × face / thickness) and the openings (`leak` per m²), light and rain only the openings.
A body inside exchanges through that link in series with its own surface. "Shelter" is only this low conductance.

### 3.5 Moving needs force
A mover pushes what it passes with `bump` of its weight, so it moves only if its force is at least `bump` × its whole
mass and load × gravity (`choose::go`). The starting plants are too weak to move. A move refused for room or force
changes nothing: a body lets go of what it cannot carry only once it can step. A move that passes both but cannot pay
its cost does not happen either, and the body has let go of what it cannot carry before it was asked to pay.

### 3.6 What a body is built of holds it up and resists breaking
**Height** (`Body::height`), by Greenhill's self-buckling column: h⁴ = 7.837 / 4π × (Σ E·V) × V / (m·g), V the volume
of soft tissue and each built solid, E their Young's modulus (`modulus`); it orders light and rain (1.4). **Breaking**
(`Body::breaking`) a share f takes f × Σ V × each material's breaking stress (`rupture`): a bite pays it, pushing
through breaks what its energy pays for (6.8).

## 4. Copy: a piece of the graph is copied, with small changes
Files: `core/src/copy/` (`mix`, `grow`), `nodes.rs`, `body/genes.rs`, `lineage.rs`; the cover in `core/src/cover/` (4.5).

### 4.1 Genes
16 scalar genes (`body::Layout`; `HEAT`, `STRENGTH`, `CHILD`, `COPY_MASS`, `MUT`, `LEARN`, the brain sizes
`HIDDEN`, `MEMORY`, `SEEN`, `SIGHT`, and `DISPERSE`, `REPAIR`, `WET`, `ALLOC`, `KEEP_T`, `COOL_T`; meanings in
`body/genes.rs`), a diet gene per reaction (on above 0), per substance a safe minimum, an uptake and
a keep gene, and the born brain rows. **Mutation** (`body::vary`): parent's gene plus noise of one step × 10^`MUT`,
within range (not the brain-size genes of a body with no brain, 4.4); born rows change with `row_chance` by
`row_noise`; rarely (`new_lineage`) a new lineage with a changed brain base. **Likeness** (`body::alike`): every
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
`DISPERSE` of the children starts in a neighbour cell in a random direction, one for all of a node's children of an
update (for an individual, that chance), where they land when the batch ends (0). Less than one member with no node to
join waits in the brood, or, landed in a neighbour cell, goes back into it.

### 4.3 Two parents
A mature individual takes the most alike mature **partner** in its cell (`copy::mix::partner`); each gene and born row
of the children is one parent's, drawn evenly, then mutated (`mix`, `mix_rows`). The partner gives nothing. Without a
partner, one parent. Bodies not alike never mix, so **kinds are the groups that mix**. A node's children have one
parent: its members mix inside it (one variance per gene and no covariances is free recombination, Bulmer 1971).
`--no-mix` is the control.

### 4.4 Population nodes
A node is one body for many members (handover 7.1): a speed tool that must give the patterns of individuals.
- **Exact members** (`nodes::Lot` for every split, `nodes::rejoin` for every join, inside `nodes::add` for merges and
  births): members, stores, heat and brood split by shares and join as sums; genes, sizes and rows become means by
  members; the spread per gene (`Body::spread`, `Body::row_spread`) and of the sizes (`Body::size_spread`) pools each
  one's spread and its gap to the new mean. Members of a node with a brain that take any action but a move or a
  bond (`choose::spread`) act as the node itself while the others wait aside with their share; these come back by
  `rejoin` alone: their genes and spreads are the node's, so only amounts and sizes (members who fed are now larger
  than those who waited) join; what members who watched copy is applied when the batch ends (5.5).
- **One lineage** per node (one brain). Children join the young node of their kind in their cell when alike within
  `alike` steps plus `WITHIN` standard deviations (`body::within`), else start one.
- **Young nodes**: children join a young node of their kind in their cell (`copy::kin`: not mature, of their lineage
  and alike within `alike` steps plus `WITHIN` standard deviations), or start one; a young node matures as a whole when
  its mean member reaches the copy mass, and alike nodes of one stage merge (below).
- **Merging** (`nodes::crowd`, monthly): alike bodies of one lineage and stage in a cell join a node of more than one
  member if they are in the room of the same structure (its root) or none and hold the same structure or nothing
  (`nodes::Kin`). Past `crowd` nodes a cell merges its most alike pairs of one lineage and stage while alike, whatever
  they hold and whatever room they are in (the one that joins lets go of its part and takes the other's room); unlike
  nodes never merge. This merge is a speed tool with a physical side effect: the joiners' members drop the structure
  they held, which stays in the cell for any body to take, and leave or enter a room (decision 56).
- **Splitting** (`nodes::split`): a node whose halves would no longer be alike splits along its widest gene if its cell
  has room; selection then acts between the halves. Means are kept exactly.
- **Choices as a spread of actions** (`choose/spread.rs`) for a node with a brain (5.3).
- **Pace**: every node updates and decides once a month, as an individual does (0).
- **Brain size** (`lineage::resize`): a mean brain-size gene a whole unit off its lineage's starts a new lineage. A
  body with no brain does not mutate its brain-size genes (`body::vary`): they do nothing without one, so a brain
  arises only in a lineage founded with one (decision 72). An
  individual's child joins its parent's lineage when its size genes fit it, else its partner's when they fit that
  (its rows carried over to that brain); only a child that fits neither starts a new lineage (`copy::child_lineage`).
  `nodes::open` draws members as individuals. `--individuals` keeps bodies with a brain out of nodes (handover 7.1).

### 4.5 The cover: light-eaters as a field
`core/src/cover/`. The light-eaters are not bodies: each cell holds up to `SLOTS` (4) kinds of **cover**, each the
amounts of four substances of chemistry 0 per cell (the fuel 5 its light reaction makes, body matter 22, its walls 25
and the hard solid 10: `cover::Parts`, found from the chemistry's roles) and 9 genes (`cover::LIGHT`, `LEAF`, `WALLS`,
`STORE`, `FROST`, `MOIST`, `SEED`, `REPAIR`, `FIX`; ranges in `cover::RANGE`). The cover of a cell is a node of the
store (`Class::Cover`) whose law books its own side; it is kept in the domain of its cell (`Domain::cover`), so the
cell's domain is its one writer. Its temperature is its ground's: at the end of each month it trades heat with its
ground to that temperature. **One month** (`cover::step`, its own process after the air's step at a month's start, one
task per domain), slot by slot, the tallest (most walls per m²) first:
- **Light**: the light the cell caught since its last month per m² of capture area (`World::light`), on its capture
  area (10^`LEAF` m² per kg of built cover) less what it reflects (its parts' colours by volume), shared among its
  kinds by height (Beer's law, `math::shade`); under standing liquid times e^(−depth / `light_depth`).
- **Light reaction** (reaction 0): the share `light_yield` of the light caught is stored, at most its capacity
  10^`LIGHT` × 10^`MOIST` mol per kg of built cover per s, times the reaction's pace k e^(−Ea/T) over its value at
  `t_cat`, times its activity a = min(1, wet / 10^`MOIST`)^`dormancy` (wet: how wet its ground is as bodies feel it,
  d / (d + `wet_depth`), d the depth of its key liquid in its pores and standing, `Physics::wetness`); both the light
  and the capacity limit go with the air's share x of the reaction's gas as (x − `gas_floor`) / (x + 2 `gas_floor`)
  over its value at `gas_ref` (`Physics::gas_factor`; the light-limited rate of Farquhar, von Caemmerer and Berry
  1980): nothing at or below the floor, so the cover cannot strip the air of that gas. All kinds of a cell are cut
  together to the key liquid of its ground and the carbon gas of its share of its air node. A cover that works on drier ground works more slowly: its capacity goes with its
  moisture need. The light its bodies' node set aside for it (`Cover::keep`, 1.4) pays first; what it stores beyond
  that comes from its ground, which that light warmed.
- **Upkeep**, paid by burning its fuel (reaction 1) with the air's oxidising gas: per kg of built cover and day
  `c_basal` plus `c_light` × what its capacity could store in a day (dearer by 1 + (melting point of the key liquid −
  `FROST`) / `c_firm` for a cover that stands the cold), plus `c_leaf` per m² of capture area; times its repair factor
  (`Physics::repair`), its activity and the burning's pace at T (seasons come from temperature only). The share unpaid
  dies (`SPENT`). **Wear**: the share 1 − e^−wear dies (`AGE`), wear its upkeep over (built cover × `e_life` ×
  10^`REPAIR`), as bodies wear (6.7). **Frost**: below its `FROST` gene the share min(1, (`FROST` − T) / `unfold`) × a
  dies (`PHASE`).
- **Growth**: the fuel over its reserve (10^`STORE` kg per kg of built cover) and the minerals of its ground (shared
  out as kinds build, tallest first) build body matter, walls (10^`WALLS` kg per kg of matter) and the hard solid
  (`cover_hard` kg per kg of matter) by the conversions of growth (`Chemistry::build`, `builds`), paying `c_copy` per kg
  by burning fuel. A fixer (`FIX` above `FIX_OFF`) makes the mineral its growth lacks by the fixing reaction (24), at
  most 10^`FIX` mol per kg per s at the reaction's pace, paid by the reaction's fuel. The fuel past its reserve that
  it could not build of goes to its ground, as roots exude the sugars a plant cannot use (Jones, Hodge and Kuzyakov
  2004), where the open world's rot takes it.
- **Seeds**: the share `SEED` of what it builds leaves for a neighbour cell drawn (or stays where there is none), with
  one generation's mutation of its genes (noise × each gene's step × `cover_mut`); all of it where one more kg would
  not pay: what a kg more would store with the light at the bottom of its layer (`math::shade`) is less than a kg's
  upkeep, so the slot is **full** (`Cover::full`) and stops thickening where its lowest leaves would not pay for
  themselves (Monsi and Saeki 1953; Hirose 2005). When every domain has had its month the seeds land (`cover::land`),
  domain by domain in the order made: they join the most alike kind of the cell (every gene within `alike` steps,
  `cover::distance`; the genes the mean by built cover; they die if that slot is full), else an empty slot, else the
  lightest slot if they outweigh its built cover (whose cover dies as remains); else they die as remains. Alike kinds
  of a cell share one slot (`merge_alike`). Selection is between kinds and cells: what grows better sends more seeds.
- **What it takes and gives**: liquids and minerals from and to its ground with their heat; gases from the air (with
  their heat at the air's temperature and their latent heat) and to its cell's bodies' node (with theirs), as bodies
  breathe; its dead (remains) to its ground as those substances, where the open world's rot runs (reactions 1, 21,
  26). Every move is booked through the domain's moves (`store::Moves`).
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
or part seen), a signal, the shape of a part, the next memory and a critic. Sizes are genes with costs: hidden units
(0 to 64; 0 is no brain, no decisions, no cost), memory (0 to 12), bodies seen (1 to 6), sight
(1 to 4 cells). One brain per body or node (`Body::rows`); a child starts with its born rows plus what its parent
learned (`Life::inherit`).

### 5.2 Perception
Local (`choose/perceive.rs`): own stores against minimums, warmth, size, the cover of its cell (its capture area per m²); its cell's loose substances, liquid,
temperature and crowding; the season; signals; memory; cells within sight ring by ring (weight 1 / ring²), with their
parts, as the last survey saw them; up to `SEEN` bodies in or next to its cell as the batch's start saw them (their
faces, 0: looks, signals, well-being, past dealings); up to 3 parts. Walls and cover hide.

### 5.3 Actions
One action is drawn by the scores at each decision (`choose::act`), paid by its own reactions (`body::pay`) or failed;
a node spreads its members over the actions by those chances (`choose::spread`). Every body decides at every update,
once a month. The members of a node that move leave as one group: count × the sum of the chances to go in every
direction, at most all (less than one member: one member with that chance; fewer than one member left behind: the whole node,
with the chance that keeps the mean at count × that share), all in the one direction drawn by its chance over the sum,
so each direction gets its share of members on average, in groups. A
group splits off only if the cell it enters has room and the node bears the `bump` share of its weight
(`choose::go::can_go`). **rest**; **move** (`choose::go`:
`walk` per kg and metre, climbing, the bodies in the way; needs force, 3.5); **take**, **put**, **give**
(`choose/feed.rs`); **watch** (5.5); Bond's form, hold, take from or put to a part, join, break, let go, anchor, dig,
enter (`bond/verbs.rs`). A move lands when the batch ends, and a watch and an act on a part are applied then, one by one (0).
No copy action, no "farm", "trade" or "attack": those must be patterns of these.

**Taking** (`feed::intake`, one law for the ground and for bodies) goes by uptake genes (`Body::wanted`) at `c_handle`
per kg, of food only (`Body::uses`: what it burns, the body matter and built solids it digests, the minerals it builds
of; not what it cannot use, nor the key liquids it drinks, 6.4): per kg of supply, Holling's type II take × s /
(take + s), s what the swept area holds (`sweep`) of its food, so thin food takes time to find (Holling 1959). The
minerals it builds of count in s as food does: one search finds all it uses, so rich ground shortens it. Another body's
members lie over the cell like the ground's food: a bite reaches those the swept area holds (one individual body
whole, by one draw with the chance the area holds it, whether a node or an individual takes it; of a node, less than one member is one whole member met with that chance:
it takes, tears and pays that meeting's outcome times the chance), limited by force against theirs. **Giving** (`Body::spare`) is of what the other uses, of stores only (no
matter, brood, built solids or key liquid below level), above `keep_min` × their minimum, at most what the members met
could take.

### 5.4 Learning
**Well-being** (`Body::wellbeing_at`): the energy its stores could release, in decisions' worth, less how near it is
to its limits; no reward is hand-shaped. **Within a life** (`choose::learn`, `brain::learn`): a policy gradient
against the critic (actor-critic) on the change of its own well-being since its last decision, the critic's estimate
from now discounted by `discount` (one month), at its `LEARN` rate, in units
of the root of the critic's running mean square of errors; that mean and each row's mean square of steps both weigh the newest value
by `learn_beta`. A node learns as from one
member drawn among those who stayed, with the node's change plus how much more its part's action raised it than the
mean, so its brain moves as an individual's does. **Across lives**: evolution of born rows and base.

### 5.5 Imitation, signals, memory
**Watching** (`choose::learn::watched`) costs the turn; when the batch ends, if both live and the watcher is still of
the lineage it watched in, the watcher's rows move part way (`blend`) toward the watched one's, if the brains have the
same sizes, less the less alike they are, times the share of the watcher's members now who watched (for a node, the
mean by members). **Signals**: 2 numbers a decision with
noise (`signal_noise`), meaningless at first, at an upkeep (`c_signal`). **Memory**: up to 4 others and how dealings
went (`Body::note`, `Body::recall`). **Thinking** (`Physics::think`) costs by brain size and mass^0.25 one decision per
update, once a month. **Null run** (`--null`): no learning or imitation, rows frozen, genes still evolve.

## 6. Bodies
Files: `core/src/body/`, `copy::grow`. Bodies are the eaters (animals); the light-eaters are the cover (4.5).
`body::live` runs an update in stages: upkeep, drinking, heat, store checks, growth and children, shedding; then the
decision. Its trades are gathered as moves and booked with its batch's, each number of the store once per domain
(`store::Moves`, 0), so nodes and individuals take alike, never more than is there.

### 6.1 What a body is
**Stores**: every substance (all members together) and heat; in practice its fuel (5, and 6 for a diet that burns
it), body matter (22), its key liquid (0) and the two minerals. **Body matter** is what it is built of; its **living
body** (`Body::built`) is matter less brood, over 1 − `WET_TISSUE`. Taking, building, upkeep and force go by the living
body; fuel and extra liquid are carried, and storing fuel costs no work. `Body::size` (with its liquid) sets height,
volume and minimums; `Body::mass` (everything) moving and load. **Levels** (`Body::level`): the key liquid h / (1 − h)
of matter (h its keep gene), a built solid its share of matter, others keep gene × kg of living body; a full store is
satiety. **Supply** (`Body::supply`): living body^`intake_exp` from `m_ref` × its pace (6.2). Bodies catch no light:
their capture area is 0.

### 6.2 Upkeep and paying
`body::upkeep` (`body/pay.rs`):

```
tissue = living body × (copy mass / m_ref)^(intake_exp − 1)
upkeep = [ days × tissue × (c_basal + c_strength·STRENGTH + learning + signal) + one decision ]
         × repair (1 − c_repair + c_repair·2^REPAIR) × activity × pace(T)
```

Costs go by the kg while supply goes with mass^0.75 (West, Brown and Enquist 1997), so a body grows until keeping what
it has takes all it can supply: its costs set its size, not a cap; across kinds adults pay Kleiber's mass^0.75.
**Pace** (`Body::pace`): the bare rate law of its main burning, k e^(−Ea/T), over its value at `t_cat`; supply and
upkeep follow it, so seasons come from temperature, not a calendar. **Paying** (`body::pay`, `body::burn`): its
burnings run on demand as far as its fuel and the air's gases allow; the gas they take comes from its air node and the
gas they make goes to its cell's bodies' node, each with its heat and latent heat (gas exchange is free: no pores);
the work becomes heat in its cell's ground, and the body keeps its temperature. The unpaid share of upkeep dies
(`SPENT`).

### 6.3 Heat
A body's temperature follows its ground's (`live::keep_warm`): the heat its upkeep released over the update leaves
through its coat (`HEAT`, × mass^(2/3), `Body::conductance`; in series with the walls around it, 3.4), so it stands
that much above the ground. Below its `KEEP_T` gene it burns what its coat loses at `KEEP_T` beyond that heat; above
its `COOL_T` gene it evaporates its key liquid for the excess, at the liquid's vaporisation enthalpy, never below its
safe minimum, the vapour going to the bodies' node with its latent heat. Its heat then comes to that temperature with
its ground (`body::settle_heat`). Its key liquids hold their fusion heat when molten (`Body::thermal`, at their own
melting points).

### 6.4 Water: drinking
A body drinks its key liquids from its cell's ground up to the level it keeps, as far as the ground holds them liquid,
each day at most its uptake gene's share of that level, paying `c_handle` per kg; they come with their heat. It
catches no rain and has no roots: its minerals come from what it digests (6.9).

### 6.5 Activity and dormancy
`Body::activity`: a = min(1, l / l*)^`dormancy`, l the key liquid, l* its level. It multiplies upkeep and the building
allowance: a dry body pays and ages almost nothing and can hardly act. **Frost**: a key liquid below its melting point
kills the share frozen (`PHASE`); **boiling** kills.

### 6.6 Growth
`copy::grow`. Its fuel and minerals become matter at `c_copy` of work per kg, paid by burning (`body::pay`), as far as
all allow: the **allowance** (`c_build` × supply / `m_ref` × days × activity), the **spare** (what its fuel can release
beyond a month's upkeep), and its stores down to their minimums. Built solids (frame, lime) come first by their
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
`body/death.rs`. **A bite** (`take_body`) tears from each member reached the share taken and pays to break it (3.6);
matter breaks first and spills its stores; in a node those members lose it (grazing) or die (`body::wound`). **Pushing
through** (`choose::wear`) breaks what its energy pays for: the energy movers spent in a cell is kept until the next
survey (`Domain::pushed`, of the cell's domain); then each body there, a mover that stayed too, takes its share by mass and loses what that
breaks of it, each of its members alike. So a cell's bodies are worn once a survey, not once for each move into it.
A member dies when `lethal` of its body is torn at once; a hurt adult is
immature until healed. **Starving** (`starved`): a body below its copy mass that did not rebuild what it lost keeps
the share x and dies with chance 1 − x^a, a = ln 2 / −ln(1 − `lethal`), so starving to the lethal loss is survived
with even chances in one update or many (`SPENT`).
Causes: `LACK`, `PHASE`, `SPENT`, `TAKEN`, `BROKEN`, `AGE`; at death all goes to the ground (`body::die`).

### 6.9 Materials and minerals in bodies
Built solids count in mass and height, not living body; children get them, gifts never give them. Bodies get
minerals by digestion (the cover from its ground, 4.5); what is past its level it sheds. Fixing (24) runs in the cover
only (4.5).

## 7. Soil and minerals
The minerals (`Chemistry::minerals`) live in the ground's liquid and move with it, so rain washes them to low ground
and the sea and fertility differs by place. **Nitrogen mineral (24)**: placed when life starts (`worldgen::life::soil`)
on land under less than half a metre of liquid; then only fixing adds it, rotting (21) returns it, heat (25) sends it
to the air. **Rock salt (23)**: made in wet cells from lime (22), turned back by 23, near balance (2.6).

## 8. Starting conditions (not laws)
Placed by `worldgen` (`worldgen::schedule::Schedule`, carried by a snapshot) and printed by every run; the laws never
read a kind.
- **Terrain** (`worldgen::terrain`): an island, or a globe with 29% of its area above 0 m (`LAND_SHARE`); a metre of
  loose ground; the sea's salt at balance (2.6); ground and air temperatures falling with latitude and height. Every
  land cell drains from the start (`drain`): a priority flood raises closed basins to their outlet at 1 m per km
  (`DRAIN_SLOPE`; Planchon and Darboux 2001), from every cell below 0 m on a globe, from those joined to the lowest
  cell on an island. Quarters read the same noise at their own centres.
- **Life** at year 2 (`--life-year`): the cover (`worldgen::life::KINDS` rows of `Place::Field`: kg per m² and genes,
  on every open land cell by its chance), then eaters 20 years later; `--roster` places ten kinds (`ROSTER`). Founders
  are adults on land under at most half a metre of liquid; mass 30% fuel (60% for the two big slow eater kinds).
- **Brains**: eaters start with hand-set biases (`Seeding::hand`; `--neutral-eaters` without), since neutral founders
  die before they learn: they take from the place, its ground and its cover (1), before they bite a body (0.5), and keep
  out of deep liquid (a move's odds fall as 1 / (1 + depth / m)). Genes and rows get a spray (`--spray`).
- **Fixers**: a tenth of the cells of the starting cover carry the fixing gene (`FIXERS`, `FIX_GENE`; `--fixers`).
- **Founder genes**: some measured from the laws (`LITTER_X`); bodies build solids at a small share (`BUILT_SHARE`);
  `HEAT`, `CHILD`, `ALLOC` the same for all; `KEEP_T` and `COOL_T` 5 K above their liquid's melting point and 5 K below where life's enzymes fail (320 K) (`EATER_KEEP_T`, `EATER_COOL_T`).

