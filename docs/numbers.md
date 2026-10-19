# The numbers of the world: what each one is, what it is anchored to, and how far to trust it

**Keep it true:** a test (`core/tests/numbers.rs`) fails when a field of `Physics` or `Config` has no row here, or a row names a field that does not exist. A new number cannot be added without saying what it is.

## How to read this table

Every field of `Config` and of `Physics` (`core/src/params.rs`) has one row. A row says:

- **unit:** the unit of the number.
- **fact:** the fact of matter or energy it is (a capacity, a cost, a conductance, a rate, a reference mass, a speed-tool setting...). A number that is not a fact of matter or energy is a setting, and the row says so.
- **anchor:** the real-world size it is anchored to, with the source in a few words. "Run:" means the run or the reason it was set for. "None found" means no real size is known.
- **mark:** one of three, plus three for things that are not numbers of the world.
  - **physical:** a measured size, used as measured.
  - **anchored:** a real order of magnitude, chosen within it.
  - **tuned:** set so a run came out a certain way. The row names the run or the reason. `tuned, first guess` is the same mark for a number with no real size and no recorded run: it was written by hand to get a sensible start. It counts as tuned. The first job of any of them is to find a size and become anchored.
  - **switch:** an on/off setting of an experiment, not a number of the world. **container:** a field that holds other rows.
  - **speed:** a speed setting: it says when the model may save time, not how the world works. Speed settings are outside the tuned budget, and each must pass the test that ×2 and ÷2 change no key number beyond its noise.

**The M7 world search varies only tuned and anchored numbers, within their anchors, and never the physical ones.** Switches are not searched. For an anchored number "within its anchor" means within the real order named in its row (about a factor of 3 each way unless the row says more). For a tuned number it means around its value, by the factor its row suggests. Every run prints the numbers it used (`physics.json`).

**The budget for tuned numbers is in handover 14.3.** The count is at the end of this file.

**How sure a source is.** "checked" means a source was read and agrees (author and year given); "checked in part" means a source was found for part of the figure; "from memory" or "unchecked" means it was not looked up; "my arithmetic" means the figure is worked out from others; "differs" means a real figure is more than 2 times off the one used.

## Config

