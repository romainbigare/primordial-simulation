# Idea 6: animals look around less (lane D)

> **Status after phase P (L14):** D1 and D2 are not built (PR6: perception 0.55 to 0.91% of the year, the take's payer 0.6%). Idea 6 became lane H (`design/animals.md`).

Steps: PR6, D1, D2 (`plan.md` section 4). Decision: L14. "Most of a decision is perception, not the thought" (GM: a
12-unit thought is 8.0 of a decision's 41.2 thousand instructions at `think_days` 20, 19%).

## 1. What the code does today

`choose::act` (`core/src/choose/mod.rs`), for a body with a brain, after its body's day (`survey::update`):
- **A habit** (a node between decisions, `think_days` 20): `spread::repeat` takes again from the ground
  (`feed::take_node`) and from each body of its last decision's prey still in the cell (`feed::take_body`), each
  through `spread::stays` (the acting members split off as a `nodes::Lot` and rejoin by `nodes::rejoin`). GM derived
  about **21.8 thousand instructions a habit**, half of a decision.
- **A decision:**
  1. `perceive::perceive`: its own cell, then the 8 neighbours from a random side, walking each cell's faces
     (`w.life.faces_of(j)`, made once a batch by `survey::faces::make`) until `nseen` (the lineage's `SEEN`, at most
     `MAX_SEEN` 6): per face a test of the room's walls (`bond::room` of its own, then a draw), a cover draw
     (`Face::show`, `rng.f64()`), and in its own cell a look-up `d.find(o.id)` to skip bodies leaving it; then up to 3
     parts of the cell;
  2. `perceive::inputs`: its stores against minimums, the cell's `seen` substances, warmth, size, light, the season,
     the signals heard, Bond's senses, memory;
  3. `brain::think` (8.0 thousand at 12 hidden units), `learn::learn` (the critic and the rows over every learned row);
  4. `perceive::walk`: the cells within `sight` (1 to 4) rings, marked by a stamp; `options`: per seen cell, the
     direction whose heading is nearest (a max over the links) and 10 features added; per seen body its 11 features
     (`NT`) and a score per verb (`brain::score`, a dot of 12); per part its features and verbs;
  5. `pick`, `noise`, the habit kept; `spread::spread` (a node) or `act_as` (an individual), then the acts:
     `take_node`, `take_body`, moves (`go`), puts, gives, watches, Bond's verbs.

**Where the time goes** (one thread, the end's profile of `benchZ`; shares of `run_for`, the year without the load
and the last save): `choose::act` 546 M (11.1%; 8.95% of the whole run) on island22, 609 M (11.3%; 9.22%) on globe36;
`feed::take_node` 199 M (4.0%) and 168 M (3.1%), a third of `act`; `act_as` 148 M (island22). On island22 a take costs
**16.0 thousand instructions** (199.4 M for 12,497 calls), of which `body::pay` 5.6 thousand; building the payer
(`Payer::at` 1.5 thousand, `thermal` 0.4, `activity` 0.3, `cover_of` 0.25) is about 2.5 thousand, **16% of a take,
about 0.5% of the year**; the rest of `pay` is `spend` (the plan and the gas booking). The last report's "18.5
thousand a call for the payer" was wrong. A payer is "valid until its stores or heat change" (`body/pay.rs`), and the
body's physics and growth change them before the take, so only the cover over the body (and its depth) can be reused
bit for bit. **The faces:** 87% (island22), 77% (globe36) and 97% (region24) of the faces made are stands (R2, "P0");
a decision walks them until it has seen `nseen`, drawing cover for each.

## 2. What changes

**D1, a cheaper take (level 1), only if PR6 finds the payer and the repeats over 1% of the year (L14).** The part
of the take's payment that does not change since the upkeep (the cover over the body, its depth) read once per update,
and what PR6 finds repeated in `intake` (`b.supply`, `b.temp`, `b.uses` per substance) and in `stays` (the Lot split
and rejoin when the whole node takes). The world is bit for bit the same. Expected: about -0.5% of the year, under the
kill line (-1%): **likely not built**, and D2 is the lane's step.

**D2, a cheaper view of the bodies seen (level 2, or 4 if it is a law), after the cover's gate (L14).** PR6 names the
cut. The candidates, in the
order they are tried:
1. **Level 2:** what a face shows that does not depend on the one who looks, worked out once a batch with the faces
   (`survey::faces`): the face's own features (`vis[1] / vis[0]` in `ln`, the colours), and whether it is leaving (no
   `d.find` per face); the observer then adds only its own part (its mass, its memory of that id).
2. **Level 2:** the faces of a cell kept in an order that puts the bodies a decision can deal with first, so the walk
   stops sooner. Changes which bodies are seen when a cell holds more than `nseen`: a law (level 4) unless the order
   is the same as today's.
3. **Level 4, waits for the cover's gate:** stands are not seen as bodies; a cell's plants are seen through its
   features (`perceive::cells` already gives `leaf` and `members` per cell), and eaters take from them as from the
   ground. This is what the cover does anyway (`design/cover.md`): if the cover lands, D2 is that, for free; if it
   fails, this is D2's law step, with eating changed with it.
4. **Level 4:** fewer bodies seen (`MAX_SEEN` 6 to 4; the `SEEN` gene's range). Only if 1 to 3 give too little.

**Not chosen:** batched brains (X12: each body owns 76% of its weights); a thought less often (G2 already: a node
decides about every 20 days); sharing a cell's senses between decisions (MI4: at most 0.3% of a year, X84).

## 3. Risks

- **Learning and the story.** Animals are the story: a cut of what they perceive must not slow what they learn, their
  trails, imitation or signals. D2 at level 4 runs the learning scene (`body_learns_to_move_toward_what_it_can_take`),
  `selection_moves_a_nodes_brain_size`, the nodes check, and the `social` columns of the tape (watches, gifts, signals)
  within their seeds' noise; watch-for M2.
- **What eaters eat.** Today an eater eats a stand only after seeing it (`take_body` by the face's id). Hiding stands
  (candidate 3) changes how eaters find food; that is the cover's law and is judged with it.
- **Overlap with the cover.** Most faces are stands. A cut that only makes stands' faces cheaper is deleted by the
  cover: L14 makes D2 wait for the gate when PR6 says so.
- D1 must stay bit for bit: only what does not change between the upkeep and the take may be read once (the cover over
  the body, its depth), not the mass, the plateau state, the share or the rates.

## 4. What PR6 must show first

On a branch, `#[inline(never)]` on the stages (as R2's `r2-split`), one year, one thread, each `benchZ` save:
1. a decision's instructions by stage: `perceive` (its faces loop, the room and cover draws, `d.find`), `inputs`,
   `think`, `learn`, `walk`, `options` (directions, bodies, parts), `pick`, `noise`, `spread` or `act_as`, and the acts
   by verb;
2. a habit's by part: `repeat`, `stays` (split, rejoin), `take_node` (`wanted`, `intake`, `body::pay` and its payer,
   the ground's heat), `take_body`;
3. counters: faces walked per decision and how many were stands, animals, others; faces seen (`nseen`) by kind; the
   share of decisions whose walk stops on `nseen`.

**Gates.** D1 goes ahead only if the take's payer and repeats are over 1% of the year. D2 comes after the cover's
gate: if the cover lands, D2's candidate is chosen on the world without stands' faces (PR6 run again there); if it
fails, candidate 3 (stands seen through the cell) is D2.
