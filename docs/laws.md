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
| the shape of the world | `space/` (`Mesh`: levels of cells and links with their widths, parents and children; `square`: the square map; `stack`: layers above and below a cell); `grid.rs` is a thin view on it |
| the shared maths | `solve/` (`spread`: the implicit step along links; `downhill`: the high-to-low order and one pass down it; `lines`: the implicit step along a line) |
| settings, checks | `params.rs` (`Config`, `Physics`), `world.rs` (`World`, `Counters`), `check.rs` (the books); the nodes and their books in `store/` |
| ground heat | `ground.rs` (heat capacity, melting plateaus, `World::thaw`, `World::t_ground_last`); `flow::land::conduct` (sideways) |
| 1 Flow | `flow/` (`mod`, `scratch`, `land`, `liquid`, `drain`, `creep`), `flow/air/` (`mod`, `radiation`, `exchange`, `wind` with `wind/along` and `wind/mesh`, `sky`); `lanes.rs` (paths) |
| 2 Change | `change/` (`mod`, `kernel`, `balance`, `gas`); the chemistry in `chem/` (`mod`, `thermo`, `roles`) |
| 3 Bond | `bond/` (`mod`, `verbs`, `sense`, `month`) |
| 4 Copy | `copy/` (`mod`, `mix`, `grow`); population nodes in `nodes.rs`; lineages in `lineage.rs` |
| 5 Choose | `choose/` (`mod`, `perceive`, `learn`, `feed`, `go`); the network in `brain.rs` |
| bodies | `body/` (`mod`, `genes`, `tissue`, `metab`, `exchange`, `live`, `physics`, `pay`, `death`, `intake`); the registry in `life.rs`; the cells seen by bodies in `survey.rs` |

**Words used here**

| Word | Meaning |
|---|---|
| cell | one square of the map (2 km × 2 km). It has a ground node and sees an air node: its parent at the mesh's air level (`grid::AIR`), whose share over it is its area over the node's (`Grid::air_share`). |
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

1. Every `phys_days` days (3): `survey::survey` (what each cell holds, light and shade),
   then Flow (`flow::step`), then Change (`change::step`).
2. On the first day of the month: Bond (`bond::step`): parts, loads, breaking.
3. Every day: `survey::day` updates the bodies whose interval ends that day (each body every 5 to
   360 days, set by its `INTERVAL` gene).

Inside Flow (`flow::step`): radiation every call. Each other part has its own period and runs
when that period is over, for all the time since: evaporation and rain every `exchange_days`
(3), wind every `air_days` (3), liquid, drainage and the work of moving liquid every `flow_days`
(15), heat conducted sideways through the ground and creep once a year.

**Sleep and wake** (`clock::Sleep`). A node's own process may sleep while the node is calm: until an
alarm time, or until a call of `store::apply` brings the node, net, more than `wake_share` (5%) of
what it holds of an amount (heat wakes no node; what only passes through a node wakes nothing). When it is due, its law brings it up to date for all
the time since it last did, in one span. Change's reactions in ground and air nodes sleep so (2.2).

**One ground temperature.** Every law and the tape read the ground temperature of Flow's last
step (`World::t_ground_last`). Where Flow has not run yet, `flow::ensure` builds that state from
the world as it is.

**Checks before a run.** `Config::validate` checks the settings: each period is a whole multiple
of `phys_days`, and `Physics::validate` (`dormancy` above 1, `light_yield` above 0 and at most 1).
`Chemistry::validate` checks a chemistry: indices, densities, and that every reaction and
conversion keeps its elements and its mass.

**The books.** Every move takes from one node exactly what it gives to another, with its heat.
Elements are never made or lost. Three boundary nodes stand for what lies beyond the world
(`store::Boundary`): the sun gives the light the world absorbs, space takes the heat it
radiates, and outside gives and takes what edits put in and take out. They hold what crossed,
with the sign of a node (the sun's heat falls by what it gave), and every crossing is booked
on them (a `Move` from or to one, a link flux, or `World::cross`). The potential energy of the
world's matter is one more node of the store (`store::Class::Potential`, whose heat is
`World::potential`): a lift moves heat into it and a fall takes it back. So the world and the
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
- **Deep store.** Under the loose ground lies a store in the bed (`World::deep`, the store's `Class::Deep`): the
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
  fill), in the same `spread` as the pores.
