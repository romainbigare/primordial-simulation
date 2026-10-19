# The laws as they are now

This file says how each law of the world works. It is written in the present tense. It does
not say when or why a rule came: that is in `docs/history.md`.

- The plan and the rules of work: `docs/handover.md` (the section numbers in this file, like
  "handover 6.2", point there).
- Every number, its unit and where it comes from: `docs/numbers.md`. This file gives a number
  only when it helps to understand the rule.
- What a run records: `docs/tape.md`.

The code is the truth. Each part below names the file and the function that does it, for
example `body::upkeep` (the function `upkeep` in `core/src/body/pay.rs`).

**Where the code is** (all under `core/src/`)

| Law or part | Files |
|---|---|
| time and the month's order | `clock/` (the calendar, processes with periods; `sleep`: the sleep and wake of nodes), `schedule.rs` (`step`, `step_marked`, `day`) |
| the shape of the world | `space/` (`Mesh`: levels of cells and links with their widths, parents and children; `square`: the square map; `stack`: layers above and below a cell). `World::mesh` is built from the saved map (`World::map`, `World::air_scale`; `World::mesh_of`), and the laws read cells, links, areas and air nodes from it. The square map is read only where a law is about its layout: the eight moves and sides a brain sees (`choose`), the side a child lands on (`copy`), the lowest neighbour a drifting part goes to (`bond::month`, first in compass order on a tie), and the four air nodes a cell reads the air between (`flow::scratch::tables`) |
| the shared maths | `solve/` (`spread`: the implicit step along links; `downhill`: the high-to-low order and one pass down it; `band`: the implicit step of amounts carried along the links of a whole level, solved directly) |
| settings, checks | `params.rs` (`Config`, `Physics`), `world.rs` (`World`, `Counters`), `check.rs` (the books); the nodes and their books in `store/` |
| ground heat | `ground.rs` (heat capacity, melting plateaus, `World::thaw`, `World::t_ground_last`); `flow::land::conduct` (sideways) |
| 1 Flow | `flow/` (`mod`, `scratch`, `land`, `liquid`, `drain`, `creep`), `flow/air/` (`mod`, `step`, `newton`, `book`, `radiation`, `exchange`, `wind`, `sky`); `lanes.rs` (paths) |
| 2 Change | `change/` (`mod`, `kernel`, `balance`, `gas`); the chemistry in `chem/` (`mod`, `thermo`, `roles`) |
| 3 Bond | `bond/` (`mod`, `verbs`, `sense`, `month`) |
| 4 Copy | `copy/` (`mod`, `mix`, `grow`); population nodes in `nodes.rs`; lineages in `lineage.rs` |
| 5 Choose | `choose/` (`mod`, `perceive`, `learn`, `feed`, `go`); the network in `brain.rs` |
| bodies | `body/` (`mod`, `genes`, `tissue`, `metab`, `exchange`, `live`, `physics`, `pay`, `death`, `intake`); the registry in `life.rs`; the cells seen by bodies in `survey.rs` |

**Words used here**

| Word | Meaning |
|---|---|
| cell | one square of the map (2 km × 2 km). It has a ground node and sees an air node: its parent at the mesh's air level (`AIR`), whose share over it is its area over the node's (`Mesh::share`). |
| node | anything that holds stuff: the ground of a cell, an air column, a body, a part |
| body | a living node with genes and (maybe) a brain |
| member | one living thing inside a body. A body with 1 member is an **individual**. A body with many members is a **population node** (all members alike, counted together). |
| store | the amount of one substance a body holds (in mol) |
| mol | a fixed number of molecules; the unit of amounts in the code |
| gene | a number a body inherits. Many genes are log10: 1 means ×10, 2 means ×100. A gene below 0 is often "off". |
| catalyst | a gene that makes one reaction faster inside the body (metabolism) |
| key liquid | the liquid a body's chemistry runs in (substance 0, ≈ water, in chemistry 0) |
| light-eater | a body whose light-reaction gene is high (plant-like). Others are "animals" in the tape. |
| interval | the days between two updates of a body (a gene) |
| plateau | a node stays at a melting point while one substance melts or freezes in it |

---

## 0. One step of the world

Time is a whole number of seconds since the start (`World::time`); the day, the month, the year and
the season come from it by the calendar (`clock::Calendar`, from `Config::calendar`). The world moves
one month per call of `step` (`core/src/schedule.rs`). Each process has a period inside the month
(`clock::Every`), and at one time they run in this order, written once in `schedule::processes`:

1. Every `phys_days` days (10): `survey::survey` (what each cell holds, light and shade, its bodies' node); then the
   parts of Flow due: the state of ground and air, which the other laws read until the next physics step
   (every physics step; rebuilt in full at each liquid step), the air's step (every `air_hours`, 240:
   radiation, evaporation, rain, gases dissolving and the wind, together, 1.4 to 1.6), liquid, drainage and the work of moving liquid
   (every `flow_days`, 30), heat conducted sideways through the ground and creep (every
   `flow::CREEP_YEARS` years, 1), the recheck of what is present and of the slow ground (every
   `flow::REFRESH_MONTHS` months, 3); then Change (`change::step`).
2. On the first day of the month: Bond (`bond::step`): parts, loads, breaking.
3. Every day: `survey::day` updates the bodies whose interval ends that day (each body every 5 to
   360 days, set by its `INTERVAL` gene; a population node without a brain at least every
   `node_days`, 36, 4.4).

Each run stands for the time to the process's next run (the last one in a month to the month's end), so
a part of Flow moves for its own period, counted from the start of the month (the yearly and the
three-monthly ones from the start of the world). Flow keeps no timer of its own.

**Sleep and wake** (`clock::Sleep`). A node's own process may sleep while the node is calm: until an
alarm time, or until one of its amounts has grown by more than `wake_share` (5%) of what it held when it
fell asleep, whatever brought it (Flow, bodies, parts) and in as many gains as it took. Its law looks
for such gains once at its own step (`Sleep::watch`), not at every move (heat wakes no node; what only
passes through a node wakes nothing). When it is due, its law brings it up to date for all the time since
it last did, in one span. Change's reactions in ground and air nodes sleep so (2.2).

**One ground temperature.** Every law and the tape read the ground temperature of Flow's last
step (`World::t_ground_last`). Where Flow has not run yet, `flow::ensure` builds that state from
the world as it is.

**Checks before a run.** `Config::validate` checks the settings: each period at least 1, the air's step dividing the
physics step, and `Physics::validate` (`dormancy` above 1, `light_yield` above 0 and at most 1).
`Chemistry::validate` checks a chemistry: indices, densities, and that every reaction and
conversion keeps its elements and its mass.

**The books.** Every move takes from one node exactly what it gives to another, with its heat.
Elements are never made or lost. Three boundary nodes stand for what lies beyond the world
(`store::Boundary`): the sun gives the light the world absorbs, space takes the heat it
radiates, and outside gives and takes what edits put in and take out. They hold what crossed,
with the sign of a node (the sun's heat falls by what it gave), and every crossing is booked
on them (a `Move` from or to one, a link flux, or an edit's moves, `World::cross`). The potential
energy of the world's matter is one more node of the store (`store::Class::Potential`, whose heat
it is): a lift moves heat into it and a fall takes it back. So the world and the
three nodes together keep every element and the energy: `check::drift` counts it in one check. A run
prints the drift every year; it must stay near the rounding of the numbers.

---

## 1. Flow: stuff moves from high push to low push

Files: `core/src/flow/` (ground and liquid) and `core/src/flow/air/` (air, radiation, rain, wind).

### 1.1 The ground of a cell

A cell's ground has layers, from the bottom: the **bed** (rock, bonded), the **loose ground**
(grains that can move), soft cover (solids warm enough to be soft), and liquid on top
(standing liquid, a lake or the sea). `flow::land::surface` builds these layers;
`flow::land::ground_state` finds the temperature (`World::thaw` does the same from the
amounts now).

- **Heat of the ground.** Only the top `skin_m` metres of ground trade heat with the surface,
  plus everything that lies on it (liquid, ice, cover). The ground's temperature comes from its
  energy and its heat capacity.
- **Heat conducted sideways** (`flow::land::conduct`, once a year). Heat goes from cell to
  cell through the ground's active layer as one implicit step along the links with a shared border
  (`solve::spread`, booked by `store::apply`). Each node holds its heat capacity per kelvin; a link
  passes the harmonic mean of its two nodes' conductivity × thickness (the bed's skin, `skin_m` of
  its mean conductivity, beside the solid and liquid of the loose ground) times its width over its
  length (Patankar 1980). A node on a melting plateau takes its share as if it warmed; the next
  ground state sets its temperature from its energy. On 2 km cells this is slow: a hot spot in rock
  spreads by one cell in about 70,000 years. So it runs once a year, for the whole year in one
  step; the implicit step holds for any length.
