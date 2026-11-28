# The numbers of the world

Every free number: its value, unit, meaning, anchor and mark. The code is the truth for values; each row says what the
code holds. A test (`core/tests/numbers.rs`) fails when a field of `Physics` or `Config` has no row under the headings
Config, Speed settings or Physics, or a row there names no field.

## How to read this table

- **value:** the default in `core/src/params.rs` (or the code that holds the constant), with its unit.
- **anchor:** the real size it stands on, with its source (author year). "From memory" means the source was not read.
  "Run:" names what a tuned number was set against.
- **mark:**
  - **physical:** a measured size, used as measured.
  - **anchored:** a real order of magnitude, chosen within it.
  - **tuned:** set so a run came out a certain way, or with no real size behind it (`tuned, first guess`). Its first
    job is to find a size and become anchored.
  - **speed:** says when the model may save time, not how the world works; ×2 and ÷2 must change no key number beyond
    its noise.
  - **setting:** a choice of what world to make, as its seed and size are.
  - **switch:** an experiment's on/off (none today). **container:** a field that holds other rows.

A search over worlds may vary only tuned and anchored numbers, within their anchors (about a factor of 3 each way unless
the row says more), never the physical ones. The budget for tuned numbers is in handover 14.3; the count is at the end.

## Config

| name | value | meaning | anchor | mark |
|---|---|---|---|---|
| `sun` | 1361 W/m² | sun strength at the top of the air | the solar constant | physical |
| `tilt` | 23.4° | axial tilt | Earth's tilt | physical |
| `lat_south` | 40° | latitude of the map's south edge | where the surface is near the global mean, 287 K: about 38° to 40° (zonal means, from memory) | anchored |
| `lat_north` | 42.3° | latitude of the map's north edge | `lat_south` + 2.3: an island of 256 km | anchored |
| `variety` | 8 | kinds of cover a cell holds at most (1 to `cover::SLOTS`, 16) | where the world's cover no longer grows with it (`ew run --variety`) | setting |
| `radius_m` | 6.371e6 m | radius of the planet | Earth's mean radius | physical |
| `rotation` | 1 | strength of the turning of moving air by the planet's spin (0: none) | 1 is the real spin's effect | anchored |
| `gravity` | 9.81 m/s² | gravity | Earth's | physical |
| `days_per_month` | 30 | days in a month | 12 × 30 = 360 days for a year of 365 | anchored |
| `months_per_year` | 12 | months in a year | as `days_per_month` | anchored |
| `build_years` | 10 yr | years between two builds of the terrain's samples (`flow::build`) | none: the cost of a build against how long a deposit waits to steepen its reach | tuned, first guess |
| `edit_top_m` | 1 m | loose ground an edit that raises the ground lays over the rock it raises | loose ground on gentle slopes is about 1 m deep (Heimsath, Dietrich, Nishiizumi and Finkel 1997) | anchored |
| `edit_bank_m` | 3 m | bank a dug basin raises over its liquid where its rim would leak | none: a dug pond's spoil is a few metres high | tuned, first guess |
| `edit_wear_years` | 1e6 yr | years of the start's wear a shaped stroke is laid as worn by | none: half the start's 2e6 years cuts valleys into a raised hill | tuned, first guess |
| `skin_m` | 2 m | layer that exchanges with the surface within a year (heat, pores) | the yearly swing of heat reaches 2 to 4 m; 95% of roots lie in the top 2 m (Schenk and Jackson 2002, from memory) | anchored |
| `air_top_m` | 1e4 m | height of the air column | Earth's lower air, 8 to 16 km | anchored |
| `sensible` | 10 W/(m² K) | heat exchange between ground and the air touching it | bulk exchange over land in wind, 5 to 30 (unchecked) | anchored |
| `evap` | 0.006 m/s | transfer speed of evaporation, air side | exchange over open ground in wind, 0.005 to 0.02 (unchecked) | anchored |
| `dissolve` | 5e-5 m/s | transfer speed of a dissolving gas, liquid side | slightly soluble gases over open water in wind, about 4 m a day | anchored |
| `wind` | 4e5 m²/(s K) | overturning air speed per K/m of warmth gradient | the low-latitude cell moves about 1e11 kg/s (Peixoto and Oort 1992, from memory); the lifeless globe moves a fifth of that | anchored, open |
| `mix` | 1.06e6 m²/s | mixing of neighbouring air columns by eddies | the diffusivity of moist static energy, D = 1.06e6 m²/s (Hwang and Frierson 2010, from memory) | anchored |
| `cloud_life` | 1000 s | time condensed liquid stays a cloud's droplets | cloud time of orographic rain, about 1000 s (Smith and Barstad 2004) | anchored |
| `drop_m` | 1e-5 m | radius of a cloud's droplets | effective radius of liquid clouds, 8.5 µm over land to 11.8 µm over sea (Han, Rossow and Lacis 1994, from memory) | anchored |
| `drop_forward` | 0.85 | mean cosine of the angle droplets turn sunlight by | 0.85 for droplets of 10 µm (Lacis and Hansen 1974; Slingo 1989) | anchored |
| `drop_dark` | [0.214, 0.036] | share of sunlight in the wavelengths droplets absorb; share a droplet absorbs there | the sun's light past 1.19 µm; the second fitted to Slingo's (1989) four bands | anchored |
| `line_spread` | 4 | a band's core of strong lines over its mean optical depth (wings 1 / that) | fitted with the `heat` of substances 0 and 3 to Prata (1996), Myhre et al. (1998) and Loeb et al. (2009) | anchored |
| `rain_time` | 86400 s | time a saturated part of an air node rains out what it holds over saturation | rain-making weather systems live about a day (Houze 2004, from memory) | tuned |
| `moist_min` | 0.5 | least vapour ratio at which a node's moistest air saturates (Sundqvist's critical humidity) | measured critical humidity, taken as a column mean: 0.48 to 0.52 (Quaas 2012) | anchored |
| `rain_drift_s` | 2000 s | time rain drifts with the low wind before it lands | cloud time and fallout time of about 1000 s each (Smith and Barstad 2004) | anchored |
| `carry` | 4.7e-11 m³/J | loose volume moving liquid picks up per J it dissipates | the start's yield law (BQART, Syvitski and Milliman 2007) through the island's catchments | anchored |
| `wear` | 1.4e-11 m³/J × 1/bond strength | bonded volume broken per J dissipated | the global yield, 2e-11 m³/J (Milliman and Syvitski 1992), over the starting bed's 1.39 | anchored |
| `creep` | 6.3e-11 m/s × 1/hardness | speed of loose solids creeping down a unit slope | hillslope diffusivity 3e-3 m²/yr, within 1e-3 to 1e-2 (Roering et al. 1999) | anchored |
| `hard_min` | 0.05 | added to a solid's hardness before its softness 1 / (hardness + this) | none found | tuned, first guess |
| `bury` | 1.6e-10 m/s | speed at which the loose ground's mixing buries what lies on it | worms bury objects by about 5 mm a year (Darwin 1881, from memory) | anchored |
| `phys` | `Physics` | holds the rows of Physics | | container |

## Speed settings

| name | value | meaning | anchor | mark |
|---|---|---|---|---|
| `wake_share` | 0.05 | a sleeping node wakes once an amount has grown by this share of what it held | the share of its inputs a calm node's reactions may use while it sleeps (`change`, `CALM`) | speed |
| `sleep_days` | 90 days | longest a ground or air node's reactions sleep (0: none sleeps) | a season | speed |

`chunks` (64) is also a speed setting of `Config`: the cells are cut into this many runs for the cover's parallel
passes, and any count gives one world (`core/tests/threads.rs`). It is a setting of the run, not saved
(`#[serde(skip)]`), so it has no row here.

## Physics

The costs of living tissue are per kg of living cover (or per m² of its capture area) at `t_cat`; they follow the rate
law of its main burning from there.

### Change and Flow, as life feels them

| name | value | meaning | anchor | mark |
|---|---|---|---|---|
| `light_depth` | 10 m | liquid over the cover that holds back 63% of its light | clear sea water: light falls by e in 1 to 20 m (unchecked) | anchored |
| `light_yield` | 0.05 | share of the light a light reaction takes that it stores | photosynthesis stores at most 4.6% to 6% of sunlight (Zhu, Long and Ort 2008, from memory) | anchored |
| `gas_floor` | 4.3e-5 | mole fraction of the light reaction's gas at which it stores nothing | Γ* = 42.75 µmol/mol at 25 °C (Bernacchi et al. 2001) | anchored |
| `gas_ref` | 3.8e-4 | mole fraction at which it stores `light_yield` | the air of the measurements of `light_yield` (Zhu, Long and Ort 2008) | anchored |
| `wet_depth` | 0.01 m | liquid depth at which the ground counts as half wet | a film or puddle of about 1 cm | anchored |
| `soil_m` | 0.03 m | liquid a point's soil holds over the cell's mean per unit of wetness index | TOPMODEL's m, 0.01 to 0.05 m (Beven and Kirkby 1979; Beven 1997, from memory) | anchored |
| `t_cat` | 290 K | temperature at which upkeep and costs are anchored | a typical surface temperature (`units::T_SURF`); life's tempo follows the Boltzmann factor (Gillooly et al. 2001) | anchored |
| `frost_k` | 3 K | spread of a cell's microclimate below a lethal limit | tissue freezes within a few K below its point (from memory) | anchored |
| `harden` | 20 K | how far dormancy lowers the lethal temperature | woody plants harden by 20 to 35 K (Sakai and Larcher 1987, from memory) | anchored |
| `c_firm` | 20 K | K of frost tolerance below the key liquid's melting point that add the light capacity's upkeep again | none found | tuned, first guess |
| `c_basal` | 4.22e4 J/(kg day) | basal upkeep of living tissue | Kleiber's resting rate less resting muscle, at `t_cat` (Kleiber 1947; Elia 1992); plants' maintenance breathing agrees (Penning de Vries 1975) | anchored |
| `c_leaf` | 2e3 J/(m² day) | upkeep of capture area | run: so the starting plants pay their way; a leaf's own breathing is 20 times more | tuned |
| `e_life` | 1e9 J/kg | energy a kg of tissue turns over before it is worn out, at `REPAIR` 0 | about 1e6 J per g in a life (Rubner 1908, as reviewed by Speakman 2005) | anchored |
| `c_repair` | 0.25 | share of upkeep that is repair at `REPAIR` 0 | remaking a body's matter takes 20 to 30% of resting upkeep (from memory) | anchored |
| `dormancy` | 5.7 | power of a dry seed's activity on the liquid it holds | Harrington's rule: seed life doubles for each 1% less moisture, n = ln 512 / ln 3 (Harrington 1972) | anchored |
| `dormant` | 0.02 | least activity a dormant slot wears out at | dry seeds live about 50 times longer than active tissue (Walters 1998, from memory) | anchored |

