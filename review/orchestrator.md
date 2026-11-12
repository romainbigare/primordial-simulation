# Handover to the next orchestrator

For the next agent who leads the work. Simple English, as the owner asks. Read this first, then `CLAUDE.md`, then
`review/roadmap.md` (why the work has this shape, and section 10: where each stage stands), then `docs/laws.md`. The
spec is `docs/handover.md`; where it and the code differ, the code and `docs/laws.md` are the truth. The other files
in `review/` (`handover.md`, `plan.md`, `top-30.md`, `objects.md`, `big-look.md`) are older looks; this file and the
roadmap are the present.

## 1. The owner and how they work

- The owner writes short messages and reads very simple English only: short sentences, common words.
- They give a direction and expect it done with helpers: you are the orchestrator, architect and reviewer; helpers
  (subagents) write the code; you check, merge, run the gate and push. They asked for at most two helpers at once, Opus
  and Sonnet where each fits (Sonnet did one pure clean-up well; check its claims, one was wrong).
- They judge by pictures. Ask every helper for before | after pictures (same seed, date and place) and send the best
  to the owner. Look at them yourself first: twice a helper's work passed every test and looked worse.
- **Their two rules, above all others:**
  1. **Things emerge from general physics and chemistry laws.** No rule made for one thing.
  2. **Do not compute what is not needed.**
- The "Earth" view is the pretty one: vivid, a little cartoony, tricks allowed if they follow the world's numbers. The
  "True" view stays physically true.
- Tests: few, and always without animals for now (`ew run --roster --plants`). Never loosen a tolerance.
- They said: push to main often.

## 2. How to run helpers

- Give each helper a full brief: the goal, the owner's own words, what is known, the files, what not to touch (the
  other helper's area), what to measure before and after, the gate, the report's form (simple English, bad news
  first), and: commit, never push, no pull request. Each works in its own worktree (`isolation: worktree`). A new
  worktree starts from `origin/main`, not from your newest commit: tell every helper to merge the work branch first.
- Measure first. The briefs that worked asked for a table of numbers before any change, then one change per commit.
- A helper stops and asks when a key number leaves its range. The decision is yours; tell the owner plainly.
- A helper cannot give you permission. If it says an action was denied to it, do not do that action for it.
- Before merging: every commit signed (`git cat-file -p <commit> | grep -c '^gpgsig'` gives 1), the pictures looked
  at, the law commits' messages read. Merge with `git merge --no-ff`. On a conflict of `SNAPSHOT_VERSION`
  (`tape/src/snapshot.rs`) take a number above both sides.
- For a "bit for bit" claim run your own pair: the `ew` binary before and after, same run, equal `hash` and equal
  `series.csv` but the `ms_*` columns.
- Work on the session's branch and push it freely. Push to `main` only as a fast-forward of a commit whose gate
  passed: `git fetch -q origin main && git merge-base --is-ancestor origin/main HEAD && git push origin HEAD:main`.
- Worktrees and scratch files die with the container. Anything worth keeping is merged or pushed before the session
  ends.

## 3. The gate

Before every push to main (CLAUDE.md 6.3). Builds take 10 to 25 minutes when helpers build too: run it in the
background.

```
cargo build --release --workspace --all-targets
cargo test --release --workspace --no-fail-fast 2>&1 | grep "^test result" | awk '{p+=$4; f+=$6} END {print "passed",p,"failed",f}'
cargo clippy --release --workspace --all-targets      # no warning
cargo build -p pcore --target wasm32-unknown-unknown
cd web && npm run build && npm run wasm
```

The fast suite has 210 tests. Then open the page (section 6) and look. For a change of the laws, also the slow tests
(`-- --ignored`): `the_key_numbers_stay_in_their_ranges`, `a_globe_without_life_keeps_an_earth_like_climate`,
`the_long_step_keeps_the_climate`.

**The key numbers' references** (`tape/tests/ranges.rs`) are the means of three runs of the world itself, a guard that
the world stays near itself; the Earth anchors in that test are separate and fixed. After the new ground, the bodies
and the rain, all nine references were measured again (ranges unchanged). Measure them again only with a change of the
laws that moves the world on purpose, in a commit of its own, and tell the owner each time.

## 4. Where things stand

