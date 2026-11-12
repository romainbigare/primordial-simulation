import csv,sys
r=list(csv.reader(open(sys.argv[1])))
keep=[i for i,h in enumerate(r[0]) if not h.startswith('ms_')]
w=csv.writer(sys.stdout)
for row in r: w.writerow([row[i] for i in keep if i<len(row)])
