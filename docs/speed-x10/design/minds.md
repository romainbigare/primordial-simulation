# Speed x10: minds (the owner's choices 8, 9, 10 and 11)

> **After P1.2** (X67): MI3a and MI3b are dropped; G2 (`lane-a.md` 3) is a habit with the clock as its one wake rule,
> for nodes; `--full-minds` waits for the first smart kind; MI4 stays gated after G2.
>
> **Note: what the plan kept** (`plan.md` is the single source of the steps; this design keeps its old numbering).
> MI0: in R2 (plus wake causes counted by cause, `others` keyed by lineage, decisions per (cell, lineage, batch)).
> MI1: kept, lands before E1b.1. **MI3: split** into MI3a (habits, still paying per update; `--full-minds` checked
> on it) and MI3b (pay per thought, its own law commit after P1b) (X28). MI5: kept, before G2 (it needs MI1, not
> MI3). **MI3a, MI4 and MI3b come after G2** and are built only if their gates, measured again after G2, still hold
> (X36: after G2 MI3a is likely worth 5 to 8 ms, MI4 1 to 2 ms); if MI3a is not built, `--full-minds` waits for the
> first smart kind, and G2 plus G16 stand in their place. **MI6: dropped**, replaced by G2 (X16). MI7: with the first smart kind. **MI8** (batched networks here; `full_hidden` in
> the old plan): both dropped (X12, X25); GM measures a thought. The shares of section 0 and 6 are older: Z1 gives
> `choose::act` 11.6% (island), 17.8% (globe), 3.5% (region); R2 restates them. Code names say "group", never
> "herd" (X35).

Section of the x10 plan. It covers: think only when something changes (8), think in groups (9), neural networks in
batches (10), and the door for smart bodies (11). Other sections: plants, engine, air.

## 0. In short

- **Where the time goes.** `choose::act` is 16.0% (island22), 15.9% (globe36) and 9.7% (region24) of a year's
  simulation instructions (one thread). The neural network itself is small: think, learn and trace together are only
  1.4% to 2.1% of a year. The rest is **perception** (the faces loop, the cells within sight) and **acting** (a node
  spreads its members over its options: each part is split off, acts, and comes back).
- **Acting is 40% to 64% of `choose::act`.** "Think less" does not remove it. Only "update calm animals less often"
  (step MI6) removes acting and the animal's physics too.
- **Main choices:**
  1. **Habits (choice 8).** A body with a brain thinks when its senses changed, when its habit is no longer possible,
     or once a season. Between thoughts it acts by the chances of its last thought. It learns at its next thought from
     the whole time since (Sutton, Precup and Singh 1999). A thought is paid when it is thought.
  2. **Groups (choice 9).** A node already thinks once for all its members: 96% (island) to 99% (globe) of all
     decisions are node decisions. What is new: the senses of a *place* are shared by the bodies there (one table per
     cell and sight, per survey), and a node's movers leave as *one* group. One decision for several unlike nodes is
     a merge of unlike nodes. That is a world choice (S66), not a speed tool, so it is not proposed.
  3. **Batched networks (choice 10).** Not built now. Each body owns its learned rows, and the rows are 75% of a
     brain's weights, so a matrix of many brains reuses only the lineage's base layer. Gain today: about 0.1% of a
     year. It pays only for big brains with a big shared base and small rows. That is a brain change: the owner decides.
  4. **Full minds (choice 11).** A control run `--full-minds` (every body thinks at every update) is both the check of
     habits and the code path for smart bodies later. A smart lineage is marked by its brain size, stays an
     individual (`nodes::may_be_node`), and may run with a shorter step.