**Main** (and the live page) holds, all gated: fixed cells; the worn ground with bare bed on steep ground
(`space/mantle.rs`); water on one network, bodies full at the start; rain that drifts; plants from their numbers; the
Earth view's first style; the core's first speed pass; the page's world waiting for its frame.

**Also on main since** (the work branch and main are the same commit; fast suite 209 passed, clippy clean, the slow
tests of `tape/tests/ranges` pass):
- **The new ground** (roadmap section 11): a range, hills and a plain on one island (`terrain::rise`), a coast drowned
  after a low stand of 100 m, five makes of bed in folded layers (`space/strata.rs`); `SNAPSHOT_VERSION` 122. With it:
  Flow's books for liquid that only passes through a node (`potential::relayed`), and test scenes that ask the kit
  for their ground by a condition (`testkit::island_with`, `mild_with`).
- **Plants close up**: six crown forms from the kinds' numbers (`web/src/form.ts`, `lib/crown.wgsl`), stands, autumn by
  leaf life, herbs as a sward, snow buries a plant by its depth against its height, roots need loose ground.
- **A crisp map**: shores as smooth lines (`tape/src/ground/shore.rs`), no water ending at a sample's edge, fine
  relief fixed in the world (`lib/relief.wgsl`), bare bed with ragged edges (`loose_seen`), rivers' beds at their
  flood's width with bars, lakes with shallows.
- **The frame's cost**: one place for a pixel's colours, the land program made three times with the detail that cannot
  show compiled out (`LEAST_E`), no frame when nothing changed. Far views cost about 90% of what they did before both
  drawing packages; the 200 m, 50 m and 10 m bars on a phone still 140 to 165% (measured in software only).

- **An island in the planet's air** (`flow::Planet`, `worldgen/src/planet.rs`, `flow/air/{edge,stretch,planet}.rs`): the
  page's three climates (Wide 20° to 65°, the default; Mild 30° to 55°; None), `ew run --band S,N`; a world made
  without a band is bit for bit what it was. `SNAPSHOT_VERSION` 124.
- **Snow, ice and rock on the page**: snow placed where the ground keeps it, its line in patches; the frozen low ground
  off its 8 km grid, floes at its margin, a textured pack (`lib/ice.wgsl`); bare bed in its sample's make colour.
- **The plants' marks decided once a frame per cell** (`marks.wgsl`, `lib/mark.wgsl`), kept in rgba8 textures.

**Known faults on the page:**
- The frame's cost against before the drawing packages (software only; a real phone is not measured): far views 92 to
  97%, 50 m and 10 m 60 to 71%, the 200 m bar on a phone 176% (the near plant field at 0.69 m a pixel, snow and lakes).
- Snow and ice cost 25 to 45% of a far frame in the headless browser, also on an island with no ice (the software
  drawing seems to pay for the program's size, not for the work a pixel runs). The owner's word: the speed on phones
  is perfectly fine. So no more cuts for now; the headless numbers only guard against a change that makes them worse.
- The True view on water: a 2 km haze, no floes. The pack's ridges read as wiggly lines.
- The coast's squares in a Wide island's far north hold no plants (their cells are low ground): bare land there.
- A square notch where a channel joins two lakes (mended, not seen on a picture); steps on lake shores at the 2 km
  bar on a phone (texels of 31 m: `VIEW_TILES` bites); shallow lakes are a pale grey-blue (their true depth).

**In work:** the cover's law (branch `wip/cover-law`, not on main). Built there: replacement by light, growing up,
roots and the ground's depth, then what a plant pays: liquid for the gas its leaves take in, living walls that breathe,
hardiness as the kind's own frost gene. A Wide island now sorts into zones (bare far north, a dry woody belt, low
cover, taller stands in the warm south; the frost gene follows the coldest month, r 0.56 to 0.69). Its faults: the
leaves lose about three times too much liquid per kg made, in one pulse a month against the mean of the whole air
column, so the closed island is 3 K warmer and half again wetter, five key numbers leave their ranges,
`the_long_step_keeps_the_climate` and `the_frost_gene_falls_where_winters_are_cold` fail, and stands stay under 2 m.
The mend in work: the cover only sets its canopy's conductance; Flow's daily surface step evaporates through it
against the air near the ground, as it does for bare ground (one place for evaporation). Then all is measured again.
The key numbers' references will move with this law (plants give the air its vapour): one commit, told to the owner.