- **Melting plateaus** (`ground::melting`). A ground node holds the melting heat (fusion
  enthalpy) of what is molten in it. So it stays at a melting point until that substance has all
  frozen or all melted. Each substance melts at its own melting point **in that node**, lowered
  by what is dissolved in its liquid (the freezing law, 2.5). The plateaus are passed in the
  order of those points.
- **Deep store.** Under the loose ground lies a store in the bed (the store's `Class::Deep`): the
  weathered zone, `bed_depth` deep, with `bed_porosity` of it as pore space. Bodies and Change do not
  reach it.
- **Land** (`World::is_land`): a cell whose solid surface is above 0 m. This one rule is used
  everywhere (soil, founders, the tape).

### 1.2 Liquid

`flow::liquid`, every `flow_days`: first the stores drain to the channels (`flow::drain::springs`), then what may run
off moves down the cells in one pass (`solve::downhill`), then the liquid under the surface moves
(`flow::drain::under`). Every move is booked by the store (`store::apply`): link fluxes over the links of the cells,
and over the vertical links between each ground node and its store.

- **Pores first.** The loose ground has pore space: `porosity` × the volume of its loose solids.
  A cell's liquid fills the pores first. Only the liquid above them **stands** and has a head
  (`flow::land::stand`).
- **Running off.** The cells are ordered by the head of what may run off, highest first
  (`downhill::Order`, renewed every liquid step), and one pass down that order moves it: each cell sends to every
  lower neighbour (all 8) at least `split` as steep as the steepest one, by slope; a head difference under `still`
  does not move. Its head falls no further than to a neighbour still to come (or to its own bed), and half way to
  one already passed, so heads never cross. Liquid can cross the map in one step while depressions fill and spill.
- **Pore water.** The liquid in the pores flows by **Darcy's law** across the borders of cells (not across corners),
  by one implicit step of `solve::spread`: the potential is the liquid table (`flow::land::table`), the capacity
  `porosity` × area per metre of head (the area where the pores are full), and a link passes `seep` × the
  saturated thickness of its higher end × the border's width / the distance. Pores at or under their field
  capacity (`field`) hold their liquid against gravity and give none.
- **Channels inside a cell.** Real land drains to small channels a few hundred metres apart.
  So the loose ground above its field capacity and the deep store also drain by Darcy's law down
  the slope of the cell, over a hillslope of `hill` metres, into the liquid that runs off. A node
  under deep liquid has no slope and does not drain this way.
- **The deep store** is the lower layer of each cell's stack (`space::Stack`), joined to the ground node by a
  vertical link (`flow::drain`). Down it, liquid above field capacity sinks into the store (at most `seep` per
  second, never more than the store has room for); up it, the store drains to the channels (a spring) and gives
  back all it holds over its room. The stores flow to their neighbours by Darcy's law (`bed_seep` × the depth they
  fill), in the same `spread` as the pores. No pores and no store give more than they hold: where all a node's
  outgoing links would take more, each is cut by the same share (`solve::spread::within`; only that node's own
  links change).
- **What goes with the liquid.** A move takes its share of every liquid and of what is
  dissolved in it (salts, dissolved gases), with its heat. A salt is written in the data as a
  liquid that never boils (substances 23 and 24): it goes with the liquid by its share, and
  stays behind when the liquid evaporates. There is no other rule for dissolved solids.
- **Lost head is heat.** The height a liquid loses becomes heat in the ground (or in the deep
  store), and does work (1.3).

