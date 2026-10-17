# The laws as they are now

This file says how each law of the world works **today**, at the end of M3. It is written in
the present tense. It does not say when or why a rule came: that is in `docs/history.md`.

- The plan and the rules of work: `docs/handover.md` (the section numbers in this file, like
  "handover 6.2", point there).
- Every number, its unit and where it comes from: `docs/numbers.md`. This file gives a number
  only when it helps to understand the rule.
- What a run records: `docs/tape.md`.

The code is the truth. Each part below names the file and the function that does it, for
example `body::upkeep` (the function `upkeep` in `core/src/body.rs`).

**Words used here**

| Word | Meaning |
|---|---|
| cell | one square of the map (2 km × 2 km). It has a ground node and sees an air node. |
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

---

## 0. One step of the world

The world moves one month per call of `step` (`core/src/lib.rs`, `step_marked`). Inside the month:

1. Every `phys_days` days (3): `body::survey` (what each cell holds, light and shade),
   then Flow (`flow::step`), then Change (`change::step`).
2. On the first day of the month: Bond (`bond::step`): parts react, loads are checked.
3. Every day: `body::day` updates the bodies whose interval ends that day (each body every 5 to
   360 days, set by its `INTERVAL` gene).

Inside Flow (`flow::step`): radiation every call; evaporation and rain every `exchange_days` (3);
wind every `air_days` (3); liquid and drainage every `flow_days` (15); erosion work every 7 months;
creep once a year. A part that is not due waits and then runs for all the time it missed.

**The books.** Every move takes from one node exactly what it gives to another, with its heat.
Elements are never made or lost. Energy changes only by the sun, the heat that leaves to space,
and edits from outside (booked as edits). `core/src/check.rs` counts all of it. A run prints
the drift every year; it must stay near the rounding of the numbers.

---

## 1. Flow: stuff moves from high push to low push

Files: `core/src/flow.rs` (ground and liquid), `core/src/flow/air.rs` (air, radiation, rain,
wind).

### 1.1 The ground of a cell

A cell's ground has layers, from the bottom: the **bed** (rock, bonded), the **loose ground**
(grains that can move), soft cover (solids warm enough to be soft), and liquid on top
(standing liquid, a lake or the sea). `flow::surface`
builds these layers; `flow::ground_state` and `World::thaw` find the temperature.

- **Heat of the ground.** Only the top `skin_m` metres of ground trade heat with the surface,
  plus everything that lies on it (liquid, ice, cover). The ground's temperature comes from its
  energy and its heat capacity.
- **Melting plateaus.** A ground node holds the melting heat (fusion enthalpy) of what is
  molten in it. So it stays at a melting point until that substance has all frozen or all
  melted. Each substance melts at its own melting point **in that node**, lowered by what is
  dissolved in its liquid (the freezing law, 2.5). The plateaus are passed in the order of those
  points.
- **Deep store.** Under the loose ground lies a store in the bed (`World::deep`): the weathered
  zone, `bed_depth` deep, with `bed_porosity` of it as pore space. Bodies and Change do not
  reach it.

### 1.2 Liquid

`flow::liquid`, one ordered sweep over the cells, from high to low.

- **Pores first.** The loose ground has pore space: `porosity` × the volume of its loose solids.
  A cell's liquid fills the pores first. Only the liquid above them **stands** and has a head
  (`flow::stand`).
- **Running off.** Standing liquid runs to lower neighbours, by the head (height of the ground
  plus the liquid), to every neighbour at least `split` as steep as the steepest one. A head
  difference under `still` does not move.
- **Pore water.** The liquid in the pores flows by **Darcy's law**: flow = `seep` × slope of the
  liquid table × the thickness of ground it fills × the width of the edge. It goes to the
  neighbour with the lower table, and never more than half way to a level table (`flow::table`).
- **Channels inside a cell.** Real land drains to small channels a few hundred metres apart.
  So the loose ground above its field capacity (`field` of its pores) and the deep store also
  drain by Darcy's law down the slope of the cell, over a hillslope of `hill` metres, into the
  liquid that runs off. A node under deep liquid has no slope and does not drain this way.
- **Drainage down** (`flow::drain`). Liquid above field capacity sinks into the deep store (at
  most `seep` per second). The stores flow to their neighbours by Darcy's law (`bed_seep`).
  Liquid that reaches a full store comes out in that cell (a spring).
