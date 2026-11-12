#!/bin/bash
# usage: ir.sh BIN TAG -- instructions of the month's steps (run_for) over 6 years, size 22, one thread
S=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/speed
mkdir -p $S/ir; cd $S/ir
RAYON_NUM_THREADS=1 valgrind --tool=callgrind --toggle-collect='pcore::schedule::run_for' --callgrind-out-file=$S/ir/cg.$2 $1 run --seed 1 --size 22 --years 6 --roster --plants --out $S/ir/o.$2 > $S/ir/log.$2 2>&1
grep "Collected" $S/ir/log.$2
