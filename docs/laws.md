# The laws as they are now

What each law does, its rule, its physical reason and where it lives. How the code does it (rounds, caches, buffers,
orderings) is said once, in the code's doc comments; the code is the truth. Numbers, units and anchors are in
`docs/numbers.md`, what a run records in `docs/tape.md`, when and why a rule came in `docs/history.md`.

**Where the code is** (`core/src/`): time in `clock/` and `schedule.rs`; the shape of the world in `space/`; shared
maths in `solve/` (`spread`, `downhill`, `band`); nodes and books in `store/`, `world.rs`, `check.rs`; ground heat in
`ground.rs`; settings in `params.rs`, constants in `units.rs`; bodies in `body/`, `life.rs`, `nodes.rs`, `lineage.rs`,
`brain.rs`; the cells as bodies see them in `survey.rs`; paths in `lanes.rs`; keyed random numbers in `rng.rs`; the
tape's events in `event.rs`; threads in `par.rs` (the same results with and without).

**Words.** A **cell** is a square of the map (2 km on the island, latitude-longitude on a globe) or a quarter of one
in a refined region. A **node** is anything that holds stuff: a cell's ground, an air column, a body, a part. A
**body** is a living node with genes and maybe a brain; a **member** is one living thing in it (one member: an
**individual**; many: a **population node**). A **gene** is a number a body inherits, often log10 (1 means ×10) and
"off" below 0. The **key liquid** is the liquid a body's chemistry runs in (substance 0). **Light-eaters** are bodies
whose light-reaction gene is high (plant-like); the others are **eaters**.

## 0. The world, one step, the books
**The map** (`space/`, `World::mesh`): cells, links with widths and headings, and a coarser air level. The laws step
only on links. Each link takes one of 8 compass directions (`Level::way`), so the brain's 8 directions hold everywhere.
An island of 2 km squares, or a globe (`--globe`) of latitude-longitude squares on a sphere of 6,371 km whose rows wrap
east to west. A rectangle may be refined at the start (`--region`, `Square::refine`): each square becomes four
quarters, linked to a square beside them at the distance between centres across the border, so a steady change
across it passes exactly.

**One step** (`schedule::step`, a month; time in whole seconds, the calendar from it). Processes run by their periods
(`clock::Every`) in the order of `schedule::processes`: every `phys_days` the survey (`survey::survey`), the parts of
Flow that are due (air every `air_hours`, liquid and erosion every `flow_days`, heat sideways and creep yearly) and
Change; on the month's first day Bond; every `batch_days` days (2) the bodies whose interval ends (`survey::batch`).
**Intervals are whole batches**: a body's interval is its `INTERVAL` gene (5 to 360 days; a population node without a
brain at least `node_days`, a multiple of `batch_days`) rounded to the nearest whole number of batches, at least one
(`Body::interval`), so an update always starts a batch, all the updates of a batch run at its first day, and a body
placed during a batch first updates in a later one. Each run stands for the time to its next.
**Bodies and their order** (`life`): the cells are cut into domains, the cells under one air node; each domain stores
the bodies of its cells, and a body that moves into a cell of another domain moves to its storage. A body's id is the
domain it was placed in and that domain's count of bodies placed (children, split halves, parts of nodes acting apart,
members drawn); it never changes, and its random draws are keyed by it. The bodies
due in a batch update domain by domain, cell by cell, and in a cell by id; sums over bodies go domain by domain, cell by
cell, each cell's bodies in the order they came. No order depends on where a body is stored.
**What a body sees in a batch** (`survey::batch`). At a batch's start each living body leaves a face (`survey::Faces`:
its id, members, visible traits, signal, well-being, and the room it is in), cell by cell. A body perceives the others
only by these faces, as they were then, whatever they did in the batch since; a body placed in the batch is seen from
the next. It acts on a body it saw only if that body still lives and, to take from it or give to it, is still in its
cell; the partner of its children and the cover over its cell are the bodies there now. It reads the store through its
domain's view (`store::Moves`): the ground, bed and bodies' node of each cell of its domain and its air node, as the
moves of the bodies before it in the batch left them; a node of another domain as it was at the batch's start. The
batch's moves are booked when it ends, by one `apply` per domain in domain order. A sample member's update
(`copy::sample`) runs in a checkpoint of the view and is rolled back.

