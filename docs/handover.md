# Handover: Emergent World, version 2

**For:** an AI agent (or engineer) who works on version 2.
**From:** a design conversation between the project owner and Claude, after a review of version 1, an earlier attempt with the same goal. You do not have version 1. Everything useful from it is written in this document.

Read this whole document before you write any code. Sections 1 to 4 explain *why*. Sections 5 to 12 explain *what* to build. Sections 13 to 17 explain *how* to work. Section 18 holds the ideas kept from version 1.

The other documents:

- `docs/laws.md`: how each law works **now**, part by part, with the code that does it.
- `docs/numbers.md`: every free number, its unit, its source and whether it is physical, anchored or tuned.
- `docs/tape.md`: what a run records.
- `review/roadmap.md`: the plan, the priorities and the owner's decisions.
- `review/orchestrator.md`: the handover of the work, with where things stand.

---

## 0. How to use this document

- **Which document wins.** This document holds the principles and the rules of work: where another document conflicts with it on those, **this document wins**. `docs/laws.md` and the code are the truth for how a law works now. `review/roadmap.md` is the truth for the plan and the order of work.
- **Version 2 lives in this repository.** You do not have the version 1 code or documents, and you do not need them. Section 2 explains what went wrong in version 1. Section 18 contains, in full, the few ideas worth keeping from it.
- When the owner changes a decision, update this file in the same change (and `docs/laws.md` when a law changes).
- When this document says "must", it is a decision the owner has made. When it says "suggested" or "default", you may change it if you measure a better option. Tell the owner when you do.
- When you are unsure, re-read Section 3 (principles). Most hard choices are answered there.

### Working with the owner

- **Write to the owner in very simple English.** Short sentences. Common words. Explain any technical word the first time you use it. The owner is not a native English speaker.
- **Be honest.** If something does not work, say so plainly, with numbers or a picture. Never call a timeout, a skipped check or a partial run a success.
- **Show, don't describe.** The owner wants to *see* the world: screenshots, short videos, maps. Every milestone ends with something visible.
- The owner cares about **emergence first**. Realism is welcome only when it helps things emerge.

---

## 1. What we are building

A **game** in which a whole world lives and develops by itself over thousands of years, and the player watches it on a map they understand instantly.

The world starts with physics, chemistry and simple life. Everything else must **emerge**. Nobody writes it in the code. This includes:

- climate, climate change, ice ages, deserts, rain forests, rivers carving valleys
- plants and animals adapting to places; food chains; herds; extinctions
- people (or people-like beings) spreading across land; landscape shaping their habits and cultures
- something like **agriculture** and **domestication** of animals
- **settlements**, then towns, then cities of different types (dense, spread out, walled, ports)
- **trade routes** by land and sea; discovery of far lands
- tools, then metal-like materials (**bronze-like**, **iron/steel-like**), new **energy sources** (wood-like fuels, fossil-like fuels, wind, flowing water, heat engines)
- **transport**: walking, pack animals, carts, boats, ships, and maybe car-like and plane-like machines
- **conquest**, kingdoms, empires, **dynasties**, **cultural dominance** of one group over others
- medicine (for example, something **penicillin-like**), gathering halls (**temple-like**), houses, walls, workshops

None of these needs to match Earth exactly. They can be new mixtures of physical and chemical properties. **After** they appear, a separate translation layer recognises them and shows them with the closest familiar Earth concept, for example "≈ bronze" or "≈ sailing ship".

### What the player can do

1. **Watch.** A 2D map with small angled sprites, from the whole world down to one street. A world is one map of 128 x 128 squares of 2 km (256 or 512 squares a side at the very most): one island or several, not a planet. Speed controls. A timeline over thousands of years.
2. **Understand.** Things look like what they are: a house looks like a house, a herd like a herd. Clicking anything explains what it is, what it is made of, its history, and why it got that name.
3. **Play god.** Set a city on fire, move or dry up a river, cause a drought, change the climate, add or remove animals. Then fast-forward and see what happens over the next 200 years. **There is only one timeline** (Section 11).

---

## 2. Why version 1 failed (lessons you must not repeat)

Version 1 had the same goal and good first ideas. It grew to about **440,000 lines of Rust, 255,000 lines of documents and 3,000 tests**, and **nobody ever watched anything emerge**. Its own review says: "No game has been played." The main check ("do the patterns appear in 70% of worlds?") was never run.

What went wrong:

1. **Rules were written at the level of results, not causes.** Version 1 has hand-written code for a market with buy/sell order books, organisations with "offices", land parcels and leases, loans with pledges, and farming practices (terraces, fences, manure) that unlock after a trigger. Each of these is a named system. The concept said "no `create_council` rule", but the code wrote councils anyway.
2. **The "choice" rule was many small scripts.** Functions like `in_sowing_window` and `winter_stocking_window` told agents when to sow and when to store food. Agents did not work it out.
3. **Realism won over emergence.** The code modelled phosphorus in rivers, groundwater temperature and wild boar piglet survival from field studies. Every number needed a source and a review. This rewards detail, not emergence.
4. **Machinery came before results.** Exact replay across machines, receipts, GPU kernels "from day 1", save formats and proof campaigns were built before anyone checked that the rules produce history.
5. **The scale did not fit the goal.** One valley, 1-hectare cells, one step per day, about 500 people. You cannot reach empires or steel from there in a run you can watch.
6. **Process overload.** Adversarial reviews, decision records, qualification stages and test-receipt rules consumed most of the effort.

**The single most important lesson:**
> Write the **laws of the world**, never its **history**. A law must be about causes (how matter, energy and choice behave). It must never be about a result we expect (a route, a farm, a market, a house).

**A test for any rule:** *Can you list all of its uses in advance?* If yes, it is a named system in disguise. A good law has uses you cannot list. Example: "moving matter wears the matter it moves through" gives rivers carving valleys, animal trails, footpaths, roads and trade routes. Nobody wrote any of them.

---

## 3. Principles (must)

1. **Laws, not outcomes.** No code in the simulation core may describe a human or Earth concept (farm, herd, house, city, market, money, king, temple, road, river, metal, fire…). Section 14.4 gives a word ban that is checked automatically.
2. **Five laws.** The whole world runs on the five laws of Section 6. A sixth law may be added only under the rules of Section 14.2.
3. **Everything is a graph.** One data structure holds the whole world: nodes and edges (Section 5).
4. **Watch first, measure second, polish last.** Build the smallest thing that can be watched. Look at it. Then decide.
5. **Physics makes things possible; it never makes them happen.** If an Earth-like thing cannot appear, add the *physics* it needs (Section 9.4). Never add the thing.
6. **The translation layer is one-way.** Names, Earth matches and stories are computed *from* the history. They never flow back into the simulation.
7. **One timeline.** No branches, no alternative futures.
8. **Cheap repeatability only.** Same seed, same version and same machine give the same history. Do not build anything more (Section 12.5).
9. **Small code.** Keep `core` small by computing less (Section 14.3).
10. **Speed matters.** Thousands of years must run in minutes. Rough far away, detailed where it matters (Section 7).

---

## 4. Decisions already made

| Topic | Decision |
|---|---|
| Core idea | 5 laws on one graph: Flow, Change, Bond, Copy, Choose |
| Coordination, trade, property, institutions | **Not** primitives. They must emerge from Choose + Bond + Flow + Copy (+ signals) |
| Actors | Plants are a field per cell, the cover, not bodies. Animals are being rebuilt as fields (`review/roadmap.md` section 4); the old animals, bodies merged into population nodes, are deleted (the tag `animals-v2` keeps them). People come after the animal layer |
| Body scale | Living things are **real-sized**: an individual weighs what a real living thing weighs (grams to tonnes, set by its genes and selection). Plant cover holds real mass per m² (about 1 to 50 kg/m²). **No hard-coded limit on how much a cell holds**: density is an outcome of light, cover, crowding and food. Speed comes from fields (Section 7.1) and better algorithms, never from giant bodies |
| Free numbers | A number comes from a real measured size where one exists (anchored). The count of tuned numbers may not grow (Section 14.3) |
| What a body is | A body is fuel plus a body built of matter. Building matter costs work and time; storing fuel is cheap |
| Children | Bodies copy by physics only: a mature body puts a share of what it builds into children, never by a brain's choice |
| Size | Body size is set by energy (keeping tissue costs by the kg, supply grows more slowly), never by a cap |
| Ageing | Bodies die of age by their rate of living; how long they live is a gene with a cost |
| Temperature and seasons | The whole tempo of a body follows its temperature by the rate law. **No calendar rule, ever**: breeding seasons must emerge |
| Liquid | A body's chemistry runs in its liquid: a dry body is nearly dormant |
| Damage | Hits break body matter; abilities read the living body; healing is growth |
| Mixing | Children may have two parents; only alike bodies mix, so kinds are the groups that mix |
| Warmth | A body keeps a temperature only by genes it pays for (`KEEP_T`, `COOL_T`); one heat network for every body |
| Soil | The ground holds liquid in pores. One mineral (nitrogen) comes from plain chemistry, so fertility differs by place |
| Nitrogen fixers | A tenth of the starting light-eaters carry the gene that fixes the air's nitrogen: an initial condition from real floras (a rule chosen for a run, Section 14.3) |
| Carbon | The sea holds carbon by plain chemistry and starts at balance with the air |
| Law 2 | One rate law (mass action with the contact of the phases) in the open world and in bodies; a catalyst is slowed by its own product |
| Moving | Moving needs force |
| Materials | Bodies are built of several materials; physics reads the material (breaking, digestion) |
| Freezing | One freezing law for bodies, ground, lakes and the sea |
| The start | What is placed at the start is an initial condition and every run prints it. Plants are the cover, a field with no brain: their life is the physics of their numbers. The rebuilt animals start as the roadmap's design says (section 4) |
| Waits | Natural fire starts and disease (bodies inside bodies) wait until after the animal layer (`review/roadmap.md` section 7) |
| World chemistry | One hand-made, Earth-like "chemistry 0" (numbers only, no names in core). No random chemistry is planned (Section 6.0) |
| Earth concepts | Only in the recognition library, viewer and analysis code. Never in the core |
| Visuals | **2D map, no 3D**: colour-coded pixels for land plus small axonometric sprites. One Earth view, with layers of the model's numbers over it |
| Translation | Pattern finder (normal code) → fact cards → matcher (normal code) → small LLM for leftovers and stories (cached, read-only) |
| God mode | Physics edits on one timeline, auto-save before each action, "then and now" view |
| Timelines | One only |
| Repeatability | Cheap kind only |
| Language | Rust for the core (compiles to native and to WebAssembly), TypeScript for the viewer |
| Version 1 | Not available and not reused. The useful ideas are written out in Section 18 |