- **What goes with the liquid.** A move takes its share of every liquid and of what is
  dissolved in it (salts, dissolved gases), with its heat. A salt is written in the data as a
  liquid that never boils (substances 23 and 24): it goes with the liquid by its share, and
  stays behind when the liquid evaporates. There is no other rule for dissolved solids.
- **Lost head is heat.** The height a liquid loses becomes heat in the ground, and does work
  (1.3). Matter lifted or dropped is booked in a gravity ledger, so energy stays exact.

### 1.3 Erosion and creep

Moving liquid loses energy (its lost head). That energy does work where it is lost
(`flow::liquid`, every `WORK_MONTHS` months for all the time since):

- **Carrying.** It carries loose solids: `carry` m³ per J. They settle where the flow slows.
- **Wear.** It breaks bonded bed into loose ground: `wear` m³ per J, divided by the bed's
  strength.
- Bodies on a cell hold the ground: they cut carrying and creep by 1 / (1 + their mass per
  area / `hold`).
- **Creep** (`flow::creep`, once a year): loose ground slides downhill like a slow diffusion,
  `creep` × softness × thickness × slope.

`carry`, `wear` and `creep` come from real erosion rates (`docs/numbers.md`). Most rock that
leaves the land is lime that Change dissolves (reaction 22, 2.6) and the liquid takes away.

### 1.4 Sun, heat and radiation

`air::radiation`, every Flow call.

- The sun warms each cell by latitude, season and day length (`insolation`, the planet settings
  `sun`, `tilt`, `lat_south`, `lat_north`). The surface reflects part of it by the reflectance
  of what covers it (ice and sand more, dark plants less).
- Every surface emits σT⁴. The air absorbs a share 1 − exp(−x) of what the ground emits; x is
  the optical depth, the sum over the gases of amount × absorption (each gas past its own column
  amount grows only as the square root).
- The air emits from 4 grey layers whose temperatures fall with height by the lapse rate
  (`air::column`, `air::glow`). More gas lifts the layer that reaches space into colder air, so
  the ground warms. There is no ceiling.
- The ground trades heat with the air touching it (`sensible`). The ground temperature is solved
  implicitly (stable at any step). All the air absorbs and emits is booked to its node.

**Light on bodies.** The light that bodies' leaves absorb warms the ground as it falls, except
the share their light reactions store. That share is set aside in the cell (`World::light_pool`,
`body::survey`). A body takes its share at its update. Only the difference to what it really
stored is settled with the ground. **Heat drawn from the ground** works the same way
(`World::heat_pool`): the heat each body draws from its ground per second at its update is set
aside as time passes, so a body that updates every 20 days does not pull 20 days of heat out of
the ground at once.

**Shade.** Light passes the bodies of a cell from the tallest member down (`body::shade`).
Each layer of leaf area x per ground area takes 1 − exp(−x) of what reaches it. Equal heights
share a layer. Height is what the body's materials hold up (6.9). Rain on bodies is shared the
same way (6.4).

### 1.5 Evaporation, rain and dissolved gases

`air::exchange`, every `exchange_days`.

- **Evaporation** by vapour pressure: the ground's saturation pressure against the vapour
  pressure of the air near it. Wet ground evaporates at the open rate when liquid stands on it
  or its pores hold their field capacity or more; drier ground in proportion to its liquid.
- **Rain.** Where the air a cell sees holds more vapour than it can at its temperature (colder
  on higher ground), the rest rains out. Condensing air makes clouds that cover part of the sky
  (`cloud`).
- **Gases dissolve** (`air::dissolve`). A gas that cannot condense on the ground dissolves in
  the liquid there toward Henry's share (solubility × partial pressure × the liquid's amount),
  and comes out when the liquid warms, thins or freezes. It crosses the surface through the
  air's film (`evap`) and the liquid's film (`dissolve`) in series. Dissolving frees the
  vaporisation enthalpy into the liquid; coming out takes it back. Dissolved gas moves with the
  liquid and returns to the air only here.
- **Known limit.** The whole depth of a liquid is one mixed layer (for heat and for gas). A deep
  sea warms over centuries and takes up gas in months; a real one is the other way round.

### 1.6 Wind

`air::wind`, every `air_days`. Gas moves between neighbouring air nodes by three pushes on each
edge:

1. the pressure of the whole column (p = nRT/V), from high to low;
2. overturning: at equal height, the lower half flows to the warmer column and the upper half
   back; both are turned sideways by the planet's rotation (`rotation`); vapour crowds in the
   lower half;
3. eddies mix the two columns (`mix`).

