# The roadmap

The plan, the priorities and the owner's decisions, for the agent who leads the work. Simple English. How each law
works is in `docs/laws.md`, each number in `docs/numbers.md`, what a run records in `docs/tape.md`, the spec in
`docs/handover.md`, how to build, test, run helpers and publish the page in `docs/workflow.md`. Where this file and
the code differ, the code and `docs/laws.md` are the truth; fix this file.

Words: **the books** are the accounts of matter and energy (nothing is made or lost). **A field** is numbers per cell
in place of a list of single things. **A slot** is one kind of cover or of takers in one cell. **Takers** are the
animals: kinds that take from the cover.

## 1. The owner's decisions and words

These stand until the owner says otherwise. Their own words are in quotes; the other lines are rules they agreed to.

**How the world is made**
- "Things emerge from general physics and chemistry laws." No rule made for one thing: none that says where a habitat
  goes, where a herd moves, or what a kind should do. If you can list all uses of a rule in advance, it is a named
  system: do not write it.
- "Do not compute what is not needed."
- Every trait has a cost, or nothing sorts.
- Detail is derived, not simulated: a cell holds means; a law inside the cell shares them over its own ground for
  whoever needs detail (the page today, a single animal later).
- Judge a change of the laws by pictures and by numbers in a real range. Only a pure clean-up is checked to the last
  digit.
- One hand-made chemistry (chemistry 0) for every world. The chemistry generator stays off `main` until worlds need
  their own matter.
- No speed target now: "build the most true, most interesting and best-made system; cut back when it has become too
  slow." Never remove a law to get speed.

**The world's size and shape**
- No globe as a world to play. The globe stays only for the climate's slow test and the CLI.
- A world is a square map of 128 x 128 squares (256 km a side); 64 to judge by; 256 or 512 at the very most.
- Worlds should differ as much as they can: one island, several, a ring, a long coast.
- The page makes islands in the planet's air only ("remove None from the page"). The closed island stays for the
  tests and the CLI.
- The strong weather: chosen by the owner knowing its cost.
- The amount of variety is a setting of a world, like its seed, size and climate (`Config::variety`, default 8). How
  many kinds live together in a cell stays with the laws.
- Habitats: "honest or nearly honest and easy ways" to get variety on a land of manageable size.
- Wide deltas at some mouths are wanted; the owner agreed to wait, as it is a big change.
- Dead plants of a thinning stand should fall to the ground: "later".

**Life**
- The animals are rebuilt as a field, as the cover is: slots per cell, real counts, one month's pass, exact books.
- "The animals do not count for now": a takers' number that moves, or takers that fare badly, holds nothing back.
- The takers are paused: "we'll continue that when we have sorted the habitat variety for them to live in."
- Thirst is a law of F1: a body drinks open liquid and pays the walk to it.
- The animals' drawing is paused until the takers' numbers are final. The three looks tried "all look terrible,
  grid-like, far too many animals, and too big". A next try starts from the final numbers, never places members on a
  grid, never draws them larger than they are.
- "What the land offers" goes beside the first grazers, the pieces they read first.
- The small game (god mode) comes before people. People come after the animal layer is done; no work on them before.

**The order of work**, in the owner's words: (1) the variety of habitat; (2) the sorted start, for plants and then
animals, so life fits that variety from the start; (3) the animals' numbers, run again after those. After the variety
of habitat: "the look of the habitats, and everything else paused until it is done." Now: the look of the natural
landscape and god mode, side by side; the rest waits.

**The page and the look**
- The owner judges by pictures: before and after (same seed, date and place) with every change they will see.
- The Earth view is the pretty one: vivid, a little cartoony, tricks allowed if they follow the world's numbers. The
  map's layers show the true numbers and stay physically true.
