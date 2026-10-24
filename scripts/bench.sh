#!/usr/bin/env bash
# The speed bench kit (docs/speed-post-foundation/plan.md 1.2).
#   scripts/bench.sh [--remake]        run both benches for 5 years each, print one line per bench
#   scripts/bench.sh profile BENCH     run 1 year of BENCH (island22 or globe36) under callgrind, print the top functions
# The bench worlds are snapshots kept in $BENCH_DIR (default ~/.cache/ew-bench), each with a stamp of the commit and
# the snapshot version. A missing snapshot is made. A snapshot of another version stops the script: use --remake.
# Run it with nothing else on the machine. Outputs stay in $BENCH_DIR and /tmp.
set -euo pipefail

cd "$(dirname "$0")/.."
DIR=${BENCH_DIR:-$HOME/.cache/ew-bench}
EW=target/release/ew
VERSION=$(grep -o 'SNAPSHOT_VERSION: u32 = [0-9]*' tape/src/snapshot.rs | grep -o '[0-9]*$')
# Per bench: the flags of `ew run`, and the year of the snapshot. No pictures or frames beyond the first and the last.
QUIET="--monthly-years 0 --frames-every 100000 --every 100000"
declare -A RUN=([island22]="--size 22 --seed 1 --years 300" [globe36]="--globe --size 18 --seed 1 --years 200")
declare -A YEAR=([island22]=300 [globe36]=200)
BENCHES=(island22 globe36)

make_world() { # make_world NAME: run the world and keep its last snapshot with a stamp
    local tmp; tmp=$(mktemp -d /tmp/ew-bench-make.XXXXXX)
    echo "making $1 (year ${YEAR[$1]}) ..." >&2
    # shellcheck disable=SC2086
    $EW run ${RUN[$1]} $QUIET --out "$tmp" > "$tmp/log.txt"
    mv "$tmp/$(printf 'snap_%05d.bin.gz' "${YEAR[$1]}")" "$DIR/$1.bin.gz"
    echo "commit $(git rev-parse HEAD)" > "$DIR/$1.stamp"
    echo "snapshot_version $VERSION" >> "$DIR/$1.stamp"
    rm -rf "$tmp"
}

check_world() { # check_world NAME REMAKE: make the world if it is missing or asked for, stop if its version is old
    if [[ $2 == 1 || ! -f $DIR/$1.bin.gz || ! -f $DIR/$1.stamp ]]; then make_world "$1"; return; fi
    local have; have=$(awk '$1 == "snapshot_version" {print $2}' "$DIR/$1.stamp")
    if [[ $have != "$VERSION" ]]; then
        echo "$1: its snapshot has version $have, the source has $VERSION. Run scripts/bench.sh --remake" >&2
        exit 1
    fi
}

resume() { # resume NAME YEARS OUTDIR: run `ew resume` from the bench snapshot
    # shellcheck disable=SC2086
    $EW resume --from "$DIR/$1.bin.gz" --years "$2" --out "$3" $QUIET > "$3.log"
}

# The mean of the last 5 rows of the series for the named columns, one value per column, ms columns first.
means() { # means SERIES.CSV COLUMN...
    local f=$1; shift
    tail -n 5 "$f" | cat <(head -n 1 "$f") - | awk -F, -v cols="$*" '
        NR == 1 { for (i = 1; i <= NF; i++) at[$i] = i; next }
        { n = split(cols, c, " "); for (j = 1; j <= n; j++) sum[j] += $(at[c[j]]) }
        END { for (j = 1; j <= n; j++) printf "%.6f ", sum[j] / (NR - 1) }'
}

bench() { # bench NAME: two runs of 5 years, print the line of the lower total
    local name=$1 best="" low=inf out total
    for _ in 1 2; do
        out=$(mktemp -d /tmp/ew-bench-run.XXXXXX)
        resume "$name" 5 "$out"
        total=$(means "$out/series.csv" ms_flow ms_change ms_bodies ms_parts ms_tape | awk '{print $1+$2+$3+$4+$5}')
        if [[ -z $best ]] || awk -v a="$total" -v b="$low" 'BEGIN {exit !(a < b)}'; then
            rm -rf "$best" "$best.log"; best=$out; low=$total
        else rm -rf "$out" "$out.log"; fi
    done
    means "$best/series.csv" ms_flow ms_change ms_bodies ms_parts ms_tape bodies updates | awk -v name="$name" '{
        printf "%-9s %6.3f s/yr  flow %6.0f  change %6.0f  bodies %6.0f  parts %6.0f  tape %6.0f (ms)  alive %7.0f  updates/yr %9.0f\n",
            name, ($1+$2+$3+$4+$5)/1000, $1, $2, $3, $4, $5, $6, $7}'
    rm -rf "$best" "$best.log"
}

top() { # top OUT_FILE [--inclusive=yes]: 25 functions (start-up frames left out; inclusive lists each once)
    local file=$1; shift
    callgrind_annotate --auto=no "$@" "$file" 2>/dev/null | awk -v inc="${1:+1}" '
        /^ *[0-9][0-9,]* \(/ && !/ld-linux|libc-start|lang_start|ew::main|:main |\(below main\)|backtrace/ \
            && (!inc || / \[\//) && ++n <= (inc ? 25 : 26)' | cut -c1-170
}

profile() { # profile NAME: one year under callgrind; the instruction total, then the top 25 inclusive and exclusive
    local out=$DIR/$1.callgrind.out tmp; tmp=$(mktemp -d /tmp/ew-bench-prof.XXXXXX)
    # shellcheck disable=SC2086
    valgrind --tool=callgrind --callgrind-out-file="$out" $EW resume --from "$DIR/$1.bin.gz" --years 1 --out "$tmp" $QUIET > "$tmp.log" 2>&1
    echo "== $1: instructions in one year: $(grep -o 'Collected : [0-9]*' "$tmp.log" | grep -o '[0-9]*$')"
    echo "== $1: top 25 inclusive =="; top "$out" --inclusive=yes
    echo "== $1: top 25 exclusive =="; top "$out"
    rm -rf "$tmp" "$tmp.log"
}

mkdir -p "$DIR"
cargo build --release -p cli >&2
REMAKE=0; [[ ${1:-} == --remake ]] && REMAKE=1
if [[ ${1:-} == profile ]]; then
    [[ -n ${YEAR[${2:-}]:-} ]] || { echo "usage: scripts/bench.sh [--remake] | profile island22|globe36" >&2; exit 1; }
    check_world "$2" 0; profile "$2"
else
    for b in "${BENCHES[@]}"; do check_world "$b" $REMAKE; done
    for b in "${BENCHES[@]}"; do bench "$b"; done
    echo "commit $(git rev-parse --short HEAD)"
fi