| name | unit | fact | anchor (source) | mark |
|---|---|---|---|---|
| `sun` | W/m² | strength of the star at the top of the air | the solar constant, 1361 | physical |
| `tilt` | degrees | axial tilt: sets the size of the seasons | the planet's tilt, 23.4 | physical |
| `lat_south` | degrees | latitude of the map's south edge | run: the island spans 2.3° (256 km), placed at 31° so the starting ground is near 287 K (handover 6.1) | tuned |
| `lat_north` | degrees | latitude of the map's north edge | the same run as `lat_south` (31 + 2.3) | tuned |
| `rotation` | none (0 = none) | strength of the turning of moving air by the planet's spin | 1 is the real size of the spin's effect; the wind closure is crude (unchecked) | anchored |
| `days_per_month` | days | the calendar | 12 x 30 = 360 days for a year of 365 | anchored |
| `months_per_year` | months | the calendar | the same | anchored |
| `skin_m` | m | depth of ground that exchanges heat with the surface and conducts it sideways | the yearly swing of temperature reaches 2 to 4 m in soil (my arithmetic, unchecked) | anchored |
| `air_top_m` | m | height of the air column | the lower air of Earth, 8 to 16 km | anchored |
| `gravity` | m/s² | pull of the planet | 9.81 | physical |
| `sensible` | W/(m² K) | heat passed between ground and the air touching it (not bodies: they go by `wind_ground`, their size and `forced`) | measured bulk exchange over land in wind, 5 to 30 (unchecked) | anchored |
| `evap` | m/s | transfer speed on the air's side of a surface (evaporation) | usual exchange speed over open ground in wind, 0.005 to 0.02 (unchecked) | anchored |
| `dissolve` | m/s | transfer speed on the liquid's side of a surface for a dissolving gas; also the film through which a ground node under standing liquid reaches the air for its reactions (film time = depth / `dissolve`, laws 2.3) | slightly soluble gases over open liquid in wind, 5e-5 (4 m a day) | anchored |
| `wind` | m/s per K | overturning air speed per K of warmth difference between air nodes | none found; a sea breeze of a few m/s for about 10 K is the same order (my arithmetic, unchecked) | tuned, first guess |
| `mix` | m²/s | mixing of neighbouring air columns by eddies | the eddy diffusivity of the weather systems of middle latitudes, about 1e6 (from memory: the diffusive climate models of North 1975 and Held 1999); the air then crosses the map in about a day | anchored |
| `rain_time` | s | time in which vapour over saturation rains out | run: a cell sees the mean air of its node, so with the life of real cloud water, an hour or two, its rain falls only on the coldest cells (64 x 64, no life: 27% of the land gets rain, 70% of it on a tenth); at 1 day 85% of the land gets rain and it comes to 0.88 m a year, near the 0.7 to 1 m of real land (Trenberth et al. 2007, from memory); weather systems that make rain live about a day (Houze 2004, from memory) | tuned |
| `cloud` | mol/(m² s) | condensation at which droplets cover half the sky | none found; 1.2e-2 is a light rain of 0.8 mm an hour (my arithmetic) | tuned, first guess |
| `carry` | m³/J | loose solid volume moving liquid carries off a cell per J its runoff loses falling to the neighbour | rivers carry 15 to 20 Gt of sediment a year (Milliman and Syvitski 1992; Wittmann et al. 2020: 15.2) with a runoff of 4e13 m³ a year falling a mean 840 m, 3.4e20 J: 2e-11 m³/J (checked). The yield, not Bagnold's capacity (250 times more): nothing holds the model's loose ground | anchored |
| `wear` | m³/J, times 1/bond strength | bonded volume broken per J dissipated | the same 2e-11 m³/J for the starting bed: over its sum of volume share / bond strength, 1.39, it is 1.4e-11. Bedrock lowers about as the land does, 0.05 mm a year (checked: Wittmann et al. 2020; Portenga and Bierman 2011) | anchored |
| `regrow` | m/s | speed at which bodies spread back into a cleared lane, from each side, under dense cover (times cover / `rho_pass` below that) | 1 nm/s is 3 cm a year, the slow end of the spread of a ground cover by runners (my arithmetic, unchecked) | anchored |
| `creep` | m/s per unit slope, times 1/hardness | loose solids creeping down a slope: a flux of D x slope, D = creep x softness x thickness | hillslope diffusivity 1e-3 to 1e-2 m²/yr (Roering et al. 1999; Fernandes and Dietrich 1997; Hanks 2000; checked in part in Whipple's MIT notes, 2004); 3e-3 m²/yr on the starting loose ground (1 m, softness 1.53) is 6.3e-11 m/s | anchored |
| `nodes` | on/off | bodies with a brain may be population nodes (off, `--individuals`: they are individuals; bodies without a brain stay nodes) | a setting, for the check that nodes give the same patterns as individuals (handover 7.1) | switch |
| `two_parents` | on/off | a mature body's children may have a second, alike parent (off, `--no-mix`: copies of one parent) | a setting: the check that nodes keep the mean of their genes without selection, and the measure of what mixing changes (handover 6.4) | switch |
| `null` | on/off | a null run: no learning, no imitation, born rows and brain bases never change; genes still mutate | the run to compare a seed with | switch |
| `phys` | (a `Physics`) | holds the numbers below | | container |

## Speed settings

Fields of `Config` that say when the model may save time (mark **speed**). The test that ×2 and ÷2 change no key number beyond its noise comes with the first law that uses each one. `MIN_INTERVAL`, a speed constant outside the structs, is in the last table.

| name | unit | fact | anchor (source) | mark |
|---|---|---|---|---|
| `phys_days` | days | how often the survey of the cells, the state of ground and air and Change run | 10, the air's step: three even steps a month; the long-step test (`tape/tests/ranges.rs`) holds the climate to that of 3-day steps within the seeds' noise | speed |
| `air_hours` | hours | the air's step: radiation, evaporation, rain, dissolving gases and the wind in one implicit step, booked whole (`flow::air`); it divides the physics step | 240, the physics step: the long-step test holds the climate to that of 6-hour steps within the seeds' noise; backward Euler is first order in the step, so a much longer step drifts | speed |
| `flow_days` | days | how often liquid moves, drains and wears the ground (`flow::liquid`) | 30: the rain on land and the river cells of 15 days within 1% (worlds without life) | speed |
| `wake_share` | share | a sleeping node wakes before its alarm when one of its amounts has grown by more than this share of what it held when it fell asleep (`clock::Sleep::watch`) | the share of its inputs a calm node's own reactions may use while it sleeps (`change`, `CALM`, 0.05) | speed |
| `sleep_days` | days | the longest a ground or air node's reactions sleep before they run for the time since (`change::step`); 0: every node reacts at every physics step | a season: a node does not react through a whole season at the temperature of another | speed |
| `node_days` | days | a population node without a brain updates at most every this many days; a body with a brain by its `INTERVAL` gene (`Body::interval`) | about 10 updates a year (handover 7.3 suggests a month for population nodes) | speed |
| `crowd` | nodes per cell | past this many body nodes in a cell at a month's start the most alike merge (unlike ones never); a node splits only while its cell has fewer than this minus one; it never bounds the members | 4; nodes must give the patterns of individuals (handover 7.1: the check takes `--crowd 4` and `8` as its noise) | speed |
| `sample_every` | updates | a node without a brain runs its sample member on one update in this many, drawn; its weight and its cap are that many times larger, so each gene moves at the same mean rate (`copy::select`) | 2 halves the samples' time; ×2 and ÷2 keep plants, animals, carbon gas and land temperature within their noise | speed |

## Physics

The five costs of keeping tissue (`c_basal`, `c_cat`, `c_strength`, `c_learn`, `c_signal`) are per kg of living body (`Body::built`) of a kind that matures at `m_ref`, times (copy mass / 1 kg)^-0.25, so adults of every kind pay Kleiber's mass^0.75 (laws 6.2). Where a row says "set at the eater founder", the number is what the starting eater (it matures at 44 kg) pays.

### Change and Flow, as bodies feel them

| name | unit | fact | anchor (source) | mark |
|---|---|---|---|---|
| `light_depth` | m | liquid thickness over a body that holds back 63% of its light | clear sea water: broadband light falls by e in 1 to 20 m; 10 m is the clear end (unchecked) | anchored |
| `light_yield` | share | the share of the light a light-driven reaction takes that it stores; the rest becomes heat | real photosynthesis stores at most 4.6% (C3) to 6% (C4) of the sunlight, 1 to 2% in crops over a season (Zhu, Long and Ort 2008, from memory): 5% | anchored |
| `gas_depth` | m | liquid thickness over a body that cuts its gas exchange by a factor e | none found; a gas moves 10^4 times slower in liquid than in air, so 1 m is generous (my arithmetic) | tuned, first guess |
| `wet_depth` | m | depth of the liquid of a cell, standing or in the pores of its ground, at which the ground counts as half wet (what a body soaks up goes by it) | a film or puddle of about 1 cm | anchored |
| `shelter` | W kg/(m⁴ K) | heat the cover of other bodies passes to a body: a layer of m kg/m² passes shelter/m W per K and m² of its surface, in series with its own | run: set so 10 kg/m² of cover warms a 30 kg member about as much as thick walls do (3 K) | tuned |
| `c_cat` | J per kg of body per day per unit of capacity (1/s) | upkeep of catalysts: `c_cat` x the sum over the catalyst genes that are on of 10^gene x k x exp(-Ea / `t_cat`), its gases at their share of the starting air to their order | keeping enzymes costs a resting animal 10 to 25% of its upkeep (from memory); set at the eater founder, whose burning enzyme costs about a tenth of its upkeep | anchored |
| `t_cat` | K | the temperature at which the costs are anchored; upkeep, and what a body takes and builds per day, go from it by the rate law of its main burning reaction, exp(-Ea (1/T - 1/`t_cat`)) (`Body::pace`) | a typical surface temperature, 290 K (`pcore::units::T_SURF`); the tempo of life follows the Boltzmann factor (Gillooly et al. 2001, Science 293: 2248, E about 0.6 eV; checked: Ernest et al. 2003) | anchored |
| `unfold` | K | width over which catalysts lose their shape above their gene (smooth, `change::denature`) | life's enzymes unfold within a few K | anchored |
| `t_firm` | K | catalysts that keep their shape up to here cost no more | about where life's enzymes fail, 320 K | anchored |
| `c_firm` | K | each this many K more of the firmness gene adds the upkeep of catalysts once more (a firmer shape needs more and stronger bonds) | none found; at 360 K catalysts cost three times as much | tuned, first guess |
| `c_strength` | J per kg of body per day at strength 1 | upkeep of strength | about 1 W/kg at 30 kg: a 30 kg mammal rests at about 45 W (Kleiber); set at the eater founder | anchored |
| `c_basal` | J per kg of body per day | basal upkeep every body pays; strength and catalysts are on top | Kleiber's resting metabolism, about 3e5 J a day per kg^0.75 (checked: Kleiber 1932 and 1947; Smil 2000, Nature 403); this is 1/15 of it, chosen so a 1 kg plant keeps a spare of over twice its upkeep | tuned |
| `c_leaf` | J per m² per day | upkeep of light-capture area | run: so the starting plants pay what they were set to pay. A leaf's own breathing is about 4e4 J per m² per day (1 µmol of carbon gas per m² per s, my arithmetic), 20 times more | tuned |
| `c_brain` | J per kg of a member of `m_ref` kg, per decision | cost of one decision of the starting brain (12 hidden, 6 memory, 3 seen, sight 1) | none found; kept as a speed incentive on purpose: it pushes bodies toward long intervals (handover 6.5). The four rows below split it | tuned |
| `m_ref` | kg | the one reference mass: the member at which a per-kg number is per kg (brain cost, `take`, `sweep`, the heat gene, the costs of tissue) | a unit, not a size: 1 kg | anchored |
| `c_hidden` | J per kg of a member of `m_ref` kg, per decision | cost per hidden unit | derived: a share of `c_brain` (12 x 5e3 = 6e4 of 1e5) | tuned, first guess |
| `c_memory` | the same | cost per memory number | derived: a share of `c_brain` (6 x 5e3/3 = 1e4) | tuned, first guess |
| `c_seen` | the same | cost per body perceived | derived: a share of `c_brain` (3 x 5e3 = 1.5e4) | tuned, first guess |
| `c_sight` | the same | cost per square cell of sight | derived: a share of `c_brain` (1 x 1.5e4) | tuned, first guess |
| `c_learn` | J per kg of body per day at learning rate 1 | upkeep of learning | none found; set at the eater founder | tuned, first guess |
| `c_signal` | J per kg of body per day at signal size 1 | cost of signalling | none found; set at the eater founder | tuned, first guess |
| `e_life` | J per kg | energy a kg of body turns over in its upkeep before it is worn out, at a repair gene of 0 (a body lives 10^gene times this) | Rubner 1908: many kinds that move turn over about 1,000 kJ per gram of body in a life | anchored |
| `c_repair` | share | share of upkeep that is repair at repair 0: upkeep x (1 - c + c 2^gene), so each unit of the gene doubles it and every value of the gene has a cost | remaking a body's own matter takes 20 to 30% of resting upkeep in life | anchored |
| `dormancy` | none | how steeply a body's activity falls as it dries: activity = min(1, l / l*)^n, l the key liquid it holds, l* the level it keeps (`Body::activity`); must be above 1 (`Physics::validate`) | Harrington's rule (checked: Harrington 1963 and 1972): seed life doubles for each 1% less moisture between about 14% and 5%, so a 3-fold drop is 2^9 = 512 times less activity: n = ln 512 / ln 3 = 5.7 | anchored |

### Choose

| name | unit | fact | anchor (source) | mark |
|---|---|---|---|---|
| `c_move` | J per kg per m, for a member of `MOVE_REF` (30 kg); others by `Physics::walk` | energy to move one kg one metre on the flat | the real cost of transport, 10.7 m^-0.32 J per kg per m, 3.6 at 30 kg (Taylor et al. 1982; checked in part: the exponent -0.316, J. Exp. Biol. 215: 2349, 2012); 3 | anchored |
| `rho_pass` | kg/m² | body mass per area that doubles the cost of entering a cell and of handling what is taken there | none found | tuned, first guess |
| `lane` | m | width of the passage a mover clears through bodies | the width of a body or a path, about 1 m | anchored |
| `layer` | m | the bodies of a cell must fit in a layer this thick over it | a tall plant, about 10 m; a ceiling far above real cover (a forest holds under 100 kg/m², this allows 10,000) | anchored |
| `rho_body` | kg/m³ | density of bodies | living matter is about the density of liquid, 1,000 | physical |
| `wind_ground` | m/s | the wind at a body, which the boundary layer of every body's surface goes by (Flow's air moves only as whole columns, so not that) | the mean wind over land at 10 m is about 3.3 m/s (reanalyses, from memory); the log profile to 1 m over a roughness of 0.1 m gives half of it: 1.7 (my arithmetic) | anchored |
| `forced` | mol/(m² s) per (m/s / m)^0.5 | forced convection through a surface's boundary layer: heat conductance this x sqrt(`wind_ground` / size) x the air's molar heat capacity; vapour the same (the Lewis relation) | Campbell and Norman 1998, eq. 7.30: 0.135 sqrt(u/d) for a flat plate in laminar flow, times 1.4 outdoors (from memory): 0.189 | anchored |
| `leaf_size` | m | the length of the boundary layer of a body's capture area (a leaf) | broad leaves are 2 to 10 cm across; Campbell and Norman take 0.72 of the width (from memory): 0.05 | anchored |
| `take` | share per day | most a body can take per day of the mass it takes by, per kg at `m_ref` | real food intake is 1 to 5% of body mass a day (unchecked); a 57 kg eater takes at most 1.8% of its mass a day, about 2.7 times Kleiber's resting need: a real appetite (my arithmetic) | anchored |
| `intake_exp` | none | what a member takes, sweeps, receives and builds, and its leaf, go with its living body (`Body::supply`) to this power | Kleiber's 0.75 (checked: production of 305 mammals goes with mass^0.755, 95% range 0.73 to 0.78, Ernest et al. 2003, Ecology Letters 6: 990); the supply in West, Brown and Enquist's growth law (2001) | anchored |
| `sweep` | m² per kg per day | area a body works per day per kg of its mass that works, at `m_ref` | run: set for eaters to live. A 30 kg body works 300 m² in 10 days | tuned |
| `c_handle` | J per kg | energy to handle one kg taken or put | none found; set for a run | tuned |
| `shape_noise` | none | spread of the shape of a part made, around its mean | none found | tuned, first guess |
| `signal_noise` | none | spread of the signal, around its mean | none found | tuned, first guess |
| `discount` | none | weight of well-being one decision ahead against well-being now | the usual 0.9 of learning machines | anchored |
| `blend` | share | share of the way a watcher's rows move toward the watched one's | none found | tuned, first guess |
| `learn_beta` | share | weight of the newest step in a learned row's mean square of steps | the usual 0.1 of a running average in learning machines | anchored |
| `learn_most` | learning-rate units | the most a row moves in one step | none found | tuned, first guess |