- **Gain, honestly.** Steps MI1 to MI5 make `choose::act` about half as dear: a year 1.04x to 1.08x faster (one
  thread). MI6 (calm animals update less often, gated on the plants' choice 3) may give 1.10x to 1.22x more. The minds
  work is still needed: without it, `choose::act` alone caps the whole round at 1 / 0.16 = 6.25x (Amdahl). After MI1
  to MI5 the cap is about 11x.
- **Lines:** about +280 and -65 core code lines (net about +215).

## 1. What exists today

### 1.1 The decision

- `survey::update` (`core/src/survey.rs:259-297`) runs `body::live` (physics), then, for a body with a brain,
  `choose::act` (`survey.rs:277`). So **one update = one decision**. A body updates every `Body::interval` days
  (`core/src/body/mod.rs:245`), its `INTERVAL` gene: 5 to 360 days, eaters about 7 to 20 days
  (`worldgen/src/life/kinds.rs:81-111`).
- `choose::act` (`core/src/choose/mod.rs:79-108`), in order:
  1. well-being now (`Body::wellbeing_at`, `body/mod.rs:309`);
  2. `perceive` (`choose/perceive.rs:71-109`): up to `SEEN` bodies from the faces of its cell and 8 neighbours. For
     each face: walls, then cover with chance `exp(-cover)` (one f64 `exp` per face, `perceive.rs:86-87`), and a
     `BTreeMap` look-up (`Domain::find`) for the faces of its own cell. Then up to 3 parts;
  3. `inputs` (`perceive.rs:112-127`): own stores against minimums, the cell's loose substances (`Life::seen`),
     8 scalars (warmth, size, light, liquid, ground temperature, crowding, season sin and cos), signals heard,
     `bond::sense` (5), memory;
  4. `brain::think` (`core/src/brain.rs:104-123`): one hidden layer (the lineage's base), then every row (the body's);
  5. `learn::learn` (`choose/learn.rs:18-38`): actor-critic on the change of well-being since the last decision;
  6. `walk` (`perceive.rs:44-67`): the cells within `sight` rings, breadth first, then sorted;
  7. `options` (`mod.rs:125-185`): a score for each fixed action, each of 8 directions (weighted mean features of the
     cells seen that way), each verb toward each body and part seen;
  8. `pick` (`mod.rs:188-205`): softmax, one draw; `noise` (`mod.rs:209-217`): signal, part shape, next memory;
  9. an individual: `trace` and `act_as` (`mod.rs:263-287`). A node: `spread::spread` (`choose/spread.rs:14-28`):
     `count × p[o]` members do each option o. Members that stay in the cell act as the node while the others wait
     aside (`nodes::Lot::split`, `nodes::rejoin`). Movers and bond actors are placed as a body of their own
     (`nodes::part`, `Domain::place`), act, and either leave (a move queued in the outbox) or come back. Each part act
     measures well-being before and after (`spread.rs:42-44`) for the learning advantage.
- **The brain** (`brain.rs`): `Shape` (`brain.rs:34-80`). A body's founders have 12 hidden units and 6 memory numbers
  (`worldgen/src/testkit.rs:84-85`); the gene allows 0 to 64 hidden and 0 to 12 memory (`body/genes.rs:105-108`).
  With 9 substances: base 480 weights (lineage), rows 119 × 13 = 1,547 weights (body). So **the rows are 76% of a
  brain**, and they are each body's own (born rows plus what it learned, `Body::rows`). A brain of 64 hidden units:
  base 2,944, rows 8,125 (73%).
- **Lineages** (`core/src/lineage.rs`): shape, `seen`, `sight`, base, root. `Lineage::cost` (`lineage.rs:28-34`) is
  the cost of one decision; `pay::upkeep` (`core/src/body/pay.rs:17-29`) pays **one decision per member per update**.
- **Faces** (`survey.rs:154-196`): each batch starts with a face of every living body (id, n, vis, sig, wb, room).
  Perception reads only faces.
- **Watching** (`learn::watched`, `learn.rs:46-59`) is an outbox entry, applied in the serial phase.
- **Nodes:** `nodes::may_be_node` (`core/src/nodes.rs:96-99`) is the one predicate (plan 2.2 rule 1).
  `--individuals` turns nodes off for bodies with a brain (`core/src/params.rs:82`).
- `brain::shake` (1.5% / 1.1% of the run) is mutation of born rows for children (`copy/mod.rs:281, 302`) and members
  drawn from a node (`nodes.rs:303`). It is part of Copy, not of the decision; it is not in this section.
- "plan ~3%" in the brief is `body::pay::plan` (the burn planner of paying, `pay.rs`), about 5 calls per update. It
  is body physics, not a decision planner. Part of it is called by acts (`feed`, `go` pay through it).

### 1.2 Where the decision cost goes (measured)

From the callgrind profiles in the scratch `gates-out` (one year, one thread, the code of `5fa02b0`; the decision code
is unchanged since). Line-level sums were made from `prof-*.callgrind` (script in scratch `x10/py/lines.py`). Shares
are of the simulation (`schedule::run_for`: 11,871, 7,972 and 11,310 million instructions).

| | island22 | globe36 | region24 |
|---|---|---|---|
| body updates a year | 54,314 | 22,791 | 51,320 |
| decisions (`choose::act` calls) | 24,030 | 10,801 | 16,287 |
| of them by individuals (not nodes) | 909 | 77 | 656 |
| `choose::act`, million instructions | 1,899 | 1,271 | 1,096 |
| `choose::act`, share of the simulation | **16.0%** | **15.9%** | **9.7%** |
| instructions per decision | 79,000 | 118,000 | 67,000 |
| **acting**: node parts act (`act_as` from `spread`) | 584 | 365 | 348 |
| acting: well-being before and after each part act | 126 | 168 | 71 |
| acting: parts placed, split, back (`nodes::part`, `place`, `back`, `rejoin`) | ~50 | ~276 | ~20 |
| **acting, total** (share of `choose::act`) | ~760 (40%) | ~810 (64%) | ~440 (40%) |
| perception of bodies: faces loop with the cover `exp` (130 / 25 / 54) | ~260 | ~50 | ~110 |
| perception of cells: `walk` and the direction features of `options` (estimate) | ~350 | ~200 | ~250 |
| the network: `think` (~120 / 58 / 87), `learn` (~80 / 38 / 51), `trace` (~47 / 20 / 24) | ~250 | ~115 | ~160 |
| the rest: setup, well-being at start, inputs, `bond::sense`, pick, noise | ~280 | ~95 | ~135 |
| **thinking, total** (share of the simulation) | ~1,140 (9.6%) | ~460 (5.8%) | ~655 (5.8%) |

Notes:

- The "estimate" rows: many lines of `walk` and `options` are inlined into the standard library's iterator and
  comparison code, so the profile does not name them. The remainder after all named lines is given to them.
- Acting is dear on the globe because node movers split often: 22,903 parts placed for 10,801 decisions, and 12,942
  of them came back without moving (a move that could not be paid or pushed). Each costs about 10,000 instructions,
  and every move that lands costs more in the serial phase (`outbox::deliver` 5.1%, `nodes::add` 5.1% of the globe's
  year, part of it from movers landing).
- One decision costs about half a body update (island: 149,000 instructions an update on average).

## 2. The new design

### 2.1 Habits: think only when something changes (choice 8)

**Biology.** Animals do not decide anew at each moment. They keep a course of action while it stays good enough
(satisficing, Simon 1956) and react to events: a predator, a mate, hunger, a change of season. Individual-based
models schedule agents by events for the same reason (Grimm and Railsback 2005, chapter on scheduling). A decision
that is kept for several steps is an *option* in reinforcement learning; the learner then learns at the end of the
option from all that happened during it (Sutton, Precup and Singh 1999; Bradtke and Duff 1995).

**Data.** A new file `core/src/choose/habit.rs` (about 120 lines):

```rust
/// What a body with a brain keeps doing between its thoughts (laws.md 5.6): the chances of its options at its last
/// thought, the targets they had, and what it sensed then.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Habit {
    /// The chance of each option at the last thought (empty: none yet, so it thinks)
    pub p: Vec<f32>,
    /// The targets of the options then: bodies seen (ids, and whether in its own cell), parts seen, the cells of its
    /// moves, and the shape of a part it would make
    pub plan: Plan,
    /// Its senses then (`senses`), and the other kinds in its cell then (`others`)
    pub senses: Vec<f32>,
    pub others: u64,
    /// The day of its last thought, and the updates since
    pub since: u64,
    pub held: u32,
    /// It thought since its last update: its upkeep pays that thought (`pay::upkeep`)
    pub owed: bool,
}
```

- `Plan` holds what `act_as` needs and nothing else: `seen: [(u64, bool); MAX_SEEN]`, `nseen`, `parts: [usize;
  PARTS]`, `nparts`, `to: [usize; MOVES]`, `form: [f32; FORM_OUT]`. Today `act_as` reads these from `Choice` and
  `Kit`. `Choice` will hold a `Plan`, and `act_as` takes a `&Plan`. One place, used by the thinking path and the habit
  path.
- `Body` gets `pub habit: Habit` (saved: bump `SNAPSHOT_VERSION`). About 420 bytes for a body with a brain, empty for
  one without.
- `Face` gets `lineage: u32` (4 bytes), for `others`.

**The senses.** `habit::senses(w, b, lived, temp, out)` writes the part of the brain's inputs that does not need the
faces: own stores against minimums (ns), the cell's loose substances (ns), warmth, size, light, liquid, ground
temperature, crowding, and `bond::sense` (5). It leaves out the season (it moves every day; the floor handles it),
the signals heard and memory. It shares the code of `inputs` (`inputs` calls it and adds the rest), so there is one
place for each sense.

**`others`.** An xor of the ids of the faces in its own cell that are of another lineage. It changes when a body of
another kind comes or goes (danger, prey, a stranger). Bodies of its own lineage come and go all the time (children,
merges) and do not wake it.

**The rule** (`habit::asleep`). A body with a brain keeps its habit at an update when all hold:

1. it has a habit (`p` not empty), and `w.cfg.full_minds` is off;
2. it thought less than `sleep_most` days ago (`Physics::sleep_most`, 90 days: one season);
3. no sense moved by more than `wake` since its last thought (`Physics::wake`, 0.25, the largest absolute change
   over the senses);
4. `others` is unchanged;
5. its habit is still possible: each body seen with a chance of at least `1 / count` of a member is still in the faces
   (in its cell if the verb needs it) and not leaving; each such part still exists in its cell and the verb is still
   `bond::possible`; the move cells are still neighbours (always true on a fixed mesh).

Else it thinks, as today. Its rows changed from outside since (a watch applied, a merge with other rows in
`nodes::add`, `lineage::resize`) also make it think: these call one helper, `Habit::forget` (clears `p`).

**While asleep** (`habit::repeat`):

- An individual draws one option from `habit.p` (a fresh draw with its rng; a habit is a policy, not one act) and
  acts with `act_as` on `habit.plan`.
- A node spreads its members over `habit.p` exactly as today (`spread::spread`, the same stochastic rounding), but
  without the well-being before and after each part act and without a trace: nothing is learned from an asleep
  update.
- No perception, no network, no `walk`, no `options`, no `pick`, no `noise`. The signal and the memory stay as they
  were. `b.wb` stays the well-being of its last thought (others see that in its face, as today: "at its last
  decision").

**Learning still gets its signal.** At the next thought, `learn::learn` compares the well-being now with that of the
last thought (`b.wb`), as today, and discounts the critic's estimate by `discount^k`, `k = held + 1` updates since the
last thought (today `k = 1`). So the decision that started a habit gets the credit or blame of the whole habit: the
semi-Markov form of actor-critic (Bradtke and Duff 1995; Sutton, Precup and Singh 1999). The trace of that decision
is kept in `Body::trace` until then, as today.

**Paying.** A thought is paid when it is thought. `pay::upkeep` pays one decision per member only when `habit.owed`
(it thought since its last update), then clears it. A body without a brain pays none, as today. This keeps the
handover's rule ("a brain is paid per decision, so a body that decides less often pays less") with "decision" read as
"thought". No new call of `body::pay`, so no new cost.

**Counters** (not state): `Counters::thoughts` beside `decisions` (`core/src/world.rs:13-17`). `docs/tape.md` gets
the column.

**Where it plugs in.** Only `choose::act` changes its start:

```rust
pub fn act(w: &World, d: &mut Domain, b: &mut Body, lived: &Lived, t: &mut ActTmp) {
    let mut rng = Rng::new(w.seed, w.day(), b.id, Purpose::Choose);
    ...
    habit::senses(w, b, lived, temp, &mut t.senses);
    if habit::asleep(w, d, b, &t.senses) {
        return habit::repeat(w, d, b, &mut rng);
    }
    ... // the thinking path as today; at its end:
    habit::keep(w, b, &c, &t.senses);
}
```

`survey::update`, the batch, the outbox and the faces are unchanged.

### 2.2 Shared senses of a place (choice 9, first part)

The direction features of `options` (`mod.rs:125-162`) depend on the cell, the sight, the survey's caches
(`cellf`, `env`, `seen`, all made once a survey), the mesh, and two things of the body: what it can take (`smell`,
a dot product of its uptake genes with `seen`) and the lane's resistance to the next cell (`lanes::resist`, which
changes only in the serial phase).

**Design.** A per-domain cache, `Domain::places: Vec<Option<Box<Place>>>`, indexed by (cell slot, sight - 1), cleared
when the survey runs on that domain (`survey::survey`, which already runs per domain on the pool):

```rust
/// What a place offers to look at in each direction, for one sight (choose::options): per direction the weighted mean
/// features of the cells seen that way, but what a body can take there and the step along the link, and the weighted
/// mean of each loose substance there.
pub(crate) struct Place {
    pub ways: [(usize, usize, [f64; 2]); MOVES],  // direction, neighbour cell, heading
    pub nw: usize,
    pub feat: [[f32; NF]; MOVES],
    pub seen: Vec<f32>,                            // MOVES × ns: the weighted mean of `Life::seen`
    pub ck7: [f32; MOVES],                         // the next cell's own resistance feature
}
```

- Built at the first decision in that cell and sight after a survey (`walk` moves into the builder). Each later
  decision there reads it.
- Per decision: `fd[8] = Σ (k, u) u · seen_d[k] - here` (the sum over the cells is done once, so the order of sums
  changes: check level 2), and `fd[7] += wt_j · (ln(resist(i, j)) - ck7)` with the live lane, 8 calls.
- Physics: none. All bodies in one place see the same land; each still sees it with its own uptake and its own brain.

### 2.3 Movers leave as one group (choice 9, second part)

Today a node of `count` members sends `count × p[d]` members in each direction d that has a chance: up to 8 parts,
each split off, placed, paying, queued, landing in the serial phase and merging there.

**New rule** (`spread::spread`): the movers of one decision are `x = count × Σ_d p[d]` members (less than one: one
member with chance x, as today). They take **one** direction, drawn with chance `p[d] / Σ p`. Each direction still
gets `count × p[d]` members on average, so the mean flow of members is unchanged; members that leave together move
together, as a herd splits off a group (group fission; Couzin, Krause, Franks and Levin 2005). The spread of
members over a place grows (more patchy), the mean does not.

Also: the cheap checks of `go` that do not depend on the part (the target cell's room, `bump` per kg) run on the node
before a part is split, so a move that cannot happen splits nothing (`go` gets a `can_go(w, b, j, share)` helper used
by both).

### 2.4 Calm animals update less often (choice 8, the part that removes acting)

Habits remove thinking only. The acting and the physics of an animal's update stay. A calm animal can be updated less
often: its world is not changing, so a longer step is accurate enough (adaptive time steps; event-driven scheduling,
Zeigler 1976).

**Rule** (in `Body::interval`, the one place for it, `body/mod.rs:245`): a body with a brain that kept its habit at
this update gets the next interval `min(2 × this interval, sleep_most)`, in whole batches; a thought sets it back to
its gene's interval. The rates of the update already follow `Body::paces` (`body/mod.rs:250-258`): intake rooms,
deaths per span, `feed::intake` by days. So no new pace code.

**Honest risk.** This is "fewer updates" (phase C). C2 failed (S62) because upkeep is paid at the start of an interval
before its light comes in (the C2 finding, report open item 6). The plants section fixes this splitting (choice 3:
upkeep and income together across the interval). MI6 waits for that fix and must pass the pace test for animals.

### 2.5 Full minds for smart bodies (choice 11)

The three rules of plan 2.2 hold:

1. **Individuals stay real.** `nodes::may_be_node` (`nodes.rs:96-99`) stays the one predicate. A full mind is never a
   node: `w.cfg.nodes && !full_mind(w, b) || w.life.brainless(b)`.
2. **The step per place.** Habits are per body and the wake check is per update, so a domain with full minds can later
   run sub-batches; nothing here assumes one batch length.
3. **Kinds of body code.** Three paths: fields for plants (plants section); animal minds (habits, nodes, shared
   place senses, movers as one group, calm intervals); full minds (think at every update, no habit, no stretched
   interval, individual). Nothing in the animal path may assume every body sleeps.

**The mark.** Now: the control run `--full-minds` (`Config::full_minds`): every body with a brain thinks at every
update, and the calm interval is off. It is the detailed form that habits are checked against, as `--individuals` is
for nodes. Later, when the first smart kind is made (after M4, handover 13.2): `Lineage::full(&self, cfg)` is true
with `--full-minds` or when the lineage's hidden units are at least `Config::full_hidden` (a run choice, like `nodes`).
The mark is on the lineage, as the brain shape is: all bodies of a lineage are of one kind, so a domain can group
them. It is not built now (no dead code).

**Cost of a full mind**, from the measures: thinking about 42,000 instructions a decision today; a brain of 64 hidden
units makes think and learn about 5.5 times dearer (11,069 weights against 2,027), about 85,000 instructions a
decision. 10,000 smart individuals deciding every 5 days: 730,000 decisions, about 62 billion instructions, about
15 s a year on one core. That is when batching (2.6) and the habit-free path need care.

### 2.6 Batched networks (choice 10): designed, not built now

**The kernel.** In a domain's batch, group the bodies that think by lineage. Stack their inputs `X` (n × ni, f32,
row by row). The base layer is one matrix product with reuse: `H = squash(X · W1ᵀ + b1)`, tiles of 4 bodies × 8
lanes, each base row read once per tile (auto-vectorised plain loops as `brain::dot`; no new crate, no `std::simd`).
The rows stay per body (`brain::think`'s quad loop, already 4 sums at once).

**Why not now.** The base is 24% of a brain's weights today (27% at 64 hidden units). Reading it 4 times less saves
about 12% of `think`: 0.1% of a year. The network (think, learn, trace) is 1.4% to 2.1% of a year in total.

**What it needs from the order.** Inputs read the body's own state after its own `live`, and the store view as the
domain's earlier bodies left it. A batch of brains needs: `live` and the senses of all due bodies of the domain
first, then the networks, then the acts in order. Then a body's `live` runs before the acts of the bodies before it,
which changes what it finds in the store view (check level 3). Faces are not a problem: perception already reads the
batch's start.

**When it pays.** When the network is at least 5% of a year (the gate), and when the brain keeps most of its size in
the shared base and only small rows per body (like small learned adapters, Hu et al. 2021). That is a change of the
brain (laws.md 5.1). It is the owner's choice, at the intelligence step.

### 2.7 Small fix: the cover of a face once a batch

`perceive` computes `exp(-cover)` for every face it looks at (`perceive.rs:86-87`), about 92 times per decision on
the island. `cover` depends only on the face and its cell's canopy, which do not change in a batch. `Faces::make`
computes `hide: f64 = exp(-cover)` once per face (cover <= 0: no draw, as today). The draw is the same, so the result
is the same bit for bit (level 1).

### 2.8 The step loop, snapshot, tape, viewer

- **Step loop:** unchanged. Habits live inside `choose::act`; the calm interval inside `Body::interval`; place senses
  in the domain, cleared by the survey.
- **Snapshot:** `Body::habit` is saved (bump `SNAPSHOT_VERSION`). `Domain::places` is a cache, not saved
  (`#[serde(skip)]` is a skip of a cache, not compatibility code; or rebuilt as `Life::faces` is).
- **Tape:** a column `thoughts` (per year, beside `decisions`). `docs/tape.md`.
- **Viewer:** the inspect panel can show "thinks" or "by habit" from `habit.held` (optional, no core cost).

## 3. The law changes in physical terms

| Change | What a body does differently | Why it is honest |
|---|---|---|
| Habits (MI3) | Between events it keeps acting by the chances of its last thought, without new perception | It still acts, pays every act, takes only what is there (the act code is the same, so no overdraw). It wakes on any change of its own state or of the other kinds beside it, and once a season |
| Paid per thought (MI3) | A body that keeps its habit does not pay for a decision | Fewer thoughts, less energy: what the handover already says (6.5). Books: the cost comes from its stores through `pay::upkeep`, as today |
| Learning over a habit (MI3) | It learns at its next thought from the whole habit, discount `γ^k` | The semi-Markov actor-critic; no reward is hand-shaped |
| Place senses (MI4) | Nothing (rounding only) | Same features, order of sums changes |
| Movers as one group (MI5) | A node's movers of one decision go one way | Each way gets the same members on average; more patchy |
| Calm interval (MI6) | A calm animal updates every 2, 4, ... intervals, up to a season | The rates over an update follow `paces`; waits for choice 3 |

No change of Flow, Change, Bond or Copy. Books: every act still goes through the same helpers (`feed`, `go`, `bond`,
`nodes::Lot`), so every transfer is booked both sides.

## 4. Rules that change, with the new words

- **CLAUDE.md 2.6:** "A switch stays only if it is a control run that a law is checked against (today:
  `--individuals`, `--full-minds`, `--no-mix`, `--null`)."
- **handover 6.5, Implementation**, the cost bullet: "**A brain has a size and a cost.** Its sizes are genes, and zero
  is a size: a body with no hidden units takes no decisions and pays nothing for a brain. A brain is paid per
  thought, so a body that thinks less often pays less (a speed incentive dressed as physics, kept on purpose)."
- **handover 6.5, a new bullet:** "**Habits.** A body thinks when what it senses has changed (its stores, its cell,
  bodies of other kinds beside it), when what it was doing is no longer possible, and at least once a season. Between
  thoughts it acts by the chances of its last thought, and it learns at its next thought from the whole time since.
  `ew run --full-minds` makes every body think at every update: the detailed form (7.5)."