**Potential energy.** Lifting matter takes heat and a fall gives it back. The world holds the
difference as potential energy (the store's potential node), so energy stays
exact. This holds for liquid and solids going downhill, for gas going into the air and coming out
(1.5), and for gas between air nodes. In Flow each lift and fall is a link flux between that node
and the node that pays or gains (`flow::Falls`), booked with the move that makes it.

### 1.3 Erosion and creep

Moving liquid loses energy (its lost head). That energy does work where it is lost
(`flow::liquid`, at every liquid step, in the same pass as the liquid):

- **Carrying.** It carries loose solids: `carry` m³ per J. The load leaves a cell in the share the liquid leaves it,
  so it settles where the flow slows (in pools and lakes), and what is over the capacity drops.
- **Wear.** It breaks bonded bed into loose ground: `wear` m³ per J, divided by the bed's
  strength. The bed is a class of the store that holds amounts and no heat (`Class::Bed`: its
  skin's heat is the ground node's); wear and digging (3.3) break it by a move (`Move::loosen`).
- Bodies on a cell hold the ground: they cut carrying and creep by 1 / (1 + their mass per
  area / `hold`).
- **Creep** (`flow::creep`, once a year): loose ground slides downhill like a slow diffusion,
  `creep` × softness × thickness × slope, link by link of the cells (corners too, as wide as a side), each from
  what the links before it left;
  the store books it all in one call.

`carry`, `wear` and `creep` come from real erosion rates (`docs/numbers.md`). Most rock that
leaves the land is lime that Change dissolves (reaction 22, 2.6) and the liquid takes away.

### 1.4 The air's step

`flow::air::step`, every `air_hours` (240). Radiation, the exchange of the ground with the air
(evaporation, rain) and the wind are one implicit step (backward Euler) over the whole span, so a
long step gives the world of short ones: the long-step test holds the climate of 10-day steps of the
air and of the physics to that of 6-hour air steps and 3-day physics steps within the seeds' noise
(`tape/tests/ranges.rs`).

- The unknowns are each air node's temperature and gases at the end of the step. The sun is that of
  the step's end. The gases are those the air holds more of than the rounding of its sums, and those
  that can evaporate from the ground (a substance's saturation pressure over 1 Pa at the warmest ground):
  a trace of less stays where it is.
- Each round reads them and finds what every ground node exchanges with its air node over the step:
  its surface balance solved implicitly (1.4 below), the rain it gets and what evaporates from it
  (1.5), and how these change with the air it sees (each cell sees the four nearest air nodes by
  their weights). Then it solves the air nodes with the wind in one implicit step (`solve::band`, an
  LU solve of the banded system of the whole air level, with partial pivoting): first the total
  amounts with the evening of pressure, then the gases the ground does not exchange, then the
  exchanged gases and the heat together, then each trace gas alone (Newton's method). A round moves
  the air at most 10 K, and keeps each gas of a node at least half of what it held.
- The rounds stop when the air moves by less than 0.05 K and 1e-5 of a node's gas (4 to 5 rounds),
  or after 16. What the last round found crosses every link and is booked by the store, so the books
  are exact whether or not the rounds converged.
- **Booked whole** (`air::book`). The step's moves are booked at once as it is solved: the other laws
  read the ground, the air, the light on bodies and the rain it leaves until the next air step. A
  ground node gives no more of a gas than it holds and gets, and an air node that would give more of a
  gas than it holds rains and blows that much less.

**Sun, heat and radiation** (`flow::air::radiation`).

- The sun warms each cell by latitude, season and day length (`insolation`, the planet settings
  `sun`, `tilt`, `lat_south`, `lat_north`). The surface reflects part of it by the reflectance
  of what covers it (ice and sand more, dark plants less).
- Every surface emits σT⁴. The air absorbs a share 1 − exp(−x) of what the ground emits; x is
  the optical depth, the sum over the gases of amount × absorption (each gas past its own column
  amount grows only as the square root).
- The air emits from 4 grey layers whose temperatures fall with height by the lapse rate
  (`flow::column`, `flow::glow`). More gas lifts the layer that reaches space into colder air,
  so the ground warms. There is no ceiling.
- The ground trades heat with the air touching it (`sensible`). The ground temperature is solved
  implicitly over the step, with the latent heat of what evaporates at that temperature and the heat
  of the rain, again about the temperature it gives until it moves by less than 1e-3 K (at most 3
  times). The store books the sunlight from the sun node to each ground node, what the ground gives
  its air node (with what leaves to space through it), and what each air node sends to the space
  node.

**Light on bodies.** Each cell has one bodies' node for all its bodies (`store::Class::Bodies`):
it holds, as heat, what its bodies will take at their updates. The light that
bodies' leaves absorb warms the ground as it falls, except the share their light reactions store
(`Body::keep`, the share each stored at its last update): Flow books that share from the sun node
into the bodies' node. **Heat drawn from the ground** goes there too: before each physics step the
node settles with its ground so that it holds what its bodies will claim, the light set aside that
each caught since, and the heat each draws from its ground at the rate of its last update
(`Body::drew`) over the time since (`survey::survey`). The ground gives what is missing (as far as
it holds heat above `T_FLOOR`) and gets back what no body can claim. So a body that updates every
20 days does not pull 20 days of heat out of the ground at once. At its update a body takes its
share from the node, and only the rest is settled with the ground.

**Shade** (`survey.rs`). Light passes the bodies of a cell from the tallest member down. Each
layer of leaf area x per ground area takes 1 − exp(−x) of what reaches it. Equal heights share a
layer. Height is what the body's materials hold up (3.6), kept with the other visible traits
(`Body::vis`, set at each body's update, when a body is placed, as a child or a part split off a
node, and wherever its stores or members change outside its update: `World::see`). Rain on bodies
is shared the same way (6.4). The survey shares out a cell's light again only when its bodies
changed since (one came, went, or changed its members or what others see of it: `Life::stir`).

### 1.5 Evaporation, rain and dissolved gases

`flow::air::exchange` and `flow::air::step`, in the air's step.

- **Evaporation** by vapour pressure: the ground's saturation pressure at the end of the step
  against the vapour pressure of the air near it. Wet ground evaporates at the open rate when liquid
  stands on it or its pores hold their field capacity or more; drier ground in proportion to its
  liquid (the bucket of Manabe 1969), integrated exactly over the step with the rain it gets.
- **Rain.** Where the air a cell sees holds more vapour than it can at its temperature (colder
  on higher ground), the rest rains out in `rain_time` (1 day). Condensing air makes clouds that
  cover part of the sky (`cloud`).
- **Lift.** Gas that goes up into the air, from the ground or from bodies, parts, ground
  reactions and growth, is lifted from the ground's head to the air's mean height: the air's heat
  pays for it. Gas that comes down (rain, gas taken by reactions or bodies) gives that back to the
  air's heat. The difference is the heat of the store's potential node.
- **Booked by the store.** A link from each ground node to its air node carries the gas and the
  heat that leaves the ground; the air receives that heat less the latent heat it holds of the gas
  (`store::held`). Every gas of bodies (their breath, reactions, upkeep, evaporation and growth) and
  of parts crosses as moves of their updates (`store::Moves::gas`): the same lift, from the air's heat
  into the potential node.
- **Gases dissolve** (`exchange::dissolve`, at the start of the air's step, from the air as it is).
  A gas that cannot condense on the ground dissolves in the liquid there toward Henry's share (solubility × partial pressure × the liquid's amount),
  and comes out when the liquid warms, thins or freezes. It crosses the surface through the
  air's film (`evap`) and the liquid's film (`dissolve`) in series. Dissolving frees the
  vaporisation enthalpy into the liquid; coming out takes it back. Dissolved gas moves with the
  liquid and returns to the air only here.
- **Known limit.** The whole depth of a liquid is one mixed layer (for heat and for gas). A deep
  sea warms over centuries and takes up gas in months; a real one is the other way round.

### 1.6 Wind

`flow::air::wind`, in the air's step. The air nodes are the coarser level of the mesh
(`AIR`); the wind runs on that level's links (the cells of two nodes that share a border).
Gas moves over each link by three pushes, each a rate:

1. the pressure of the whole column (p = nRT/V), from high to low, evened in the time sound takes
   to cross the link (Lamb waves travel at the speed of sound, Lindzen 1990);
2. overturning: at equal height, the lower half flows to the warmer column and the upper half
   back, at `wind` per K through half the column across the link's width; both are turned
   sideways by the planet's rotation (`rotation`, by the warmth's gradient across the link, at the
   latitude of the link: the mean of its two nodes'); vapour crowds in the lower half;
3. eddies mix the two columns (`mix`, across the link's width over its length).

They are solved implicitly with the rest of the air's step (1.4), so any speed is stable, on any
mesh. What crosses the links is booked by `store::apply` (gas and heat; only what a node holds at
the end counts, so gas that passes through it needs nothing held). Air in Flow moves as whole
columns, so there is no wind near the ground; bodies use the anchored `wind_ground` (6.3).

### 1.7 Paths

`core/src/lanes.rs`. A body that moves through a cell spends energy pushing through the bodies
there (`choose::go`, `choose::wear`). That breaks what it pushes through (6.8) and clears the
link it used (`lanes::clear`, kept per link of the cell level in `World::links`, `store/links.rs`). A
cleared link is cheaper to cross (`lanes::resist`, `lanes::eased`). Once a year, with creep, the bodies grow back
into cleared links (`lanes::regrow`, rate `regrow`). This one rule is what can make paths.

### 1.8 Stability

No amount goes below zero: a law never asks a node for more than it holds. `spread` keeps amounts
positive by its implicit step, `downhill` by its pass, the reactor by going no further than its scarcest
input, and where a law could still ask too much it cuts that node's own outgoing links
(`solve::spread::within`), never a whole field. `store::apply` only adds: what the rounding of its sums
leaves just below zero it sets to zero and counts (`Counters::rounded`); a debug build checks that it is
rounding, and a real overdraw breaks the books. A body or a part takes heat from a ground node no
further than `T_FLOOR` (`World::take_from_ground`); it gets that much less, and it is counted
(`Counters::clamped_heat`).

---

## 2. Change: substances react inside nodes, faster when hot

Files: `core/src/change/` (the reactor), `core/src/chem/` (the chemistry data and its physics),
`worldgen/src/chem0.toml` (chemistry 0).

### 2.1 The rate law

```
rate   = catalyst · k · exp(−Ea / T) · contact · Π over inputs (share of its phase ^ order)
lim    = the least, over the inputs, of (amount / count)      (the scarce input)
contact = what lim would be if each input were all of its phase
extent = lim · (1 − (1 + p·z)^(−1/p)),   z = rate / lim · time   (1 − exp(−z) when p = 0)
p      = (order of the scarce input − 1) · (1 − its share)   for a gas or liquid; 0 for a solid
```

- **Share** is an input's share of its phase (gas or liquid). A solid is a phase of its own
  (share 1). So a reaction inside one phase goes with that phase, and a reaction between a solid
  and the air goes with the solid, like a surface. There is no jump when another input becomes
  the scarcest. A trace input at order 1 goes first order: it loses the same share of itself per
  second whatever its amount.
- **Order.** The data may give each input its own power (`Reaction::ord`, read by
  `Reaction::order`); else the count is the power. A big molecule is not a collision of 40
  molecules.
- **Extent.** Over a step the reaction goes as far as the integral of its rate law (`gone` in
  `change/kernel.rs`): never past the scarce input. For an input of order above 1 the rate falls
  as the input goes, and the step follows that.
- **Heat.** Each reaction unit gives −ΔH of heat to the node (`Chemistry::dh`). The formation
  enthalpy `hf` of a substance is that of its **solid**. A liquid holds its fusion heat `lf`
  more, in the heat of the node that holds it; a gas also its vaporisation heat `lv`, in the air.
  So the books count each phase once. `Chemistry::dh_at` gives a reaction's heat with each
  substance in its phase at a temperature (the heat as measured; `ew chem` prints it at 290 K).
- **Light.** A **light reaction** takes absorbed light instead of heat: it stores
  `Physics::light_yield` (5%) of the light it takes; the rest is heat.
- `change::pace` holds the shared formula. Ground nodes, air nodes, parts and bodies all use it.

### 2.2 Where it runs

- **Ground and air nodes** (`change::step`, `Reactor::react`): each when it is due
  (`clock::Sleep`), for all the time since it last reacted, with sub-steps when the temperature
  changes fast. Then it sleeps until its reactions, at their rates, would use 5% of their inputs
  or warm it by 0.5 K (`Reactor::rest`), at most `sleep_days` (90): a node where a reaction goes
  that far within a step reacts every step. It wakes when one of its amounts has grown by more
  than `wake_share` of what it held when it fell asleep, whatever brought it (Flow, bodies, parts),
  looked at once a step (1, "Sleep and wake"). Its temperature does not wake it: `sleep_days`
  bounds how long it reacts at an old one. This is a speed tool, not physics (`sleep_days` 0: every node
  reacts every step). A reaction too slow to matter over the world's year is frozen.
- **Booked by the store.** The nodes due in a step react from the world as it was before any of
  them did (each ground node from its share of its air node), and `store::apply` books them
  together. A node's reactions are one `store::Transform`: what they made less what they used of
  each substance, from how far each reaction went (`Reactor::made`; summed exactly where its
  terms cancel, so the transform keeps every element). The heat of reaction follows from the
  formation enthalpies (`store::held`).
- **A ground node reacts at Flow's last temperature.** On a melting plateau that is the melting
  point: the fusion heat it holds is set aside while it reacts (`FlowView::fused`).
- **Gas a ground node takes or gives** crosses the link between it and its air node, one link
  flux for each substance (`store::Batch`), booked in one call with the reactions (a node gives the
  gas its reactions made, and they use the gas it took). Gas taken brings its heat at the air's temperature and
  gives the node back its latent heat; gas given takes its share of the node's heat by heat
  capacity and its latent heat. The air node pays the lift (1.5), a link flux to the potential.
  A node gives no more heat than it holds: where its gas would take more, each gas it gives is cut by the
  same share, and the rest stays in the node. Parts and bodies settle their gas through
  `change::GasMove` and `change::send_gas` (`change/gas.rs`) in the same way.
- **Parts** (`bond::step`): every 6 months, for that time, at their cell's temperature, with
  the air; their gas is settled with the air as a ground node's is.
- **Bodies** (`body::physics`): at each update, with catalysts (2.4).
- **Growth** is a conversion too, but only in bodies and paid by work (6.6, Copy).

### 2.3 What gas a ground node touches

A ground node's gas is its own gas plus its cell's share of the air above. **Under standing
liquid** (more than the pores hold) it touches only the gas dissolved in that liquid, which the
liquid's film renews from the air (the film time τ = depth / `dissolve`, set by `change::film`).
Each gas it takes is held at Henry's share × (1 + made·τ/eq) / (1 + used·τ/eq): a film in series
with the reactions. So dead matter on the sea floor rots only as fast as the film brings oxygen.

### 2.4 Change inside bodies

- **Catalysts.** A body's catalyst gene for reaction r gives a speed-up f = 10^gene (off below
  0). This is metabolism: "eating plants" means "my genes speed up the reaction that burns what
  plants store".
- **Activity.** The chemistry of a body runs in its key liquid, so the speed-up is
  1 + (f − 1) · a, with activity a (6.5). A dry body's enzymes do not work; the bare reaction
  still runs.
- **Inhibition by the product** (`Body::inhibited`). The speed-up is also times
  1 / (1 + Σ x / x*): x is the body's store of each product, x* the level it keeps of it. A
  product it keeps no level of stops the enzyme. Gases and the key liquid do not count. It acts on
  the reactions a body runs at its catalyst's pace (the light reaction, nitrogen fixing, any
  other); not on the burnings it runs on demand to pay its costs.
- **Heat kills enzymes** (`change::denature`). Above the `DENATURE` gene the speed-up falls off
  within a few K (`unfold`), smoothly. Keeping the shape to a higher temperature costs more upkeep
  (`t_firm`, `c_firm`).
- **Burnings and keepers.** A catalysed reaction that releases heat and takes a gas is a
  **burning**: the body runs it on demand to pay its costs (`body::plan`, `body::pay`), as far as
  the rate law allows at its temperature, with the air it breathes. A reaction that gets less
  heat out of a fuel than burning it would (`Chemistry::keeps`) keeps the rest in what it makes
  (nitrogen fixing, reaction 24): it runs at its catalyst's pace and its heat pays no upkeep.
- **What is fuel.** One rule, read at the body's own temperature (`Body::burns_at`): a store is
  fuel if it is an input of one of the body's burnings and is not a gas at that temperature.
- Enzymes cost upkeep by their **capacity**, whether they are used or not (6.2).

### 2.5 Phases and the freezing law

A substance is solid, liquid or gas by its melting and boiling points (`Chemistry::phase`).
**The melting point of a liquid in a mixture** (`Chemistry::melt_in`, used by `phase_in`) is
lowered by what is dissolved in it, by the ideal-solution law:

```
1/T = 1/Tm − R · ln(x) / Lf        x = the liquid's share among itself and what is dissolved in it
```

- Only the **solvent** is lowered: the liquid (liquid at 290 K) that is the most of the liquids
  there. Where the solvent is absent, it melts at its own point.
- Dissolved means: substances that melt below the solvent's melting point and are still liquid
  there (the salts 23 and 24, and the fat, 6, for want of better data). Gases do not count: a gas
  dissolves only to Henry's share, a very small amount. A solid is not dissolved.
- A solute keeps its own melting point.
- The whole amount of the liquid (ice and liquid) counts, so it freezes at one point.
- The same law is used by bodies (frost, evaporation, key liquids), by the ground's plateaus,
  ice cover and surface (`World::thaw`, `World::melt`), by parts, and by Change's own phases
  (`Reactor::phases`).

**Latent heat.** Flow keeps the fusion and vaporisation heat of the ground and the air. A gas in
the air holds its melting and boiling heat (`Substance::latent_gas`); every gas that enters or
leaves the air pays or gets it. A body holds the fusion heat of its molten stores, as a ground
node does (`Body::thermal`): it freezes and melts at a plateau, liquid it takes or gives carries
its fusion heat (`Body::heat_with`, `Chemistry::fused_heat`), and liquid that evaporates from it
takes its vaporisation heat beside. The books stay exact.

### 2.6 Pairs at balance and the sea's carbon

`change::reversible` finds every pair of reactions where one undoes the other.
`Reactor::balance` finds, from the chemistry alone, the amount at which such a pair goes at equal
rates. In chemistry 0 that is the lime–salt pair: 22 makes the dissolved salt 23 from lime, the
carbon gas and the liquid; 23 turns it back. At the start (`worldgen::terrain`) each wet cell
holds the salt at balance with its temperature and the 400 ppm air. A dry cell holds none. So the
sea holds several times the air's carbon and gives it back when the air thins. There is no
special rule for the sea.

### 2.7 Chemistry 0 in short

One fixed, Earth-like chemistry until M7 (handover 6.0): 9 elements, 26 substances, 28
reactions (`worldgen/src/chem0.toml`). In `core` and `worldgen` they are numbers only. Earth names
(for people) are in `library/chem0.toml`. The file gives the mass of each element
(`element_masses`); a substance's molar mass is the sum over its elements, so every reaction keeps
mass. Roles that the laws use:

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

Key reactions: 0 light to sugar; 1 burning sugar; 21 rotting/burning body matter (gives back
the nitrogen mineral and lime); 22 and 23 the lime–salt pair; 24 nitrogen fixing; 25 the nitrogen
mineral breaking down when hot (the nitrogen goes to the air); 26 rotting of the frame; 27
splitting the frame into sugar (needs an enzyme). Dead plant matter rots at a litter constant
(reactions 1, 21, 26: about a quarter a year at 290 K in open air).

**Growth conversions** (`Chemistry::build`, `Chemistry::builds`), which only bodies run, paid
by work:
- body matter: 118 sugar + 3 nitrogen mineral + 1 rock salt → 1 matter + 27 liquid
  (no heat in or out, with each substance in the state a body holds it);
- frame: 6 sugar → 1 frame + 1 liquid;
- lime: 1 rock salt → 1 lime + 1 carbon gas + 1 liquid (the gas goes to the air).

`Chemistry::minerals` lists the inputs of growth that no reaction burns (24 and 23).
`Chemistry::materials` lists the solids bodies build (25 and 10). A random chemistry has none of
these yet: its bodies do not grow.

---

## 3. Bond: things can be joined, joins break when pushed too hard

Files: `core/src/bond/` (`verbs` what bodies do to parts, `sense` what parts give a body,
`month` the monthly step). Also `choose::go`, `Body::height`, `Body::breaking`.

### 3.1 Parts and structures

- A **part** is a made object: solid walls, the liquid held in its hollow, heat, a size (3
  numbers) and a hollowness. It is a node of the store (`Class::Part`) that comes and goes: its id is
  its slot and generation (`bond::id`), and its slot goes to a new part, with a new generation, when
  it is gone. A fixed node (ground, air, deep, the boundary nodes, the potential) is named by its class
  and its index (`store::Node`). It lies
  in a cell or is held by a body. A body makes one with the action "form" (`bond::form`, cost
  `c_form` per kg), at most `form` of its mass.
- A **bond** is a link (`bond::Bonds`, each bond once) that joins two parts, or a part and the
  bonded ground. Its strength (N) comes from the weaker material at the joint and the contact
  area (`tensile`, `joint`). Making it costs `c_join` per m².
- A **structure** is the parts joined by bonds. It moves as one.
- Parts take their cell's temperature each month and react with the air every 6 months (2.2).

### 3.2 Loads and failure

Once a month (`bond::step`, `load` in `bond/month.rs`): each bond carries the weight hanging on it
from the anchored part, less what the liquid around holds up (buoyancy), plus the push of moving
liquid and of bodies moving through the cell (`bump` of their weight). A bond loaded past its
strength breaks. What has no support lies on the ground. A structure the liquid holds up floats
and drifts down the liquid's slope (`bond::afloat`).

### 3.3 Holding, carrying, digging

- Holding a part is a bond too. Its strength is the body's **force**: strength gene × its living
  body (`Body::built`) × gravity × `grip` (`bond::bears`, `bond::force`). A held part can add a
  lever (`bond::lever`). A body takes hold of a part only if its force bears the weight of the
  part's structure, and lets go when it no longer does (checked each month).
- Fuel a body carries adds no force. A hurt body is weaker.
- **Digging** (`bond::dig`) breaks bonded ground; its work goes with the body's force and time.

### 3.4 Containment

If a structure's walls close around a space, it has an **inside** (`bond::room`), joined to its
cell by one link, its enclosure (`bond::Enclosure`). Heat passes the link through the walls (their
conductivity × face over thickness, for the share of the space they close) and through the
openings (`leak` per m² of opening); light and rain pass only through the openings. A body inside
exchanges with the outside only through its share of that link (`bond::enclosure`: the bodies
inside share it), in series with its own surface. The inside holds almost no heat, so it is no
node: the series is solved in closed form. "Shelter" is nothing more than this low conductance.

### 3.5 Moving needs force

A mover pushes on what it passes with `bump` of its weight. So it must push that much itself: a
body moves only if its own force is at least `bump` × the weight it moves (its whole mass and
what it carries) × gravity (`choose::go`). The starting plants are too weak to move; the
starting eaters move while their mass is under about 6 times their living body.

### 3.6 What a body is built of holds it up and resists breaking

- **Height** (`Body::height`): a member stands as high as its materials hold it against its own
  weight, by Greenhill's self-buckling column:
  h⁴ = 7.837 / 4π × (Σ E·V) × V / (m·g). V is the volume of its soft tissue and of each solid it
  built, E their stiffness (Young's modulus, data field `modulus`), m the whole member (fuel and
  liquid are load). Soft tissue alone holds a 1 kg body 0.4 m. Height sets the order of light and
  rain (1.4).
- **Breaking** (`Body::breaking`): the energy to break a share f of a body is f × Σ V × the
  stress each material breaks at (data field `rupture`). A bite pays it; pushing through breaks
  as much as its energy pays for (6.8).

---

## 4. Copy: a piece of the graph is copied, with small changes

Files: `core/src/copy/` (`copy`, `mix`, `grow`, `young`, `select`), `core/src/nodes.rs` (population nodes),
`core/src/body/genes.rs` (genes, likeness, mutation), `core/src/lineage.rs`. The body side of
growth is in 6.6.

### 4.1 Genes

A body's genes (`body::Layout`) are:
- 21 **scalar genes**: `GAS`, `LIQ`, `HEAT`, `LEAF`, `STRENGTH`, `CHILD`, `COPY_MASS`,
  `INTERVAL`, `MUT`, `LEARN`, the four brain sizes `HIDDEN`, `MEMORY`, `SEEN`, `SIGHT`,
  `DISPERSE`, `DENATURE`, `REPAIR`, `WET`, `ALLOC`, `KEEP_T`, `COOL_T` (meanings in
  `core/src/body/genes.rs`);
- one **catalyst** gene per reaction (log10 speed-up);
- three genes per substance: **safe minimum** (a share of the body; below it the body dies,
  6.7), **uptake** (how much it takes of that substance), **keep** (the most it holds of it);
- the **born brain rows** (5.1), which are genes too.

**Mutation** (`body::vary`, `body::mutate`): each gene of a child is the parent's plus noise of
one step × 10^`MUT`. Each gene has a step and a range (`range` in `body/genes.rs`). Genes that are
off (catalysts, minimums, uptake below 0) have 20 steps of room below 0, so an off gene can wander
without hitting a wall. A catalyst step is 0.3 decades. Born brain rows change with a chance
`row_chance`, by `row_noise` (`brain::row_step`, `brain::shake`). Rarely (`new_lineage`) a child
changes the base of its lineage's brain and starts a new lineage.

**Likeness** (`body::alike`): two bodies are alike when every gene is within `alike` steps (4) of
the other; brain and sense sizes count in whole units. Likeness decides who mixes, who joins a
node and who merges.

### 4.2 Children come from a brood

- A body is **mature** when its living body per member (`Body::built`) is at or past its
  `COPY_MASS` gene (`Body::adult`). The members of a young node pass it one by one and move into
  the adult node of their kind (4.4).
- At each update `copy::grow` builds what it can (6.6). An immature body puts it all into its own
  growth. A mature body puts the share `ALLOC` into a **brood** (`Body::brood`: body matter held
  for children, not yet body) and the rest into its own growth. With `ALLOC` below 1 it grows all
  its life; with 1 it stops at maturity.
- When matter leaves a body (a bite, members that die, what it sheds), the brood falls in the
  same share as the matter (`Body::lose`).
- Whenever the brood covers one child's matter, a child is made (`copy::copy`, `emit` in
  `copy/mod.rs`), at the body's update, whatever its brain does. Bodies copy by physics only.
- **A child** has a body of 10^`CHILD` × its parent's copy mass (from seeds at −6 to half the
  parent at log10 0.5). It gets that matter from the brood, provisions of every other store in
  the proportion its parent's body holds them to its matter (the key liquid times 10^`WET`, so a
  body can make dry, dormant children), its share of the heat (it starts as warm as its parent),
  and its genes plus noise. The children of one litter get alike provisions, wherever they start.