How each of these works in the code: `docs/laws.md`.

---

## 5. The world model: one graph

### 5.1 Nodes and edges

The world is a graph.

- A **node** is anything that holds stuff: a patch of ground, a column of air, a body, a part of a body, a made object, a population of bodies, a whole region (when seen from far away).
- An **edge** is a link between two nodes that lets stuff pass, or holds them together.

A node holds:
- `amount[k]` for each substance `k` (Section 5.3), as a non-negative number
- `energy` (heat), from which temperature is derived
- for spatial nodes: position, elevation, area, volume
- for bodies: genes, brain, memory (Section 6.5)
- optional children: a node can contain a smaller graph (Section 5.2)

An edge holds:
- `conductance` for each phase (how easily solid, liquid, gas, heat and signals pass)
- `length` (gives travel delay)
- for bonds: `strength` and `stiffness` (Section 6.3)
- an optional in-transit buffer for slow, long edges (Section 6.1)

**There are no edge types like "road", "river" or "trade link".** Every edge is the same kind of thing. What differs are its numbers. A route is simply an edge whose conductance became high because it was used a lot.

### 5.2 Three layers: fixed cells, a fixed ground, a picture

```
Layer 3  THE PICTURE  per pixel, on the graphics card, only where one looks. It reads layers 1 and 2 and writes
                      nothing back.
Layer 2  THE GROUND   fixed samples, made once at the world's start by an erosion law. It is state, and it changes
                      slowly.
Layer 1  THE BOOKS    slow physics and life as fields on fixed cells. Every element and the energy are kept exactly.
```

- **Fixed cells.** The cells of a world are made once and stay the same for its whole life. The same seed gives the
  same world; the camera is not an input of the run. Zoom is drawing. A field is a number per cell (kg per m²), not a
  list of single things, and the cost of a cell is fixed whatever lives in it.
- **A fixed ground with a history.** The ground below the cells is held at samples (125 m on the island), made once
  when the world starts by running the erosion law (the stream power law, creep, the rebound of what was worn). So it
  has valleys, ponds with one level and one network of channels. `docs/laws.md` 0 says how.
- **The picture per pixel.** The page draws the map from tiles of the ground and the cells' table (10.1, 10.3). It
  makes nothing the world reads.
- **Inside a cell.** A cell holds means (its plant cover, the liquid in its soil, its snow). Inside it, one rule shares
  each mean over the cell's own ground, from the ground's shape alone: how much water gathers at a point (its upslope
  area over its slope, Beven and Kirkby 1979), its slope, the way it faces, its curve, and its height over the cell's
  liquid level; and for each plant kind, how wet a place its genes like. The shares keep each mean exactly. The page
  draws inside a cell with the same shares, so what the owner sees is what the world goes on with.
- **Detail is derived, not simulated.** Whoever needs detail calls the law inside a cell at a point: the picture today,
  a single animal later. The cover holds kg per m²; the count and size of its plants follow from it. Nobody stores a
  tree.

### 5.3 Storage

- **Space** (land, sea, air) is a regular grid. Store it as plain arrays, one array per quantity ("struct of arrays"). The grid is still a graph: each cell's neighbours are its edges. The laws are the same, the storage is just faster.
- **Bodies, structures and long links** are a sparse graph: arrays of nodes and edges with stable integer IDs and a generation counter for reuse.
- Use chunks (for example 32×32 cells) so parts of the world can be saved, loaded, collapsed and expanded separately.

### 5.4 World sizes

- **A world** is one square map of 128 x 128 squares. A square is 2 km, so the map is 256 km a side. 64 x 64 is the
  size to judge by in tests and pictures. 256 or 512 squares a side is the most we plan for.
- A world of 512 has 64 times the cells of a world of 64, so it costs about 64 times as much per year. Sizes above 128
  wait until speed is looked at (`review/roadmap.md` section 7).
- There is no planet and no globe as a world to play. Worlds should differ as much as they can: one island or several.
- There is no speed target for now (12.6).

---

## 6. The five laws

Each law is one equation, or a small set of equations, applied everywhere. Each law lives in its own folder (`core/src/<law>/`) and stays small.

This section is the design of each law. How each one works now, in detail and with the code that does it, is in `docs/laws.md`.

Notation: `a[i,k]` is the amount of substance `k` in node `i`. `E[i]` is the heat energy of node `i`. `T[i]` is its temperature.

### 6.0 The world's chemistry

The world has its own chemistry. This is not a law. It is the **data** the laws read.

**Decision: one hand-made, Earth-like chemistry, "chemistry 0". No random chemistry is planned.**

- It is copied roughly from Earth: 6 elements, 14 substances, 14 reactions. Only rough numbers (melting points, densities, reaction heats); never precise field data.
- In `core` and `worldgen`, substances are **numbers only** (substance 0, 1, 2…). The data file holds numbers, not names. The Earth names of chemistry 0 ("≈ water", "≈ sugar") live **only** in `library/`, as a normal recognition entry. The word ban still holds.

**Elements.** A small number of base elements, for example `B = 6 to 8`. Elements are never created or destroyed.

**Substances.** Each substance is a small vector of element counts (for example `[2,0,1,0,0,0]`). This makes conservation automatic: every reaction must balance element counts.

**Substance properties.** Each substance gets physical numbers, set by hand in `worldgen/src/chem0.toml`; where a number is derived, the derivation is in `docs/numbers.md`:

| Property | Used by |
|---|---|
| molar mass | weight, gravity, buoyancy |
| heat capacity | temperature from energy |
| melting point, boiling point | phase: solid / liquid / gas at a given temperature |
| density (per phase) | volume, buoyancy, layering |
| hardness and bond strength (solid) | Bond law, cutting, wear |
| the stress it breaks at (solid as a material) | the energy to break what a body is built of |
| reflectance in 3 visible bands | colour on screen, how much sunlight is reflected |
| absorption of outgoing heat radiation (gas), and the column amount past which it grows only as the square root of the amount (its bands fill up, a stand-in for the logarithm of real gases) | greenhouse effect |
| solubility in each liquid | dissolving, carrying |
| conductivity of heat | heat flow |

**Reactions.** Each reaction has inputs and outputs (balanced in elements), a speed `k`, an energy barrier `Ea`, and an energy change `ΔH` (gives heat or takes heat). Some reactions take **light** instead of heat as their energy source.

**What chemistry 0 offers.** These are physical possibilities. They never place an outcome or name an Earth thing.

- A common substance that is **liquid** across most surface temperatures: it rains, flows and wears the ground.
- A **gas mixture** with one gas that **absorbs outgoing heat**: climate and climate change.
- One reaction that **stores light energy** in a solid: the energy of plants.
- Energy-rich substances that **release it** when they react with the oxidising gas above some temperature: burning and breathing.
- Rock of five makes, with different hardness and bond strength.
- It has **no metal**: no solid is freed from rock by heat plus fuel, and no pair of hard solids mixes into a harder one. Useful rocks and metals are an open item of the roadmap (`review/roadmap.md` section 6).

**Chemistry 0 as built** (`worldgen/src/chem0.toml`: 6 elements, 14 substances, 14 reactions) also has what life needs: a **body matter** that bodies build from the energy-rich solid by a growth conversion (`Chemistry::build`), so the fuel a body stores and the tissue it is built of are different matter; one **mineral** that body matter needs (a nitrogen mineral, living dissolved in the liquid); and one more **solid bodies can build** (a frame, `Chemistry::builds`). The roles and reactions are listed in `docs/laws.md` (2.7).

### 6.1 Law 1: Flow

**Stuff moves along edges, from high "push" to low "push", with delay and loss.**

```
J[e,k] = g[e,phase(k)] · (Φ[i] − Φ[j]) · share[i,k]
```

- `J[e,k]` is the flow of substance `k` along edge `e` from node `i` to node `j`.
- `g` is the conductance of the edge for that phase.
- `Φ` is the "push" for that phase:
  - **gas:** pressure, `p = n · T / free_volume` (ideal-gas-like)
  - **liquid:** head, `elevation + liquid_depth`. The first liquid of a ground node fills the pores of its loose ground; only the liquid above them has a head. Liquid in the pores flows by Darcy's law.
  - **dissolved or loose solids:** concentration, and they are carried by the moving liquid or gas they sit in. A salt that lives in the liquid is written as a liquid that never boils: it moves with the liquid by its share and stays when the liquid evaporates.
  - **dissolved gases:** a gas that cannot condense on the ground dissolves in the liquid there by Henry's law and moves with that liquid.
  - **heat:** temperature, `J_heat = g_heat · (T[i] − T[j])`, plus heat carried by moving matter
