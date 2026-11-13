#!/bin/bash
# Build the worktree's page as it stands into NAMEf (held at the screen's pixels) and time its three views on the
# phone: the median CPU ms a frame and what that is over the page before the rivers (pEf) and the tip q9f, as
# measured in cost-q9.txt. abl.sh NAME
S=/tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/rv
W=/home/user/primordial-simulation/.claude/worktrees/agent-ab76b80ac68603520
cd $W/web && npm run build 2>&1 | grep -iE "error|panic"; [ -n "$WASM" ] && (npm run wasm 2>&1 | tail -1)
cd $S && bash deploy.sh ${1}f --full > /dev/null
for r in $(seq 1 "${ROUNDS:-1}"); do
PAGE=${1}f/index.html FRAMES=${FRAMES:-5} timeout 3000 node bench.js 8761 phone 1 150 time /dev/null "fit:0:11:11,b2km:2000:15.7099:12.6688,b200m:200:15.7099:12.6688" 2>&1 | python3 $S/cpustat.py tag ${1}f
done | python3 -c "
import sys, json, statistics as st
base = {'fit': (5515, 6325), 'b2km': (7895, 9780), 'b200m': (11710, 17215)}
rows = {}
for line in sys.stdin:
    d = json.loads(line)
    for x in d['results']: rows.setdefault(x['view'], []).extend(x['cpu'][1:])
    if d['errors']: print('errors', d['errors'])
for v in ('fit', 'b2km', 'b200m'):
    m = st.median(rows[v]); print('$1', v, m, 'ms; over pEf %+.1f%%; over q9f %+.1f%%' % (100 * (m / base[v][0] - 1), 100 * (m / base[v][1] - 1)), 'spread', min(rows[v]), max(rows[v]))
"