- **handover 7.5, new:** "### 7.5 Habits. A speed tool, not a law: they must give the patterns of full minds. The
  check: seeds 1 and 5 at 32×32 for 50 years with and without `--full-minds`, the rules (a) to (c) of 7.1 with the
  same noise runs. A calm animal also updates less often (`Body::interval`); `--full-minds` turns that off too."
- **laws.md 5.6, new "Habits"**; 5.3 ("One action is drawn ... at each thought; between thoughts by the habit"); 5.4
  ("discounted by `discount` per update since the last thought"); 6.2 upkeep: "one decision per member for each
  thought since its last update"; 4.4: "a node's movers of one decision go one way, drawn by the chances of the moves".
- **numbers.md, two rows** (both anchored, so the tuned budget does not grow):
  - `wake` | sense units | 0.25 | anchored: one e-fold of a store against its minimum as the brain senses it
    (`SHARE_SENSE` · ln e) | a body thinks again when a sense moved this much since its last thought.
  - `sleep_most` | days | 90 | anchored: one season, a quarter of the year | the longest a body keeps a habit, and the
    longest calm interval. `Config::validate`: at least `batch_days`, at most `MAX_INTERVAL`.
- **plan 2.2 rule 3** (in the x10 plan): "**Three kinds of body code.** Fields for plants; animal minds (habits, nodes,
  place senses, movers as one group, calm intervals); full minds (a lineage marked full thinks at every update, is
  never a node, and may run with a shorter step). Nothing in the animal path may assume that every body sleeps."