- `share[i,k]` is the fraction of the moving phase in node `i` that is substance `k`.

**Delay.** If `length / speed > dt`, the flow enters the edge's in-transit buffer and arrives after the delay. This matters for long, coarse edges such as sea crossings.

**Loss and dissipation.** Moving stuff loses energy along the edge: `P = |J| · ΔΦ`. This dissipated energy is not thrown away. It is **available to do work at the place it is lost**:

1. **Carry loose solids.** Moving liquid or gas can carry loose solid material up to a capacity proportional to `P`. Above capacity, material settles. This gives erosion, sediment, deltas and valley carving.
2. **Break weak bonds.** Dissipated energy can break bonds whose strength is below the energy available, for example plant structures or loose ground. This gives wear.

The sizes of carrying, wear and creep are anchored to real erosion rates (`docs/numbers.md`).

**Bodies moving** across an edge also dissipate energy (their movement cost), and it acts the same way. So repeated movement removes obstacles and makes the ground cheaper to cross. **This one sub-rule is what makes rivers, trails, paths, roads and routes emerge.**

**Movement cost for bodies:**
```
cost = base(body mass) · slope_factor · (1 + resistance of bonded structures in the destination node)
```
When the structures (for example plants) are worn away, the cost falls.

**Radiation (climate).** Radiation is also Flow, between surface nodes, air nodes and a single "space" node:
- The sun gives energy to surface nodes by latitude, season and planet settings (sun strength, axial tilt, day length). The island's rows span only what its size spans on the planet (40° to 42.3°, where Earth's surface is near its mean); an island can stand in the planet's air with a real latitude band (`--band`, `docs/laws.md` 1.6).
- Surface reflects part of it, based on the reflectance of its substances (ice and sand reflect more, dark plants less).
- Surface emits `ε · σ · T⁴`.
- The air column absorbs a fraction `1 − exp(−x)` of the outgoing heat, where `x` (the optical depth) is the sum over the gases of amount × absorption.
- The air emits through a few grey layers whose temperatures fall with height, so more gas lifts the layer that reaches space into colder air and the ground keeps warming, with no ceiling.
- Light that bodies absorb warms the surface as it falls, except the share their light reactions store; and bodies radiate like every surface.
- Burning energy-rich substances can add heat-absorbing gas to the air. **Climate change then emerges with no climate-change rule.**

**Wind.** Air moves between air nodes by pressure differences (from temperature differences). Planet rotation (deflection of moving air) is an optional physical setting. It gives prevailing winds, which matter for rain patterns and sailing.

**Stability rules (must):**
- Never let an amount become negative. Before applying flows, scale down all outgoing flows of a node if they would take more than it has.
- Respect a stable time step (`dt ≤ capacity / total conductance`), or use sub-steps.
- Check conservation of every element and of energy, with the declared boundary terms (sun in, radiation out to space).

### 6.2 Law 2: Change

**Substances react inside nodes, faster when hot, by the world's chemistry table.**

```
rate[r] = k[r] · exp(−Ea[r] / T) · contact[r] · Π over all inputs (share[input]^order) · catalyst[r] · light[r]
          (reaction units per second; contact = the least, over the inputs, of the amount of the input's phase per ν)
lim[r]    = the least, over the inputs, of the input's amount per ν (the scarce input)
extent[r] = lim[r] · (1 − (1 + p·z)^(−1/p)),  z = rate[r] / lim[r] · time   (1 − exp(−z) when p = 0)
p[r]      = (order of the scarce input − 1) · (1 − its share), in a gas or a liquid; 0 for a solid
da[k]/dt ≈ Σ_r (out[k,r] − in[k,r]) · rate[r]
dE/dt   = −Σ_r ΔH[r] · rate[r]
```

- `share` is the amount's share of its phase (gas or liquid); a solid is a phase of its own (share 1). So the rate has no jump when another input becomes the scarcest, and it is first order in a trace input.
- `catalyst[r]` is 1 in the open world. Inside a body, the body's genes can raise it for a few chosen reactions (Section 6.4). **This is metabolism.** "Eating plants" means "my genes speed up the reaction that breaks down what plants store". Keeping a catalyst costs energy each step, by its capacity. A body's catalysts fail when hot, work only as far as the body is wet, and are slowed by their own product (`docs/laws.md` 2.4).
- `light[r]` is used only by light-driven reactions. They take absorbed light energy instead of heat.
- Phase changes (melting, freezing, boiling) follow from temperature and the substance's melting and boiling points. A liquid freezes at its mixture's melting point, lowered by what is dissolved in it.

Burning, rotting, rusting-like decay, smelting-like reduction, cooking-like changes, digestion, growth and the decay of old buildings are **all this one law**. Growth is the same kind of conversion, not a free one: it runs only in bodies and is paid by the work of Copy (Section 6.4).

### 6.3 Law 3: Bond

**Nodes can be joined. Joins break when pushed too hard.**

- A **bond** is an edge with `strength` and `stiffness`. Strength comes from the substances at the joint and how the joint was made.
- A **structure** is a group of nodes connected by bonds. It moves as one unit.
- **Load.** Each part has weight. Weight passes along bonds down to where the structure touches the ground. A simple approximation is enough: for each bond, the load is the weight of everything that hangs on it. Add pushes from moving bodies, wind, water and impacts.
- **Failure.** If `load > strength`, the bond breaks. If the structure loses its support, parts fall.
- **Holding.** A body holding a part is a bond too. Its strength is the body's force (strength gene × its living body × gravity × grip). A body takes hold of a part only if that force bears the weight of the part's structure, and lets go when it no longer does.
- **Force is of the body.** Taking, lifting, holding, carrying, digging and pushing go by the living body, not by the fuel it carries; a hurt body is weaker. **Moving needs force:** a body moves only if its force can push what it passes.
- **Materials.** How high a body stands is what its materials hold up against its weight; the energy to break a body is what its materials resist (`docs/laws.md` 3.6 in the tag `animals-v2`).
- **Making bonds.**
  - By Change: some reactions make solids that join their neighbours (setting, drying).
  - By Choose: a body spends energy to join two things it holds or touches.
- **Containment.** If a structure's parts close around a space, that space becomes a child node. The edges between the inside and outside get the low conductance of the walls (from wall materials and how closed the walls are). So a closed structure keeps heat in, keeps liquid in, and keeps bodies out. "Containment" is not a separate rule. It is just low-conductance edges.
- **Floating.** A structure in a liquid gets an upward push equal to the weight of liquid it displaces. Hollow structures displace more.

**Geometry, added in stages.** Parts need a little geometry for some things to be possible:
- **v1:** size (3 numbers), density, hollowness. This allows tools, shelters, containers and floating things.
- **v2:** rotating joints (a bond that allows turning) and roundness. This allows rolling, so wheel-like things become possible.
- **v3:** drag and lift (a flat part moving fast through gas or liquid gets pushed sideways). This allows sails, then plane-like things.

Add v2 and v3 only when the world is ready for them (Section 9.4).

### 6.4 Law 4: Copy

**A piece of the graph can be copied, with small random changes.**

- **Bodies have children.**
  - A body builds its own matter from food, paying work (growth). Once mature, it puts a share of what it builds into a brood; whenever the brood covers a child, a child is made. The child gets that matter and provisions of the parent's other stores. The parent keeps its body.
  - The child's genes are the parent's genes plus small noise.
  - **Two parents.** A mature individual may find an alike mature partner in its cell; each gene of the children is then one parent's. With no partner, one parent. A population node's children have one parent: its members already mix their genes inside it.
- **Imitation.** A body that watches another body can copy part of its brain, its learned behaviour. This costs time and attention. It is how culture, skills and designs spread.
- **Matter is never copied for free.** Copying a body costs real substances (Law 1 and Law 2 still hold). Copying behaviour costs time.
- **Structures are not copied directly.** Bodies that learned to build will build again. Bodies that imitate them will build similar things. That is how "designs" spread.

**What genes hold:**
- body plan (size, which parts, their materials and the share of each)
- which reactions the body catalyses, and how strongly
- minimum safe amounts of key substances (below them, the body dies), and how much of each it keeps
- reproduction settings (size of maturity, share of surplus for children, size of a child)
- the starting state of the brain, its sizes, and how fast it learns
- how much it repairs itself (a longer life for more upkeep)
- its insulation and the temperatures it keeps (it burns fuel when colder, evaporates when warmer)

### 6.5 Law 5: Choose

**Some nodes have a small brain. They see a little, act, learn what helps them, and copy those who do better.**

**Perception** (limited, local):
- own stores, energy and temperature
- the main substances in its own node and neighbouring nodes
- up to N nearby bodies: their visible traits, what they hold, their signals, their distance
- memory slots (recent events, known individuals and how past dealings with them went)

**Actions** (generic, physical; each costs energy computed by physics):
- **move** to a neighbouring node
- **take** a substance from a node, a structure or another body
- **put** a substance somewhere (a gift to another body is of stores only, at most what the receiver could take)
- **join** or **break** a bond (hold, let go, attach, detach)
- **push** a structure (apply force)
- **signal** (emit a value that nearby bodies can perceive)
- **watch** another body (imitation)
- **rest**

Having children is not a choice: it is physics (Section 6.4). There is no "farm", "trade", "build house" or "attack" action. Those are patterns of the actions above. Attacking is "take from another body" or "break bonds in another body". Trading is "put near another body" followed by "take" in return.

**Learning:**
- **Within a life:** the brain changes to raise the body's own well-being. Well-being means energy and key stores staying above the minimums. Do not hand-shape rewards for specific behaviours.
- **Across lives:** evolution (Law 4) changes how the brain starts and how it learns.
- **From others:** imitation (Law 4) copies parts of the brain of bodies that seem to do better.