### Bond

| name | unit | fact | anchor (source) | mark |
|---|---|---|---|---|
| `tensile` | Pa | tensile strength of a solid of bond strength 1 | 10 MPa: the order of soft solids made by life and of weak rock (unchecked) | anchored |
| `joint` | share | share of the contact area a joint pressed by a body holds | none found | tuned, first guess |
| `c_form` | J per kg | energy to shape one kg into a part | none found | tuned, first guess |
| `c_join` | J per m² | energy to make a joint, per m² of contact | none found | tuned, first guess |
| `leak` | W/(m² K) | heat passed through an opening | free and light-wind exchange at a surface, 5 to 25 (unchecked) | anchored |
| `form` | share | most of its mass a body puts into one part | none found | tuned, first guess |
| `min_kg` | kg | walls lighter than this fall apart | none found | tuned, first guess |
| `lever` | none | a held part multiplies a body's force by 1 + lever x hardness x m / (m + 0.05 body mass) | none found | tuned, first guess |
| `grip` | share of weight | force of a body of strength 1 | none found; animals push and pull about their own weight (unchecked) | tuned, first guess |
| `bump` | share of weight | push of a body moving through a cell on each structure there; also the push its own force must give for it to move (`choose::go`) | none found; the starting plants (force 0.025 of their weight) stay put | tuned, first guess |
| `dig` | share per day | most bonded ground a body breaks per day of its interval, as a share of its mass | none found | tuned, first guess |
| `deep` | m | liquid deeper than this is crossed afloat by a body holding a structure the liquid holds up | none found | tuned, first guess |
| `afloat` | share | the cost of crossing afloat, as a share of the cost of walking | none found | tuned, first guess |