- An individual makes whole children (the rest of the brood waits). A node makes fractional
  members. The parent keeps its body and pays nothing more: the matter was paid for when it was
  built.
- A share `DISPERSE` of a node's children starts in a random neighbour cell, however few (for an
  individual, the chance that its children do): less than one member with no node of its kind to
  join there waits in the brood, as the children who stay do. A birth is recorded in the cell where
  the children start.

### 4.3 Two parents

When a mature individual makes children, it looks in its own cell for a **partner** (`partner` in
`copy/mix.rs`): another mature body alike to it, the most alike one (a keyed random pick among
equals), never itself. A node's children have one parent: its members already mix their genes
among themselves (a node keeps one variance per gene and no covariances, which is free
recombination, Bulmer 1971), so a second parent would only do early what the monthly merge does.

- Each gene of the children is one parent's, drawn gene by gene. Each born brain row is one
  parent's, drawn row by row (a row is kept whole). Then the normal mutation.
- The chance to take the partner's is the partner's share of the members of both (`share` in
  `copy/mix.rs`), so the mean of all members is kept.
- A group of many children takes about that share from each, and its spread is what a merge of
  the two parents would pool (`mix`, `mix_rows` in `copy/mix.rs`).
- The partner gives and loses nothing: genes are information.
- No partner: the children copy one parent. Bodies that are not alike never mix, so **kinds are
  the groups that mix**.