- **What goes with the liquid.** A move takes its share of every liquid and of what is
  dissolved in it (salts, dissolved gases), with its heat. A salt is written in the data as a
  liquid that never boils (substances 23 and 24): it goes with the liquid by its share, and
  stays behind when the liquid evaporates. There is no other rule for dissolved solids.
- **Lost head is heat.** The height a liquid loses becomes heat in the ground (or in the deep
  store), and does work (1.3).

**Potential energy.** Lifting matter takes heat and a fall gives it back. The world holds the
difference as potential energy (`World::potential`, the store's potential node), so energy stays
exact. This holds for liquid and solids going downhill, for gas going into the air and coming out
(1.5), and for gas between air nodes. In Flow each lift and fall is a link flux between that node
and the node that pays or gains (`flow::Falls`), booked with the move that makes it.

### 1.3 Erosion and creep

Moving liquid loses energy (its lost head). That energy does work where it is lost
(`flow::liquid`, at every liquid step, in the same pass as the liquid):

- **Carrying.** It carries loose solids: `carry` m³ per J. The load leaves a cell in the share the liquid leaves it,
  so it settles where the flow slows (in pools and lakes), and what is over the capacity drops.
- **Wear.** It breaks bonded bed into loose ground: `wear` m³ per J, divided by the bed's
  strength. The bed is no node of the store yet (its skin's heat is the ground node's): wear and
  digging (3.3) change it by one helper, `World::loosen`.
- Bodies on a cell hold the ground: they cut carrying and creep by 1 / (1 + their mass per
  area / `hold`).
- **Creep** (`flow::creep`, once a year): loose ground slides downhill like a slow diffusion,
  `creep` × softness × thickness × slope, edge by edge, each from what the edges before it left;
  the store books it all in one call.

`carry`, `wear` and `creep` come from real erosion rates (`docs/numbers.md`). Most rock that
leaves the land is lime that Change dissolves (reaction 22, 2.6) and the liquid takes away.

### 1.4 Sun, heat and radiation

`flow::air::radiation`, every Flow call.

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
  implicitly (stable at any step). The store books it all in one call: the sunlight from the sun
  node to each ground node, what the ground gives its air node (with what leaves to space through
  it), and what each air node sends to the space node.

**Light on bodies.** The light that bodies' leaves absorb warms the ground as it falls, except
the share their light reactions store. That share is set aside in the cell (`World::light_pool`,
`survey::survey`). A body takes its share at its update. Only the difference to what it really
stored is settled with the ground. **Heat drawn from the ground** works the same way
(`World::heat_pool`): the heat each body draws from its ground per second at its update is set
aside as time passes, so a body that updates every 20 days does not pull 20 days of heat out of
the ground at once.

**Shade** (`survey.rs`). Light passes the bodies of a cell from the tallest member down. Each
layer of leaf area x per ground area takes 1 − exp(−x) of what reaches it. Equal heights share a
layer. Height is what the body's materials hold up (3.6). Rain on bodies is shared the same way
(6.4).

### 1.5 Evaporation, rain and dissolved gases

`flow::air::exchange`, every `exchange_days`.

- **Evaporation** by vapour pressure: the ground's saturation pressure against the vapour
  pressure of the air near it. Wet ground evaporates at the open rate when liquid stands on it
  or its pores hold their field capacity or more; drier ground in proportion to its liquid.
- **Rain.** Where the air a cell sees holds more vapour than it can at its temperature (colder
  on higher ground), the rest rains out. Condensing air makes clouds that cover part of the sky
  (`cloud`).
- **Lift.** Gas that goes up into the air, from the ground or from bodies, parts, ground
  reactions and growth, is lifted from the ground's head to the air's mean height: the air's heat
  pays for it. Gas that comes down (rain, gas taken by reactions or bodies) gives that back to the
  air's heat. The difference is `World::potential`.
- **Booked by the store.** Each pass (evaporation, condensation, dissolving) is booked at its end
  in one call: a link from each ground node to its air node carries the gas and the heat that
  leaves the ground; the air receives that heat less the latent heat it holds of the gas
  (`store::held`). Each pass reads the air as it was before it. Bodies, parts, ground reactions
  and growth still change the air through `flow::Sky::cross` (`flow/air/sky.rs`).
- **Gases dissolve** (`exchange::dissolve`). A gas that cannot condense on the ground dissolves
  in the liquid there toward Henry's share (solubility × partial pressure × the liquid's amount),
  and comes out when the liquid warms, thins or freezes. It crosses the surface through the
  air's film (`evap`) and the liquid's film (`dissolve`) in series. Dissolving frees the
  vaporisation enthalpy into the liquid; coming out takes it back. Dissolved gas moves with the
  liquid and returns to the air only here.
