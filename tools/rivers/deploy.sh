#!/bin/bash
# Copy the built page (web/dist) into folder NAME here, hooked for the bench (hook.py); with --full, held at the screen's
# own pixels. deploy.sh NAME [--full]
S=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad
W=/home/user/primordial-simulation/.claude/worktrees/agent-ab76b80ac68603520/web
mkdir -p $S/rv/$1 && python3 $S/rv/hook.py $W/dist/index.html $S/rv/$1/index.html $2 && cp $W/dist/ew.wasm $S/rv/$1/ && echo deployed $1