- The page draws the angled view only ("let's focus on the angled view for now").
- The speed on phones "is perfectly fine": no more cuts of the frame's cost for now.
- The look's requirements, in the owner's words:
  - "Make the new variety of habitat look much much much nicer."
  - "As full of details and information as possible and as realistic as possible (with dramatised features for the
    game)! Adding textures, details, relief, objects."
  - "It's okay to use procedural generation with some artistic liberties ... as long as the numbers are still
    guiding the general outline and behaviours of the world. Having a big boulder in the visualisation that isn't
    tracked in the numbers isn't going to be the end of the world."
  - "We need to ensure continuity in color, texture and styling at all zoom levels."
  - "Detailed textures that follow the seasons: leaf on the floor in the forest in fall, textured snow in winter."
  - "For dense environments, like forests, do not treat flowers and bushes as instances of the same item, but more as
    detailed blankets of greenery wrapped around the floor."
  - "In sparse environment like snowed peaks, barren land, deserts ... there are still many many many detailed
    elements that can be freely added."
  - "A 'stylised' realism for everything": huge detail in trees and greenery, with stylised colour.
  - "Small variations in even those 8 varieties, while still keeping the numbers correct."
  - Nothing may stand on a grid, ever.
- What that means for the drawing:
  - The numbers decide the outline (where forest, grass, bare ground, liquid, snow and rock are; how tall, dense and dry;
    the season). Detail may be invented; the outline may not.
  - One look at every zoom: a far tone is the mean of what the near view draws there, near detail fading in over it.
  - Free detail is keyed to the world's place (it does not move between frames or zooms) and follows the ground (no
    boulder on open water, no log on a snowfield with no trees near).
  - Variation inside a kind spreads around the kind's own numbers and never changes what a square holds in sum.
  - A look does not change the laws. A number the page lacks is added to what `tape` gives, cheaply, with
    `docs/tape.md` in the same commit.
- The owner's reference pictures are on `wip/look-refs` (section 7): `faults/` and `want/` (a dry grassland with
  flat-topped trees, a forest glade with a stony stream, a swamp, high mountains with ice and a milky lake, a braided
  river with gravel bars). The tents, cars and animals in them are style only.

**Tests and working**
- The owner reads very simple English only: short sentences, common words, bad news first.
- Tests: few, fast, meaningful. The gate and the key numbers run on worlds of plants (no takers placed).
- "It feels like we're running and re-running slow tests every few minutes": one full gate for a merge; the slow tests
  once, and only when a law changed. Never loosen a tolerance.
- The key numbers' references (`tape/tests/ranges.rs`) move only with a change of the laws that moves the world on
  purpose, in its own commit. Tell the owner each value.
- `the_laws_do_not_depend_on_the_size_of_the_cells`: "land the air package, forget the red test, move on." If it is
  red, it does not hold a merge. Do not loosen, skip or hunt it without the owner's word.
- Push to `main` often. No work lives only on one machine.

## 2. Where things stand

What `main` holds. The laws are in `docs/laws.md`.

- **Ground** (`core/src/space/`): fixed cells of 2 km for a world's whole life; samples of 125 m made once by the
  erosion law (valleys, one network of channels, ponds with one level, drowned coasts); drawn worlds of one to several
  rises and rings (`space/rises.rs`; one island with `--shape one`); a bed of several makes in folded layers, with seams
  of clay, coal, rock salt and an iron ore, a copper ore round hard bodies (`space/strata.rs`; nothing uses them yet);
  delta lobes at the mouths built at the start (`space/delta.rs`).
- **Water and air** (`core/src/flow/`): an island in a band of the planet's air (the page: Wide, Mild, Warm); the open
  low ground held at the planet's level at the edge (`flow/runoff/edge.rs`); sunlight absorbed by gases and reflected by
  clouds; eddies and an overturning that follows the month's sun; a wet windward east and a dry lee (`flow/air/rise.rs`);
  rain that drifts with the wind; years that differ (`flow/air/weather.rs`).
- **Soil** (`core/src/soil/`): texture from the bed, weathering and the flood; pores by Saxton and Rawls 2006; a slow
  organic pool; exchange sites. No soil type is placed.
