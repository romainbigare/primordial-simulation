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
| `lat_south` | degrees | latitude of the map's south edge | where the surface is 287 K, near the global mean: on Earth at about 38° to 40° in both hemispheres (zonal means, from memory); on the model's globe without life at 37.5° N and 44° S | anchored |
| `lat_north` | degrees | latitude of the map's north edge | `lat_south` + 2.3: the island spans 256 km | anchored |
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
| `mix` | m²/s | mixing of neighbouring air columns by eddies; in a day they even out the air over sqrt(`mix` × day), the side of the air's blocks (laws.md 1.6) | the eddy diffusivity of the weather systems of middle latitudes, about 1e6 (from memory: the diffusive climate models of North 1975 and Held 1999); the air then crosses the map in about a day, and a block is about 300 km | anchored |
| `rain_time` | s | time in which vapour over saturation rains out | run: a cell sees the mean air of its node, so with the life of real cloud water, an hour or two, its rain falls only on the coldest cells (64 x 64, no life: 27% of the land gets rain, 70% of it on a tenth); at 1 day 85% of the land gets rain; the island with life gets 1.71 m a year on its land, within Earth's 0.76 m x/ 3 (Trenberth et al. 2007; `the_key_numbers_stay_in_their_ranges`); weather systems that make rain live about a day (Houze 2004, from memory) | tuned |
| `cloud` | mol/(m² s) | condensation at which a cloud reflects half of what its droplets can (scattering depth 1) | none found; 1.2e-2 is a light rain of 0.8 mm an hour (my arithmetic) | tuned, first guess |
| `cloud_ir` | none | optical depth to heat radiation of a cloud at the condensation `cloud` | a layer of drops reflects x / (1 + x) at scattering depth x = √3 (1 − g) τ / 2 = 0.13 τ, g = 0.85 (Lacis and Hansen 1974), so `cloud` is τ = 7.7; drops of 10 µm absorb heat radiation with a depth of 0.87 τ (130 m²/kg, Stephens 1978, against an extinction of 3 / (2 ρ r) = 150 m²/kg): 6.7 | anchored |
| `carry` | m³/J | loose solid volume moving liquid carries off a cell per J its runoff loses falling to the neighbour | rivers carry 15 to 20 Gt of sediment a year (Milliman and Syvitski 1992; Wittmann et al. 2020: 15.2) with a runoff of 4e13 m³ a year falling a mean 840 m, 3.4e20 J: 2e-11 m³/J (checked). The yield, not Bagnold's capacity (250 times more): nothing holds the model's loose ground | anchored |
| `wear` | m³/J, times 1/bond strength | bonded volume broken per J dissipated | the same 2e-11 m³/J for the starting bed: over its sum of volume share / bond strength, 1.39, it is 1.4e-11. Bedrock lowers about as the land does, 0.05 mm a year (checked: Wittmann et al. 2020; Portenga and Bierman 2011) | anchored |
| `regrow` | m/s | speed at which bodies spread back into a cleared lane, from each side, under dense cover (times cover / `rho_pass` below that) | 1 nm/s is 3 cm a year, the slow end of the spread of a ground cover by runners (my arithmetic, unchecked) | anchored |
| `creep` | m/s per unit slope, times 1/hardness | loose solids creeping down a slope: a flux of D x slope, D = creep x softness x thickness | hillslope diffusivity 1e-3 to 1e-2 m²/yr (Roering et al. 1999; Fernandes and Dietrich 1997; Hanks 2000; checked in part in Whipple's MIT notes, 2004); 3e-3 m²/yr on the starting loose ground (1 m, softness 1.53) is 6.3e-11 m/s | anchored |
| `hard_min` | none (a hardness) | added to a solid's hardness before its softness, 1 / (hardness + this), is taken: a solid of hardness 0 creeps about 20 times as fast as one of hardness 1 | none found | tuned, first guess |

| `null` | on/off | a null run: no learning, no imitation, born rows and brain bases never change; genes still mutate | the run to compare a seed with | switch |
| `phys` | (a `Physics`) | holds the numbers below | | container |

## Speed settings

Fields of `Config` that say when the model may save time (mark **speed**). The test that ×2 and ÷2 change no key number beyond its noise comes with the first law that uses each one.

| name | unit | fact | anchor (source) | mark |
|---|---|---|---|---|
| `wake_share` | share | a sleeping node wakes before its alarm when one of its amounts has grown by more than this share of what it held when it fell asleep (`clock::Sleep::watch`) | the share of its inputs a calm node's own reactions may use while it sleeps (`change`, `CALM`, 0.05) | speed |
| `sleep_days` | days | the longest a ground or air node's reactions sleep before they run for the time since (`change::step`); 0: every node reacts every month | a season: a node does not react through a whole season at the temperature of another | speed |

## Physics

The three costs of keeping tissue (`c_basal`, `c_strength`, `c_learn`) are per kg of living body (`Body::built`) of a kind that matures at `m_ref`, times (copy mass / 1 kg)^-0.25, so adults of every kind pay Kleiber's mass^0.75 (laws 6.2). Where a row says "set at the eater founder", the number is what the starting eater (it matures at 44 kg) pays.

### Change and Flow, as bodies feel them

| name | unit | fact | anchor (source) | mark |
|---|---|---|---|---|
| `light_depth` | m | liquid thickness over the cover that holds back 63% of the light reaching it (`cover::step`) | clear sea water: broadband light falls by e in 1 to 20 m; 10 m is the clear end (unchecked) | anchored |
| `light_yield` | share | the share of the light a light-driven reaction takes that it stores; the rest becomes heat | real photosynthesis stores at most 4.6% (C3) to 6% (C4) of the sunlight, 1 to 2% in crops over a season (Zhu, Long and Ort 2008, from memory): 5% | anchored |
| `gas_floor` | mole fraction | share of a light reaction's gas in the air at which it stores nothing (`Physics::gas_factor`) | the compensation point without day breathing, Γ* = 42.75 µmol/mol at 25 °C (Bernacchi et al. 2001) | anchored |
| `gas_ref` | mole fraction | share of that gas in the air at which a light reaction stores `light_yield` | the air of the measurements `light_yield` comes from, about 380 µmol/mol (Zhu, Long and Ort 2008) | anchored |
| `wet_depth` | m | depth of the key liquid of a cell, standing or in the pores of its ground, at which the ground counts as half wet (`Physics::wetness`: the cover's activity goes by it) | a film or puddle of about 1 cm | anchored |
| `t_cat` | K | the temperature at which the costs are anchored; upkeep goes from it by the bare rate law of its main burning reaction (`Body::upkeep_pace`), and what a body takes and builds per day by that rate law as the body catalyses it, its rate constant at T over that at `t_cat` (`Body::pace`) | a typical surface temperature, 290 K (`pcore::units::T_SURF`); the tempo of life follows the Boltzmann factor (Gillooly et al. 2001, Science 293: 2248, E about 0.6 eV; checked: Ernest et al. 2003). A cost or a rate of taking measured at another temperature T is the measured value over the eater founder's pace at T: 4.80 at 310 K | anchored |
| `frost_k` | K | the spread of a cell's microclimate: below a freezing point (a body's key liquid's melting point, the cover's frost gene `cover::FROST`) the share min(1, (point − T) / `frost_k`) × activity freezes and dies | tissue freezes within a few K below its point (from memory) | anchored |
| `c_firm` | K | each this many K of frost tolerance (the cover's frost gene below the key liquid's melting point) adds the upkeep of its light capacity once more (a cell that stands the cold needs more and stronger bonds) | none found; at 253 K the light capacity costs twice as much | tuned, first guess |
| `c_strength` | J per kg of body per day at strength 1 | upkeep of strength, on top of the basal | the resting muscle: skeletal muscle burns about 13 kcal per kg a day at rest in a man, about half his body's mean (Elia 1992, from memory); a body of strength 1 taken as muscular as a mammal, 40% muscle: 22 kJ per kg of body a day at 70 kg and 310 K, 63 kJ for a kind of 1 kg (Kleiber's mass^-0.25), 1.31e4 at `t_cat` (/4.80) | anchored |
| `c_basal` | J per kg of body per day | basal upkeep every body pays; strength and catalysts are on top | Kleiber's resting metabolism, about 3e5 J a day per kg^0.75 at a mammal's 310 K, 6.3e4 at `t_cat` (checked: Kleiber 1932 and 1947; Smil 2000, Nature 403). Plants' maintenance breathing, 15 to 25 mg of sugar per g of dry mass a day (Penning de Vries 1975; McCree 1970; from memory), is 6e4 to 1e5 J per kg of living body a day at a quarter dry matter and 20 to 25 °C (my arithmetic): about the same | anchored |
| `c_leaf` | J per m² per day | upkeep of light-capture area | run: so the starting plants pay what they were set to pay. A leaf's own breathing is about 4e4 J per m² per day (1 µmol of carbon gas per m² per s, my arithmetic), 20 times more | tuned |
| `c_brain` | J per kg of a member of `m_ref` kg, per decision | cost of one decision of the starting brain (12 hidden, 6 memory, 3 seen, sight 1) | none found; every body decides once a month, so it is a monthly cost of the brain. The three rows below split it (sight costs nothing: it reads another level of the same features, with no more inputs or rows) | tuned |
| `m_ref` | kg | the one reference mass: the member at which a per-kg number is per kg (brain cost, `take`, `sweep`, the heat gene, the costs of tissue) | a unit, not a size: 1 kg | anchored |
| `c_hidden` | J per kg of a member of `m_ref` kg, per decision | cost per hidden unit | derived: a share of `c_brain` (12 x 5e3 = 6e4 of 8.5e4) | tuned, first guess |
| `c_memory` | the same | cost per memory number | derived: a share of `c_brain` (6 x 5e3/3 = 1e4) | tuned, first guess |
| `c_seen` | the same | cost per body perceived | derived: a share of `c_brain` (3 x 5e3 = 1.5e4) | tuned, first guess |
| `c_learn` | J per kg of body per day at learning rate 1 | upkeep of learning | none found; set at the eater founder | tuned, first guess |
| `e_life` | J per kg | energy a kg of body turns over in its upkeep before it is worn out, at a repair gene of 0 (a body lives 10^gene times this) | Rubner 1908: many kinds that move turn over about 1,000 kJ per gram of body in a life | anchored |
| `c_repair` | share | share of upkeep that is repair at repair 0: upkeep x (1 - c + c 2^gene), so each unit of the gene doubles it and every value of the gene has a cost | remaking a body's own matter takes 20 to 30% of resting upkeep in life | anchored |
| `dormant` | share | the least activity a dormant body or slot of cover wears at, a share of its active rate (`body::live`, `cover::grow`): a dry body pays almost no upkeep but still ages | dry seeds and spores live years to decades (Walters 1998, from memory) where the active life of the same tissue is weeks to months: about 50 times longer, a floor of 0.02 | anchored |
| `dormancy` | none | how steeply a body's activity falls as it dries: activity = min(1, l / l*)^n, l the key liquid it holds, l* the level it keeps (`Body::activity`); must be above 1 (`Config::validate`) | Harrington's rule (checked: Harrington 1963 and 1972): seed life doubles for each 1% less moisture between about 14% and 5%, so a 3-fold drop is 2^9 = 512 times less activity: n = ln 512 / ln 3 = 5.7 | anchored |
| `lethal` | share | share of its body whose loss kills a member: a hit that tears that much kills it; a starving member that lost that much lived through it with even chances (`live::starved`) | starving mammals die near the loss of half their body mass (from memory); a wound that takes half the body kills | anchored |