- **Known limit.** The whole depth of a liquid is one mixed layer (for heat and for gas). A deep
  sea warms over centuries and takes up gas in months; a real one is the other way round.

### 1.6 Wind

`flow::air::wind`, every `air_days`. The air nodes are the coarser level of the mesh
(`grid::AIR`); the wind runs on that level's links (the cells of two nodes that share a border).
Gas moves over each link by three pushes:

1. the pressure of the whole column (p = nRT/V), from high to low;
2. overturning: at equal height, the lower half flows to the warmer column and the upper half
   back, through half the column across the link's width; both are turned sideways by the
   planet's rotation (`rotation`, by the warmth's gradient across the link, at the latitude of
   the link: the mean of its two nodes'); vapour crowds in the lower half;
3. eddies mix the two columns (`mix`, across the link's width over its length).

It is solved implicitly, so any speed is stable. Where the links form lines (the rows and
columns of the square map) it is solved along them (`wind/along`, `solve::lines`). A call is long
against the time the air takes to mix from node to node, so one order of the two directions would
leave stripes along the lines solved last: the lines are solved in both orders (east–west then
north–south, and the other way) from the same start, and the air keeps the mean of the two. On
any other mesh (`wind/mesh`) three steps of `solve::spread` on the links do it: the mixing and
the overturning carry each gas by its share of the column, and the heat per mol; the evening of
pressure moves the amount by the pressure, each gas and the heat going with it at the share of
the node it leaves. There the overturning is an exchange of whole columns, so the vapour it
brings to the warmer column on lines is not carried.

Each call starts from the air nodes' amounts as they are (`air::sums`), with what Law 2 and bodies
changed. What crosses the links over the whole solve is booked at once by `store::apply` (gas and
heat; a node gives only its net loss, so gas that passes through it needs nothing held). Air in
Flow moves as whole columns, so there is no wind near the ground; bodies use the anchored
`wind_ground` (6.3).

### 1.7 Paths

`core/src/lanes.rs`. A body that moves through a cell spends energy pushing through the bodies
there (`choose::go`, `choose::wear`). That breaks what it pushes through (6.8) and clears the
link it used (`lanes::clear`, kept per link of the cell level in `World::links`, `store/links.rs`). A
cleared link is cheaper to cross (`lanes::resist`, `lanes::eased`). Once a year, with creep, the bodies grow back
into cleared links (`lanes::regrow`, rate `regrow`). This one rule is what can make paths.

### 1.8 Stability

No amount goes below zero: a node never gives more than it has. `store::apply` is the one guard
of Flow's moves: what would take more from a node than it holds is scaled down, and what the
scaling left is counted (`Counters::scaled`). The moves the store does not book yet (bodies, parts,
ground reactions) take heat from a ground node no further than `T_FLOOR` (`ground::draw`);
whoever asked gets that much less, and it is counted (`Counters::clamped_heat`).

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
  that far within a step reacts every step. A call of `store::apply` that brings it, net, more
  than `wake_share` of an amount wakes it. The bodies change it outside `apply`, and its
  temperature does not wake it: `sleep_days` bounds how long it reacts without those changes. This is a speed tool, not physics (`sleep_days` 0: every node
  reacts every step). A reaction too slow to matter over the world's year is frozen.
- **Booked by the store.** The nodes due in a step react from the world as it was before any of
  them did (each ground node from its share of its air node), and `store::apply` books them
  together. A node's reactions are one `store::Transform`: what they made less what they used of
  each substance, from how far each reaction went (`Reactor::made`; summed exactly where its
  terms cancel, so the transform keeps every element). The heat of reaction follows from the
  formation enthalpies (`store::held`).
- **A ground node reacts at Flow's last temperature.** On a melting plateau that is the melting
  point: the fusion heat it holds is set aside while it reacts (`FlowView::fused`).
- **Gas a ground node takes or gives** moves between it and its air node as `store::Move`s: first
  the gas it takes, then its reactions, then the gas it gives, each booked in its own call, as
  each needs what the one before brought. Gas taken brings its heat at the air's temperature and
  gives the node back its latent heat; gas given takes its share of the node's heat by heat
  capacity and its latent heat. The air node pays the lift (1.5). `apply` scales the gas the node
  cannot pay for, so it stays in the node. Parts and bodies settle their gas through
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
  numbers) and a hollowness. It is a node of the store (`Class::Part`): the node table gives it its
  id and its cell, and gives its slot to a new part, with a new generation, when it is gone. It lies
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

