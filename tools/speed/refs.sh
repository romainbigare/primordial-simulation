#!/bin/bash
# usage: refs.sh BIN OUTDIR  -- the four bit-for-bit runs
B=$1; O=$2; mkdir -p $O
$B run --seed 1 --size 22 --years 10 --roster --plants --out $O/a > $O/a.log 2>&1 &
$B run --seed 2 --size 32 --years 10 --roster --plants --out $O/b > $O/b.log 2>&1 &
$B run --seed 1 --size 16 --years 3 --null --out $O/c > $O/c.log 2>&1 &
$B run --seed 1 --size 64 --years 5 --roster --plants --out $O/d > $O/d.log 2>&1 &
wait