- **docs/tape.md:** the column `thoughts`.

## 5. How we check it

| Step | Level | Checks |
|---|---|---|
| MI1 cover once | 1 | golden unchanged; a few `series.csv` the same bit for bit |
| MI3 habits | 3 | golden re-pinned; key-number test once; learning scene (`core/tests/choose.rs`, normal and slow); the full-minds check (slow, new); the nodes check (slow) |
| MI4 place senses | 2 | golden re-pinned; series columns (eaters' members, land K, decisions) within the seed noise |
| MI5 movers | 3 | golden re-pinned; the nodes check; key numbers |
| MI6 calm interval | 3 | the pace test for animals (new, slow); key numbers; full-minds check |

**Tests (few, fast, meaningful):**

- New, fast (`core/tests/choose.rs`): `a_calm_body_keeps_its_habit_and_wakes_on_change`. The learning scene's world
  with its strip removed (nothing changes): one body thinks at its first update, then keeps its habit (`thoughts`
  does not grow, `decisions` does). Then a body of another lineage placed in its cell wakes it at its next update;
  so does half of its key store taken away. Books kept. About 40 lines, under 1 s.
- New, slow (`tape/tests/nodes.rs`, the same harness): `habits_give_the_patterns_of_full_minds`, as 7.5 says.
- New, slow, at MI6: the pace test for animals: the eaters with the calm interval and with `--full-minds`, three
  seeds, births less deaths and real life per kind within the seed noise.
- Kept and must pass, not loosened: the learning scene (`a_body_that_learns_lives_on_what_it_can_take`: learning on
  lives longer by more than 3 standard errors; and the slow `body_learns_to_move_toward_what_it_can_take`). If it
  fails under habits, the habit hides the signal: the answer is a wake rule on surprise (the change of well-being
  over the critic's running error, `sqrt(Body::var)`), never a looser test.
- `each_body_learns_into_its_own_rows` and `brain_grown_and_shrunk_back_is_unchanged` stay.

**Key numbers that must hold** (`tape/tests/ranges.rs` and report): the eaters' mass at year 30 (5.354 ± 0.5 log10
kg); the eaters' members and births less deaths over years 71 to 100; the light-eaters (they eat nothing but are
eaten); land temperature; the books (drift under 1e-12). Per run: decisions and thoughts a year, the greedy share.

