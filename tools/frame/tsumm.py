# Summary of times.sh output: per build and view, the median CPU s of the GPU process per frame (single-frame samples),
# the median wall s (the page's fullMs), the samples, and the load range.
# python3 tsumm.py OUT [base]
import sys, json, statistics as st
rows = {}
loads = {}
for line in open(sys.argv[1]):
    try:
        d = json.loads(line)
    except Exception:
        continue
    r = d['r']
    if not r:
        continue
    loads.setdefault(d['build'], []).append(float(r['load'].split()[0]))
    for v in r['results']:
        k = (d['build'], v['view'])
        for ms, cpu, fr in zip(v['ms'], v['cpu'], v['frames']):
            if fr == 1:
                rows.setdefault(k, []).append((cpu, ms))
builds = list(dict.fromkeys(b for b, _ in rows))
views = list(dict.fromkeys(v for _, v in rows))
base = sys.argv[2] if len(sys.argv) > 2 else None
print('build'.ljust(14) + ''.join(f'{v:>22s}' for v in views) + '   load')
for b in builds:
    out = b.ljust(14)
    for v in views:
        x = rows.get((b, v), [])
        if not x:
            out += ' ' * 22; continue
        c = st.median(t[0] for t in x) / 1000; w = st.median(t[1] for t in x) / 1000
        rel = ''
        if base and rows.get((base, v)):
            rel = f'{100 * c / (st.median(t[0] for t in rows[(base, v)]) / 1000):4.0f}%'
        out += f'{c:6.2f} ({w:5.2f}) n{len(x):<2d}{rel:>5s}'
    l = loads.get(b, [])
    out += f'   {min(l):.0f}-{max(l):.0f}' if l else ''
    print(out)
