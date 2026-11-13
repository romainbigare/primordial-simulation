#!/bin/bash
# Paired frame timing on the phone profile of page folders A and B (built with deploy.sh NAME --full): ROUNDS rounds, each
# A then B, the median full-frame ms of FRAMES frames a view. cost.sh A B [VIEWS]
cd /tmp/claude-0/-home-user-primordial-simulation/6e08a002-8108-53e2-823b-534b6e8cb25d/scratchpad/rv || exit 1
a=$1; b=$2
VIEWS=${3:-"fit:0:11:11,b2km:2000:15.7099:12.6688,b200m:200:15.7099:12.6688"}
for r in $(seq 1 "${ROUNDS:-3}"); do
  for v in $a $b; do
    PAGE=$v/index.html FRAMES=${FRAMES:-5} timeout 3000 node bench.js 8761 ${PROFILE:-phone} 1 150 time /dev/null "$VIEWS" 2>&1 | python3 -c "
import sys, json
for line in sys.stdin:
    try: d = json.loads(line)
    except Exception: print(line.strip()[:200]); continue
    print('$v', d['when'], 'load', d['load'].split()[0], ' '.join('%s=%s' % (x['view'], sorted(x['ms'])[len(x['ms'])//2]) for x in d['results']), d['errors'])
"
  done
done