**"The same world" now:** default runs and `--full-minds` runs agree by the rules of the nodes check (each kind's mass
and real life within a factor of two, gene means move the same way, the same kind wins), and the key numbers stay in
their ranges.

## 6. Expected gain

Shares of a year's simulation instructions, one thread (island22 / globe36 / region24). "Sleep" is the share of
brained updates that keep their habit, unknown today (MI0 measures it); the table uses 50%, with 30% to 70% as the
range.

| Step | What it removes | island22 | globe36 | region24 |
|---|---|---|---|---|
| today: `choose::act` | | 16.0 | 15.9 | 9.7 |
| MI1 cover once | the `exp` per face | -1.1 | -0.3 | -0.5 |
| MI4 place senses | `walk` and the cell loop, shared about 1.5 to 3 times | -2.0 (1.0 to 2.5) | -0.8 (0.4 to 1.2) | -0.9 (0.5 to 1.4) |
| MI3 habits, sleep 50% | half of the thinking left, and the well-being around part acts | -3.6 (2.1 to 5.1) | -3.2 (1.9 to 4.5) | -2.4 (1.4 to 3.4) |
| MI5 movers as one group | parts and landings of moves (also serial phase) | -0.5 | -2.5 (1 to 4) | -0.5 |
| **after MI1 to MI5** | | **8.8** | **9.1** | **5.4** |
| the year on one thread | | **1.08x** | **1.07x** | **1.05x** |
| MI6 calm interval (gated) | the animal's whole update, 25% to 40% of animal updates | 1.10 to 1.22x more | 1.10 to 1.22x | 1.05 to 1.12x |