**Sleep and wake** (`clock::Sleep`, a speed tool): a calm node sleeps until an alarm or until an amount grows by more
than `wake_share`, then its law brings it up to date over the whole time since.
**One ground temperature**: every law and the tape read that of Flow's last step (`World::t_ground_last`).
**Checks before a run**: `Config::validate`, `Chemistry::validate` (every reaction keeps its elements and mass).

**The books.** Every move takes from one node exactly what it gives another, with its heat; `store::apply` is the
store's only writer. A move out of the bed, which holds far more than its loose ground, takes only what the bed can
lose exactly (`Move::loosen`), so no amount below the bed's rounding reaches the ground for free. A ground node under
deep standing liquid holds far more of it, and an air node far more of the main gases, than a body moves: what rounding takes from their
amounts is kept apart and counted in the books (`Store::carry`, Neumaier 1974). Boundary nodes stand for the sun, space and outside (`store::Boundary`, edits by `World::cross`);
potential energy is one more node (`Class::Potential`): a lift moves heat into it, a fall takes it back. So every
element and the energy are kept; `check::drift` checks it yearly and must stay near rounding.
**No overdraw.** A law never asks a node for more than it holds; where it could, it cuts that node's own outgoing
links (`solve::spread::within`), never a whole field. A body takes no more than its domain's view holds, what the
bodies before it in the batch left (checked in debug builds). `apply` only adds and sets rounding below zero to zero, counted
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
sideways yearly by one implicit `solve::spread`, a link passing the harmonic mean of conductivity × thickness, times
width over length (Patankar 1980; `flow::land::conduct`): rock spreads a hot spot one cell in about 70,000 years.
**Melting plateaus** (`ground::melting`): a node holds the fusion enthalpy of what is molten in it, so it stays at a
melting point (lowered by solutes, 2.5) until that substance has all frozen or melted, in a long step as in short ones.

### 1.2 Liquid
`flow::liquid`, every `flow_days`.
- **Pores first**: the loose ground's pores (`porosity`) fill first, under any soft cover; only liquid above them stands
  and has a head (`flow::land::stand`), the depth bodies and Change read.
- **Spill levels**: a priority flood over the ground gives each cell its spill level (Barnes, Lehman and Mulla 2014;
  `downhill::Order`). Liquid stands below it or in the lowest basin; elsewhere it passes on, counting as no higher than
  its ground, so it crosses the map in one step while depressions fill and spill.
- **Running off** (`solve::downhill`): each cell sends its liquid down its steepest way by D-infinity (Tarboton 1997;
  `Level::steepest`): two neighbours next in compass order make a triangle with the centre, the plane through their
  drops gives its slope and way, and the steepest triangle shares the liquid between its two neighbours by angle.