It is solved implicitly along lines (east–west, then north–south), so any speed is stable. Air
in Flow moves as whole columns, so there is no wind near the ground; bodies use the anchored
`wind_ground` (6.4).

### 1.7 Paths

A body that moves through a cell spends energy pushing through the bodies there
(`choose::go`, `choose::wear`). That breaks what it pushes through (6.8) and clears the edge it
used (`World::pass`). A cleared edge is cheaper to cross (`choose::edge`, `choose::eased`). Once
a year the bodies grow back into cleared edges (`choose::regrow`, rate `regrow`). This one rule
is what can make trails and paths.

### 1.8 Stability

No amount goes below zero: a node never gives more than it has. Small negative numbers from
rounding are set to zero and counted.

---

## 2. Change: substances react inside nodes, faster when hot

Files: `core/src/change.rs` (the reactor), `core/src/chem.rs` (the chemistry data and its
physics), `worldgen/src/chem0.toml` (chemistry 0).

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
- **Order.** The data may give each input its own power (`Reaction::ord`); else the count is the
  power. A big molecule is not a collision of 40 molecules.
- **Extent.** Over a step the reaction goes as far as the integral of its rate law
  (`change::gone`): never past the scarce input. For an input of order above 1 the rate falls as
  the input goes, and the step follows that.
- **Heat.** Each reaction unit gives −ΔH of heat to the node. A **light reaction** takes
  absorbed light instead of heat: it stores `LIGHT_YIELD` (5%) of the light it takes; the rest is
  heat.
- `change::pace` holds the shared formula. Ground nodes, air nodes, parts and bodies all use it.

### 2.2 Where it runs

- **Ground and air nodes** (`change::step`, `Reactor::react`): every Change call, with
  sub-steps when the temperature changes fast. To save time, a calm node reacts every `LAZY`
  ticks with that much time, and a very calm one every `SLOW` ticks (`Reactor::calm`); this is a
  speed tool, not physics.
- **Parts** (`bond::step`): once a month, at their cell's temperature, with the air.
- **Bodies** (`body::live`): at each update, with catalysts (2.4).
- **Growth** is a conversion too, but only in bodies and paid by work (6.6, Copy).

### 2.3 What gas a ground node touches

A ground node's gas is its own gas plus its cell's share of the air above. **Under standing
liquid** (more than the pores hold) it touches only the gas dissolved in that liquid, which the
liquid's film renews from the air (`Reactor::film`, time τ = depth / `dissolve`). Each gas it
takes is held at Henry's share × (1 + made·τ/eq) / (1 + used·τ/eq): a film in series with the
reactions. So dead matter on the sea floor rots only as fast as the film brings oxygen.

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
- Enzymes cost upkeep by their **capacity**, whether they are used or not (6.2).

### 2.5 Phases and the freezing law

A substance is solid, liquid or gas by its melting and boiling points (`Chemistry::phase`).
**The melting point of a liquid in a mixture** (`Chemistry::melt_in`, `Chemistry::phase_in`) is
lowered by what is dissolved in it, by the ideal-solution law:

```
1/T = 1/Tm − R · ln(x) / Lf        x = the liquid's share among itself and what is dissolved in it
```

- Only the **solvent** is lowered: the liquid that is the most of the liquids there.
- Dissolved means: substances liquid there that melt below it (the salts 23 and 24, and the fat,
  6, for want of better data), and gases held in the liquid (Henry). A solid is not dissolved.
- A solute keeps its own melting point.
- The whole amount of the liquid (ice and liquid) counts, so it freezes at one point.
- The same law is used by bodies (frost, evaporation, key liquids), by the ground's plateaus,
  ice cover and surface (`World::thaw`, `World::melt`), by what bodies take from or give to the
  ground with its fusion heat (`body::fused`), and by Change's own phases (`Reactor::phases`).

**Latent heat.** Flow keeps the fusion and vaporisation heat of the ground and the air. A gas in
the air holds its melting and boiling heat (`body::held`); every gas that enters or leaves the
air pays or gets it. A body holds its liquid without its fusion heat: when it soaks liquid from
the ground it takes the ground's fusion heat as warmth, and when it gives liquid back it gives
only its sensible heat. The books stay exact.

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
reactions. In `core` and `worldgen` they are numbers only. Earth names (for people) are in
`library/chem0.toml`. Roles that the laws use:

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
  (no heat in or out);
- frame: 6 sugar → 1 frame + 1 liquid;
- lime: 1 rock salt → 1 lime + 1 carbon gas + 1 liquid (the gas goes to the air).