Arithmetic of MI3: thinking left after MI1 and MI4 is 9.6 - 1.1 - 2.0 = 6.5 (island). The wake check costs about
0.3 (senses and faces of its own cell, about 2,500 instructions an update). Sleep 50%: 0.5 × (6.5 - 0.3) = 3.1, plus
0.5 × 1.06 (well-being around part acts) = 0.5: 3.6.

Arithmetic of MI6: animal updates are 44% / 47% / 32% of updates; with `choose::act`, an animal update costs about 1.7
times a plant's (island: 114,000 + 79,000 against 114,000), so animals are about 35% to 45% of a year on the island and
the globe. A third fewer animal updates saves 12% to 15%.

**On 4 threads** the gain is smaller: `choose::act` runs in the parallel phase. MI5 helps the serial phase (fewer
relocations to apply, fewer merges), so it counts more there. Expect 1.04x to 1.08x of the 4-thread year for MI1 to MI5.

**What it buys (Amdahl).** If the other sections make everything else 5 times cheaper and minds do nothing, a year is
0.16 + 0.84 / 5 = 0.33 of today: 3.0x, and `choose::act` is half of it. With MI1 to MI5: 0.088 + 0.168 = 0.26: 3.9x.
Without minds work, no amount of other work passes 6.25x. After MI1 to MI5 the cap is 11x; with MI6 higher. For later
smart bodies it gives the full-mind path, the control run and the measure of what a thought costs.