- **Pore water** flows by Darcy's law (one implicit `spread` on the liquid table): `seep` × saturated thickness × width
  / distance; pores at or under field capacity (`field`) give nothing. Liquid above it sinks to the deep store, which
  flows by `bed_seep`. Real land drains to channels a few hundred metres apart, so both also drain by Darcy's law down
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
`flow::air::step`, every `air_hours`: radiation, the ground's exchange with the air and the wind in one implicit step
(backward Euler by Newton's method, `newton`, with a banded solve of the air level, `solve::band`), booked whole as
solved (`air::book`). A long step gives the world of short ones: the long-step test (`tape/tests/ranges.rs`) holds each
seed's climate at 10-day steps to its own at 6-hour steps. Wind pushes and clouds come from the step's start.
- **Sun** (`radiation`): the daily mean by latitude, season and day length (Berger 1978), declination sin δ = sin ε sin
  λ. The surface reflects as its cover is at the step's end (`land::reflectance`), so new snow reflects at once.
- **Heat radiation**: surfaces emit σT⁴; the air absorbs 1 − exp(−x), x the optical depth of the gases (each as the
  square root past its own column amount) and clouds, and emits from 4 grey layers that cool with height, so more gas
  lifts the layer that reaches space into colder air and the ground warms.
- **Clouds** (`exchange::cloud`): scattering depth x = condensation rate / `cloud`; they reflect x / (1 + x) of the
  sunlight (two-stream, Lacis and Hansen 1974) and absorb heat radiation at depth `cloud_ir` × x (Stephens 1978). A
  globe of Earth's size without life holds about Earth's climate. **Known limit:** their depth is spread like the air's
  mass, not set where they condense. The ground also trades sensible heat with the air (`sensible`).
- **The bodies' node** (`Class::Bodies`, one per cell): light bodies absorb warms the ground, except the share their
  light reactions store, booked from the sun into this node. Each body's draw of ground heat between its updates is
  part of the ground's implicit balance (`survey::Draw`): its last draw, falling to nothing as the ground nears where
  its evaporation stops (the dew point, or `COOL_T`), so a cooling ground gives less and bodies cannot pull it below
  that. A body takes at its update no more than it is owed; what none claims goes back (`survey::Draws::settle`).
- **Gas from bodies** goes into the bodies' node with its latent heat; the air's step takes it up with its lift.
- **Shade** (`survey.rs`): light passes a cell's bodies from the tallest down; a layer of leaf area x per ground area
  takes 1 − exp(−x), equal heights sharing a layer. Rain on bodies is shared the same way.

### 1.5 Evaporation, rain and dissolved gases
`flow::air::exchange`. **Evaporation** by vapour pressure (the ground's saturation pressure against the air's): wet
ground at the open rate, drier ground in proportion to its liquid (Manabe 1969's bucket), taking the heat it held at
the node's temperature; a node over a substance's boiling point boils of it what its heat over that point pays,
cooling to it (`Scratch::boil_off`; gas coming out of the liquid too). **Rain**: vapour beyond what the cell's air holds at its temperature (colder higher up) rains out
in `rain_time`, from the four air nodes the cell sees by their shares; where the air at the ground is below the gas's
melting point it falls as a solid (snow) and the air keeps the fusion enthalpy its freezing frees; where an air node
holds less than it would give, its rain is cut, and the ground evaporates of the rain only what it gets
(`book::hold_air`). **Lift**: gas going up is lifted from the ground's head to the air's mean height, paid by the
air's heat. **Dissolving** (`exchange::dissolve`): a gas that cannot condense dissolves in the ground's liquid toward
Henry's share, through the air's film (`evap`) and the liquid's (`dissolve`) in series, freeing its vaporisation
enthalpy, and comes out when the liquid warms, thins or freezes. **Known limit:** a liquid's whole depth is one mixed
layer, so a deep sea takes up gas in months and warms over centuries; a real one is the other way round.

### 1.6 Wind
`flow::air::wind`. Gas moves over each link of the air level by three pushes: the column's pressure (p = nRT/V),
evened in the time sound takes to cross the link (Lamb waves, Lindzen 1990); overturning, the lower half flowing to the
warmer column and the upper half back at `wind` per K, turned by the planet's rotation (`rotation`), vapour crowding
low; and eddies mixing the columns (`mix`, width over length). A cell sees its air node and three neighbours
bilinearly (`flow/scratch.rs`). Air moves as whole columns, so bodies use the anchored `wind_ground` (6.3).

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
node is moist, by the body's rule 1 + (`open` − 1) × moisture (`Body::catalyst`). Moisture is the node's liquid over
what its pores hold at field capacity, at most 1 (`FlowView::moist`, the bucket of evaporation); a part has its cell
ground's; an air node has none. `open` is 1 for most reactions; 200 for reaction 1 (the free sugar's rot, about a
week); 5 for 21 (body matter rots 3.8 times faster than the frame); 10^6 for 26 (the frame rots by the decomposers
only: bare, a millionth of that). No decomposer works in a body: it runs its reactions by its own catalysts.
`change::pace` is the one formula for all nodes.

### 2.2 Where it runs
**Ground and air nodes** (`change::step`, `Reactor::react`) react when due over all the time since, at Flow's last
temperature (a plateau's fusion heat set aside), then sleep until their reactions would use 5% of an input or warm
them 0.5 K, at most `sleep_days`. A node's reactions are one `store::Transform`. Gas a ground node takes or gives
(what its reactions made less what the node itself gained, so it never gives more than it holds) crosses to its air node with its heat and latent heat (parts and bodies the same, `change/gas.rs`). **Parts** react
every 6 months (`bond::step`), **bodies** at each update with their catalysts (2.4); **growth** is a conversion only
bodies run, paid by work (6.6).

### 2.3 What gas a ground node touches
Its own gas and its cell's share of the air. **Under standing liquid** only the gas dissolved there, renewed through
the liquid's film in τ = depth / `dissolve` (`FlowView::standing`): a gas it takes is held at Henry's share × (1 +
made·τ/eq) / (1 + used·τ/eq). So dead matter on the sea floor rots as fast as the film brings the oxidising gas.

### 2.4 Change inside bodies
- **Catalysts**: a gene speeds its reaction by f = 10^gene (off below 0). "Eating plants" means "my genes speed up the
  reaction that burns what plants store". The speed-up is 1 + (f − 1) · a, a the activity (6.5).
- **Product inhibition** (`Body::inhibited`): × max(0, 1 − Σ x / x*), x each product's store, x* its level (not gases or
  the key liquid), on reactions run at the catalyst's pace, not on burnings run on demand.
- **Heat kills enzymes** (`change::denature`): above `DENATURE` the speed-up falls within a few K (`unfold`); a firmer
  shape costs upkeep (`t_firm`, `c_firm`). One rule (`Body::rate_at`) for release and supply; upkeep never falls with
  heat (6.2).
- **Burnings** (catalysed, release heat, take a gas) run on demand to pay costs (`body::plan`, `body::pay`). A reaction
  that keeps part of a fuel's energy in its product (`Chemistry::keeps`, fixing) runs at its catalyst's pace. **Fuel**
  (`Body::burns_at`) is a non-gas store a burning takes. Enzymes cost upkeep by their capacity, used or not (6.2).

### 2.5 Phases and the freezing law
Solid, liquid or gas by melting and boiling points (`Chemistry::phase`). A liquid's melting point in a mixture falls by
the ideal-solution law (`Chemistry::melt_in`): 1/T = 1/Tm − R · ln(x) / Lf, x the solvent's share among itself and
its solutes. Only the solvent (the most abundant liquid) is lowered; solutes are liquids melting below it (the salts 23
and 24, the fat 6); ice and liquid count together, so it freezes at one point. One law for bodies, ground plateaus,
ice cover (`World::thaw`), parts and Change. **Latent heat**: a gas in the air holds its melting and boiling heat
(`Substance::latent_gas`), paid or got as it enters or leaves; a body holds the fusion heat of its molten stores
(`Body::thermal`), and liquid it takes or gives carries it (`Body::heat_with`).

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
**structure** is the parts joined by bonds and moves as one. Parts take their cell's temperature monthly.

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
(Archimedes): it breathes and catches light at the surface, reaches no ground, and crosses at `afloat` of walking's cost.

### 3.4 Containment
Walls that close a space make an **inside** (`bond::room`) joined to its cell by one link (`bond::Enclosure`): heat
passes the walls (conductivity × face / thickness) and the openings (`leak` per m²), light and rain only the openings.
A body inside exchanges through that link in series with its own surface. "Shelter" is only this low conductance.

### 3.5 Moving needs force
A mover pushes what it passes with `bump` of its weight, so it moves only if its force is at least `bump` × its whole
mass and load × gravity (`choose::go`). The starting plants are too weak to move.

### 3.6 What a body is built of holds it up and resists breaking
**Height** (`Body::height`), by Greenhill's self-buckling column: h⁴ = 7.837 / 4π × (Σ E·V) × V / (m·g), V the volume
of soft tissue and each built solid, E their Young's modulus (`modulus`); it orders light and rain (1.4). **Breaking**
(`Body::breaking`) a share f takes f × Σ V × each material's breaking stress (`rupture`): a bite pays it, pushing
through breaks what its energy pays for (6.8).

## 4. Copy: a piece of the graph is copied, with small changes
Files: `core/src/copy/` (`mix`, `grow`, `young`, `select`), `nodes.rs`, `body/genes.rs`, `lineage.rs`.

### 4.1 Genes
21 scalar genes (`body::Layout`; `GAS`, `LIQ`, `HEAT`, `LEAF`, `STRENGTH`, `CHILD`, `COPY_MASS`, `INTERVAL`, `MUT`,
`LEARN`, the brain sizes `HIDDEN`, `MEMORY`, `SEEN`, `SIGHT`, and `DISPERSE`, `DENATURE`, `REPAIR`, `WET`, `ALLOC`,
`KEEP_T`, `COOL_T`; meanings in `body/genes.rs`), a catalyst per reaction, per substance a safe minimum, an uptake and
a keep gene, and the born brain rows. **Mutation** (`body::vary`): parent's gene plus noise of one step × 10^`MUT`,
within range (not the brain-size genes of a body with no brain, 4.4); born rows change with `row_chance` by
`row_noise`; rarely (`new_lineage`) a new lineage with a changed brain base. **Likeness** (`body::alike`): every
gene within `alike` steps; it decides who mixes, joins and merges.

### 4.2 Children come from a brood
A body is **mature** (`Body::adult`) when its living body per member reaches `COPY_MASS`; then growth (6.6) puts the
share `ALLOC` into a **brood** (`Body::brood`), matter held for children that leaves with the body's matter in the same
share (`Body::lose`). Whenever the brood covers a child, children are made (`copy::copy`) whatever the brain does:
bodies copy by physics only. A child has a body of 10^`CHILD` × the copy mass, provisions of the other stores in its
parent's proportions (the key liquid × 10^`WET`, so children may be dry and dormant), its share of heat, and the genes
plus noise. A node keeps half a child of brood per member (`copy::BROOD_HELD`), so its members' broods lie evenly
between none and one child, as individuals' do. A share `DISPERSE` of the children starts in a neighbour cell in a
random direction (for an individual, that chance). Less than one member with no node to join waits in the brood.

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
  `rejoin` alone: their genes and spreads are the node's, so only amounts, sizes (members who fed are now larger than
  those who waited) and, after watching, rows join.
