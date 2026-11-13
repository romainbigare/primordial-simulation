P = 'tape/src/ground/valley.rs'
t = open(P).read()
i = t.index("/// The valleys' work over a window's texels:")
new = '''/// Add to `f` over `win` the valley of the line `pts` of sample `q` of `s`, which reaches `reach` samples: each of
/// its straight pieces gives each texel within reach the weight of its floor at the texel's nearest point of it, as
/// much as it is long (over `PIECE_SHARE` of the reach), and that floor's height so weighted. A texel's floor is so
/// the mean of the line's floors about it, the nearer the heavier: smooth across the ground, with no step where the
/// nearest point of a line that bends leaps from one limb to the other.
pub(super) fn carve(f: &mut Field, win: &Window, s: &Samples, q: usize, pts: &[Vertex], reach: f64) {
    let mut a = pts[0];
    for (n, &b) in pts.iter().enumerate().skip(1) {
        let long = (b.p[0] - a.p[0]).hypot(b.p[1] - a.p[1]);
        // Short pieces are passed over, up to the line's last point.
        if n + 1 < pts.len() && long < CHORD_SHARE * reach {
            continue;
        }
        for y in win.texels(a.p[1].min(b.p[1]) - reach, a.p[1].max(b.p[1]) + reach, 1) {
            for x in win.texels(a.p[0].min(b.p[0]) - reach, a.p[0].max(b.p[0]) + reach, 0) {
                let (at, i) = win.texel(x, y);
                let (_, d2, t) = to_segment(at, a.p, b.p);
                if d2 < reach * reach {
                    let w = weight(d2.sqrt() / reach) * long / (PIECE_SHARE * reach);
                    (f.valley[i], f.weight[i]) = (f.valley[i] + w * floor(s, q, a.t + (b.t - a.t) * t), f.weight[i] + w);
                }
            }
        }
        a = b;
    }
}
'''
t = t[:i] + new
for a, b in [
    ("""const CHORD_SHARE: f64 = 1.0 / 32.0;""", """const CHORD_SHARE: f64 = 1.0 / 32.0;
/// A straight piece of a line this share of `VALLEY_M` long weighs as one whole line did at a point: the pieces of a
/// line about a point add up, so a valley is as wide as one line's weight makes it about `VALLEY_M` / 12 from the
/// line, a little narrower nearer and wider farther."""+"""
const PIECE_SHARE: f64 = 0.25;"""),
]:
    assert t.count(a) == 1, a[:50]
    t = t.replace(a, b)
open(P, 'w').write(t)

def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)
edit('tape/src/ground/channels.rs', [
 ("""        let mut carve = Carve::new(n);
""", ""),
 ("""                carve.line(&mut f, win, self.r.s, l.q, &l.drawn.line, reach_valley);""", """                carve(&mut f, win, self.r.s, l.q, &l.drawn.line, reach_valley);"""),
 ("""use super::valley::{Carve, VALLEY_FROM, VALLEY_M};""", """use super::valley::{carve, VALLEY_FROM, VALLEY_M};"""),
])