`Chemistry::minerals` lists the inputs of growth that no reaction burns (24 and 23).
`Chemistry::materials` lists the solids bodies build (25 and 10). A random chemistry has none of
these yet: its bodies do not grow.

---

## 3. Bond: things can be joined, joins break when pushed too hard

File: `core/src/bond.rs`. Also `choose::go`, `Body::height`, `Body::breaking`.

### 3.1 Parts and structures

- A **part** is a made object: solid walls, the liquid held in its hollow, heat, a size (3
  numbers) and a hollowness. It lies in a cell or is held by a body. A body makes one with the
  action "form" (`bond::form`, cost `c_form` per kg), at most `form` of its mass.
- A **bond** joins two parts, or a part and the bonded ground. Its strength (N) comes from the
  weaker material at the joint and the contact area (`tensile`, `joint`). Making it costs
  `c_join` per m².
- A **structure** is the parts joined by bonds. It moves as one.
- Parts react with the air once a month, at their cell's temperature (Change).

### 3.2 Loads and failure

Once a month (`bond::step`, `bond::load`): each bond carries the weight hanging on it from the
anchored part, less what the liquid around holds up (buoyancy), plus the push of moving liquid
and of bodies moving through the cell (`bump` of their weight). A bond loaded past its strength
breaks. What has no support lies on the ground. A structure the liquid holds up floats and drifts
down the liquid's slope (`bond::afloat`).

### 3.3 Holding, carrying, digging

- Holding a part is a bond too. Its strength is the body's **force**: strength gene × its living
  body (`Body::built`) × gravity × `grip` (`bond::bears`, `bond::force`). A held part can add a
  lever (`bond::lever`). A body takes hold of a part only if its force bears the weight of the
  part's structure, and lets go when it no longer does (checked each month).
- Fuel a body carries adds no force. A hurt body is weaker.
- **Digging** (`bond::dig`) breaks bonded ground; its work goes with the body's force and time.

### 3.4 Containment

If a structure's walls close around a space, it has an **inside** (`bond::room`). A body in it
loses heat to the outside only through the walls and the openings (a conductance in series with
its own; `leak` per m² of opening), and gets the light and rain that come through the openings.
"Shelter" is nothing more than this low conductance.

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

File: `core/src/copy.rs`. Growth is also here (`copy::grow`), and the body side in 6.6.

### 4.1 Genes

A body's genes (`body::Layout`) are:
- 22 **scalar genes**: `GAS`, `LIQ`, `HEAT`, `LEAF`, `STRENGTH`, `CHILD`, `COPY_MASS`,
  `INTERVAL`, `MUT`, `LEARN`, `FORGET`, the four brain sizes `HIDDEN`, `MEMORY`, `SEEN`, `SIGHT`,
  `DISPERSE`, `DENATURE`, `REPAIR`, `WET`, `ALLOC`, `KEEP_T`, `COOL_T` (meanings in
  `core/src/body.rs`);
- one **catalyst** gene per reaction (log10 speed-up);
- three genes per substance: **safe minimum** (a share of the body; below it the body dies,
  6.7), **uptake** (how much it takes of that substance), **keep** (the most it holds of it);
- the **born brain rows** (5.1), which are genes too.

**Mutation** (`copy::vary`, `copy::mutate`): each gene of a child is the parent's plus noise of
one step × 10^`MUT`. Each gene has a step and a range (`copy::range`). Genes that are off
(catalysts, minimums, uptake below 0) have 20 steps of room below 0, so an off gene can wander
without hitting a wall. A catalyst step is 0.3 decades. Born brain rows change with a chance
`row_chance`, by `row_noise` (`copy::shake`). Rarely (`new_lineage`) a child changes the base of
its lineage's brain and starts a new lineage.

**Likeness** (`copy::alike`): two bodies are alike when every gene is within `alike` steps (4) of
the other; brain and sense sizes count in whole units. Likeness decides who mixes, who joins a
node and who merges.

### 4.2 Children come from a brood

- A body is **mature** when its living body per member (`Body::built`) is at or past its
  `COPY_MASS` gene (`Body::adult`).
- At each update `copy::grow` builds what it can (6.6). An immature body puts it all into its own
  growth. A mature body puts the share `ALLOC` into a **brood** (`Body::brood`: body matter held
  for children, not yet body) and the rest into its own growth. With `ALLOC` below 1 it grows all
  its life; with 1 it stops at maturity.