- `Config::two_parents` off (`ew run --no-mix`) gives one parent always (a control).

### 4.4 Population nodes

A node (count > 1) is one body for many members (handover 7.1). It is a speed tool: it must give
the same patterns as individuals.

- A node keeps its **spread**: the variance of its members' genes around its mean, per gene, and
  one number for its born rows (`Body::spread`, `Body::row_spread`). The spread never grows past
  the split threshold (`body::most`), and a gene that is off (a catalyst or a gene of a substance
  below 0) keeps none where members join or are born (`body::spread_kept`): their differences
  there do nothing.
- **Joining** (`kin` and `emit` in `copy/mod.rs`). Children join the young (immature) node of
  their kind in their start cell: in every gene within `alike` steps plus `WITHIN` (0.5)
  standard deviations of both spreads (`body::within`). A child is always immature, so it never
  joins its mature parent node. Children who join are booked on the node: no body is built for
  them. With no node to join they start a body; less than one member with nowhere to go waits in
  the brood.
- **Young that grow up** (`copy/young.rs`). A kind in a cell lives as an adult node and a young
  node joined by a link (`Body::link`). A node's children who stay in its cell are booked on its
  young node along the link (found by kind and linked when there is none yet; children who start
  a young node are linked to their parent's node). A node keeps the mean and the variance of log10
  of its members' bodies (`Body::size_moments`: the mean as its lag below log10 of the mean body,
  `Body::size_lag`, the variance `Body::size_spread`), pooled at every join as the moments of the
  mix; newborns come in at one size. The mean is kept apart from the mean body: a mix of sizes is
  no lognormal. So members who joined earlier, and so have grown more, are the larger ones. At
  each update of a young node its members grow alike in proportion (their log10 bodies all rise
  by the growth of the mean), and those who passed the copy mass in it move along the link into
  the adult node (`copy::mature`). Young members lie below the copy mass (born smaller, they
  leave when they reach it), so their log10 bodies are a normal cut above at the copy mass,
  fitted to their mean and variance; those who leave are the top slice the growth pushed past it,
  with the share of the body a lognormal puts there and the stores with the body. No growth, no
  maturing: it is the flux across a size of a size-structured population (McKendrick 1926; von
  Foerster 1959). Those who stay are again a normal cut at the copy mass, and those who leave
  keep the mean and the variance of their slice. With no adult node in the cell, at least one
  member maturing starts one; less than one member left young joins the adult node whole. So
  members mature one by one, at the rate their growth gives, the mean of a young node never jumps
  to maturity, and while its members only grow its sizes stay between the birth size and the
  copy mass. Members who starve shrink alike, below their birth size too.
- **Members are kept exactly** (`nodes::Lot`, `nodes::add`: one helper for every join, merge,
  split and booking). Members, stores, heat and brood split by shares (the node keeps the
  remainder, so the parts add up to it) and join as sums: when members join a node, the count
  grows by exactly what they are (0.4 of a member adds 0.4). Genes, sizes (log10 bodies), born
  rows and brain rows become the means **by members**, and the spread pools each one's spread and
  its gap to the new mean.