### Dispersal and living tissue

| name | value | meaning | anchor | mark |
|---|---|---|---|---|
| `kernel_near` | 40 m | standard deviation of the dispersal kernel's near scale | most seeds fall within tens of metres (Clark et al. 1999) | anchored |
| `kernel_far` | 800 m | standard deviation of its far scale | the tail reaches hundreds of metres to km (Clark et al. 1999) | tuned |
| `kernel_far_share` | 0.1 | share that lands by the far scale | long-distance dispersal is rare but sets a front's speed (Clark et al. 1999) | tuned |
| `rho_body` | 1000 kg/m³ | density of living tissue | about the density of the liquid | physical |
| `c_copy` | 1e7 J/kg | work to make one kg of body matter | growth keeps 10 to 20% of the energy it eats | anchored |
| `alike` | 4 steps | genes within this many mutation steps are alike | run: it says who mixes, so what a kind is | tuned |

### The cover (`cover`)

| name | value | meaning | anchor | mark |
|---|---|---|---|---|
| `c_capacity` | 0.1 | upkeep of a catalyst's capacity, a share of what it could make | light enzymes are a quarter to half of leaf protein, turned over in a week to a month (unchecked) | anchored |
| `mut_size` | 0.063 | mutation size a generation, times each gene's step | 10^−1.2 | anchored |
| `seed_kg` | 1e-5 kg | built cover of one plant a seed founds | seeds weigh 0.1 to 100 mg (Moles et al. 2005, from memory) | anchored, first guess |
| `drawdown` | 0.3 | share of the air's gas a leaf draws down inside | C3 leaves hold about 0.7 of the air's gas inside (Wong, Cowan and Farquhar 1979, from memory) | anchored |
| `huber` | 2e-4 | stem cross-section per m² of capture area | the Huber value, 1e-4 to 5e-4 (Tyree and Ewers 1991, from memory) | anchored |
| `c_sapwood` | 1600 J/(kg day) | upkeep of conducting walls at `t_cat` | sapwood breathing gives 500 to 5000 J/(kg day) (Ryan et al. 1995, from memory) | anchored, range 500 to 5000 |
| `root_uptake` | 1.6e-3 mol/(kg s) | key liquid a kg of living fine roots takes up | peak transpiration over live fine roots, tropical deciduous forest (Jackson, Mooney and Schulze 1997) | anchored |
| `root_turnover` | 0.6 /yr | fine roots built anew a year | 0.56 over the world's biomes (Gill and Jackson 2000, from memory) | anchored, from memory |
| `wall_kg` | 0.05 kg | built cover of a plant that builds half the walls its `WALLS` gene sets | stems pass half of a plant's mass near 0.1 to 1 kg (Poorter et al. 2012, from memory) | anchored, first guess |
| `thin_kg` | 5 kg (per m² to the 1/2) | self-thinning: mean plant mass = this × (plants per m²)^−3/2 | the law (Yoda et al. 1963; White 1981); forests give 3 to 8 | anchored |
| `tall` | 35 m^(1/3) | height over stem width to the 2/3 (buckling limit) | about 35 for McMahon's trees (McMahon 1973, from memory) | anchored |
| `stem_form` | 0.5 | a stem's volume over its cylinder's | form factors 0.4 to 0.5 (from memory) | anchored |
| `stem_solid` | 0.4 | share of a stem that is wall solid | wood of 300 to 900 kg/m³ of walls of 1500 (Gibson and Ashby 1997, from memory) | anchored |
| `root_per_height` | 0.1 m/m | loose ground a plant roots in per m of height | rooting depth grows with size (Schenk and Jackson 2002, from memory) | tuned, first guess |
| `root_least` | 0.02 m | least loose ground any plant roots in | a low mat's centimetres | tuned, first guess |
| `rooted_from` | 0.25 | share of the roots' need below which a point holds none of a kind | none: lithic soils under 0.1 m hold herbs and mats only | tuned, first guess |

### The takers (`fauna`)

| name | value | meaning | anchor | mark |
|---|---|---|---|---|
| `c_muscle` | 1.12e4 J/(kg^0.75 day) | upkeep of resting muscle at `t_cat`, on top of `c_basal` | muscles take a fifth of the resting rate (Elia 1992) | anchored |
| `upkeep_exp` | 0.75 | power of living mass upkeep grows as | Kleiber's 3/4 (Kleiber 1947) | anchored |
| `meeh` | 0.1 m²/kg^(2/3) | surface over living mass to the 2/3 | Meeh's rule for mammals (Meeh 1879, from memory) | anchored |
| `r_air` | 0.12 m² K/W | insulation of the still air at a taker's surface | 0.1 to 0.2 in light wind (Campbell and Norman 1998, from memory) | anchored |
| `r_coat` | 0.9 m² K/W | insulation of the coat at `COAT` 1 | arctic fur, 0.5 to 1.5 (Scholander et al. 1950, from memory) | anchored |
| `drink_least` | 0.0928 kg/day | least key liquid a 1 kg taker passes in a day besides cooling | 0.326 M^0.818 mL a day, M in g (Nagy and Peterson 1988) | anchored, from memory |
| `drink_exp` | 0.818 | power of living mass it grows as | the same fit | anchored, from memory |
| `walk_froude` | 0.25 | walking speed squared over g × length | animals walk near Froude 0.25 (Alexander and Jayes 1983, from memory) | anchored |
| `c_move` | 3 J/(kg m) | work to walk a m per kg at `move_ref` | 10.7 m^−0.32 J/(kg m): 3.6 at 30 kg (Taylor et al. 1982, from memory) | anchored |
| `move_ref` | 30 kg | reference mass of `c_move` | a middle mass of the takers, at which `c_move` is given | anchored |
| `move_exp` | −0.32 | power of mass over `move_ref` the work goes as | Taylor et al. 1982 (from memory) | anchored |
| `chew` | 2.9e3 J/(kg^0.75 day) | chewing power at `t_cat` | grazers take 42 to 80 g dry per kg^0.75 a day (van der Merwe and van Niekerk 1980, from memory) | anchored, from memory |
| `busy` | 0.5 | share of the day a taker searches and takes | grazers feed 8 to 13 hours a day (from memory) | anchored |
| `bite_floor` | 0.1 body lengths | least height a bite reaches down to | grazing heights of 3 to 10 cm (from memory) | anchored, weak |
| `reach_len` | 5 body lengths | highest a taker reaches | a tall browser reaches 5.5 m at about 1 m long (from memory) | anchored |
| `store_most` | 0.3 kg/kg | most fuel a taker stores per kg living | mammals store 20 to 40% of lean mass before winter (from memory) | anchored |
| `dry_most` | 0.12 kg/kg | key liquid a taker can lose before it dies | most mammals die at a loss of about 12% (Schmidt-Nielsen 1962; Kemenyfy Rojas et al. 2026) | anchored |
| `litter_rho` | 40 kg/m³ | bulk density of remains on the ground | forest floors 12 to 58 kg/m³ (FERA tables; Sharratt 1997) | anchored |
| `litter_k` | 0.06 W/(m K) | heat conductivity of those remains | 0.03 to 0.09 in a spruce floor (Sharratt 1997) | anchored |
| `jaw` | 50.9 N | jaw force at 1 kg living | log10 F = 0.43 log10 M(g) + 0.416 over 11 mammals (J. Zool. 275, 2008) | anchored |
| `jaw_exp` | 0.43 | power of living mass it grows as | the same fit | anchored |
| `bite_time` | 0.9 s | time to clip one bite, any size | 0.015 min a bite over 15 species (Shipley 1993) | anchored |
| `bite_breadth` | 8.6e-3 m | bite breadth at 1 kg living (area its square) | incisor arcade 8.6 M^0.36 mm (Illius and Gordon 1987, from memory) | anchored, from memory |
| `bite_exp` | 0.36 | power of living mass it grows as | the same fit | anchored, from memory |
| `crown_density` | 1 m²/m³ | capture area a crown holds per its volume | leaf area densities 0.5 to 2 m²/m³ (from memory) | anchored, from memory |
| `thin_exp` | 2/3 | the frame thinner than d goes as (d / main stem)^this | the pipe model (Shinozaki et al. 1964) with West, Brown and Enquist (1999) | anchored |

### Flames in the remains on the ground (`flame`)

| name | value | meaning | anchor | mark |
|---|---|---|---|---|
| `flux_ahead` | 0.05 | share of a burning kg's heat that reaches the remains ahead | the propagating flux ratio, 0.03 to 0.1 (Rothermel 1972, from memory) | anchored |
| `t_light` | 600 K | temperature at which remains light from a flame | piloted ignition of wood and litter, 550 to 650 K (Babrauskas 2003, from memory) | anchored |
| `t_burn` | 1100 K | temperature of a flaming bed | surface fires, 1000 to 1200 K (from memory) | anchored, from memory |
| `duff_hold` | 3 kg/kg dry | liquid the bottom of the remains holds where the ground holds all it can for roots | duff holds 2 to 4 times its dry mass (from memory) | anchored, from memory |
| `smoulder_moist` | 1.15 kg/kg dry | most liquid at which remains smoulder | 110 to 125% (Frandsen 1997; Rein 2008, from memory) | anchored, from memory |
| `surface_m` | 1.8 m | plants no taller keep dead leaves standing as a loose bed | surface fuels reach 6 feet (Scott and Burgan 2005, from memory) | anchored, from memory |
| `cured_life` | 6 months (1.56e7 s) | mean time a cured dead leaf stands before it falls | standing dead grass falls in a half to a whole year (from memory) | anchored, from memory |
| `cured_rho` | 3 kg/m³ | bulk density of a bed of cured leaves | grass beds 1 to 5 kg/m³ (Scott and Burgan 2005, from memory) | anchored, from memory |
| `base_share` | 0.5 | share of a low plant's built cover below the ground | root to shoot ratios about 1 (Mokany, Raison and Prokushkin 2006, from memory) | anchored, from memory |
| `snag_life` | 8 years (2.52e8 s) | mean time a wall killed by flame stands | most fire-killed trees fall in 5 to 15 years (Everett et al. 1999, from memory) | anchored, from memory |
| `strikes` | 6e-9 /(m² yr) | strikes that light dry joined remains | run: about 3% of the land burns a year (Giglio et al. 2018); 0.06 to 0.6% of ground flashes (Christian et al. 2003) | tuned, checked |
| `fine_d` | 1.5e-3 m | width of the remains' fine particles | dead leaves and herb stems 0.2 to 2 mm (from memory) | anchored, from memory |
| `front_life` | 4 days | how long a lit front runs | mean fire life 3 to 15 days (Andela et al. 2019) | anchored |
| `sorb_scale` | [0.1238, −0.001419 /K] | dead remains' liquid per dry mass: scale of (−ln(1 − h))^power, at `flame::SORB_T` | Nelson's (2000) fit of wood's sorption | physical |
| `sorb_power` | [0.3702, −0.003578 /K] | its power | the same fit | physical |