- Whenever the brood covers one child's matter, a child is made (`copy::copy`, `copy::emit`), at
  the body's update, whatever its brain does. Bodies copy by physics only.
- **A child** has a body of 10^`CHILD` × its parent's copy mass (from seeds at −6 to half the
  parent at log10 0.5). It gets that matter from the brood, provisions of every other store in
  the proportion its parent's body holds them to its matter (the key liquid times 10^`WET`, so a
  body can make dry, dormant children), its share of the heat (it starts as warm as its parent),
  and its genes plus noise.
- An individual makes whole children (the rest of the brood waits). A node makes fractional
  members. The parent keeps its body and pays nothing more: the matter was paid for when it was
  built.
- A share `DISPERSE` of a node's children starts in a random neighbour cell (for an individual,
  the chance that its children do).

### 4.3 Two parents

When a mature body makes children, it looks in its own cell for a **partner** (`copy::partner`):
another mature body alike to it, the most alike one (a keyed random pick among equals), never
itself.

- Each gene of the children is one parent's, drawn gene by gene. Each born brain row is one
  parent's, drawn row by row (a row is kept whole). Then the normal mutation.
- The chance to take the partner's is the partner's share of the members of both
  (`copy::share`), so the mean of all members is kept.
- A group of many children takes about that share from each, and its spread is what a merge of
  the two parents would pool (`copy::mix`, `copy::mix_rows`).
- The partner gives and loses nothing: genes are information.
- No partner: the children copy one parent. Bodies that are not alike never mix, so **kinds are
  the groups that mix**.
- `Config::two_parents` off (`ew run --no-mix`) gives one parent always (a control).

### 4.4 Population nodes

A node (count > 1) is one body for many members (handover 7.1). It is a speed tool: it must give
the same patterns as individuals.

- A node keeps its **spread**: the variance of its members' genes around its mean, per gene, and
  one number for its born rows. The spread never grows past the split threshold (`copy::most`).
- **Joining.** Children join the first node in their start cell that they belong to
  (`copy::within`, `copy::near`): in every gene within `alike` steps plus `WITHIN` (0.5)
  standard deviations of both spreads; in the same **size class** (member bodies within `alike`
  steps of the copy-mass gene, about a factor 6); and in the same **maturity state**. A child is
  always immature, so it never joins its mature parent node. With no node to join it starts a
  body; less than one member with nowhere to go waits in the brood.
- **Merging** (`body::crowd`, monthly). Alike nodes of one class in a cell merge. A cell with more
  than `crowd` nodes merges its most alike pairs while they are alike. Past twice `crowd` the most
  alike merge however unlike (a safeguard). A merge pools stores, heat, the means by count, and
  the spread (each one's spread plus its gap to the new mean).
- **Splitting** (`copy::split`). A node whose two halves would no longer be alike splits in two
  along its widest gene (brain sizes measured in their own steps here), half the members, stores
  and heat each, if its cell has room. The mean is kept exactly. Then selection acts between the
  halves.
- **Gene means move only by selection.** Splits, draws and joins keep the mean of each gene
  exactly; near a wall the noise is shrunk, never clamped.
- **Members as individuals.** A node does an action of Bond through one member split off as an
  individual (`copy::one`); if it then holds something or is inside, it lives on alone, else it
  goes straight back. `copy::open` draws several members as individuals (for a close view).
- **Brain size in nodes** (`copy::resize`): a node whose mean brain-size gene is a whole unit away
  from its lineage's size moves itself to a new lineage of that size, its brain carried over.
- **Age in nodes.** A node has no single age: at each update a share of its members dies of age,
  equal to the wear an individual would take in that time (6.7).
- **Individuals mode** (`Config::nodes` off, `ew run --individuals`): animals are never nodes.
  Light-eaters stay nodes in both modes. It is for the M4 check of nodes against individuals.

---

## 5. Choose: a small brain sees a little, acts, learns and copies others

Files: `core/src/choose.rs` (perception, actions), `core/src/brain.rs` (the network).

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
- **Shared group brains** (`choose::regroup`, monthly). Bodies that stay near each other share
  one set of rows: the mean of their members' born rows (a node counts its count) plus what the
  group learned. A body further than `leave` cells from its group's centre leaves with a copy.
  One that stays `stay` days within `join` cells of a group of its lineage with more members
  joins it. What a group learned fades at the mean `FORGET` rate of its members. A body moves the
  shared rows by its share of the members. `Config::share_brains` off gives one brain per body.

### 5.2 Perception

Limited and local (`choose::act`): its own stores against their safe minimums, warmth, size and
light; its cell's loose substances, liquid, temperature and crowding; the season; signals heard;
its memory; the cells in 8 directions as far as its sight (nearer cells count more); up to
`SEEN` bodies in or next to its cell (their looks, signals, well-being, and how past dealings
with them went); up to 3 parts in its cell. Walls and cover hide bodies.

### 5.3 Actions

One action is drawn by the scores at each decision (once per update). Each costs energy that the
body's own reactions must pay (`body::pay`); if they cannot, it fails.

- **rest**;
- **move** to a neighbour (`choose::go`): cost `Physics::walk` per kg and metre (the real cost of
  transport, more per kg for a small body), plus climbing and the bodies in the way; needs force
  (3.5);
- **take** from the ground (`choose::take_node`) or from another body (`choose::take_body`);
- **put** to the ground (`choose::put`) or **give** to another body (`choose::give`);
- **watch** another body (imitation, 5.5);
- Bond actions: form a part, hold, take from or put to a part, join, break bonds, let go, join to
  the ground, dig, enter an inside (`bond::act` and the others).

There is no copy action by choice: children come from the brood (4.2). There is no "farm",
"trade" or "attack" action: those must be patterns of the actions above.

**Taking** goes by uptake genes, up to what the body can take in the time (6.7), and from a cell
only what the area it works holds (`sweep`, `choose::reach`). Handling costs `c_handle` per kg.
From another body it is also limited by its force against the other's; a quicker body (shorter
interval) may get away. **A bite tears body matter** (6.8).

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
  share.
