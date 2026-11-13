#!/bin/bash
# Build the step probe at main, S2, S5, S3a, S3b-first and the tip into final/bin; the probe put in and taken out again.
W=/home/user/primordial-simulation/.claude/worktrees/agent-ab92bfed03ac3c12c
F=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/s3/final
cd $W
for pair in ${PAIRS:-main:9da721ca S2:16defd44 S5:9058a961 S3a:d85c1710 tip:air-light}; do
  n=${pair%%:*}; c=${pair##*:}
  git checkout -q $([ $n = tip ] || echo --detach) $c || exit 1
  mkdir -p cli/examples; cp $F/tools/stepprobe.rs cli/examples/stepprobe.rs
  cargo build --release -p cli --example stepprobe 2>&1 | grep -E "^error" -A10 | head -30
  cp target/release/examples/stepprobe $F/bin/stepprobe_$n && echo "built stepprobe $n"
  rm -rf cli/examples
done
git checkout -q air-light
git status --short
echo STEP_BUILT