### Flow, as ground cover and moving liquid feel it

| name | value | meaning | anchor | mark |
|---|---|---|---|---|
| `hide` | [0.2, 0.5, 0.02] m | layer that hides half of what lies below: soft solid, liquid, loose | none found | tuned, first guess |
| `soft` | 0.75 | a solid warmer than this share of its melting point is soft cover | none found; solids sinter at 0.3 to 0.8 of their melting point (unchecked) | tuned, first guess |
| `still` | 0.01 m | liquid does not move for a smaller head difference | none found | tuned, first guess |
| `hold` | 2 kg/m² | standing mass that halves what liquid carries off and how fast loose solids creep | none found | tuned, first guess |
| `bed_depth` | 30 m | depth of the weathered zone of the bed | weathered basement is 10 to 60 m deep (Chilton and Foster 1995, from memory) | anchored |
| `bed_porosity` | 0.02 | its specific yield | weathered basement 0.01 to 0.05 (from memory) | anchored |
| `bed_seep` | 3e-6 m/s | its hydraulic conductivity | saprolite aquifers pass 4 to 50 m² a day (checked in part) | anchored |
| `hill` | 250 m | half the spacing of a cell's channels | drainage densities of 1 to 5 km/km² (Horton 1945; Strahler 1957, from memory) | anchored |
| `channel_width` | 7.2 m | width of a channel that carries 1 m³/s | w = 7.2 Q^0.5 over the world's rivers (Moody and Troutman 2002) | anchored |
| `channel_width_exp` | 0.5 | power of discharge width grows as | b about 0.5 (Leopold and Maddock 1953) | anchored |
| `channel_depth` | 0.3 m | depth of a channel that carries 1 m³/s | d = 0.27 Q^0.30 (Moody and Troutman 2002) | anchored |
| `channel_depth_exp` | 0.4 | power of discharge depth grows as | f about 0.4 (Leopold and Maddock 1953, from memory) | anchored |
| `h_mix` | 50 m | depth of a standing liquid's mixed top layer | the ocean's mixed layer, 50 to 70 m on the global mean (de Boyer Montégut et al. 2004, from memory) | anchored |
| `thermocline` | 1e-4 m²/s | eddy diffusivity under the top layer | 1 cm²/s (Munk 1966; Hansen et al. 1984, from memory) | anchored |

### The weather (`flow::air::weather`)

| name | value | meaning | anchor | mark |
|---|---|---|---|---|
| `eddy_time` | 3 days | time over which the weather at a place keeps | mid-latitude systems of 2.5 to 6 days (Lorenz 1969; Blackmon 1976, from memory) | anchored |
| `eddy_speed` | 6 m/s | rms of the eddies' north-south wind in the lower air | v'² of 30 to 60 m²/s² at 850 hPa (Blackmon et al. 1977, from memory); soft weather sets sqrt(`mix` / `eddy_time`), 2 m/s | anchored |
| `eddy_size` | 1e6 m | distance over which the weather is one | the Rossby radius, about 1000 km (Gill 1982, from memory) | anchored |
| `top_damp` | 20 W/(m² K) | heat flux by which the air damps a warmth anomaly of a liquid's top layer | 15 to 25 W/(m² K) (Frankignoul and Hasselmann 1977, from memory) | anchored |

## Rules chosen for a run

Not numbers of the world, but picked so a run came out a certain way (handover 2, lesson 3), so they count like tuned
numbers. **Their number must not grow: one.**

| rule | what it does | chosen for | where |
|---|---|---|---|
| the fixing founders (`FIXERS` 0.1, `FIX_GENE` −8.3) | a tenth of the starting light-eaters carry the gene of the reaction that fixes the air's nitrogen; `--fixers 0` for the control | with one gift of soil nitrogen and no input, the plants collapsed | `worldgen/src/life/seeding.rs` |

## Constants of the laws and the start

Named constants in the code that act like free numbers. The test does not see them. Numerical guards and schedules of
slow work are left out.