- **Across lives**: evolution of the born rows and the base (4.1).

### 5.5 Imitation, signals, memory

- **Watching** (`choose::watch`) costs the turn. The group's rows move part of the way (`blend`)
  toward the rows of the watched body's group, if the brains have the same sizes, and less the
  less alike the two bodies are (size, leaf, strength).
- **Signals**: each decision emits a small signal (2 numbers, with noise `signal_noise`). It has no
  meaning at first; others perceive it. It costs upkeep (`c_signal`).
- **Memory of individuals**: a body remembers up to 4 others (`Body::note`, `Body::recall`) and how
  dealings with them went (better or worse).
- **Cost of thinking** (`Physics::think`, `Lineage::cost`): each decision costs by the brain's
  sizes and weakly by the member's mass (mass^0.25). So a long interval saves brain cost.
- **Null run** (`Config::null`, `ew run --null`): no learning, no imitation, brain rows frozen;
  genes still evolve. It is the run to compare a real run with.

---

## 6. Bodies

File: `core/src/body.rs` (and `copy::grow` for growth).

### 6.1 What a body is

- **Stores**: an amount of every substance (mol, all members together), and heat. Its
  temperature is its heat over its heat capacity (`Body::temp`).
- **Body matter** (substance 22) is what a body is built of. Its **living body**
  (`Body::built`) is its matter, less the brood, over 1 − `WET_TISSUE` (0.75: living tissue is
  three quarters liquid). What a body takes, builds, pays to keep, wears, pushes with and catches
  light with goes by its living body.
- **Fuel** (sugar, fat) and liquid beyond the body are carried, not body. Storing fuel costs no
  work; building matter does (6.6).
- `Body::size` is the body with the liquid it holds now (for height, volume and the safe
  minimums). `Body::mass` is everything (for moving, weight and load).
- The **key liquids** are the liquids its keep gene keeps (a bit mask, `Body::wet`).
- **Levels** (`Body::level`): the most it holds of each substance. For the key liquid: h / (1 − h)
  of its matter (h its keep gene, at most 0.95). For a solid it builds: what its share makes of its
  matter. For any other store: keep gene × kg of living body. A full store is the satiety: there is
  no other limit on eating.
- **Supply** (`Body::supply`): what it can take and build per day, in kg-equivalents: its living
  body to the power `intake_exp` (0.75) from `m_ref` (1 kg), times the pace of its temperature
  (6.2).

### 6.2 Upkeep and paying

`body::upkeep`: the cost of living over the interval.

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

- **One heat network** (`body::surface`, `body::film`). Heat goes from the core through the coat
  (the `HEAT` gene, which goes with mass^(2/3)) to the outer surface (skin and both faces of the
  leaf). From there it goes in parallel to the still air and by radiation. The still air's
  conductance is forced convection: `forced` × √(`wind_ground` / d) × the air's heat capacity, d
  the body's length or a leaf's size (`leaf_size`). Cover of other bodies (`shelter`) and walls
  (3.4) are in series. Radiation from the surface goes to the ground (`body::radiated`).