### Choose

| name | unit | fact | anchor (source) | mark |
|---|---|---|---|---|
| `c_move` | J per kg per m, for a member of `MOVE_REF` (30 kg); others by `Physics::walk` | energy to move one kg one metre on the flat | the real cost of transport, 10.7 m^-0.32 J per kg per m, 3.6 at 30 kg (Taylor et al. 1982; checked in part: the exponent -0.316, J. Exp. Biol. 215: 2349, 2012); 3 | anchored |
| `rho_pass` | kg/m² | body mass per area that doubles the cost of entering a cell and of handling what is taken there | none found | tuned, first guess |
| `lane` | m | width of the passage a mover clears through bodies | the width of a body or a path, about 1 m | anchored |
| `layer` | m | the bodies of a cell must fit in a layer this thick over it | a tall plant, about 10 m; a ceiling far above real cover (a forest holds under 100 kg/m², this allows 10,000) | anchored |
| `rho_body` | kg/m³ | density of bodies | living matter is about the density of liquid, 1,000 | physical |
| `take` | share per day | most a body can take per day of the mass it takes by, per kg at `m_ref` | real food intake is 1 to 5% of body mass a day (unchecked), measured on mammals at their body's 310 K; at `t_cat` 1/4.80 of the value at 310 K, as `c_strength`. At 310 K a 57 kg eater takes at most 1.8% of its mass a day, about 2.7 times Kleiber's resting need: a real appetite (my arithmetic) | anchored |
| `intake_exp` | none | what a member takes, sweeps, receives and builds, and its leaf, go with its living body (`Body::supply`) to this power | Kleiber's 0.75 (checked: production of 305 mammals goes with mass^0.755, 95% range 0.73 to 0.78, Ernest et al. 2003, Ecology Letters 6: 990); the supply in West, Brown and Enquist's growth law (2001) | anchored |
| `sweep` | m² per kg per day | area a body works per day per kg of its mass that works, at `m_ref` | run: set for eaters to live, not a rate measured on an animal. A 30 kg body (12.8 kg-equivalents) works about 390 m² in a month (30 days) at `t_cat` | tuned |
| `c_handle` | J per kg | energy to handle one kg taken or put | none found; set for a run | tuned |
| `keep_min` | times | a body keeps this many times the safe minimum of each store it gives away (`choose::give`) or puts out to parts (`bond::verbs`) | none found: a margin over the minimum below which it dies | tuned, first guess |
| `shape_noise` | none | spread of the shape of a part made, around its mean | none found | tuned, first guess |
| `discount` | none | weight of well-being one month (one update) ahead against well-being now | the usual 0.9 of learning machines | anchored |
| `plan_weight` | per unit of well-being | weight in a move's score of the critic's gain from the step over staying, γ(V(s') − V(s)) (`choose::plan`) | none found: 1, a unit of well-being as a unit of log-odds; well-being and a score are not one unit, and the actor learns the same gain | tuned, first guess |
| `blend` | share | share of the way a watcher's rows move toward the watched one's | none found | tuned, first guess |
| `learn_beta` | share | weight of the newest value in a learner's running mean squares: of a learned row's steps, and of the critic's errors | the usual 0.1 of a running average in learning machines | anchored |
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
| `grab` | share | the work of taking hold of a part, as a share of the work of handling its mass (`c_handle`) | none found | tuned, first guess |
| `grip` | share of weight | force of a body of strength 1 | none found; animals push and pull about their own weight (unchecked) | tuned, first guess |
| `bump` | share of weight | push of a body moving through a cell on each structure there; also the push its own force must give for it to move (`choose::go`) | none found; the starting plants (force 0.025 of their weight) stay put | tuned, first guess |
| `deep` | m | in liquid deeper than this a body the liquid holds up, with the structure it holds, floats (`bond::afloat`) | none found | tuned, first guess |

### Copy

| name | unit | fact | anchor (source) | mark |
|---|---|---|---|---|
| `c_copy` | J per kg of body matter | work to build one kg of body matter (`copy::grow`); storing fuel costs none, and a child costs its parent none (it is given matter already built) | growth in life keeps 10 to 20% of the energy it eats, so a kg of body costs about 1e7 J; a kg of matter then takes 27.3 MJ of fuel, 62% efficient (my arithmetic) | anchored |
| `c_build` | J per day per kg^0.75 of body | most a body pays into building body matter per day: the rate at which it grows and rebuilds what it gave its children (`copy::grow`) | P/B (what bodies make a year over their standing body) of bodies at the ground's temperature, as all kinds here are: herbs turn their standing crop over about once or twice a year (from memory). P/B x 25% dry matter (`WET_TISSUE`) x `c_copy` / 365 / m^0.75: 6,800 to 13,700 at 1 kg, 8,100 to 16,300 at a 2 kg light-eater (the mass^0.75 checked, Ernest et al. 2003). 1e4 lies inside. Warm mammals' P/B is measured at 310 K and is not used (decision 51) | anchored |
| `row_chance` | chance per child | a child's born brain rows change | none found | tuned, first guess |
| `row_noise` | none | step of that change per generation, times the body's mutation size (a child that changes takes a step bigger by 1 / sqrt(`row_chance`)) | none found | tuned, first guess |
| `new_noise` | none | spread of the weights a bigger brain adds (none in a null run) | none found | tuned, first guess |
| `alike` | mutation steps | bodies whose genes differ by at most this in every gene are alike: they mix, and nodes of different lineages merge | run: 4 steps. It says who mixes, so what a kind is: not only a speed setting. To be checked against individuals at M4 | tuned |

### The cover (`cover`)

| name | unit | fact | anchor (source) | mark |
|---|---|---|---|---|
| `c_light` | share | upkeep of the cover's light capacity: per kg and day, this share of what the capacity (`cover::LIGHT`, times the moisture need) could store in a day | the enzymes of the light reaction are a quarter to a half of a leaf's protein (Rubisco, from memory) and leaf protein turns over in about a week to a month: keeping them costs about a tenth of what they can make (my arithmetic, unchecked) | anchored |
| `cover_mut` | steps per generation | the size of one generation's mutation of the cover's genes, times each gene's step (`cover::RANGE`) | the founders' mutation gene of bodies, 10^-1.2 = 0.063 | anchored |
| `cover_hard` | kg/kg | the solid the cover builds of a mineral (`cover::HARD`, substance 9, ≈ lime) per kg of its body matter | the founders' share of the solids bodies build besides their walls, 0.02 (`worldgen::life::make`, `BUILT_SHARE`): plants hold a few per cent of their dry mass as the rock element's salts (from memory) | anchored |

### Flow, as ground cover and moving liquid feel it

| name | unit | fact | anchor (source) | mark |
|---|---|---|---|---|
| `hide` | m, three numbers | thickness of a layer that hides half of what lies below it, for the reflectance and the work on the bed: soft solid 0.2, liquid 0.5, loose 0.02 | none found | tuned, first guess |
| `soft` | share of melting point | a solid warmer than this share of its melting point is soft cover lying on top | none found; solids soften and sinter at 0.3 to 0.8 of their melting point in K (unchecked) | tuned, first guess |
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
| the hand-set brain biases at the start (the default in `ew run` and `--roster`) | animals are placed with hand-set brain biases (`Seeding::hand`), not neutral brains: they bite a body (2) before they take from the ground (0.5) and weigh the depth of standing liquid in a move by -1 (on ln(1 + depth / m)); `--neutral-eaters` for the neutral start | nearly all neutral-brained founder eaters died in their first year, a lottery; founders that walked into the sea drowned in their first year | `worldgen::life::Seeding::default`, `H_EATER`, `founder_lineage` |
| the roster's stores (`Kind::store`: 30% of mass, 60% for the two big slow eater kinds) | the fuel a starting member holds | seed 1, 64x64 roster: with half the store the two big kinds died out in their first decades | `worldgen::life::KINDS` |
| the fixing founders (`worldgen::life::FIXERS` 0.1 and `FIX_GENE` 9.8, rows below) | a tenth of the starting light-eaters carry the gene of the reaction that fixes the air's nitrogen; `--fixers 0` for the control | with one gift of soil nitrogen and no input the plants collapsed. The share is anchored to real floras, but it was wanted after a run collapsed | `worldgen::life` |

## Constants outside the structs that act like free numbers

These are in the code, not in `Physics`. They are not covered by the test. Numerical safeguards (`T_FLOOR`, `MAX_DT_K`, `NEGLIGIBLE`...) and the schedules of the slow work (`REFRESH_MONTHS`...) are left out: they change how fast or how exactly the laws run, not the laws' sizes.

| name | unit | fact | anchor (source) | mark |
|---|---|---|---|---|
| `body::MARGIN` (2) | none | well-being's margins count up to this: a key store's log share over its safe minimum, and what its reactions could release against its upkeep (`live` plans for this many upkeeps) | none found: a body twice its needs is taken as safe | tuned, first guess |
| `PHASE_MARGIN` (5, `core/src/body/mod.rs`) | K | well-being falls as a key liquid comes within this of freezing or boiling | none found: a few kelvin, as a body's temperature swings within a day | tuned, first guess |
| `EATER_KEEP_T`/`EATER_COOL_T` (278 and 315, `worldgen/src/life/kinds.rs`) | K | the temperatures the eaters' founders keep their bodies between (`KEEP_T`, `COOL_T`; initial conditions) | the melting point of chemistry 0's key liquid (273 K) plus, and the founders' catalysts' unfolding (`DENATURE`, 320 K) less, the margin in which well-being falls for nearness to a limit (`PHASE_MARGIN`, 5 K): an animal must keep its liquid from freezing and sweat before its enzymes unfold; at 250 K the keep gene did nothing and never moved, and the eaters froze in winter on the hills | anchored |
| `body::WET_TISSUE` (0.75) | share | the liquid share of living tissue: a kg of body matter makes 1 / (1 - this) kg of living body (`Body::built`), whatever liquid the body holds | lean animal tissue is 0.7 to 0.75 liquid, herbs 0.8 to 0.9 (from memory) | anchored |
| `body::MAX_KEEP` (0.95, `core/src/body/tissue.rs`) | share | the most share of a body its keep gene of a key liquid, or the solids it builds together, may make up (the level goes with share / (1 - share), so it must stay below 1) | the wettest bodies, jellies, are about 95% liquid (from memory) | anchored |
| `pcore::units::T_SURF` (290) | K | typical surface temperature: where the starting kinds' reactions, the richness checks and `ew chem` look at phases | the same as `t_cat` | anchored |
| `MOVE_REF` (30 kg) and `MOVE_EXP` (-0.32), `core/src/params.rs` | kg; none | the mass at which `c_move` is the cost of moving, and the exponent of the cost of transport in the mover's mass (`Physics::walk`) | Taylor et al. 1982 (see `c_move`) | anchored |
| `Kind::store` of the roster (0.3; 0.6 for the two big slow eater kinds) | share of mass | the energy-rich solid a starting member holds as fuel; the rest is liquid and body matter | run: see "the roster's stores" above | tuned |
| the roster's genes (`COPY_MASS`, `STRENGTH`, `CHILD`, `ALLOC` in `KINDS`) | log10 kg, share, log10 share, share | the size each kind is mature at, its strength, the size of its children and the share of what a mature body builds that goes to them | run: set so each kind lives its first years; `CHILD` (-0.523) and `ALLOC` (0.8) are the same in every kind and evolve | tuned |
| `Kind::year` (0, 10, 20, 24) | years | when each starting kind is placed after the first life | run: once what it eats has spread | tuned |
| `Seeding::default` (20 rounds), `ROW_SPRAY` (0.2, the row-bias step) and `GENE_SPRAY` (genes x3), `worldgen/src/life/seeding.rs` | none | how much a start is sprayed with mutation | run: one generation's step is too small for selection to pick from in the time a start has | tuned |
| `worldgen::life::FIXERS` (0.1) | share | the share of the starting light-eaters that carry the fixing gene (an initial condition; `--fixers` sets it) | the legumes are about 19,000 species, 7% of flowering plants (checked in part: Judd et al. 2002; Christenhusz and Byng 2016), not all of them fix, so about 5%; 10% is within 2 times | anchored |
| `worldgen::life::FIX_GENE` (9.8) | log10 | the speed-up of the fixing reaction in those founders | the speed a growing plant needs: it turns its matter over in about half a year, 3e-9 mol of the mineral per kg and second | anchored |
| `RADIUS_M` (6.371e6, `worldgen/src/terrain.rs`) | m | the radius of a globe (`ew run --globe`): with `--size` rows, a cell is pi x radius / rows from north to south | the mean radius of Earth | anchored |
| `LAND_SHARE` (0.29, `worldgen/src/terrain.rs`) | share | the share of a globe's area its starting level leaves above 0 m (an initial condition) | Earth's land is 29% of its surface | anchored |
| `DRAIN_SLOPE` (1e-3, `worldgen/src/terrain.rs`) | m per m | the least slope of the floor of a closed basin of the starting terrain once it is raised to its outlet (an initial condition) | the slope of a lowland river is 1e-4 to 1e-3 (Leopold, Wolman and Miller 1964; from memory) | anchored |
| `nodes::MAX_NODES` (8) | bodies per cell | past this many bodies after its monthly merge a cell keeps one young and one mature node of each starting kind in each room: the bodies of one kind, stage and room join the largest of them, whatever their lineage and brain sizes (`nodes::cap`) | run, 64 x 64 roster, 100 years: the most bodies in a cell is 10 to 12, of 5 or 6 kinds, and 13,280 bodies in all; it binds only where five or more kinds share a cell | speed |
| `choose::perceive::SMALL_KG`, `BIG_KG` (1 and 100) | kg | the members of the three classes of animals a cell's features give the mass of weigh below 1 kg, 1 to 100 kg, and more | none: fixed decades, chosen apart from the starting kinds' sizes, so the classes do not say which kinds eat which | tuned, first guess |
| `lineage::MAX_LINEAGES` (8) | lineages per starting kind | the most living lineages a starting kind keeps; past them a body whose brain sizes change joins the living lineage of its kind nearest in sizes (`lineage::resize`) | run, 32 x 32 default, seed 1, year 60 (4 / 8 / 16): the same world (one lineage: the default's eaters do not change sizes in 60 years); it binds on the roster (36 to 42 lineages of 7 animal kinds on 64 x 64) | speed |
| `choose::spread::MOST_ACTS` (4) | options | the most options a node spreads its members over at a decision, the moves counted as one (`spread::top_chances`) | run, 32 x 32 default, seed 1, year 60 (2 / 4 / 8): bodies 1,034 / 1,544 / 913, eaters 7.0e8 / 6.6e8 / 8.6e8 kg, cover 7.4e8 / 2.0e9 / 1.3e9 kg, gas 338 / 249 / 285 ppm: within the spread of seeds 1 to 3 at the default (bodies 1,125 to 1,544) | speed |
| `worldgen::life` `COVER_GENES` and the roster's cover kinds | the cover genes (`cover::LIGHT` ... `cover::FIX`) | the starting cover's genes: capture area, walls, store, frost gene, moisture need, seeds, repair (an initial condition; `worldgen/src/life/kinds.rs` says the reason of each) | run: set so the starting cover covers the land in its first years; the frost gene evolves (`core/tests/cover.rs`) | tuned |
| `cover::RANGE` | gene units | each cover gene's mutation step (times `cover_mut`) and its range | none found: a step of about a twentieth of the range, the frost gene's 5 K | tuned, first guess |
| `cover::APART` (0.5) | share of `alike` | seeds farther than this share of `alike` steps from every slot of their kind in a cell found a slot of their own; slots nearer merge (`seed::settle`, `grow::merge_alike`) | none: half, so a variant that founds a slot is not merged back, and kinds with four slots a cell keep apart | tuned, first guess |
| `BUILT_SHARE` (0.02, `worldgen/src/life/make.rs`) | share | the keep gene of each solid bodies build besides their matter, in every founder: an initial condition, the same for every kind | none: chosen small before any run, so selection can find what each solid is worth | tuned |

## Chemistry data that bodies live by (not rows of `Physics`)

Chemistry 0 (`worldgen/src/chem0.toml`) is data, not a struct of numbers, so it is not in the tables above. These rows are the numbers that decide what bodies can do and the liquid's store of the carbon gas. The budget does not count them: they are set from facts of matter. The decomposers' factors `open` are free numbers of life, not of matter: they are counted among the anchored numbers (Counts).

**Enthalpies.** A value quoted here as measured is for the state the substance is in at 290 K. The data's `hf` is the solid's: a liquid's is the measured one less its fusion enthalpy, a gas's also less its vaporisation enthalpy (laws 2.1). `Chemistry::dh_at` gives a reaction's heat in the measured states (`ew chem` prints it). Molar masses are sums of the file's `element_masses`.

| name | value | fact | anchor |
|---|---|---|---|
| substance 10, body matter | C120H196N6O106Ca (3,456 g/mol), data hf -20,315.8 kJ/mol, cp 4,266 J/(mol K), 1,500 kg/m³, melts at 1,500 K | what bodies are built of (`Chemistry::build`), with no heat in or out: hf = 118 x -212 + 3 x -365 - 1,926.8 + 27 x 286. Burning it gives 16.0 MJ/kg. C:N 20; 1.16% rock element by mass | like cellulose (1,500 kg/m³, 17.5 MJ/kg); foliage C:N 43 (checked: McGroddy et al. 2004), animals about 6; shoots hold 0.1 to 5% of the rock element (checked: White and Broadley 2003) |
| substance 12, the nitrogen mineral | N2H4O3 (80 g/mol), hf -365 kJ/mol as measured (data -370.9), a liquid that melts at 255 K and never boils | the nitrogen of the soil, the only form a body is built of; a salt that lives dissolved, so it is written as a liquid: it moves with the liquid by its share | ammonium nitrate: hf -365.6, cp 140 J/(mol K), very soluble; its solution freezes near 255 K (the eutectic with water) |
| soil, `worldgen::Init::soil` | 0.357 mol of 12 per m² of open land, given when life starts (year 2, `worldgen::life::soil`): 10 g of nitrogen per m² | the start of the nitrogen of the world | mineral nitrogen of a soil is 1 to 10 g per m² (the top of that: the world starts with no humus); checked in part for farm soil: 1.7 to 18 g per m² (CSIRO Soil Research SR9960949; spring counts in Europe and Turkey) |
| `Chemistry::build`, growth | 118 of the solid + 3 of the nitrogen mineral + 1 of the rock salt -> 1 of body matter + 27 of the liquid; no rate | what `copy::grow` runs, paid by `c_copy` of work per kg of matter; the inputs no reaction burns are the minerals (`Chemistry::minerals`: 12, 11) | chemistry data |
| reaction 1, burning the energy-rich solid | CH2O + O2 -> CO2 + H2O, k 1.72e4 /s, Ea 7,760 K | bare, 8.7e-9 /s at 290 K, 0.27 a year: the litter constant, which also runs in a body's stores under its catalyst (founders' burning genes are lower by `LITTER_X`, 0.602, so a body's own burning stays as it was set) | litter decay constants: forest floor 0.25 a year (checked in part: Olson 1963, Ecology 44: 322). Century's metabolic litter pool turns over 3.8 times faster than its structural pool (14.8 and 3.9 a year at the optimum: Parton et al. 1987; from memory) |
| `open` of reaction 1 | 200 | the decomposers' catalyst on the free solid in the open world (`Reaction::open`), as far as the node is moist (laws 2.1): 54 a year in moist ground, about a week | the free solid is free sugar, which soil microbes take up within hours and respire within days (Gunina and Kuzyakov 2015, Soil Biol. Biochem. 90: 87; from memory); a week is the slow end. Anchored |
| reaction 7, burning body matter | C120H196N6O106Ca + 118 O2 -> 119 CO2 + 92 H2O + 3 N2H4O3 + CaCO3, k 2.28e9 /s, Ea 9,000 K, orders 1 and 6 (`Reaction::ord`) | bare, 0.2 a year at 290 K, a little slower than the solid's 0.27; the nitrogen goes back as the mineral and the rock element as lime | Olson's litter constants (see reaction 1); bodies speed it up with a catalyst gene |
| `open` of reaction 7 | 5 | the decomposers' catalyst on dead body matter in the open world: 1.0 a year in moist ground at 290 K, 3.8 times the frame's 0.27 | soft litter rots as Century's metabolic pool, 3.8 times faster than the structural pool (Parton et al. 1987; see reaction 1): 3.8 x 0.27 / 0.2 = 5. Anchored |
| reaction 10, fixing the nitrogen of the air | 2 N2 + 28 CH2O + 29 O2 -> 2 N2H4O3 + 28 CO2 + 24 H2O, k 8e-8 /s, Ea 6,600 K, orders 1; dH -12,676 kJ | it burns 14 units of the solid a mol of the gas (6 g C per g N) and gives all their carbon back to the air as the gas; the mineral keeps its formation energy (207 kJ a mol of the gas), so it is no burning (`Chemistry::keeps`); written for two of the gas, as one needs half a unit of the burning gas | symbioses spend 4.7 to 6.6 g C per g N (Layzell et al. 1979); nitrogenase spends 16 ATP per N2, 488 kJ (checked), which ends as heat; it gives off hydrogen, not carbon monoxide (from memory) |
| reaction 11, the mineral breaking down hot | 2 N2H4O3 -> 2 N2 + 4 H2O + O2, k 2.6e9 /s, Ea 20,000 K; dH -414 kJ | where it is hot (burning matter) the nitrogen leaves as the gas, to the air: 3e-21 /s at 290 K, 1e-3 at 700 K, 5 /s at 1000 K | the salt breaks down above 450 K with an energy near 170 kJ/mol (20,000 K); real fires lose most of the nitrogen of what they burn |
| substance 11, the salt that exists only dissolved | Ca(HCO3)2 (162 g/mol), hf -1,926.8 kJ/mol as measured (data -1,929.37), cp 194 J/(mol K), melts at 271 K, never boils, 30 cm³/mol | holds the liquid's carbon beyond the free gas; the second mineral of growth; in a body it falls out as lime in hours to days | physical: hf of Ca2+(aq) and 2 HCO3-(aq), so dissolving lime has dH -40.3 kJ/mol (real -40.1); cp by Kopp's rule (anchored) |
| reaction 8, dissolving: CaCO3 + CO2 + H2O -> salt | k 1.62e-2 /s, Ea 2,800 K | the liquid takes the air's gas into the salt; its contact is the rock (the least of its inputs) | anchored: the Earth's gross air-sea exchange of this gas, 80 of 589 PgC a year (Ciais et al. 2013), over 71% of the planet; Ea from gas transfer rising 1.4 times per 10 K |
| reaction 9, settling: 3 salt -> 3 CaCO3 + 3 CO2 + 3 H2O | k 1.8e12 /s, Ea 7,647 K | the way back, written for three of the salt, as its real rate takes three ions. The balance (`Reactor::balance`) is S³ = (k8 / k9) exp(4,847 / T) x R x x_gas x W (W + S): S the salt, W the liquid, R the rock (mol), x_gas the gas's share of the air | physical: Ea(9) - Ea(8) = dH / R = 4,847 K. Anchored: 1.0 mol of salt per m³ at 288 K, 400 ppm, 500 m deep (the sea holds 2.0 mol/m³ of dissolved carbon) |
| `Chemistry::builds` | the frame: 6 of the solid -> 1 of 13 + 1 of the liquid (no heat in or out); lime: 1 of the rock salt -> 1 of lime + 1 of the gas + 1 of the liquid (+40.3 kJ, the gas to the air); no rates | the other solids `copy::grow` builds, by `c_copy` of work a kg, each in the share the body's keep gene of it sets (`Body::share`). They pay no upkeep | chemistry data |
| substance 13, the frame | C6H10O5 (162 g/mol), hf -986 kJ/mol (no heat in or out when a body builds it from 6 of the solid), cp 210 J/(mol K), 1,500 kg/m³, melts at 1,500 K | a solid bodies can build, of the energy-rich solid; no nitrogen and no rock element | cellulose: 17.5 MJ/kg, about 1,500 kg/m³ in the cell wall, 40 to 50% of plant dry mass (from memory) |
| reaction 12, the frame rots | C6H10O5 + 6 O2 -> 6 CO2 + 5 H2O, k 1.72e-2 /s, Ea 7,760 K, orders 1 and 1; dH -2,805 kJ | bare, 2.7e-7 a year at 290 K: the frame of a living body does not rot | the cellulose of living plants is not decomposed (from memory) |
| `open` of reaction 12 | 1e6 | the decomposers' catalyst on dead frame in the open world: 0.27 a year in moist ground at 290 K, the `k` and `Ea` of reaction 1 | Olson's litter constant (see reaction 1): litter is mostly cellulose and lignin. A bound for the bare rate: any factor over 1e3 keeps a living body's frame within 0.03% a year. Anchored |
| reaction 13, an enzyme splits the frame | C6H10O5 + H2O -> 6 CH2O, k 9.6e-4 /s, Ea 7,760 K; no heat in or out | nothing without a catalyst: 2.3e-15 /s at 290 K. A body whose diet gene for it is on takes frame in (`Body::uses`) but runs no reaction on it: it carries it as a built solid and sheds it past its level; no founder has it | the spontaneous hydrolysis of the bond between sugar units, 4.7e-15 /s at 25 °C (Wolfenden, Lu and Young 1998, from memory) |
| `modulus` of substances 10, 13 and 9 | 1 MPa, 30 GPa, 70 GPa | Young's modulus of what bodies build: soft tissue with its liquid, the frame, lime (`Body::height`) | soft tissue 0.1 to 10 MPa, the middle on a log scale; tree stems about 10 GPa along their fibres at 500 kg/m³, so the cell wall at 1,500 about 30; calcite 70 to 80 (from memory) |
| `rupture` of substances 10, 13 and 9 | 1, 120 and 100 MPa | the stress each breaks at: the energy to break a body (`Body::breaking`) | soft tissue 0.1 to 10 MPa; tree stems 40 to 50 MPa in compression at 500 kg/m³, so the cell wall about 120; marble 70 to 140, shells 100 to 300 (from memory) |

## Real figures to compare runs with

Not rows of the model: real sizes that a run's results can be held against.

- **Biomass of large plant-eaters.** Hempson et al. 2015 (Science 350: 1056): reconstructed African large herbivores 53 to 11,810 kg/km² over 1° squares, typical about 1,700 kg/km² (without elephants). Damuth 1981 (Nature 290: 699; from memory): density = 4.23 m^-0.75 per km² for plant-eaters, about 250 to 450 kg/km² for 50 to 500 kg animals.
- **Biomass of soil animals that live on dead matter.** Earthworms in temperate grassland and pasture hold 10 to 100 g of fresh mass per m², 1e4 to 1e5 kg/km² (Edwards and Bohlen 1996; Lee 1985; from memory). The eaters live on dead body matter from the ground, so their bound in `tape/tests/ranges.rs` is set on these, not on large plant-eaters.
- **Remaining lives of adults.** Bison: adult cows survive 0.92 to 0.95 a year (Fuller et al. 2007, J Wildl Manage 71; Yellowstone report 2023), so 12 to 20 years. Deer: wild adults survive 0.83 to 0.91 a year with no hunting (a Wisconsin study, quoted), so 5 to 11 years. Mouse: a farm study found apparent survival 0.54 a month (it counts leaving too). Rabbit: no source found.
- **P/B a year** (mouse 3 to 5, rabbit 1 to 2, deer 0.3 to 0.5, bison 0.15 to 0.25): from memory, not checked. Banse and Mosher 1980 (Ecol. Monogr. 50: 355) and Humphreys 1979 (J Anim Ecol 48: 427) are the sources to read.

## Counts

The 100 rows of `Config` and `Physics` are their 100 fields.

| mark | rows | which |
|---|---|---|
| physical | 4 | `sun`, `tilt`, `gravity`, `rho_body` |
| anchored | 52, and 11 outside the structs | 52 fields (among them `c_basal`, `dormant`, `lethal`, `lat_south`, `lat_north`, `cloud_ir`, `mix`, `light_yield`, `gas_floor`, `gas_ref`, `c_build`, `c_move`, `take`, `dormancy`, `porosity`, `seep`, `field`, `bed_depth`, `bed_porosity`, `bed_seep`, `hill`, `carry`, `wear` and `creep`) and the constants `EATER_KEEP_T`/`EATER_COOL_T`, `WET_TISSUE`, `T_SURF`, `MOVE_REF`/`MOVE_EXP`, `FIXERS`, `FIX_GENE` and `MAX_KEEP`, and the three decomposers' factors `open` of the chemistry (reactions 1, 21 and 26) |
| tuned (run or reason named) | 6, and 6 outside the structs | the thresholds of who is alike and who is grouped (`alike`), set for an outcome (`rain_time`, `c_leaf`, `c_brain`, `sweep`, `c_handle`; outside: the roster's stores, genes and years, the spray of the start, `BUILT_SHARE`, the cover's starting genes) |
| tuned, first guess | 30 | 27 free numbers (`wind`, `cloud`, `hard_min`, `c_firm`, `c_learn`, `rho_pass`, `keep_min`, `shape_noise`, `blend`, `plan_weight`, `learn_most`, the Bond numbers `joint` to `deep`, the mutation numbers `row_chance` to `new_noise`, and the Flow numbers `hide` to `hold`) and the three `c_hidden`, `c_memory`, `c_seen`, which are the split of `c_brain` and not three numbers |
| switch | 1 | `null` |
| speed | 2, and 6 outside the structs | `wake_share`, `sleep_days`; outside: `Config::chunks`, a setting of the run that a snapshot does not keep (`ew --chunks`, 64: the cells cut into that many runs for the bodies' parallel passes, many more than the threads of a machine, so the pool shares out uneven runs; any count gives the same world), the air's step `schedule::AIR_STEP_DAYS` (10 days, three a month: against 1-day steps the land within 0.3 K, `the_long_step_keeps_the_climate`), the caps `MAX_NODES`, `MAX_LINEAGES`, `MOST_ACTS`, and `copy::MATE_HIDDEN` (8 hidden units: two parents only for individuals of a brain that big, a level of detail) |
| container | 1 | `phys` |

Rows marked tuned in all: 45 (36 in the structs, 9 outside: also `cover::RANGE`, `cover::APART` and the mass classes `SMALL_KG`, `BIG_KG`, first guesses). The test holds that count at its budget or below (handover 14.3). Rules chosen for a run: 3, held by the test too.
