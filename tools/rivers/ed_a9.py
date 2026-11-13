def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)
edit('tape/src/ground/channels.rs', [
 ("""        let mut pts = Vec::new();
        r.walk(k, &frame, start, &mut |m: Mark| {
            if m.step % kept(m.stride, texel) == 0 {
                let t = frame.share(m.u);
                let drawn = ends[0] + (ends[1] - ends[0]) * t;
                pts.push(Vertex { p: frame.at(m.u, m.v * drawn), t, s: m.s, curve: m.curve * drawn });
            }
        });
        Some(pts)
    }
""", """        let mut marks = Vec::new();
        r.walk(k, &frame, start, &mut |m: Mark| {
            if m.step % kept(m.stride, texel) == 0 {
                marks.push(m);
            }
        });
        // A bend drawn less than fully is drawn that much from the curve: across it, and along it from where the
        // line's own length puts the point (a loop's limb that runs back up the valley would else fold the line over
        // itself on the curve).
        let (from, run) = (marks[0].s, (marks[marks.len() - 1].s - marks[0].s).max(1e-9));
        let point = |m: &Mark| {
            let even = len * (m.s - from) / run;
            let drawn = ends[0] + (ends[1] - ends[0]) * even / len;
            let u = even + (m.u - even) * drawn;
            Vertex { p: frame.at(u, m.v * drawn), t: frame.share(u), s: m.s, curve: m.curve * drawn }
        };
        Some(marks.iter().map(point).collect())
    }
"""),
])