### Copy

| name | unit | fact | anchor (source) | mark |
|---|---|---|---|---|
| `c_copy` | J per kg of body matter | work to build one kg of body matter (`copy::grow`); storing fuel costs none, and a child costs its parent none (it is given matter already built) | growth in life keeps 10 to 20% of the energy it eats, so a kg of body costs about 1e7 J; a kg of matter then takes 27.3 MJ of fuel, 62% efficient (my arithmetic) | anchored |
| `c_build` | J per day per kg^0.75 of body | most a body pays into building body matter per day: the rate at which it grows and rebuilds what it gave its children (`copy::grow`) | P/B (what real animals make a year over their standing body) x 30% dry matter x `c_copy` / 365 / m^0.75: mouse 9,300 to 15,000, deer 6,800 to 11,000, bison 5,800 to 9,700 (P/B from memory; the mass^0.75 checked, Ernest et al. 2003). 1e4 lies inside | anchored |
| `new_lineage` | chance per child | a child starts a new lineage with a changed base | none found | tuned, first guess |
| `base_noise` | none | size of that change | none found | tuned, first guess |
| `row_chance` | chance per child | a child's born brain rows change | none found | tuned, first guess |
| `row_noise` | none | step of that change per generation, times the body's mutation size (a child that changes takes a step bigger by 1 / sqrt(`row_chance`)) | none found | tuned, first guess |
| `new_noise` | none | spread of the weights a bigger brain adds (none in a null run) | none found | tuned, first guess |
| `alike` | mutation steps | bodies whose genes differ by at most this in every gene are alike: they mix, merge and join; a node splits when its halves differ by more | run: 4 steps. It says who mixes, so what a kind is: not only a speed setting. To be checked against individuals at M4 | tuned |