- **Merging** (`nodes::crowd`, monthly). Alike nodes of one stage (both mature or both young) in
  a cell merge: the young never merge into their adult node, they mature into it. A cell with
  more than `crowd` nodes merges its most alike pairs while they are alike. Past twice `crowd` the
  most alike merge however unlike (a safeguard). A merge pools stores, heat, the means by count,
  and the spread.
- **Splitting** (`nodes::split`). A node whose two halves would no longer be alike splits in two
  along its widest gene (brain sizes measured in their own steps here), half the members, stores
  and heat each, if its cell has room. The mean is kept exactly. Then selection acts between the
  halves.
- **Gene means move only by selection.** Splits, draws and joins keep the mean of each gene
  exactly; near a wall the noise is shrunk, never clamped.
- **Selection inside a node** (`copy/select.rs`). Members are born and die in proportion whatever
  their genes, so a node's genes would move only by selection between nodes. So a node without a
  brain also runs its update, as far as its growth, for a sample member (`copy::sample`): the
  node's genes but one, drawn among those its members differ in, which is one standard deviation
  of its spread up or down (the gene and the side drawn by keyed random numbers), within its
  walls. The sample runs on the world as it is and its moves are dropped (6): it is a member that
  might be, and changes nothing. Its gain and the node's are the log of what can become body after
  growth over that before the update (`copy::buildable`: the body matter, brood with it, and the
  matter its stores could make as growth makes it, `copy::grow`): growth and reserves less deaths, so a
  gene that brings more fuel gains though the update builds no more. The mean
  of the gene then moves by the sample's step times tanh(sample's gain − node's gain), times the
  number of genes drawn from times `sample_every` (each gene is drawn that much less often: a
  node runs its sample on one update in `sample_every`, drawn, a speed setting), at most by the step
  times `sample_every`
  (`copy::select`): Price's equation for two halves a step either side of the mean (Price 1970)
  moves the mean by the step times tanh(half the difference of their log gains); the node's own
  update stands for the other side, and since the side is drawn, the error of that one-sided
  difference averages out. This is the breeder's equation, the change of a mean is its genetic
  variance times the gradient of log fitness (Lande 1976; Falconer and Mackay 1996). A gene that
  does nothing gains nothing and does not move, so where the step cannot change the gain no
  sample is run (`copy::select::read`): a scalar gene the update as far as its growth never reads
  in a node without a brain (`UNREAD`: those of its children, `CHILD`, `MUT`, `DISPERSE`, `WET`,
  `ALLOC`, read when children are made, and those of a brain); a gene of a reaction or a substance
  that is off at both ends; a gene that acts at a threshold the update does not reach: a safe
  minimum whose store holds more than twice the step's upper end, `KEEP_T` above the body and its
  ground by more than the step, `COOL_T` below them by more; an uptake gene, unless it turns the
  roots on or off for a substance that does not dissolve and the ground holds as liquid; and a keep
  gene, unless its substance is a solid it builds, one of its key liquids, a product it holds and
  that is not gas of a reaction it catalyses, or a liquid of the ground that dissolves (soaked up) or
  its roots take while its store lies under the level at the step's upper end (the update reads the
  level it keeps only for those). A wrong skip only slows that gene. The genes the update never
  reads move only by selection between nodes. A node with a brain has none: its update is also its decisions, which act on
  the world.
- **Members as individuals.** A node does an action of Bond through one member split off as an
  individual (`nodes::one`); if it then holds something or is inside, it lives on alone, else it
  goes straight back, and the node's rows and their spread stay as they were. `nodes::open` draws
  several members as individuals (for a close view), each with genes and born rows of its own
  drawn from the node's spread, and what the node learned on top.
- **Brain size in nodes** (`lineage::resize`): a node whose mean brain-size gene is a whole unit
  away from its lineage's size moves itself to a new lineage of that size, its brain carried over.
- **Age in nodes.** A node has no single age: its members die of age at the rate of the wear an
  individual would take (6.7).
- **The pace of nodes** (`Body::interval`, decision D13). A node without a brain updates at most every
  `node_days` (36 days, a speed setting): a node of many members that takes no decisions needs no
  update more often, and its update integrates the time since, as any body's does. A body with a
  brain, node or individual, keeps its `INTERVAL` gene: its actions and its learning go by decisions,
  and they do not add up over a longer span. The rules of an update that count by the gene's interval
  become rates over the update (`Body::paces`, the update over the gene's interval): the stores it uses
  refill a level per interval of its gene (`Body::room`; what is not used is shed), and frost kills
  at the rate its share per interval gives (`Body::over_span`, 6.5). So for a body without a brain the
  `INTERVAL` gene is the turnover time of its stores and of its frost, not a pace of decisions, and a
  shorter one costs it nothing (a known limit, `docs/foundation/decisions.md`). A shortfall of upkeep kills the
  share of the members its upkeep over the update left unpaid, whatever the update's length (6.2).
  Members who join a node between its updates (children, members maturing, merges) count their time
  from when they came: the node's last day and its mark of light become the means by members of its
  own and theirs (`nodes::add`), so its next update integrates as many member-days as its members
  lived in it.
- **Individuals mode** (`Config::nodes` off, `ew run --individuals`): bodies with a brain are
  never nodes. Bodies without one (the light-eaters) stay nodes in both modes: the check compares
  brains in nodes and alone. Siblings born together as individuals each get born rows
  of their own. It is for the M4 check of nodes against individuals.

---

## 5. Choose: a small brain sees a little, acts, learns and copies others

Files: `core/src/choose/` (`mod` the decision, `perceive`, `learn`, `feed` taking and giving,
`go` moving), `core/src/brain.rs` (the network and the named actions).

### 5.1 The brain

- One hidden layer over what the body perceives and remembers. Its **base** belongs to a
  **lineage** and changes only by evolution. Its **rows** (the readers of the hidden layer) are
  genes of each body.
- The rows give fixed action scores, shared scorers (one for every neighbour direction, one per
  verb for every body or part perceived), a signal, the shape of a part to make, the next memory,
  and an estimate of how well-being will go (a critic).
- Sizes are genes, each with a cost: hidden units (0 to 64; 0 is no brain), memory numbers (0 to
  12), bodies perceived (1 to 6), sight (1 to 4 cells). A body with no hidden units takes no
  decisions and pays nothing for a brain; its update is physics only. It has no born rows and no
  rows. The light-eaters start so (8): everything they do is their body's physics (6). A lineage may grow a brain back through the same gene (`lineage::resize`) if
  a brain pays: its rows then start as new weights (`brain::rerow`).
- **One brain per body or node** (`Body::rows`). Each body keeps its own rows: the rows it was
  born with plus what it learned. A population node's members share the node's rows. A child
  starts with its born rows plus what its parent learned (`Life::inherit`). When members join a
  node, its rows become the mean of both by members, as its genes and born rows do (4.4); where
  two brains differ in size, the members bring only their born rows (`Life::carry_rows`).

### 5.2 Perception

Limited and local (`choose/perceive.rs`): its own stores against their safe minimums, warmth,
size and light; its cell's loose substances, liquid, temperature and crowding; the season; signals
heard; its memory; the cells in 8 directions as far as its sight (nearer cells count more); up to
`SEEN` bodies in or next to its cell (their looks, signals, well-being, and how past dealings with
them went); up to 3 parts in its cell. Walls and cover hide bodies. Each input is scaled to about
one by a fixed table of scales.

### 5.3 Actions

One action is drawn by the scores at each decision (once per update, `choose::act`). If no action
is open, the body rests. Each costs energy that the body's own reactions must pay (`body::pay`);
if they cannot, it fails.

- **rest**;
- **move** to a neighbour (`choose::go`): cost `Physics::walk` per kg and metre (the real cost of
  transport, more per kg for a small body), plus climbing and the bodies in the way; needs force
  (3.5);
- **take** from the ground (`take_node`) or from another body (`take_body`), in `choose/feed.rs`;
- **put** to the ground (`put`) or **give** to another body (`give`);
- **watch** another body (imitation, 5.5);
- Bond actions: form a part, hold, take from or put to a part, join, break bonds, let go, join to
  the ground, dig, enter an inside (`bond::act` and the others in `bond/verbs.rs`).

The named actions are in `brain.rs` (`REST`, `PUT`, `TAKE`, `FORM`, `RELEASE`, `ANCHOR`, `DIG`;
per body `TAKE_FROM`, `GIVE`, `WATCH`). There is no copy action: children come from the brood
(4.2). There is no "farm", "trade" or "attack" action: those must be patterns of the actions
above.

**Taking** goes by uptake genes (`Body::wanted`, `body/intake.rs`), up to what the body can take
in the time (6.7), and from a cell only what the area it works holds (`sweep`). Handling costs
`c_handle` per kg. From another body it is also limited by its force against the other's; a
quicker body (shorter interval) may get away. **A bite tears body matter** (6.8).

**Giving** is of stores only: never body matter, brood or built solids, nor a key liquid below its
level, and only from stores above twice their safe minimum. A gift is at most what the receiver
could take by its own size in the time. A node gives as one member.

### 5.4 Learning

- **Well-being** (`Body::wellbeing_at`): the energy its reactions could release from its stores,
  in decisions' worth, less how near it is to its limits (stores to their minimums, release to
  upkeep, key liquids to freezing or boiling). No reward is hand-shaped.