- **Cover** (`core/src/cover/`): slots per cell (variety 8, up to 16) with 12 genes, each with a cost, two of them
  defences against eaters; patches inside a cell (`cover/patches.rs`); self-thinning by size groups; seeds lying on the
  ground (`cover/lying.rs`); roots in full pores need gas spaces (`cover/soak.rs`); leaves shed in a dry season; cured
  leaves of low plants stand through it.
- **Fire** (`core/src/flame/`): remains and cured leaves burn by the chemistry's reactions at a speed from the heat
  balance (Rothermel 1972); keyed strikes; a burnt patch is a firebreak until it grows back; low plants sprout again.
- **Takers** (`core/src/fauna/`): F1 built in part (section 5). The page places them 50 years after the first life
  (`TAKERS_YEAR` in `wasm/src/lib.rs`); the CLI places none unless `--fauna-year N`. No single animal is drawn.
- **The contract** (`core/src/contract/`): what a cell shows; take, give, press; what stands on a cell. The natural world
  never names a taker's module (`cli/tests/sides.rs`).
- **God mode** (`core/src/edit/`, `web/src/god*.ts`): edits are physics edits at a step's boundary. Built: raise and
  lower land, a lake brush, plants of the world's kinds, fire by heat added, clearing and splitting patches, the undo of
  the last stroke. The other tools of the panel show as "soon".
- **The page** (`web/`): WebGL2, shaders in WGSL turned into GLSL by naga; the Earth view and map layers; ground
  photographs, rendered tiles of ground cover and model sprites laid by the world's numbers (`web/src/detail.ts`,
  `web/shaders/lib/detail.wgsl`); a saved world opens paused at its date by URL.

## 3. The priorities, in order

Each item is a package: measure first, one change a commit, pictures before and after. A change of the laws deletes
what it replaces in the same merge. Items 1 and 2 are in work side by side, by the owner's word.

1. **God mode: finish the small game.** In work. Each "soon" tool becomes a physics edit (`docs/handover.md` 11): flood
   (add liquid), drought (less rain over an area for some years), climate (the sun or the heat-absorbing gas),
   mountains and rivers (the ground brush, the liquid finds its way). Grey out what the world cannot do and say why.
   Then a save before each action and "then and now". The test: an action has a result one can see after 50 years.
2. **The look of the natural landscape: finish it** (section 1). In work. Open: the mean colour still jumps between some neighbouring
   scale bars; the closed canopy between drawn crowns is noise; grass does not sway; the texture array's memory on a
   phone. Judge in the angled view, at a phone's size and pixel ratio.
3. **The sorted start**: every world's cover placed fitted to its land. Carried over by hand from `wip/sorted-start`
   (its nursery, about 700 lines); the branch itself cannot be merged. For the port:
   - No race by mass: the mixed stand's own competition says who stays. Newcomers are draws near known kinds, far
     enough to found a slot; no mutant rounds.
   - First the garden check: a garden grown on a cell of main's year-300 world, with its kinds, against the world's own
     mass there. Then main and a sorted world side by side to year 300, with pictures.
   - Do not touch the palette before the kinds are judged, and never to hide a fault.
   - Not carried over: anything of bodies, a renumbering of `rng::Purpose`, `World::rained` as saved state, a second
     wetness or area rule. To mend from the branch: its mutants do nothing, it scores kinds by mass alone (walls are
     cheap mass), it places stands far older and heavier than main's, it adds about nine tuned rows.
   - To `main` only when its pictures are at least as good as main's.
4. **The takers' numbers**, then the gate of F1 (5.4). The faults to start from are in 5.5. After the gate: the first
   drawing of animals (5.1, decision 9), then F2 and F3.
5. **The air.** The planet's dry belt stays too wet: measure the `wind` row at 2, 3 and 5 times (it sets the dry belt,
   its season and the low wind; its row is "anchored, open"). The long air step moves the land's runoff more than
   shorter steps do: `the_long_step_keeps_the_climate` may fail on it; fix it in the air step. Open too: no low cloud
   made by cooling from below; no westerlies (the wet side is the east at every latitude).
