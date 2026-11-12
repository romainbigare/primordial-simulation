#!/bin/bash
# A knock-out build of the tip: its shaders edited by python script EDIT.py (run in the shaders' copy), page only, the
# tip's wasm. knock.sh NAME EDIT.py
set -e
F=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/frame
W=/home/user/primordial-simulation/.claude/worktrees/agent-a2f9648fe0863f025
T=$W/target
NAME=$1; EDIT=$2; OUT=$F/dist-$NAME; TMP=$F/tmpweb-$NAME
mkdir -p $OUT $TMP
cp -r $W/web/src $W/web/build.mjs $W/web/index.html $W/web/style.css $W/web/tsconfig.json $W/web/shaders $TMP/
(cd $TMP/shaders && python3 $EDIT)
(cd $TMP && mkdir -p dist && $T/debug/ew-shade shaders dist/.shaders.json && tsc -p . && node build.mjs > /dev/null)
python3 $F/hook.py $TMP/dist/index.html $OUT/index.html
python3 $F/hook.py $TMP/dist/index.html $OUT/full.html --full
cp $F/dist-tip/ew.wasm $OUT/
echo built $OUT
