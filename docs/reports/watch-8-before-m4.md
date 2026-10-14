# Watch report 8: body matter and food storage

14 October 2026. Code: `main` after the owner approved splitting "body matter" from "food
storage" (watch report 7). All runs are 64 x 64, 60 years, seeds 1 to 3, normal and roster. The
tables of the first two steps are in `watch-8-before-m4/`.

## Short answer

A body is now made of two things:

- **Body matter:** a new substance, made from sugar.
- **Food storage:** sugar and fat.

What that means:

- **Growing costs work and time.** A body builds its matter from sugar, pays building work, and
  builds no faster than its daily allowance (`c_build`).
- **Storing food is cheap.** Refilling sugar costs no building work.
- **Size is the body:** body matter plus the water it holds. The copy trigger, leaf area and height
  all go by it. Stored food and free liquid give no size.
- **Eaters digest.** Body matter taken from outside (a plant, a body, the ground) is broken into
  sugar as it comes in. An eater must build its own matter again.
- **Stores are carried by the body.** How much food and liquid a body may keep is a share of its
  body, not of its whole mass. A full store is the satiety, so the rule "satiety only past `full`"
  is gone.

Results against the code before (seeds 1 to 3, year 60):

| | before | now |
|---|---|---|
| eaters: real life | 0.72 to 0.90 years | **0.81 to 0.94 years** |
| eaters: births per member a year | 1.2 to 1.4 | 1.1 to 1.3 |
| heaviest member, normal runs | 200 to 470 kg | 416 to 442 kg (eaters) |
| roster: animal kinds alive (of 21 kind-runs) | 19 | **21** |
| roster: eaters' real life | 0.43 to 0.67 years | **0.88 to 1.03 years** |
| rules chosen for a run (`docs/numbers.md`) | 4 | **3** |

The plants (1 kg light-eaters) live 0.37 to 0.49 years, with 5.7 x 10⁹ to 1.1 x 10¹⁰ members.
Carbon gas is 199 to 229 ppm in the normal runs and 123 to 233 ppm in the roster runs.

## The owner's question: should plants pay less to build?

Yes in effect, but not as a number per kind.

- **What makes building dearer in life is what is built.**
  - Plant tissue is mostly cellulose-like matter: about 1.2 to 1.5 g of sugar per g.
  - Animal tissue has more protein and fat: about 1.5 to 2.5 g per g.
  - That is 30 to 60% per dry kg, and it comes from chemistry, not from being a plant.
- **The big difference is water.** A plant is 80 to 90% water held by cheap walls, so per kg of
  living mass it costs 2 to 5 times less than an animal (about 70% water).
- **A number per kind would be a rule for a result.** The core cannot know what a "plant" is
  (handover 3 and 14.4). The number would also stop evolving the moment a kind changed its way of
  life.
- **What we built instead:** the cost follows what is built (body matter), and water held by body
  matter counts as body. Light-eaters hold more water per kg of matter, so they are cheap per kg by
  themselves.
- **Possible later:** a second, dearer body matter (protein-like) that strength needs. Animals would
  then also be dearer per dry kg, for a physical reason. Not built.

## What we built

| Step | Change |
|---|---|
| Chemistry 0 | Substance 22, body matter. It is C6H10O5 (six sugar units less one water) and holds the same energy as its sugar: burning gives 17.3 MJ/kg (real cellulose 17.5). Reaction 21 burns it. Dead body matter on the ground oxidises at about 5% a year at 290 K (dead sugar: 7%). |
| Growth | `copy::grow`: 6 sugar → 1 matter + 1 water, paying `c_copy` per kg of matter, within the `c_build` allowance and what the body can spare. |
| Children | A child gets body matter its parent already built, so there is no second payment. Births are limited by how fast parents build again. The child store `paid` is gone. |
| Size | `Body::size` = matter / (1 - h), where h is the share of the body that is held liquid (from the liquid's keep gene). The copy trigger, `full`, leaf, light and height order use it. Intake, upkeep and heat use the whole mass. Starting COPY_MASS and LEAF genes are reset, so each kind copies at the same mass and has the same leaf area at placement. |
| Digestion | Body matter taken from outside becomes sugar as it enters (`Body::digest`). Only a body whose catalyst for reaction 21 is on can take matter. At placement, animals get that gene equal to their sugar-burning gene; light-eaters have it off. |
| Stores | `Body::level` (the keep genes) and the safe minimums are shares of the body, not of the whole mass. |
| Satiety | Off by default: a full store is the satiety. `--sate-all` keeps the review's "upkeep plus building" satiety at every size, for comparison. It kills every animal. |
| Enzyme cost | A gas beyond the first order now counts at its share of the starting air, so a reaction that takes six oxidising gas does not cost 1,800 times more to catalyse. |
| Tape | New columns `root_<k>_matter` (share of member mass) and `root_<k>_body` (kg). |

## How we got there (three steps, all measured)

1. **First version.** Eaters took plant matter straight into their own body, ready-made. They grew
   as fast as they ate:
   - eater lives 0.18 to 0.36 years;
   - births 4.4 to 6.6 per member a year.

   Fix: digestion.
2. **With digestion.**
   - Lives were 0.26 to 0.37 years, and 19 of 21 roster kinds lived.
   - But eaters swelled with stored sugar and liquid, to 1.6 to 2.5 t. Each store was bounded as a
     share of the whole mass, so all of them together had no bound.

   Fix: stores carried by the body.
3. **Stores bounded by the body** (now the default).
   - The results are in the table above.
   - "Satiety only past `full`" (setting A) gave the same normal runs, and 19 of 21 roster kinds.
   - Satiety at every size killed every animal in all six runs, as it did each of the three times
     it was tried.

## What it does not fix yet

- **Plants still live only 0.4 to 0.5 years.** Their births come from the brain's copy chance
  (review 6, finding 4).
- **Lives are still months, not years,** for every kind but the biggest. Eaters reach about one
  year; the 600 kg kind 1.0 to 1.6 years.
- **A member weighs about twice its body,** because it may store food up to 0.9 of its body. That is
  real (fat and gut), but it is upkeep.
- **The roster's heaviest members** are the 600 kg kind, at 2.6 to 3.5 t (before 3.2 to 5.3 t).
- **Living body matter oxidises at the open rate** (5% a year) while a body lives.
- **Old snapshots do not load:** the gene layout grew.
- **Every result is three seeds and 60 years.** The 300-year runs at 128 x 128 are still to do.

## What next

1. The three 300-year seeds at 128 x 128.
2. The small-cell world for the nodes-against-individuals check (plants included).
3. Then M4.

Code size: `core` is 7,633 lines (budget 8,000 through M7).
