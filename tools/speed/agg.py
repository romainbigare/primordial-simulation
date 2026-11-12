import sys, re, collections
# sum callgrind_annotate self costs by function name (ignore the inlined file)
tot = collections.Counter(); T = 0
for line in open(sys.argv[1]):
    m = re.match(r'\s*([\d,]+) \(\s*[\d.]+%\)\s+(.*)$', line)
    if not m:
        continue
    n = int(m.group(1).replace(',', ''))
    rest = m.group(2)
    if 'PROGRAM TOTALS' in rest:
        T = n; continue
    fn = rest.split(':', 1)[1] if ':' in rest else rest
    fn = re.sub(r'\s*\[.*\]\s*$', '', fn).strip()
    fn = re.sub(r'rayon::iter::plumbing::bridge_producer_consumer::helper::<.*?IterProducer<(.{0,60}).*', r'rayon-helper<\1>', fn)
    tot[fn[:150]] += n
for f, n in tot.most_common(int(sys.argv[2]) if len(sys.argv) > 2 else 40):
    print(f"{n/T*100:6.2f}%  {f}")