Files: `core/src/copy/` (`copy`, `mix`, `grow`), `core/src/nodes.rs` (population nodes),
`core/src/body/genes.rs` (genes, likeness, mutation), `core/src/lineage.rs`. The body side of
growth is in 6.6.

### 4.1 Genes

A body's genes (`body::Layout`) are:
- 22 **scalar genes**: `GAS`, `LIQ`, `HEAT`, `LEAF`, `STRENGTH`, `CHILD`, `COPY_MASS`,
  `INTERVAL`, `MUT`, `LEARN`, `FORGET`, the four brain sizes `HIDDEN`, `MEMORY`, `SEEN`, `SIGHT`,
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
  `COPY_MASS` gene (`Body::adult`).
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
- A share `DISPERSE` of a node's children starts in a random neighbour cell (for an individual,
  the chance that its children do). A birth is recorded in the cell where the children start.

### 4.3 Two parents

When a mature body makes children, it looks in its own cell for a **partner** (`partner` in
`copy/mix.rs`): another mature body alike to it, the most alike one (a keyed random pick among
equals), never itself.

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
  the split threshold (`body::most`).
- **Joining** (`target` in `copy/mod.rs`, `nodes::join`). Children join the first node in their
  start cell that they belong to (`body::within`): in every gene within `alike` steps plus
  `WITHIN` (0.5) standard deviations of both spreads; in the same **size class** (member bodies
  within `alike` steps of the copy-mass gene, about a factor 6); and immature. A child is always
  immature, so it never joins its mature parent node. With no node to join it starts a body; less
  than one member with nowhere to go waits in the brood.
- **Members are kept exactly** (`nodes::take_in`). When members join a node, the count grows by
  exactly what they are (0.4 of a member adds 0.4). Genes and born rows become the means **by
  members**, and the spread pools each one's spread and its gap to the new mean.
- **Merging** (`nodes::crowd`, monthly). Alike nodes of one class in a cell merge. A cell with
  more than `crowd` nodes merges its most alike pairs while they are alike. Past twice `crowd` the
  most alike merge however unlike (a safeguard). A merge pools stores, heat, the means by count,
  and the spread.
- **Splitting** (`nodes::split`). A node whose two halves would no longer be alike splits in two
  along its widest gene (brain sizes measured in their own steps here), half the members, stores
  and heat each, if its cell has room. The mean is kept exactly. Then selection acts between the
  halves.
- **Gene means move only by selection.** Splits, draws and joins keep the mean of each gene
  exactly; near a wall the noise is shrunk, never clamped.
- **Members as individuals.** A node does an action of Bond through one member split off as an
  individual (`nodes::one`); if it then holds something or is inside, it lives on alone, else it
  goes straight back, and the node's rows and their spread stay as they were. `nodes::open` draws
  several members as individuals (for a close view), each with genes and born rows of its own
  drawn from the node's spread.
- **Brain size in nodes** (`lineage::resize`): a node whose mean brain-size gene is a whole unit
  away from its lineage's size moves itself to a new lineage of that size, its brain carried over.
- **Age in nodes.** A node has no single age: at each update a share of its members dies of age,
  equal to the wear an individual would take in that time (6.7).
- **Individuals mode** (`Config::nodes` off, `ew run --individuals`): animals are never nodes.
  Light-eaters stay nodes in both modes. Siblings born together as individuals each get born rows
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
  decisions and pays nothing for a brain; its update is physics only.