### Flow, as ground cover and moving liquid feel it

| name | unit | fact | anchor (source) | mark |
|---|---|---|---|---|
| `hide` | m, three numbers | thickness of a layer that hides half of what lies below it, for the reflectance and the work on the bed: soft solid 0.2, liquid 0.5, loose 0.02 | none found | tuned, first guess |
| `soft` | share of melting point | a solid warmer than this share of its melting point is soft cover lying on top | none found; solids soften and sinter at 0.3 to 0.8 of their melting point in K (unchecked) | tuned, first guess |
| `split` | share | liquid goes only to neighbours at least this share as steep as the steepest | none found; a rule of the routing of liquid | tuned, first guess |
| `still` | m | liquid does not move for a smaller head difference | none found; a film of a centimetre does not run on rough ground (unchecked) | tuned, first guess |
| `hold` | kg/m² | body mass per area that halves what moving liquid can carry off a cell and how fast its loose solids creep | none found | tuned, first guess |
| `porosity` | share | the share of the volume of the loose ground that is pore space: a cell's liquid fills the pores first, and only what is over them stands and runs off | random close packing leaves 0.36 (Scott 1960; checked in part, Radin 2008), the loosest random packing 0.445 (Onoda and Liniger 1990); sand and loam 0.43, clay 0.47 (Schaake 2000, VIC model docs): 0.4 | anchored |
| `seep` | m/s | hydraulic conductivity of the loose ground: the pore liquid flows by Darcy's law at `seep` x the slope of its table through the thickness it fills | a loam passes 1.3 cm an hour, 3.6e-6 m/s (Rawls et al. 1992; Schaake 2000 gives 2.0 cm/h); sand 5.8e-5, clay about 2e-7 (from memory) | anchored |
| `field` | share | the share of its pores the loose ground holds against gravity (field capacity): only liquid over it sinks to the bed's store (`flow::drain`) and flows through the pores; ground at it or wetter evaporates at the open rate | a loam holds 0.25 to 0.32 of its volume at field capacity in 0.47 of pores (Saxton and Rawls 2006, as tabled by the Minnesota stormwater manual; checked): 0.28 / 0.47 = 0.6 | anchored |
| `bed_depth` | m | depth of the weathered zone of the bed under the loose ground: the store the drained liquid fills (`Class::Deep`) | weathered basement is 10 to 60 m deep (Chilton and Foster 1995, from memory): 30 | anchored |
| `bed_porosity` | share | the share of that zone that holds and gives up liquid (its specific yield) | weathered basement 0.01 to 0.05 (from memory): 0.02 | anchored |
| `bed_seep` | m/s | hydraulic conductivity of that zone: the store flows by Darcy's law at `bed_seep` x the depth it fills x the slope of its table | saprolite aquifers pass 4 to 50 m² a day through their thickness (Western Australia; checked in part); 3e-6 m/s through 30 m is 7.8 m² a day | anchored |
| `hill` | m | half the spacing of the channels inside a cell (a hillslope, L = 1 / (2 x drainage density)): the bed's store and the loose ground over field capacity drain to them down the cell's slope | drainage densities of 1 to 5 km per km² in humid lands (Horton 1945; Strahler 1957; from memory) give 100 to 500 m; 2 km per km²: 250 m | anchored |

## Rules chosen for a run

