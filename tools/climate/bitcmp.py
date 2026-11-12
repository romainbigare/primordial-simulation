"""Compare two runs: series.csv without the ms_* columns, and the snapshots (decompressed) up to the new Config field
(8 bytes of 1.0f64 inserted) and the version word.
usage: bitcmp.py OLD_DIR NEW_DIR"""
import csv, gzip, struct, sys

old, new = sys.argv[1], sys.argv[2]


def series(d):
    rows = list(csv.reader(open(f"{d}/series.csv")))
    keep = [i for i, h in enumerate(rows[0]) if not h.startswith("ms_")]
    return [[r[i] for i in keep] for r in rows], len(rows[0]) - len(keep)


a, na = series(old)
b, nb = series(new)
print("series.csv rows", len(a), len(b), "columns kept", len(a[0]), "ms_ columns dropped", na, "equal:", a == b)
sa = gzip.open(f"{old}/snap_00010.bin.gz").read()
sb = gzip.open(f"{new}/snap_00010.bin.gz").read()
print("snapshot bytes", len(sa), len(sb), "difference", len(sb) - len(sa))
diffs = [i for i in range(min(len(sa), len(sb))) if sa[i] != sb[i]]
first = [i for i in diffs][:3]
print("first differing offsets", first)
one = struct.pack("<d", 1.0)
# Skip the version word: find the first difference after it, check the 8 inserted bytes are 1.0, the rest equal.
k = next(i for i in diffs if i >= 8)
print("first difference after the header at", k, "new bytes there", sb[k:k + 8].hex(), "== 1.0f64:", sb[k:k + 8] == one)
print("rest equal after removing those 8 bytes:", sa[k:] == sb[k + 8:], "header before it equal but the version:", [i for i in range(k) if sa[i] != sb[i]])
