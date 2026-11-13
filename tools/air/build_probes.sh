#!/bin/bash
# Build the key island probe at air-light and at main (9da721ca) into final/bin, then run the rivers probe on air-light.
# The probe files are put in and taken out again; the tree is left clean on air-light.
W=/home/user/primordial-simulation/.claude/worktrees/agent-ab92bfed03ac3c12c
F=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/s3/final
cd $W
for pair in tip:air-light main:9da721ca; do
  n=${pair%%:*}; c=${pair##*:}
  if [ $n = main ]; then git checkout -q --detach $c || exit 1; fi
  mkdir -p cli/examples; cp $F/tools/keyprobe.rs cli/examples/keyprobe.rs
  cargo build --release -p cli --example keyprobe 2>&1 | grep -E "^(error|warning: unused)" -A8 | head -30
  cp target/release/examples/keyprobe $F/bin/keyprobe_$n && echo "built keyprobe $n"
  rm -rf cli/examples
done
git checkout -q air-light
cp $F/tools/zz_rivers_probe2.rs tape/tests/zz_rivers_probe2.rs
cargo test --release -p tape --test zz_rivers_probe2 --no-run 2>&1 | grep -E "^error" -A8 | head -30
B=$(cargo test --release -p tape --test zz_rivers_probe2 --no-run 2>&1 | grep -o "target/release/deps/zz_rivers_probe2-[0-9a-f]*")
cp $B $F/bin/rivers_probe2 && echo built rivers probe
rm tape/tests/zz_rivers_probe2.rs
git status --short
echo PROBES_DONE