| name (where) | value | meaning | anchor | mark |
|---|---|---|---|---|
| `WET_TISSUE` (`core/src/life/shared.rs`) | 0.75 | liquid share of living tissue | lean tissue 0.7 to 0.75, herbs 0.8 to 0.9 (from memory) | anchored |
| `GAS_DIFFUSIVITY`, `GAS_DIFFUSIVITY_MM` (`units.rs`) | 2.0e-5 m²/s, 0.032 kg/mol | a gas's diffusivity in air at 293 K, for that molar mass (Graham's law for others) | 2.0e-5 to 2.1e-5 for 32 g/mol (Marrero and Mason 1972, from memory) | physical |
| `FERMENT_YIELD` (`units.rs`) | 2/32 | energy fuel frees without the oxidising gas, over with it | 2 ATP against about 32 (Berg, Tymoczko and Stryer) | physical |
| `T_SURF` (`units.rs`) | 290 K | typical surface temperature | as `t_cat` | anchored |
| `LIQUID_VISCOSITY` (`units.rs`) | 1.0e-6 m²/s | kinematic viscosity of the key liquid | water at 20 °C | physical |
| `cover::RANGE[GAS]` | 0.02 to 0.6, step 0.05 | share of a root's volume that is gas space | drained ground 2 to 7%, wet ground 20 to 55% (Justin and Armstrong 1987, from memory) | anchored |
| `flame::CARRY_SHARE` | 0.592746 | remains covering more than this share of the ground join and carry a flame | site percolation threshold of the square lattice (Newman and Ziff 2000) | physical |
| `flame::RESIDENCE_PER_M` | 8 min an inch | how long a fine particle flames per m of width | Anderson 1969 (from memory) | anchored |
| `flame::FLAME_K`, `FLAME_EXP` | 0.0775, 0.46 | flame length of intensity I kW/m | Byram 1959 (from memory) | anchored |
| `flame::WIND_C`, `WIND_C_EXP`, `WIND_C_POW`, `WIND_B`, `WIND_B_POW`, `WIND_E`, `WIND_E_EXP`, `PACK_OP`, `PACK_OP_POW` | 7.47, −0.133, 0.55, 0.02526, 0.54, 0.715, −3.59e-4, 3.348, −0.8189 | wind factor of a flame's spread, φ_w = C U^B (β / β_op)^−E | Rothermel 1972 (from memory) | anchored |
| `OPEN_K`, `SHELTER_K`, `WAF_A`, `WAF_B`, `WAF_C`, `CROWN_FILL` (`core/src/flame/bed.rs`) | 1.83, 0.555, 20, 0.36, 0.13, 3 | share of the 20 ft wind at mid-flame, open and under a canopy | Albini and Baughman 1979, as Andrews 2012 gives them (from memory) | anchored |
| `flame::SCORCH_K`, `SCORCH_EXP` | 0.1483, 2/3 | height the plume kills leaves to | Van Wagner 1973 | anchored |
| `Kind::year` (`worldgen/src/life/kinds.rs`) | 0 | when each starting kind is placed after the first life | run: the kinds start together | tuned |
| `Seeding::default` (`worldgen/src/life/seeding.rs`) | 20 rounds | mutation sprayed on a start | run: one generation's step is too small to select from | tuned |
| `FIXERS` (`worldgen/src/life/seeding.rs`) | 0.1 | share of starting light-eaters that fix | legumes are 7% of flowering plants, not all fix (Judd et al. 2002) | anchored |
| `FIX_GENE` (`worldgen/src/life/seeding.rs`) | −8.3 log10 mol/(kg s) | fixing capacity of those founders | about 10 g N/m² a year, as real fixing stands (Cleveland et al. 1999) | anchored |
| `fauna::take::SUBSTEPS` | 6 | steps of the month the take is found in | a rule of the model: within 1% of the exact take (`core/tests/fauna.rs`) | anchored |
| `WORK_T` (`core/src/fauna/body.rs`) | 310 K | body temperature of the measurements of work | the rows of work are mammals' | anchored |
| `BAND` (`core/src/fauna/body.rs`) | 0.5 decades | band of heights a taker searches each side of its `REACH` gene | a browser's reach from knees to over its head (from memory) | anchored |
| `contract::GROUND_M` | 0.01 m | height what lies on the ground stands to a mouth | about a seed's size | anchored |
| `LEVELS` (`core/src/contract/liquid.rs`) | 13 | decades of the ways to open liquid, 10^0 to 10^12 mol | a rule of the model | anchored |
| `fauna::N_WHOLE` | 64 | below this many members a slot's count is whole | the noise of a count of 64 is an eighth | anchored |
| `TAKER_KINDS` (`worldgen/src/life/takers.rs`) | five kinds | the starting takers' genes | real plant-eaters of 30 g, 3 kg, 60 kg, 400 kg and a cold-bodied 2 kg; the rest set by hand | tuned |
| `START_SHARE`, `DAMUTH_A`, `DAMUTH_EXP` (`worldgen/src/life/takers.rs`) | 1/3, 4.23, 0.75 | starting takers a cell: a third of log10 D = 4.23 − 0.75 log10 W | Damuth 1981 | anchored |
| `START_STORE` (`worldgen/src/life/takers.rs`) | 0.1 kg/kg | store a starting taker carries | half the low end of 20 to 40% (from memory) | anchored |
| `COVER_GENES` and the roster's kinds (`worldgen/src/life/kinds.rs`) | genes | the starting cover's genes | run: the cover covers the land in its first years; the tree from a forest stand (kinds.rs) | tuned |
| `cover::UNIT` | gene units | each cover gene's unit of likeness (a decade for the two dose genes) | none found | tuned, first guess |
| `cover::RANGE` | gene units | each cover gene's step and range | none found: a step about a twentieth of the range | tuned, first guess |
| `fauna::RANGE` | gene units | each taker gene's step and range | none found: a hundredth to a twentieth of each range | tuned, first guess |
| `life::kin::APART` | 0.5 | share of `alike` past which a seed or newborn founds its own slot | none | tuned, first guess |
| `cover::PATCHES` | 3 | most patches a cell's cover holds | a clearing, its regrowth and the old stand (Watt 1947) | tuned, first guess |
| `ALIKE_HEIGHT`, `HOLDS`, `ALIKE_COVER` (`core/src/cover/patches.rs`) | 0.2, 0.1, 0.2 | two patches fuse when their kinds, heights and cover per m² are alike | none: patches the model cannot tell apart are one | tuned, first guess |
| `EVENT_LEAST` (`core/src/cover/patches.rs`) | 0.15 | least share of a patch an event must hit to make a patch | far above a canopy's 1 to 2% yearly deaths (Stephenson and van Mantgem 2005) | tuned, first guess |
| `GROUND_LOTS` (`core/src/cover/lying.rs`) | 12 | lots of seeds lying a cell's ground holds | none: more than `variety` | tuned, first guess |
| `TAN_MIN` (`core/src/inside/mod.rs`) | 1e-3 | least slope of the liquid table in the wetness index | valley floors fall at least 1e-4 to 1e-3 (Leopold, Wolman and Miller 1964) | anchored |
| `LAND_SHARE` (`worldgen/src/terrain.rs`) | 0.29 | share of a globe above 0 m at the start | Earth's land share | anchored |
| `LAND_RING` (`worldgen/src/terrain.rs`) | 1 square | squares round the land kept when open water is made coarse | the coasts keep their detail | speed |
| `WEAR_YEARS` (`core/src/space/wear.rs`) | 2e6 yr | time the land rises while it is worn at the start | run: valleys branch while the land's share and heights stay within 10% | tuned |
| `LOW_STAND_M` (`core/src/space/wear.rs`) | −100 m | level of the open low ground while the start wears the land | the last low stand, about −120 m (Fairbanks 1989) | anchored |
| `CUT` (`core/src/space/wear.rs`) | 3e-7 /yr | K of the stream power law at m = 1/2, n = 1 | run: as `WEAR_YEARS`; real K 1e-7 to 1e-4 (Stock and Montgomery 1999) | tuned |
| `CREEP` (`core/src/space/wear.rs`) | 3e-3 m²/yr | creep diffusivity of the start's wear | as `creep` (Roering et al. 1999) | anchored |
| `ROUGH_M` (`core/src/space/wear.rs`) | 1 m | spread of the land's heights where it starts to rise | none found | tuned, first guess |
| `OOZE_M` (`core/src/space/wear.rs`) | 1 m | loose ground on the floor under the shelf | none | tuned, first guess |
| `STAGES`, `NOISE_BY`, `MOST_SAMPLES` (`core/src/space/wear.rs`) | (4, 0.4, 16), (2, 0.3, 8), (1, 0.3, 4); 2; 65,536 | grids, time shares and steps of the start's wear | none: the steps set the cost | speed |
| `FLOATS`, `ROCK_KG_M3` (`core/src/space/slopes.rs`) | 2700/3300, 2700 kg/m³ | Airy rebound share, and the bed's density | crust and mantle densities (Turcotte and Schubert 2002, from memory) | anchored |
| `BENDS_M`, `BEND_SAMPLES` (`core/src/space/slopes.rs`) | 50 km, 16 | width of the plate's bending, and its samples | flexural parameter of tens of km (Watts 2001, from memory) | anchored |
| `GRADED_M` (`core/src/space/ponds.rs`) | 1 m | a closed basin shallower than this is a graded plain, no pond | none | tuned, first guess |
| `ANCHOR_M`, `ACROSS` (`core/src/space/terrain.rs`) | 128 km, 6 | width that holds 6 units of an island's noise | main valleys of a range lie 20 to 30 km apart (Hovius 1996, from memory) | anchored |
| `RANGE_OFF`, `RANGE_WIDE`, `RISES` (`rises.rs`); `FINE_FROM` (`terrain.rs`) | 1; 0.7 to 1.4; 0.2, 1.3; 2 | an island's range: axis offset, half-width, rise, plain's octave | none: set so islands hold a range, hills and a plain | tuned, first guess |
| `FALL_POWER`, `ONE_REACH` (`core/src/space/rises.rs`) | 1.6, 0.5 | an island of one rise falls as (distance / reach)^power | none: one island fills a third of its map | tuned |
| `NUMBER_AREA` (`core/src/space/rises.rs`) | 0.5 to 0.9 | number-area exponent of drawn rises | B 0.5 to 0.9 for archipelagos (Korčák 1938; Mandelbrot 1975, from memory) | anchored |
| `LONG_MOST` (`core/src/space/rises.rs`) | 4 | most a drawn rise is longer than wide | raised blocks are 1 to 5 times as long as wide (from memory) | anchored, from memory |
| `TOP`, `MEAN_TOP` (`core/src/space/rises.rs`) | 0.75, 0.5 | the noise's rise over the level, 90th percentile and mean | measured on the noise, seeds 1 to 12 | anchored, on the noise |
| `SHARE`, `LEAST_KM2` (`core/src/space/rises.rs`) | 0.3 to 0.45, 600 km² | land share of a drawn map, and least land a rise raises | run: about the land of one-rise islands, 1 to 7 rises by size | tuned |
| `POWER`, `RING_SHARE`, `RING`, `TILT_MOST`, `BREACHED`, `LIFT_MOST` (`rises.rs`) | 1.8 to 3, 0.03, 1.5 to 2.5, 0.8, 0.85, 0.1 | a drawn rise's shape: dome or block, ring, tilt, breach, lift | run: about one world in ten holds a ring, nearly all open | tuned |
| `DIVIDE`, `DIVIDE_WIDE` (`core/src/space/rises.rs`) | 0.5, 0.2 | the sag between two drawn rises | run: two lands keep a strait where the map has room | tuned |
| `APART`, `TRIES`, `EDGE_SQUARES`, `SHRINK` (`rises.rs`); `SCALES`, `SCALE_STEPS`, `SCALE_HALVINGS` (`terrain.rs`) | 1, 24, 2, 0.95; 0.2 to 3, 14, 6 | how drawn rises are placed, and the search of their scale | none: the open low ground runs along every edge | speed |
| `LAYERS`, `THICK_M`, `FOLD_M`, `FOLD_PER_SQUARE`, `FOLD_OCTAVES`, `BODY_PER_SQUARE`, `BODY_OCTAVES`, `BODY_ABOVE` (`core/src/space/strata.rs`) | 12, 60 to 400 m, 2000 m, 0.25, 3, 0.5, 4, 0.66 | the bed's layers, folds and bodies | beds metres to hundreds of metres thick, folded over km (from memory) | tuned, first guess |
| `RIM_ABOVE`, `RIM_PER_SQUARE`, `RIM_OCTAVES`, `RIM_GATE` (`core/src/space/strata.rs`) | 0.63, 0.5, 2, 0.7 | rims round some bodies | ore gathers round some intrusions (from memory) | tuned, first guess |
| `seams`, `rim` (`worldgen/src/chem0.toml`) | [[5, 100, 1], [6, 30, 3], [7, 40, 2], [8, 30, 1]], 9 | makes laid as seams (make, m, count a repeat), and the rims' make | real seams 1 to 100s of m, thickened to show at the samples (from memory) | tuned, first guess |
| `makes` (`worldgen/src/chem0.toml`) | ten mixes | the makes of the bed in chemistry 0 | none: mixes of the bed solids; real rock spans more (Stock and Montgomery 1999) | tuned, first guess |
| `BREAKS`, `BREAKS_UNDER_M` (`core/src/space/mantle.rs`) | 7.7e-5 m/yr, 0.4 m | soil production under none, and its e-fold depth | 77 m a million years, e-fold 30 to 50 cm (Heimsath et al. 1997) | anchored |
| `SLOWEST` (`core/src/space/mantle.rs`) | 1e-6 | slowest lowering the mantle's law reads | none: the mantle would grow without end | tuned, first guess |
| `REST`, `CREEP_DEPTH_M` (`core/src/space/mantle.rs`) | 0.8, 1 m | slope loose ground rests at; depth `CREEP` is anchored at | dry grains rest at 0.7 to 0.85 (from memory) | anchored |
| `SLID_UNDER_DEG` (`core/src/space/slide.rs`) | 2° | an edited slope comes to rest this much under `REST` | angle of repose about 2° under the most stable (Jaeger, Liu and Nagel 1989, from memory) | anchored |
| `SPREAD`, `SPREAD_RADII` (`core/src/edit/shape.rs`) | 0.2, 2 | how far edited ground spreads about a brush | none: a hill's crest rounds by creep (Gilbert 1909) | tuned, by eye |
| `CLASSES` (`core/src/soil/texture.rs`) | 0.2 to 2 µm, 2 to 50 µm, 50 µm to 2 mm | grain classes of the texture | clay, silt, sand (Soil Survey Staff 1993) | physical |
| `WEATHER_YEARS` (`core/src/soil/texture.rs`) | 1e5 yr | e-time of weathering toward fine grain | feldspar grains last 1e5 to 1e6 years (White and Brantley 2003) | anchored |
| `SORTING` (`core/src/soil/texture.rs`) | 2 | least spread of sizes that settle together | about one φ unit (Folk and Ward 1957, from memory) | tuned, first guess |
| `DRAG_STOKES`, `DRAG_FORM` (`core/src/soil/texture.rs`) | 18, 1 | drag of a sinking natural grain | Ferguson and Church 2004 | physical |
| `WILT_FIT`, `FIELD_FIT`, `OVER_FIT`, `WILT_STEP`, `FIELD_STEP`, `OVER_STEP`, `FULL_FIT`, `SEEP_FIT`, `SUCTIONS_KPA`, `FINE_FITTED`, `COARSE_FITTED` (`core/src/soil/pores.rs`) | the fits' numbers | pores from texture | Saxton and Rawls 2006, no organic matter | physical |
| `ORGANIC` (`core/src/soil/pores.rs`) | 0.85, 0.6, 0.3, 1e-6 m/s | organic ground: porosity, field capacity, wilting point, conductivity | sapric peat (Boelter 1969; Päivänen 1973, from memory) | anchored |
| `SITES_FINE`, `SITES_ORGANIC` (`core/src/soil/exchange.rs`) | 1.06e3, 2.8e3 mol/m³ | exchange sites of fine particles and of organic matter | 0.4 and 2 mol/kg (Brady and Weil 2008, from memory) | anchored |
| `HELD_PER_SITE` (`core/src/soil/exchange.rs`) | 0.04 m³/mol | liquid whose minerals a mol of sites holds | ammonium's Kd 1 to 10 L/kg (from memory) | anchored, from memory |
| `SHELTERED` (`core/src/soil/mod.rs`) | 0.75 | slow matter's rot is cut by this × the fine and middle share | CENTURY's 1 − 0.75 T (Parton et al. 1987) | physical |
| `ERODIBLE_EXP`, `ORGANIC_SPAN`, `ORGANIC_MOST` (`core/src/soil/mod.rs`) | 1.14, 12, 4 | erodibility from texture and organic per cent | Wischmeier and Smith 1978 | physical |
| `FLOOD_RUNOFF` (`core/src/space/delta.rs`) | 1e-6 m/s | runoff of the flood that sorts what settles and builds the lobes | a bankfull flood of about 1 m³/s per km² (Dunne and Leopold 1978, from memory) | anchored |
| `FETCH_REF_M` (`core/src/space/delta.rs`) | 50 km | run of open liquid that counts as an open coast | none | tuned |
| `TOPSET_M`, `FORESET`, `ALONG_M` (`core/src/space/shelf.rs`) | 10 m, 0.3, 5 km | the shelf: platform depth, front slope, half the shore a mouth spreads along | fair-weather wave base 5 to 15 m; Gilbert deltas (Gilbert 1885, from memory) | anchored |
| `ROUNDED_M` (`core/src/space/shelf.rs`) | 250 m | half the width the shelf's edge is rounded over | clinoforms of hundreds of m (Pirmez, Pratson and Steckler 1998, from memory) | tuned, first guess |
| `DEPTH_COST_M`, `ALONG_COAST`, `DOWN_CHANNEL` (`core/src/space/fill.rs`) | 10 m, 3, 0.5 | costs of a lobe's steps: depth, along the coast, down its channel | waves spread a delta along the coast (Wright and Coleman 1973) | tuned |
| `HIGH_STAND_YEARS` (`core/src/space/stand.rs`) | 6000 yr | time the liquid stands at 0 m at the start | the sea reached its level 7000 to 6000 years ago (Fairbanks 1989) | anchored |
| `YIELD_OMEGA`, `YIELD_Q_EXP`, `YIELD_A_EXP`, `YIELD_T0_K` (`core/src/space/stand.rs`) | 6e-4, 0.31, 0.5, 273.15 K | a catchment's load at the high stand, Qs = ω Q^0.31 A^0.5 R T | BQART (Syvitski and Milliman 2007, from memory), read below its range | extrapolated, from memory |
| `RUNOFF_M` (`core/src/space/stand.rs`) | 0.3 m/yr | runoff of a catchment for the yield law | Earth's rivers carry about 0.3 m a year (Fekete, Vörösmarty and Grabs 2002, from memory) | anchored, from memory |

