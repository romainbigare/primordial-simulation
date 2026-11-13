#!/bin/bash
# Build the globe probe at air-light and at main (9da721ca) into final/bin, the probe put in and taken out again.
W=/home/user/primordial-simulation/.claude/worktrees/agent-ab92bfed03ac3c12c
F=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/s3/final
cd $W
for pair in ${PAIRS:-tip:air-light main:9da721ca}; do
  n=${pair%%:*}; c=${pair##*:}
  git checkout -q $([ $n = main ] && echo --detach) $c || exit 1
  python3 $F/tools/polar_toggle.py on $n || { echo TOGGLE FAILED $n; git checkout -q -- .; continue; }
  cargo build --release -p cli --example globeprobe 2>&1 | grep -E "^error" -A10 | head -40
  cp target/release/examples/globeprobe $F/bin/globeprobe_$n && echo "built globeprobe $n"
  python3 $F/tools/polar_toggle.py off $n
  echo "left: $(git status --short | wc -l) files, PROBE lines $(git diff | grep -c PROBE)"
done
git checkout -q air-light
git status --short
echo GLOBE_BUILT
