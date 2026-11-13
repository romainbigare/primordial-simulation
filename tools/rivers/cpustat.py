# cpustat.py tag PAGE: put the page's name into each bench line. cpustat.py sum: the median and spread of the CPU
# time of a frame for each view and page, and the second page over the first.
import sys, json, statistics as st
if sys.argv[1] == 'tag':
    for line in sys.stdin:
        try:
            d = json.loads(line)
        except Exception:
            continue
        d['page'] = sys.argv[2]
        print(json.dumps(d), flush=True)
else:
    rows, order = {}, []
    for line in sys.stdin:
        d = json.loads(line)
        if d['page'] not in order:
            order.append(d['page'])
        for x in d['results']:
            # The first frame of a view warms it: left out.
            rows.setdefault((x['view'], d['page']), []).extend(x['cpu'][1:])
        if d['errors']:
            print('errors', d['errors'])
    for v in sorted({k[0] for k in rows}):
        m = {p: st.median(rows[(v, p)]) for p in order if (v, p) in rows}
        for p in order:
            c = rows[(v, p)]
            print(v, p, 'median', m[p], 'ms cpu a frame; least', min(c), 'most', max(c), 'of', len(c))
        if len(order) == 2:
            print(v, order[1], 'over', order[0], '%+.1f%%' % (100 * (m[order[1]] / m[order[0]] - 1)))