- **Work heat stays** (`body::spend`, `body::hold`). The heat of upkeep and of paid work stays in
  the body and leaves through that network over the interval; the share (1 − e^−x)/x stays,
  x = conductance × time / heat capacity. The rest goes to the ground.
- **Reactions' heat** (`body::live`): a body reacts at its temperature, held by the ground it
  stands on as a store of the heat that ground can conduct into it over the update. Gas taken from
  the air brings its heat; gas put out takes its latent heat and its share of the body's heat.
- **KEEP_T** (keeping warm): if the update would leave the body colder than this gene, it burns
  on demand, like its upkeep, the heat that brings it there, as far as its reactions can release
  it.
- **COOL_T** (keeping cool): if the update would leave it warmer than this gene, it evaporates the
  latent heat that brings it there, no faster than its still air carries vapour away (the Lewis
  relation), against the air's vapour deficit, and never below its liquid's safe minimum.
- Between the two genes it does neither. Founders start at 250 K and 320 K, so neither acts at the
  start.

### 6.4 Water: breath, rain, soaking, roots

- **Breath** (`body::breath`): the `GAS` gene is the conductance of the body's pores (like a
  leaf's stomata), mol of air per m² of open surface (`Body::open_area`: leaf area plus skin) per
  day, in series with the still air. Gases and vapour go through it. Liquid over the body slows it
  (`gas_depth`).
- **Pores close as it dries**: for vapour only, they open as far as the activity (6.5), solved
  exactly over the update (`body::closing`).
- **Rain** (`body::live`): the bodies of a cell catch 1 − e^−x of the cell's rain (x their leaf
  area per ground area), shared by height like the light. A body catches no vapour.
- **Soaking** (`LIQ` gene): a flow along the body's contact with the ground, at rate `LIQ` × the
  ground's wetness (standing or pore liquid; half wet at `wet_depth`) × its working share ×
  solubility per day, driven by how far it is below its level, while it loses what it
  evaporates. Solved exactly over the update. A body afloat reaches no ground.
- **What flows through** (`body::through`): a soaking body loses where its soaking meets what its
  pores pass; what passes beyond its store comes from the ground as it goes. This is
  transpiration.
- **Roots for minerals**: the uptake genes of the minerals take them from the loose ground of the
  cell, at the rate it soaks liquids, up to its level, no more than half of what the cell holds.

### 6.5 Activity and dormancy

`Body::activity`: a = min(1, l / l*)^n. l is the key liquid the body holds, l* the level it keeps,
n = `dormancy` (5.7, from the life of dry seeds). It multiplies the catalysts' speed-up (2.4), the
whole upkeep (6.2) and the building allowance (6.6). A dry body pays almost nothing, ages hardly
at all and can hardly act, but still takes its updates and decisions.

- **Frost**: if a key liquid freezes (at its lowered melting point, 2.5), the frozen share times
  the activity dies (a node loses that share of its members, an individual dies with that
  chance; cause `PHASE`). A dry body holds little free liquid to freeze.
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
  solid only if a catalyst of its own breaks it (`Body::breaks`, `choose::can_take`). Matter taken is checked
  against the room of the stores it becomes (`choose::digest_room`).
- **Safe minimums** (`Body::lacking`): a store below its minimum gene (a share of the body) kills
  it (cause `LACK`). A store it burns has no death line: there the minimum is only the reserve that
  growth and giving keep. A body short of fuel dies when it cannot pay (`SPENT`).
- **Shedding** (`body::shed`): at its update a body gives back to the ground the minerals and
  built solids past their level, the key liquid past its level, and all of any other solid or
  liquid it neither burns nor is made of.
- **Ageing**: living wears a body. Its wear grows by the energy its upkeep turns over per kg of
  living body, over `e_life` × 10^`REPAIR` (`e_life` = 10⁹ J/kg, the rate-of-living figure of real
  life). An individual dies of age (`AGE`) when its wear reaches 1. A node loses that share of its
  members at each update. A member split off a node gets a random wear in [0, 1).

### 6.8 Damage

`body::tear`, `body::thin`, `body::hurt`.

- **A bite** (`choose::take_body`) tears the share of the other's living body that the taker
  takes (or the share of its matter the taker ate, if that is more), and pays the energy to break
  it (3.6). Body matter breaks first, and spills the stores it held, in proportion, to the ground.
