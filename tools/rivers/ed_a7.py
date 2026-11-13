def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a, t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)
edit('tape/src/ground/bends.rs', [
 ("""pub const BED_RUNOFF: f64 = FLOOD * WET_SHARE * TYPICAL_RUNOFF;
/// The lands a tile is drawn for pass on about this many times `TYPICAL_RUNOFF` on a year's mean.
const WET_SHARE: f64 = 4.0;
""", """pub const BED_RUNOFF: f64 = FLOOD * TYPICAL_RUNOFF;
"""),
 ("""const TURN_SHARE: f64 = 0.4;
""", """const TURN_SHARE: f64 = 0.4;
/// A reach's slope and its valley's floor are read over this many samples up and down the channel from its own: the
/// ground from one sample to the next steps, a reach's bends follow the valley over a few of them.
const REACH_SAMPLES: usize = 2;
"""),
 ("""        let (c, r) = ((k % s.across) as i64, (k / s.across) as i64);
        let (dx, dy) = ((to as usize % s.across) as i64 - c, (to as usize / s.across) as i64 - r);
        let run = self.metres * ((dx * dx + dy * dy) as f64).sqrt();
        let slope = (s.h[k] - s.h[to as usize]).max(0.0) / run;
        let at = |c: i64, r: i64| s.h[r.clamp(0, s.down as i64 - 1) as usize * s.across + c.clamp(0, s.across as i64 - 1) as usize];
        let across = ((at(c - dy, r + dx) - s.h[k]).max(0.0) + (at(c + dy, r - dx) - s.h[k]).max(0.0)) / (2.0 * run);
        let floor = if across > 0.0 { depth / across } else { f64::INFINITY };
        let free = BELT_WIDTHS * width * (1.0 - ease(BENDS_BELOW, STRAIGHT_ABOVE, slope));
        if std::env::var("DBG").is_ok() && (c - 253).abs() < 6 && (r - 205).abs() < 6 && s.up[k] > 5e6 { eprintln!("DBG {c} {r} up {:.1} km2 width {width:.2} slope {slope:.4} across {across:.4} floor {floor:.1} free {free:.1} radius {:.0}", s.up[k] / 1e6, frame.radius()); }
        Room { width, belt: free.min(floor).min(TURN_SHARE * frame.radius()).min(BEND_MOST * self.metres) }
    }
""", """        // The reach: up the main way and down the channel's own, out of the ponds.
        let (mut top, mut bottom, mut run, mut across, mut n) = (k, k, 0.0, self.rise_across(k, to as usize), 1.0);
        for _ in 0..REACH_SAMPLES {
            if s.main[top] != NONE {
                (run, top) = (run + self.apart(top, s.main[top] as usize), s.main[top] as usize);
                (across, n) = (across + self.rise_across(top, s.to[top] as usize), n + 1.0);
            }
            let j = s.to[bottom];
            if j != NONE && s.pond[j as usize] == NONE {
                (run, bottom) = (run + self.apart(bottom, j as usize), j as usize);
                if s.to[bottom] != NONE {
                    (across, n) = (across + self.rise_across(bottom, s.to[bottom] as usize), n + 1.0);
                }
            }
        }
        let slope = if run > 0.0 { (s.h[top] - s.h[bottom]).max(0.0) / run } else { f64::INFINITY };
        let floor = if across > 0.0 { depth * n / across } else { f64::INFINITY };
        let free = BELT_WIDTHS * width * (1.0 - ease(BENDS_BELOW, STRAIGHT_ABOVE, slope));
        Room { width, belt: free.min(floor).min(TURN_SHARE * frame.radius()).min(BEND_MOST * self.metres) }
    }

    /// The distance between the centres of the neighbours `a` and `b`, m.
    fn apart(&self, a: usize, b: usize) -> f64 {
        let n = self.s.across;
        self.metres * ((a % n) as f64 - (b % n) as f64).hypot((a / n) as f64 - (b / n) as f64)
    }

    /// How fast the ground rises across the valley at sample `k`, which runs to `to`, rise over run: the mean of its
    /// two neighbours square to its way, as far as they stand over it.
    fn rise_across(&self, k: usize, to: usize) -> f64 {
        let s = self.s;
        let (c, r) = ((k % s.across) as i64, (k / s.across) as i64);
        let (dx, dy) = ((to % s.across) as i64 - c, (to / s.across) as i64 - r);
        let at = |c: i64, r: i64| s.h[r.clamp(0, s.down as i64 - 1) as usize * s.across + c.clamp(0, s.across as i64 - 1) as usize];
        ((at(c - dy, r + dx) - s.h[k]).max(0.0) + (at(c + dy, r - dx) - s.h[k]).max(0.0)) / (2.0 * self.apart(k, to))
    }
"""),
])
