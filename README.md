# Emergent World

A world that runs on five physical laws; everything else must emerge.

- `docs/handover.md`: the plan, the decisions and the rules of work
- `docs/laws.md`: how each law works now, with the code that does it
- `docs/numbers.md`: every free number, its unit and its source
- `docs/tape.md`: what a run records
- `docs/history.md`: what changed in M0 to M3 and in its clean-up, and where old files are in git
- `docs/foundation/plan.md`: the foundation work before M4 (read its main rule first: it replaces code, it does not add patches)

```
cargo run --release -p cli --bin ew -- run --years 300 --frames-every 5 --out out/run       # simulate: pictures, tape, snapshots
cargo run --release -p cli --bin ew -- resume --from out/run/snap_00300.bin.gz --years 100 --out out/run   # go on from a snapshot
cargo run --release -p cli --bin ew -- chem                                                   # chemistry 0 as a table
cargo run --release -p cli --bin ew -- numbers out/a out/b                                    # the numbers two runs differ in
cargo run --release -p cli --bin ew                                                           # every command and flag
cargo run --release -p cli --features threads --bin ew -- run ...                             # flow and change on threads (same hash)
cd web && npm run build && cp ../out/run/world.ewf.gz dist/                                   # viewer: open dist/index.html via a local server
```

By default the animals start with hand-set brain biases and the light-eaters with neutral brains
(`--neutral-eaters` and `--plant-bias` change this).
