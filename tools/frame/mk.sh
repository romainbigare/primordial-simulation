#!/bin/bash
# Build the page of a source tree (its web/) into frame/dist-NAME: index.html (hooked), full.html (hooked, --full),
# and with COUNT=1 count-land.html and count-cover.html (instrumented, --full); WASM=1 builds its wasm too.
# mk.sh SRC NAME
set -e
F=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/frame
T=/home/user/primordial-simulation/.claude/worktrees/agent-a2f9648fe0863f025/target
SRC=$1; NAME=$2; OUT=$F/dist-$NAME
mkdir -p $OUT
SHADE=$T/debug/ew-shade
page() { # shaders dir -> $OUT/$1.html via a scratch copy of web
  local W=$F/tmpweb-$NAME
  rm -rf $W; mkdir -p $W; cp -r $SRC/web/src $SRC/web/build.mjs $SRC/web/index.html $SRC/web/style.css $SRC/web/tsconfig.json $W/
  cp -r $2 $W/shaders
  (cd $W && mkdir -p dist && $SHADE shaders dist/.shaders.json && tsc -p . && node build.mjs > /dev/null)
  cp $W/dist/index.html $OUT/raw-$1.html
  rm -rf $W
}
page plain $SRC/web/shaders
python3 $F/hook.py $OUT/raw-plain.html $OUT/index.html
python3 $F/hook.py $OUT/raw-plain.html $OUT/full.html --full
if [ "$COUNT" = 1 ]; then
  for which in land cover; do
    rm -rf $F/sh-$NAME; cp -r $SRC/web/shaders $F/sh-$NAME
    python3 $F/count.py $F/sh-$NAME $which
    page count-$which $F/sh-$NAME
    python3 $F/hook.py $OUT/raw-count-$which.html $OUT/count-$which.html --full
    rm -rf $F/sh-$NAME
  done
fi
if [ "$WASM" = 1 ]; then
  (cd $SRC && CARGO_TARGET_DIR=$T cargo build -q -p ew-wasm --profile wasm --target wasm32-unknown-unknown)
  cp $T/wasm32-unknown-unknown/wasm/ew_wasm.wasm $OUT/ew.wasm
fi
ls -la $OUT