- **One lineage** per node (one brain). Children join the young node of their kind in their cell when alike within
  `alike` steps plus `WITHIN` standard deviations (`body::within`), else start one.
- **Young that grow up** (`copy/young.rs`): a kind in a cell is an adult and a young node joined by a link. Below the
  copy mass the young's log10 bodies have a density e^(−k y) fitted to their mean and variance (`math::Pass`), the
  steady state of a stage entered at one size; those pushed past the copy mass move to the adult node with their slice
  (`copy::mature`): the flux across a size of a size-structured population (McKendrick 1926; von Foerster 1959).
- **Merging** (`nodes::crowd`, monthly): alike bodies of one lineage and stage in a cell join a node of more than one
  member if they are in the room of the same structure (its root) or none and hold the same structure or nothing
  (`nodes::Kin`). Past `crowd` nodes a cell merges its most alike pairs of one lineage and stage while alike, whatever
  they hold and whatever room they are in (the one that joins lets go of its part and takes the other's room); unlike
  nodes never merge. This merge is a speed tool with a physical side effect: the joiners' members drop the structure
  they held, which stays in the cell for any body to take, and leave or enter a room (decision 56).
- **Splitting** (`nodes::split`): a node whose halves would no longer be alike splits along its widest gene if its cell
  has room; selection then acts between the halves. Means are kept exactly.
- **Selection inside a node** (`copy/select.rs`): a node without a brain also runs its update for a sample member one
  standard deviation up or down in one drawn gene, its moves dropped; the gene's mean moves by the step × tanh(sample's
  gain − node's gain) × genes drawn from × `sample_every`, the gain the log of what is or soon becomes body
  (`copy::buildable`: body matter with its brood, and of what its stores could make as much as one interval's growth
  allowance builds; built solids count by what they do, decision 69): Price's equation (Price 1970), the breeder's
  equation (Lande 1976). Skipped where the step cannot change the gain.
- **Choices as a spread of actions** (`choose/spread.rs`) for a node with a brain (5.3).
- **Pace** (`Body::interval`): a node without a brain updates at most every `node_days`, and every interval is a whole
  number of batches (0); rules per interval are rates over the update (`Body::paces`), so its `INTERVAL` gene is the
  turnover time of its stores (a known limit).
- **Brain size** (`lineage::resize`): a mean brain-size gene a whole unit off its lineage's starts a new lineage. A
  body with no brain does not mutate its brain-size genes (`body::vary`): they do nothing without one, so a brain
  arises only in a lineage founded with one (decision 72). An
  individual's child joins its parent's lineage when its size genes fit it, else its partner's when they fit that
  (its rows carried over to that brain); only a child that fits neither starts a new lineage (`copy::child_lineage`).
  `nodes::open` draws members as individuals. `--individuals` keeps bodies with a brain out of nodes (handover 7.1).

## 5. Choose: a small brain sees a little, acts, learns and copies others
Files: `core/src/choose/` (`mod`, `perceive`, `learn`, `feed`, `go`, `spread`), `core/src/brain.rs`.

### 5.1 The brain
One hidden layer over what the body perceives and remembers. Its **base** belongs to a lineage and changes only by
evolution; its **rows** are genes of each body: fixed action scores, shared scorers (per direction, per verb and body
or part seen), a signal, the shape of a part, the next memory and a critic. Sizes are genes with costs: hidden units
(0 to 64; 0 is no brain, no decisions, no cost; light-eaters start so), memory (0 to 12), bodies seen (1 to 6), sight
(1 to 4 cells). One brain per body or node (`Body::rows`); a child starts with its born rows plus what its parent
learned (`Life::inherit`).

### 5.2 Perception
Local (`choose/perceive.rs`): own stores against minimums, warmth, size, light; its cell's loose substances, liquid,
temperature and crowding; the season; signals; memory; cells within sight ring by ring (weight 1 / ring²), with their
parts, as the last survey saw them; up to `SEEN` bodies in or next to its cell as the batch's start saw them (their
faces, 0: looks, signals, well-being, past dealings); up to 3 parts. Walls and cover hide.