**Memory of individuals.** A body remembers a few other bodies by their visible traits, and how dealings with them went. This general ability is what lets reciprocity, trust, revenge, loyalty and leadership emerge. It names none of them.

**Signals.** A signal has no meaning at first. Meaning comes from learning. If signals become useful, a language-like system may emerge.

**Planning** (a first step is built: a look-ahead of one step with the critic, `docs/laws.md` 5.3 in the tag `animals-v2`): bodies learn a small model of "what happens if I do X" and look a few steps ahead. This makes long plans much easier to find. Taming an animal, for example, pays off only after many seasons.

**Implementation:**
- A small neural network per body: a base per lineage, rows that are genes of each body, plus what the body (or its population node) learned. Keep the brain code apart from the rest, so it can be swapped.
- **One brain per body or population node.** Each body has its own brain rows; a population node's members share the node's.
- **A brain has a size and a cost.** Its sizes are genes, and zero is a size: a body with no hidden units takes no decisions and pays nothing for a brain. A brain is paid and learns per second of its pace, not per decision.
- Simple learning is enough to start: a policy gradient against a learned baseline (actor-critic), plus evolution.

### 6.6 What is deliberately NOT a law

These must emerge. If one does not, improve physics, perception, memory or learning. **Do not add it as a law.**

> farming, herding, domestication, trade, money, markets, prices, property, claims, law, councils, kingdoms, empires, war, religion, houses, villages, cities, roads, routes, rivers, tools, metals, fuels, engines, vehicles, medicine, families, dynasties

---

## 7. Speed: fields, groups and threads

Thousands of years of a living world will not run on one computer if every living thing is a single body with its own brain. The world is therefore made of fields on fixed cells, with groups where single bodies are too many. These are **speed tools, not new laws**. They must give the same patterns as the detailed form.

### 7.1 Fields and population nodes

- **Plants are a field.** Each cell holds a few slots of cover, each with amounts and genes (`docs/laws.md` 3). A plant is not a body.
- **Animals will be a field too**, built the same way (`review/roadmap.md` section 4).
- A body of count 1 is an individual in the maths: there is one set of rules for both.

### 7.2 Fixed cells

- The cells of a world are made once, with the world, and stay for its whole life: squares on land, and in the open low ground cells of 2 or 4 squares on a side (`docs/laws.md` 0). The air lives on a coarser level of blocks of 4 x 4 squares.
- The camera is not an input of the run, and zoom does not change the step or the cells. The same seed gives the same history.
- The ground below the cells is the fixed samples of 125 m, made once with the world (5.2). The page reads the cells and the samples; it adds detail per pixel and writes nothing back.

### 7.3 Time steps (multi-rate)

Not everything needs the same step. Each law steps at its own pace (`core/src/schedule.rs`, `docs/laws.md` 0 and 1.8):

