#!/bin/bash
# Build the radiation probe of the tree as it is into s3/$1/radprobe (TAG given); the probe's edits are taken out again.
T=$1
S=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad
W=/home/user/primordial-simulation/.claude/worktrees/agent-ab92bfed03ac3c12c
TOG=${2:-rad_toggle3.py}
mkdir -p $S/s3/$T
cd $W
python3 $S/s3/$TOG on || exit 1
cargo build --release -p cli --example radprobe 2>&1 | grep -E "^(error|warning: unused)" -A8 | head -40
cp target/release/examples/radprobe $S/s3/$T/radprobe
python3 $S/s3/$TOG off
git -C $W diff | grep -c PROBE
echo built $T