### 5.3 Actions
One action is drawn by the scores at each decision (`choose::act`), paid by its own reactions (`body::pay`) or failed;
a node spreads its members over the actions by those chances (`choose::spread`). **rest**; **move** (`choose::go`:
`walk` per kg and metre, climbing, the bodies in the way; needs force, 3.5); **take**, **put**, **give**
(`choose/feed.rs`); **watch** (5.5); Bond's form, hold, take from or put to a part, join, break, let go, anchor, dig,
enter (`bond/verbs.rs`). No copy action, no "farm", "trade" or "attack": those must be patterns of these.

**Taking** (`feed::intake`, one law for the ground and for bodies) goes by uptake genes (`Body::wanted`) at `c_handle`
per kg, of food only (`Body::uses`: what it burns, the body matter and built solids it digests, the minerals it builds
of; not what it cannot use, nor the key liquids it drinks, `soak`): per kg of supply, Holling's type II take × s /
(take + s), s what the swept area holds (`sweep`) of its food, so thin food takes time to find (Holling 1959). The
minerals it builds of count in s as food does: one search finds all it uses, so rich ground shortens it. Another body's
members lie over the cell like the ground's food: a bite reaches those the swept area holds (one individual body
whole, by one draw with the chance the area holds it, whether a node or an individual takes it; of a node, less than one member is one whole member met with that chance:
it takes, tears and pays that meeting's outcome times the chance), limited by force against theirs; a quicker body
escapes with the chance its interval gives. **Giving** (`Body::spare`) is of what the other uses, of stores only (no
matter, brood, built solids or key liquid below level), above `keep_min` × their minimum, at most what the members met
could take.

