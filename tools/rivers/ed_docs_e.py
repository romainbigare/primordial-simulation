p = 'docs/numbers.md'
lines = open(p).read().split('\n')
k = [i for i, l in enumerate(lines) if l.startswith('| `LANE_M` (0.45 m)')]
assert len(k) == 1
lines.insert(k[0] + 1, "| `BED_MOST_M` (60 m), `web/shaders/lib/rock.wgsl` | m | the widest half bed a program made for draws from afar reckons with: what of a bed that wide cannot show at its pixels is compiled out of it (from 30 m a pixel: a bed's width along it, its bars, its banks' light, the signs of its flow; from 1 m: single stones) (drawing only) | the bed of a river of some 300 m³/s by the channel law; the 22 islands' rivers carry under 1 m³/s | tuned |")
open(p, 'w').write('\n'.join(lines))