6. **More variety of the land.** A wider spread of the bed's wear rates and flat layers on a high land, for cliffs and
   scarps (real layers wear 10 to 100 times apart). One large land in a world of 512 for a large river (it needs about
   100,000 km²) and a wide delta. Channels that lay down their load in the running world (plains, marsh, fans), the
   tiles splitting a channel over its delta (Edmonds and Slingerland 2007; a delta's size is the load at the mouth over
   the water's depth there, Gilbert 1885). Ice that cuts valleys where snow outlasts the summer.
7. **Clean-up**, each checked to the last digit: the tree's tools for cells finer than a square (`DEPTH_MOST` 24 in
   `space/tree.rs`, `Leaf::parent`, `Terrain::below` in `space/below.rs`, a mesh of any number of levels); stale
   comments of a nursery `main` does not have (`cover/around.rs`, `cover/mod.rs`); `Cover::sent`, called by a test only.
   The heat capacity of a ground node is summed in two orders (`ground.rs` `heat_cap_of`, `flow/land.rs`
   `capacities`): joining them moves the last bit, so it is a law commit of its own.
8. **What the land offers, as numbers and maps**: in the tape, per cell and year, what a grazer can digest of what
   grows, seeds made, fresh liquid, the soil's minerals, the make of bed that crops out; on the page, maps with familiar
   names ("food for grazers, kg per km² and year", "fresh water", "building stone", "good soil"). It says how many
   grazers a valley can feed before any is placed: a check of F1, and later of how many people a land holds.