### 5.4 Learning
**Well-being** (`Body::wellbeing_at`): the energy its stores could release, in decisions' worth, less how near it is
to its limits; no reward is hand-shaped. **Within a life** (`choose::learn`, `brain::learn`): a policy gradient
against the critic (actor-critic) on the change of its own well-being, at its `LEARN` rate, in units of the root of
the critic's running mean square of errors; that mean and each row's mean square of steps both weigh the newest value
by `learn_beta`. A node learns as from one
member drawn among those who stayed, with the node's change plus how much more its part's action raised it than the
mean, so its brain moves as an individual's does. **Across lives**: evolution of born rows and base.

### 5.5 Imitation, signals, memory
**Watching** (`choose::learn::watch`) costs the turn and moves the watcher's rows part way (`blend`) toward the
watched one's, if the brains have the same sizes, less the less alike they are. **Signals**: 2 numbers a decision with
noise (`signal_noise`), meaningless at first, at an upkeep (`c_signal`). **Memory**: up to 4 others and how dealings
went (`Body::note`, `Body::recall`). **Thinking** (`Physics::think`) costs by brain size and mass^0.25 per decision, so
a long interval saves it. **Null run** (`--null`): no learning or imitation, rows frozen, genes still evolve.

## 6. Bodies
Files: `core/src/body/`, `copy::grow`. `body::live` runs an update in stages: upkeep, the physics of the time since
(`body/physics.rs`), keeping warm, store checks, growth and children, shedding; then the decision. Its trades are
gathered as moves and booked with its batch's by one `store::apply` per domain (`store::Moves`, 0), each set by its
law's rate over the update's span, so nodes and individuals take alike, never more than is there.

