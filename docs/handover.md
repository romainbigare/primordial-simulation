# Handover: Emergent World, version 2

Revised: 17 October 2026, after the M3 clean-up.

**For:** an AI agent (or engineer) who works on version 2.
**From:** a design conversation between the project owner and Claude, after a review of version 1, an earlier attempt with the same goal. You do not have version 1. Everything useful from it is written in this document.

Read this whole document before you write any code. Sections 1 to 4 explain *why*. Sections 5 to 12 explain *what* to build. Sections 13 to 17 explain *how* to work. Section 18 holds the ideas kept from version 1.

The other documents:

- `docs/laws.md`: how each law works **now**, part by part, with the code that does it.
- `docs/numbers.md`: every free number, its unit, its source and whether it is physical, anchored or tuned.
- `docs/tape.md`: what a run records.
- `docs/history.md`: what changed in M0 to M3, what was tried and not kept, and where old files are in git.
- `docs/reports/`: the watch reports (Section 12.4).
- `docs/foundation/plan.md`: the foundation work before M4 (the owner's choice at the M3 check point, 13.1). Its decisions are in its Section 3. Its first rule: the foundation replaces the special code, it is not one more patch. Where it and this document differ, this document wins until it is updated.

---

## 0. How to use this document

- This is the single source of truth for the plan and the rules of work. If anything else conflicts with it, **this document wins**. For how a law works now, read `docs/laws.md`; there the code is the truth.
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

1. **Watch.** A 2D map with small angled sprites, from the whole planet down to one street. Speed controls. A timeline over thousands of years.
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
9. **Small code.** Hard budgets in Section 14.3.
10. **Speed matters.** Thousands of years must run in minutes. Rough far away, detailed where it matters (Section 7).

---

## 4. Decisions already made

| Topic | Decision |
|---|---|
| Core idea | 5 laws on one graph: Flow, Change, Bond, Copy, Choose |
| Coordination, trade, property, institutions | **Not** primitives. They must emerge from Choose + Bond + Flow + Copy (+ signals) |
| Actors | Individual bodies where it matters. Groups of similar bodies merged into population nodes elsewhere (Section 7) |
| Body scale | Bodies are **real-sized**: an individual weighs what a real living thing weighs (grams to tonnes, set by its genes and selection). Plant cover holds real mass per m² (about 1 to 50 kg/m²). **No hard-coded limit on bodies per cell**: density is an outcome of light, cover, crowding and food. Speed comes from population nodes (Section 7.1) and better algorithms, never from giant bodies |
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
| Soil | The ground holds liquid in pores. Two minerals (nitrogen and the rock element) come from plain chemistry, so fertility differs by place |
| Nitrogen fixers | A tenth of the starting light-eaters carry the gene that fixes the air's nitrogen: an initial condition from real floras (a rule chosen for a run, Section 14.3) |
| Carbon | The sea holds carbon by plain chemistry and starts at balance with the air |
| Law 2 | One rate law (mass action with the contact of the phases) in the open world and in bodies; a catalyst is slowed by its own product |
| Moving | Moving needs force |
| Materials | Bodies are built of several materials; physics reads the material (height, breaking, digestion) |
| Freezing | One freezing law for bodies, ground, lakes and the sea |
| The start | What is placed at the start is an initial condition and every run prints it. Animals start with hand-set brain biases, plants with no brain (`HIDDEN` 0: their life is their body's physics), founders as adults |
| Waits | Natural fire starts wait for M4. Disease (bodies inside bodies) waits until after M4 |
| World chemistry | One fixed Earth-like "chemistry 0" through M6 (numbers only, no names in core). Random chemistry per world from M7 (Section 6.0) |
| Earth concepts | Only in the recognition library, viewer and analysis code. Never in the core |
| Visuals | **2D map, no 3D**: colour-coded pixels for land plus small axonometric sprites. Earth view (default) with a switch to true view |
| Translation | Pattern finder (normal code) → fact cards → matcher (normal code) → small LLM for leftovers and stories (cached, read-only) |
| God mode | Physics edits on one timeline, auto-save before each action, "then and now" view |
| Timelines | One only |
| Repeatability | Cheap kind only |
| Language | Rust for the core (compiles to native and to WebAssembly), TypeScript for the viewer |
| Version 1 | Not available and not reused. The useful ideas are written out in Section 18 |

How each of these works in the code: `docs/laws.md`. When and why each came: `docs/history.md`.

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

### 5.2 Graph inside a graph (hierarchy)

```
planet → regions → cells → groups / bodies → parts
```

- A node can **collapse**: its children are replaced by one node with the summed contents and combined conductances.
- A node can **expand**: the children are created again (or generated to match the totals).
- Collapse where nothing interesting happens. Expand where something changes fast, or where the player looks closely.
- **Conservation must hold exactly across collapse and expand.** The total of every element and the total energy must be the same before and after.

This is the main tool for speed. Do not build it first (see milestones). Build the detailed form first, then the collapsed form, and check that both give similar patterns.

### 5.3 Storage

- **Space** (land, sea, air) is a regular grid. Store it as plain arrays, one array per quantity ("struct of arrays"). The grid is still a graph: each cell's neighbours are its edges. The laws are the same, the storage is just faster.
- **Bodies, structures and long links** are a sparse graph: arrays of nodes and edges with stable integer IDs and a generation counter for reuse.
- Use chunks (for example 32×32 cells) so parts of the world can be saved, loaded, collapsed and expanded separately.

### 5.4 Suggested world sizes

- **Prototype island:** 128 × 128 cells, about 1 km per cell. Up to about 10,000–50,000 individual bodies.
- **Later, planet:** a latitude-longitude grid of squares that wraps east to west, with finer cells only where needed.
- **The size follows a time budget, not the other way round:** the planet has as many cells as fit in the time a run
  may take (a planet run of 1,000 years in under 15 minutes on the owner's machine). The cost of one cell and of one
  body node per simulated year, measured at each step of the work, gives that size (`docs/foundation/plan.md`, "The
  time budget" and 5.7). Faster code buys a bigger planet.

---

## 6. The five laws

Each law is one equation, or a small set of equations, applied everywhere. Each law lives in its own folder (`core/src/<law>/`) and stays small.

This section is the design of each law. How each one works now, in detail and with the code that does it, is in `docs/laws.md`.

Notation: `a[i,k]` is the amount of substance `k` in node `i`. `E[i]` is the heat energy of node `i`. `T[i]` is its temperature.

### 6.0 The world's chemistry (generated per world)

The world has its own chemistry. This is not a law. It is the **data** the laws read.

**Decision: start with one fixed, Earth-like chemistry. Random chemistry comes later (M7).**

A random chemistry can be dead or boring, and finding out can waste weeks. So for M0 to M6:

- Use **one hand-made chemistry, "chemistry 0"**, copied roughly from Earth. About 8 elements and about 30 substances. Only rough numbers (melting points, densities, reaction heats); never precise field data.
- It must contain at least: a common liquid that rains and flows; a gas mixture with one heat-absorbing gas; a light-storing reaction (plant-like energy); energy-rich substances that burn in the gas; hard solids freed from rock by heat plus fuel; and pairs of hard solids that mix into a harder one. That is, it must pass all the richness checks below.
- In `core` and `worldgen`, substances are **numbers only** (substance 0, 1, 2…). The data file holds numbers, not names. The Earth names of chemistry 0 ("≈ water", "≈ iron") live **only** in `library/`, as a normal recognition entry. The word ban still holds.
- Write the chemistry generator and the richness checks anyway (they are small), but run the world on chemistry 0 until M7. At M7, the world search generates random chemistries and chemistry 0 is just one of them.

**Elements.** A small number of base elements, for example `B = 6 to 8`. Elements are never created or destroyed.

**Substances.** About 16 to 48 substances. Each substance is a small vector of element counts (for example `[2,0,1,0,0,0]`). This makes conservation automatic: every reaction must balance element counts.

**Substance properties.** Each substance gets physical numbers. They are derived partly from its elements (so related substances have related properties, which makes discovery possible) plus random noise:

| Property | Used by |
|---|---|
| molar mass | weight, gravity, buoyancy |
| heat capacity | temperature from energy |
| melting point, boiling point | phase: solid / liquid / gas at a given temperature |
| density (per phase) | volume, buoyancy, layering |
| hardness and bond strength (solid) | Bond law, cutting, wear |
| Young's modulus and the stress it breaks at (solid as a material) | how high what a body is built of holds it, the energy to break it |
| reflectance in 3 visible bands | colour on screen, how much sunlight is reflected |
| absorption of outgoing heat radiation (gas), and the column amount past which it grows only as the square root of the amount (its bands fill up, a stand-in for the logarithm of real gases) | greenhouse effect |
| solubility in each liquid | dissolving, carrying |
| conductivity of heat | heat flow |

**Reactions.** Each reaction has inputs and outputs (balanced in elements), a speed `k`, an energy barrier `Ea`, and an energy change `ΔH` (gives heat or takes heat). Some reactions take **light** instead of heat as their energy source.

**Richness checks.** A random chemistry can be dead. Generate many chemistries and keep only those that pass physical checks like these. The checks state physical possibilities. They never place an outcome or name an Earth thing.

- At least one abundant substance is **liquid** across most surface temperatures (needed for rain, flow and erosion).
- There is an abundant **gas mixture**, and at least one gas **absorbs outgoing heat** (needed for climate and climate change).
- At least one reaction stores **light energy** into a solid or liquid (needed for life to get energy).
- Substances that store energy **release it** when they react with an abundant gas, above some temperature (needed for burning and breathing).
- Some solids in rocks **release a new, harder solid** when heated together with an energy-rich substance above a high temperature (needed for metal-like processing).
- Some pairs of hard solids **mix into a harder solid** (needed for alloy-like materials).
- The range of melting points and bond strengths is wide (needed for a variety of tools and structures).

**Chemistry 0 as built** (`worldgen/src/chem0.toml`: 9 elements, 26 substances, 28 reactions) also has what life needs: a **body matter** that bodies build from the energy-rich solid by a growth conversion (`Chemistry::build`), so the fuel a body stores and the tissue it is built of are different matter; two **minerals** that body matter needs (a nitrogen mineral and the rock element's salt, both living dissolved in the liquid); and two more **solids bodies can build** (a frame and lime, `Chemistry::builds`). A random chemistry has none of these yet (its bodies do not grow); a generator that makes them is for M7. The roles and reactions are listed in `docs/laws.md` (2.7).

Later, add a **world search** (Section 13, M7): generate hundreds of worlds and keep the ones where most new things happen.

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
- The sun gives energy to surface nodes by latitude, season and planet settings (sun strength, axial tilt, day length). The island's rows span only what its size spans on the planet (31° to 33.3°); real latitude bands come with the planet at M4.
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
- **Materials.** How high a body stands is what its materials hold up against its weight; the energy to break a body is what its materials resist (`docs/laws.md` 3.6).
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
  - **Two parents.** A mature body may find an alike mature partner in its cell; each gene of the children is then one parent's. With no partner, one parent.
- **Imitation.** A body that watches another body can copy part of its brain, its learned behaviour. This costs time and attention. It is how culture, skills and designs spread.
- **Matter is never copied for free.** Copying a body costs real substances (Law 1 and Law 2 still hold). Copying behaviour costs time.
- **Structures are not copied directly.** Bodies that learned to build will build again. Bodies that imitate them will build similar things. That is how "designs" spread.

**What genes hold:**
- body plan (size, which parts, their materials and the share of each)
- which reactions the body catalyses, and how strongly
- minimum safe amounts of key substances (below them, the body dies), and how much of each it keeps
- reproduction settings (size of maturity, share of surplus for children, size of a child)
- the starting state of the brain, its sizes, and how fast it learns and forgets
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

**Planning** (not built yet): bodies learn a small model of "what happens if I do X" and look a few steps ahead. This makes long plans much easier to find. Taming an animal, for example, pays off only after many seasons.

**Implementation:**
- A small neural network per body: a base per lineage, rows that are genes of each body, plus what the body (or its population node) learned. Keep the brain code apart from the rest, so it can be swapped.
- **One brain per body or population node.** Each body has its own brain rows; a population node's members share the node's.
- **A brain has a size and a cost.** Its sizes are genes, and zero is a size: a body with no hidden units takes no decisions and pays nothing for a brain. A brain is paid per decision, so a body that decides less often pays less (a speed incentive dressed as physics, kept on purpose).
- Simple learning is enough to start: a policy gradient against a learned baseline (actor-critic), plus evolution.

### 6.6 What is deliberately NOT a law

These must emerge. If one does not, improve physics, perception, memory or learning. **Do not add it as a law.**

> farming, herding, domestication, trade, money, markets, prices, property, claims, law, councils, kingdoms, empires, war, religion, houses, villages, cities, roads, routes, rivers, tools, metals, fuels, engines, vehicles, medicine, families, dynasties

---

## 7. Speed: groups and coarse regions

Thousands of years with millions of individual brains will not run on one computer. Two tools solve this. **Both are speed tools, not new laws.** They must give the same patterns as the detailed form.

### 7.1 Population nodes

- When many bodies of the same lineage sit in the same area, have similar states and act similarly, **merge** them into one population node.
- A population node holds: the count, average stores, a small summary of how they differ, and one shared brain.
- It runs the same five laws in average form: flows × count, reactions × count, children as a birth rate, choices as a spread of actions.
- **Split** it back into individuals when the player looks closely, or when its members start to differ a lot.
- Conservation must hold exactly when merging and splitting.

Population nodes are built (since M3): a node keeps its spread, joins and merges only alike bodies of one size and maturity class, splits when its halves would no longer be alike, and does the actions of Bond through one member split off. How it works: `docs/laws.md` 4.4. `ew run --individuals` turns nodes off for bodies with a brain, for the check below.

**The check of nodes against individuals (M4's first job, fixed before it runs).** Seeds 1 and 5 at 32×32 for 100 years, with nodes and with `--individuals`, then with `--roster`; one seed with `--crowd 4` and `--crowd 8` beside them as the noise. On the `root_<k>_*` columns (means of years 41–50 and 91–100), the modes agree if: (a) each kind's mass and real life are within a factor of two, and no kind lives in one mode only; (b) the main gene means move the same way wherever one moves more than 0.3; (c) the same kind wins (most mass, heaviest member, the order of what eaters eat). A difference larger than the `--crowd` noise is a bug in the node rules.

### 7.2 Coarse regions

- The planet is coarse everywhere, and finer where it matters: a region of finer cells is built into the mesh at the
  start, or rebuilt at a god action. The sea lives on a coarser level, as the air does, so its cells are cheap.
- Cells that split and join by themselves while the world runs come later, when a player must zoom anywhere.

### 7.3 Time steps (multi-rate)

Not everything needs the same step. These are suggested defaults to measure and adjust:

| Process | Default step |
|---|---|
| Heat, air, rain, water (coarse world) | 1 month, with sub-steps if unstable |
| Ground uplift, slow wearing | 10 to 100 years |
| Population nodes | 1 month |
| Individual bodies in detailed areas | 1 day |
| Snapshots for the history tape | every 10 years (and before every god action) |

As built: ground and air physics every 3 days (radiation at every step; evaporation, rain and wind every 3 days), liquid every 30, the reactor every physics step, bodies every 5 to 360 days (their `INTERVAL` gene), the loads of parts once a month and their reactions every 6 months. Each part of Flow keeps its own period. Heat, air and rain do not hold a longer step as written: each of radiation (the air's temperature), the exchange and the wind is limited by what a node holds in one call, so rain on land falls as the step grows, and the 3-day world is itself far from the world of short steps (rain on land 3 to 4 times less than at 6 hours; `docs/numbers.md`, `phys_days`). A longer step needs one implicit solve of the air's heat, vapour and wind together (`docs/foundation/plan.md`, J1).

There is **one world clock**. Fast processes in detailed areas sub-step inside a month. Seasons must exist, because they matter for food and migration.

---

## 8. Code architecture

### 8.1 Workspace layout

```
core/       the five laws, world state, random numbers. No IO, no Earth words.
  src/flow/ src/change/ src/bond/ src/copy/ src/choose/
              one folder per law (core/src/<law>/; the air in flow/air/)
  src/body/   bodies: genes, tissue, metabolism, exchange, the update, its physics, paying, death, intake
  src/chem/   the chemistry: its data, its physics (phases, melting), the roles of reactions
  src/space/  the shape of the world: the mesh (cells, links, levels), the square map, stacks of layers
  src/store/  the names of the nodes (a fixed node by class and index; a part by slot and generation), the boundary
              nodes sun, space and outside, and the one way to change them
              (`apply`: moves, transforms, link fluxes)
  src/clock/  time in whole seconds, the calendar, processes with periods, sleep and wake of nodes
  src/solve/  the shared maths: spread (implicit step on links), downhill, lines
  src/*.rs    schedule (the month), world, params (Config, Physics), ground (heat and plateaus), check (the books),
              life (the registry of bodies), lineage, nodes (population nodes), lanes (paths), survey (the cells
              bodies see), brain, rng, event, math, units, par
  tests/      one file per law (flow, change, bond, copy, choose, body), one per base part (space, store, clock,
              solve), chemistry0, numbers
worldgen/   terrain, chemistry 0 (chem0.toml) and the chemistry generator, richness checks, first life (life/), what is
              placed when (schedule.rs); the test kit (worldgen::testkit, behind the `testkit` feature)
tape/       the recorder (rec/, its yearly columns in rec/cols/), snapshots (snapshot.rs), map layers, measures,
              the fingerprint of a world; tests: tape.rs, golden.rs
library/    the recognition library (data files); today the Earth names of chemistry 0
cli/        headless runs (`ew run`, `ew resume`, `ew chem`, `ew numbers`; flags in args.rs), pictures; the word ban test
web/        the 2D map viewer (TypeScript)
analyze/    not built yet (M5): pattern finder, fact cards, recognition matching, interest score
```

### 8.2 Core rules (must)

1. **The step is a pure function:** `next_state = step(state, actions, seed)`. No hidden global state.
2. **Random numbers are keyed by place, time and purpose:** `rng(seed, tick, node_id, purpose)`, using a counter-based generator. This makes runs repeatable and lets parts of the world run in any order.
3. **Fixed iteration order.** Never iterate a hash map to decide anything.
4. **No IO, no clocks, no threads required in `core`.** Parallelism (for example `rayon`) is optional behind a feature flag.
5. **The same `core` compiles to native (for batch experiments) and to WebAssembly (for the game).** Keep this working from the first day.
6. **Few dependencies.**

### 8.3 What the simulation records (the history tape)

- **Snapshots:** every 10 years, and before every god action, save the full state, compressed (bincode, then gzip). A run can go on from any snapshot (`ew resume`).
- **Raw physical events:** body born, body died (and cause: lacking a store, frozen or boiled, could not pay, taken, broken, age), taken from, given to, watched, bond made or broken, first time a substance appears in the world, god action applied. Only raw physical facts. **"A city was founded" is not a raw event.** The pattern finder detects that later.
- **Summary series** every year (`series.csv`): totals per element, energy, temperatures, absorbing-gas amounts, population counts, births and deaths by cause, traffic, gifts, imitation and signals. Columns are named by index, never by Earth names (`elem_<j>`, `air_<k>`, `cat_<r>`).
- **Columns by starting kind** (`root_<k>_*`): `<k>` is the starting kind a body descends from. Members, mass, births and deaths by cause, the life the genes allow and the life lived, and the means of the main genes, counted by members (a node of 10⁸ members counts 10⁸ times). In a roster run, read the number of kinds alive from these columns, not from the clustering column `kinds` (it can count at most 6).
- **Months** (`months.csv`): births and deaths by kind and cause each month, so seasons show.
- **The run's numbers** (`physics.json`): every field of `Physics` and `Config`. `ew numbers OLD NEW` prints the fields that differ between two runs, so no number changes without the reader seeing it.
- **Viewer frames** (`world.ewf.gz`): map layers every few years, with the series and the events.
- **A null run** (`ew run --null`, column `null`): learning and imitation off, brain rows frozen, genes still evolve. Compare a real run with its null run (Section 18.3).

The full list of columns is in `docs/tape.md`.

### 8.4 Runtime shells

- **Native CLI:** runs worlds headless, writes tapes, runs batches of many worlds, runs the world search.
- **Game:** the core runs as WebAssembly in a Web Worker, so the screen never freezes. The viewer receives state as transferable arrays.
- **Early prototype shortcut:** the native CLI writes tape files, and a simple web page loads and plays them. Move to the in-browser worker when god mode needs live interaction (M6).

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
VIEWER (Earth view / true view)   ◄── LLM TRANSLATOR (cached, read-only)
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

**Goals:**
- A player understands what they see **instinctively**, without reading.
- What is drawn is **what is really there**. The Earth view only gives it a familiar face.

### 10.2 Two views

- **Earth view (default):** map colours and sprites for familiar concepts (forest, farmland, village, house, boat…), with a "≈" badge where the match is only similar.
- **True view:** every pixel is coloured from physics: the reflectance of the substances in the cell, brighter when hot. Unmatched structures are drawn as simple blocks coloured by their materials. Flows are moving dots along edges.
- One key switches between them.

### 10.3 The base map (pixels)

Each cell is one pixel (or one small tile when zoomed in). Its colour comes from a **map class**, chosen by the **viewer**, never by the core:

| Map class | Comes from | Colour idea |
|---|---|---|
| deep sea / shallow sea | liquid depth, connected to the large liquid body | dark blue / light blue |
| lake | liquid in a closed low area | mid blue |
| river | cell with high, lasting liquid flow | bright blue line |
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

- The base map is **one texture**: one pixel per cell, filled from the world arrays and uploaded when it changes. This is very cheap.
- Sprites and lines are drawn with a 2D WebGL sprite library (for example PixiJS), which batches tens of thousands of sprites easily.
- A plain HTML canvas is fine for the very first prototype.
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

- Use `f64` for the world state in the prototype. Optimise to `f32` only where measurement shows it is needed and conservation still holds.
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
- **Repeatability:** run the same seed twice on the same machine, and the state must be the same, bit for bit.
- **Snapshot round-trip:** save, load, continue. The result must match an unbroken run.
- **One tiny scene per law** showing basic physical behaviour: liquid flows downhill; heat spreads; a reaction runs faster when hot; a bond breaks under too much load; a child has its parent's genes plus noise; a body learns to move towards food.
- **Speed benchmark:** years per second on the reference island. Track it over time.

How the suite is built:

- **The normal suite** (`cargo test --release --workspace`) takes about 15 seconds. Its tests are in one file per law: `core/tests/{flow, change, bond, copy, choose, body}.rs`, one per part of the foundation's base (`core/tests/{space, store, clock, solve}.rs`), with `chemistry0.rs` (chemistry 0's facts) and `numbers.rs` (the rows and budgets of `docs/numbers.md`); `worldgen/tests/` (the start, the roster, the fixers, the names of chemistry 0); `tape/tests/tape.rs` (snapshots, the viewer file, the series, resuming, the null run); `cli/tests/word_ban.rs` (14.4) and `cli/tests/core_size.rs` (14.3).
- **The test kit.** The tests share one kit: `worldgen::testkit` (a small world, founders, edits booked from the outside node, the rock of a cell, the books of a world, chemistry 0's substances by number). It is behind the `testkit` feature, which only the dev-dependencies of `core` and `tape` turn on, so it is never in a run.
- **The golden test** (`tape/tests/golden.rs`). Three small runs (16 × 16 cells, 2 years, every starting kind) must stay whole, keep their books and end on pinned fingerprints (`tape::fingerprint`, a hash of the whole saved state). Only a bug fix may change a fingerprint: `GOLDEN=print` prints the new values. Run it again with `--features pcore/threads`, which must give the same fingerprints.
- **Slow tests** are marked `#[ignore]`: a century that keeps its books, the speed benchmark (`speed`), the learning scene over many seeds, long checks of genes, nodes and the island's erosion, and the key-number test (`tape/tests/ranges.rs`: ten numbers of climate, land and life of three 30-year runs, each near its reference of `docs/foundation/plan.md` 6.3). Run them before the last push of a phase (`cargo test --release --workspace -- --ignored --no-fail-fast`); there is no CI on GitHub for now.

That is the core of testing, until something emerges. Reviews of the code and of the laws happen in rounds, by the agent and the owner together (M0 to M3 used several). What stays banned is proof machinery: receipts, proof campaigns, cross-machine determinism, replay proofs.

### 12.4 Watch reports

Each milestone ends with a short report for the owner, in simple English, saved in the repository as `docs/reports/<name>.md` with its pictures (the repository is the record):
- what we built (a few lines)
- **pictures or a short video**
- key numbers (speed, population, how many kinds, how many patterns)
- **what emerged**, and what did not
- what we will change next, and why

### 12.5 Repeatability (cheap kind only)

- Same seed, same code version and same machine/build give the same history.
- Purpose: finding bugs, and replaying a run.
- **Do not** build identical results across machines, CPU/GPU agreement to the last bit, receipts or replay proofs.

### 12.6 Speed targets (goals to measure, not promises)

| Milestone | Target |
|---|---|
| M0 island physics | 1 simulated year in under 50 ms |
| M1–M3 island with life | 1,000 years in under 10 minutes |
| M4 planet, coarse with detail in places | 1,000 years in under 15 minutes, on the largest planet that fits (the size follows the budget, 5.4) |
| M6 god mode | 200 years of one region in about 1–2 minutes |

**Be honest about these targets.** They are hard. For example, 50,000 bodies with one step per day for 1,000 years in 10 minutes means about 30 million brain decisions per second, plus physics. Expect to miss the M1–M3 target at first. Report the real number (years per second) in every watch report, and do not slow down the project to chase the target before something has emerged.

If a target fails, use the speed tools of Section 7 (population nodes, coarse regions, multi-rate steps), profile, or move the field laws to the GPU. **Never remove a law to get speed.**

---

## 13. Milestones

Each milestone ends with "watch it and see". The **watch-for** column says what we hope to see. If it does not appear, the next step is to change physics, perception, memory or learning, and **never** to add a named rule.

| # | Build | Watch for |
|---|---|---|
| **M0** | Workspace; chemistry generator with richness checks; terrain generator; Flow (liquid, gas, heat, radiation, wind, dissipation and carrying); Change (non-living); history tape; simple viewer: terrain plus layers plus timeline | Rain falls; **rivers and lakes form by themselves** and carve valleys over centuries; climate bands; deserts behind mountains; weaker sun → ice spreads; more heat-absorbing gas → warmer world |
| **M1** | Bodies; catalysis; Copy with mutation; simple Choose (move, take, put, copy, rest) | Light-eaters spread where conditions fit; eaters of eaters appear; kinds differ by climate; populations rise, fall and cycle; extinctions happen |
| **M2** | Better Choose: memory, memory of individuals, imitation, signals; movement dissipation; optional planning | **Trails form** between often-visited places; seasonal movement; learned behaviours spread by imitation; signals settle into consistent use; first giving or exchange between bodies |
| **M3** | Bond v1: holding, joining, structures, containment, load and failure, floating | Held objects used as tools (they change what actions achieve); **shelters** used in cold times; containers; floating structures |
| **M4** | Speed: population nodes, coarse regions, multi-rate steps, planet-size world | M1–M3 patterns still appear in merged form; speed targets met |
| **M5** | Pattern finder; recognition library (about 40 concepts); matcher; LLM translator; Earth view with map colours and sprites | A non-expert looks at the screen and **can say what they see** |
| **M6** | God mode: actions, auto-save, then-and-now view, "what happened since" story | Interventions have visible, believable consequences |
| **M7** | Long runs and **world search**: an interest score, hundreds of worlds, keep the best | In some worlds: **tending and domestication**, settlements, exchange routes, metal-like processing, large settlements, polity-like patterns, burning of fuel, climate change |
| **M8+** | Bond v2 (rotation) and v3 (lift); more library concepts; art; performance (GPU); game polish | Wheel-like and sail-like things; engines; car-like and plane-like things in rich worlds |

### 13.1 Check point after M3 (must)

Part 1 (M0 to M3) is well understood: rivers, climate, simple life, trails and shelters have been shown by other projects. Part 2 (M4 to M7: farming, metal, settlements, polities) has **never** been done from a few laws and small learning brains. It is research, and it may fail.

So after M3, **stop and decide**. Write a watch report with:
- which M0–M3 watch-for items really appeared (with pictures), and which did not
- the real speed (years per second) and the real code size
- the three biggest things that did not emerge, and the best guess why (physics, perception, memory, learning)

Then the owner decides one of:
1. **Continue** to M4–M7 as planned.
2. **Spend a fixed time** (for example one month) on Choose improvements (planning, longer memory, imitation) to get past "animals and trails", then check again.
3. **Stop and polish** Part 1 as a smaller, finished thing: a living island with climate, life, trails and shelters that can be watched and poked in god mode. This is already a good result.

Do not drift into Part 2 without this decision.

**The owner's decision (17 October 2026):** first a foundation phase, then M4. The work, its decisions and its
checks are in `docs/foundation/plan.md`.

**Interest score (for M7), suggested ingredients:**
- number of different repeated structures
- size of the largest lasting settlement
- number of new substances made by bodies
- length of the longest copy lineage (of behaviours and designs)
- variety of cultures
- number of polity-like patterns that rose and fell
- distance covered by the longest lasting route

Use it only to **choose worlds and tune free physics numbers**. Never use it to add rules.

### 13.2 Open items for M4

Only items that are still open. Do not build them before M4 starts unless the owner asks.

- **Nodes against individuals.** The check of Section 7.1 is M4's first job. It needs a small test world where the animals stay near 10⁵ members.
- **Long runs.** Three 300-year runs at 128×128 (seeds 1, 5, 7) are in `docs/reports/foundation-a.md`: life lasts, the seasons hold, the light gene rises only a little (11.30 to about 11.45), fixing does not fall by itself (22% to 58% of plant members at year 300). Open: the carbon gas settles near 300 ppm in seeds 5 and 7, but falls to about 200 ppm in seed 1 (see "Carbon").
- **No people-like kind is expected from M4 itself.** Nothing on the island pays for a tool, a shelter or a bigger brain yet, and planning is not built (6.5). The intelligence step is option 2 of the check point (13.1), after M4.
- **What eaters eat.** Eaters live almost only on dead plant matter, digested in full; nothing stops them before they eat most of the plants' production. A structural plant matter that only a priced catalyst breaks is an option (the owner decides).
- **Warm blood.** A body whose upkeep rises about 9% a kelvin has no steady warmth with real fur, because no body can raise its conductance (no sweating or blood flow to the skin). `KEEP_T` and `COOL_T` are not selected on the mild island; M4's cold and seasons will test them.
- **Carbon.** The carbon cycle has no source (a volcano-like input is the same physics as the god action). The sea's store goes with its depth to the power 2/3, the Revelle factor is 3, and the whole depth is one mixed layer (6.1), so the sea answers the air in years, not centuries.
- **Inside bodies.** The litter constants also run inside living bodies (real living stems and bones do not rot). The rock salt cannot be kept in a body (real bodies bind calcium in solids). A stronger form of product inhibition (a full store stops the enzyme) was measured. Each is the owner's choice.
- **Water and air.** Transpiration is set by the breath (`GAS`), not by the liquid or the heat. Still air is forced convection at one anchored wind; free convection (calm air) is not modelled.
- **Tuned numbers that matter.** `bump` (tuned) decides whether a body can move; `c_brain` sets most of what an animal pays to think (plants have no brain).
- **The methane-like gas** lives by the air's temperature (months at 280 K, years at 255 K). If a warm world has a methane story, look here first.
- **Fire and disease.** Natural fire starts wait for M4: they need a "burning share of a cell" law (a fire front) and M4's weather, with the spark booked like the sun. Disease (bodies inside bodies) waits until after M4.
- **Reading the eaters.** Eaters still grow about 0.2 a year at year 60, so report log10 kg at year 60 with the growth rate over years 30 to 60, three seeds at least. Under 2 times is noise.
- **Core size.** `core` must reach its budget of 8,000 code lines (14.3, counted by `cli/tests/core_size.rs`) by the end of the foundation, mostly by a small store and by deleting the special code the foundation replaces (`docs/foundation/plan.md`, "The line budget").

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

### 14.3 Code budgets

| Part | Budget |
|---|---|
| `core` (five laws, state, random numbers) | ≤ 8,000 lines of code through M7, not counting comments and blank lines (the owner's decision, 17 October 2026) |
| `worldgen` | ≤ 3,000 lines |
| `analyze` + `library` loader | ≤ 6,000 lines |
| whole repository (excluding art assets and library data) | ≤ 25,000 lines through M7 |

**Budget for tuned numbers.** `docs/numbers.md` has one row for every field of `Physics` and `Config` (and for the free constants outside them), with its unit, the fact it is, the real size it is anchored to and a mark: physical, anchored or tuned. A tuned number is one set so a run came out a certain way (a rule in a smaller coat, Section 2, lesson 3), or one with no real size behind it. A test (`core/tests/numbers.rs`) fails when a field has no row, when the tuned rows are more than `TUNED_BUDGET` (**61** at the end of M3), or when the second table, "Rules chosen for a run" (things picked for a run that are not numbers, such as the hand-set eater start and the fixing founders), has more than `RULES_BUDGET` (**3**) rows. **The rule: the counts may not grow.** A new number must be physical or anchored, or it replaces a tuned one. When a tuned number is anchored or removed, lower the budget in the test. The M7 world search varies only tuned and anchored numbers, within their anchors, never the physical ones.

**Code size at the end of M3:** `core` is 10,372 lines: 7,508 of code, 2,260 of comments, 604 blank. A comment line
starts with `//` after its spaces. The test `cli/tests/core_size.rs` counts the code lines of `core/src`, prints them
per folder (`cargo test --release -p cli --test core_size -- --nocapture`) and fails above its ceiling: 9,000 while
the foundation is built, 8,000 at its end. Comments are not counted, so do not delete them to
save lines. The foundation work keeps `core` at or under 8,000 code lines by replacing special code with general
pieces, not by adding to it (`docs/foundation/plan.md`, "The main rule" and "The line budget").

If you hit a code budget, simplify before adding. The budgets are **goals, not hard stops**: the viewer and the pattern finder may need more than expected. If a part goes over its budget, tell the owner with the number and the reason, and keep going. Going over by 20% with a reason is fine. Going over by 3× means something is wrong (see Section 2).

### 14.4 Earth-word ban in `core` and `worldgen`

Add an automatic check (a script in CI or a test) that fails if these words appear in identifiers, comments or strings in `core/` or `worldgen/` (case-insensitive, whole words). Extend the list as needed.

> farm, crop, harvest, sow, herd, livestock, domestic, tame, pasture, house, home, hut, village, town, city, settlement, road, route, trail, river, lake, ocean, sea, market, trade, money, price, coin, wage, debt, loan, property, owner, king, queen, lord, council, kingdom, empire, nation, war, army, weapon, church, temple, religion, god, tool, axe, plough, wheel, cart, car, boat, ship, plane, engine, metal, iron, copper, bronze, steel, gold, wood, stone, coal, oil, fire, food, grain, meat, water, medicine, disease, tribe, family, household, culture

Allowed elsewhere: `analyze/`, `library/`, `web/`, documentation and LLM prompts.

Note: words like `liquid`, `gas`, `solid`, `energy`, `heat`, `body`, `bond`, `flow`, `node`, `edge`, `kind` are fine. They describe physics or structure, not Earth history.

**Do not ban these words.** They are normal programming or physics words, and this document itself uses them in the core: `state` (as in `next_state = step(state, …)`), `field` (as in "field laws", a grid of numbers), `path` (a path through the graph, or a file path). If a banned word is also a common code word, prefer a different word in the ban list (for example `kingdom` instead of `state`).

### 14.5 Do not

- Do not write a rule, action or system named after an Earth outcome.
- Do not hand-shape rewards for specific behaviours.
- Do not add realism (field data, calibrated species, real-world parameters) unless it is needed for something to emerge.
- Do not build proof machinery: cross-machine determinism, receipts, replay proofs, proof campaigns. (Reviews of the code and the laws in rounds, by the agent and the owner together, are part of the work.)
- Do not build branching timelines.
- Do not let the LLM or the library influence the simulation.
- Do not try to find or rebuild version 1.
- Do not run `cargo clean` or delete build folders as routine cleanup.

---

## 15. Risks and what to do

| Risk | Sign | What to do |
|---|---|---|
| Emergence stalls at "animals and trails" | M3 or M7 watch-for fails for a long time | Improve Choose: planning, memory, imitation. Search more worlds (M7). Check that the physics allows the wanted thing (Section 9.4). |
| Long-term behaviours (taming, farming) never appear | No lasting flows between kinds | Planning and longer memory; check that tended kinds really change under Copy; check that seasons give a reason to store |
| Bond physics too slow | Profile shows load computation dominates | Simpler load approximation; update loads only when a structure changes |
| Brains too slow | Bodies dominate the step time | Shared brain bases per lineage; no brain where nothing is decided (light-eaters); population nodes; fewer updates; GPU later |
| Long chains of actions never learned (for example heat rock with fuel, wait, get a new solid) | Bodies find food and shelter but never make new substances | This is the main research risk. Planning (look-ahead), curiosity-like exploration in the brain, longer memory, imitation. Use the M3 check point (Section 13.1). Do not add a named recipe |
| Boring chemistry | Few reactions happen; nothing new is made | Stronger richness checks; world search |
| Pattern finder too slow | Analysis falls far behind the simulation | Analyse fewer snapshots; analyse changed chunks only; run in the background |
| Art cost explodes | Many concepts without sprites | Few variants per concept, material tints instead of new drawings; generic block sprites as fallback |
| Earth view misleads players | Players think the world copies Earth | Keep "≈" badges, "why this name", and the true-view switch |
| Team adds named rules "just this once" | A named concept appears in `core` | The word-ban check; review against Section 3 |
| Results are strange, not Earth-like | Nothing matches the library | That is allowed. Use local names and true view. Grow the library only from what appears. |

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
| **Earth view / true view** | familiar map colours, sprites and names / pure physics colours |
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

**Null run:** neutral brains draw every action by chance, so many "yes" verdicts (gifts, watching reaching many lineages, busy crossing cells) are what chance predicts. Run the same seed with `ew run --null` (`Config::null`: learning off, a watch is drawn and counted but moves no rows, born rows and brain bases never mutate or split; genes still mutate and evolve). The config line printed at the start and the series column `null` say which run it is. An item counts as emerged only where the real run beats the null run clearly, on the same columns (`gave`, `watched`, `watched_groups`, `kinds`...). The column `learned` is not zero in a null run (it also moves when members of a node die or leave), so read it as a floor.

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
