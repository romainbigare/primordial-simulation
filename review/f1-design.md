# F1: grazers. A design on paper

For the orchestrator, then a builder. Simple English. Read against `main` at commit `05fcb31` (seeds on the ground and
snow are merged there). "Checked" means I read the source or the code. "From memory" means I did not. "Not checked"
means I did not look in the code.

Words: a **taker** is an animal kind. A **slot** is one kind of taker in one cell. A **layer** is a part of the cell
that a taker can take from (a slot of cover, the seeds lying). **Members** are the animals of a slot (a real count).
**Living mass** is body matter with the liquid of living tissue (matter / (1 − `WET_TISSUE`), as the cover's).

## The orchestrator's decisions on this design

The design is accepted as the plan of F1, with these decisions. Where the text below says "open" or "the owner's
call" on one of these points, this list holds.

1. **One parent** may breed in F1. A law of how often adults meet belongs to F2 (it needs movement).
2. **Thirst is a law of F1** (the owner's word; roadmap 6, item 3). It replaces what sections 1.1 and 2 (step 6) say
   of where a body's liquid comes from:
   - **A body drinks open liquid only**: the liquid standing on a cell and the liquid running over it in the month.
     The liquid held in the ground's pores is the cover's, not a mouth's. `contract::liquid` (gap G4) gives the two
     apart. A body also has the liquid of what it eats (the share `WET_TISSUE` of the living mass taken) and the
     soft solid lying on the ground, paying its heat of fusion.
   - **The need** is the vapour that cools it (step 6) plus a least need by body mass for breath and what it voids
     (Nagy and Peterson 1988, a power of the mass near 0.8; the builder checks the source before coding; one
     anchored row in `docs/numbers.md`). Only the vapour is booked; what is drunk and voided in the same month goes
     back to the cell.
   - **The way to the liquid is paid by the law of walking.** A member whose cell holds no open liquid in the month
     walks to the nearest cell that does and back, as often as its need and its body's store of liquid ask; the
     energy is work it burns. Where the way takes more of the day than the day has, the slot's work is cut by the
     share of its need it cannot meet (as step 6 does today), and the members it cannot keep die as `LACK`. Nothing
     says "live near a river": the cost of the way does.
   - **No term for what is dissolved yet.** The low liquid of chemistry 0 is nearly pure, so a shore is a place to
     drink. The term comes with the work that makes it truly salty (roadmap 6, item 3, second step); a number that
     never bites is not added before.
   - The gate gains one row: on a land with a dry season, the living mass of takers per km² falls with the way to
     open liquid for the large kinds, and less or not at all for kinds under 1 kg (which live on their food's
     liquid). Adults that move to the liquid as a herd belong to F2: open liquid is then one of the things sensed.
3. **Small kinds** (under 1 kg) may swing; none may die out from the whole land (gate, row "hold each other", c).
4. **`bite_floor`** is the refuge the whole stage rests on, and its anchor is weak. The gate is run at half and at
   twice its value too, and the three results are reported. If plants and takers hold each other only in a narrow
   band of it, the number is tuned and is marked so; the count of tuned rows may not grow without a word.
5. **Young disperse by the seeds' kernel in F1**, as the design argues (build plan, commit 5): without a way back,
   the stage would measure the size of a cell, not the ecology.
6. **Slots of takers never join by `room`** (the nearest pair when all are held). They join only when alike in every
   gene.
7. **The growth cap's energy per kg** is checked against West, Brown and Enquist (2001) before step 7 is coded.
8. **The mineral does not limit a taker** in chemistry 0 (plant and animal matter are one substance). This is known
   and accepted for F1; a second body matter is chemistry work for a later stage.
9. **The numbers of the gate are judged on the new air** (the true light changes how much the cover grows). Commits
   1 and 2 of the build plan do not wait for it.
10. **After F1's gate, the first drawing of animals on the page comes before F2** (single animals and soft marks
    from the field's counts, each at a place fixed by its cell, slot and number): the owner judges by pictures.

## 0. In short

- F1 is a second field next to the cover: `Fauna` in `life::Cell`, 8 slots, 8 genes, one month's pass per cell, run
  in the same chunk pass as the cover, right after the cover's own month.
- It reuses the laws of roadmap section 13 almost unchanged: Kleiber's upkeep, pace by temperature, the heat balance
  through a coat, the walk (now only for searching inside the cell), wear by the rate of living, frost, the exact
  split and join. Two section-13 numbers go: `sweep` and `c_handle` (both tuned). They are replaced by laws that use
  the walk and the breaking stress. One section-13 law fails a check: `c_build` (section 2, step 7). I replace it.
- The take is the disk law of Holling with many layers and many slots. Inside the month it runs in 6 steps of 5 days,
  each step solved in the exponential form. It never overdraws and needs no ordering of slots.
- **The real brake on overgrazing is not the time scheme. It is a refuge.** A mouth cannot bite below a floor. That
  floor is a share of the body's length, plus the snow. With one helper (`contract::above`, already on main for snow),
  this gives a stubble that no taker can reach. Without it, every cell is eaten bare in days (section 3).
- I argue that F1 **needs** dispersal of young by the seeds' kernel. Without it, the large kinds die out cell by cell
  by chance within decades, and never come back. Then the 500-year gate measures that loss, not the ecology.
- Three gaps in the land must close before or during F1 (section 7). F1 can start without the others.

## 1. The state

### 1.1 Which substances a slot holds

The cover holds three parts: fuel (5), body matter (10), walls (13). A slot of takers holds **two**: fuel (5) as its
store, and body matter (10). No frame (13): an animal of F1 builds none. Its frame of bones and shells is for later
(F3, when eaters must break it).

Why fuel 5 and not the store-like substance 6? Substance 6 holds more energy per kg. On Earth, that is why animals
store it: it is lighter to carry. But in F1 nothing moves between cells, so carrying costs nothing. Making 6 from 5
(reaction 3) only loses energy. So F1 stores 5, and the `Parts` of the cover serve both. Revisit in F2, when the walk
costs mass.

**A body's liquid: implied, as the cover's.** Decision: a member's living mass is its matter / (1 − `WET_TISSUE`)
(0.75). The liquid is not in the books.

The reasons:
1. The cover does the same. One rule for living tissue (`life::WET_TISSUE`).
2. The liquid a body takes in and gives back in the same month nets to zero in the books. Food's liquid, drink, urine
   and droppings all come from the cell's ground and go back to it.
3. What does **not** net to zero is booked: liquid that leaves as vapour to cool a warm body (section 2, step 6). It
   is taken from the cell's ground, and its latent heat is paid from the body's heat. Also booked is the liquid the
   reactions take or give (reaction 13 takes 1 per unit, burning gives it).
4. Size check: 50 grazers of 100 kg per km² drinking 10 L a day each move 0.18 mm a year. The rain is 1,000 mm. Booking
   all of it would cost work and change nothing.

Thirst is still a law: a warm body that must shed heat by evaporation and finds no liquid in its cell cannot shed it.
So its work is cut (section 2, step 6).

### 1.2 The struct

`life::Cell` gets `fauna: Fauna`. One writer: the chunk of the cell.

| Field | Type | Unit | Meaning |
|---|---|---|---|
| `adults[FS]` | f64 | members | real count; whole below `N_WHOLE` |
| `young[FS]` | f64 | members | real count; whole below `N_WHOLE` |
| `mol_a[FS][NF]` | f64 | mol | adults' fuel (5) and matter (10) |
| `mol_y[FS][NF]` | f64 | mol | young's fuel and matter |
| `e[FS]` | f64 | J | heat of each slot (its members at their own temperature) |
| `genes[FS][NGF]` | f32 | by gene | below |
| `root[FS]` | u32 | label | starting kind (`NO_ROOT`: empty); for the tape only |
| `net[FS]` | f32 | J per kg living per month | running mean over a year: digested less upkeep (as `Cover::net`) |
| `young_out` | `Vec<Brood>` | not saved | young that disperse, landed after all chunks (as `Cover::seeds`) |

`FS = 8`, `NF = 2` (fuel, matter). Matter and energy are 64-bit (`CLAUDE.md` 3.1). Genes and the balance hold
nothing: 32-bit. Each slot has its own heat, because a warm slot and a cold slot in one cell stand at different
temperatures. The cover has one heat for all its slots, because all its slots stand at the ground's temperature.

Mean mass of a young: `m_y` = its living mass / `young`. Mean mass of an adult: `m_a` likewise. Adults stand near the
gene's adult mass, but not exactly: starving adults keep their count and lose their store first.

### 1.3 The genes

Eight genes. Each is a log10 or a plain number with a step and a range, as `cover::RANGE`. The mutation size per
generation is the cover's (`cover_mut` 0.063 steps). I propose to rename it to one shared `mut_size`.

| Gene | Unit | Range | Step | What it does | Its cost |
|---|---|---|---|---|---|
| `MASS` | log10 kg, living adult | −1.5 to 3.5 (30 g to 3 t) | 0.05 | adult size; with it Kleiber, the walk, the coat's surface, the reach, the floor | slower young (more months to mature), fewer members (more chance to die out), a higher bite floor (cannot reach the low stubble), less fuel per kg in a lean month |
| `CHILD` | log10 share of adult mass at birth | −3 to −0.5 | 0.05 | size of a newborn | a large child costs more matter per birth: fewer births. A small one is longer young; small young lose more heat and pay more upkeep per kg |
| `YOUNG` | share of surplus | 0 to 1 | 0.05 | share of the month's surplus put into young (growth of young, then births); the rest goes to the store | a high share leaves no store for winter; a low share makes few young |
| `ENZYME` | log10 mol of frame split per kg living per s at `t_cat` | −12 (off) to −5 | 0.2 | capacity of reaction 13 in the gut: frame becomes fuel | upkeep: `c_light` of what the capacity could free, as the cover pays its light capacity (section 2, step 4) |
| `REACH` | log10 m | −2 to 1 | 0.05 | middle of the band of heights it searches; the band is ± `BAND` (0.5 decade) | what lies out of the band is not found. The top is clipped at what its body reaches (`reach_len` × L) |
| `KEEP` | K | 250 to 315 | 1 | body temperature it keeps by burning, when the ground is colder | the heat lost through the coat must be paid with fuel; a warm body's upkeep runs at its own pace (5.6 times `t_cat`'s at 310 K) |
| `COAT` | share of `r_coat` | 0 to 2 | 0.05 | insulation of the coat | in warm months, heat that cannot pass the coat must leave as vapour: liquid and latent heat; where no liquid, work is cut |
| `REPAIR` | log10 | −3 to 3 | 0.2 | wear life is 10^REPAIR times the rate of living's | upkeep × (1 − `c_repair` + `c_repair` 2^REPAIR) (`Physics::repair`, shared) |

A cold-bodied kind is not a separate gene. It is a kind whose `KEEP` is below its ground's temperature. Its body then
stands at the ground's temperature and runs at the ground's pace. So "warm or cold" is one axis. The heat balance
gives each place on that axis its price.

Genes F1 cannot test, so F1 does not have them: group size, prey by size, the place inside a cell, range per day,
strength (no fights, no pulling), habits.

## 2. The month's pass for one cell

It runs in `life::run`, right after `cover::step` on the same cell and the same view of the store. So the cell has one
writer, and the fauna sees the cover as its month has just left it. The cover feels what was taken in its own next
month (contract rule 3). Notation: `n` members of mass `m` (living kg), `T_b` body temperature, `t` the ground's
temperature (`contract::place`), `dt` the month (s), `days` = dt / DAY. Mark: **[13]** a law of roadmap section 13,
**[new]** new, **[cover]** the cover's law reused.

**Step 0. Body temperature and pace.** [13] `T_b` = `KEEP` if `KEEP` > t, else t. pace(T) = e^(−Ea/T) / e^(−Ea/`t_cat`)
of reaction 1 (`Reaction::arrhenius`, as the cover uses it). Work rates (walk, chewing, building) use min(1, pace);
upkeep uses pace in full. Check: 310 K gives 5.62, 280 K gives 0.38. So a cold-bodied kind at 280 K pays 1/15 of a
warm one's upkeep, and works at 0.38 of its speed.

**Step 1. What the cell shows.** [contract] For each layer: `contract::shown` (kg, mol by substance, within the reach
above the snow), its height (`contract::slots`), and **its breaking energy per kg** (gap G2): soft tissue at
`rho_body` and 1 MPa, walls at their density and 120 MPa, the formula of `Cover::breaking`. Layers in F1: each slot of
cover, and the seeds lying (`Layer::Seeds`, height 0). Size check from chemistry 0: 1 kg of dry matter is 4 kg living,
4e-3 m³ × 1e6 Pa = 4e3 J. 1 kg of walls is 6.7e-4 m³ × 1.2e8 Pa = 8e4 J. A herb (half and half) costs 4.2e4 J per kg
to break. A tree slot (walls 10 times matter) costs 7.3e4 J per kg.

**Step 2. What each slot reaches of each layer.** [new] For a member of length L = (m / `rho_body`)^(1/3):
- its floor: f = `bite_floor` × L + snow depth (`Place::snow`). A mouth cannot bite below it.
- its band: from 10^(REACH − BAND) to min(10^(REACH + BAND), `reach_len` × L).
- a plant's matter is spread evenly over its height h (the rule `contract::above` already uses for snow). So the share
  of a layer this member reaches is w = (min(top, h) − max(bottom, f))⁺ / h. The seeds lying (h = 0) are reached
  where the band's bottom is under the floor of a small mouth and no snow lies: w = 1 if f < `GROUND_M` (1 cm), else 0.

One helper: `contract::above(floor, height)` grows into `contract::reached(band, floor, height)`. Snow and the bite
floor are the same thing to it.

**Step 3. The take: Holling's disk law, many layers, many slots.** [13, with two numbers replaced]
- Search rate a = v × L, m²/s, with v = √(`WALK_FROUDE` g L) × min(1, pace) [13]: the strip a member walks is as wide
  as its body. (This replaces the tuned `sweep`.)
- Handling time of layer l: h_l = b_l / P, s per kg, with b_l the breaking energy per kg (step 1) and P = `chew` ×
  m^0.75 × min(1, pace) the chewing power, J/s. (This replaces the tuned `c_handle` and `take`.) `chew` is anchored on
  the measured intake of grazers on grass, 42 to 80 g dry per kg^0.75 a day (van der Merwe and van Niekerk 1980, as
  section 13 cites; from memory): 0.07 kg × 4.2e4 J/kg = 2.9e3 J per kg^0.75 a day, 0.034 W per kg^0.75.
- Working time T = `busy` × DAY, s a day. `busy` 0.5: grazers feed 8 to 13 hours a day (from memory).
- Per member, a day, from layer l (D_l the shown kg per m² of the cell):
  I_l = T a w_l D_l / (1 + a Σ_k w_k D_k h_k), kg a day.
  Units: m²/s × kg/m² = kg/s; × s/day = kg/day. The denominator: m²/s × kg/m² × s/kg = no unit. Checked.
  When food is thick, I tends to T / Σ(handling): the chewing cap. When it is thin, I = T a w D: thin bites are slow
  to find. Search time a day: T_s = T / (1 + a Σ w D h).
- **Who gets what when they ask for more than is there:** by share, not by order. Over a step of Δ days, the layer
  falls by the rate k_l = Σ over slots and classes of n I_l / (D_l A), per day, A the cell's area:
  taken_l = S_l (1 − e^(−k_l Δ)), S_l the reachable stock, kg.
  Each slot gets its part of k_l. The last slot takes what the others leave, so the shares add up exactly.
  1 − e^(−x) < 1 for every x, so no layer is ever overdrawn, at any Δ, in any order.
  Why share and not order: an order by slot index gives slot 0 the first bite for no physical reason, and the index
  changes when slots join.
- The month runs in `SUBSTEPS` = 6 steps of 5 days (section 3). The denominators are found again at each step's start.
- The integrator works on a copy of the shown numbers. At the end, **one** `contract::take` per layer per substance
  asks for the total taken. The layer gives its parts in its own proportions (gap G1). What is granted is shared among
  the slots by their shares.

**Step 4. Digestion.** [new, with the chemistry's own reactions]
- Fuel (5) eaten: the slot's fuel.
- Matter (10) eaten: it is the same substance as the body's. It is used to build young (step 7). What is not built is
  burnt by reaction 7, which gives back 3 mol of the mineral (12) per mol. The mineral goes to the ground at once (as
  urine).
- Walls (13) eaten: split by reaction 13 into 6 fuel (it takes 1 liquid from the ground), at most
  X = 10^ENZYME × (living mass of the slot) × pace(T_b) × dt mol. The rest passes through as droppings: walls to the
  ground at once, with their heat at T_b.
- The enzyme's upkeep: `c_light` × 10^ENZYME × 6 e_5 × DAY × pace × repair, J per kg living a day (e_5 the fuel's
  content, `Parts::content`). This is the cover's capacity cost (`upkeep_rates`). I propose to rename `c_light` to
  one `c_capacity` used by both.
- Size check: a 100 kg grazer that splits 1 kg of walls a day needs 6.2 mol / (100 kg × 86,400 s × 5.62) = 1.3e-7
  (gene −6.9). Its upkeep: 0.1 × 1.75e7 J = 1.75e6 J a day = 20 W, against a resting 110 W. So the enzyme costs about
  18% of rest. On Earth, the methane and heat of fermentation lose 10 to 15% of what is eaten (from memory). The order
  fits.
- **Size helps digestion by itself.** Capacity goes with living mass (m), intake with m^0.75. So a large taker splits a
  larger share of what it eats for the same gene. This is the Jarman-Bell effect, and nobody wrote it.

**Step 5. Upkeep.** [13] Per class (adults, young), J over the month:
U = days × n × [ m^0.75 × (`c_basal` + `c_muscle`) × pace(T_b) × repair + enzyme + walk ] + heat.
- Kleiber: `c_basal` 4.22e4 (shared with the cover) + `c_muscle` 1.12e4 (section 13's `c_strength` at strength 1: F1
  has no strength gene). At 310 K this is 3.0e5 J a day per kg^0.75: Kleiber's (checked in `docs/numbers.md`).
- Walk while searching: v × T_s × `c_move` × (m / `MOVE_REF`)^`MOVE_EXP` × m [13]. It is large only when food is thin:
  this is the brake "bites get thin and slow to find".
- Size check of all of it: Damuth 1981 (checked) says free-living mammals spend 1.5 to 3 times their basal rate. Here
  the walk, the heat and the enzyme add to Kleiber's rest. A measure of the built model.
- Upkeep is paid by burning: first the fuel eaten and split, then the matter eaten that step 7 will not build, then
  the slot's store. With the cover's `Books::room` for the gas of the cell's share of its air node.

**Step 6. Heat and thirst.** [13, closed over the month]
- Loss to the ground per member: G = g (T_b − t), with g = `meeh` m^(2/3) / (`r_air` + COAT × `r_coat`) W/K [13].
  Over the month, the ground at its monthly mean: the closed form of the balance at steady state. A warm body reaches
  its steady state in hours, so the month needs no steps.
- If the heat of upkeep H < G: it burns G − H more (it keeps `KEEP`).
- If H > G: the excess H − G leaves as vapour: (H − G) / λ(T_b) mol of the key liquid, taken from the cell's ground
  (liquid first; frozen liquid too, paying its fusion heat), given to the aside node, the latent heat paid from the
  body's heat. That is the cover's `Moves::gas` path.
- If the ground cannot give that liquid: the share it can give limits the slot's work this month (search, handling and
  building scale down together). The members it can no longer feed die as `LACK`.