| Process | Step |
|---|---|
| The air (heat, wind, rain, the sun's season) | 15 days, two a month |
| Liquid and erosion, the cover, Change, Bond, the bodies | 1 month |
| Heat sideways and creep | 1 year |
| The slow ground and the substances present, rechecked | every 3 months |
| The ground's long wear | once, when the world is made |
| Snapshots for the history tape | at the end of a run, every `--snapshots-every` years if asked (and before every god action) |

There is **one world clock**. The cells are fixed, so no step depends on the size of a cell or on where one looks. Seasons exist because the air steps twice a month and the sun moves, and they matter for food and migration.

### 7.4 Threads: chunks and the serial pass

This is a speed tool, not a law: the history is the same with and without threads, on any number of threads (the feature `threads` of `core`) and for any count of chunks.

- The cells are cut into **chunks**: `Config::chunks` runs of neighbouring cells, the same for any number of threads. A chunk owns its cells, their bodies, their cover and its view of the store.
- Bodies update once a month, in one **parallel pass**: one task per chunk on the pool, the world read only, a chunk writing only its own cells. What leaves a chunk (a move to another cell, a litter that starts in another cell, a watch, an act on a part) waits in the chunk's queue. A chunk updates its bodies cell by cell, all the stages of one cell before the next, so its queue, its events and the lineages and born rows it makes come in the order of the cells, whatever the chunks. Then one **serial pass** lands the queues in that order (by the cell each lands in, then the order queued), the moves and litters on the pool, each in the chunk of the cell it goes to; the acts on parts run there too.
- Flow's air step and Change's reactors and the survey's wear, merge and shade use the same pool where a loop is large enough to pay for a call into it (30 to 55 µs).
- How it works: `docs/laws.md` 0.
- The tape's recorder uses the same pool (`pcore::par::map`, `map_n` and `join`, under the one feature `threads` of `pcore`): its walks over bodies and cells, and the parts of a yearly row that write different columns, run side by side, and every sum is added in the order of the data, so its files are the same with and without threads.

---

## 8. Code architecture

### 8.1 Workspace layout

```
core/       the laws of the natural world, world state, random numbers. No IO, no Earth words.
  src/flow/ src/change/ src/cover/
              one folder per law (core/src/<law>/; the air in flow/air/)
  src/chem/   the chemistry: its data, its physics (phases, melting), the roles of reactions
  src/space/  the shape of the world: the mesh (cells, links, levels), the square map (an island, or a globe whose rows
              wrap east to west), the tree of cells under each air block, the terrain and its worn samples (wear,
              ponds, the network of channels, the makes of bed in strata, the shelf), stacks of layers
  src/inside/ the law inside a cell: how a cell's means lie over its own ground (read by the tape and the page)
  src/store/  the names of the nodes (a fixed node by class and index, among them the bed and the aside node of
              each cell; a part by slot and generation), what the fixed ones hold (`Store`: private arrays, read
              through methods), the boundary nodes sun, space and outside, the one way to change them (`apply`:
              moves and link fluxes; and `book_by_array` for the chunks' moves, the air's step and Change's rows,
              which adds as `apply` adds),
              and the moves of a chunk (`Moves`)
  src/clock/  time in whole seconds, the calendar, processes with periods, sleep and wake of nodes
  src/solve/  the shared maths: sweep (explicit step on links), line (implicit step along lines of links)
  src/life/   the cells' cover and the chunks of cells the cover's month runs on (the parallel pass), the seeds' landing
  src/cover/  the plant field: the starting cover of a cell, its growth, seeding and yearly budget
  src/*.rs    schedule (the month), world, params (Config, Physics), ground (heat and plateaus), check (the books),
              contract (what a taker may read and do to a cell), rng, event, math, units, par
  tests/      one file per law (flow, change, cover; bodies for the bodies of standing liquid), one per base part
              (space, store, clock, solve, inside), threads (threads and chunks change nothing), chemistry0, numbers
worldgen/   terrain, chemistry 0 (chem0.toml), first life (life/), what is
              placed when (schedule.rs), the planet's strip for an island's air (planet.rs); the test kit
              (worldgen::testkit, behind the `testkit` feature)
tape/       the recorder (rec/, its yearly columns in rec/cols/, the viewer file in rec/ewf.rs), snapshots
              (snapshot.rs), map layers (layers.rs), the cells' table (table.rs), the cover's stands as a viewer draws
              them (stands.rs), the things of a window (things.rs), the ground in tiles (ground/), measures, the
              fingerprint of a world; tests: tape.rs, ranges.rs, scale.rs, ground.rs, things.rs
library/    the recognition library (data files); today the Earth names of chemistry 0
cli/        headless runs (`ew run`, `ew resume`, `ew chem`, `ew numbers`, `ew view`, `ew cells`; flags in args.rs), pictures; tests:
              the word ban, sides, writer, view
wasm/       the core as WebAssembly for the live viewer: a small shell of `extern "C"` exports (`ew-wasm`)
web/        the 2D map viewer (TypeScript): plays a tape, or runs a live world in a worker; its shaders draw the map
```

### 8.2 Core rules (must)

1. **The step is a pure function:** `next_state = step(state, actions, seed)`. No hidden global state.
2. **Random numbers are keyed by place, time and purpose:** `rng(seed, tick, node_id, purpose)`, using a counter-based generator. This makes runs repeatable and lets parts of the world run in any order.
3. **Fixed iteration order.** Never iterate a hash map to decide anything.
4. **No IO, no clocks, no threads required in `core`.** Parallelism (`rayon`, 7.4) is optional behind the feature `threads`; the history is the same with and without it.
5. **The same `core` compiles to native (for batch experiments) and to WebAssembly (for the game).** Keep this working from the first day.
6. **Few dependencies.**

### 8.3 What the simulation records (the history tape)

- **Snapshots:** at the end of a run, every `--snapshots-every` years if asked, and before every god action, save the full state, compressed (bincode, then gzip). A run can go on from any snapshot (`ew resume`).
- **Raw physical events:** the first time a substance appears in the world. Only raw physical facts. **"A city was founded" is not a raw event.**
- **Summary series** every year (`series.csv`): totals per element, energy, temperatures, absorbing-gas amounts, the water, the cover and what it lost by cause, the cover's budget. Columns are named by index, never by Earth names (`elem_<j>`, `air_<k>`, `min<q>_*`).
- **Columns by starting kind** (`root_<k>_*`): `<k>` is the starting kind of cover a slot descends from (its row of `worldgen::life::KINDS`). Its slots, mass and capture area, and the means of its genes, weighted by its built cover. In a roster run, read the number of kinds alive from these columns.
- **The run's numbers** (`physics.json`): every field of `Physics` and `Config`. `ew numbers OLD NEW` prints the fields that differ between two runs, so no number changes without the reader seeing it.
- **Viewer frames** (`world.ewf.gz`): map layers every 10 years (`--frames-every`), with the series and the events.

The full list of columns is in `docs/tape.md`.

### 8.4 Runtime shells

- **Native CLI:** runs worlds headless, writes tapes, runs batches of many worlds, runs the world search.
- **Game:** the core runs as WebAssembly in a Web Worker, so the screen never freezes. The viewer receives state as transferable arrays.
- **Early prototype shortcut:** the native CLI writes tape files, and a simple web page loads and plays them. The live viewer (`wasm/`, a shell of plain `extern "C"` exports that runs the core in a Web Worker and posts the frames of the tape as transferable arrays) exists before M6; god mode (M6) adds the live interaction.

---

## 9. From simulation to things a player understands

```
SIMULATION (graph, 5 laws, no names)
   │ snapshots + raw events
   ▼
HISTORY TAPE
   ▼
PATTERN FINDER (normal code) ──► FACT CARDS
   ▼
MATCHER (normal code) + RECOGNITION LIBRARY ──► Earth concept + match level
   ▼                                      (leftovers)
VIEWER (Earth view + layers)      ◄── LLM TRANSLATOR (cached, read-only)
```

### 9.1 Pattern finder

It reads snapshots and events. It runs in the background, never inside the simulation step. What it detects:

| Detector | Method (suggested) | Example output |
|---|---|---|
| Repeated structures | a canonical hash of each structure's bond graph, with part materials grouped into bands (for example Weisfeiler–Lehman graph hashing); count copies, first seen, spread | "pattern #417: 41,200 copies" |
| How a structure is used | what flows through it, who is inside and when, whether it moves, what it carries | "bodies inside at cold times; heat loss −40%" |
| Kinds of living things | cluster genes into kinds; build family trees | "kind #12, descended from kind #3" |
| Groups and settlements | spatial clusters of bodies that persist over years | "1,800 bodies, stable 140 years, 300 structures" |
| Routes | edges with high, lasting traffic of bodies or substances | "path carrying substance 9 from cluster A to B" |
| Relations between kinds | lasting flows between kinds: who eats whom, who feeds or protects whom | "kind #12 gives food to kind #31; #31 traits shifting" |
| New substances | substances first produced by body actions, and what they are used for | "substance 22 first made year 3,410" |
| Cultures | clusters of brain settings and signal use among groups | "culture C4 spreading east" |
| Power patterns | lasting one-way flows of substances from many groups to one group, with taking or breaking when flows stop | "polity-like pattern, 14 groups" |
| Lineages of leaders | family lines of bodies that sit at the centre of such flows over generations | "dynasty-like line, 6 generations" |
| Sudden changes | big drops or rises, splits, abandonment, first occurrences | "cluster abandoned, year 5,120" |

**Fact card format** (JSON; one per pattern, with a stable ID):

```json
{
  "pattern_id": "struct-417",
  "category": "structure",
  "alive": false,
  "made_of": [{"substance": 12, "share": 0.8, "hardness": 0.7, "melt_c": 1350},
              {"substance": 4,  "share": 0.2}],
  "shape": {"parts": 3, "hollow": true, "open_top": true, "size_m": [0.3, 0.3, 0.4]},
  "use": {"holds_liquid": true, "carried_by_bodies": true, "often_heated": true},
  "count": 41200,
  "first_seen_year": 3410,
  "regions": 6,
  "replaced": "struct-88",
  "evidence_refs": ["snap:3420", "evt:..."]
}
```

### 9.2 Recognition library

A set of data files, one entry per Earth concept. **The only place where Earth concepts live,** together with the viewer and the LLM prompts.

Each entry has:
- `id`, `name`, `parent` (more general concept)
- `fingerprint`: tests over fact-card fields, with thresholds
- `levels`: which tests must pass for **same**, **similar**, **analogy**
- `physics_needed`: what the world must allow for this to be possible
- `visual`: which map colour or sprite to use, and how fact-card numbers choose its variant (size tier, material tint)
- `icon`
- `label_notes`: naming rules (see the naming policy in Section 18.1)

**Example entries (sketch):**

```toml
[[concept]]
id = "house"
parent = "shelter"
fingerprint = [
  "alive == false",
  "category == 'structure'",
  "enclosed_fraction >= 0.8",
  "regular_occupants >= 1",
  "occupied_share_cold_or_night >= 0.5",
  "occupant_heat_loss_ratio <= 0.7",
]
physics_needed = ["bond_v1", "containment"]
visual = { sprite = "house", variant = { tint = "made_of[0]", size = "shape.size_m" } }

[[concept]]
id = "herd"
parent = "animal_group"
fingerprint = ["category == 'group'", "same_kind == true", "members >= 10",
               "stay_together_years >= 1", "move_together == true", "uses_structures == false"]

[[concept]]
id = "domestication"            # an event, not a thing
parent = "inter_kind_relation"
fingerprint = [
  "relation.flows_both_ways == true",
  "relation.years >= 50",
  "partner_kind.trait_shift_generations >= 5",
  "trait_shift_only_in_linked_population == true",
]
label_notes = "Show as 'First domestication (similar to: <closest Earth animal or plant>)'."

[[concept]]
id = "car"
parent = "land_vehicle"
fingerprint = ["alive == false", "moves_on == 'land'", "carries_bodies_or_goods == true",
               "self_powered_from_stored_substance == true", "has_rotating_joints == true"]
physics_needed = ["bond_v2_rotation", "heat_to_motion"]
# If it is pulled by bodies instead of self-powered → 'cart'.

[[concept]]
id = "plane"
parent = "air_vehicle"
fingerprint = ["alive == false", "airborne_minutes >= 10", "carries_bodies == true"]
physics_needed = ["bond_v3_lift", "heat_to_motion"]

[[concept]]
id = "temple"
parent = "gathering_hall"
fingerprint = [
  "category == 'structure'",
  "build_effort_vs_local_house >= 10",
  "periodic_gatherings == true",
  "material_output_low == true",
  "signal_flow_high == true",
  "maintained_generations >= 3",
]
label_notes = "Show 'gathering hall' first; 'temple' only when every test passes. Never name a real religion."

[[concept]]
id = "penicillin"
parent = "antibiotic"
fingerprint = [
  "category == 'substance'",
  "made_by_living_kind == true",
  "source_kind_small_and_grows_on_food == true",
  "taken_by_sick_bodies == true",
  "reduces_harmful_kind_inside_bodies == true",
  "use_spreads_by_copying == true",
]
physics_needed = ["bodies_inside_bodies"]   # i.e. disease must be possible
label_notes = "Usually 'Antibiotic (≈ penicillin)'. Plain 'Penicillin' only on a very close match."
```

**Starter list (about 40 concepts) for the first version of the library.** Grow it as the world grows.

- **Landscape:** mountain, hill, valley, river, lake, sea, coast, island, desert, forest, grassland, glacier/ice sheet, swamp, volcano
- **Life:** plant cover, tree, herd, flock, pack, predator, grazer, food chain, extinction, crop field, orchard, domesticated animal, domestication (event)
- **Settlement:** camp, village, town, city, port, abandoned settlement
- **Structures:** shelter, house, wall, gathering hall/temple, store/granary, workshop, furnace/kiln, bridge, path/road, canal, mine
- **Things:** tool, blade, container/pot, raft, boat, sailing ship, cart, car, train-like, plane
- **Materials:** fuel, metal, bronze-like alloy, iron/steel-like, glass-like, medicine, antibiotic
- **Society:** trade route, exchange place/market, kingdom/polity, empire, war, dynasty, culture, language-like signalling
- **Events:** famine, epidemic, migration, discovery of a new land, collapse

### 9.3 Matching rules

- **The matcher is normal code.** It is fast, cheap and consistent: the same pattern always gets the same name.
- It walks **down the concept tree** only as far as the evidence allows. If it cannot decide between "car" and "cart", it shows "land vehicle".
- **Match levels:**

| Level | What the player sees |
|---|---|
| same | Earth sprite or map colour, and Earth name |
| similar | Earth sprite with a small "≈" badge, for example "≈ penicillin" |
| analogy | the parent concept's sprite, plus the local name |
| none | a generic block sprite coloured by its materials, in the same art style, plus a short description and a local name |

- **Names are stable.** Once a pattern has a name, keep it unless the evidence changes a lot. Record every change with the year.

### 9.4 The library is also a physics checklist

Each concept lists the physics it needs. So the library tells you what the world must allow:

- no lift → no plane-like things
- no rotating joints → no wheel-like things
- no bodies inside bodies → no disease → no medicine or antibiotic
- no high-temperature reduction in the chemistry → no metal-like materials

**When a wanted concept cannot appear, add the missing physics. Never add the concept.**

### 9.5 The LLM translator

- **Model:** a small, cheap model, for example Claude Haiku 5.5 (`claude-haiku-5-5`). A larger model may be used **offline** to draft library entries.
- **Jobs:**
  1. **Offline:** draft fingerprints for new library entries. People review them before use.
  2. **At runtime, rarely:** for patterns with no match, propose the closest concept (or "none"), a match level, a local name and a one-line description.
  3. **Stories:** short text for important events, and the "what happened since your action" summary (Section 11).
  4. **Local names** in a consistent style per culture.
- **Input:** fact cards (and for stories, a list of changes with evidence). **Never the raw graph.**
- **Output:** strict JSON, validated by code:
  ```json
  {"closest_concept": "house" | "none",
   "match_level": "same|similar|analogy|none",
   "evidence": ["enclosed_fraction=0.86", "occupied_share_cold_or_night=0.72"],
   "local_name": "Varrek hall",
   "description": "A closed shelter of hard stone where families stay in cold months."}
  ```
  Reject the answer if the concept is not in the library (or "none"), or if the evidence does not refer to real card fields.
- **"No Earth equivalent" must always be an allowed answer.** Otherwise everything will look like Rome or Egypt.
- **Cache** answers by pattern hash. Call the LLM once per new pattern and once per important event. That is thousands of calls per world, not millions.
- **One-way.** LLM output is never read by the simulation.
- **Sensitive themes** (war, epidemics, famine): follow Section 18.2.

---

## 10. Visualisation

### 10.1 Decision: a 2D map, not 3D

The owner decided: **no 3D.** The game is a **flat 2D plan view, like a good atlas map**, with:
- **colour-coded pixels** (or small tiles) for the land: mountains, lakes, sea, snow, forest, farmland, settlements
- **small axonometric sprites** (drawn at an angle, isometric-like) placed on the map for things: houses, walls, temples, boats, herds, carts

This is cheaper to build, faster to draw and easier to read. It fits a world seen mostly from far away.

The map stays a plan seen from above, but it shows matter, not symbols: water is a flat surface whose colour comes from
its depth, the shore goes from dry ground through wet soil and mud to shallow water by the height over the liquid and
the soil's water, rivers are channels of their true width and depth cut in the ground (hydraulic geometry, Leopold and
Maddock 1953), small channels follow the water on the drawn ground, the ground shows its materials (rock and stones on
steep slopes, soil, sand and gravel on shores, the load the water laid as flats of rippled sand and silt, mud and
gravel where the water reaches it, snow by slope), and plants are single plants placed by the plant cover
and the inside-a-cell shares (5.2), each kind by the wetness its genes like, their crowns with form and shadow. All of
it comes from the world's numbers and the ground's shape, seeded by place, so it holds while the camera moves.