- **Within a life** (`choose::learn`, `brain::learn`): at its next decision the brain learns from
  the change of the body's own well-being, by a policy gradient against the critic's estimate
  (actor-critic), into its own rows. The `LEARN` gene sets the rate. An error too big for a number
  changes nothing.
- **Across lives**: evolution of the born rows and the base (4.1).

### 5.5 Imitation, signals, memory

- **Watching** (`choose::learn::watch`) costs the turn. The watcher's rows move part of the way
  (`blend`) toward the watched body's rows, if the brains have the same sizes, and less the less
  alike the two bodies are (size, leaf, strength).
- **Signals**: each decision emits a small signal (2 numbers, with noise `signal_noise`). It has no
  meaning at first; others perceive it. It costs upkeep (`c_signal`).
- **Memory of individuals**: a body remembers up to 4 others (`Body::note`, `Body::recall`) and how
  dealings with them went (better or worse).
- **Cost of thinking** (`Physics::think`, `Lineage::cost`): each decision costs by the brain's
  sizes and weakly by the member's mass (mass^0.25). So a long interval saves brain cost.
- **Null run** (`Config::null`, `ew run --null`): no learning, no imitation, brain rows frozen,
  no new brain weights; genes still evolve. It is the run to compare a real run with.

---

## 6. Bodies

Files: `core/src/body/` (and `copy::grow` for growth). `body::live` (`body/live.rs`) runs one
update in stages: upkeep, the physics of the time since the last update (`body/physics.rs`),
keeping warm, the checks of the stores, growth and copying, shedding. Its decision
(`choose::act`) then sees the body as these stages leave it: its mass, heat capacity and
temperature after it sheds.

**Books.** What a body takes from and gives to its cell's ground node, air node and bodies'
node (its liquids, minerals and food, its breath and the gas of its growth, its heat, its
remains) are moves (`store::Move`), gathered over its update and booked by one `store::apply` at
its end (`store::Moves`, `World::open_moves`, `book_moves`). The body changes its own stores and
heat; the store books the other side. While it updates, it reads its cell's nodes as its moves
leave them (`World::g`, `a`, `heat_now`). It never asks a node for more than it holds: what it
breathes, soaks up, takes by its roots or eats is set by the law's own rate over the update's span
(6.4), so nodes and individuals take alike, and is never more than is there; and no heat below
`T_FLOOR`. A sample member's update (4.4) drops its moves. A part's monthly turn books its moves
the same way.

### 6.1 What a body is

- **Stores**: an amount of every substance (mol, all members together), and heat. Its
  temperature comes from its heat and heat capacity, with the fusion heat of its molten stores
  (`Body::temp`, `Body::thermal`).
- **Body matter** (substance 22) is what a body is built of. Its **living body**
  (`Body::built`) is its matter, less the brood, over 1 − `WET_TISSUE` (0.75: living tissue is
  three quarters liquid). What a body takes, builds, pays to keep, wears, pushes with and catches
  light with goes by its living body.
- **Fuel** (sugar, fat) and liquid beyond the body are carried, not body. Storing fuel costs no
  work; building matter does (6.6).
- `Body::size` is the body with the liquid it holds now (for height, volume and the safe
  minimums). `Body::mass` is everything (for moving, weight and load).
- The **key liquids** are the liquids its keep gene keeps (a bit mask, `Body::key_liquids`).
- **Levels** (`Body::level`): the most it holds of each substance. For the key liquid: h / (1 − h)
  of its matter (h its keep gene, at most 0.95). For a solid it builds: what its share makes of its
  matter. For any other store: keep gene × kg of living body. A full store is the satiety: there is
  no other limit on eating.
- **Supply** (`Body::supply`): what it can take and build per day, in kg-equivalents: its living
  body to the power `intake_exp` (0.75) from `m_ref` (1 kg), times the pace of its temperature
  (6.2).

### 6.2 Upkeep and paying

`body::upkeep` (`body/pay.rs`): the cost of living over the interval.

```
tissue = living body × (copy mass / m_ref)^(intake_exp − 1)
upkeep = [ days × ( tissue × (c_basal + c_cat·capacity + c_strength·STRENGTH + learning + signal)
                    + c_leaf × leaf area )
           + one decision ]
         × (1 − c_repair + c_repair·2^REPAIR) × activity × pace(T)
```

- **By the kg of the living body**, mass^1.0, while supply goes with mass^0.75 (the growth law of
  West, Brown and Enquist). So a body grows until keeping what it has takes all it can supply:
  its size is set by its own costs, not by a cap.
- **Across kinds** the cost per kg is also times (copy mass / 1 kg)^−0.25, so adults of different
  kinds pay Kleiber's mass^0.75, as real adults do.
- **Catalysts cost by capacity**: the sum, over the catalyst genes that are on, of 10^gene × k ×
  exp(−Ea / `t_cat`), with gases at their share of the starting air to their order.
- **Repair**: `c_repair` (a quarter) of upkeep at `REPAIR` 0, doubling with each unit of the gene.
- **Pace** (`Body::pace`): exp(−Ea (1/T − 1/`t_cat`)), Ea the barrier of its main burning
  reaction. A cold body pays less and a warm one more. Supply (6.1) uses the same factor, so the
  whole tempo of a body follows its temperature. No calendar rule: seasons come from this.
- **Paying** (`body::pay`, `body::plan`): the body runs its burnings one after the other, each as
  far as the rate law allows at its temperature with the air it breathes in the time. A cold body
  releases little. If they cannot pay, the body dies of `SPENT` (a node loses the members it
  cannot pay for: the share of its upkeep over the update left unpaid, 4.4).
- The products of burning take their heat at the body's temperature, so the body keeps its
  temperature as its stores change.

### 6.3 Heat

- **One heat network** (`body/exchange.rs`: `surface`, `film`). Heat goes from the core through
  the coat (the `HEAT` gene, which goes with mass^(2/3)) to the outer surface (skin and both faces
  of the leaf). From there it goes in parallel to the still air and by radiation. The still air's
  conductance is forced convection: `forced` × √(`wind_ground` / d) × the air's heat capacity, d
  the body's length or a leaf's size (`leaf_size`). Cover of other bodies (`shelter`; the mass of
  the others in its cell now, `cover_of`) and walls (3.4) are in series. Radiation from the surface goes to the ground (`radiated`).
- **Work heat stays** (`body::spend`, `body::hold` in `body/pay.rs`). The heat of upkeep and of
  paid work stays in the body and leaves through that network over the interval; the share
  (1 − e^−x)/x stays, x = conductance × time / heat capacity. The rest goes to the ground.
- **Reactions' heat** (`body/physics.rs`): a body reacts at its temperature, held by the ground it
  stands on as a store of the heat that ground can conduct into it over the update. It feels the
  ground temperature of Flow's last step. Gas taken from the air brings its heat; gas put out
  takes its latent heat and its share of the body's heat.
- **KEEP_T** (keeping warm): if the update would leave the body colder than this gene, it burns
  on demand, like its upkeep, the heat that brings it there, as far as its reactions can release
  it.
- **COOL_T** (keeping cool): if the update would leave it warmer than this gene, it evaporates the
  latent heat that brings it there, no faster than its still air carries vapour away (the Lewis
  relation), against the air's vapour deficit, and never below its liquid's safe minimum.
- Between the two genes it does neither. Founders start at 250 K and 320 K, so neither acts at the
  start.

### 6.4 Water: breath, rain, soaking, roots

- **Breath** (`breath` in `body/exchange.rs`): the `GAS` gene is the conductance of the body's
  pores (like a leaf's stomata), mol of air per m² of open surface (`Body::open_area`: leaf area
  plus skin) per day, in series with the still air. Gases and vapour go through it. Liquid over
  the body slows it (`gas_depth`). It reaches no more of each gas than its air node holds.
- **Pores close as it dries**: for vapour only, they open as far as the activity (6.5), solved
  exactly over the update (`closing`).
- **Rain** (`body/physics.rs`): the bodies of a cell catch 1 − e^−x of the rain that fell on the
  cell since their last update (x their leaf area per ground area; `World::rained`, the cell's rain
  since the start, and `Body::rained`, its mark, as for light), shared by height like the light. A
  body catches no vapour.
- **Soaking** (`LIQ` gene): a flow along the body's contact with the ground, at rate `LIQ` × the
  ground's wetness (standing or pore liquid; half wet at `wet_depth`) × its working share ×
  solubility per day, driven by how far it is below its level, while it loses what it
  evaporates. Solved exactly over the update; the ground's wetness makes it first order in what the
  ground holds when it is dry. A body afloat reaches no ground. After it grows (6.6)
  and has children (4.2), it soaks in the same way again at the same update (`soak_again`): the
  liquid its new tissue lacks and its children took, so its activity does not lag by one update.