- Size checks. 100 kg, COAT 0.5: g = 2.15 m² / 0.57 = 3.8 W/K; at 273 K ground, G = 140 W against 110 W of rest: it
  burns about 30 W more. At 290 K ground, the excess is 220 − 76 W at a field rate of 220 W: 280 mol, 5 kg of liquid a
  day. A 30 g member with COAT 0.2: g = 0.032 W/K; at 273 K, 1.2 W against 0.25 W of rest: five times its rest. Mice in
  the cold run at 4 to 5 times their rest (from memory).
- A cold-bodied slot (T_b = t) has G = 0. All its heat goes to the ground. It never needs to evaporate.
- The slot's heat `e[x]` ends at its heat capacity × T_b. The difference goes to or comes from the ground node, as the
  cover's `book` does.

**Step 7. Growth of young, births, the store.** [13 for the brood and the split; the cap is new]
- Surplus: the energy of the month digested less upkeep (positive), and the matter eaten.
- **The share `YOUNG` of the surplus** builds young. First the young that live grow, then births. The rest goes to the
  store, up to `STORE_MOST` × the lean mass of the slot. What is over that goes to young too.
- Building a kg of matter costs `c_copy` (1e7 J) of work [cover], burnt from fuel. The matter comes from matter eaten.
  So growth needs matter in the diet: that is the mineral limit of F1 (body matter holds the mineral; fuel and walls
  hold none).