## Constants of the drawing (the tiles and the page)

Drawing only: no law reads them. Each name is a `const` in the files named; the values and their reasons are there.
`lib/` is `web/shaders/lib/`.

| names | where | what they set | anchor | mark |
|---|---|---|---|---|
| `TYPICAL_RUNOFF` (8.5e-9 m/s), `WET_TIMES` (10) | `tape/src/ground/channels.rs` | stand-in discharge per m² drained, and its wettest multiple | Earth's mean, 0.27 m a year (Dai and Trenberth 2002) | anchored |
| `MOST_RUNOFF` (3.6e-6 m/s) | `tape/src/ground/channels.rs` | how far a tile holds a channel from its line | the wettest month known, 9.3 m of rain (Cherrapunji, July 1861) | anchored |
| `SHOW_M`, `LINE_PX`, `LINE_GROW`, `LINE_MOST`, `CSS_TEXELS_MOST`, `LINE_MARGIN`, `ALONG_WRAP_M`; `LINE_FADE`, `FRINGE_PX` | `channels.rs`; `lib/river.wgsl` | which channels show as lines, and how wide | none: a map line of 1 to 2.5 css pixels | tuned, by eye |
| `FLOOD` (5), `STATION_EXP` (0.26); `BED_PX` | `tape/src/ground/bends.rs`; `lib/river.wgsl` | a bed is the channel law at 5 × the flow; this month's water within it | bankfull flood 2 to 10 × mean flow; at-a-station exponent 0.26 (Leopold and Maddock 1953) | anchored |
| `BEND_WIDTHS`, `BELT_WIDTHS`, `HARD_SHARE`, `WANDER_SAMPLES`, `WANDER_HARD`, `BENDS_BELOW`, `STRAIGHT_ABOVE`, `WALL_DEPTHS`, `TURN_SHARE`, `BEND_MOST`, `BENDS_FROM_M`, `REACH_SAMPLES`, `JOIN_SWAY`; `WANDER_EVERY`, `WANDER_OCTAVES`, `WANDER_FINE`, `WANDER_FINE_SHARE`, `TURN_WIDTHS`, `WANDER_GAIN`; `WANDER_FROM`, `WANDER_TEXELS`, `BEND_HELD`, `STEP_TEXELS`, `CALM_STEPS`; `BEND_PX` | `bends.rs`; `wander.rs`; `lines.rs`; `lib/land.wgsl` | a drawn channel's wander and bends | meander wavelength about 11 widths (Leopold and Wolman 1960); belts 4 to 20 widths (Williams 1986); pool-riffle below 0.015, step-pool above 0.03 (Montgomery and Buffington 1997) | anchored |
| `STEPS`, `HEADING_MOST`, `RADIUS_WIDTHS`, `REACH_SHARE`, `REACH_SKEW`, `FAR_LEAST`, `SWELL_BENDS`, `SWELL_LEAST`, `ARC_WIDTHS`, `ARC_VARY`, `ADVANCE_WIDTHS`, `HARD_RADIUS`, `HARD_HEADING`, `HARD_ARC`, `HARD_ADVANCE`, `HARD_VARY`, `STRAIGHT_WIDTHS`, `ARC_HALVINGS` | `tape/src/ground/belt.rs` | the walk of bends from apex to apex | bends turn on 2 to 3 widths (Hickin and Nanson 1975); curvature falls evenly (Langbein and Leopold 1966) | tuned, by eye |
| `SMOOTH_SAMPLES`, `SMOOTH_MOST`; `VALLEY_FROM`, `VALLEY_M`, `CHORD_SHARE`, `PIECE_SHARE`; `STRIP` | `stems.rs`; `valley.rs`; `mend.rs` | a channel's smoothed line, its valley, and the mending of small crossings | none | tuned, first guess |
| `SHORE_M`, `SMOOTH_SAMPLES`, `SMOOTH_REACH`, `WATER_M`, `SIDE`, `SIDE_FROM`, `LEAN_M`, `KEEP_M`, `KEPT`, `NEAR_SAMPLES`, `EDGE_M`, `SIDE_SOFT`; `MOUTH_UNDER_M`, `MOUTH_MOST`, `MOUTH_STEP`, `MOUTH_DROWN`, `MOUTH_WIDE`, `MOUTH_LONG` | `tape/src/ground/shore.rs` | the smooth shore drawn, its shallows, and river mouths | none: the shore moves about a quarter of a sample | tuned, first guess |
| `SHORE_RISE_LEAST`, `RELIEF_EDGE_SLOPE`, `RELIEF_UNDER_M` | `lib/live_map.wgsl` | least depth gradient at the water's edge; where the relief moves the height, and not under deep standing liquid | none: keeps the edge within a pixel | tuned |
| `MOUTH_FLARE`; `MOUTH_BARS`, `MOUTH_MELTS`, `MOUTH_CALM`, `MOUTH_REACH`; `FOAM_M`, `FOAM_LINES`, `FOAM_LINE_M`, `FOAM_LINE_FALL`, `FOAM_LINE_W`, `FOAM_BREAK_M`, `FOAM_LINE_PX`, `FOAM_RIM`, `FOAM_EDGE_KEPT`, `FOAM_WANDER`, `FOAM_WANDER_M`; `FILM_M` | `lib/rock.wgsl`; `lib/river.wgsl`; `lib/style.wgsl`; `lib/draw.wgsl` | a river's mouth in standing water, the shore's foam, the least depth drawn as water | none: a channel widens where it meets standing water | tuned, by eye |
| `RUNNING_CLEAR`; `LOAD_ATTENUATION`, `LOADED_WATER`, `LOAD_HALF` | `lib/water.wgsl` | attenuation of clear running water and of its load | a few tenths per m; 0.03 to 0.1 m²/g of grains (Kirk 1994, from memory); turbid reflectance saturates (Nechad et al. 2010) | anchored |
| `BED_FILM` | `lib/river.wgsl` | a stream bed's light over soaked bare ground | about 5% against 20% (Legleiter et al. 2004, from memory) | anchored |
| `CLEAR_ATTENUATION`, `INLAND_ATTENUATION`, `INLAND_DEEP`, `LAKE_CLEAR`, `LAKE_WATER`, `BED_LIGHTEN`, `RIVER_SEEN_M`, `STANDING_SEEN_M`, `RIVER_SEEN_POW`, `BED_SHOWN`, `LAKE_SCALE`, `LAKE_DEEP`, `LAKE_OPEN_M`, `LAKE_DEEP_M`, `RUNNING_LIGHTEN`, `SHELF_M`; `RIVER_LINE` | `lib/style.wgsl`; `lib/river.wgsl` | the Earth view's water colours, a lake's depth and shore shelf, a far river's line | clear water 0.03 to 0.4 per m (Jerlov 1976); pure water takes red at 0.35 (Pope and Fry 1997) | tuned, by eye |
| `GLINT_SLOPE`, `GLINT_MOST`, `SKY_TILT`, `RIPPLE_LONGEST_M`, `RIPPLE_OCTAVES`, `RIPPLE_SLOPE` | `lib/water.wgsl` | a liquid's ripples and the sun's glints on them | wind over a few km raises waves 2 to 8 m long (Shore Protection Manual 1984); the glints are drawn, not found | tuned, by eye |
| `WIND_WAY`, `GUST_LANE_M`, `PAW_M`, `PAW`, `WINDROW_M`, `WINDROW_W`, `WINDROW`, `GUST_SHEEN`, `GUST_RIPPLE`, `SWAY_PX`, `BED_SWAY` | `lib/water.wgsl` | the wind's lanes, patches and streaks on standing liquid, and its uneven bed | windrows lie along the wind, metres to tens of m apart (Langmuir 1938) | tuned, by eye |
| `RIPARIAN_M`, `RIPARIAN_BEDS`, `RIPARIAN_VARY`, `RIPARIAN_VARY_M`, `RIPARIAN_DARK` | `lib/river.wgsl` | the darker, greener strip along a channel | riparian strips of a few to tens of m line most streams (from memory) | tuned, by eye |
| `LOAD_SHOWS` (0.25), `DISCHARGE_SHOWS` (0.1) | `wasm/src/lib.rs` | change that sends the samples' load and discharge again | none | speed |
| `MUD_PLUME_DEEP_M`, `MUD_PLUME_DAYS`, `MUD_PLUME_MOST_M`, `MUD_PLUME_MOST_G`, `MUD_CLEAR_G`, `MUD_Q_LEAST`; `PLUME_STREAK`, `PLUME_STREAK_M` | `web/src/land.ts`; `lib/live_map.wgsl` | a river's mud plume in a body | a plume a metre thick settling in hours to days (from memory) | tuned, by eye |
| `LITTER_KG` (0.3 kg/m²) | `tape/src/layers.rs` | remains that hide all but e^−1 of the ground | a few tenths of a kg/m² hide the soil (from memory) | anchored |
| `SOIL_OF_BED`, `SOIL_OF_BED_M`, `SOIL_SMEAR_M`, `SOIL_SMEARED`; `HIDES_BED_M`, `PATCHY`, `PATCHY_OCTAVES`, `SMOOTH_LOOSE_M` | `lib/live_map.wgsl`; `lib/rock.wgsl` | how the bed shows through thin loose ground | lithic soils, under 0.1 m, show rock (from memory) | tuned, by eye |
| `SEA_WAVE`, `WIND_M_S`; `SWASH_VARY`, `SWASH_VARY_M`, `SWASH_DRY`, `SWASH_EDGE`, `SWASH_CALM`, `BORE_REACH`, `BORE_CLUMP_M`, `BORE_DENSE`, `BORE_SPARSE`, `BORE_SOFT`, `BORE_FOAM`, `BORE_EDGE`, `BORE_EDGE_M` | `lib/water.wgsl`; `lib/colour.wgsl` | waves at a shore and their swash | fetch-limited waves (Shore Protection Manual 1984); run-up (Stockdon et al. 2006) | anchored, by eye |
| `BEACH_SLOPE`, `ROCK_SLOPE`, `BEACH_LOOSE_M`, `LAKE_BEACH`, `BEACH_DEEP_M`, `GRAVEL_SLOPE`, `STONY_PEBBLES`, `MUD_SLOPE`, `STORM_RUN_UP`; `BEACH_OWN`, `BEACH_LIFT`; `SAND_WET_RAG`, `SAND_WET_RAG_M` | `lib/colour.wgsl`; `lib/style.wgsl`; `lib/detail.wgsl` | beaches of sand, pebbles or mud, read from slope | beach slope grows with grain (Bascom 1951) | tuned, by eye |
| `LAID_M`, `FLOOD_M`, `FLOOD_FULL`, `WINNOW_M`, `MARSH_RISE_M`, `MARSH_FULL`, `REWORK_M`, `FRESH_LOG_YEARS`; `LAID_SOAKED`, `RIPPLE_SWING_M` | `lib/colour.wgsl`; `lib/detail.wgsl` | deposits, marshes and ripples | capillary fringe 1 to 3 m in silt (Bear 1972, from memory) | tuned, by eye |
| `LEAF_REACH_M`, `LEAVES_MOST`, `LEAF_CELL_M`, `LEAF_LONG_M`, `PAD_WARM_K`, `PADS_MOST`, `PAD_PATCH_M`, `PAD_CELL_M`, `PAD_R_M`, `PAD_BLOOM_K`, `PAD_BLOOMS`, `PAD_FLOWER_M`, `PAD_RGB`, `PAD_FAR`; `FLOAT_TALL` | `lib/afloat.wgsl`; `lib/live_map.wgsl` | fallen leaves and floating leaves on still water | water lilies leaf in water of 15 °C and more (Sculthorpe 1967) | tuned, by eye |
| `STONE_M` (0.001) | `tape/src/ground/rock.rs` | largest stone a channel moves per unit of √A × slope | torrents move boulders of 0.2 to 2 m (from memory) | anchored |
| `STONE_TOP`, `STONE_SHADOW`, `STONE_SHADOW_REACH`, `STONE_TONE`, `STONE_OCTAVES`, `STONE_STEP`, `STONE_FEW`, `STONE_RARE`, `STONES_LEAST`, `STONE_GAP`, `STONE_SIZES`, `STONE_MOST_M`, `WAKE_LONG`, `WAKE_SPREAD`, `WAKE_EDDY`, `CUSHION`, `WAKE_RIPPLE`, `CELL_INSIDE`; `SCREE_STONE_M`, `SCREE`, `SCREE_FULL`, `BELOW_LEAST`, `STONES_ON_BED`, `SUNK_MOST`, `SUNK_LOOSE_M`, `STONE_SEEN`, `COBBLE_M`, `BED_SUNK`; `IN_CHANNEL`, `STONES_OUT`, `WORN_Q`, `BAR_FINES`, `WALL`, `BLOCKS`, `STEEP_SHARP`, `STONE_SMALL`; `STONE_STANDS`, `WAKE`, `STONE_WATERED` | `lib/stone.wgsl`; `lib/crag.wgsl`; `lib/live_map.wgsl`; `lib/river.wgsl` | stones, scree and a channel's stones | debris rounds within a few km of travel (Krumbein 1941, from memory) | tuned, by eye |
| `WET_STONE_DARK`, `WET_GLOSS`, `GLOSS_LIT`, `DRY_PALE` | `lib/stone.wgsl` | the wet look of a stone | wet rock sends back half to two thirds of dry (Lekner and Dorf 1988) | anchored |
| `BED_COARSE`, `BED_ROUND`, `BED_ROCK_STONE`, `BED_ROCK_CELL`, `BED_ROCK_FILL`, `BED_ROCK_SIZE`, `COBBLE_CELL_M`, `COBBLE_FILL`, `COBBLE_SIZE_M`, `DRIFTWOOD_CELL_M`, `DRIFTWOOD_FILL`, `DRIFTWOOD_M` | `lib/things.wgsl` | a channel bed's gravel, cobbles and boulders; cobbles on a shore; driftwood | a stream rounds its stones within a few km (Krumbein 1941, from memory) | tuned, by eye |
| `DRY_M` (0.1 m) | `tape/src/table.rs` | a body holding less on its mean is drawn as soaked ground | a film over a flat basin | tuned, first guess |
| `SNOW_RISE_M`, `SNOW_SUNNY`, `SNOW_HOLLOW_M`, `SNOW_SPREAD`, `SNOW_BARES_MOST`, `SNOW_PATCH_M`, `SNOW_PATCHY`, `SNOW_THIN` | `lib/land.wgsl` | where a cell's snow lies inside it, and its patches | high ground, shaded slopes and hollows keep snow longest | tuned, by eye |
| `ICE_*`, `FLOE_*`, `LEAD*`, `BRASH*`, `PACK_*`, `RIDGE_*`, `CRACK_*`, `LINE_FIELD_M`, `LINE_TURN`; `SEA_BLUR`, `SEA_REACH`, `OPEN_SEA_M` | `lib/ice.wgsl`; `web/src/land.ts` | ice on standing liquid: its edge, floes, leads, the pack | the share drawn matches the cells' within a few hundredths | tuned, by eye |
| `SNOW_PER_ICE` (3) | `lib/draw.wgsl` | m of snow per m of ice | settled snow about 300 kg/m³ against ice 917 (Paterson 1994, from memory) | anchored |
| `BURIED_FROM`; `CROWN_SNOW`, `SNOW_LIT`; `CANOPY_SNOW` | `lib/draw.wgsl`; `lib/models.wgsl`; `lib/cover_field.wgsl` | snow burying plants and lying on crowns | none | tuned, by eye |
| `BED_VARY`, `BED_VARY_M`, `BED_NEAR`, `BEND_OFF`, `POOL_NARROW`, `RIFFLE_WIDE`, `POOL_AT_BANK`, `POOL_SKEW`, `BAR_SAND_STONE`, `BED_MOST_M`; `RAG_M`, `RAG_SHARE`, `RAG_MOST_M`, `STEEP_STONE`, `BED_OUT`; `CHANNEL_NEAR` | `lib/rock.wgsl`; `lib/river.wgsl`; `lib/live_map.wgsl` | a bed's width along it, pools and riffles | pools at bends, riffles between, 5 to 7 widths apart (Leopold and Wolman 1957; Keller and Melhorn 1978) | tuned, by eye |
| `BAR_MOTTLE`, `BAR_PATCH_M`, `STRAND_M`, `STRAND_W`, `STRAND`, `STRAND_WAVE_M`, `BAR_GROWN`, `BAR_TUFT_M`, `BAR_LIFT`, `BAR_PALE`, `BAR_RISE`, `BANK_M`, `BANK_SHARE`, `BANK_SHADE`, `BANK_LOW`, `EDGE_M`, `EDGE_LIGHT`; `CUT_M`, `CUT_SHARE`, `CUT_STRAIGHT`, `CUT_DARK`, `LIP_DARK`, `ROOT_M`, `ROOT_DARK`, `DAMP_M`, `DAMP_SWING`; `MID_BAR_WD`, `MID_BAR_M`, `MID_BAR_FROM`, `MID_BAR_FULL`, `MID_BAR_WIDE`, `MID_BAR_UP`, `MID_BAR_PX`, `DRIFTWOOD_UP` | `lib/river.wgsl` | bars, banks, cut banks and mid-channel bars | the outer bank is cut, a point bar builds inside (Leopold and Wolman 1960); bars from width over depth 20 (Church 1983, from memory) | tuned, by eye |
| `LANE_M`, `LANES`, `STREAK_M`, `STREAK`, `FLECK_M`, `FLECK`; `REFLECT_STREAK`, `FAST`, `FAST_WATER`; `FLOW_FPS`, `FLOW_M_S`, `FLOW_SHOWS_E`, `FLOW_PX` | `lib/flow.wgsl`; `lib/river.wgsl`; `web/src/land.ts` | signs of a channel's flow and their motion | lowland streams run at 0.3 to 1 m/s (from memory) | tuned, by eye |
| `STEP_SLOPE`, `STEP_JUDD`, `STEP_WIDTHS`, `CASCADE_SLOPE`, `STEP_ROUGH` | `lib/flow.wgsl` | where a reach falls in steps, and their spacing | step spacing 0.31 S^−1.19 m (Judd 1964), 1 to 4 widths (Chin 1989) | anchored |
| `STEP_CELL`, `STEP_JITTER`, `STEP_RUN_CELLS`, `STEP_RUNS`, `STEP_TILT`, `STEP_BOW`, `STEP_WAVE`, `STEP_WAVE_M`, `STEP_SPAN`, `STEP_SPAN_SOFT`, `STEP_OFF`, `STEP_EDGE_M`, `STEP_FADE`, `STEP_TAIL`, `STEP_CALM`, `STEP_BACK`, `FOAM_STEEP`, `CASCADE_FOAM`, `STEP_POOL`, `POOL_CALM` | `lib/flow.wgsl` | where steps lie and how they look | none found | tuned, by eye |
| `RELIEF_LONGEST_M`, `RELIEF_STEP`, `RELIEF_OCTAVES`, `SWELL`, `CRAG`, `CRAG_STEEP`; `CLIFF_FROM`, `CLIFF_FULL`, `SCREE_FROM` | `lib/relief.wgsl`; `lib/crag.wgsl` | relief finer than the tiles; cliffs and scree | ground is about self-similar (Mark and Aronson 1984) | tuned, by eye |
| `GULLY_LONGEST_M`, `GULLY_STEP`, `GULLY_OCTAVES`, `GULLY_K`, `GULLY_DECAY`, `GULLY_STEEP_MOST`, `WALL_MOST`, `GULLY_PER_CELL`, `GULLY_JITTER`, `WAVE_VARY`, `BEND`, `NORM_SOFT`, `FORK_FADE`, `SHARP_FLOOR`, `FLOOR_HALF_M`, `ROUND_LONG_M`, `KEEP_FALL`, `KEEP_SOFT`, `GULLY_SHOW`, `GULLY_SEEN`, `SOIL_LOOSE_M`, `SOIL_STEEP`, `FAN_M`, `SOIL_SHORT_M`, `SOIL_SHORT_CUT`, `SOIL_ROUND`, `VARY_M`, `VARY_DEPTH`, `VARY_SHARP` | `lib/gullies.wgsl` | a branching net of gullies between the samples and the fine relief, sharp on bare bed, rounded on soil | water cuts slopes into gullies that join, creep rounds them (Horton 1945; Montgomery and Dietrich 1992) | tuned, by eye |
| `CALM_SLOPE`, `CALM_TWI`, `CALM_SHORE_M`, `CALM_CHANNEL_M`, `CALM_CHANNEL_GROW`, `CALM_Q` | `lib/gullies.wgsl` | where no gully is cut: gentle or wet ground, shores, valley floors | a channel carries about 0.008 m³/s at 1 km² drained, as `TYPICAL_RUNOFF` | anchored (the discharges); tuned, by eye (the widths) |
| `HOLLOW_TONE`; `SNOW_FOLD_M`, `SNOW_SPUR_M`; `FILL`, `FILL_SHARE` | `lib/style.wgsl`; `lib/land.wgsl`; `lib/style.wgsl` | darker hollows and lighter spurs, snow in hollows, a fill light on shaded ground | none: a hollow sees less sky; under a low sun the sky lights the far side | tuned, by eye |
| `PLACE_WARP_M`, `PLACE_WARP`, `LAID_PATCHY`, `LAID_AGE_PATCHY`, `LOOSE_SPREAD`; `TONE_SWAY`, `CAST_REACH_M`, `CAST_KNOB_PX` | `lib/rock.wgsl`; `lib/style.wgsl` | the samples' fields read at a place moved by noise; the soft edge of cast shade | none: a sample's square has no edge on the ground | tuned, by eye |
| `MAKE_RAGGED`, `MAKE_PATCH`, `MAKE_OCTAVES`, `MAKE_PX`, `MAKE_EDGE_PX`, `MAKE_EDGE_M` | `lib/rock.wgsl` | where two makes of the bed meet, ragged on rock and soft on soil | none: folded layers crop out in ragged bands | tuned, by eye |
| `BAND_M`, `BANDS`, `BAND_WOBBLE_M`, `BAND_PX`, `VARNISH`, `LICHEN_HUE`, `SCREE_LIFT`; `GRAIN_LUSH`, `GRAIN_PLACE`, `GRAIN_PATCHY`, `GRAIN_PATCH_M`, `GRAIN_OCTAVES`; `SOIL_LOW`, `SOIL_HOLLOW`, `SOIL_MOTTLE`, `SOIL_PATCH_M` | `lib/crag.wgsl`; `lib/rock.wgsl`; `lib/crag.wgsl` | bare bed's bands, varnish and lichen; grain sorted by relief; open soil's tone | varnish and lichen darken old rock, wash sorts fines into hollows (from memory) | tuned, by eye |
| `FORM_SCORES`, `BY_LEAF`, `BY_FROST`, `BY_WET`, `BY_DRY`, `BY_SEED`, `SPANS_MOST`, `FORM_SURE`, `TENDER_SPANS` | `web/src/form.ts` | a tree's crown form from its kind's genes | needle trees hold the cold and dry ends (Woodward 1987) | tuned, by eye |
| `STYLE` and the style's other numbers | `lib/style.wgsl`, `web/src/style.ts`, `lib/cover_field.wgsl` | how far the Earth view's style goes from the physical colours | none: a painted map's look | tuned, by eye |
| `COLD_NEAR_K`, `COLDEST`, `SHED_LEAF`, `RED_FROM`, `RED_TO`, `KEPT_DARK`, `KEPT_DULL`, `SWARD_L`, `SWARD_L_KEEP`, `SWARD_CHROMA`, `TREE_M`, `BUSH_WOOD`, `SOFT_MIX`, `SOFT_STANDS_M`; `SOFT_WOOD` | `web/src/style.ts`; `lib/models.wgsl`, `lib/plant.wgsl` | autumn colour, the sward's colour, tree or bush or soft plant | thin leaves shed in the season that cannot pay (Kikuzawa 1991; Wright et al. 2004) | tuned, by eye |
| `RAGGED_STEP`, `RAGGED_FALL`, `PATCH_SLOPE`, `MEET_OCTAVES`, `MEET_LEAST_M`, `SHARE_SPREAD`, `HEIGHT_RANGE`, `HEIGHT_MOST`, `HEIGHT_SHARP`; `PATCH_EDGE_M`, `CELL_EDGE_M`; `SOIL_EDGE`, `HEX_EDGE` | `lib/ragged.wgsl`; `lib/cover_field.wgsl`; `lib/detail.wgsl` | ragged meetings of looks, patches and cells | ecotones are tens to hundreds of m wide (from memory) | tuned, by eye |
| `CLUMP_M`, `CLUMP_STEP`, `CLUMP_FALL`, `CLUMP_OCTAVES`, `CLUMP_PX`, `CLUMP_HIDDEN`, `CLUMP_BLUR`, `CANOPY_FILL`, `LOW_FILL`, `LOW_SOFT`, `CANOPY_BUMP`, `LOW_BUMP`, `CLUMP_TONE`; `CANOPY_RIM`, `CANOPY_RIM_CROWNS`, `REED_FLAT` | `lib/clumps.wgsl`; `lib/cover_field.wgsl` | plants stand in clumps, groves and gaps that keep the cell's mean | none: woods seen from a plane are groves and clearings | tuned, by eye |
| `LUSH_TWI_MID`, `LUSH_TWI`, `LUSH_SUN`, `LUSH_PLACE`, `LUSH_LOW_M`, `LUSH_SPUR`, `LUSH_FOLD`, `LUSH_SUM_MOST`, `LUSH`, `LUSH_HUE` | `lib/lush.wgsl` | how lush a point is inside its cell: wet, shaded and hollow ground holds more plants | none: plants follow drainage lines and thin on spurs | tuned, by eye |
| `MIDFLAME`, `LB_A` to `LB_E`, `LB_MOST`, `FRONT_M_S`; `BURN_RAGGED`, `BURN_EDGE_M` | `web/src/burns.ts`; `lib/fire.wgsl` | the shape and play of a burn inside a cell | fire ellipses of length over breadth 1 to 8 (Anderson 1983; Alexander 1985) | anchored (the shape); tuned, by eye (the rest) |
| `PLANT_GLOW`, `PLANT_GLOW_LEAST`, `BURNING_M`, `SAME_BURN_MONTHS`, `ASH_HIDES_KG`; `DEAD_AFTER`; `LOW_REGROW_MONTHS`, `BARE_COVER`, `DEAD_STAND_MONTHS`, `ASH_DRIFT_M`; `CHAR_OF_WOOD` | `lib/fire.wgsl`; `lib/mark.wgsl`; `lib/burn.wgsl`; `lib/snag.wgsl` | plants in a passing front, the dead, the scar and its ash | a grain-deep layer of fine ash is about 0.08 kg/m² (Bodí et al. 2014) | tuned, by eye |
| `REED_BAND_M`, `REED_HERBS`, `REED_CELL_M`, `REED_SOUTH`, `REED_SHADOW`, `CATTAIL_M`, `REED_STRAW`, `REED_STEM`, `RUSH_CELL_M`, `RUSH_SOUTH`, `RUSH_SHADOW`, `REED_STANDS`, `REED_CLUMP_M`, `REED_CLUMPS`, `REED_CLUMP_EDGE`, `TUFT_CELL_M`, `TUFT_COVERS`, `TUFT_ASPECT`, `TUFT_SOUTH`, `TUFT_SHADOW`, `FERN_STANDS`, `FERN_CELL_M`, `FERN_SOUTH`, `FERN_SHADOW`, `STAND_CALM_E`, `THIN_COVER`, `THIN_FROM_M`, `THIN_TO_M`, `STAND_M`, `SWARD_STAND_M`, `STAND_MIX` | `lib/cover_field.wgsl` | reeds, rushes, ferns and tufts; how stands mix and fade | marsh grasses 1.5 to 3 m in clumps (from memory) | tuned, by eye |
| `THING_PX`, `CROWN_PX`; `CROWN_LEAST_PX`; `REACH`, `ROWS_MOST`, `SPRITE_WIDER`; `MODEL_WIDER`, `FOOT_FLARE`, `FOOT_RISE`, `FOOT_TRUNK`, `FOOT_FINS`, `FIN_SHADE`, `FOOT_BARK`; `WATERLINE` | `lib/draw.wgsl`; `web/src/textures.ts`; `web/src/lattices.ts`; `lib/models.wgsl`; `lib/style.wgsl` | when a thing or crown is drawn one by one, how far marks reach, swamp trees' feet, a stem's waterline | none: a mark of one or two pixels reads as a speck | tuned, by eye |
| `FRAME_BUDGET_MS`, `SCALES` | `web/src/map.ts` | a slow frame is drawn at fewer pixels | 60 frames a second is about 16 ms | tuned, by eye |