- **What flows through** (`through` in `body/exchange.rs`): a soaking body loses where its
  soaking meets what its pores pass; what passes beyond its store comes from the ground as it
  goes. This is transpiration.
- **Roots for minerals**: the uptake genes of the minerals take them from the loose ground of the
  cell, at the rate it soaks liquids, up to its level and no more than the cell holds.

### 6.5 Activity and dormancy

`Body::activity`: a = min(1, l / l*)^n. l is the key liquid the body holds, l* the level it keeps,
n = `dormancy` (5.7, from the life of dry seeds). It multiplies the catalysts' speed-up (2.4), the
whole upkeep (6.2) and the building allowance (6.6). A dry body pays almost nothing, ages hardly
at all and can hardly act, but still takes its updates and decisions.

- **Frost**: if a key liquid freezes (at its lowered melting point, 2.5), the frozen share (all
  of it below that point; on its plateau, the share frozen there) times the activity dies (a node
  loses that share of its members, at that rate over an update longer than its gene's interval,
  4.4; an individual dies with that chance; cause `PHASE`). A dry body
  holds little free liquid to freeze.
- **Boiling**: a key liquid that boils kills the body.

### 6.6 Growth

`copy::grow`. A body turns sugar (and the minerals) into body matter and pays `c_copy` (10⁷ J)
of work per kg of matter, through `body::pay` (the work leaves as heat). It builds as far as all
of these allow:

1. its **allowance**: `c_build` × its supply / `m_ref` × the days of its interval × its activity
   (building takes time);
2. its **spare**: after the work and what it converts, its reactions must still release its
   upkeep over an interval at today's rate;
3. what its reactions can release now;
4. its stores, which must also cover the fuel the work burns. A body with none of a mineral builds
   no matter (6.9).

The solids it builds besides matter (frame, lime) are built first, each with its share
(`Body::share`, the keep gene of the solid, together at most 0.95) of the allowance and the spare,
up to the level its share makes of its matter. They pay no upkeep and are not the living body.
**Healing is growth**: lost matter is rebuilt the same way, at the same cost.

### 6.7 Taking, digesting, minimums, ageing

- **Intake limits**: per day, at most `take` × supply (6.1) at each take; from a cell only what the
  area it works (`sweep`) holds of the substances it takes.
- **Digestion** (`Body::digest`): body matter taken from outside (the ground, a body, a part, a
  gift) is broken back into what it was built of (sugar and the minerals) as it comes in, the
  reverse of growth. So food enters as fuel, never as body. A body takes body matter or a built
  solid only if a catalyst of its own breaks it (`Body::breaks`, `Body::can_take`). Matter taken
  is checked against the room of the stores it becomes (`Body::digest_room`).
- **Safe minimums** (`Body::lacking`): a store below its minimum gene (a share of the body) kills
  it (cause `LACK`). A store it burns (`Body::burns_at`) has no death line: there the minimum is
  only the reserve that growth and giving keep. A body short of fuel dies when it cannot pay
  (`SPENT`).
- **Shedding** (`shed` in `body/live.rs`): at its update a body gives back to the ground the
  minerals and built solids past their level, the key liquid past its level, and all of any other
  solid or liquid it neither burns nor is made of.
- **Ageing**: living wears a body. Its wear grows by the energy its upkeep turns over per kg of
  living body, over `e_life` × 10^`REPAIR` (`e_life` = 10⁹ J/kg, the rate-of-living figure of real
  life). An individual dies of age (`AGE`) when its wear reaches 1. A node's members die at the rate
  of that wear: at each update it loses the share 1 − e^−wear of the wear over the time since, so
  as many in a year whatever the length of its updates. A member split off a node gets a random
  wear in [0, 1).

### 6.8 Damage

`core/src/body/death.rs` (`thin`, `tear`, `wound`, `die`).

- **A bite** (`take_body` in `choose/feed.rs`) tears the share of the other's living body that the
  taker takes (or the share of its matter the taker ate, if that is more), and pays the energy to
  break it (3.6). Body matter breaks first, and spills the stores it held, in proportion, to the
  ground.
- **Pushing through** (`choose::wear`) breaks the share of each body there that the energy spent
  on it pays for.
- **Heat** burns body matter by its own reaction (21), so a burnt body is smaller.
- An individual lives on smaller and weaker (force reads the living body) and dies when half of
  its body or more is torn at once, or when a store falls below its minimum. A hurt adult is
  immature (its body is under its copy mass) until it heals: it makes no children meanwhile.
- **In a node** the size of the bite decides: less than one member's body takes part of every
  member's matter (grazing: the members live, smaller); one member's body or more takes whole
  members, the same share of them, so the members killed carry the bite's weight. What the taker
  takes of a store beyond the share those members held (a small store it takes much of) comes
  from the members who live.
- **Death causes**: `LACK` (a store under its minimum), `PHASE` (froze or boiled), `SPENT` (could
  not pay), `TAKEN` (taken apart by another body), `BROKEN` (broken by bodies moving through),
  `AGE` (worn out). At death the body's matter, stores and heat go to the ground (`body::die`).

### 6.9 Materials and minerals in bodies

- Founders start with a share of 0.02 of each built solid (`BUILT_SHARE` in
  `worldgen/src/life/make.rs`). Built solids count in the mass (moving, load) and set height (3.6),
  but are not the living body. Children get them as provisions; gifts never give them. Hard parts
  protect mostly by not being digested: a body without the enzyme that breaks a solid cannot use
  it as food.
- Growth takes 3 of the nitrogen mineral and 1 of the rock salt for each mol of matter. Plants get
  them by roots (6.4), animals by digesting the matter they eat (6.7). The rock salt does not last
  in a body: it falls out as lime by reaction 23 (in hours to weeks). Nitrogen fixing (reaction 24)
  lets a body with the gene make the nitrogen mineral from the air, burning 14 units of its sugar
  for each mol, slowed as its store nears its level (2.4).

---

## 7. Soil and minerals

The two minerals (`Chemistry::minerals`) live in the liquid of the ground.

- **Nitrogen mineral (24).** Placed when life starts (year 2, `worldgen::life::soil`): 0.357 mol
  per m² on every land cell (1.1) not under half a metre of standing liquid. After that its only
  input is fixing by bodies (reaction 24). Rotting of body matter (21) gives it back; heat breaks
  it down (25) and sends the nitrogen to the air.
- **Rock salt (23).** Made in wet cells from lime, the carbon gas and the liquid (reaction 22),
  turned back by reaction 23. The pores of wet land hold it near its balance (2.6). Rotting
  gives the rock element back as lime, which meets liquid and gas and becomes the salt again.
- Both move with the liquid by its share (1.2): rain washes them from slopes into low ground and
  the sea, and they stay where liquid evaporates. So fertility differs from place to place, and
  plants that fix nitrogen and shed it make rich ground around them.

---

## 8. Starting conditions (not laws)

What is placed at the start is an initial condition. Every run prints a seeding line for each
kind. `worldgen` places it (`worldgen::schedule::Schedule`, which a snapshot carries, so a resumed
run places what is still due); the laws never read a kind.

- **Terrain** (`worldgen::terrain`): an island with mountains, a metre of loose ground, the sea's
  salt at balance with a 400 ppm air (2.6). The starting air thins with height by its own scale
  height.
- **Life** starts at year 2 (`--life-year`). `ew run` places light-eaters first, then eaters 20
  years later (`worldgen::life::KINDS`, rows 0 and 1). `ew run --roster` places ten kinds
  (`worldgen::life::ROSTER`): light-eaters of three sizes, eaters of remains, eaters of
  light-eaters and eaters of eaters of several sizes, each at its own year. Founders go on open
  land: land (1.1) under no more than half a metre of standing liquid.
- **Founders are adults**: a founder's living body is its copy-mass gene. Its mass is 30% sugar
  (60% for the two big slow kinds), the rest liquid and matter.
- **Brains**: animals start with hand-set brain biases (`Seeding::hand`), because neutral
  founders die before they learn (`--neutral-eaters` for neutral brains). Light-eaters start with
  no brain (`HIDDEN` and `MEMORY` 0): they take no decisions and pay for none, and live by the
  physics of their bodies (6). Genes and rows get a spray of mutation (`--spray`).
- **Fixers**: a tenth of the starting light-eaters carry the nitrogen-fixing gene
  (`worldgen::life::FIXERS`, `FIX_GENE`; `--fixers`). The share comes from real floras.
- **Founder genes**: some are measured from the laws (`LIGHT_X`, `LITTER_X`, a real leaf's
  `GAS`); `HEAT`, `KEEP_T`, `COOL_T`, `CHILD` and `ALLOC` are the same for every kind.