- **Shared group brains** (`choose::learn::regroup`, monthly; `life::Group`). Bodies that stay
  near each other share one set of rows: the mean of their members' born rows plus what the group
  learned. Every group number is **weighed by members** (a node counts its count): the mean rows,
  the forgetting rate, which group is bigger, and how far a body's step moves the shared rows. The
  group's count is right at all times. A body further than `leave` cells from its group's centre
  leaves with a copy (`choose::go`). One that stays `stay` days within `join` cells of a bigger
  group of its lineage joins it. What a group learned fades at the mean `FORGET` rate of its
  members. `Config::share_brains` off (`ew run --own-brains`) gives one brain per body.

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
  (actor-critic). The `LEARN` gene sets the rate. A node's step moves the group by its members'
  share. An error too big for a number changes nothing.
- **Across lives**: evolution of the born rows and the base (4.1).

### 5.5 Imitation, signals, memory

- **Watching** (`choose::learn::watch`) costs the turn. The group's rows move part of the way
  (`blend`) toward the rows of the watched body's group, if the brains have the same sizes, and
  less the less alike the two bodies are (size, leaf, strength).
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
keeping warm, the checks of the stores, growth and copying, shedding.

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
  cannot pay for).
- The products of burning take their heat at the body's temperature, so the body keeps its
  temperature as its stores change.

### 6.3 Heat

- **One heat network** (`body/exchange.rs`: `surface`, `film`). Heat goes from the core through
  the coat (the `HEAT` gene, which goes with mass^(2/3)) to the outer surface (skin and both faces
  of the leaf). From there it goes in parallel to the still air and by radiation. The still air's
  conductance is forced convection: `forced` × √(`wind_ground` / d) × the air's heat capacity, d
  the body's length or a leaf's size (`leaf_size`). Cover of other bodies (`shelter`) and walls
  (3.4) are in series. Radiation from the surface goes to the ground (`radiated`).
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
  the body slows it (`gas_depth`).
- **Pores close as it dries**: for vapour only, they open as far as the activity (6.5), solved
  exactly over the update (`closing`).
- **Rain** (`body/physics.rs`): the bodies of a cell catch 1 − e^−x of the cell's rain (x their
  leaf area per ground area), shared by height like the light. A body catches no vapour.
- **Soaking** (`LIQ` gene): a flow along the body's contact with the ground, at rate `LIQ` × the
  ground's wetness (standing or pore liquid; half wet at `wet_depth`) × its working share ×
  solubility per day, driven by how far it is below its level, while it loses what it
  evaporates. Solved exactly over the update. A body afloat reaches no ground.
- **What flows through** (`through` in `body/exchange.rs`): a soaking body loses where its
  soaking meets what its pores pass; what passes beyond its store comes from the ground as it
  goes. This is transpiration.
- **Roots for minerals**: the uptake genes of the minerals take them from the loose ground of the
  cell, at the rate it soaks liquids, up to its level, no more than half of what the cell holds.

### 6.5 Activity and dormancy

`Body::activity`: a = min(1, l / l*)^n. l is the key liquid the body holds, l* the level it keeps,
n = `dormancy` (5.7, from the life of dry seeds). It multiplies the catalysts' speed-up (2.4), the
whole upkeep (6.2) and the building allowance (6.6). A dry body pays almost nothing, ages hardly
at all and can hardly act, but still takes its updates and decisions.

- **Frost**: if a key liquid freezes (at its lowered melting point, 2.5), the frozen share (all
  of it below that point; on its plateau, the share frozen there) times the activity dies (a node
  loses that share of its members, an individual dies with that chance; cause `PHASE`). A dry body
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
  life). An individual dies of age (`AGE`) when its wear reaches 1. A node loses that share of its
  members at each update. A member split off a node gets a random wear in [0, 1).

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
  founders die before they learn; light-eaters start with neutral brains but for one bias, taking
  from a body, which keeps its hand-set -4, so almost no founder plant leans to bite another.
  `--neutral-eaters` and `--plant-bias` change this. Genes and rows get a spray of mutation (`--spray`).
- **Fixers**: a tenth of the starting light-eaters carry the nitrogen-fixing gene
  (`worldgen::life::FIXERS`, `FIX_GENE`; `--fixers`). The share comes from real floras.
- **Founder genes**: some are measured from the laws (`LIGHT_X`, `LITTER_X`, a real leaf's
  `GAS`); `HEAT`, `KEEP_T`, `COOL_T`, `CHILD` and `ALLOC` are the same for every kind.
- **The big-brain test** (`--clever-year N`, `worldgen::life::seed_clever`) copies living eaters
  twice: once with a big brain, once unchanged.
