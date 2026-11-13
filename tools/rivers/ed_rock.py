p = 'tape/src/ground/rock.rs'
t = open(p).read()
def sub(a, b, n=1):
    global t
    assert t.count(a) == n, (a, t.count(a))
    t = t.replace(a, b)
sub("""//! its discharge times its slope (Bagnold 1966); the wear's stream power law takes the discharge as the square root of the area
//! drained""", """//! its discharge times its slope (Bagnold 1966); the wear's stream power law takes the discharge as the square root of the area
//! drained""") if False else None
sub("""//! A function of the samples alone.
""", """//! And the links of its channel (`links`), by which a shell finds the samples up and down it: what runs through a
//! channel grows from one sample's centre to the next, not in a step at their edge. A function of the samples alone.
""")
sub("""/// Of each sample of `s`, `apart` m between neighbours' centres, row by row, four 16-bit floats: the depth of its loose
/// ground, m, the largest stone its channel moves, m (`STONE_M`; 0 where it runs nowhere: a sink, or under a pond), and
/// two zeros; then of each sample again""", """/// The step from sample `k` of `s` to its neighbour `j`, 1 to 8: 1 + (dx + 1) + 3 (dy + 1), less one past the middle.
fn step(s: &Samples, k: usize, j: usize) -> u32 {
    let (dx, dy) = ((j % s.across) as i64 - (k % s.across) as i64, (j / s.across) as i64 - (k / s.across) as i64);
    debug_assert!(dx.abs() <= 1 && dy.abs() <= 1 && (dx, dy) != (0, 0), "a neighbour");
    let n = (dx + 1) + 3 * (dy + 1);
    (if n > 4 { n } else { n + 1 }) as u32
}

/// The links of sample `k`'s channel in `s`, as one whole number a 16-bit float holds: the step to the sample it runs
/// to (`step`; 0: none), 8 more where it joins that one's channel as a tributary (that one's main way comes from
/// elsewhere), and 17 times the step to its main neighbour upstream (0: none).
fn links(s: &Samples, k: usize) -> u32 {
    let down = match s.to[k] {
        NONE => 0,
        j => step(s, k, j as usize) + if s.main[j as usize] == k as u32 { 0 } else { 8 },
    };
    let up = if s.main[k] == NONE { 0 } else { step(s, k, s.main[k] as usize) };
    down + 17 * up
}

/// Of each sample of `s`, `apart` m between neighbours' centres, row by row, four 16-bit floats: the depth of its loose
/// ground, m, the largest stone its channel moves, m (`STONE_M`; 0 where it runs nowhere: a sink, or under a pond), its
/// channel's links (`links`) and a zero; then of each sample again""")
sub("        out.extend([half(s.loose[k] as f32), half(stone as f32), 0, 0]);", "        out.extend([half(s.loose[k] as f32), half(stone as f32), half(links(s, k) as f32), 0]);")
open(p, 'w').write(t)
