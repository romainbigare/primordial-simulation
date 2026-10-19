# The laws as they are now

How each law of the world works, in the present tense. When and why a rule came is in `docs/history.md`. Every
number, its unit and its anchor are in `docs/numbers.md`: this file gives a number only where it helps to read the
rule. What a run records is in `docs/tape.md`; the plan and the rules of work in `docs/handover.md` ("handover 6.2"
points there). The code is the truth: each part names its file and function, for example `body::upkeep` (`upkeep` in
`core/src/body/pay.rs`).

**Where the code is** (under `core/src/`; each law below names its own files): time in `clock/` and `schedule.rs`;
the shape of the world in `space/` (`World::mesh`: cells, links with their widths, levels; the square map is read only
where a law is about its layout). The map is an island of 2 km squares, or a globe (`ew run --globe`): squares of
latitude and longitude from pole to pole whose rows wrap east to west, each row's area and borders from its latitude
on a sphere of 6,371 km (cells narrow toward the poles; no link crosses a pole), the air a coarser level that wraps
too; the shared maths in `solve/`; settings in `params.rs`; the nodes and their books in
`store/`, `world.rs` and `check.rs`; ground heat in `ground.rs`; the registry of bodies in `life.rs`, the cells as
bodies see them in `survey.rs`.

**Words used here**

| Word | Meaning |
|---|---|
| cell | one square of the map (2 km × 2 km on the island; on a globe a square of latitude and longitude), with a ground node; it sees its parent air node at the air level (`Mesh::share`: its area over the node's) |
| node | anything that holds stuff: the ground of a cell, an air column, a body, a part |
| body | a living node with genes and maybe a brain |
| member | one living thing inside a body. A body of 1 member is an **individual**; of many, a **population node** (alike members counted together) |
| gene | a number a body inherits. Many genes are log10 (1 means ×10). A gene below 0 is often "off" |
| catalyst | a gene that makes one reaction faster inside the body (metabolism) |
| key liquid | the liquid a body's chemistry runs in (substance 0 in chemistry 0) |
| light-eater | a body whose light-reaction gene is high (plant-like); others are "animals" in the tape |
| plateau | a node stays at a melting point while one substance melts or freezes in it |

---

## 0. One step of the world

Time is a whole number of seconds (`World::time`); the day, month, year and season come from it by the calendar
(`Config::calendar`). `step` (`schedule.rs`) moves the world one month. Each process has a period (`clock::Every`)
and, at one time, they run in the order written once in `schedule::processes`:

1. Every `phys_days` (10 days): the survey of the cells (`survey::survey`: what each cell holds, light and shade, its
   bodies' node); then the parts of Flow that are due: the state of ground and air that the other laws read until the
   next physics step, the air's step (every `air_hours`, the same 10 days), liquid, drainage and erosion (every
   `flow_days`, 30), heat conducted sideways and creep (yearly), the recheck of what is present (every 3 months); then
   Change (`change::step`).
2. On the first day of the month: Bond (`bond::step`).
3. Every day: `survey::day` updates the bodies whose interval ends that day (each by its `INTERVAL` gene, 5 to 360
   days; a population node without a brain at least every `node_days`, 4.4).

Each run stands for the time to that process's next run, so Flow keeps no timer of its own.

**Sleep and wake** (`clock::Sleep`). A calm node's own process may sleep until an alarm, or until one of its amounts
has grown by more than `wake_share` of what it held when it fell asleep, whatever brought it. Its law looks for such
gains once at its own step (`Sleep::watch`); heat wakes nothing. When due, the law brings the node up to date for all
the time since, in one span. Change's ground and air nodes sleep so (2.2).

**One ground temperature.** Every law and the tape read the ground temperature of Flow's last step
(`World::t_ground_last`); before Flow first runs, `flow::ensure` builds it from the world as it is.

**Checks before a run.** `Config::validate` (each period at least 1, the air's step divides the physics step,
`Physics::validate`) and `Chemistry::validate` (every reaction and conversion keeps its elements and its mass).

**The books.** Every move takes from one node exactly what it gives to another, with its heat. Three boundary nodes
stand for what lies beyond the world (`store::Boundary`): the sun gives the light the world absorbs, space takes the
heat it radiates, outside gives and takes what edits put in and take out (`World::cross`). The potential energy of
the world's matter is one more node (`store::Class::Potential`): a lift moves heat into it, a fall takes it back. So
the world and these nodes keep every element and the energy; `check::drift` checks it, and a run prints the drift
every year. It must stay near rounding.

**No overdraw.** A law never asks a node for more than it holds: `spread` keeps amounts positive by its implicit
step, `downhill` by its pass, the reactor by going no further than its scarcest input, and where a law could still ask
too much it cuts that node's own outgoing links (`solve::spread::within`), never a whole field. `store::apply` only
adds: what rounding leaves just below zero it sets to zero and counts (`Counters::rounded`; a debug build checks it is
rounding). A body or part takes heat from a ground node no further than `T_FLOOR` (`World::take_from_ground`,
counted in `Counters::clamped_heat`).

---

## 1. Flow: stuff moves from high push to low push

Files: `core/src/flow/` (ground and liquid), `core/src/flow/air/` (air, radiation, rain, wind).

### 1.1 The ground of a cell

A cell's ground has layers, from the bottom: the **bed** (bonded rock), the **loose ground** (grains that can move),
soft cover (solids warm enough to be soft) and standing liquid. `flow::land::surface` builds them;
`flow::land::ground_state` finds the temperature (`World::thaw` the same from the amounts now).

- **Heat.** The top `skin_m` of ground and everything on it trade heat with the surface; its temperature comes from
  its energy and heat capacity.
- **Heat conducted sideways** (`flow::land::conduct`, once a year for the whole year): one implicit step of
  `solve::spread` along the links with a shared border; a link passes the harmonic mean of its nodes' conductivity ×
  thickness, times width over length (Patankar 1980). On 2 km cells a hot spot in rock spreads one cell in about
  70,000 years, so a yearly step is enough; the implicit step holds for any length.
- **Melting plateaus** (`ground::melting`). A ground node holds the fusion enthalpy of what is molten in it, so it
  stays at a melting point until that substance has all frozen or melted. Each substance melts at its point in that
  node, lowered by what is dissolved in it (2.5); the plateaus are passed in the order of those points.
- **Deep store** (`Class::Deep`): the weathered zone of the bed, `bed_depth` deep, with `bed_porosity` of pore space.
  Bodies and Change do not reach it.
- **Land** (`World::is_land`): a cell whose solid surface is above 0 m. The one rule for soil, founders and the tape.

### 1.2 Liquid

`flow::liquid`, every `flow_days`: the stores drain to the channels (`flow::drain::springs`), what may run off moves
down the cells in one pass (`solve::downhill`), then the liquid under the surface moves (`flow::drain::under`). The
store books every move as link fluxes.

- **Pores first.** The loose ground's pores (`porosity` × its volume) fill first; only the liquid above them stands
  and has a head (`flow::land::stand`).
- **Running off.** Cells are ordered by head, highest first (`downhill::Order`), and one pass moves the liquid: each
  cell sends to every lower neighbour (all 8) at least `split` as steep as the steepest, by slope; under `still` of
  head nothing moves. A head falls no further than to a neighbour still to come, and half way to one already passed,
  so heads never cross. Liquid can cross the map in one step while depressions fill and spill.
- **Pore water** flows by Darcy's law across the borders of cells in one implicit step of `solve::spread`, on the
  liquid table (`flow::land::table`): a link passes `seep` × the saturated thickness of its higher end × width /
  distance. Pores at or under their field capacity (`field`) give nothing.
- **The deep store** is the lower layer of the cell's stack, joined to the ground node by a vertical link: liquid
  above field capacity sinks into it (at most `seep`, never past its room), and the stores flow to their neighbours
  by Darcy's law (`bed_seep`) in the same `spread`.
- **Channels inside a cell.** Real land drains to channels a few hundred metres apart, so the loose ground above field
  capacity and the deep store also drain by Darcy's law down the cell's slope, over a hillslope of `hill`, into the
  runoff (not under deep liquid, which has no slope).
- **What goes with the liquid.** A move takes its share of every liquid and of what is dissolved in it, with its
  heat. A salt is data for a liquid that never boils (substances 23 and 24): it moves with the liquid by its share and
  stays behind when the liquid evaporates. There is no other rule for dissolved solids.
- **Lost head is heat** in the ground (or the deep store), and does work (1.3).

**Potential energy.** Lifting matter takes heat and a fall gives it back, through the potential node: liquid and
solids going downhill, gas going into the air and out (1.5), gas between air nodes. In Flow each lift and fall is a
link flux booked with the move (`flow::Falls`).

### 1.3 Erosion and creep

The head moving liquid loses does work where it is lost (`flow::liquid`, in the same pass):

- **Carrying:** `carry` m³ of loose solids per J. The load leaves a cell in the share the liquid leaves it, so it
  settles where the flow slows; what is over the capacity drops.
- **Wear:** `wear` m³ of bonded bed per J, over the bed's strength, broken into loose ground (`Move::loosen`; the bed
  is `Class::Bed`, amounts without heat).