9. **The stages after F1**: F2, F3, F4, F5 (5.6), then the design round for people.
10. **Later, each with the stage that needs it**: life in the water (a floating cover, then takers in liquid cells);
    fresh against salt (a term in drinking and roots; a truly salty sea changes the climate's laws); day and night,
    and the spread of temperature inside a month (frost uses a tuned `frost_k`); a plants-only run of 3,000 years to
    check the carbon gas and the mineral (the gas has no source); sickness, after people; the exact solve of the
    surface's balance by pieces in place of backward Euler (`docs/laws.md` 1.1).

## 4. Open items and known problems

Each law's known limits are in `docs/laws.md`. These touch the plan.

- **Red or skipped tests:** `the_laws_do_not_depend_on_the_size_of_the_cells` is kept as the owner decided (section
  1). `the_long_step_keeps_the_climate` may fail on the land's runoff (priority 5).
- **The variety is not neutral:** cover grows with the setting up to about 8 slots, then stops changing. Full cells of
  stands that pay their way shut better kinds out. A better rule at the limit is open.
- **The moisture gene costs nothing real:** one step of the light gene gives the same capacity at the same upkeep. For
  the sorted start's garden check.
- **Thinning keeps the dead's matter** (`cover::stand::thin` loses plants, not mass); tall plants grow partly on it.
- **Seeds lying:** far more seeds per m² than measured seed banks (a full slot sends all it builds as seeds), and they
  would last centuries where the row says years to decades.
- **Bare places:** cold near-tundra stays bare where real ground holds a low cover; in hot wet months roots cost all of
  a plant's gain, so the plants stay shut (real roots acclimate).
- **No marsh:** soaked ground makes no habitat of its own. Not tried: no seedling under standing liquid; ground that
  differs by its grain.
- **A closed island** draws its carbon gas far below Earth's range. No test holds that world; the page makes none.
- **Chemistry 0:** plant and animal matter are one substance (C:N about 20; real animals about 6), so the mineral never
  limits a taker; dead leaves keep all their nitrogen, so a flame sends too much of it to the air.
- **Water:** liquid poured over a one-sample pond stands over that part only, as a pillar. Where a level truly moves
  (ice, a god's hand) the ground must be worn again there.
- **The page:** a cell with no cover takes green from the cells about it through the smooth blend.

## 5. The next stage of life: takers as a field

### 5.1 The decisions on F1

1. **One parent** may breed. A law of how often adults meet belongs to F2 (it needs movement).
2. **Thirst is a law.** A body drinks open liquid only (standing, running in the month, the soft solid at its heat of
   fusion) and has the liquid of what it eats. Its need is the vapour that cools it plus a least need by body mass
   (Nagy and Peterson 1988). The way to the liquid is paid by the law of walking. No term for what is dissolved until
   the sea is truly salty.
3. **Small kinds** (under 1 kg) may swing; none may die out from the whole land.
4. **`bite_floor` is the refuge the whole stage rests on**, and its anchor is weak. The gate runs at half and twice its
   value too. If plants and takers hold each other only in a narrow band of it, it is marked tuned. The count of tuned
   rows may not grow without a word to the owner.
5. **Young disperse by the seeds' kernel already in F1.** Without a way back, large kinds die out cell by cell by
   chance, and the gate would measure the size of a cell, not the ecology.
6. **Slots of takers join only when alike in every gene**, never because all slots are held: a mean of a grazer and a
   browser is good at neither.
7. **The mineral does not limit a taker** in chemistry 0. Accepted for F1.
8. **The gate is judged on the air as it is on `main`.**
9. **After F1's gate, the first drawing of animals comes before F2**: single animals and soft marks from the field's
   counts, each at a place fixed by its cell, slot and number, under the owner's rules of section 1.

### 5.2 The state

A slot (`fauna::Fauna` in `life::Cell`; one writer, the cell's chunk; up to 8 a cell) holds its adults and young, each
a real count, whole below `N_WHOLE` (64) by keyed draws; the mol of their fuel store and body matter, 64-bit; one heat
(each slot at its own temperature); a running balance; 8 genes. Living mass is matter / (1 − `WET_TISSUE`); that
liquid is not booked.

| Gene | Sets | Its price |
|---|---|---|
| `MASS` | adult size | slower young, fewer members, a higher bite floor |
| `CHILD` | newborn's share of adult mass | fewer births, or longer young |
| `YOUNG` | share of the surplus put into young | no store for winter, or few young |
| `ENZYME` | the gut's capacity to split walls | upkeep, as the cover pays for light capacity |
| `REACH` | the middle of the band of heights it searches | what lies out of the band is not found |
| `KEEP` | the body temperature it keeps | the coat's loss; fast upkeep when warm |
| `COAT` | insulation | heat that must leave as vapour in warm months |
| `REPAIR` | wear life | upkeep |

"Warm or cold" is one axis: a cold-bodied kind is one whose `KEEP` is below its ground's temperature. Group size, prey
by size, the place inside a cell, range per day and habits wait for F2 and F3.

### 5.3 The month's pass, in outline

In the cell's life pass right after the cover's month, on the same view of the store; the cover feels what was taken in
its own next month. Details in `docs/laws.md` 3b.

1. **Body and pace**: the body stands at `KEEP` or at the ground's temperature; every rate runs at the rate law's pace.
2. **What it reaches** through the contract: each cover slot as a soft part and a frame by height, and the seeds lying;
   a band of heights about `REACH`, above `bite_floor` times its length over the snow; a jaw cuts only what it can
   rupture. What is out of reach is the cover's refuge.
3. **The take**: Holling's disk law over many layers and slots, bite by bite, in 6 steps of 5 days. Each layer falls
   by S (1 − e^(−k Δ)): never overdrawn, shared by rates, not by order; a slot switches layer as one runs out.
4. **Digestion, upkeep, heat**: the gut splits walls up to its capacity; Kleiber's rest, the walk while searching, the
   coat's loss; a resting body sits in the remains on the ground; excess heat leaves as vapour.
5. **Thirst**: the walk to open liquid; work is cut first, deaths only where no work is left to cut.
6. **Deaths** by unpaid energy, wear, frost and thirst; the dead go to the ground as remains.
7. **Growth and births**: the growth cap from Kleiber (West, Brown and Enquist 2001); maturing as a flux across a mass
   (von Foerster); one parent; the mother pays for her unborn.
8. **Dispersal and mutation**: maturing young leave by the seeds' kernel through an outbox, landed in cell order;
   newborns join their slot or found one by the shared slot rule (`life::kin`).

### 5.4 The gate of F1

Years 100 to 500 after the takers' start, the roster world, size 32, seeds 1 to 3, against a plants-only twin. Not
built yet as a slow test; the fast tests are in `core/tests/fauna.rs`.

| Check | Real range | Passes when |
|---|---|---|
| Share of growth eaten | median 18%; a few % in forest, 10 to 60% in grassland (Cyr and Pace 1993; McNaughton and others 1989) | land mean 2 to 40%; herb cells lose more than tree cells |
| Takers' mass | 1e2 to 1e4 kg/km², typical 1,700 (Fritz and others 2002) | land mean 300 to 10,000 kg/km², rising with growth across cells |
| Numbers against mass | slope −0.75 (Damuth 1981: log10 D = 4.23 − 0.75 log10 W, W in g, D per km²) | slope over living kinds in −0.5 to −1.0 |
| Hold each other | | every kind over 10 kg alive at year 100 lives at 500, its yearly count varying under 0.5 and its 20-year low above a fifth of its high; small kinds may swing but none dies out; under 5% of land cells under a tenth of the twin's reachable cover for 3 years running; the cover at least half the twin's |
| Thirst | | on a land with a dry season, large kinds' mass falls with the way to open liquid; kinds under 1 kg less or not at all |
| Books, cost | drift under 1e-9 | every year; under 40 µs per land cell and month at sizes 32 and 64 |

### 5.5 Risks and open faults, most serious first

1. **Grazers eat the reachable layer bare and the pair swings.** A cell's takers eat all they can reach within the
   month; everything rests on the refuge (the bite floor, the band of heights, the walls' toughness). Watch
   `bite_floor` first.
2. **Large grazers eat too easily**: the soft part breaks so easily that the most they eat is many times their need.
   Walls inside the leaf (one anchored row: 30 to 60% of a leaf's dry mass) would bring it near the real. Not built.
3. **The 30 g kind dies out.** Its limit is its chewing: the `chew` row stands for a whole day's eating but is spent
   over the working half of the day (`busy`); it finds nothing for three months under snow; seeds are mostly walls.
   Chewing spread over the feeding hours made it live in a trial (not committed, not checked).
4. **Births are too many**: the measured figure counts young at weaning; ours feed themselves at once, at a few percent
   of the adult's mass, where real young become independent at 17 to 40%.
5. **No part below the ground is a refuge** from a mouth (a root-to-shoot share; the fire's `base_share` may serve).
6. **Large kinds die out by chance** in cells of few members; the kernel is the way back. Measure times to extinction.
7. **The slot rule selects between kinds; it never splits one** (a gene moves 0.063 steps a generation; the join is 2
   steps wide). A split needs a law of its own: a slot's spread of a gene, split where the two ends do better than the
   middle. F2 or later.

### 5.6 The stages after F1

| Stage | Goal | Ends when |
|---|---|---|
| **F2. Movement** | members flow to the 8 neighbours by a spread and a pull by what they sense next door (food, crowd, warmth, eaters), each weight a gene, paid by the walk; a meeting law for breeding | kinds sort by zone; the gain per member comes out nearly even between neighbours without being put in (the ideal free distribution is a check, never the rule); a wide ranger's mean latitude turns over the year |
| **F3. Eaters and the web** | eaters of takers and of remains; group size (less risk each, Hamilton 1971; more fight for food); meetings by how two kinds' places overlap in a cell | eater mass grows as prey mass to about 0.75 (Hatton and others 2015); eaters and prey swing in some places; a kind dies out in a region and comes back from next door |
| **F4. Animals on the page** | single animals and herds drawn from the field, on the ground their kind likes, each at a place seeded by (cell, slot, index), holding no state | the owner sees a herd at the 200 m view where its kind likes |
| **F5. Habits** | first inherited weights over what is sensed (no hidden layer; all weights at zero is the null run); then learned habits as a field, passed to the young by a share that is a gene (Boyd and Richerson 1985) | a habit spreads faster than a gene, against the null run |
| **P. People** | a design round with the owner first | a page of decisions before any code |

For all of them: a kind uses only what it can sense, at a cost, with weights that are genes (nobody writes
"migrate"); build one trophic level and make it stand before the next; the places inside a cell are one function the
core and the page both read; kinds that live for days are rates, not slots; make no object before a kind wants one.

### 5.7 People: the field is not enough

A field can hold "0.3 houses per km²", not one house in one valley with its family, its store and its path. So people
will need a thin layer of single agents and things (a band, a household, a store, a boat) on top of the field, each
with a place inside a cell, a memory and what it holds. The passage keeps exact books, real counts and one writer for
each number: a member leaves a slot with its exact share of the slot's matter and heat and joins again by sums
(section 6). The contract already lets a single agent be one more taker beside the field.

## 6. Laws kept for the animal field

The anchored laws of the old animal world (its code is at commit `16a774d` of main's history) are now in
`core/src/fauna/` where F1 needs them: Kleiber's upkeep, pace by temperature, the coat's heat balance, the walk, wear by
the rate of living, the brood, frost, breaking. Their rows are in `docs/numbers.md`. The walk, the bite and the
breaking stress replace the old tuned `sweep`, `c_handle` and `take`; the growth cap replaces `c_build`. Kept for later:

| Law | Formula | Values | Source | For |
|---|---|---|---|---|
| Drag in liquid | J per kg per m = ½ ρ `DRAG` (wet L²) v² / (m `MUSCLE`) | `DRAG` 1.05, `MUSCLE` 0.25 | Hoerner 1965; Smith, Barclay and Loiselle 2005 | takers in water |
| A held edge | each material's energy × min(1, e / l) where the edge is harder; the edge loses `abrade` × W / (`TABOR` σ) | `abrade` 1e-5, `TABOR` 3 | Atkins 2009; Tabor 1951; Archard 1953 | teeth, tools |
| Strength | upkeep term `c_strength` × STRENGTH, 1.12e4 J/(kg^0.75 day) at `t_cat` | | Elia 1992 | fights, pulling |

**The exact split and join of members** (`nodes::Lot`, `nodes::rejoin` at `16a774d`), for single agents:
- A split takes the share f of the members and the share fa of the stores, heat and brood (fa > f where those who
  leave hold more); the rest stays, so the two add to what was held.
- A join adds members, stores, heat and brood as sums; genes and sizes become means by members; a gene's spread pools
  each one's spread plus its gap to the new mean.
- Members back from acting apart rejoin by amounts and sizes alone.

## 7. Branches with unmerged work

On the remote (`git ls-remote --heads origin`). Study files never go to `main`.

- `wip/sorted-start`: the sorted start, to carry over by hand (priority 3). `wip/nursery-start`,
  `wip/cover-nursery-g2` and `experiment/sorted-start` are its older states.
- `wip/look-refs`: the owner's reference pictures (`review/looks/faults/`, `review/looks/want/`).
- `wip/animals-look-dots`, `wip/animals-look-bodies`, `wip/animals-look-herds`: the three rejected looks of animals.
  Not to merge; their road (each square's slots sent to the page) may be read for F4.
- `wip/years-centred`, `wip/years-moist-try`: two failed tries at the heat of the weather's drawn flow (the commit
  messages say why). Next idea: the rising column's static energy goes to the sinking area of its group.
- `tools/checks`: check scripts and probes (climate, ground, sorted start, the page's cost).
