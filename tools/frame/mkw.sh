#!/bin/bash
# Page of the worktree as frame/dist-NAME, with the tip wasm (or the worktree's with WASM=own).
F=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/frame
W=/home/user/primordial-simulation/.claude/worktrees/agent-a2f9648fe0863f025
$F/mk.sh $W $1 > $F/mk-$1.log 2>&1 || { tail $F/mk-$1.log; exit 1; }
if [ "$WASM" = own ]; then cp $W/web/dist/ew.wasm $F/dist-$1/; else cp $F/dist-tip/ew.wasm $F/dist-$1/; fi
echo built $1