### 6.1 What a body is
**Stores**: every substance (all members together) and heat. **Body matter** (22) is what it is built of; its
**living body** (`Body::built`) is matter less brood, over 1 − `WET_TISSUE`. Taking, building, upkeep, force and light
go by the living body; fuel and extra liquid are carried, and storing fuel costs no work. `Body::size` (with its
liquid) sets height, volume and minimums; `Body::mass` (everything) moving and load. **Levels** (`Body::level`): the
key liquid h / (1 − h) of matter (h its keep gene), a built solid its share of matter, others keep gene × kg of living
body; a full store is satiety. **Supply** (`Body::supply`): living body^`intake_exp` from `m_ref` × its pace (6.2).

### 6.2 Upkeep and paying
`body::upkeep` (`body/pay.rs`):

```
tissue = living body × (copy mass / m_ref)^(intake_exp − 1)
upkeep = [ days × ( tissue × (c_basal + c_cat·capacity + c_strength·STRENGTH + learning + signal)
                    + c_leaf × leaf area ) + one decision ]
         × (1 − c_repair + c_repair·2^REPAIR) × activity × upkeep pace(T)
```

Costs go by the kg while supply goes with mass^0.75 (West, Brown and Enquist 1997), so a body grows until keeping what
it has takes all it can supply: its costs set its size, not a cap; across kinds adults pay Kleiber's mass^0.75.
**Capacity**: Σ over catalysts on of 10^gene × k × exp(−Ea / `t_cat`). **Pace** (`Body::pace`): its main burning's
rate constant as its catalyst runs it at T (`Body::rate_at`, unfolding past `DENATURE`) over that at `t_cat`; supply
follows it. **Upkeep pace** (`Body::upkeep_pace`): the bare rate law of that reaction, k e^(−Ea/T), over its value at
`t_cat`: it never falls past the unfolding, as unfolded proteins are replaced, not saved (Hofmann and Somero 1995).
Seasons come from temperature, not a calendar. **Paying** (`body::pay`, `body::plan`): burnings run
as far as the rate law allows with the air breathed; the unpaid share dies (`SPENT`).

### 6.3 Heat
One heat network (`body/exchange.rs`): core → coat (`HEAT`, × mass^(2/3)) → surface → still air (forced convection,
`forced` × √(`wind_ground` / d)) and radiation in parallel; others' cover (`shelter`) and walls (3.4) in series. Work
heat leaves through it; the share (1 − e^−x)/x stays, x = conductance × time / heat capacity (`body::spend`).
Reactions run at the body's temperature, held by the ground as a store of what it can conduct in over the update; the
gas they release takes its latent heat as far as heat pays (`physics::settle_gas`). Below `KEEP_T` the body burns the
heat that brings it there; above `COOL_T` it evaporates it, no faster than the still air carries vapour (the Lewis
relation), never below its liquid's minimum.

### 6.4 Water: breath, rain, soaking, roots
**Breath** (`breath`): `GAS` is the pores' (stomata) conductance per m² of open surface, in series with still air,
slowed by liquid over the body (`gas_depth`); for vapour the pores close as the body dries (`closing`). **Rain**: a
cell's bodies catch 1 − e^−x of the rain since their last update, shared by height. **Soaking** (`LIQ`): through its
ground contact at `LIQ` × wetness (half at `wet_depth`) × solubility, driven by how far it is below its level, solved
exactly (`soak_again` after growth). **Transpiration** (`through`): where soaking meets what the pores pass, liquid
flows through the body, its latent heat paid by the body and what the ground conducts in (`physics::heat_bound`).
**Roots**: mineral uptake genes take them from the loose ground at the soaking rate.