Life and objects are drawn at their true size in metres and the plant cover as textures; the shapes of individuals
are drawn angled (10.4), each standing up on its foot (the owner's decision). True
size is the owner's decision: far away no herd is drawn; one or two zoom steps before single animals show, a herd shows
as a light, subtle blob; the zoom goes deep enough to see single animals.

**Goals:**
- A player understands what they see **instinctively**, without reading.
- What is drawn is **what is really there**. The Earth view only gives it a familiar face.

### 10.2 The view and its layers

- **Earth view:** map colours and sprites for familiar concepts (forest, farmland, village, house, boat…), with a "≈" badge where the match is only similar.
- **Layers:** one number of the model at a time (a temperature, the rain, the soil's grain…) drawn as a colour ramp over the Earth view, turned on and off.

### 10.3 The base map (pixels)

The base map is drawn per pixel, on the graphics card, from the tiles of the ground and the cells' table (10.1). The colour of a pixel comes from a **map class**, chosen by the **viewer**, never by the core:

| Map class | Comes from | Colour idea |
|---|---|---|
| deep sea / shallow sea | liquid depth, connected to the large liquid body | dark blue / light blue |
| lake | liquid in a closed low area | mid blue |
| river | cell that carries much more liquid than its own area sheds (`tape::is_river`) | bright blue line |
| snow / ice | frozen liquid covering the ground | white |
| bare rock / mountain | high, steep, little plant cover | grey, with shading |
| desert | dry, little plant cover | sand |
| grassland | low plant cover | light green |
| forest | high plant cover of tall bonded structures | dark green |
| farmland | the pattern finder found tended plant kinds here (library concept "crop field") | yellow-green stripes |
| settlement | the pattern finder found a settlement pattern here | warm brown, darker when denser |

- **Hill shading** from elevation makes mountains and valleys readable at a glance.
- Colours blend with the underlying numbers (thicker forest is darker, deeper sea is darker), so the map is never flat blocks.
- Simple thresholds on physical fields decide the natural classes. Farmland and settlement come from the pattern finder and the recognition library, so they are Earth-view interpretations and get the same match badges.

### 10.4 Sprites (axonometric)

- Small sprites drawn at a fixed angle, in one consistent style (for example clean pixel art).
- Each library concept has a **few variants**: size tiers (small, medium, large) and **material tints** from the world (a hard mineral substance gives a stone look, a plant fibre gives thatch, colours from reflectance). So each culture's settlements look different.
- Draw sprites sorted by their map position (back to front) so they overlap correctly.
- **Unmatched things** use a generic sprite built from coloured blocks for their parts, in the same style.
- **True-view avatars from genes:** close up, each body, plant and object is drawn from its own numbers (an animal's size from its mass, bulk and legs from its strength, coat from its heat gene, fangs if it bites; a plant's form from its walls gene; an object's block from its sizes and walls), with a little randomness seeded by its id, so a herd's members differ but read as one kind.
- Sprite art can be hand-drawn or generated, but must keep one palette and one angle.

Examples: house, hut, wall, gathering hall/temple, store, workshop, furnace, bridge, mine, boat, sailing ship, cart, car-like, plane-like, herd animals, flock, pack, people.

### 10.5 Zoom levels

| Level | What is shown |
|---|---|
| World | one pixel per cell or less: terrain colours, ice, sea; colour areas for cultures and polities; city icons; thick lines for the biggest routes |
| Region | bigger tiles with hill shading; settlement sprites in clusters, paths and roads, rivers, farmland, herds moving, boats on sea routes |
| Close | single building sprites, people walking, carts and boats moving, smoke and fire |

Sprites appear when tiles get big enough. Populations far away are drawn as density and colour. Individual sprites are drawn only when zoomed in.

### 10.6 Layers (toggle buttons)

terrain, temperature, rain, plant cover, animals, people, cultures, polities/borders, routes and flows, buildings, energy use, materials, heat-absorbing gas.

- **Culture colours:** turn each group's culture numbers (brain settings and signal use) into a colour, so similar cultures get similar colours. Spread, mixing and splitting become visible like paint.
- **Flows:** animated lines or moving dots. Thickness shows amount. Colour shows what moves.

### 10.7 Time

- Speed controls: pause, ×1 up to very fast.
- A **timeline** over thousands of years, with small charts above it: world population, temperature, heat-absorbing gas, biggest settlement, number of kinds.
- **Event markers** on the timeline in plain words, for example "Year 3,410: first metal tools (≈ bronze)". Click one to go there.

### 10.8 Inspect panel

Click anything to see:
- Earth name with match badge, and the local name
- what it is made of
- its history (when it appeared, how it spread, what it replaced)
- **"Why this name"**: the fingerprint tests that passed
- **"Show evidence"**: links to the fact card and events

### 10.9 Technology (suggested)

- The map is drawn on the graphics card with **WebGL2** (every browser has it, a phone's too), its shaders written in
  WGSL and translated to GLSL at build time, from one description of what is drawn. The worker hands over fields (height, water, wetness, materials, plants); the card draws them at the
  screen's own resolution, with light on the slopes and a texture for each material mixed by its share.
- Budget: about 50,000 visible sprites. Above that, show density instead.

---

## 11. God mode

### 11.1 Actions are physics edits

The player picks familiar actions. Each one is a change to the graph. Disasters are never scripted.

| Player action | What really changes |
|---|---|
| Set on fire | add a lot of heat to nodes in an area; burning comes from the chemistry |
| Move a river | lower the ground along a new path and raise it on the old path; liquid finds the new path |
| Dry up a river | reduce rain upstream, or remove the liquid source |
| Flood | add liquid |
| Drought | reduce rain over an area for some years |
| Earthquake | break many bonds in an area; shift ground heights |
| Volcano or meteor | add heat, rock and gas at one spot |
| Plague | add a harmful small living kind into bodies in an area (only if the world has disease) |
| Change climate | change sun strength, or add or remove heat-absorbing gas |
| Add or remove life | add or remove bodies of a kind |
| Raise or lower land | terrain brush |

If the world cannot do something (for example nothing burns, or there is no disease), **grey the action out** and say why.

### 11.2 One timeline

- The action is applied at the next step boundary. It is saved in the history tape as a god-action event.
- **Auto-save before every action.** This gives a simple undo. It is a save file, not a second timeline.
- To see "200 years later", the player **fast-forwards** and watches.
- **"Then and now" view:** compare today's map with the snapshot from the year of the action. Highlight what changed: people, cultures, routes, forests, buildings.
- **"What happened since":** the pattern finder lists the main changes since the action, and the LLM writes a short story from them, with links to evidence. For example: "After the river dried up, the valley farms failed within 30 years. Most families moved north and mixed with the hill people."

**Speed target for god mode:** 200 years of a region should fast-forward in about one to two minutes. Measure this in M6.

---

## 12. Engineering details

### 12.1 Numbers

- Use `f64` for every number that holds matter or energy, and book each move on both sides with one `f64` number: the world's amounts span 27 decades and the books must close to about 1e-12. Use `f32` only for numbers that hold nothing (cards, brain rows, faces, rates), and only where a measure shows a gain.
- Clamp tiny negative amounts from rounding to zero, and count how much you clamp. If the clamped total grows, there is a bug.

### 12.2 Conservation checks

Every N steps, check:
- total of each element (all nodes, all edge buffers, all bodies) is unchanged, apart from god actions
- total energy changes only by sun in, radiation out and god actions

Sun, space and outside are three boundary nodes: what crosses the world's boundary is booked on them, so the world and
these nodes together keep every element and the energy, and one check (`check::drift`) covers every law.

Log any drift. A drift above a small tolerance is a bug and stops the run in debug builds.

### 12.3 Tests (keep few, fast and meaningful)

What the tests must cover:

- **Conservation** (as above) on a small world for 100 years.
- **No NaN, no infinity, no negative amounts.**
- **Repeatability:** run the same seed twice on the same machine, and the state must be the same, bit for bit, with any number of threads (12.5).
- **Snapshot round-trip:** save, load, continue. The result must match an unbroken run.
- **One tiny scene per law** showing basic physical behaviour: liquid flows downhill; heat spreads; a reaction runs faster when hot; the cover grows by what its budget books and dies in frost; what a taker takes the cover loses.
- **Speed benchmark:** years per second on the reference island. Track it over time.

How the suite is built:

- **The normal suite** (`cargo test --release --workspace`) stays under about 30 seconds on 4 cores. Its tests are in one file per law: `core/tests/{flow, change, cover}.rs` (with `bodies.rs` for the bodies of standing liquid), one per part of the base (`core/tests/{space, store, clock, solve, inside}.rs`), `core/tests/threads.rs` (the number of threads and the count of chunks change nothing), with `chemistry0.rs` (chemistry 0's facts) and `numbers.rs` (every field has a row in `docs/numbers.md`); `worldgen/tests/` (the start, the roster, the fixers, the names of chemistry 0); `tape/tests/tape.rs` (snapshots, the viewer file, the series, resuming), `tape/tests/ground.rs` (the ground's tiles) and `tape/tests/things.rs` (the slots of cover of a window); `tape/tests/ranges.rs`, whose test `the_world_lives_and_keeps_its_books` is the main guard (a roster island runs past its last placement, keeps every element and the energy, has no number that is not finite, and ends with each of its cover kinds alive); `cli/tests/word_ban.rs` (14.4), `cli/tests/sides.rs` (the natural world names no module of a taker), `cli/tests/writer.rs` (a run resumed into its folder leaves the files of the unbroken run) and `cli/tests/view.rs`.
- **The contract between the layers** needs no test: the store's arrays are private
  fields of `store::Store`, which only `store::apply` and `store::book_by_array` (for the chunks' moves and the air's step, adding as `apply`
  adds) write, both inside the module `store`, so the compiler checks it; the sums of the boundary nodes and the
  potential are added by `Store::add_far_heat`. A test or an edit changes the
  store by moves from the outside node (`World::cross`).
- **The test kit.** The tests share one kit: `worldgen::testkit` (a small world, edits booked from the outside node, the rock of a cell, the books of a world, chemistry 0's substances by number, a plain taker on the contract). It is behind the `testkit` feature, which only the dev-dependencies of `core` and `tape` turn on, so it is never in a run.
- **Slow tests** are marked `#[ignore]`: long runs that keep their books, the island's erosion, coarse against fine (`tape/tests/scale.rs`), and the key-number tests of climate, land and life (`tape/tests/ranges.rs`). Run them before the last push of a phase (`cargo test --release --workspace --no-fail-fast -- --ignored`); there is no CI on GitHub for now.

That is the core of testing, until something emerges. Reviews of the code and of the laws happen in rounds, by the agent and the owner together. What stays banned is proof machinery: receipts, proof campaigns, cross-machine determinism, replay proofs.

### 12.4 Watch reports

Each milestone ends with a short report for the owner, in simple English, with its pictures:
- what we built (a few lines)
- **pictures or a short video**
- key numbers (speed, population, how many kinds, how many patterns)
- **what emerged**, and what did not
- what we will change next, and why

### 12.5 Repeatability (cheap kind only)

- Same seed, same code version and same machine/build give the same history.
- Neither the number of threads nor the count of chunks changes the history: every order (of sums, of landings, of writes, of new ids) is fixed by the data, never by which thread is first or where a chunk ends. `core/tests/threads.rs` checks it on a world crowded enough that two chunks land in one cell. (The tape's yearly sums of counters, added chunk by chunk, can differ in their last digits; the world does not.)
- Purpose: finding bugs, and replaying a run.
- **Do not** build identical results across machines, CPU/GPU agreement to the last bit, receipts or replay proofs.

### 12.6 Speed

There is **no speed target for now**. Build the most true, most interesting and best-made system; cut back when it has become too slow. Each merge writes down its cost per year (native, ms a year at sizes 32, 64 and 128), so that the day of cutting back has numbers to cut by. **Never remove a law to get speed**: use the speed tools of Section 7, profile, or move field laws to the GPU.

---

## 13. Order of work

The order of work is in `review/roadmap.md` section 7: two tracks side by side, **nature** and **life**; then the small game; people after the animal layer. The old milestones M0 to M8 are not used.

Each stage ends with "watch it and see": pictures at the owner's views and a few key numbers in range. A stage has a **watch-for** list: what we hope to see. If it does not appear, the next step is to change physics, perception, memory or learning, and **never** to add a named rule.

---

## 14. Rules of work

### 14.1 Do

- Build the smallest watchable thing first.
- Look at the world often. Make videos.
- Keep each law in one folder (`core/src/<law>/`).
- Write a watch report at the end of each milestone.
- Ask the owner when a choice changes what the game is.

### 14.2 Adding a sixth law (rare)

All of these must be true:
1. A milestone's watch-for failed, and you can show why the five laws cannot produce it.
2. The new law is **one equation** on nodes or edges.
3. It has **at least three unrelated uses** (for example rivers, trails and blood flow). If it only explains one human thing, it is a named system in disguise.
4. It is under about 300 lines.
5. The owner agrees.

### 14.3 Size and tuned numbers

**Size.** Keep `core` small by computing less, never by cramming lines (`CLAUDE.md` 2.11). There is no line count to pass.

**Budget for tuned numbers.** `docs/numbers.md` has one row for every field of `Physics` and `Config` (and for the free constants outside them), with its unit, the fact it is, the real size it is anchored to and a mark: physical, anchored or tuned. A tuned number is one set so a run came out a certain way (a rule in a smaller coat, Section 2, lesson 3), or one with no real size behind it. A test (`core/tests/numbers.rs`) fails when a field has no row. **The rule: the count of tuned rows and of the rules chosen for a run (the second table, things picked for a run that are not numbers, such as the hand-set eater start and the fixing founders) may not grow.** A new number must be physical or anchored, or it replaces a tuned one.

### 14.4 Earth-word ban in `core` and `worldgen`

Add an automatic check (a script in CI or a test) that fails if these words appear in identifiers, comments or strings in `core/` or `worldgen/` (case-insensitive, whole words). Extend the list as needed.

> farm, crop, harvest, sow, herd, livestock, domestic, tame, pasture, house, home, hut, village, town, city, settlement, road, route, trail, river, lake, ocean, sea, market, trade, money, price, coin, wage, debt, loan, property, owner, king, queen, lord, council, kingdom, empire, nation, war, army, weapon, church, temple, religion, god, tool, axe, plough, wheel, cart, car, boat, ship, plane, engine, metal, iron, copper, bronze, steel, gold, wood, stone, coal, oil, fire, food, grain, meat, water, medicine, disease, tribe, family, household, culture

Allowed elsewhere: `library/`, `web/`, documentation and LLM prompts.

Note: words like `liquid`, `gas`, `solid`, `energy`, `heat`, `body`, `bond`, `flow`, `node`, `edge`, `kind` are fine. They describe physics or structure, not Earth history.

**Do not ban these words.** They are normal programming or physics words, and this document itself uses them in the core: `state` (as in `next_state = step(state, …)`), `field` (as in "field laws", a grid of numbers), `path` (a path through the graph, or a file path). If a banned word is also a common code word, prefer a different word in the ban list (for example `kingdom` instead of `state`).

### 14.5 Do not

- Do not write a rule, action or system named after an Earth outcome.
- Do not hand-shape rewards for specific behaviours.
- Do not add realism (field data, calibrated species, real-world parameters) unless it is needed for something to emerge.
- Do not build proof machinery: cross-machine determinism, receipts, replay proofs, proof campaigns. (Reviews of the code and the laws in rounds, by the agent and the owner together, are part of the work. Orders fixed by the data, so that threads do not change the world on one machine (12.5), are not proof machinery: they are the same-machine repeatability that 12.3 asks for.)
- Do not build branching timelines.
- Do not let the LLM or the library influence the simulation.
- Do not try to find or rebuild version 1.
- Do not run `cargo clean` or delete build folders as routine cleanup.

---

## 15. Risks and what to do

| Risk | Sign | What to do |
|---|---|---|
| Emergence stalls at "animals and trails" | A watch-for of a stage fails for a long time | Improve Choose: planning, memory, imitation. Check that the physics allows the wanted thing (Section 9.4). |
| Long-term behaviours (taming, farming) never appear | No lasting flows between kinds | Planning and longer memory; check that tended kinds really change under Copy; check that seasons give a reason to store |
| Bond physics too slow | Profile shows load computation dominates | Simpler load approximation; update loads only when a structure changes |
| Brains too slow | Bodies dominate the step time | Shared brain bases per lineage; no brain where nothing is decided (light-eaters); population nodes; fewer updates; GPU later |
| Long chains of actions never learned (for example heat rock with fuel, wait, get a new solid) | Bodies find food and shelter but never make new substances | This is the main research risk. Planning (look-ahead), curiosity-like exploration in the brain, longer memory, imitation. Use the watch of each stage (Section 13). Do not add a named recipe |
| Pattern finder too slow | Analysis falls far behind the simulation | Analyse fewer snapshots; analyse changed chunks only; run in the background |
| Art cost explodes | Many concepts without sprites | Few variants per concept, material tints instead of new drawings; generic block sprites as fallback |
| Earth view misleads players | Players think the world copies Earth | Keep "≈" badges, "why this name", and the true-view switch |
| Team adds named rules "just this once" | A named concept appears in `core` | The word-ban check; review against Section 3 |
| Results are strange, not Earth-like | Nothing matches the library | That is allowed. Use local names and the layers. Grow the library only from what appears. |

---

## 16. Glossary

| Word | Meaning here |
|---|---|
| **emergence** | a pattern that appears from the laws without being written in the code |
| **node** | anything that holds stuff: ground cell, air column, body, part, object, population, region |
| **edge** | a link between two nodes that lets stuff pass (flow) or holds them together (bond) |
| **conductance** | how easily stuff passes along an edge |
| **push (Φ)** | what drives flow: pressure, height of liquid, concentration, temperature |
| **dissipation** | energy lost by moving stuff; it does work where it is lost (carrying, wearing) |
| **substance** | an unnamed material, made of base elements, with physical properties |
| **catalysis** | a body's genes speeding up chosen reactions (metabolism) |
| **bond** | a join between two nodes, with strength and stiffness |
| **structure** | nodes held together by bonds, moving as one |
| **body** | a structure with genes and a brain that can act |
| **kind** | a cluster of bodies with similar genes (species-like) |
| **population node** | many similar bodies merged into one node, for speed |
| **history tape** | snapshots, raw events and god actions saved over time |
| **pattern finder** | code that detects repeated structures, groups, routes and events in the tape |
| **fact card** | a short JSON summary of one detected pattern |
| **recognition library** | data files describing Earth concepts as fingerprints |
| **match level** | same / similar / analogy / none |
| **Earth view / layer** | familiar map colours, sprites and names / one number of the model as a colour ramp over it |
| **axonometric sprite** | a small picture drawn at a fixed angle, placed on the flat map |
| **god action** | a player's physics edit to the world |

---

## 17. References (ideas, not rules)

- **Sugarscape** — J. Epstein and R. Axtell, *Growing Artificial Societies* (1996). Trade, migration, inequality and conflict from a few local rules.
- **Turchin et al.**, "War, space, and the evolution of Old World complex societies", *PNAS* (2013). Large polities emerge from simple rules on a map.
- **W. B. Arthur and W. Polak**, "The evolution of technology within a simple computer model", *Complexity* (2006). Complex technology built by combining simpler parts.
- **Kiyotaki and Wright** (1989), and **Marimon, McGrattan and Sargent** (1990). Money-like goods emerging among learning traders.
- **L. Yaeger, Polyworld** (1994). Evolving neural-network creatures in a simple world.
- **Baker et al. (OpenAI)**, "Emergent Tool Use From Multi-Agent Autocurricula" (2019). Agents with only move/grab/lock invented forts and other tool use.
- **Tero et al.**, "Rules for biologically inspired adaptive network design", *Science* (2010). Slime-mould network reinforcement reproduces efficient transport networks (the "used paths get stronger" idea).
- **H. Paynter, bond graphs** (from 1959). One kind of maths for flows of water, heat, electricity and motion.
- **Kumar et al. (Sakana AI)**, "Automating the Search for Artificial Life with Foundation Models" (2024). Searching parameter spaces for interesting artificial life.
- **B. Chan, Lenia** (2019). Rich life-like forms from simple continuous rules.
- **Shervashidze et al.**, Weisfeiler–Lehman graph kernels (2011). Basis for hashing repeated structures in the pattern finder.

---

## 18. Ideas carried over from version 1 (written out in full)

You do not have access to version 1. Everything worth keeping from it is written here. **Do not try to find or rebuild version 1.**

### 18.1 Naming policy (use it for the recognition library and the LLM prompts)

**Three separate jobs.** Keep them apart:

| Job | Example |
|---|---|
| The simulation defines what is possible | a material conducts heat well and melts at 1,080° |
| Interpretation recognises a concept | "copper-like metal"; "conductor"; "traded good" |
| Historical analysis explains this case | "exports of this metal paid for food and drew migrants" |

Defining a material does not decide its discovery, use, trade or value. A reddish conducting material is **not** automatically "copper".

**Three ways to name a thing.** Show all three where useful:

| Perspective | Purpose | Example |
|---|---|---|
| World description | say what the thing is, in plain physical terms | "soft reddish metal from hill rock" |
| Local name | what the simulated people call it (made up, consistent per culture) | "redstone" |
| Earth comparison | link to a familiar pattern | "≈ copper"; "mining boom town" |

**Name rules:**
- Names attach to **stable pattern IDs**. They may change over time; keep the history of which group used which name, and when.
- Concepts can split, merge and gain new meanings. Never rewrite earlier history when that happens.
- For real **materials and working technologies**, familiar Earth categories can often be used directly ("blade", "boat", "furnace").
- For world-specific **religions, nations, movements and events**, prefer **original names** with a concrete description. Never use the name of a real religion, nation or person.
- The vocabulary is a **graph, not a ladder**: one thing can be a medicine, a traded good and a forbidden substance at the same time.
- When no concept fits, **describe the observed features**. Do not force a match.

**How much evidence a label needs:**

| Label | Evidence needed |
|---|---|
| Medicine | used with the intent to help the sick; whether it really works is tracked separately |
| Antibiotic | the mechanism (it reduces a harmful small living kind inside bodies) is really there |
| Penicillin | a very close match (made by a small mould-like kind growing on food); a generic mould-made remedy is only "≈ penicillin" |
| Projectile weapon | the thing really throws or shoots objects |
| Firearm | the operating mechanism (a fast burning substance pushes the projectile in a tube) is really there |
| Breech-loading rifle | the defining loading arrangement and the grooved barrel are both really there; "it works better" alone is not enough |
| Council | a lasting pattern of the same members, a coordinator, contributions and monitoring is recorded; one shared work party is not enough |
| Monotheistic religion | the recorded beliefs and practices support the description |
| Any real-world religion or nation by name | never; use an original name |

### 18.2 Presenting hard themes

War, epidemics, addiction, hunger and displacement may emerge. Present them:
- with neutral descriptions, never celebration, never casualty scores, never graphic detail
- separating what the simulated people **claim** from what the **evidence** shows
- without turning cultures or groups into "naturally better or worse" rankings
- with a player setting to hide distressing story detail; this setting changes only the presentation, never the simulation

### 18.3 Watch-for list (things that MAY emerge)

A list of interesting patterns to look for in long runs. **Each one is something that may emerge, never something to build.** For each, an equally valid different outcome is listed. A world that only ever produces the familiar story is a warning sign that something was tuned too much.

**Null run:** neutral brains draw every action by chance, so many "yes" verdicts (gifts, watching reaching many lineages, busy crossing cells) are what chance predicts. Run the same seed with `ew run --null` (`Config::null`: learning off, a watch is drawn and counted but moves no rows, born rows and brain bases never mutate or split; genes still mutate and evolve). The config line printed at the start and the series column `null` say which run it is. An item counts as emerged only where the real run beats the null run clearly, on the same columns (`gave`, `watched`, `watched_bodies`...). The column `learned` is zero in a null run.

| Might emerge | Equally valid other outcome |
|---|---|
| Plants changed by tending (early farming) | tending is abandoned; selection favours storage or taste instead of yield |
| Predator and prey cycles | stable coexistence, local extinction, or recolonisation |
| Animals adapting to people's land use | animals avoid people; corridors keep populations alive |
| Animal domestication | cooperation without much change in the animals |
| Shared upkeep of a water channel | each family keeps its own; or free riders end it |
| Salt building up in watered land, then food crisis | flushing, drainage, crop change or trade prevent it |
| Over-use of a shared resource | local agreements keep it alive |
| Raiding and property customs | watching, compensation and respected claims; or hidden and guarded stores |
| Nutrient run-off and collapse of fish | the system recovers |
| Forest, rain and fire changes | regrowth or land-use change |
| Epidemic spreading along trade routes | people adapt and contain it while keeping trade |
| Crop monoculture crisis | crop variety absorbs the shock |
| Mining boom and abandonment | skills and infrastructure start a new activity |
| Wealth concentrating, then revolt | reform, mobility, sharing, or scattered dissent |
| Chains of unpaid debts | reserves and renegotiation contain them |
| Knowledge lost when specialists die, then found again | teaching and records preserve it |
| Cultures blending (for example in music-like signals) | separate traditions persist despite contact |
| New practices spreading | messages spread but nobody adopts; rare skills survive behind barriers |
| Fast long-distance signalling linking far markets | uneven access makes connected hubs dominant |
| A new weapon shifting power | supply or coordination outweighs better equipment |
| Long military stalemate | negotiation or supply failure ends it |
| Different city forms (dense, spread out, walled, ports) | infill, edge growth or abandonment, following real limits |
| Industrial burning changing the climate | substitution and coordination limit the damage |
| Recovery after a collapse | broken links delay recovery, or a very different society appears |

### 18.4 An example of a long chain of causes

This is the kind of story we hope to see. Every step must come from the five laws, with nothing scripted:

```
land and rain → a few families dig a channel to water their plants
  → bigger harvests → more families move near the channel
  → more wood is cut for building and storage → forest shrinks
  → more run-off, salt builds up in the watered land → harvests fall
  → the channel wears down, fewer people help repair it
  → food deliveries fail, trust between families drops
  → some families leave; others copy a drainage trick from neighbours
  → new trade links and a different settlement pattern
```

At every step, other outcomes were possible.

---

## 19. Open items

- **A law for mud and sand (sediment) in a running world.** The made ground already has a history and a shelf under the
  shore (`docs/laws.md` 0). What is open is the law for a running world: liquid that runs fast takes up ground; where it
  slows, it lays it down (Exner 1925; transport after Meyer-Peter and Müller 1948), so deltas, mud flats, beaches and
  filled valleys form by themselves. It is a change of the laws (Flow on the ground), with its own books and long runs
  (`review/roadmap.md` section 12).