- **Pushing through** (`choose::wear`) breaks the share of each body there that the energy spent
  on it pays for.
- **Heat** burns body matter by its own reaction (21), so a burnt body is smaller.
- An individual lives on smaller and weaker (force reads the living body) and dies when half of
  its body or more is torn at once, or when a store falls below its minimum. A hurt adult is
  immature (its body is under its copy mass) until it heals: it makes no children meanwhile.
- **In a node** the size of the bite decides: less than one member's body takes part of every
  member's matter (grazing: the members live, smaller); one member's body or more takes whole
  members.
- **Death causes**: `LACK` (a store under its minimum), `PHASE` (froze or boiled), `SPENT` (could
  not pay), `TAKEN` (taken apart by another body), `BROKEN` (broken by bodies moving through),
  `AGE` (worn out). At death the body's matter, stores and heat go to the ground (`body::die`).

### 6.9 Materials and minerals in bodies

- Founders start with a share of 0.02 of each built solid (`worldgen::life::BUILT_SHARE`). Built
  solids count in the mass (moving, load) and set height (3.6), but are not the living body.
  Children get them as provisions; gifts never give them. Hard parts protect mostly by not being
  digested: a body without the enzyme that breaks a solid cannot use it as food.
- Growth takes 3 of the nitrogen mineral and 1 of the rock salt for each mol of matter. Plants get
  them by roots (6.4), animals by digesting the matter they eat (6.7). The rock salt does not last
  in a body: it falls out as lime by reaction 23 (in hours to weeks). Nitrogen fixing (reaction 24)
  lets a body with the gene make the nitrogen mineral from the air, burning 14 units of its sugar
  for each mol, slowed as its store nears its level (2.4).

---

## 7. Soil and minerals

The two minerals (`Chemistry::minerals`) live in the liquid of the ground.

- **Nitrogen mineral (24).** Placed when life starts (year 2, `worldgen::life::soil`): 0.357 mol
  per m² on every land cell not under half a metre of standing liquid. After that its only input
  is fixing by bodies (reaction 24). Rotting of body matter (21) gives it back; heat breaks it down
  (25) and sends the nitrogen to the air.
- **Rock salt (23).** Made in wet cells from lime, the carbon gas and the liquid (reaction 22),
  turned back by reaction 23. The pores of wet land hold it near its balance (2.6). Rotting
  gives the rock element back as lime, which meets liquid and gas and becomes the salt again.
- Both move with the liquid by its share (1.2): rain washes them from slopes into low ground and
  the sea, and they stay where liquid evaporates. So fertility differs from place to place, and
  plants that fix nitrogen and shed it make rich ground around them.

---

## 8. Starting conditions (not laws)

What is placed at the start is an initial condition. Every run prints a seeding line for each
kind. `worldgen` places it; the laws never read a kind.

- **Terrain** (`worldgen::terrain`): an island with mountains, a metre of loose ground, the sea's
  salt at balance with a 400 ppm air (2.6).
- **Life** starts at year 2 (`--life-year`). `ew run` places light-eaters first, then eaters 20
  years later (`worldgen::life::KINDS`, rows 0 and 1). `ew run --roster` places ten kinds
  (`worldgen::life::ROSTER`): light-eaters of three sizes, eaters of remains, eaters of
  light-eaters and eaters of eaters of several sizes, each at its own year.
- **Founders are adults**: a founder's living body is its copy-mass gene. Its mass is 30% sugar
  (60% for the two big slow kinds), the rest liquid and matter.
- **Brains**: animals start with hand-set brain biases (`Seeding::hand`), because neutral
  founders die before they learn; light-eaters start with neutral brains. `--neutral-eaters` and
  `--plant-bias` change this. Genes and rows get a spray of mutation (`--spray`).
- **Fixers**: a tenth of the starting light-eaters carry the nitrogen-fixing gene
  (`worldgen::life::FIXERS`, `FIX_GENE`; `--fixers`). The share comes from real floras.
- **Founder genes**: some are measured from the laws (`LIGHT_X`, `LITTER_X`, a real leaf's
  `GAS`); `HEAT`, `KEEP_T`, `COOL_T`, `CHILD` and `ALLOC` are the same for every kind.
- **The big-brain test** (`--clever-year N`, `worldgen::life::seed_clever`) copies living eaters
  twice: once with a big brain, once unchanged.