### 6.5 Activity and dormancy
`Body::activity`: a = min(1, l / l*)^`dormancy`, l the key liquid, l* its level. It multiplies the catalysts' speed-up,
upkeep and the building allowance: a dry body pays and ages almost nothing and can hardly act. **Frost** kills a key
liquid's frozen share × a (`PHASE`); **boiling** kills.

### 6.6 Growth
`copy::grow`. Sugar and minerals become matter at `c_copy` of work per kg, paid by `body::pay`, as far as all allow:
the **allowance** (`c_build` × supply / `m_ref` × days × activity), the **spare** (its reactions must still release an
interval's upkeep), what its reactions release now, and its stores. Built solids (frame, lime) come first by their
share (`Body::share`), up to level; they pay no upkeep. **Healing is growth.**

### 6.7 Taking, digesting, minimums, ageing
Intake at most `take` × supply a day (5.3). **Digestion** (`Body::digest`): matter taken is broken back into what it
was built of, so food enters as fuel, and only if a catalyst of its own breaks it (`Body::uses`). **Minimums**
(`Body::lacking`): a store below its minimum kills (`LACK`), except a fuel, whose minimum is a reserve. **Shedding**
(`shed`): minerals, built solids and key liquid past level, and anything it neither burns nor is made of. **Ageing**:
wear grows by the energy upkeep turns over per kg, over `e_life` × 10^`REPAIR` (the rate of living); 1 − e^−wear of
the time since dies (`AGE`), in nodes as in individuals. **One rule of death** (`body::live::lose_share`): what kills
a share of a node's members kills an individual with that chance.

### 6.8 Damage
`body/death.rs`. **A bite** (`take_body`) tears from each member reached the share taken and pays to break it (3.6);
matter breaks first and spills its stores; in a node those members lose it (grazing) or die (`body::wound`). **Pushing
through** (`choose::wear`) breaks what its energy pays for: the energy movers spent in a cell is kept until the next
survey (`Life::pushed`); then each body there, a mover that stayed too, takes its share by mass and loses what that
breaks of it, each of its members alike. So a cell's bodies are worn once a survey, not once for each move into it.
**Heat** burns matter by reaction 21. A member dies when `lethal` of its body is torn at once; a hurt adult is
immature until healed. **Starving** (`starved`): a body below its copy mass that did not rebuild what it lost keeps
the share x and dies with chance 1 − x^a, a = ln 2 / −ln(1 − `lethal`), so starving to the lethal loss is survived
with even chances in one update or many (`SPENT`).
Causes: `LACK`, `PHASE`, `SPENT`, `TAKEN`, `BROKEN`, `AGE`; at death all goes to the ground (`body::die`).

### 6.9 Materials and minerals in bodies
Built solids count in mass and height, not living body; children get them, gifts never give them. Light-eaters get
minerals by roots, eaters by digestion. The rock salt falls out of a body as lime (23). Fixing (24) makes the nitrogen
mineral from the air, slowed as the store fills and stopped at its level.

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
- **Life** at year 2 (`--life-year`): light-eaters, then eaters 20 years later (`worldgen::life::KINDS`); `--roster`
  places ten kinds (`ROSTER`). Founders are adults on land under at most half a metre of liquid; mass 30% sugar (60%
  for the two big slow kinds).
- **Brains**: eaters start with hand-set biases (`Seeding::hand`; `--neutral-eaters` without), since neutral founders
  die before they learn: they bite a body (2) before they take from the ground (0.5) and keep out of deep liquid (a
  move's odds fall as 1 / (1 + depth / m)). Light-eaters have no brain. Genes and rows get a spray (`--spray`).
- **Fixers**: a tenth of the starting light-eaters carry the fixing gene (`FIXERS`, `FIX_GENE`; `--fixers`).
- **Founder genes**: some measured from the laws (`LIGHT_X`, `LITTER_X`, a real leaf's `GAS`); light-eaters build
  their frame at a herb's share of cell walls (`WALL_SHARE`), other solids at a small share (`BUILT_SHARE`); `HEAT`,
  `CHILD`, `ALLOC` the same for all; `KEEP_T`, `COOL_T` off for light-eaters and, for eaters, 5 K above their
  liquid's melting point and 5 K below their catalysts' unfolding (`EATER_KEEP_T`, `EATER_COOL_T`).

