#!/bin/bash
# The measuring binaries of the tree as it is, into s3/$1: the ranges test, the ice probe, the cover probe, the
# radiation probe (toggle $2, default rad_toggle3.py). The probes' edits are taken out again.
T=$1
TOG=${2:-rad_toggle3.py}
S=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad
W=/home/user/primordial-simulation/.claude/worktrees/agent-ab92bfed03ac3c12c
D=$S/s3/$T
mkdir -p $D
cd $W
B=$(cargo test --release -p tape --test ranges --no-run 2>&1 | grep -o "target/release/deps/ranges-[0-9a-f]*")
cp $B $D/ranges
B=$(cargo test --release -p pcore --test cover --no-run 2>&1 | grep -o "target/release/deps/cover-[0-9a-f]*")
cp $B $D/cover
cargo build --release -p cli 2>&1 | grep -E "^error" -A8 | head -20
cp target/release/ew $D/ew
python3 $S/probe_toggle.py on
python3 $S/budget_toggle.py on
python3 $S/s3/$TOG on || echo "TOGGLE FAILED"
mkdir -p $W/cli/examples
cp $S/iceprobe.rs $W/cli/examples/iceprobe.rs
cargo build --release -p cli --example coverprobe --example radprobe --example iceprobe 2>&1 | grep -E "^error" -A8 | head -40
cp target/release/examples/coverprobe target/release/examples/radprobe target/release/examples/iceprobe $D/
rm $W/cli/examples/iceprobe.rs
python3 $S/s3/$TOG off
python3 $S/budget_toggle.py off
python3 $S/probe_toggle.py off
rmdir $W/cli/examples 2>/dev/null
echo "PROBE lines left: $(git -C $W diff | grep -c PROBE); status: $(git -C $W status --short | wc -l) files"
echo built $T
