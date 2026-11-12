import re, subprocess, sys
# Inclusive instructions of chosen functions, per simulated year (6 years of steps), from two callgrind files.
WANT = [
    ('the month (run_for)', 'pcore::schedule::run_for '),
    ('flow::liquid', 'pcore::flow::liquid::liquid '),
    ('  runoff::surface', 'pcore::flow::runoff::surface '),
    ('  level::level_over / curve::level_over', 'level_over '),
    ('flow::air', 'pcore::flow::air '),
    ('  air::step::step', 'pcore::flow::air::step::step '),
    ('parts::solve_nodes (all callers)', 'pcore::flow::parts::solve_nodes '),
    ('  parts::found', 'pcore::flow::parts::found '),
    ('liquid::book (cells_state after moves)', 'pcore::flow::liquid::book '),
    ('survey::survey (full state)', 'pcore::survey::survey '),
    ('survey::cover', 'pcore::survey::cover '),
    ('change::step', 'pcore::change::step '),
    ('air::daily::exchange', 'pcore::flow::air::daily::exchange '),
    ('Seen::landing', 'seen::Seen>::landing '),
    ('World::thaw', 'pcore::world::World>::thaw '),
    ('memcpy+memset', None),
]


def load(f):
    out = subprocess.run(['callgrind_annotate', '--inclusive=yes', '--auto=no', '--threshold=100', f], capture_output=True, text=True).stdout
    rows = []
    for l in out.split('\n'):
        m = re.match(r'\s*([\d,]+) \(\s*[\d.]+%\)\s+(.*)$', l)
        if m:
            rows.append((int(m.group(1).replace(',', '')), m.group(2)))
    return rows


def get(rows, key):
    if key is None:
        return sum(n for n, t in rows if ('__memcpy_avx_unaligned_erms [' in t or '__memset_avx2_unaligned_erms [' in t) and t.startswith('./string'))
    best = 0
    for n, t in rows:
        if key in t and '/rayon' not in t.split(':')[0] and 'core/src/slice' not in t:
            best = max(best, n)
    return best


a, b = load(sys.argv[1]), load(sys.argv[2])
ta, tb = get(a, WANT[0][1]), get(b, WANT[0][1])
print(f"{'function':42s} {'before Mi/yr':>12s} {'share':>6s} {'after Mi/yr':>12s} {'share':>6s}")
for name, key in WANT:
    x, y = get(a, key), get(b, key)
    print(f"{name:42s} {x/6e6:12.0f} {100*x/ta:5.1f}% {y/6e6:12.0f} {100*y/tb:5.1f}%")