- Bodies hold the ground: they cut carrying and creep by 1 / (1 + their mass per area / `hold`).
- **Creep** (`flow::creep`, yearly): loose ground slides downhill like a slow diffusion, `creep` × softness ×
  thickness × slope, link by link (corners too, as wide as a side).

Most rock that leaves the land is lime that Change dissolves (reaction 22, 2.6) and the liquid takes away.

### 1.4 The air's step

`flow::air::step`, every `air_hours`. Radiation, the exchange of the ground with the air (evaporation, rain, gases
dissolving) and the wind are one implicit step (backward Euler) over the whole span, so a long step gives the world
of short ones: the long-step test (`tape/tests/ranges.rs`) holds the climate of the 10-day steps to that of 6-hour air
steps and 3-day physics steps within the seeds' noise. The wind's pushes come from the air at the step's start
and the clouds from what condensed in the step before; they carry the amounts and heat at the step's end, so the
step stays implicit, and the rounds solve only the exchange with the ground.

- The unknowns are each air node's temperature and gases at the step's end (the gases it holds more of than the
  rounding of its sums, and those that can evaporate from the ground), under the sun of the step's end.
- Each Newton round (`flow::air::newton`) solves every ground node's exchange with the air it sees (its surface
  balance, rain and evaporation, and their slopes), then the whole air level with the wind in one banded solve
  (`solve::band`; on a globe each row of air nodes is numbered in a folded order, 0, w-1, 1, w-2, ..., so the link
  that closes a row and the air a ground cell sees round the wrap stay within a band of one row). A node's energy at
  the end is the heat capacity of its gases then times its temperature. The rounds stop when the air moves by less
  than 0.05 K and 1e-5 of a node's gas, or after 16.
- **Booked whole** (`air::book`): what the last round found crosses the links as it is solved, so the books are exact
  whether or not the rounds converged, and the other laws read what it leaves until the next step. A ground node gives
  no more of a gas than it holds and gets; an air node that would give more rains and blows that much less.

**Sun, heat and radiation** (`flow::air::radiation`).