## Chemistry data that life lives by

Chemistry 0 (`worldgen/src/chem0.toml`) is data, not a struct, so the test does not see it. These rows are the numbers
that decide what bodies can do and the liquid's store of the carbon gas. They are set from facts of matter and the
budget does not count them; the decomposers' factors `open` are free numbers of life and count as anchored. A value
called measured is for the state the substance is in at 290 K; the data's `hf` is the solid's (laws 2.1). `ew chem`
prints the file.

| name | value | meaning | anchor |
|---|---|---|---|
| substance 10, body matter | C120H198N6O107Ca (3474 g/mol), hf −20,601.8 kJ/mol, cp 4282 J/(mol K), 1500 kg/m³ | what bodies are built of, with no heat in or out (`Chemistry::build`); 15.9 MJ/kg burnt; C:N 20 | like cellulose (17.5 MJ/kg); foliage C:N 43 (McGroddy et al. 2004) |
| substance 11, the salt that exists only dissolved | Ca(HCO3)2 (162 g/mol), hf −1929.37 (data), cp 194, melts at 271 K | holds the liquid's carbon beyond the free gas; also an input of growth | Ca2+(aq) and 2 HCO3−(aq): dissolving lime takes −40.3 kJ/mol (real −40.1) |
| substance 12, the nitrogen mineral | N2H4O3 (80 g/mol), hf −370.9 (data; −365 measured), melts at 255 K, never boils | the soil's nitrogen, the only form a body is built of; lives dissolved | ammonium nitrate: hf −365.6; its solution freezes near 255 K |
| substance 13, the frame | C6H10O5 (162 g/mol), hf −986, cp 210, 1500 kg/m³ | a solid bodies build of 6 of the energy-rich solid, no heat in or out | cellulose, 17.5 MJ/kg (from memory) |
| substance 14, the slow organic matter | C59H78N6O42 (1542 g/mol), hf −6098, cp 1900, 1400 kg/m³, `bound` | what rotting body matter leaves on fine particles: half its carbon, all its nitrogen (C/N 10) | soil organic matter, C/N 10 to 12 (from memory) |
| substances 15 to 22 | Fe, Fe2O3, Cu, CuO, C, NaCl, Al2Si2O5(OH)4, Al2Si2O7 | free elements 6 and 7 and their oxides; the solid of element 1 (charcoal); a crystal of 8 and 9; a clay and the clay baked | real formulas; their `cp`, `melt`, `boil`, `rho`, `hf`, `lf`, `lv`, `cond` measured (from memory); `hard` and `str` guessed on the rocks' scale |
| substances 23 and 24 | C15H12O6 (288 g/mol, hf −552); C20H27NO11 (457 g/mol, hf −1496.7); cp 375 and 590 | solids life may build to slow a reaction (`inhibit`); hf set so building takes no heat | a condensed tannin unit; amygdalin (from memory) |
| `Chemistry::build`, growth | 118 of 5 + 3 of 12 + 1 of 11 → 1 of 10 + 26 of 0 | what `cover::build` runs, paid by `c_copy`; 12 and 11 are the minerals (`Chemistry::minerals`) | chemistry data |
| `Chemistry::builds` | 6 of 5 → 13 + 0; 15 of 5 → 23 + 9 of 0; 43 of 5 + 1 of 12 → 2 of 24 + 3 of 3 + 18 of 0 | the other solids `cover::build` builds; no upkeep | chemistry data |
| soil, `worldgen::Init::soil` | 0.357 mol of 12 per m² of land, when life starts | 10 g of nitrogen per m² | mineral nitrogen of a soil, 1 to 10 g/m² |
| reaction 1, burning the energy-rich solid | k 1.72e4 /s, Ea 7760 K, `open` 200 | bare 0.27 a year at 290 K: the litter constant; `open` makes it about a week | forest floor 0.25 a year (Olson 1963); free sugar respired within days (Gunina and Kuzyakov 2015) |
| reaction 7, burning body matter | k 2.28e9 /s, Ea 9000 K, orders 1 and 6, `open` 5 | bare 0.2 a year; `open` 5 gives 1.0 a year, 3.8 times the frame's | Century's metabolic pool, 3.8 times its structural pool (Parton et al. 1987) |
| reaction 8, dissolving | k 1.62e-2 /s, Ea 2800 K | lime + gas + liquid → the salt 11 | Earth's gross air-sea exchange, 80 PgC a year (Ciais et al. 2013) |
| reaction 9, settling | k 1.8e12 /s, Ea 7647 K | 3 of 11 → back; balance with reaction 8 | Ea(9) − Ea(8) = dH / R; 1.0 mol/m³ at 288 K, 400 ppm (the sea holds 2.0) |
| reaction 10, fixing the air's nitrogen | 2 N2 + 28 CH2O + 29 O2 → 2 of 12 + …, k 8e-8 /s, Ea 6600 K | 6 g C per g N; keeps energy (`Chemistry::keeps`) | symbioses spend 4.7 to 6.6 g C per g N (Layzell et al. 1979) |
| reaction 11, the mineral breaking down hot | k 2.6e9 /s, Ea 20,000 K | nitrogen leaves as gas where it is hot | the salt breaks down above 450 K; fires lose most nitrogen |
| reaction 12, the frame rots | k 1.72e-2 /s, Ea 7760 K, `open` 1e6 | bare 2.7e-7 a year: living frame does not rot; `open` gives reaction 1's 0.27 a year | Olson's litter constant (1963) |
| reaction 13, an enzyme splits the frame | k 9.6e-4 /s, Ea 7760 K | 2.3e-15 /s bare; no law brings a catalyst | 4.7e-15 /s at 25 °C (Wolfenden, Lu and Young 1998, from memory) |
| reaction 14, body matter rots into the slow organic matter | k 5.7e8 /s, Ea 9000 K, `open` 5 | a quarter of reaction 7's rate | 5 to 15% of litter carbon enters CENTURY's slow pool (Parton et al. 1987, from memory) |
| reaction 15, the slow organic matter rots | k 9.12e7 /s, Ea 9000 K, `open` 5 | 25 years at 290 K moist, slower by `SHELTERED` | the slow pool turns over in 20 to 50 years (Parton et al. 1987, from memory) |
| reactions 16 to 21 | 16 charring the frame (k 2.8e19, Ea 29,000 K); 17 burning charcoal (2.3e8, 20,000 K); 18 freeing element 6 (1.05e7, 30,000 K); 19 freeing element 7 (7.3e6, 25,000 K); 20 baking the clay (3.7e10, 25,000 K); 21 element 6 rusting (0.044, 5000 K) | fast only in a flame's or a kiln's heat (21 at the ground's) | 16 physical (Bradbury, Sakai and Shafizadeh 1979); 17 and 20 anchored (Smith 1978; from memory); 18, 19 and 21 tuned |
| reactions 22 and 23, the solids 23 and 24 rot | k 1.72e4 /s, Ea 7760 K, `open` 200 | about a week in moist ground; 24 gives back its nitrogen as 12 | real tannins rot slower; tuned, first guess |
| `inhibit` | 23 slows 13 (Ki 0.1 mol/kg); 24 slows 1, 2 and 7 (Ki 1e-5) | at Ki the reaction runs at half pace (`Reaction::inhibited`) | tannins halve digestion at a few % (Robbins et al. 1987); 1 to 3 mg/kg of cyanide kills a mammal (from memory) |
| `rupture` of substances 10 and 13 | 1 and 120 MPa | stress each breaks at (`Cover::breaking`) | soft tissue 0.1 to 10 MPa; cell wall about 120 (from memory) |
| `shear` of substance 13 | 60 MPa | stress to cut the frame across its grain (`contract::cut`) | green wood 15 to 20 MPa at 500 kg/m³, × 3 for the wall (from memory) |
| `rupture` of substances 7, 8 and 9 | 10, 15 and 7 MPa | tensile strength of the rock solids; no law reads it | rock 4 to 25 MPa (Jaeger, Cook and Zimmerman 2007, from memory) |
| `rupture` of substance 15 | 250 MPa | tensile strength of element 6; no law reads it | wrought iron 250 to 350 MPa (from memory) |
| `hard` of substances 10 and 13 | 0.2 and 0.2 | scratch hardness: no part of a body cuts a body | as the file gives them |
| `sol` of substance 20 | 0 | the crystal does not dissolve; the sea stays nearly fresh | no law dissolves a solid that melts above the ground's warmth |
| `sw`, `swsat` of substance 0 | 1.55e-4 m²/mol, 550 mol/m² | sunlight its vapour absorbs on a path | fitted to Lacis and Hansen (1974), 12% rms |
| `heat_bands` | [[8, 12.5], [12.5, 18]] µm | the bands of heat radiation (the third all others) | the vapour's window and gas 3's band at 15 µm |
| `heat` of substances 0, 3 and 4 | 0: [0.362 m²/mol, 46.9 mol/m²] in band 3; 3: [0.447, 3.63] in band 2; 4: [0.25, 0.1] in band 3 | lines' optical depth in each band | 0 and 3 fitted with `line_spread` (see its row); 4 none found, tuned, first guess |
| `cont` of substance 0 | 4.4e-7 m²/(mol Pa) | its continuum | the self continuum (Roberts et al. 1976, not read); tuned, first guess |
| `ray` of substances 1 and 2 | 2.62e-7, 2.25e-7 m²/mol | Rayleigh scattering per mol | Hansen and Travis (1974) over a 5778 K sun; oxygen 0.86 of nitrogen (Bates 1984) |
| `dense`, `dense_drop` of substance 0 | 277 K, 0.215 K per g/kg | temperature of densest liquid, and its fall with what is dissolved | water at 3.98 °C (Kell 1975); Despretz 1839 |

## Real figures to compare runs with

Not rows of the model: real sizes a run's results can be held against.

- **Large plant-eaters:** African large herbivores 53 to 11,810 kg/km², typical 1,700 (Hempson et al. 2015). Damuth
  1981: log10 (count per km²) = 4.23 − 0.75 log10 (mass in g).
- **Soil animals on dead matter:** earthworms of grassland 10 to 100 g/m² (Edwards and Bohlen 1996, from memory).
- **Adult survival:** bison cows 0.92 to 0.95 a year (Fuller et al. 2007); wild deer 0.83 to 0.91 a year.
- **P/B a year:** mouse 3 to 5, rabbit 1 to 2, deer 0.3 to 0.5, bison 0.15 to 0.25 (from memory; read Banse and Mosher
  1980).

## Counts

The 134 rows of Config, Speed settings and Physics are the 134 saved fields of `Config` and `Physics`.

| mark | fields | rows outside the structs (laws and start; drawing) |
|---|---|---|
| physical | 7 | 9; 0 |
| anchored | 105 | 39; 13 |
| extrapolated | 0 | 1; 0 |
| tuned (and tuned, by eye) | 6 | 13; 28 |
| tuned, first guess | 12 | 19; 3 |
| speed | 2 | 3; 1 |
| setting | 1 | 0; 0 |
| container | 1 | 0; 0 |

Rows marked tuned: 18 fields, and 63 rows outside the structs (32 of the laws and the start, 31 of the drawing). A
drawing row groups the constants of one look. Rules chosen for a run: 1.
