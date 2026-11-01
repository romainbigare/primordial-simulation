# Emergent World

A world that runs on five physical laws; everything else must emerge.

- `docs/handover.md`: the plan, the decisions and the rules of work
- `docs/laws.md`: how each law works now, with the code that does it
- `docs/numbers.md`: every free number, its unit and its source
- `docs/tape.md`: what a run records

```
cargo run --release -p cli --bin ew -- run --years 300 --frames-every 5 --out out/run       # simulate: the tape (series.csv, viewer frames) and a snapshot at the end
cargo run --release -p cli --bin ew -- run --years 300 --snapshots-every 100 --pictures-every 50 --months --out out/run   # also snapshots, PNG pictures and months.csv
cargo run --release -p cli --bin ew -- resume --from out/run/snap_00300.bin.gz --years 100 --out out/run   # go on from a snapshot
cargo run --release -p cli --bin ew -- chem                                                   # chemistry 0 as a table
cargo run --release -p cli --bin ew -- numbers out/a out/b                                    # the numbers two runs differ in
cargo run --release -p cli --bin ew                                                           # every command and flag
cargo run --release -p cli --no-default-features --bin ew -- run ...                         # flow and change without threads (same hash)
cd web && npm run build && cp ../out/run/world.ewf.gz dist/                                   # viewer: open dist/index.html via a local server
```

By default the animals start with hand-set brain biases (`--neutral-eaters` for neutral brains) and the
light-eaters with no brain.
