"""Old and new snapshot: equal once the new one's inserted fields are taken out (the version word, then each run of
inserted bytes, which must be 1.0f64 for the stretch or zeros for an empty planet and zero counters).
usage: bitcmp2.py OLD_DIR NEW_DIR"""
import gzip, struct, sys

a = gzip.open(f"{sys.argv[1]}/snap_00010.bin.gz").read()
b = gzip.open(f"{sys.argv[2]}/snap_00010.bin.gz").read()
a, b = a[8:], b[8:]  # magic and version word
removed = []
i = 0
while True:
    n = min(len(a), len(b))
    while i < n and a[i] == b[i]:
        i += 1
    if i == len(a) and i == len(b):
        break
    # Find the shortest run inserted in b at i after which a[i:] matches b[i+k:] for a while.
    for k in range(1, 64):
        if b[i + k:i + k + 4096] == a[i:i + 4096]:
            removed.append((i, b[i:i + k]))
            b = b[:i] + b[i + k:]
            break
    else:
        print("not an insertion at", i)
        sys.exit(1)
print("equal after removing:", [(at, len(x), x.hex() if len(x) <= 8 else ("zeros" if not any(x) else x.hex())) for at, x in removed], "1.0f64 =", struct.pack("<d", 1.0).hex())