## 5. What to do next, in order

1. **The cover's law** to main when its liquid goes through Flow, the slow tests pass or their references are measured
   again on purpose, and a Wide island still shows zones with stands of a true height.
2. **The sorted start** (`experiment/sorted-start`) follows the new law (`cover::lodge`, `sown`, the new fields of
   `Cover`), then is measured on a Wide island.
3. Wet and dry sides: a law for the mean wind's momentum. The strip of the planet with land and life (it is an
   all-liquid, lifeless planet: its air is warmer and its heat-absorbing gas higher than a globe's with life).
4. **The air and sunlight.** The clear air takes nothing from sunlight and the clouds reflect about 5% of it
   (`flow/air/cells.rs`, `exchange::cloud`): the ground at 38° gets about 1.6 times Earth's (284 against about 180
   W/m²), so a warm wet stand builds 2 to 4 kg of dry matter a year where real ones build 0.5 to 2.5. A law for what
   clear air scatters and absorbs and for the cloud's share, with the globe's climate test as its guard; the air's
   heat-absorbing side then has to hold the same temperature with less sun.
5. Channels that lay down their load (plains, marsh, deltas); ice that cuts valleys; the exact solve of the surface's
   step by pieces; the True view's water.

## 6. The live world for the owner

The owner tests an artifact: https://claude.ai/artifact/GKxSQCDfgBAwwL2B8RvEuC (private to them). To update it after
a push to main:
1. `cd web && npm run build && npm run wasm` (makes `web/dist/index.html` and `web/dist/ew.wasm`).
2. Copy `ew.wasm` to a scratch folder, and the page with these removed (regexes, case-insensitive, with `\b` after
   the tag names): the doctype, the `html`, `head` and `body` tags, the `meta charset` and `meta viewport` tags, the
   `link rel=icon`.
3. Publish with the Artifact tool to that URL, the page as `file_path` and
   `files: {"ew.wasm": {"from": <the copied wasm>, "contentType": "application/wasm"}}`. If it refuses because the
   published version was not read in this session, read it first (`action: read`), then publish.

To look at the page before publishing: serve `web/dist` (`python3 -m http.server PORT`) and drive it with Playwright
(`require('/opt/node22/lib/node_modules/playwright')`, `chromium.launch()` with no options): open
`index.html?seed=1&size=22`, wait 6 s, click the last button of `#speeds`, wait, click `#play` to pause, wait 6 s,
screenshot `#map`; to zoom, move the mouse over `#cv` and send wheel steps of -600, then wait 12 s for the tiles. Count
console errors (0 expected; ignore "GPU stall" lines) and read `.stats`. Pause before every picture. The headless
browser draws in software, so its frames per second mean nothing; the world's pace in months per second does.

## 7. Choices the owner should know of

- **References moved this round**, told to the owner: river cells with the rain's drift, then all nine with the
  bodies' start. Each is one commit.
- **A world is alone or in the planet's air.** "None" is today's closed island and keeps every reference. Wide and
  Mild open the island to a strip of the planet run once: drier (0.5 m of rain in the middle of a Wide island, none in
  its far north) and its air's gases pulled to the lifeless strip's.
- **The plateau test** states the melting rule on the balance itself; the long step is backward Euler and is off
  against short steps (laws.md 1.1 says by how much).
- **The page's world waits for its frame**, so the frame's cost is the world's pace.
- Animals are out of every run and test for now, by the owner's word.

## 8. Branches kept for the next session

The owner's word: no work lives only on one machine. Push a helper's branch as soon as it holds something.
- `experiment/climate-bands`: merged to main; kept as it was pushed.
- `experiment/sorted-start`: the sorted start (`worldgen/src/life/nursery/`). Its first commit, the cover's month
  reading one view (`cover::Around`), is on the work branch.
- `wip/drawing`, `wip/cover-law`: the running helpers' work, pushed as it grows.
- `tools/checks`: the check scripts and probes of this work (`tools/check/paused.js`, `phone.js`, `inspect.js`; the
  frame's bench in `tools/frame/`; the climate's and the ground's probes). Study files: never on main.