- **The cap of building** [new]: a member builds at most B0 × m^0.75 × min(1, pace) / E_c kg a day. B0 is Kleiber's
  rate, E_c the energy to make a kg (West, Brown and Enquist 2001, ontogenetic growth: from memory). This is one cap
  for the young's growth and for the matter a mother puts into newborns.
  Check: a 10 kg young, B0 = 3e5 J per kg^0.75 a day at 310 K: 1.7e6 J a day. With E_c = `c_copy` per kg dry, it
  builds 0.17 kg dry, 0.68 kg living a day. Calves gain 0.5 to 1 kg a day (from memory). With E_c including the
  matter's content (2.7e7 J), it builds 0.25 kg living a day. **This choice must be checked against West 2001 when
  built.** Either way it is from Kleiber, with no new number.
- **Why not section 13's `c_build`:** read as written (1e4 J per day per kg^0.75, over `c_copy` 1e7 J per kg), a 10 kg
  young builds 5.6 g a day, 100 times too slow. I could not see the old code's units ("supply"). Not checked at the
  tag. Do not reuse it without reading `16a774d`.
- Births: per adult, the matter for newborns is at most its building cap. The newborns are (matter put in) /
  (child matter: 10^CHILD × 10^MASS × (1 − `WET_TISSUE`)). Whole by a keyed draw below `N_WHOLE` (section 4). Only
  adults give birth, one parent (as the spec's population nodes, 6.4).
- **Maturing** [13: von Foerster's flux across a size]: the young's masses are taken as even in log mass between
  the child's mass m0 and the adult's M. Then the share that crosses M in the month is
  min(1, Δln m / ln(M / m0)), with Δln m the month's growth of the mean young. They join the adults with their matter
  (the young's matter × that share). Whole by a draw below `N_WHOLE`.

**Step 8. Deaths.** [13 and cover]
Each cause is a share of the members, combined as the cover combines them (each on what the one before left):
- `SPENT`: the share of upkeep not paid.
- `AGE`: 1 − e^(−wear), wear = the month's upkeep per kg / (`e_life` × 10^REPAIR) [13]. Check: 100 kg, field rate twice
  rest, 1.9e5 J per kg a day: 1e9 / 1.9e5 = 5,300 days, 14 years. 30 g at four times rest: 1.4 years. On Earth deer
  live 5 to 11 years in the wild, mice about a year. The order fits.
- `PHASE`: a cold-bodied slot freezes by `Physics::frozen` with the liquid's melting point as its limit and `harden` as
  its depth, its activity min(1, pace) (the cover's law and its two numbers, shared).
- `LACK`: the share step 6 could not let shed its heat.
- Members that die take their share of the class's matter and fuel to the ground as remains, with their heat at T_b
  (`contract::give`). Rot (reaction 7, open) gives the mineral back from there.

**Step 9. The mineral's cycle.** Where the mineral (12) goes, step by step:
1. In the cover's matter (10). A taker eats matter: the mineral moves inside the matter.
2. Matter built into young: the mineral stays in the slot.
3. Matter burnt for energy (reaction 7): its 3 mol of the mineral go to the ground **at once**.
4. Members that die: their matter is remains in the ground; open rot (reaction 7, `open` 5, about a year) frees the
   mineral.
5. Walls and fuel hold none.
So takers return the mineral faster than the cover's own death and rot do. Grazers speed the cycle, as real ones do
(McNaughton 1997, from memory). No new rule: it is the chemistry.

**Step 10. Gases, heat and the books.** The slot's reactions run through the cover's `Books` (`ext`, `heat`, `dead`,
`room`). They are booked through the cell's view by the cover's `book` (gases from the air share with their heat and
latent heat, to the aside node; liquids and minerals from and to the ground; remains to the ground; the heat to the
body's temperature). **That code must become one shared helper first** (`life::books`; build plan commit 1). The fauna
must not copy it (`CLAUDE.md` 2.1).

**Step 11. Mutation, the slot rule, the dispersal** (section 5, section 4).

## 3. Time inside the month

**The worked example.** One slot of grazers of 100 kg, 5,000 kg per km² (5e-5 members per m²), in a cell in late
winter. Two layers: low herbs with 0.5 g/m² reachable (w = 1), and shrubs with 20 g/m², of which the band reaches
0.5% (w = 0.005).
- Search: L = 0.46 m, v = 1.07 m/s, a = 0.50 m²/s; T = 43,200 s.
- Found a day: low 0.5 × 5e-4 × 43,200 = 10.8 kg; shrubs 0.5 × 1e-4 × 43,200 = 2.2 kg. The cap (chewing) is about
  2.2 kg a day.
- **One explicit monthly step** reads the rates at the month's start. It asks for 30 days × 2.2 × 5e-5 = 3.3e-3 kg/m²,
  83% from the low layer: 2.7e-3 kg/m². The low layer holds 5e-4: the step asks 5.5 times what is there. The
  contract cuts it to what is there, so the books stay right. But the slot then gets only 5e-4 + 5.6e-4 = 1.06e-3
  kg/m²: **32% of its need**. About two thirds of the members die of `SPENT`.
- **What happens in nature:** the low layer is gone in about 5 days. Then the members find only shrubs, 2.2 kg a day
  found, of which they eat 2.2 / (1 + 2.2 / 2.2) = 1.1 kg: half the cap. Over the month: 5 days full and 25 days at
  half: **58% of the need**. Half the deaths.
- The explicit step is wrong by a factor near 2 in deaths. It does not fail by overdraw (the contract stops that). It
  fails because it does not let the takers **switch** to the next layer when the first runs out.

**The scheme.** Six steps of 5 days. In each, the rates' denominators are found from the stocks at the step's start,
and each layer falls by S (1 − e^(−k Δ)), shared by k's parts (section 2, step 3).
- Exact in the books: the shares add to what was taken, and one take per layer and substance asks the cover.
- Stable at any step length: the stock never goes below zero and never swings. A longer step only switches later.
- With 5-day steps the example gives 59% instead of 58%. With one step of 30 days it would give 32%.
- For one slot and one layer the exact answer is known: ln(D/D0) + a h (D − D0) = −N a t (Lambert's W). It can be the
  test of the scheme (test 6).

**Why 6 steps and not 30.** The scheme's error is in the frozen denominator, which changes when a layer runs out. A
layer that runs out in under 5 days is still taken in full, and stable. So 5 days is enough to switch. Measure 3, 6 and
30 steps on the gate run: the numbers of the gate must not move beyond their noise between 6 and 30.

**What it costs.** Per step: slots × classes × layers multiply-adds, about 8 × 2 × 6 = 96, and one e^x per layer.
Six steps: under 600 operations and 36 e^x: 1 to 3 µs per cell.

**What the time scheme does not fix.** In the example, the low layer is still eaten bare in 5 days. A finer scheme does
not stop that. With 200 animals of 100 kg in a cell of 4 km², each searching 2e4 m² a day, the herd searches the whole
cell every day. So **at the scale of a cell, a takers' field always eats what it can reach to zero within the month.**
The real brake is what it cannot reach:
- the floor of the bite (step 2): with `bite_floor` 0.1 of L, a 100 kg grazer leaves 4.6 cm of every plant, about 15%
  of a 30 cm herb; a 30 g member leaves 4 mm;
- the band of heights: a tree slot is mostly out of reach;
- the parts under the ground (gap G3).
That refuge is what the cover grows back from. On Earth, grazed grass keeps a stubble too (grazing heights of 3 to
10 cm, from memory). **This is the first number to watch when built.**

Plants answer monthly and takers eat in the month after the plants' growth. The pair is then a map from month to month,
in which the reachable part is eaten and grows back from the refuge. That map can be stable (a grazing lawn) or swing.
The gate measures it.

## 4. Small numbers and the end of a kind

- **Keyed draws:** `Rng::new(w.seed, w.month(), (i × FS + x) as u64, Purpose::Fauna)`, one stream per slot and month,
  drawn in a fixed order: deaths of adults, deaths of young, maturing, births, dispersal lots, mutation. A new
  `Purpose::Fauna`. The draws are made by the cell's own task, so the order does not depend on threads.
- **When a count is whole:** below `N_WHOLE` = 64 members in a class (a `const` next to its use; the relative noise of
  a count of 64 is 1/8, which a mean would hide). Above it, counts are real and change by their means.
- **Crossing below:** when a class first falls under `N_WHOLE`, its count becomes floor(n) + (1 if a draw < the
  fraction). In the mean it is unchanged, never rounded up as a rule (`CLAUDE.md` 3.2). The matter stays: the mean mass
  per member moves by under 1/64.
- **Whole changes:** deaths ~ Binomial(n, p), shared over the causes by their shares; maturing ~ Binomial(n_y, share);
  births = floor(matter / child) + Bernoulli(the rest). The matter of a birth not drawn stays in the store. So a draw
  never makes matter. `Rng` has no binomial today: add one helper (n ≤ 64 uniform draws, or inversion).
- **A slot is gone** when adults + young = 0 (with whole counts that is "under one"). Its matter and fuel go to the
  ground as remains, its heat with them, as the cover empties a slot.
- **One parent.** A single adult can still give birth (the spec's rule for nodes). This lets a kind of one member live
  on. On Earth, a lone animal cannot breed (the Allee effect). I left it open (section 11).
- **How a kind comes back in F1:**
  - without movement: never, except by a mutant of another slot of the same cell, which (section 5) almost never
    founds a slot. So without dispersal, an extinction in a cell is forever.
  - **with the dispersal of young by the seeds' kernel** (`life::lot`): a few young land next door, and a kind can come
    back. I argue F1 needs it:
    - A cell of 4 km² holds 8,000 kg of grazers at 2,000 kg/km². That is 80 members of 100 kg, or 8 of 1,000 kg.
    - A count of 8 that draws its deaths and births dies out by chance alone in a few decades (from the binomial
      variance. Not measured: test it as the first thing of commit 4).
    - Over 500 years, with no way back, each large kind goes out cell by cell. The gate would see a slow decline that
      is an artifact of the cell's size, not of the ecology.
    - The kernel is already a law (Copy: children land by it, laws.md 3). It needs no sensing and no choice. So it is
      not F2's movement.
    - Cost: young that leave cross cells after all chunks ran, landed cell by cell in the order made (as
      `cover::land`), with their potential energy (`store::life_sent`). About 2 µs per cell.
    - With the kernel's scales (40 m near, 800 m far, a tenth far), about 5% of the young leave a cell of 2 km (my
      arithmetic). That is little, and the same for all sizes. Real dispersal grows with body size (Sutherland and
      others 2000, from memory). That is F2's work.
  - **Proposal:** build F1 first without dispersal (commit 4), measure the times to extinction per kind, then add it
    (commit 5), and keep the run without it as the measured contrast, not as a switch on main.

## 5. Kinds and slots

**Mutation.** The young born in a month form one packet per slot. Its genes are the parent's plus one generation's
noise (`cover::mutate` with each gene's step and `mut_size`). The packet joins its slot (the genes the mean by living
mass) or founds a slot, by the shared rule of `cover::kin` (`kin`, `empty`, `together`). The rule moves to `life::kin`
and serves cover, seeds and fauna.

**When it founds a slot, the arithmetic.** A packet founds a slot only when one gene is more than `APART` × `alike` =
2 steps from its slot. One generation moves a gene by about `mut_size` = 0.063 steps. So a packet **never** founds a
slot by its own mutation. And each month it joins back by the mean, so a slot's genes drift by under 0.063 × (packet's
share of the slot) steps a month. **The slot rule gives selection between slots that exist. It does not give a kind
that splits in two inside a cell.** The cover has the same property. Its variety comes from the start's spray (20
rounds of mutation per cell) and from seeds of other cells. For takers the same will hold: variety from the start's
spray and from dispersal.

**When two slots may join.** Only when alike in every gene (`together`), as now. **Not** the cover's `room` (the
nearest pair joins when all slots are held). For takers, a mean of a cold-bodied kind with low reach and a warm one
with high reach is good at neither. Rule for fauna: when all slots are held, an arriving packet takes the slot in
deficit (`net` < 0, the worst first; its members die as remains). Else it joins its parent slot, with no new genes. It
never forces a join of unlike slots.

**How a new kind can split off** (open for F2): a slot keeps a spread of each gene as well as its mean (section 13's
pooled spread). When the spread of a gene is wide and the two ends do better than the middle, the slot splits in two.
This is a law (disruptive selection measured on the slot's own numbers), not a target. It is not for F1.

**The test that shows it** (part of commit 4): two kinds in a cell, far apart in `REACH` and `KEEP`, 4 slots, 100 years
with dispersal off. Pass: they never share a slot, both persist where both layers exist, and no slot holds genes
between them (each gene's slot mean stays within 2 steps of one of the two).

**How many slots.** 8 to start (roadmap 4.1). Measure 6, 8 and 12 on the gate run: (1) the share of cell-months in
which a packet or a disperser found no room, (2) the number of starting kinds alive per cell at year 200, (3) the
gate's numbers. The right count is the smallest whose (1) is under 1% and whose (3) lies within the noise of 12's.

## 6. The start

When: **year 50 after the first life** (`--fauna-year`, default life year + 50). Reason: herbs fill the land in a few
years; trees need decades; the cover is steady from about year 100 (roadmap 1.4). By year 50, the low layers are
steady and the high ones grow. Check before choosing: the cover's land mass changes by under 2% a year at that year.

Where and how many: on every open land cell (worldgen's `open_land`), each kind by its chance, at a third of the
Damuth line: n = ⅓ × 10^4.23 × (M in g)^−0.75 per km² × area. Whole counts below `N_WHOLE`, by a keyed draw
(`Purpose::Place`). All adults; matter at their mass; store at a tenth of lean mass; heat at T_b. Booked from the
outside node by `World::cross`, as `cover::place` does. A spray of 20 rounds of mutation per cell, as the cover's.

Damuth's line (checked: Damuth 1981, Nature 290: 699, log D = −0.75 log W + 4.23, D per km², W in g, 307 species, r =
−0.86).

**The kinds** (start conditions, not rules; names in physical words):

| Kind | MASS (kg) | CHILD | YOUNG | ENZYME | REACH (m) | KEEP | COAT | REPAIR | from real sizes |
|---|---|---|---|---|---|---|---|---|---|
| small low takers | 0.03 | −1.2 (2 g) | 0.8 | −8 | 0.03 | 310 | 0.2 | −0.5 | voles |
| small takers | 3 | −1.5 | 0.6 | −7 | 0.15 | 311 | 0.4 | 0 | hares |
| high takers | 60 | −1.2 | 0.4 | −6.8 | 1.0 | 311 | 0.4 | 0 | deer that browse |
| large low takers | 400 | −1.3 | 0.3 | −6.5 | 0.3 | 311 | 0.8 | 0 | bison |
| cold low takers | 2 | −1.7 | 0.3 | −7 | 0.1 | 250 (never warm) | 0 | 0.5 | tortoises |

Density at the start, a third of Damuth (10^4.23 = 16,980): 440, 14, 1.5, 0.36 and 19 members per km². In mass: 13, 42,
89, 143 and 38 kg/km². That is 325 kg/km², near the low end of the real range, so the takers grow into their range,
not fall into it. On a 4 km² cell the large low takers start at 1.4 members: whole counts from the first month.

Shown at the start: a line per kind, as `seed_with` prints for the cover.

## 7. What the land must offer that it does not yet

| # | Gap | Smallest change | Can F1 start without it? |
|---|---|---|---|
| G1 | **A taker can ask for one substance alone.** `contract::take` takes mol of one substance from a layer. A taker can ask a tree slot for its matter and fuel, and leave the walls. A real bite takes a part of the plant whole | `take` of a layer returns a **bite**: a share of all its parts in their proportions (one call, shares out). Then a tree slot is mostly walls, slow to chew (step 3), mostly not digested. Browse becomes poor food by physics | No: without it the take law can cheat. It is small (`take_from` already takes shares) |
| G2 | **No breaking energy per layer.** `Cover::breaking` gives the whole cover's | `breaking(amounts)` as one function of the amounts; `Shown` gets `breaking` J; `press` uses the same | No: the handling time needs it. Small |
| G3 | **Roots are reachable.** The cover has no part under the ground. A share of a slot is above the ground and below it alike. Its fuel stands for leaf sugar and root store together | The reach (step 2) counts only the part above the ground: share ≈ 1 / (1 + root share), with the root share from the stand's law (`root_per_height` gives depth, not mass: a new root-to-shoot number, anchored on Earth's 0.2 to 1 for herbs, from memory) | Yes. The bite floor gives a refuge already. Measure the share eaten against a run with the root share |
| G4 | **No drink in the contract.** The cover reads the ground's liquid from its view (`cover::wet`); a taker should have one call | `contract::liquid(i)` → mol of the key liquid that is liquid and that is frozen in the ground and standing; and `contract::vapour(i, mol, heat)` that moves it to the aside node with its latent heat, as the cover's `book` does | Yes if the fauna lives in `core` and uses the view directly, as the cover does. But the contract is meant to be the only door. Do it in commit 1 |
| G5 | **No breathing in the contract.** The same for gases | Move the cover's `Books`/`book` into `life::books`, used by both | No: the books need it. It is the first commit |
| G6 | **What a cell's life weighs counts the cover only.** `flow::stack::Life::kg` (potential energy), `check::totals`, `check::broken`, `contract::Stand` read the cover alone | Each reads "the life of the cell": cover, its seeds, the fauna, its young in flight. `Class::Cover` is renamed `Class::Life` (the books' node of a cell's life) | No: without it the books drift. Small, and in commit 2 |
| G7 | **Remains on the ground are not a layer.** Dead matter in the ground node is the open world's rot's | `Layer::Remains`: the ground's 10, 13 and 5, shown and taken through the view | Yes: no start kind eats remains. Later, for eaters of remains |
| G8 | **Every year is the same year** (roadmap 6 item 1) | Not F1's | Yes, but the takers will look too steady. Say so to the owner |

What is good already:
- The cover's stored fuel is reachable (in the slot's proportions; see G3).
- The cover shows each slot by height, so a low taker and a high taker see different layers.
- Snow hides by `contract::above`, and the same helper gives the bite floor.
- The seeds lying are a layer (`Layer::Seeds`) with height 0.
- Without the seeds and the snow (if they were not there), F1 would still run: takers would find no seeds, and winter
  would be hungry only by the cover's shedding in the dark months and by frost. Both are on main now.

## 8. Recording and seeing

**Yearly columns** (`tape/src/rec/cols/`), few:
- World: `fauna_kg`, `fauna_members`, `fauna_eaten_kg` (of the cover), `fauna_seeds_eaten_kg`, `fauna_eaten_share`
  (eaten / the cover's growth of the year, `Counters::grown_last`), `fauna_slots`.
- Per starting kind `fauna_<k>_`: `adults`, `young`, `kg`, `intake_kg`, `births`, `died_spent`, `died_age`,
  `died_phase`, `died_lack`, `slots`, and the 8 genes' means by living mass.
- A budget like the cover's (`FAUNA_BUDGET`, J, with + into the slots): `eaten`, `split` (walls to fuel), `upkeep`,
  `heat` (kept warm), `walk`, `enzyme`, `building`, `died`, `droppings`, `placed`, `dispersed_out`, `dispersed_in`,
  `rounded`. A year's change of the fauna's content is the sum of its flows, checked by a test.
- About 10 + 23 per kind: 125 columns for 5 kinds.

**Map layers** (`tape/src/layers.rs`): `takers` (kg/m², log 1e-5 to 0.1) and `eaten` (share of last year's growth
taken, 0 to 1, lin). The page needs no shader work: they are layers like `grown` and `seeds`.

**What the owner gets at the end of F1, said plainly:**
- Two maps that change from year to year: where takers are, and where they eat much of what grows.
- The cover's own maps next to them: a grazed plain against an ungrazed one (the `plants` and `grown` maps, against a
  plants-only twin run).
- Charts: each kind's count and mass over 500 years; the share eaten; the Damuth plot of the kinds.
- **Not visible:** single animals, herds, where in a cell they stand. That is F4. A map of kg per m² is all a field
  shows until then.

## 9. The gate of F1

Measured over years 100 to 500 after the takers' start, on the roster world, size 32, seeds 1 to 3, against a
plants-only twin of the same seed.

| Check | Real range | Source | F1 passes when |
|---|---|---|---|
| Share of the plants' growth eaten | terrestrial median 18% of NPP (67 studies); a few % in forests, 10 to 60% in grasslands | Cyr and Pace 1993, Nature 361: 148 (checked in part: the 18% from a secondary source; the paper's own table not read); McNaughton and others 1989, Nature 341: 142 (the paper found; its numbers from memory) | land mean 2 to 40%; cells whose cover is mostly herbs eat a larger share than cells mostly of trees |
| Takers' mass per area | 1e2 to 1e4 kg/km² over African parks; typical 1,700 | Fritz and others 2002, Oecologia 131: 620, citing Fritz and Duncan 1994 (checked); Hempson and others 2015 (in `docs/numbers.md`, not read) | land mean of all kinds 300 to 10,000 kg/km²; and it rises with the cover's growth across cells (a positive slope of log mass on log growth, as Coe and others 1976 found with rain; their coefficients not found) |
| Numbers against body mass | slope −0.75 across species; within communities −0.56 to −0.95; species within about a factor 10 of the line | Damuth 1981 (checked) | the slope of log members per km² on log adult mass over the kinds alive (land means) lies in −0.5 to −1.0 |
| Plants and takers hold each other | | | (a) every kind above 10 kg that lives at year 100 lives at year 500 somewhere; (b) for each such kind the land count's yearly coefficient of variation is under 0.5 and its lowest over any 20 years is above a fifth of its highest; (c) kinds under 1 kg may swing (real voles swing 10 to 100 times, from memory), but none dies out from the whole land; (d) under 5% of land cells have their reachable cover under a tenth of the twin's for 3 years in a row; (e) the cover's land mass is at least half the twin's |
| The books | drift under 1e-9 | `check::drift` | yearly, all 500 years |
| Cost | under 40 µs per land cell and month | roadmap 4.7 | timed at sizes 32 and 64 |

Rule check (c) is my judgment of what "hold each other" means for small kinds. Ask the owner.

A first check on paper, of the order of size (my arithmetic): herbs grow 0.5 kg dry per m² a year; takers eat 20%:
0.1 kg; 60% digested at 1.65e7 J per kg: 1e6 J per m² a year. A 100 kg taker at twice its rest spends 6.8e9 J a year.
So a m² feeds 1.5e-4 of them: 150 per km², 15,000 kg/km². At 5% eaten: 3,700 kg/km². Both inside the real range. So the
gate is reachable by the laws, if the share eaten lands right.

**Fast tests** (each states a law):
1. **Matter and energy are kept with takers.** A small world with cover and fauna, 12 months: `check::drift` under
   1e-9; the fauna budget's flows sum to its change of content.
2. **A taker takes no more than it reaches.** One cell, a slot asking 10 times the reachable layer: the total taken is
   at most the shown stock within the band, above the floor and the snow, and the cover keeps at least the refuge.
3. **A slot with no reachable cover starves by the law.** One cell, cover out of reach: the store lasts store /
   upkeep days as Kleiber says (to 1%), then members fall by the unpaid share; their remains reach the ground with
   their mineral.
4. **Counts are never rounded up.** 50 months on slots of few members: every count under `N_WHOLE` is whole; a slot of
   none is empty and its matter is on the ground; the count never exceeds the start plus the births drawn.
5. **The same world for any count of threads.** 1 and 4 threads, 3 and 64 chunks, with dispersal: the same
   fingerprint.
6. **The take in steps meets the exact answer.** One slot, one layer: the six-step scheme against the closed form
   (Lambert's W) within 1%; and two layers: the slot switches (its intake from the second layer rises when the first
   runs out).

**Slow test** (`#[ignore]`): the gate table above, one seed, 500 years after the start.

## 10. Cost

From the cover: 45 ms a year on 409 land cells for the cover and the survey: about 9 µs per land cell and month for
4 slots (roadmap 4.6; not timed again by me).

Counted for one cell of 8 slots × 2 classes, about 6 layers:
- Showing: 6 layers × (`shown` + height + breaking): about 2 µs.
- The take: 6 steps × 96 multiply-adds + 36 e^x: 1 to 3 µs.
- The takes from the cover: 6 layers, one bite each (with G1): 1 µs. (Without G1, 18 calls: 3 µs.)
- Per slot and class (pace, Kleiber, coat, enzyme, growth, deaths: about 10 powers and e^x each): 16 × 0.3 µs = 5 µs.
- Draws: under `N_WHOLE`, binomials of up to 64 draws: up to 4 µs in a cell of small slots; nothing in a cell of large
  counts.
- The books (gases, ground, heat): 2 to 4 µs.
- Dispersal landing: about 2 µs.

**Range: 10 to 40 µs per land cell and month.** At size 32: 50 to 200 ms a year, against 235 ms for plants only.
Where it grows: with layers (the seeds' 4 lots, remains later), with slots (12 is 1.5 times 8), with the steps (30 is
5 times the take), and with the draws where many slots are small.

## 11. Risks and open questions, ranked

**What fails first.**
1. **Grazers eat the reachable layer bare and the pair swings.** Section 3 shows a cell's takers reach all they can
   within the month. All rests on the refuge: the bite floor, the band and the walls' toughness. If `bite_floor` is
   too low, cells crash; if too high, the takers starve. It is a new number with a weak anchor (grazing heights, from
   memory). Watch it first.
2. **Large kinds die out by chance** in cells of 8 to 80 members, and do not come back without dispersal. This is why
   I put the kernel in F1.
3. **The growth cap's E_c** (step 7): a factor near 3 between the two readings. It sets how fast kinds recover. Check
   against West 2001 when built.
4. **The chemistry makes plant matter and animal matter the same substance**, C:N 20 (checked: chemistry 0, substance
   10). Real animals are C:N about 6, so real grazers are short of the mineral in their food. Here they are not: the
   mineral never limits a taker. It is a fact of chemistry 0, not of F1. Say it to the owner. A fix is chemistry work.
5. **Every year is the same year.** The takers will look too steady (roadmap 5.7).

**What I could not decide.**
- Whether one parent may breed (the Allee effect). With one parent, a kind of one member lives on. A rule of two
  adults is a rule about an outcome unless written as a meeting law (the chance two adults meet in a cell by their
  density and search, which needs the sexes as half the members). I lean to one parent for F1 and a meeting law in F2.
  The owner's call.
- The minimum need of liquid (breath and urine) beyond cooling. I left it out: it nets to zero in the books, and it
  would need a share of oxygen taken from a breath (a new anchored number) to be a law. A dry, cool cell then never
  makes a taker thirsty. Real animals in a desert are thirsty. Defer to the saltiness work (roadmap 6 item 3).
- The root share of G3: a new number, or a law from the stand.
- What (c) of the gate should demand of small kinds.

**Where I think roadmap section 4 is wrong.**
- 4.1 "about 12 genes": F1 can test 8. The other four (group size, prey, place, range) belong to F2 and F3.
- 4.1 step 7: dispersal in F2. I argue F1 needs it (section 4).
- 4.1 step 7 and 5.11: "mutants join the most alike slot or found one". With one generation's step at 0.063 and the
  join radius at 2 steps, a mutant never founds one. The rule is selection between slots, not branching. Say so before
  the slot rule is shared.
- 4.1 step 1 lists "the other slots by their member mass" as food: F3, not F1.
- 4.7 row 2: "1 to 20 g/m² in savanna". The checked range over parks is 0.1 to 10 g/m² (Fritz and others 2002).
- Section 13: `c_build` as written gives growth 100 times too slow (step 7). `sweep` and `c_handle` can leave: the walk
  and the breaking stress give them.
- Not in section 4 but found: `docs/numbers.md`, "Real figures", writes Damuth as "density = 4.23 m^-0.75 per km²". The
  paper's line is log10 D = 4.23 − 0.75 log10 W, W in grams (checked). As written the formula gives 0.2 members of
  50 kg per km², not 5.

**New numbers** (each needs a row in `docs/numbers.md`; none is tuned, so the count of tuned numbers does not grow, and
two tuned ones of section 13 do not come back):

| Number | Value | Unit | Anchor | Mark |
|---|---|---|---|---|
| `c_muscle` | 1.12e4 | J per kg^0.75 a day at `t_cat` | Elia 1992, muscle a fifth of the resting rate (section 13) | anchored |
| `chew` | 2.9e3 | J per kg^0.75 a day | intake on grass 42 to 80 g dry per kg^0.75 a day × a herb's 4.2e4 J/kg (from memory) | anchored |
| `busy` | 0.5 | share of the day | grazers feed 8 to 13 h a day (from memory) | anchored |
| `bite_floor` | 0.1 | share of body length | grazing heights of 3 to 10 cm for 100 to 500 kg grazers (from memory) | anchored, weak |
| `reach_len` | 5 | body lengths | giraffe 5.5 m at L = 1 m; goat 2 m at L = 0.37 m (from memory) | anchored |
| `STORE_MOST` | 0.3 | share of lean mass | store of mammals before winter, 20 to 40% (from memory) | anchored |
| `meeh`, `r_air`, `r_coat`, `WALK_FROUDE`, `c_move`, `MOVE_REF`, `MOVE_EXP` | section 13 | | section 13 | anchored |
| `N_WHOLE`, `SUBSTEPS`, `BAND`, `GROUND_M` | 64, 6, 0.5 decade, 0.01 m | | code constants with a one-line reason | rules of the model |

## 12. Build plan

Each commit ends in something measurable.

1. **The shared ground (a pure clean-up).** `life::books` from the cover's `Books`/`book`; `life::kin` from
   `cover::kin`; `Class::Cover` → `Class::Life`; `breaking(amounts)` (G2); a bite in `contract::take` (G1, a law
   change for the test taker only); `contract::liquid` and `vapour` (G4). Measure: a plants-only run is the same to the
   last digit (fingerprint and `series.csv`), apart from G1, which no plants-only run uses.
2. **The fauna's state, placing and books, no eating.** `Fauna`, `life::Cell::fauna`, the snapshot version, G6, the
   start's placing, upkeep, heat, deaths, remains. Tests 1, 3 and 4 (and 5 without dispersal). Measure: the store lasts
   as Kleiber says; drift under 1e-9 over 20 years with takers that only starve.
3. **The take and digestion.** Steps 1 to 4 and the six-step scheme. Tests 2 and 6. Measure on a 50-year run: the share
   eaten, takers' kg per km², and the reachable cover against the twin.
4. **Growth, births, maturing, whole counts, mutation, the slot rule.** Step 7, section 4, section 5 and its slot test.
   Measure on a 200-year run: the Damuth slope, the times to extinction per kind per cell without dispersal, and the
   slot counts at 6, 8 and 12.
5. **Dispersal of young by the kernel, the tape and the maps.** Section 4's landing, section 8. Measure: the same
   200-year run with dispersal against the run of commit 4 (extinctions per cell, kinds alive); before and after
   pictures of the `takers` and `eaten` maps.
6. **The gate and the docs.** The slow test of section 9 over 500 years; `docs/laws.md` (a new section for the fauna),
   `docs/numbers.md` rows, `docs/tape.md` columns; the cost at sizes 32 and 64.

## Sources looked up for this design

- Damuth 1981, Nature 290: 699, read in full: https://pdodds.w3.uvm.edu/files/papers/others/1981/damuth1981a.pdf
- Fritz, Duncan, Gordon and Illius 2002, Oecologia 131: 620, read pages 1 to 3:
  https://archive.bio.ed.ac.uk/illius/herve%20megas.pdf
- Cyr and Pace 1993, Nature 361: 148 (only its abstract and secondary citations found):
  https://www.doi.org/10.1038/361148A0
- McNaughton and others 1989, Nature 341: 142 (found; numbers not read):
  https://www.scilit.net/publications/b4d8d1304a13a021b9b0e6dedc03e2f6
- Coe, Cumming and Phillipson 1976, Oecologia 22: 341 (abstract only): https://link.springer.com/article/10.1007/BF00317566