- The sun warms each cell by latitude, season and day length (`insolation`, the daily mean; Berger 1978), with the
  declination of a circular orbit, sin δ = sin ε sin λ (`declination`, ε the tilt, λ the sun's longitude); the surface
  reflects part of it. The test `the_sun_is_right_at_every_latitude` holds the yearly mean at the poles and the
  equator to the analytic values, the sphere's mean to S/4 at every season, and the south's seasons half a year from
  the north's.
- Every surface emits σT⁴. The air absorbs 1 − exp(−x) of what the ground emits, x the optical depth summed over the
  gases (each past its own column amount grows only as the square root). It emits from 4 grey layers that cool with
  height (`flow::glow`): more gas lifts the layer that reaches space into colder air, so the ground warms.
- The ground trades heat with the air touching it (`sensible`); its temperature is solved implicitly over the step,
  with the latent heat of what evaporates and the heat of the rain.

**Light and heat for bodies.** Each cell has one bodies' node (`store::Class::Bodies`) that holds, as heat, what its
bodies will take at their updates. The light bodies' leaves absorb warms the ground, except the share their light
reactions store (`Body::keep`): Flow books that share from the sun into the bodies' node. Before each physics step
the node settles with its ground (`survey::survey`) to hold also the heat each body draws from the ground at the rate
of its last update (`Body::drew`) since then. So a body that updates every 20 days does not pull 20 days of heat out
of the ground at once; at its update it takes its share from the node, and only the rest is settled with the ground.

**Shade** (`survey.rs`). Light passes the bodies of a cell from the tallest member down; a layer of leaf area x per
ground area takes 1 − exp(−x) of what reaches it, and equal heights share a layer. Height (3.6) is kept with the
other visible traits (`Body::vis`, set wherever a body's stores or members change: `World::see`). Rain on bodies is
shared the same way (6.4). The survey shares out a cell's light again only when its bodies changed (`Life::stir`).

### 1.5 Evaporation, rain and dissolved gases

`flow::air::exchange` and `flow::air::step`.

- **Evaporation** by vapour pressure: the ground's saturation pressure at the step's end against the air's near it.
  Wet ground (standing liquid, or pores at field capacity or more) evaporates at the open rate, drier ground in
  proportion to its liquid (the bucket of Manabe 1969), integrated exactly over the step with the rain it gets.
- **Rain.** Vapour over what the air a cell sees can hold at its temperature (colder on higher ground) rains out in
  `rain_time`, out of the four air nodes the cell sees, each in the share it gives of that vapour (`air::shares`), so
  no node gives more than it holds; it brings the heat of the air it comes from. Condensing air makes clouds that cover
  part of the sky (`cloud`) in the next step.
- **Lift.** Gas that goes up into the air (from the ground, bodies, parts, reactions, growth) is lifted from the
  ground's head to the air's mean height, paid by the air's heat; gas that comes down gives it back.
- **Booked by the store**: a link from each ground node to its air node carries what evaporates and its heat, and a
  link to each of the four air nodes it sees what rains out of it; the air gets that heat less the latent heat it holds
  of the gas (`store::held`). Bodies and parts book their gas as moves (`store::Moves::gas`), with the same lift.
- **Gases dissolve** (`exchange::dissolve`, at the start of the air's step). A gas that cannot condense on the ground
  dissolves in the liquid there toward Henry's share and comes out when the liquid warms, thins or freezes, through the
  air's film (`evap`) and the liquid's film (`dissolve`) in series. Dissolving frees the vaporisation enthalpy into
  the liquid; coming out takes it back. Dissolved gas moves with the liquid and returns to the air only here.
- **Known limit.** The whole depth of a liquid is one mixed layer, for heat and gas: a deep sea warms over centuries
  and takes up gas in months; a real one is the other way round.

### 1.6 Wind

`flow::air::wind`, inside the air's step. The air nodes are the coarser level of the mesh (`AIR`); gas moves over
each link of that level by three pushes, each a rate:

1. the pressure of the whole column (p = nRT/V), evened in the time sound takes to cross the link (Lamb waves,
   Lindzen 1990);
2. overturning: at equal height the lower half flows to the warmer column and the upper half back, at `wind` per K,
   turned sideways by the planet's rotation (`rotation`, at the link's latitude); vapour crowds in the lower half;
3. eddies mix the two columns (`mix`, across the link's width over its length).

The overturning's turn reads the slope of warmth across the link, from each node's neighbours by the way between
centres (`Level::offset`: on a globe along the parallel and the meridian, the shorter way round the wrap). A ground cell
sees the air of the four nearest air nodes, bilinearly, round the wrap of a globe's rows.

The pushes come from the air at the step's start and carry the amounts and heat at its end, solved implicitly with
the rest of the step: any speed is stable. Air moves as whole columns, so there is no wind
near the ground: bodies use the anchored `wind_ground` (6.3).

### 1.7 Paths

`lanes.rs`. A body moving through a cell spends energy pushing through the bodies there (`choose::go`,
`choose::wear`), which breaks what it pushes through (6.8) and clears the link it used (`lanes::clear`, a field of the
links, `store/links.rs`). A cleared link is cheaper to cross (`lanes::resist`). Once a year the bodies grow back into
cleared links (`lanes::regrow`, rate `regrow`). This one rule is what can make paths.

---

## 2. Change: substances react inside nodes, faster when hot

Files: `core/src/change/` (the reactor), `core/src/chem/` (the chemistry and its physics), `worldgen/src/chem0.toml`.

### 2.1 The rate law

```
rate    = catalyst · k · exp(−Ea / T) · contact · Π over inputs (share of its phase ^ order)
lim     = the least, over the inputs, of (amount / count)      (the scarce input)
contact = what lim would be if each input were all of its phase
extent  = lim · (1 − (1 + p·z)^(−1/p)),   z = rate / lim · time   (1 − exp(−z) when p = 0)
p       = (order of the scarce input − 1) · (1 − its share)   for a gas or liquid; 0 for a solid
```

- **Share** is an input's share of its phase; a solid is a phase of its own. So a reaction inside one phase goes with
  that phase, and one between a solid and the air goes with the solid, like a surface. A trace input at order 1 loses
  the same share of itself per second whatever its amount.
- **Order.** The data may give each input its own power (`Reaction::order`); else the count is the power.
- **Extent** is the integral of the rate law over the step (`gone` in `change/kernel.rs`): never past the scarce input.
- **Heat.** A reaction unit gives −ΔH to the node (`Chemistry::dh`). A substance's formation enthalpy `hf` is that of
  its solid: a liquid holds its fusion heat more, a gas also its vaporisation heat (in the air), so each phase is
  counted once. `Chemistry::dh_at` gives the heat as measured (`ew chem` prints it at 290 K).
- **Light.** A light reaction takes absorbed light instead of heat and stores `light_yield` of it; the rest is heat.
- `change::pace` is the one formula; ground nodes, air nodes, parts and bodies all use it.

### 2.2 Where it runs

- **Ground and air nodes** (`change::step`, `Reactor::react`): each when it is due, for all the time since it last
  reacted, with sub-steps when the temperature changes fast. Then it sleeps (0, "Sleep and wake") until its reactions
  would use 5% of their inputs or warm it 0.5 K (`Reactor::rest`), at most `sleep_days`. A speed tool, not physics
  (`sleep_days` 0: every node reacts every step). A reaction too slow to matter over a year is frozen.
- **Booked together.** The nodes due in a step react from the world as it was before any of them did (a ground node
  with its share of its air node), and `store::apply` books them in one call. A node's reactions are one
  `store::Transform` (`Reactor::made`, summed exactly where terms cancel, so it keeps every element); their heat
  follows from the formation enthalpies (`store::held`).
- **A ground node reacts at Flow's last temperature.** On a plateau the fusion heat it holds is set aside while it
  reacts (`FlowView::fused`).
- **Gas a ground node takes or gives** crosses the link to its air node, one flux per substance (`store::Batch`). Gas
  taken brings its heat at the air's temperature and gives the node its latent heat; gas given takes its share of the
  node's heat and its latent heat. The air pays the lift (1.5). A node gives no more heat than it holds: where its gas
  would take more, each gas it gives is cut by the same share. Parts and bodies settle their gas the same way
  (`change/gas.rs`).
- **Parts** react every 6 months at their cell's temperature (`bond::step`); **bodies** at each update with their
  catalysts (2.4); **growth** is a conversion that only bodies run, paid by work (6.6).

### 2.3 What gas a ground node touches

Its own gas plus its cell's share of the air above. **Under standing liquid** it touches only the gas dissolved in
that liquid, which the liquid's film renews from the air (film time τ = depth / `dissolve`, `change::film`): each gas
it takes is held at Henry's share × (1 + made·τ/eq) / (1 + used·τ/eq), a film in series with the reactions. So dead
matter on the sea floor rots only as fast as the film brings the oxidising gas.

### 2.4 Change inside bodies

- **Catalysts.** A body's catalyst gene for reaction r speeds it by f = 10^gene (off below 0): metabolism. "Eating
  plants" means "my genes speed up the reaction that burns what plants store".
- **Activity.** The speed-up is 1 + (f − 1) · a, a the activity (6.5): a dry body's enzymes do not work, the bare
  reaction still runs.
- **Inhibition by the product** (`Body::inhibited`): times 1 / (1 + Σ x / x*), x the body's store of each product, x*
  the level it keeps (gases and the key liquid do not count; a product it keeps no level of stops the enzyme). It acts
  on the reactions a body runs at its catalyst's pace, not on the burnings it runs on demand.
- **Heat kills enzymes** (`change::denature`): above the `DENATURE` gene the speed-up falls off within a few K
  (`unfold`). A firmer shape costs more upkeep (`t_firm`, `c_firm`).
- **Burnings and keepers.** A catalysed reaction that releases heat and takes a gas is a **burning**: the body runs it
  on demand to pay its costs (`body::plan`, `body::pay`), as far as the rate law allows with the air it breathes. A
  reaction that gets less heat out of a fuel than burning it would (`Chemistry::keeps`) keeps the rest in what it
  makes (fixing, reaction 24): it runs at its catalyst's pace and pays no upkeep.
- **Fuel** (`Body::burns_at`): a store that is an input of one of the body's burnings and is not a gas at its
  temperature.
- Enzymes cost upkeep by their **capacity**, used or not (6.2).

### 2.5 Phases and the freezing law

A substance is solid, liquid or gas by its melting and boiling points (`Chemistry::phase`). The melting point of a
liquid in a mixture (`Chemistry::melt_in`) is lowered by what is dissolved in it, by the ideal-solution law:

```
1/T = 1/Tm − R · ln(x) / Lf        x = the liquid's share among itself and what is dissolved in it
```

- Only the **solvent** is lowered: the liquid (at 290 K) that is the most of the liquids there.
- Dissolved means: substances that melt below the solvent's point and are still liquid there (the salts 23 and 24,
  and the fat 6). Gases (only Henry's share) and solids do not count. A solute keeps its own melting point.
- The whole amount of the solvent (ice and liquid) counts, so it freezes at one point.
- The same law serves bodies (frost, evaporation, key liquids), the ground's plateaus, ice cover and surface
  (`World::thaw`), parts, and Change's own phases.

**Latent heat.** A gas in the air holds its melting and boiling heat (`Substance::latent_gas`): every gas that enters
or leaves the air pays or gets it. A body holds the fusion heat of its molten stores, as a ground node does
(`Body::thermal`): it freezes and melts at a plateau, liquid it takes or gives carries its fusion heat
(`Body::heat_with`), and liquid it evaporates takes its vaporisation heat beside.

### 2.6 Pairs at balance and the sea's carbon

`change::reversible` finds every pair of reactions where one undoes the other; `Reactor::balance` finds from the
chemistry alone the amount at which a pair goes at equal rates. In chemistry 0 that is the lime–salt pair: 22 makes
the dissolved salt 23 from lime, the carbon gas and the liquid; 23 turns it back. At the start (`worldgen::terrain`)
each wet cell holds the salt at balance with its temperature and the starting air; a dry cell holds none. So the sea
holds several times the air's carbon and gives it back when the air thins, with no rule for the sea.

### 2.7 Chemistry 0 in short

One fixed, Earth-like chemistry until M7 (handover 6.0): 9 elements, 26 substances, 28 reactions
(`worldgen/src/chem0.toml`), numbers only in `core` and `worldgen`; Earth names for people are in
`library/chem0.toml`. A substance's molar mass is the sum over its elements (`element_masses`), so every reaction keeps
mass. The roles the laws use:

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

Key reactions: 0 light to sugar; 1 burning sugar; 21 rotting or burning body matter (gives back the nitrogen mineral
and lime); 22 and 23 the lime–salt pair; 24 nitrogen fixing; 25 the nitrogen mineral breaking down when hot (the
nitrogen goes to the air); 26 rotting of the frame; 27 splitting the frame into sugar (needs an enzyme). Dead matter
rots at a litter constant (reactions 1, 21, 26: about a quarter a year at 290 K in open air).

**Growth conversions** (`Chemistry::build`, `Chemistry::builds`), run only by bodies and paid by work:
- body matter: 118 sugar + 3 nitrogen mineral + 1 rock salt → 1 matter + 27 liquid (no heat in or out);
- frame: 6 sugar → 1 frame + 1 liquid;
- lime: 1 rock salt → 1 lime + 1 carbon gas + 1 liquid (the gas goes to the air).

`Chemistry::minerals` lists the inputs of growth that no reaction burns (24 and 23), `Chemistry::materials` the
solids bodies build (25 and 10). A random chemistry has none of these yet: its bodies do not grow.

---

## 3. Bond: things can be joined, joins break when pushed too hard

Files: `core/src/bond/` (`verbs`: what bodies do to parts; `sense`: what parts give a body; `month`: the monthly
step). Also `choose::go`, `Body::height`, `Body::breaking`.

### 3.1 Parts and structures

- A **part** is a made object: solid walls, the liquid held in its hollow, heat, a size (3 numbers) and a hollowness.
  It is a node of the store (`Class::Part`) that comes and goes: its id is its slot and generation (`bond::id`). A
  fixed node (ground, air, deep, boundary, potential) is named by its class and index (`store::Node`). A part lies in a
  cell or is held by a body. A body makes one with the action "form" (`bond::form`, `c_form` per kg), at most `form`
  of its mass.
- A **bond** (`bond::Bonds`) joins two parts, or a part and the bonded ground. Its strength comes from the weaker
  material at the joint and the contact area (`tensile`, `joint`); making it costs `c_join` per m².
- A **structure** is the parts joined by bonds. It moves as one.
- Parts take their cell's temperature each month and react with the air every 6 months (2.2).

### 3.2 Loads and failure

Once a month (`load` in `bond/month.rs`) each bond carries the weight hanging on it from the anchored part, less
buoyancy, plus the push of moving liquid and of bodies moving through the cell (`bump` of their weight). A bond loaded
past its strength breaks. What has no support lies on the ground. A structure the liquid holds up floats and drifts
down the liquid's slope (`bond::afloat`).

### 3.3 Holding, carrying, digging

- Holding a part is a bond too. Its strength is the body's **force**: strength gene × living body (`Body::built`) ×
  gravity × `grip` (`bond::force`); a held part can add a lever (`bond::lever`). A body takes hold of a part only if
  its force bears the weight of the part's structure, and lets go when it no longer does (checked each month). Fuel
  adds no force; a hurt body is weaker.
- **Digging** (`bond::dig`) breaks bonded ground; its work goes with the body's force and time.

### 3.4 Containment

If a structure's walls close around a space, it has an **inside** (`bond::room`), joined to its cell by one link
(`bond::Enclosure`). Heat passes it through the walls (conductivity × face over thickness, for the share of the space
they close) and through the openings (`leak` per m²); light and rain pass only through the openings. A body inside
exchanges with the outside only through its share of that link, in series with its own surface. The inside holds
almost no heat, so it is no node: the series is solved in closed form. "Shelter" is nothing more than this low
conductance.

### 3.5 Moving needs force

A mover pushes what it passes with `bump` of its weight, so it moves only if its own force is at least `bump` × the
weight it moves (its whole mass and what it carries) × gravity (`choose::go`). The starting plants are too weak to
move; the starting eaters move while their mass is under about 6 times their living body.

### 3.6 What a body is built of holds it up and resists breaking

- **Height** (`Body::height`): a member stands as high as its materials hold it against its own weight, by
  Greenhill's self-buckling column: h⁴ = 7.837 / 4π × (Σ E·V) × V / (m·g), V the volume of its soft tissue and of each
  solid it built, E their Young's modulus (data field `modulus`), m the whole member. Height sets the order of light
  and rain (1.4).
- **Breaking** (`Body::breaking`): breaking a share f of a body takes f × Σ V × the stress each material breaks at
  (data field `rupture`). A bite pays it; pushing through breaks as much as its energy pays for (6.8).

---

## 4. Copy: a piece of the graph is copied, with small changes

Files: `core/src/copy/` (`mod`, `mix`, `grow`, `young`, `select`), `core/src/nodes.rs` (population nodes),
`core/src/body/genes.rs` (genes, likeness, mutation), `core/src/lineage.rs`. The body side of growth is in 6.6.

### 4.1 Genes

A body's genes (`body::Layout`):
- 21 **scalar genes**: `GAS`, `LIQ`, `HEAT`, `LEAF`, `STRENGTH`, `CHILD`, `COPY_MASS`, `INTERVAL`, `MUT`, `LEARN`, the
  brain sizes `HIDDEN`, `MEMORY`, `SEEN`, `SIGHT`, and `DISPERSE`, `DENATURE`, `REPAIR`, `WET`, `ALLOC`, `KEEP_T`,
  `COOL_T` (meanings in `body/genes.rs`);
- one **catalyst** gene per reaction (log10 speed-up);
- per substance a **safe minimum** (a share of the body; below it the body dies, 6.7), an **uptake** gene and a
  **keep** gene (the most it holds);
- the **born brain rows** (5.1).

**Mutation** (`body::vary`): each gene of a child is the parent's plus noise of one step × 10^`MUT`, within the
gene's range (`range` in `body/genes.rs`). Genes that are off have 20 steps of room below 0, so they wander without
hitting a wall. Born brain rows change with a chance `row_chance`, by `row_noise`; rarely (`new_lineage`) a child
changes its lineage's brain base and starts a new lineage.

**Likeness** (`body::alike`): two bodies are alike when every gene is within `alike` steps of the other's (brain and
sense sizes in whole units). It decides who mixes, who joins a node and who merges.

### 4.2 Children come from a brood

- A body is **mature** (`Body::adult`) when its living body per member is at or past its `COPY_MASS` gene.
- At each update `copy::grow` builds what it can (6.6). An immature body puts it all into its own growth; a mature
  body puts the share `ALLOC` into a **brood** (`Body::brood`: matter held for children, not yet body), the rest into
  its own growth. With `ALLOC` below 1 it grows all its life.
- Matter that leaves a body (a bite, members that die, what it sheds) or that its own reactions use (it rots, reaction
  21, or burns) takes the brood down in the same share (`Body::lose`, `Body::brood_share`): it is matter like the rest.
- Whenever the brood covers one child's matter, children are made (`copy::copy`) at the body's update, whatever its
  brain does: bodies copy by physics only.
- **A child** has a body of 10^`CHILD` × its parent's copy mass. It gets that matter from the brood, provisions of
  every other store in the proportion its parent's body holds them (the key liquid times 10^`WET`, so a body can make
  dry, dormant children), its share of the heat (as warm as its parent), and its genes plus noise. The parent pays
  nothing more: the matter was paid for when it was built.
- An individual makes whole children; a node makes fractional members of what its brood holds beyond half a child per
  member (`copy::BROOD_HELD`): each member fills its own brood and makes a child when it covers one, so their broods lie
  evenly between none and one child, and a member who dies loses what it held. A share `DISPERSE` of a node's children
  starts in a random neighbour cell (for an individual, the chance that they do); less than one member with no node of
  its kind to join waits in the brood.

### 4.3 Two parents

A mature individual making children looks in its cell for a **partner** (`copy::mix::partner`): the most alike other
mature body that is alike to it (a keyed draw among equals). A node's children have one parent: its members already
mix their genes among themselves (one variance per gene and no covariances is free recombination, Bulmer 1971).

- Each gene and each born brain row (kept whole) of the children is one parent's, drawn; then the normal mutation.
  The chance to take the partner's is one half: the partner is one parent (a node gives the genes of its mean member).
  Many children take about half from it, and their spread is what a merge of the two parents would pool
  (`mix`, `mix_rows`).
- The partner gives and loses nothing: genes are information.
- No partner: the children copy one parent. Bodies that are not alike never mix, so **kinds are the groups that mix**.
- `Config::two_parents` off (`--no-mix`) gives one parent always (a control).

### 4.4 Population nodes

A node (count > 1) is one body for many members (handover 7.1): a speed tool that must give the patterns of
individuals.

- **Spread.** A node keeps the variance of its members' genes around its mean, per gene, and one for its born rows
  (`Body::spread`, `Body::row_spread`), never past the split threshold (`body::most`); an off gene keeps none.
- **Members are kept exactly** (`nodes::Lot`, `nodes::add`: one helper for every join, merge, split and birth).
  Members, stores, heat and brood split by shares, the node keeping the remainder, and join as sums (0.4 of a member
  adds 0.4). Genes, sizes and rows become the means **by members**; the spread pools each one's spread and its gap to
  the new mean.
- **Joining.** Children join the young node of their kind in their start cell when alike within `alike` steps plus
  `WITHIN` standard deviations of both spreads (`body::within`), booked on it with no body built; else they start one.
- **Young that grow up** (`copy/young.rs`). A kind in a cell lives as an adult node and a young node joined by a link
  (`Body::link`): births flow into the young node, maturing members into the adult one. A node keeps the mean and the
  variance of its members' log10 bodies (`Body::size_lag`, `Body::size_spread`); newborns come in at one size, so
  those who joined earlier are larger. A young node's members grow alike in proportion. Below the copy mass
  their log10 bodies have a density e^(−k y), y the distance below it, over a width, fitted to their mean and variance
  (`math::Pass`): the steady state of a stage entered at one size, with growth at one pace and deaths at one rate, k the
  death rate over the pace. It is memoryless: those who stay keep k, those who leave keep the mean and variance of their
  slice. A spread no width holds is an exponential tail of the same mean. Those the growth pushed past the copy mass
  move into the adult node with the share of the body their slice holds (`copy::mature`): the flux across a size of a
  size-structured population (McKendrick 1926; von Foerster 1959).
- **Merging** (`nodes::crowd`, monthly). Alike nodes of one stage in a cell merge (the young mature into their adult
  node instead). Past `crowd` nodes a cell merges its most alike pairs while they are alike; past twice `crowd`,
  however unlike (a safeguard).
- **Splitting** (`nodes::split`). A node whose two halves would no longer be alike splits along its widest gene, if its
  cell has room; selection then acts between the halves. Splits, draws and joins keep each gene's mean exactly.
- **Selection inside a node** (`copy/select.rs`). Members are born and die in proportion whatever their genes, so a
  node without a brain also runs its update, as far as its growth, for a sample member: the node's genes but one, one
  standard deviation of its spread up or down (gene and side drawn by keyed random numbers), whose moves are dropped.
  Their gains are the log of what can become body after over before (`copy::buildable`: matter, brood, and the matter
  the stores could build), so a gene that brings more fuel gains. The gene's mean moves by the step times
  tanh(sample's gain − node's gain), times the number of genes drawn from and `sample_every`: Price's equation for
  members a step either side of the mean (Price 1970), the breeder's equation (Lande 1976). No sample runs where the
  step cannot change the gain in one update (`copy::select::read`: genes read only for children or a brain, genes off
  at both ends, thresholds the step does not reach); a wrong skip only slows that gene. A node with a brain runs no
  sample: its update is also its decisions.
- **Choices as a spread of actions** (`choose/spread.rs`). A node with a brain spreads its members over the actions
  by the chances its brain gives, those an individual draws from (5.3).
- **Members as individuals.** Members of a node doing an action of Bond act one at a time, each split off
  (`nodes::one`), and live on alone if they then hold something or are inside. `nodes::open` draws members as
  individuals for a close view.
- **Brain size** (`lineage::resize`): a node whose mean brain-size gene is a whole unit from its lineage's moves to a
  new lineage of that size.
- **The pace of nodes** (`Body::interval`). A node without a brain updates at most every `node_days`: it takes no
  decisions, and its update integrates the time since. A body with a brain keeps its `INTERVAL` gene. The rules that
  count by the gene's interval are rates over the update (`Body::paces`): stores refill a level per interval of the
  gene (`Body::room`), frost kills at the rate its share per interval gives (`Body::over_span`); so for a body without
  a brain the gene is the turnover time of its stores (a known limit). Members who join between updates count their
  time from when they came (`nodes::add`: the node's last day and mark of light are means by members).
- **Age.** Members of a node die of age at the rate of their wear, as an individual does (6.7).
- **Individuals mode** (`Config::nodes` off, `--individuals`): bodies with a brain are never nodes; bodies without one
  stay nodes, so the check of handover 7.1 compares brains in nodes and alone.

---

## 5. Choose: a small brain sees a little, acts, learns and copies others

Files: `core/src/choose/` (`mod`: the decision; `perceive`, `learn`, `feed`: taking and giving, `go`: moving),
`core/src/brain.rs` (the network and the named actions).

### 5.1 The brain

- One hidden layer over what the body perceives and remembers. Its **base** belongs to a **lineage** and changes only
  by evolution; its **rows** (the readers of the hidden layer) are genes of each body.
- The rows give fixed action scores, shared scorers (one for every neighbour direction, one per verb for every body or
  part perceived), a signal, the shape of a part to make, the next memory, and a critic's estimate of well-being.
- Sizes are genes, each with a cost: hidden units (0 to 64; 0 is no brain), memory numbers (0 to 12), bodies
  perceived (1 to 6), sight (1 to 4 cells). A body with no hidden units takes no decisions, pays nothing for a brain
  and has no rows: its update is physics only. The light-eaters start so (8); a lineage may grow a brain back through
  the same gene if it pays, its rows starting as new weights (`brain::rerow`).
- **One brain per body or node** (`Body::rows`): the rows it was born with plus what it learned; a node's members
  share the node's. A child starts with its born rows plus what its parent learned (`Life::inherit`). When members join
  a node its rows become the means by members (4.4); where brains differ in size, the members bring only their born
  rows (`Life::carry_rows`).

### 5.2 Perception

Limited and local (`choose/perceive.rs`): its own stores against their safe minimums, warmth, size and light; its
cell's loose substances, liquid, temperature and crowding; the season; signals heard; its memory; the cells in 8
directions as far as its sight (nearer cells count more); up to `SEEN` bodies in or next to its cell (looks, signals,
well-being, how past dealings went); up to 3 parts in its cell. Walls and cover hide bodies. Each input is scaled to
about one by a fixed table.

### 5.3 Actions

One action is drawn by the scores at each decision (once per update, `choose::act`); with none open the body rests.
Each costs energy its own reactions must pay (`body::pay`), or it fails. A node spreads its members over the actions
by those chances instead (`choose::spread`): count × chance members do each, a part of less than one member being one
member with that chance. A part acts as so many members would, split off by shares and back in (4.4); a part that
moves joins the first node of its kind and stage in the cell it reaches, or lives on there; members doing an action of
Bond act one at a time.

**rest**; **move** to a neighbour (`choose::go`: `Physics::walk` per kg and metre, the real cost of transport, plus
climbing and the bodies in the way; it needs force, 3.5); **take** from the ground or another body, **put** to the
ground, **give** to another body (`choose/feed.rs`); **watch** another body (5.5); and Bond's actions: form, hold,
take from or put to a part, join, break bonds, let go, anchor, dig, enter an inside (`bond/verbs.rs`). There is no copy
action (children come from the brood), and no "farm", "trade" or "attack": those must be patterns of these.

**Taking** goes by uptake genes (`Body::wanted`, `body/intake.rs`), up to what the body can take in the time (6.7),
from a cell only what the area it works holds (`sweep`), at `c_handle` per kg. From another body it is also limited by
its force against the other's. Each member of the taker meets one member of the other, as many as both have, and
reaches what they hold (a node is found as one body, 5.2, and taken as its members are); a quicker body (shorter
interval) gets away with the chance its interval gives (of a node's bites, that share). A bite tears the body of each
member it reaches (6.8).

**Giving** is of stores only: never body matter, brood or built solids, nor a key liquid below its level, and only
from stores above twice their safe minimum; at most what the members it meets could take in the time (each member
meets one, as in taking).

### 5.4 Learning

- **Well-being** (`Body::wellbeing_at`): the energy its reactions could release from its stores, in decisions' worth,
  less how near it is to its limits (stores to their minimums, release to upkeep, key liquids to freezing or boiling).
  No reward is hand-shaped.
- **Within a life** (`choose::learn`, `brain::learn`): at its next decision the brain learns from the change of its own
  well-being, by a policy gradient against the critic (actor-critic), into its own rows, at the rate of its `LEARN`
  gene. A node learns as from one member drawn among those who stayed: from what it did, with the change of the
  node's well-being plus how much more its part's action raised its members' well-being than the mean of the members
  (`choose::spread`); so its brain moves, decision by decision, as an individual's does.
- **Across lives**: evolution of the born rows and the base (4.1).

### 5.5 Imitation, signals, memory

- **Watching** (`choose::learn::watch`) costs the turn: the watcher's rows move part of the way (`blend`) toward the
  watched body's, if the brains have the same sizes, less the less alike the bodies are.
- **Signals**: each decision emits 2 numbers with noise (`signal_noise`), with no meaning at first, at an upkeep
  (`c_signal`).
- **Memory of individuals**: a body remembers up to 4 others and how dealings with them went (`Body::note`,
  `Body::recall`).
- **Cost of thinking** (`Physics::think`): each decision costs by the brain's sizes and weakly by the member's mass
  (mass^0.25), so a long interval saves brain cost.
- **Null run** (`Config::null`, `--null`): no learning, no imitation, brain rows frozen; genes still evolve. The run to
  compare a real run with.

---

## 6. Bodies

Files: `core/src/body/` and `copy::grow`. `body::live` runs one update in stages: upkeep, the physics of the time since
the last update (`body/physics.rs`), keeping warm, the checks of the stores, growth and children, shedding. The
decision (`choose::act`) then sees the body as these stages leave it.

**Books.** What a body takes from and gives to its cell's ground, air and bodies' node (liquids, minerals, food,
breath, the gas of growth, heat, remains) are moves gathered over its update and booked by one `store::apply` at its
end (`store::Moves`). The body changes its own stores and heat; the store books the other side. While it updates it
reads its cell's nodes as its moves leave them (`World::g`, `a`, `heat_now`). What it breathes, soaks up, takes by its
roots or eats is set by the law's own rate over the update's span (6.4), so nodes and individuals take alike, and is
never more than is there. A sample member's update (4.4) drops its moves. A part's monthly turn books the same way.

### 6.1 What a body is

- **Stores**: an amount of every substance (mol, all members together), and heat. Its temperature comes from its heat
  and heat capacity, with the fusion heat of its molten stores (`Body::thermal`).
- **Body matter** (substance 22) is what it is built of. Its **living body** (`Body::built`) is its matter less the
  brood, over 1 − `WET_TISSUE` (living tissue is three quarters liquid). What a body takes, builds, pays to keep,
  wears, pushes with and catches light with goes by its living body.
- **Fuel** and liquid beyond the body are carried, not body. Storing fuel costs no work; building matter does (6.6).
- `Body::size` is the body with the liquid it holds (for height, volume and the safe minimums); `Body::mass` is
  everything (for moving, weight and load).
- The **key liquids** are the liquids its keep gene keeps (`Body::key_liquids`).
- **Levels** (`Body::level`): the most it holds of each substance. The key liquid: h / (1 − h) of its matter, h its
  keep gene; a solid it builds: what its share makes of its matter; any other store: keep gene × kg of living body. A
  full store is the satiety: there is no other limit on eating.
- **Supply** (`Body::supply`): what it can take and build per day, its living body to the power `intake_exp` from
  `m_ref`, times the pace of its temperature (6.2).

### 6.2 Upkeep and paying

`body::upkeep` (`body/pay.rs`): the cost of living over the interval.

```
tissue = living body × (copy mass / m_ref)^(intake_exp − 1)
upkeep = [ days × ( tissue × (c_basal + c_cat·capacity + c_strength·STRENGTH + learning + signal)
                    + c_leaf × leaf area )
           + one decision ]
         × (1 − c_repair + c_repair·2^REPAIR) × activity × pace(T)
```

- **By the kg of living body**, while supply goes with mass^0.75 (West, Brown and Enquist): a body grows until keeping
  what it has takes all it can supply, so its own costs set its size, not a cap. Across kinds the cost per kg is also
  times (copy mass / 1 kg)^−0.25, so adults pay Kleiber's mass^0.75.
- **Catalysts cost by capacity**: the sum over the catalyst genes that are on of 10^gene × k × exp(−Ea / `t_cat`), its
  gases at their share of the starting air to their order.
- **Repair**: `c_repair` of upkeep at `REPAIR` 0, doubling with each unit of the gene.
- **Pace** (`Body::pace`): exp(−Ea (1/T − 1/`t_cat`)), Ea the barrier of its main burning. Supply uses the same factor,
  so the whole tempo of a body follows its temperature; seasons come from this, not from a calendar rule.
- **Paying** (`body::pay`, `body::plan`): the body runs its burnings one after the other, each as far as the rate law
  allows with the air it breathes. The share of its upkeep over the update left unpaid dies of `SPENT` (a node loses
  that share of its members, an individual dies with that chance). The products take their heat at the body's
  temperature.

### 6.3 Heat

- **One heat network** (`body/exchange.rs`). Heat goes from the core through the coat (the `HEAT` gene, with
  mass^(2/3)) to the outer surface (skin and both faces of the leaf), then in parallel to the still air and by
  radiation to the ground. The still air passes forced convection: `forced` × √(`wind_ground` / d) × the air's heat
  capacity, d the body's length or `leaf_size`. Cover of other bodies (`shelter`, `cover_of`) and walls (3.4) are in
  series.
- **Work heat stays** (`body::spend`, `body::hold`): the heat of upkeep and work leaves through that network over the
  interval; the share (1 − e^−x)/x stays, x = conductance × time / heat capacity, the rest goes to the ground.
- **Reactions' heat** (`body/physics.rs`): a body reacts at its temperature, held by the ground it stands on (Flow's
  last ground temperature) as a store of the heat that ground can conduct into it over the update.
- **KEEP_T**: if the update would leave the body colder than this gene, it burns on demand the heat that brings it
  there, as far as its reactions can.
- **COOL_T**: if warmer than this gene, it evaporates the latent heat that brings it there, no faster than its still
  air carries vapour away (the Lewis relation), never below its liquid's safe minimum. Between the two it does neither.

### 6.4 Water: breath, rain, soaking, roots

- **Breath** (`breath` in `body/exchange.rs`): the `GAS` gene is the conductance of the body's pores (stomata), per m²
  of open surface (`Body::open_area`) per day, in series with the still air; liquid over the body slows it
  (`gas_depth`). It reaches no more of a gas than its air node holds.
- **Pores close as it dries**: for vapour only they open as far as the activity (6.5), solved exactly over the update
  (`closing`).
- **Rain**: the bodies of a cell catch 1 − e^−x of the rain that fell on the cell since their last update (x their
  leaf area per ground area; `World::rained` and the mark `Body::rained`, as for light), shared by height like light.
- **Soaking** (`LIQ` gene): a flow through the body's contact with the ground, at `LIQ` × the ground's wetness (half
  wet at `wet_depth`) × its working share × solubility, driven by how far it is below its level, while it loses what it
  evaporates; solved exactly over the update. A body afloat reaches no ground. After growth and children it soaks again
  in the same update (`soak_again`), so its activity does not lag one update.
- **What flows through** (`through`): a soaking body loses where its soaking meets what its pores pass; what passes
  beyond its store comes from the ground as it goes. This is transpiration.
- **Roots for minerals**: the uptake genes of the minerals take them from the loose ground at the rate it soaks
  liquids, up to its level.

### 6.5 Activity and dormancy

`Body::activity`: a = min(1, l / l*)^n, l the key liquid it holds, l* its level, n = `dormancy`. It multiplies the
catalysts' speed-up (2.4), the whole upkeep (6.2) and the building allowance (6.6). A dry body pays almost nothing,
ages hardly at all and can hardly act.

- **Frost**: a key liquid that freezes (at its lowered point, 2.5) kills its frozen share times the activity (a node
  loses that share of its members, as a rate over a long update, 4.4; an individual dies with that chance; `PHASE`).
- **Boiling**: a key liquid that boils kills the body.

### 6.6 Growth

`copy::grow`. A body turns sugar and the minerals into body matter and pays `c_copy` of work per kg, through
`body::pay` (the work leaves as heat). It builds as far as all of these allow:

1. its **allowance**: `c_build` × its supply / `m_ref` × the days of its interval × its activity;
2. its **spare**: after the work, its reactions must still release its upkeep over an interval at today's rate;
3. what its reactions can release now;
4. its stores, which must also cover the fuel the work burns (with none of a mineral it builds no matter).

The solids it builds besides matter (frame, lime) come first, each with its share (`Body::share`) of the allowance and
the spare, up to the level its share makes of its matter. They pay no upkeep and are not living body. **Healing is
growth**: lost matter is rebuilt the same way.

### 6.7 Taking, digesting, minimums, ageing

- **Intake limits**: per day at most `take` × supply; from a cell only what the area it works holds.
- **Digestion** (`Body::digest`): body matter taken from outside is broken back into what it was built of as it comes
  in, the reverse of growth, so food enters as fuel, never as body. A body takes body matter or a built solid only if a
  catalyst of its own breaks it (`Body::can_take`), and only as much as the stores it becomes have room for.
- **Safe minimums** (`Body::lacking`): a store below its minimum gene kills the body (`LACK`), except a store it burns,
  where the minimum is only the reserve that growth and giving keep; short of fuel it dies of what it cannot pay (6.2).
- **Shedding** (`shed` in `body/live.rs`): at its update a body gives the ground the minerals and built solids past
  their level, the key liquid past its level, and any other solid or liquid it neither burns nor is made of.
- **Ageing**: its wear grows by the energy its upkeep turns over per kg of living body, over `e_life` × 10^`REPAIR` (the
  rate of living). The share 1 − e^−wear of the wear over the time since dies of `AGE` at each update, so as many in a
  year whatever the length of the updates.
- **One rule of death** (`body::live::lose_share`): what kills a share of a node's members (unpaid upkeep, wear, frost,
  starving) kills an individual with that chance: a node is the average of its members.

### 6.8 Damage

`body/death.rs` (`thin`, `wound`, `die`).

- **A bite** (`take_body`) tears, of each member it reaches, the share of its living body the taker takes (or of its
  matter eaten, if more) and pays the energy to break it (3.6); body matter breaks first and spills the stores it held
  to the ground.
- **Pushing through** (`choose::wear`) breaks the share of each body there that the energy spent on it pays for, each
  of its members alike.
- **Heat** burns body matter by its own reaction (21).
- A member lives on smaller and weaker and dies when half its body or more is torn at once (an individual also when a
  store falls below its minimum). A hurt adult is immature until it heals: no children meanwhile.
- **Starving** (`starved` in `body/live.rs`): a body whose growth did not rebuild what it lost in an update (what rots,
  what it burns), below its copy mass, dies with the chance of the share of its body it lost (of its body at the start,
  or of its copy mass if less): one that starved to half its size lived through it with even chances. A node loses that
  share of its members with their share of everything, so it does not shrink alike forever. An adult larger than its
  copy mass shrinks towards it as it chooses (`ALLOC`): no starving. Cause `SPENT`.
- **In a node** the members a bite reaches lose what it tears, so the node's mean body falls (grazing), or die with all
  they held (`body::wound`).
- **Death causes**: `LACK`, `PHASE`, `SPENT`, `TAKEN`, `BROKEN` (by bodies moving through), `AGE`. At death matter,
  stores and heat go to the ground (`body::die`).

### 6.9 Materials and minerals in bodies

- Built solids count in the mass and set height, but are not living body; children get them as provisions, gifts
  never give them. Hard parts protect mostly by not being digested.
- Plants get the minerals of growth by roots (6.4), animals by digesting what they eat (6.7). The rock salt falls out
  of a body as lime by reaction 23. Fixing (reaction 24) lets a body with the gene make the nitrogen mineral from the
  air, slowed as its store nears its level (2.4).

---

## 7. Soil and minerals

The two minerals (`Chemistry::minerals`) live in the liquid of the ground.

- **Nitrogen mineral (24).** Placed when life starts (`worldgen::life::soil`) on every land cell not under half a
  metre of standing liquid. After that its only input is fixing by bodies; rotting (21) gives it back; heat breaks it
  down (25) and sends the nitrogen to the air.
- **Rock salt (23).** Made in wet cells from lime, the carbon gas and the liquid (22), turned back by 23; the pores of
  wet land hold it near its balance (2.6). Rotting gives the rock element back as lime, which becomes the salt again.
- Both move with the liquid (1.2): rain washes them from slopes into low ground and the sea, and they stay where liquid
  evaporates. So fertility differs from place to place, and plants that fix nitrogen and shed it enrich the ground.

---

## 8. Starting conditions (not laws)

What is placed at the start is an initial condition; every run prints a seeding line for each kind. `worldgen` places
it (`worldgen::schedule::Schedule`, carried by a snapshot, so a resumed run places what is still due); the laws never
read a kind.

- **Terrain** (`worldgen::terrain`): an island with mountains, or a globe (`new_globe`) whose noise is sampled at each
  cell's centre on a sphere (no seam where the rows wrap) and whose level leaves 29% of the area above 0 m (the land
  share of the one planet known; `LAND_SHARE`). Both have a metre of loose ground, the sea's salt at balance with
  the starting air (2.6), which thins with height by its scale height. Every land cell drains to the sea from the
  start (`drain`): a priority flood from the cells below 0 m raises each closed basin above 0 m to its outlet,
  with a slope of 1 m per km across the filled floor (`DRAIN_SLOPE`; Planchon and Darboux 2001, Barnes et al. 2014).
- **Life** starts at year 2 (`--life-year`): light-eaters first, eaters 20 years later (`worldgen::life::KINDS`);
  `--roster` places ten kinds (`worldgen::life::ROSTER`) of several sizes and diets, each at its own year. Founders go
  on land under no more than half a metre of standing liquid.
- **Founders are adults**: a founder's living body is its copy-mass gene; its mass is 30% sugar (60% for the two big
  slow kinds), the rest liquid and matter.
- **Brains**: animals start with hand-set brain biases (`Seeding::hand`; `--neutral-eaters` for neutral brains),
  because neutral founders die before they learn. Light-eaters start with no brain. Genes and rows get a spray of
  mutation (`--spray`).
- **Fixers**: a tenth of the starting light-eaters carry the fixing gene (`worldgen::life::FIXERS`, `FIX_GENE`;
  `--fixers`).
- **Founder genes**: some are measured from the laws (`LIGHT_X`, `LITTER_X`, a real leaf's `GAS`); `HEAT`, `KEEP_T`,
  `COOL_T`, `CHILD` and `ALLOC` are the same for every kind.
