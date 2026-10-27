# Idea 4: a cheaper sample member (A4, only if the cover fails)

Steps: A4 in phase 2 (`plan.md` section 4). Decisions: L6, L15, L22. **A1 (`sample_every` 4) is dropped** (L15): the
cap would make most samples jump a gene's mean by 4 standard deviations (section 2). The cover keeps `copy::select` and
sets the sample's cadence in its own design. PR4 is not run.

## 1. What the code does today

Selection inside a node without a brain (`copy/select.rs`; `laws.md` 4.4). A node's members are born and die in
proportion whatever their genes, so without this selection would act only between nodes.
- `survey::update` calls `copy::sample` **before** the node's own update (`body::live`), then `copy::select` after it.
- `sample` returns nothing for a body with a brain or of one member; else it draws (`Purpose::Select`) whether this
  update samples at all (`r.below(sample_every) != 0`: no), then one gene among those with a spread, and a step of one
  standard deviation up or down; `read` skips a gene the update cannot read (`UNREAD`: the genes of children and
  brains; thresholds the step cannot reach; uptakes of what the ground does not hold as a liquid).
- It clones the node (`b.clone()`), moves the gene, runs `prepare`, opens a checkpoint of the domain's view
  (`Domain::checkpoint`), runs the body's day as far as its growth (`body::gain_to_growth`, the same `physics`, reactor
  and growth as the node's own update), and rolls the view back (`roll_back`): its moves, events and counters are
  dropped.
- `select` moves the gene's mean by `step · tanh(sample's gain - node's gain) · genes · sample_every`, capped at
  `sample_every · |step|` (Robertson 1966; Lande 1976): the weight grows with `sample_every`, so each gene moves at the
  same mean rate whatever its setting.

**Cost** (PL1, `measures.md`): the sample is 11.5 of a stand update's 93.7 thousand instructions (12%); its day 9.1
thousand on about one update in six (half are drawn away by `sample_every` 2, and `read` skips many of the rest). After
PL1 a stand update is 83.9 thousand, the sample about the same 11 thousand (about 13%). `numbers.md`, `sample_every`:
"2 halves the samples' time; x2 and /2 keep plants, animals, carbon gas and land temperature within their noise".

## 2. What changes

**Why not `sample_every` 4 (A1, dropped).** The weight is `genes = differ.len() * sample_every` (about 127 times
`sample_every`), the move `genes · step · tanh(Δgain)`, the cap `sample_every · |step|`. So the cap binds whenever
`|tanh(Δgain)| > 1 / differ.len()`, about 0.008: whenever the sample's log gain differs from the node's by more than
about 1%, which is most samples. Today most samples already move the gene by the cap (2 standard deviations); at 4 the
mean would jump 4 standard deviations in one update, past where any member is. The mean rate stays, but selection
becomes a sign test with large jumps. Half the sample's time (about -1.5 to -2% of the island's year) is not worth that
nor a law cycle (L2), and the cover may need the opposite (a sample on every update).

**A4 (level 2; only if the cover fails its gate, L6):** the sample runs only the stages its gene reads and reuses the
node's own results for the rest. Today the sample runs first and the node after; A4 runs the node first, keeps what
each stage of `to_growth` read and wrote (a small record in `LiveTmp`), and for a sampled gene that only growth or
maturing reads, runs growth alone on the node's state before growth. Needs a map of which stage reads which gene (the
`read` function already knows the classes). About 1/2 to 2/3 of the sample's cost, but `to_growth`'s stages would need
a clean cut, and the order of sums changes (level 2). The cover deletes all of this, so it waits.

**Not chosen:** an estimate of the gain without running the body (a gradient by the genes' effects): every gene acts
through many stages (light, gas, liquid, heat, growth); a model of it would be a second body law (CLAUDE.md 2.1).

## 3. Risks (A4)

- A4 must give the sample's gain as the full run would, to rounding: a gene read in physics must still re-run physics
  (the `read` classes decide); a wrong reuse is a wrong selection with no trace.
- The order of sums changes (level 2): golden re-pinned; the selection tests
  (`selection_inside_a_node_moves_its_mean_as_its_members_would`, `genes_do_not_drift_without_selection`) pass.
- If the cover lands, A4 is never built.

## 4. What must be measured first (only if the cover fails)

The sample's cost by the class of the gene drawn (physics, growth, maturing), on each `benchZ` save, and the share of
samples whose gene only growth or maturing reads. **Gate for A4:** that share at least a half; else A4 is not built
and the sample stays as it is.