These are not numbers of the world and not rows of `Physics` or `Config`, but they were picked so a run came out a certain way (handover 2, lesson 3), so they are listed here and counted like tuned numbers. **Their number must not grow: three, held by a test (`core/tests/numbers.rs`).** A new one replaces one of these or is not added. Where one is also a row below (the roster's stores), the tuned count already has it.

| rule | what it does | chosen for (run) | where |
|---|---|---|---|
| the hand-set brain biases at the start (the default in `ew run` and `--roster`) | animals are placed with hand-set brain biases (`Seeding::hand`), not neutral brains; `--neutral-eaters` for the neutral start | nearly all neutral-brained founder eaters died in their first year, a lottery | `worldgen::life::Seeding::default`, `founder_lineage` |
| the roster's stores (`Kind::store`: 30% of mass, 60% for the two big slow eater kinds) | the fuel a starting member holds | seed 1, 64x64 roster: with half the store the two big kinds died out in their first decades | `worldgen::life::KINDS` |
| the fixing founders (`worldgen::life::FIXERS` 0.1 and `FIX_GENE` 9.8, rows below) | a tenth of the starting light-eaters carry the gene of the reaction that fixes the air's nitrogen; `--fixers 0` for the control | with one gift of soil nitrogen and no input the plants collapsed. The share is anchored to real floras, but it was wanted after a run collapsed | `worldgen::life` |

## Constants outside the structs that act like free numbers

These are in the code, not in `Physics`. They are not covered by the test. Numerical safeguards (`T_FLOOR`, `MAX_DT_K`, `NEGLIGIBLE`...) and the schedules of the slow work (`REFRESH_MONTHS`, `REACT_MONTHS`...) are left out: they change how fast or how exactly the laws run, not the laws' sizes.

| name | unit | fact | anchor (source) | mark |
|---|---|---|---|---|
| `WITHIN` (0.5, `core/src/body/genes.rs`) | standard deviations | children may lie this far beyond `alike` (in the spreads of both) and still join a node; nodes of a full cell merge within it | run: small enough that nodes of two kinds never join: a speed tool must not mix genes that individuals would never mix | tuned |
| `MIN_INTERVAL` (5, `core/src/body/genes.rs`) | days | the shortest time between a body's updates | bounds how often the fastest bodies update | speed |
| `MAX_INTERVAL` (360, `core/src/body/genes.rs`) | days | the longest time between updates | one year | anchored |
| `body::WET_TISSUE` (0.75) | share | the liquid share of living tissue: a kg of body matter makes 1 / (1 - this) kg of living body (`Body::built`), whatever liquid the body holds | lean animal tissue is 0.7 to 0.75 liquid, herbs 0.8 to 0.9 (from memory) | anchored |
| `pcore::units::T_SURF` (290) | K | typical surface temperature: where the starting kinds' reactions, the richness checks and `ew chem` look at phases | the same as `t_cat` | anchored |
| `MOVE_REF` (30 kg) and `MOVE_EXP` (-0.32), `core/src/params.rs` | kg; none | the mass at which `c_move` is the cost of moving, and the exponent of the cost of transport in the mover's mass (`Physics::walk`) | Taylor et al. 1982 (see `c_move`) | anchored |
| `Kind::store` of the roster (0.3; 0.6 for the two big slow eater kinds) | share of mass | the energy-rich solid a starting member holds as fuel; the rest is liquid and body matter | run: see "the roster's stores" above | tuned |
| the roster's genes (`INTERVAL`, `COPY_MASS`, `STRENGTH`, `CHILD`, `ALLOC`, `LEAF` in `KINDS`) | days, log10 kg, share, log10 share, share | when each kind updates, the size it is mature at, its strength, the size of its children and the share of what a mature body builds that goes to them | run: set so each kind lives its first years; `CHILD` (-0.523) and `ALLOC` (0.8) are the same in every kind and evolve | tuned |
| `Kind::year` (0, 10, 20, 24) | years | when each starting kind is placed after the first life | run: once what it eats has spread | tuned |
| `Seeding::default` (20 rounds), `ROW_SPRAY` (0.2, the row-bias step) and `GENE_SPRAY` (genes x3), `worldgen/src/life/seeding.rs` | none | how much a start is sprayed with mutation | run: one generation's step is too small for selection to pick from in the time a start has | tuned |
| `worldgen::life::FIXERS` (0.1) | share | the share of the starting light-eaters that carry the fixing gene (an initial condition; `--fixers` sets it) | the legumes are about 19,000 species, 7% of flowering plants (checked in part: Judd et al. 2002; Christenhusz and Byng 2016), not all of them fix, so about 5%; 10% is within 2 times | anchored |
| `worldgen::life::LIGHT_X` (2.26) | log10 | how much more light catalyst a founder light-eater is given than the kinds' number (9) | the light reaction's contact in a body is its liquid; a founder (2 kg, 20-day interval) exchanges 182 times its liquid, and log10 182 = 2.26, so its own light reaction goes as the number 9 was set for | anchored |
| `worldgen::life::FIX_GENE` (9.8) | log10 | the speed-up of the fixing reaction in those founders | the speed a growing plant needs: it turns its matter over in about half a year, 3e-9 mol of the mineral per kg and second | anchored |
| `RADIUS_M` (6.371e6, `worldgen/src/terrain.rs`) | m | the radius of a globe (`ew run --globe`): with `--size` rows, a cell is pi x radius / rows from north to south | the mean radius of Earth | anchored |
| `LAND_SHARE` (0.29, `worldgen/src/terrain.rs`) | share | the share of a globe's area its starting level leaves above 0 m (an initial condition) | Earth's land is 29% of its surface | anchored |
| `DRAIN_SLOPE` (1e-3, `worldgen/src/terrain.rs`) | m per m | the least slope of the floor of a closed basin of the starting terrain once it is raised to its outlet (an initial condition) | the slope of a lowland river is 1e-4 to 1e-3 (Leopold, Wolman and Miller 1964; from memory) | anchored |
| `BUILT_SHARE` (0.02, `worldgen/src/life/make.rs`) | share | the keep gene of each solid bodies build besides their matter, in every founder: an initial condition, the same for every kind | none: chosen small before any run, so selection can find what each solid is worth | tuned |

## Chemistry data that bodies live by (not rows of `Physics`)

Chemistry 0 (`worldgen/src/chem0.toml`) is data, not a struct of numbers, so it is not in the tables above. These rows are the numbers that decide what bodies can do and the liquid's store of the carbon gas. The budget does not count them: they are set from facts of matter.

**Enthalpies.** A value quoted here as measured is for the state the substance is in at 290 K. The data's `hf` is the solid's: a liquid's is the measured one less its fusion enthalpy, a gas's also less its vaporisation enthalpy (laws 2.1). `Chemistry::dh_at` gives a reaction's heat in the measured states (`ew chem` prints it). Molar masses are sums of the file's `element_masses`.

| name | value | fact | anchor |
|---|---|---|---|
| substance 22, body matter | C120H196N6O106Ca (3,456 g/mol), data hf -20,315.8 kJ/mol, cp 4,266 J/(mol K), 1,500 kg/m³, melts at 1,500 K | what bodies are built of (`Chemistry::build`), with no heat in or out: hf = 118 x -212 + 3 x -365 - 1,926.8 + 27 x 286. Burning it gives 16.0 MJ/kg. C:N 20; 1.16% rock element by mass | like cellulose (1,500 kg/m³, 17.5 MJ/kg); foliage C:N 43 (checked: McGroddy et al. 2004), animals about 6; shoots hold 0.1 to 5% of the rock element (checked: White and Broadley 2003) |
| substance 24, the nitrogen mineral | N2H4O3 (80 g/mol), hf -365 kJ/mol as measured (data -370.9), a liquid that melts at 255 K and never boils | the nitrogen of the soil, the only form a body is built of; a salt that lives dissolved, so it is written as a liquid: it moves with the liquid by its share | ammonium nitrate: hf -365.6, cp 140 J/(mol K), very soluble; its solution freezes near 255 K (the eutectic with water) |
| soil, `worldgen::Init::soil` | 0.357 mol of 24 per m² of open land, given when life starts (year 2, `worldgen::life::soil`): 10 g of nitrogen per m² | the start of the nitrogen of the world | mineral nitrogen of a soil is 1 to 10 g per m² (the top of that: the world starts with no humus); checked in part for farm soil: 1.7 to 18 g per m² (CSIRO Soil Research SR9960949; spring counts in Europe and Turkey) |
| `Chemistry::build`, growth | 118 of the solid + 3 of the nitrogen mineral + 1 of the rock salt -> 1 of body matter + 27 of the liquid; no rate | what `copy::grow` runs, paid by `c_copy` of work per kg of matter; the inputs no reaction burns are the minerals (`Chemistry::minerals`: 24, 23) | chemistry data |
| reaction 1, burning the energy-rich solid | CH2O + O2 -> CO2 + H2O, k 1.72e4 /s, Ea 7,760 K | the open-world decay of dead plant matter: 8.7e-9 /s at 290 K in open air, 0.27 a year. Founders' burning genes are lower by `LITTER_X` (0.602), so a body's own burning stays as it was set | litter decay constants: forest floor 0.25 a year (checked in part: Olson 1963, Ecology 44: 322), leaves 0.3 to 1, wood 0.02 to 0.1 (from memory) |
| reaction 21, burning body matter | C120H196N6O106Ca + 118 O2 -> 119 CO2 + 92 H2O + 3 N2H4O3 + CaCO3, k 2.28e9 /s, Ea 9,000 K, orders 1 and 6 (`Reaction::ord`) | the open-world decay of dead body matter: 0.2 a year at 290 K, a little slower than the solid's 0.27; the nitrogen goes back as the mineral and the rock element as lime | Olson's litter constants (see reaction 1); bodies speed it up with a catalyst gene |
| reaction 24, fixing the nitrogen of the air | N2 + 14 CH2O + 14 O2 -> N2H4O3 + CO + 13 CO2 + 12 H2O, k 8e-8 /s, Ea 6,600 K, orders 1; dH -6,055 kJ | the fixing step needs +490 kJ, paid by burning 14 units of the solid (6 g C per g N); it keeps that energy in what it makes, so it is no burning (`Chemistry::keeps`) | nitrogenase spends 16 ATP per N2, 488 kJ (checked); symbioses spend 4.7 to 6.6 g C per g N (Layzell et al. 1979) |
| reaction 25, the mineral breaking down hot | 2 N2H4O3 -> 2 N2 + 4 H2O + O2, k 2.6e9 /s, Ea 20,000 K; dH -414 kJ | where it is hot (burning matter) the nitrogen leaves as the gas, to the air: 3e-21 /s at 290 K, 1e-3 at 700 K, 5 /s at 1000 K | the salt breaks down above 450 K with an energy near 170 kJ/mol (20,000 K); real fires lose most of the nitrogen of what they burn |
| `Chemistry::air`, the cost of catalysts | the starting air's mole fractions | the cost of a catalyst (`Body::prepare`) counts every gas of a reaction at its share of the starting air to the power of its order | without the shares, the 6th power of the oxygen-like gas would make the enzyme for burning body matter cost 1,800 times that for the solid for the same speed-up |
| substance 23, the salt that exists only dissolved | Ca(HCO3)2 (162 g/mol), hf -1,926.8 kJ/mol as measured (data -1,929.37), cp 194 J/(mol K), melts at 271 K, never boils, 30 cm³/mol | holds the liquid's carbon beyond the free gas; the second mineral of growth; in a body it falls out as lime in hours to days | physical: hf of Ca2+(aq) and 2 HCO3-(aq), so dissolving lime has dH -40.3 kJ/mol (real -40.1); cp by Kopp's rule (anchored) |
| reaction 22, dissolving: CaCO3 + CO2 + H2O -> salt | k 1.62e-2 /s, Ea 2,800 K | the liquid takes the air's gas into the salt; its contact is the rock (the least of its inputs) | anchored: the Earth's gross air-sea exchange of this gas, 80 of 589 PgC a year (Ciais et al. 2013), over 71% of the planet; Ea from gas transfer rising 1.4 times per 10 K |
| reaction 23, settling: 3 salt -> 3 CaCO3 + 3 CO2 + 3 H2O | k 1.8e12 /s, Ea 7,647 K | the way back, written for three of the salt, as its real rate takes three ions. The balance (`Reactor::balance`) is S³ = (k22 / k23) exp(4,847 / T) x R x x_gas x W (W + S): S the salt, W the liquid, R the rock (mol), x_gas the gas's share of the air | physical: Ea(23) - Ea(22) = dH / R = 4,847 K. Anchored: 1.0 mol of salt per m³ at 288 K, 400 ppm, 500 m deep (the sea holds 2.0 mol/m³ of dissolved carbon) |
| `Chemistry::builds` | the frame: 6 of the solid -> 1 of 25 + 1 of the liquid (no heat in or out); lime: 1 of the rock salt -> 1 of lime + 1 of the gas + 1 of the liquid (+40.3 kJ, the gas to the air); no rates | the other solids `copy::grow` builds, by `c_copy` of work a kg, each in the share the body's keep gene of it sets (`Body::share`). They pay no upkeep | chemistry data |
| substance 25, the frame | C6H10O5 (162 g/mol), hf -986 kJ/mol (no heat in or out when a body builds it from 6 of the solid), cp 210 J/(mol K), 1,500 kg/m³, melts at 1,500 K | a solid bodies can build, of the energy-rich solid; no nitrogen and no rock element | cellulose: 17.5 MJ/kg, about 1,500 kg/m³ in the cell wall, 40 to 50% of plant dry mass (from memory) |
| reaction 26, the frame rots | C6H10O5 + 6 O2 -> 6 CO2 + 5 H2O, k 1.72e4 /s, Ea 7,760 K, orders 1 and 1; dH -2,805 kJ | the open-world decay of dead frame, 0.27 a year at 290 K, the `k` and `Ea` of reaction 1 | Olson's litter constant (see reaction 1): litter is mostly cellulose and lignin |
| reaction 27, an enzyme splits the frame | C6H10O5 + H2O -> 6 CH2O, k 9.6e-4 /s, Ea 7,760 K; no heat in or out | nothing without a catalyst: 2.3e-15 /s at 290 K; a gene of about 8.5 splits what a body holds within an interval; no founder has it | the spontaneous hydrolysis of the bond between sugar units, 4.7e-15 /s at 25 °C (Wolfenden, Lu and Young 1998, from memory) |
| `modulus` of substances 22, 25 and 10 | 1 MPa, 30 GPa, 70 GPa | Young's modulus of what bodies build: soft tissue with its liquid, the frame, lime (`Body::height`) | soft tissue 0.1 to 10 MPa, the middle on a log scale; tree stems about 10 GPa along their fibres at 500 kg/m³, so the cell wall at 1,500 about 30; calcite 70 to 80 (from memory) |
| `rupture` of substances 22, 25 and 10 | 1, 120 and 100 MPa | the stress each breaks at: the energy to break a body (`Body::breaking`) | soft tissue 0.1 to 10 MPa; tree stems 40 to 50 MPa in compression at 500 kg/m³, so the cell wall about 120; marble 70 to 140, shells 100 to 300 (from memory) |

## Real figures to compare runs with

Not rows of the model: real sizes that a run's results can be held against.

- **Biomass of large plant-eaters.** Hempson et al. 2015 (Science 350: 1056): reconstructed African large herbivores 53 to 11,810 kg/km² over 1° squares, typical about 1,700 kg/km² (without elephants). Damuth 1981 (Nature 290: 699; from memory): density = 4.23 m^-0.75 per km² for plant-eaters, about 250 to 450 kg/km² for 50 to 500 kg animals.
- **Remaining lives of adults.** Bison: adult cows survive 0.92 to 0.95 a year (Fuller et al. 2007, J Wildl Manage 71; Yellowstone report 2023), so 12 to 20 years. Deer: wild adults survive 0.83 to 0.91 a year with no hunting (a Wisconsin study, quoted), so 5 to 11 years. Mouse: a farm study found apparent survival 0.54 a month (it counts leaving too). Rabbit: no source found.
- **P/B a year** (mouse 3 to 5, rabbit 1 to 2, deer 0.3 to 0.5, bison 0.15 to 0.25): from memory, not checked. Banse and Mosher 1980 (Ecol. Monogr. 50: 355) and Humphreys 1979 (J Anim Ecol 48: 427) are the sources to read.

## Counts

The 108 rows of `Config` and `Physics` are their 108 fields.

| mark | rows | which |
|---|---|---|
| physical | 4 | `sun`, `tilt`, `gravity`, `rho_body` |
| anchored | 46, and 7 outside the structs | 46 fields (among them `mix`, `light_yield`, `wind_ground`, `forced`, `leaf_size`, `c_build`, `c_move`, `take`, `dormancy`, `porosity`, `seep`, `field`, `bed_depth`, `bed_porosity`, `bed_seep`, `hill`, `carry`, `wear` and `creep`) and the constants `MAX_INTERVAL`, `WET_TISSUE`, `T_SURF`, `MOVE_REF`/`MOVE_EXP`, `FIXERS`, `FIX_GENE` and `LIGHT_X` |
| tuned (run or reason named) | 10, and 6 outside the structs | thresholds of who is alike (`alike`; outside: `WITHIN`), set for an outcome (`lat_south`, `lat_north`, `rain_time`, `shelter`, `c_leaf`, `c_brain`, `sweep`, `c_handle`, `c_basal`; outside: the roster's stores, genes and years, the spray of the start, `BUILT_SHARE`) |
| tuned, first guess | 36 | 32 free numbers (`wind`, `cloud`, `gas_depth`, `c_firm`, `c_learn`, `c_signal`, `rho_pass`, `shape_noise`, `signal_noise`, `blend`, `learn_most`, the Bond numbers `joint` to `afloat`, the mutation numbers `new_lineage` to `new_noise`, and the Flow numbers `hide` to `hold`) and the four `c_hidden`, `c_memory`, `c_seen`, `c_sight`, which are the split of `c_brain` and not four numbers |
| switch | 3 | `nodes`, `two_parents`, `null` |
| speed | 8, and 1 outside the structs | `phys_days`, `air_hours`, `flow_days`, `wake_share`, `sleep_days`, `node_days`, `crowd`, `sample_every`; outside: `MIN_INTERVAL` |
| container | 1 | `phys` |

Rows marked tuned in all: 52 (46 in the structs, 6 outside). The test holds that count at its budget or below (handover 14.3). Rules chosen for a run: 3, held by the test too.
