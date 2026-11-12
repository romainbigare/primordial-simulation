#!/bin/bash
# The gate before a commit, in the worktree: build, tests, clippy, the core's wasm build, the page. gate.sh LOG
cd /home/user/primordial-simulation/.claude/worktrees/agent-a2f9648fe0863f025
L=$1
echo "head $(git rev-parse --short HEAD) $(date)" > $L
cargo build --release --workspace --all-targets >> $L.build 2>&1; echo "build exit $?" >> $L
cargo test --release --workspace --no-fail-fast > $L.test 2>&1; echo "test exit $?" >> $L
grep "^test result" $L.test | awk '{p+=$4; f+=$6} END {print "passed",p,"failed",f}' >> $L
cargo clippy --release --workspace --all-targets > $L.clippy 2>&1; echo "clippy exit $?" >> $L
grep -c "^warning" $L.clippy | sed 's/^/clippy warnings /' >> $L
cargo build -p pcore --target wasm32-unknown-unknown > $L.wasm 2>&1; echo "pcore wasm exit $?" >> $L
(cd web && npm run build > $L.web 2>&1 && npm run wasm >> $L.web 2>&1); echo "web exit $?" >> $L
echo "done $(date)" >> $L
