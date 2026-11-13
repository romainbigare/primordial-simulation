#!/bin/bash
# The gate (review/orchestrator.md 3) in the worktree, its output into LOG. gate.sh LOG
cd /home/user/primordial-simulation/.claude/worktrees/agent-ab76b80ac68603520 || exit 1
log=$1
{
  echo "== build"; cargo build --release --workspace --all-targets 2>&1 | tail -3
  echo "== test"; cargo test --release --workspace --no-fail-fast 2>&1 | tee $log.test | grep "^test result" | awk '{p+=$4; f+=$6} END {print "passed",p,"failed",f}'
  grep -E "^test .* FAILED|panicked" $log.test | head -20
  echo "== clippy"; cargo clippy --release --workspace --all-targets 2>&1 | grep -E "^(warning|error)" | sort | uniq -c | head -20
  echo "== wasm core"; cargo build -p pcore --target wasm32-unknown-unknown 2>&1 | tail -1
  echo "== done"
} > $log 2>&1
