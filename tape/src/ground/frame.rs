//! The curve of a sample's piece of its channel (`channels::base`: a quadratic Bézier through the sample's point, the
//! valley's line there), measured along itself: a channel's line is laid out by how far along that curve and how far
//! across it each of its points lies (`bends`).

/// The straight pieces a curve's length is summed over, and that a stretch without bends is drawn as at most.
pub(super) const CURVE_STEPS: usize = 8;

/// The point at `t` along the quadratic Bézier from `a` to `c` bent toward `b`.
fn bezier([a, b, c]: [[f64; 2]; 3], t: f64) -> [f64; 2] {
    let u = 1.0 - t;
    std::array::from_fn(|k| u * u * a[k] + 2.0 * u * t * b[k] + t * t * c[k])
}

/// The curve of a sample's piece (`channels::base`, samples) with its length: the length to each of `CURVE_STEPS`
/// even shares of it, m.
pub(super) struct Frame {
    base: [[f64; 2]; 3],
    metres: f64,
    to: [f64; CURVE_STEPS + 1],
}

impl Frame {
    pub(super) fn new(base: [[f64; 2]; 3], metres: f64) -> Frame {
        let mut to = [0.0; CURVE_STEPS + 1];
        for i in 0..CURVE_STEPS {
            let (p, q) = (bezier(base, i as f64 / CURVE_STEPS as f64), bezier(base, (i + 1) as f64 / CURVE_STEPS as f64));
            to[i + 1] = to[i] + (q[0] - p[0]).hypot(q[1] - p[1]) * metres;
        }
        Frame { base, metres, to }
    }

    /// The curve's length, m.
    pub(super) fn len(&self) -> f64 {
        self.to[CURVE_STEPS]
    }

    /// The share of the curve that lies `u` m along it, 0 to 1.
    pub(super) fn share(&self, u: f64) -> f64 {
        let i = self.to.partition_point(|&l| l <= u).clamp(1, CURVE_STEPS);
        let span = (self.to[i] - self.to[i - 1]).max(1e-12);
        ((i - 1) as f64 + ((u - self.to[i - 1]) / span).clamp(0.0, 1.0)) / CURVE_STEPS as f64
    }

    /// The curve's direction at its share `t`, a unit vector (its chord's where the curve starts on its bend point).
    fn along(&self, t: f64) -> [f64; 2] {
        let [a, b, c] = self.base;
        let mut d: [f64; 2] = std::array::from_fn(|k| (1.0 - t) * (b[k] - a[k]) + t * (c[k] - b[k]));
        if d[0].hypot(d[1]) < 1e-9 {
            d = [c[0] - a[0], c[1] - a[1]];
        }
        let len = d[0].hypot(d[1]).max(1e-12);
        [d[0] / len, d[1] / len]
    }

    /// The point `u` m along the curve and `off` m across it (past its ends, along its direction there), samples.
    pub(super) fn at(&self, u: f64, off: f64) -> [f64; 2] {
        let t = self.share(u);
        let (p, d) = (bezier(self.base, t), self.along(t));
        let (past, off) = ((u - u.clamp(0.0, self.len())) / self.metres, off / self.metres);
        [p[0] + d[0] * past - d[1] * off, p[1] + d[1] * past + d[0] * off]
    }

    /// The radius the curve turns on, m: its length over the angle between its two ends' directions.
    pub(super) fn radius(&self) -> f64 {
        let (d0, d1) = (self.along(0.0), self.along(1.0));
        let turn = (d0[0] * d1[1] - d0[1] * d1[0]).atan2(d0[0] * d1[0] + d0[1] * d1[1]).abs();
        self.len() / turn.max(1e-9)
    }
}
