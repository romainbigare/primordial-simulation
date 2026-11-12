import csv,sys,glob
# mean of years 11..30 per phase, fastest of the repeats by total
d=sys.argv[1]
for size in (22,32,64):
    best=None
    for f in glob.glob(f"{d}/{size}-*/series.csv"):
        r=list(csv.DictReader(open(f)))
        rows=[x for x in r if 11<=float(x.get('year',0))<=30]
        if not rows: continue
        cols=['ms_flow','ms_change','ms_bodies','ms_parts','ms_tape']
        m={c:sum(float(x[c]) for x in rows)/len(rows) for c in cols}
        tot=sum(m.values())
        if best is None or tot<best[0]: best=(tot,m,len(rows))
    if best:
        tot,m,n=best
        print(size, ' '.join(f"{c[3:]} {m[c]:.1f}" for c in m), f"total {tot:.1f} ms/yr = {1000/tot:.2f} yr/s (n={n})")