**Even free decisions** (all of `choose::act` gone) give only 1.19x (island) today. The minds work matters for 10x only
together with the plants and air work.

## 7. Line cost

| Step | Adds | Removes | What it deletes or replaces |
|---|---|---|---|
| MI1 | 6 | 3 | the `exp` in `perceive` |
| MI3 | 170 | 25 | `act_as` reads `Plan` (the fields of `Choice` and `Kit` it used); the upkeep's decision term becomes the paid thought; `inputs` calls `senses` |
| MI4 | 60 | 30 | `walk`'s per-decision use and the cell loop of `options` move into the place builder |
| MI5 | 20 | 5 | the per-direction parts of moves in `spread` |
| MI6 | 25 | 2 | none (one branch in `Body::interval`) |
| **total** | **about 280** | **about 65** | net about +215 core code lines |

MI7 (the full-mind mark) and MI8 (batched networks) add nothing now.

## 8. Risks and answers

| Risk | Answer |
|---|---|
| The sleep share is small (bodies wake at every update) | MI0 measures it before MI3 is built; gate 40% on island and globe |
| Habits slow learning; the learning scene fails | Wake on surprise (change of well-being above the critic's running error). Never loosen the test. If still failing: kill MI3 |
| Paying per thought makes animals richer; the world moves | Expected and honest (handover 6.5). The key-number test and the full-minds check decide. If the eaters leave their range, it is reported to the owner as a law effect, not hidden |
| A sleeping prey ignores a predator | A predator of another lineage in its cell changes `others`; a bite changes its stores; both wake it |
| Habit targets are stale (a seen body left) | Rule 5: an impossible option wakes the body, and `act_as` checks the live target as today |
| Movers as one group make the world patchy and move the nodes check | The nodes check decides; kill MI5 if the difference passes the `--crowd` noise |
| MI6 repeats C2's failure | Gated on the plants' choice 3 and on a pace test for animals; kill line below |
| Habit memory per body (about 420 bytes) costs cache | Only bodies with a brain; measure in MI3's bench |
| Rounding amplified by choices hides real changes (level 2 vs 3) | Report both the golden re-pin reason and the series of a few seeds |

## 9. Steps

| ID | What | Level | Done when | Kill line | Needs | Parallel with | Agent |
|---|---|---|---|---|---|---|---|
| MI0 | Probe on a branch: on the three benches count, per brained update, would it sleep under rules 1 to 5; distinct (cell, sight) per survey; move parts per node decision and those that come back | none (branch) | numbers in the measures file | none | — | all | Sonnet (it instruments code) |
| MI1 | The cover of a face once a batch (2.7) | 1 | golden unchanged; island instructions -0.8% or more | under 0.5% on island: revert | — | MI0, MI4 | Sonnet |
| MI3 | Habits, paid per thought, learning over a habit, `--full-minds`, counter `thoughts` (2.1, 2.5) | 3 | all tests; learning scene; full-minds check; key numbers; island -2.5% and globe -2% of a year's instructions | gain under 2% on both, or the full-minds check fails beyond the noise: revert | MI0 (sleep share at least 40%) | MI4 code, merged one at a time | Opus |
| MI4 | Place senses per domain and survey (2.2) | 2 | golden re-pinned; series within noise; island -1% | under 1% on island and 0.5% on globe: revert | MI0 (sharing at least 1.5) | MI1, MI3 | Sonnet |
| MI5 | Movers as one group; `can_go` before a split (2.3) | 3 | nodes check passes; globe -1.5% of a year | under 1.5% on globe, or nodes check fails: revert | MI3 merged (both touch `spread`) | — | Sonnet, Opus review |
| MI6 | Calm animals update less often (2.4) | 3 | pace test for animals; key numbers; full-minds check; -5% of a year on island and globe | the pace test fails, or under 5%: revert | MI3; plants' choice 3 merged | — | Opus |
| MI7 | Full-mind mark by brain size (2.5) | — | built with the first smart kind | — | intelligence step | — | — |
| MI8 | Batched networks (2.6) | 3 | network at least 5% of a year (gate) | under 3% of a year: not built | MI7, a brain with a big shared base | — | Opus |

Order: MI0 and MI1 first (cheap); MI3 and MI4 next; MI5 after MI3; MI6 after the plants' choice 3.

## 10. Interactions with the other sections

**From plants I need:**

- **How an animal sees and eats a plant field.** Today animals perceive plants as faces and eat them with
  `take_body` (eaters of light-eaters). If plants become fields, each field kind in a cell must still offer a
  face-like record (an id for the kind in the cell, members, visible traits) so the brain's inputs keep their shape,
  and `take_body` / `give` toward it must go through a field helper that books both sides. Fewer faces also make the
  faces loop much cheaper (most faces today are plants).
- The survey's per-cell sums (`Life::sum`, `cellf`, `canopy`) still filled from the fields: perception and `go` read
  them.
- Choice 3 (upkeep and income together across an interval): MI6 depends on it.

**From the engine I need:**

- The thread order: habits draw with the body's own rng key (`Rng::new(seed, day, id, Purpose::Choose)`), so they
  do not care about thread order. Movers as one group make fewer relocations, which helps if the serial phase stays.
- 32-bit inside the month: brains are already f32; habits add nothing to the exact books.
- The ceiling: about +215 net core lines for minds.

**From air:** nothing. Season and light inputs read what the air gives, as today.

**I give:**

- Plants and engine: about half the cost of `choose::act` (so the bodies' parallel phase shrinks), and fewer move
  relocations into the serial phase (MI5).
- Everyone: the third kind of body code (full minds) and its control run, so later smart bodies do not need the
  plant or animal shortcuts to be undone.
- The plants section: the same "calm means a longer step" rule (MI6) can serve fields that change slowly, through one
  helper (`Body::interval`, `paces`).
