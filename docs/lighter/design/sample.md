# Idea 4: a cheaper sample member (lane A: A1, and A4 if the cover fails)

Steps: PR4, A1, A4 (`plan.md` section 4). Decisions: L6, L15.

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

**A1 (level 3): `Config::sample_every` 4.** One number in `params.rs` (`Config::default`) and its row in `numbers.md`
(the anchor: PR4's checks). The draw, the weight and the cap follow it already; no code changes. A1 changes no saved
layout, but a saved world keeps its own `sample_every`: the step is judged on a `benchZ` save whose `Config` is set to
4 at load (a measuring branch), per stand update. Golden re-pinned (its worlds are made with the default).

Expected: about half of the sample's time, -6 to -7% of a stand update, about -1.5 to -2% of the island's year.

**Statistics.** The mean move of a gene per day is the same; its variance per day doubles (half as many samples, each
moving the mean twice as far: 1/2 x 4 = 2 times today's). The cap (4 steps a sample) is reached only
where `tanh` is near 1, which a node's gain rarely gives. The two selection tests say whether that noise matters.

**A4 (level 2; only if the cover fails its gate, L6):** the sample runs only the stages its gene reads and reuses the
node's own results for the rest. Today the sample runs first and the node after; A4 runs the node first, keeps what
each stage of `to_growth` read and wrote (a small record in `LiveTmp`), and for a sampled gene that only growth or
maturing reads, runs growth alone on the node's state before growth. Needs a map of which stage reads which gene (the
`read` function already knows the classes). About 1/2 to 2/3 of the sample's cost, but `to_growth`'s stages would need
a clean cut, and the order of sums changes (level 2). The cover deletes all of this, so it waits.

**Not chosen:** an estimate of the gain without running the body (a gradient by the genes' effects): every gene acts
through many stages (light, gas, liquid, heat, growth); a model of it would be a second body law (CLAUDE.md 2.1).

## 3. Risks

- **Selection too noisy at 4.** Genes of stands that matter for a place (light, heat, liquid) would track it slower or
  wander. Checks: `selection_inside_a_node_moves_its_mean_as_its_members_would` (the mean follows what members would
  do), `genes_do_not_drift_without_selection`, and the light-eaters' gene means at year 30 within their seeds' noise of
  today's.
- **The plants' copy-mass gene.** PR-C2 found it climbing at long paces under every law, cause not found (open item 5
  of the last report). A noisier selection could feed it. PR4 reads `COPY_MASS`'s mean over 30 years.
- **The key numbers** move a little (a changed default): S50, the light-eaters' mass and life.
- If the cover lands, A1 is deleted with the stands (one number).

## 4. What PR4 must show first

On a branch (no code change but a setting read at load), seeds 1 to 3:
1. The cost of a stand update at `sample_every` 2, 4 and 8 on each `benchZ` save (one year, one thread, the save's
   `Config` set at load).
2. `selection_inside_a_node_moves_its_mean_as_its_members_would` and `genes_do_not_drift_without_selection` at 4 and 8.
3. 64 x 64 for 30 years (the key-number world) at 2 and 4: the light-eaters' gene means at year 30 (the genes `read`
   most often), `COPY_MASS`'s mean year by year, and the key numbers.

**Gate for A1:** at 4 both tests pass and the gene means at year 30 stay within the seeds' noise; else 3 is tried; if 3
fails too, A1 is not built (A4 is the alternative, after the cover's gate).
